#![feature(proc_macro_hygiene, decl_macro)]

use gtfs_structures::Gtfs;
use gtfs_structures::Stop;
use lazy_static::lazy_static;
use rocket::*;
use rocket_contrib::json::Json;
use serde::Serialize;
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
struct DelayMapStop {
    name: String,
    lat: Option<f64>,
    lon: Option<f64>,
}

impl From<Stop> for DelayMapStop {
    fn from(stop: Stop) -> DelayMapStop {
        DelayMapStop {
            name: stop.name,
            lat: stop.latitude,
            lon: stop.longitude,
        }
    }
}

#[derive(Serialize, Debug, Clone)]
struct DelayMapTrain {
    name: String,
    delay: usize,
    stops: Vec<DelayMapStop>,
    next_stop_index: usize,
    estimated_lat: f64,
    estimated_lon: f64,
}

#[get("/stops")]
fn index() -> Json<Vec<DelayMapStop>> {
    Json(
        GTFS.lock()
            .unwrap()
            .stops
            .values()
            .map(|x| (**x).clone().into())
            .collect(),
    )
}

fn main() {
    {
        // This also loads the data at startup
        GTFS.lock().unwrap().print_stats();
    }
    rocket::ignite().mount("/", routes![index]).launch();
}
