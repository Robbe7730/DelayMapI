#![feature(proc_macro_hygiene, decl_macro)]

mod gtfs_realtime;

use gtfs_realtime::FeedMessage;

use chrono::NaiveDate;
use chrono::TimeZone;
use chrono::Timelike;
use chrono::Utc;
use chrono_tz::Europe::Brussels;

use gtfs_structures::Exception;
use gtfs_structures::Gtfs;
use gtfs_structures::StopTime;
use gtfs_structures::Trip;

use lazy_static::lazy_static;

use rocket::*;
use rocket_contrib::json::Json;

use serde::Serialize;

use protobuf::Message;

use std::collections::HashMap;
use std::sync::Mutex;

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
struct DelayMapStopTime {
    name: String,
    lat: Option<f64>,
    lon: Option<f64>,
    arrival_delay: i32,
    arrival_datetime: String,
    departure_delay: i32,
    departure_datetime: String,
}

impl DelayMapStopTime {
    fn from_gtfs(stoptime: &StopTime, delay: &Delay) -> DelayMapStopTime {
        DelayMapStopTime {
            name: stoptime.stop.name.clone(),
            lat: stoptime.stop.latitude,
            lon: stoptime.stop.longitude,
            arrival_delay: delay.arrival_delay.unwrap_or(0),
            arrival_datetime: stoptime
                .arrival_time
                .map(|time| {
                    format!(
                        "{:02}:{:02}:{:02}",
                        (time / 60 / 60) % 24,
                        (time / 60) % 60,
                        time % 60
                    )
                })
                .unwrap_or(format!("UNKNOWN")),
            departure_delay: delay.departure_delay.unwrap_or(0),
            departure_datetime: stoptime
                .departure_time
                .map(|time| {
                    format!(
                        "{:02}:{:02}:{:02}",
                        (time / 60 / 60) % 24,
                        (time / 60) % 60,
                        time % 60
                    )
                })
                .unwrap_or(format!("UNKNOWN")),
        }
    }
}

#[derive(Serialize, Debug, Clone)]
struct DelayMapTrain {
    name: String,
    stops: Vec<DelayMapStopTime>,
    next_stop_index: usize,
    estimated_lat: f64,
    estimated_lon: f64,
}

impl DelayMapTrain {
    fn from_gtfs(trip: &Trip, delaymap: &HashMap<String, HashMap<String, Delay>>) -> DelayMapTrain {
        let mut stops = vec![];
        let mut curr_delay = Delay {
            arrival_delay: Some(0),
            departure_delay: Some(0),
        };
        for stop_time in trip.stop_times.iter() {
            if let Some(trip_delaymap) = delaymap.get(&trip.id) {
                if let Some(delay_patch) = trip_delaymap.get(&stop_time.stop.id) {
                    curr_delay.arrival_delay = delay_patch
                        .arrival_delay
                        .or(curr_delay.arrival_delay);
                    curr_delay.departure_delay = delay_patch
                        .departure_delay
                        .or(curr_delay.departure_delay);
                }
            }
            stops.push(DelayMapStopTime::from_gtfs(&stop_time, &curr_delay));
        }
        DelayMapTrain {
            name: trip
                .trip_headsign
                .clone()
                .unwrap_or("Unknown Train".to_string()),
            stops: stops,
            next_stop_index: 0,
            estimated_lat: 0.0,
            estimated_lon: 0.0,
        }
    }
}

#[derive(Serialize, Debug, Clone)]
struct Delay {
    arrival_delay: Option<i32>,
    departure_delay: Option<i32>,
}

#[get("/trains")]
fn trains() -> Json<Vec<DelayMapTrain>> {
    let gtfs = GTFS.lock().unwrap();
    let delays = get_delays();
    Json(
        gtfs.trips
            .values()
            .filter_map(|trip| {
                if rides_now(&gtfs, &trip) {
                    Some(DelayMapTrain::from_gtfs(trip.clone(), &delays))
                } else {
                    None
                }
            })
            .collect(),
    )
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
                    let mut delay = Delay {
                        departure_delay: None,
                        arrival_delay: None,
                    };
                    let stop_id = update.get_stop_id().to_string();

                    if let Some(departure) = update.departure.into_option() {
                        delay.departure_delay = Some(departure.get_delay())
                    }

                    if let Some(arrival) = update.arrival.into_option() {
                        delay.arrival_delay = Some(arrival.get_delay())
                    }

                    delay_map.insert(stop_id, delay);
                }
                ret.insert(key.to_string(), delay_map);
            }
        }
    }

    ret
}

fn main() {
    get_delays();
    // This also loads the data at startup
    GTFS.lock().unwrap().print_stats();
    rocket::ignite().mount("/", routes![trains]).launch();
}
