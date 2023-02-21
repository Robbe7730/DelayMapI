#![feature(proc_macro_hygiene, decl_macro)]

mod gtfs_realtime;
mod delay;
mod delaymap_stop_time;
mod delaymap_train;
mod delaymap_stop;
mod delaymap_works;
mod delaymap_works_parser;

use delay::Delay;
use delaymap_train::DelayMapTrain;
use delaymap_works::DelayMapWorks;
use delaymap_works_parser::DelayMapWorksParser;

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

use protobuf::Message;

use std::collections::HashMap;
use std::env;
use std::sync::RwLock;
use std::thread;
use std::time::Duration;

use tracing_subscriber::prelude::*;

lazy_static! {
    static ref GTFS: RwLock<Gtfs> = {
        let gtfs = Gtfs::from_url(
            "https://sncb-opendata.hafas.de/gtfs/static/c21ac6758dd25af84cca5b707f3cb3de",
        )
        .expect("Invalid GTFS url");
        RwLock::new(gtfs)
    };
}

#[get("/trains?<language>")]
#[tracing::instrument]
fn trains(language: Option<String>) -> Json<Vec<DelayMapTrain>> {
    let gtfs = GTFS.read().unwrap();
    let delays = get_delays().ok();
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
#[tracing::instrument]
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

    let mut ret = vec!();

    let content = response.text().unwrap_or("".to_string());

    let mut found_works = true;
    let mut parser = DelayMapWorksParser::new(language, content);
    while found_works {
        // TODO: If this fails, we need to recreate the rwlock
        let gtfs = GTFS.read().unwrap();

        let res_new_works = parser.parse_next(gtfs);
        if let Ok(Some(new_works)) = res_new_works {
            ret.push(new_works);
        } else {
            if let Err(e) = res_new_works {
                sentry::capture_error(&e);
            }
            found_works = false;
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

#[tracing::instrument]
fn get_delays() -> Result<HashMap<String, HashMap<String, Delay>>, String> {
    let mut response = reqwest::blocking::get(
        "https://sncb-opendata.hafas.de/gtfs/realtime/c21ac6758dd25af84cca5b707f3cb3de",
    ).map_err(|x| {
        sentry::capture_error(&x);
        x.to_string()
    })?;

    let feed = FeedMessage::parse_from_reader(&mut response)
        .map_err(|x| {
        sentry::capture_error(&x);
        x.to_string()
    })?;

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

    Ok(ret)
}

#[tracing::instrument]
fn update_gtfs() {
    let mut gtfs = GTFS.write().unwrap();
    *gtfs = Gtfs::from_url(
        "https://sncb-opendata.hafas.de/gtfs/static/c21ac6758dd25af84cca5b707f3cb3de",
    )
    .expect("Invalid GTFS url");
}

fn main() {
    let _guard ;
    if let Ok(dsn) = env::var("SENTRY_DSN") {
        tracing_subscriber::Registry::default()
            .with(sentry_tracing::layer())
            .init();

        _guard = sentry::init((dsn, sentry::ClientOptions {
            release: sentry::release_name!(),
            traces_sample_rate: 1.0,

            ..Default::default()
        }));
    }

    thread::spawn(move || {
        loop {
            sentry::capture_message("Updating", sentry::Level::Info);
            update_gtfs();
            sentry::capture_message("Done updating", sentry::Level::Info);
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
