#![feature(proc_macro_hygiene, decl_macro)]

mod gtfs_realtime;
mod delay;
mod delaymap_stop_time;
mod delaymap_train;
mod delaymap_stop;

use delay::Delay;
use delaymap_train::DelayMapTrain;
use delaymap_stop::DelayMapStop;

use gtfs_structures::Translatable;
use gtfs_realtime::FeedMessage;

use chrono::NaiveDate;
use chrono::TimeZone;
use chrono::Timelike;
use chrono::Utc;
use chrono_tz::Europe::Brussels;

use gtfs_structures::Exception;
use gtfs_structures::Gtfs;
use gtfs_structures::Trip;

use lazy_static::lazy_static;

use rocket::*;
use rocket_contrib::json::Json;

use serde::Serialize;

use protobuf::Message;

use std::collections::HashMap;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

lazy_static! {
    static ref GTFS: Mutex<Gtfs> = {
        let gtfs = Gtfs::from_url(
            "https://sncb-opendata.hafas.de/gtfs/static/c21ac6758dd25af84cca5b707f3cb3de",
        )
        .expect("Invalid GTFS url");
        Mutex::new(gtfs)
    };
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct DelayMapWorks {
    id: String,
    name: String,
    message: String,
    impacted_station: Option<DelayMapStop>,
    start_date: String,
    end_date: String,
    start_time: String,
    end_time: String,
    urls: Vec<DelayMapURL>,
}

impl DelayMapWorks {
    pub fn empty() -> Self {
        return DelayMapWorks {
            id: "Unknown id".to_string(),
            name: "Unknown name".to_string(),
            message: "No message given".to_string(),
            impacted_station: None,
            start_date: "Unknown start date".to_string(),
            end_date: "Unknown end date".to_string(),
            start_time: "Unknown start time".to_string(),
            end_time: "Unknown end time".to_string(),
            urls: vec!(),
        }
    }
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct DelayMapURL {
    url: String,
    label: String,
}

#[get("/trains?<language>")]
fn trains(language: Option<String>) -> Json<Vec<DelayMapTrain>> {
    let gtfs = GTFS.lock().unwrap();
    let delays = get_delays();
    Json(
        gtfs.trips
            .values()
            .filter_map(|trip| {
                let translated_trip = trip.translate(&gtfs, &language.clone().unwrap_or("en".to_string()));
                if rides_now(&gtfs, &translated_trip) {
                    Some(DelayMapTrain::from_gtfs(&translated_trip, &delays))
                } else {
                    None
                }
            })
            .collect(),
    )
}

#[get("/works?<language>")]
fn works(language: Option<String>) -> Json<Vec<DelayMapWorks>> {
    let language_path = match language.as_ref().map(String::as_str) {
        Some("nl") => "nny",
        Some("en") => "eny",
        Some("fr") => "fny",
        Some("de") => "dny",
        None => "eny",
        _ => "eny",
    };

    let response_res = reqwest::blocking::get(
        format!("http://www.belgianrail.be/jp/nmbs-realtime/query.exe/{}?performLocating=512&tpl=himmatch2json&look_nv=type|himmatch|maxnumber|300|no_match|yes|pubchannels|custom1|1028|", language_path),
    );

    if response_res.is_err() {
        return Json(vec!());
    }

    let response = response_res.unwrap();

    let gtfs = GTFS.lock().unwrap();

    let mut ret = vec!();

    let content = response.text().unwrap_or("".to_string());

    let mut curr_works = DelayMapWorks::empty();

    let mut line_iter = content.lines().peekable();
    while line_iter.peek().is_some() {
        let line = line_iter.next().unwrap();
        if line == "{" {
            // Create new DelayMapWorks
            curr_works = DelayMapWorks::empty();
        } else if line == "}" {
            // Store the DelayMapWorks
            ret.push(curr_works.clone());
        } else {
            let line_split = line.split_once(':');

            if line_split.is_some() {
                let (mut key, mut value) = line_split.unwrap();
                key = key.strip_prefix(",").unwrap_or(key);
                key = key.strip_prefix("\"").unwrap_or(key);
                key = key.strip_suffix("\"").unwrap_or(key);

                value = value.strip_prefix(",").unwrap_or(value);
                value = value.strip_prefix("\"").unwrap_or(value);
                value = value.strip_suffix("\"").unwrap_or(value);

                match key {
                    "id" => curr_works.id = value.to_string(),
                    "caption" => curr_works.name = value.to_string(),
                    "message" => curr_works.message = value.to_string(),
                    "pubstartdate_0" => curr_works.start_date = value.to_string(),
                    "pubstarttime_0" => curr_works.start_time = value.to_string(),
                    "pubenddate_0" => curr_works.end_date = value.to_string(),
                    "pubendtime_0" => curr_works.end_time = value.to_string(),
                    "impactstation_extId" => curr_works.impacted_station = 
                            gtfs.get_stop_translated(
                                value,
                                &language.clone().unwrap_or("en".to_string())
                            ).map(|stop| stop.into())
                            .ok(),
                    "urllist" => {
                        let mut urls = vec!();
                        let mut urlline = line;
                        let mut curr_url = DelayMapURL {
                            label: "Link".to_string(),
                            url: "#".to_string(),
                        };
                        while urlline != "]" {
                            urlline = line_iter.next().unwrap();
                            if urlline.ends_with("{") {
                                curr_url = DelayMapURL {
                                    label: "Link".to_string(),
                                    url: "#".to_string(),
                                };
                            } else if urlline.starts_with("}") {
                                urls.push(curr_url.clone());
                            } else {
                                let urlline_split = urlline.split_once(":");

                                if urlline_split.is_some() {
                                    let (mut urlkey, mut urlvalue) = urlline_split.unwrap();
                                    urlkey = urlkey.strip_prefix(",").unwrap_or(urlkey);
                                    urlkey = urlkey.strip_prefix("\"").unwrap_or(urlkey);
                                    urlkey = urlkey.strip_suffix("\"").unwrap_or(urlkey);

                                    urlvalue = urlvalue.strip_prefix(",").unwrap_or(urlvalue);
                                    urlvalue = urlvalue.strip_prefix("\"").unwrap_or(urlvalue);
                                    urlvalue = urlvalue.strip_suffix("\"").unwrap_or(urlvalue);
                                    match urlkey {
                                        "url" => curr_url.url = urlvalue.to_string(),
                                        "label" => curr_url.label = urlvalue.to_string(),
                                        _ => {}
                                    }
                                }
                            }
                        }

                        curr_works.urls = urls.clone();
                    },
                    _ => {}
                }
            }
        }
    }

    return Json(
        ret
    );
}

fn rides_now(gtfs: &Gtfs, trip: &Trip) -> bool {
    let local_datetime = Brussels.from_utc_datetime(&Utc::now().naive_utc());
    let local_date: NaiveDate = local_datetime.date().naive_local();
    let local_timestamp = local_datetime.time().num_seconds_from_midnight();

    let trip_starting_timestamp = trip.stop_times[0].departure_time;
    let trip_ending_timestamp = trip.stop_times.last().unwrap().arrival_time;

    // if it has no start or end timestamp, assume it doesn't ride
    if trip_starting_timestamp.is_none() || trip_ending_timestamp.is_none() {
        return false;
    }

    // If it rides today and at this time, it is currently on the road
    if rides_at_date(gtfs, trip, local_date)
        && trip_starting_timestamp.unwrap() <= local_timestamp
        && trip_ending_timestamp.unwrap() >= local_timestamp
    {
        return true;
    }

    // If it rode yesterday, but after midnight it could still be on the road
    if rides_at_date(gtfs, trip, local_date.pred())
        && trip_ending_timestamp.unwrap() >= (24 * 60 * 60)
    {
        if trip_starting_timestamp.unwrap() < 24 * 60 * 60 {
            return trip_starting_timestamp.unwrap() <= local_timestamp
                || trip_ending_timestamp.unwrap() - 24 * 60 * 60 >= local_timestamp;
        } else {
            return trip_starting_timestamp.unwrap() - 24 * 60 * 60 <= local_timestamp
                && trip_ending_timestamp.unwrap() - 24 * 60 * 60 >= local_timestamp;
        }
    }

    return false;
}

fn rides_at_date(gtfs: &Gtfs, trip: &Trip, date: NaiveDate) -> bool {
    let mut ret = false;

    // Check if it rides in a normal schedule
    if let Some(calendar) = gtfs.calendar.get(&trip.service_id) {
        if calendar.start_date <= date && calendar.end_date >= date && calendar.valid_weekday(date)
        {
            ret = true;
        }
    }

    // Check if there are exceptions today
    for extra_day in gtfs
        .calendar_dates
        .get(&trip.service_id)
        .iter()
        .flat_map(|e| e.iter())
    {
        if extra_day.date == date {
            if extra_day.exception_type == Exception::Added {
                ret = true;
            } else if extra_day.exception_type == Exception::Deleted {
                ret = false;
            }
        }
    }

    ret
}

fn get_delays() -> HashMap<String, HashMap<String, Delay>> {
    let mut response = reqwest::blocking::get(
        "https://sncb-opendata.hafas.de/gtfs/realtime/c21ac6758dd25af84cca5b707f3cb3de",
    )
    .unwrap();
    let feed = FeedMessage::parse_from_reader(&mut response).unwrap();

    let mut ret = HashMap::new();

    for entity in feed.entity {
        if let Some(update) = entity.trip_update.into_option() {
            if let Some(trip) = update.trip.into_option() {
                let key = trip.get_trip_id();
                let mut delay_map: HashMap<String, Delay> = HashMap::new();
                for update in update.stop_time_update {
                    let stop_id = update.get_stop_id().to_string();
                    delay_map.insert(stop_id, update.into());
                }
                ret.insert(key.to_string(), delay_map);
            }
        }
    }

    ret
}

fn update_gtfs() {
    let mut gtfs = GTFS.lock().unwrap();
    *gtfs = Gtfs::from_url(
        "https://sncb-opendata.hafas.de/gtfs/static/c21ac6758dd25af84cca5b707f3cb3de",
    )
    .expect("Invalid GTFS url");
}

fn main() {
    thread::spawn(move || {
        loop {
            println!("Updating");
            update_gtfs();
            println!("Done updating");
            thread::sleep(Duration::new(24 * 60 * 60, 0));
        }
    });

    let cors = rocket_cors::CorsOptions::default()
        .to_cors()
        .expect("Invalid CORS settings");
    rocket::ignite()
        .mount("/", routes![trains, works])
        .attach(cors)
        .launch();
}
