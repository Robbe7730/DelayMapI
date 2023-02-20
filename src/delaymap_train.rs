use crate::delaymap_stop_time::DelayMapStopTime;
use crate::delay::Delay;

use gtfs_structures::Trip;

use std::collections::HashMap;

use chrono::Timelike;
use chrono::TimeZone;
use chrono::Utc;
use chrono_tz::Europe::Brussels;

use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DelayMapTrain {
    pub id: String,
    pub name: String,
    pub stops: Vec<DelayMapStopTime>,
    pub stop_index: usize,
    pub is_stopped: bool,
    pub estimated_lat: f64,
    pub estimated_lon: f64,
}

impl DelayMapTrain {
    pub fn from_gtfs(
        trip: &Trip,
        maybe_delaymap: &Option<HashMap<String, HashMap<String, Delay>>>
    ) -> DelayMapTrain {
        let mut ret = DelayMapTrain {
            id: trip.id.to_string(),
            name: trip
                .trip_headsign
                .clone()
                .unwrap_or("Unknown Train".to_string()),
            stops: vec![],
            stop_index: 0,
            is_stopped: false,
            estimated_lat: 0.0,
            estimated_lon: 0.0,
        };

        let mut curr_delay = Delay {
            arrival_delay: None,
            departure_delay: Some(0),
        };

        let local_datetime = Brussels.from_utc_datetime(&Utc::now().naive_utc());
        let local_timestamp = local_datetime.time().num_seconds_from_midnight() as i64;

        let mut previous_departure = 0;
        let mut previous_stop_lat = 0.0;
        let mut previous_stop_lon = 0.0;
        let last_stop_i = trip.stop_times.len() - 1;

        for (i, stop_time) in trip.stop_times.iter().enumerate() {
            // Apply delay patch
            if let Some(delaymap) = maybe_delaymap {
                if let Some(trip_delaymap) = delaymap.get(&trip.id) {
                    if let Some(delay_patch) = trip_delaymap.get(&stop_time.stop.id) {
                        curr_delay.arrival_delay =
                            delay_patch.arrival_delay.or(curr_delay.arrival_delay);
                        curr_delay.departure_delay =
                            delay_patch.departure_delay.or(curr_delay.departure_delay);
                    }
                }
            }

            // Make sure delays are not None where they shouldn't be
            if i != 0 && curr_delay.arrival_delay.is_none() {
                curr_delay.arrival_delay = Some(0);
            }

            // Having a departure delay at the final stop makes no sense
            if i == last_stop_i {
                curr_delay.departure_delay = None;
            }

            let stop = DelayMapStopTime::from_gtfs(&stop_time, &curr_delay);

            // Calculate arrival and departure time, using dummy values for
            // start and end station. Not inteded to be used in the API
            let actual_arrival = if stop.arrival_timestamp.is_some() && stop.arrival_delay.is_some()
            {
                i64::from(stop.arrival_timestamp.unwrap()) +
                    i64::from(stop.arrival_delay.unwrap())
            } else {
                0
            };
            let actual_departure = if stop.departure_timestamp.is_some() && stop.departure_delay.is_some()
            {
                i64::from(stop.departure_timestamp.unwrap()) +
                    i64::from(stop.departure_delay.unwrap())
            } else {
                i64::max_value()
            };

            // If the train has not left the station and either already arrived,
            // or is still at its first station, it is stopped at that station.
            if actual_departure > local_timestamp && (i == 0 || actual_arrival < local_timestamp) {
                ret.stop_index = i;
                ret.is_stopped = true;
                ret.estimated_lat = stop.lat.unwrap_or(0.0);
                ret.estimated_lon = stop.lon.unwrap_or(0.0);
            // If the train have left the previous station, but has not arrived
            // at the next station, it is riding between these two stations.
            } else if actual_arrival > local_timestamp && previous_departure < local_timestamp {
                ret.stop_index = i;
                ret.is_stopped = false;
                let curr_stop_lat = stop.lat.unwrap_or(0.0);
                let curr_stop_lon = stop.lon.unwrap_or(0.0);

                // Linearly interpolate between the two coordinates.
                let percentage_complete: f64 = ((local_timestamp - previous_departure) as f64)
                    / (actual_arrival - previous_departure) as f64;
                ret.estimated_lat = percentage_complete * curr_stop_lat
                    + (1.0 - percentage_complete) * previous_stop_lat;
                ret.estimated_lon = percentage_complete * curr_stop_lon
                    + (1.0 - percentage_complete) * previous_stop_lon;
            // If the train has arrived at the final station, it is still at
            // that station
        } else if actual_arrival < local_timestamp && i == last_stop_i {
                ret.stop_index = i;
                ret.is_stopped = true;
                ret.estimated_lat = stop.lat.unwrap_or(0.0);
                ret.estimated_lon = stop.lon.unwrap_or(0.0);
            }

            previous_departure = actual_departure;
            previous_stop_lat = stop.lat.unwrap_or(0.0);
            previous_stop_lon = stop.lon.unwrap_or(0.0);
            ret.stops.push(stop);
        }
        ret
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, convert::TryInto};

    use gtfs_structures::{StopTime, Stop};

    use super::*;

    // Current test setup:
    // name, time of arrival-departure (relative), coordinates
    // Stop 1: none-0:00, (0, 0)
    // Stop 2: 1:00-1:05, (3, 3)
    // Stop 3: 2:00-2:10, (3, 6)
    // Stop 4: 3:00-none, (6, 6)

    // ----- NAME/ID TESTS -----
    #[test]
    fn test_trip_id_name() {
        let trip = create_trip(0);
        let delaymap = HashMap::new();
        let train = DelayMapTrain::from_gtfs(&trip, &Some(delaymap));
        assert_eq!(train.name, "My Train".to_string());
        assert_eq!(train.id, "my-train".to_string());
    }

    // ----- INTERPOLATION TESTS -----
    #[test]
    fn test_interpolation_not_started() {
        let trip = create_trip(-10);
        let delaymap = HashMap::new();
        let train = DelayMapTrain::from_gtfs(&trip, &Some(delaymap));
        assert_eq!(train.estimated_lat, 0.0);
        assert_eq!(train.estimated_lon, 0.0);
        assert_eq!(train.is_stopped, true);
        assert_eq!(train.stop_index, 0);
    }

    #[test]
    fn test_interpolation_first_sector() {
        let trip = create_trip(40);
        let delaymap = HashMap::new();
        let train = DelayMapTrain::from_gtfs(&trip, &Some(delaymap));
        assert_eq!(train.estimated_lat, 2.0);
        assert_eq!(train.estimated_lon, 2.0);
        assert_eq!(train.is_stopped, false);
        assert_eq!(train.stop_index, 1);
    }

    #[test]
    fn test_interpolation_in_station() {
        let trip = create_trip(62);
        let delaymap = HashMap::new();
        let train = DelayMapTrain::from_gtfs(&trip, &Some(delaymap));
        assert_eq!(train.estimated_lat, 3.0);
        assert_eq!(train.estimated_lon, 3.0);
        assert_eq!(train.is_stopped, true);
        assert_eq!(train.stop_index, 1);
    }

    #[test]
    fn test_interpolation_last_sector() {
        let trip = create_trip(150);
        let delaymap = HashMap::new();
        let train = DelayMapTrain::from_gtfs(&trip, &Some(delaymap));
        assert_eq!(train.estimated_lat, 4.2);
        assert_eq!(train.estimated_lon, 6.0);
        assert_eq!(train.is_stopped, false);
        assert_eq!(train.stop_index, 3);
    }

    #[test]
    fn test_interpolation_arrived() {
        let trip = create_trip(200);
        let delaymap = HashMap::new();
        let train = DelayMapTrain::from_gtfs(&trip, &Some(delaymap));
        assert_eq!(train.estimated_lat, 6.0);
        assert_eq!(train.estimated_lon, 6.0);
        assert_eq!(train.is_stopped, true);
        assert_eq!(train.stop_index, 3);
    }

    // ----- DELAY TESTS-----
    #[test]
    fn test_delay_none() {
        let trip = create_trip(0);
        let delaymap = HashMap::new();
        let train = DelayMapTrain::from_gtfs(&trip, &Some(delaymap));

        check_delays(
            train,
            trip,
            vec![None, Some(0), Some(0), Some(0)],
            vec![Some(0), Some(0), Some(0), None]
        )
    }

    #[test]
    fn test_delay_some() {
        let trip = create_trip(0);
        let mut trip_delays = HashMap::new();

        trip_delays.insert("stop2".to_string(), Delay {
            arrival_delay: Some(2),
            departure_delay: Some(1),
        });

        // NOTE: NMBS does not do this, they always provide none or both, but
        // DelayMapI should be able to handle this imo
        // We now make the assumption that arrival delay and departure delay are
        // two different delays, so updating one does not affect the other
        trip_delays.insert("stop3".to_string(), Delay {
            arrival_delay: Some(1),
            departure_delay: None,
        });

        let mut delaymap = HashMap::new();
        delaymap.insert("my-train".to_string(), trip_delays);

        let train = DelayMapTrain::from_gtfs(&trip, &Some(delaymap));

        check_delays(
            train,
            trip,
            vec![None, Some(2), Some(1), Some(1)],
            vec![Some(0), Some(1), Some(1), None]
        )
    }

    #[test]
    fn test_delay_begin_end() {
        let trip = create_trip(0);
        let mut trip_delays = HashMap::new();

        trip_delays.insert("stop1".to_string(), Delay {
            arrival_delay: None,
            departure_delay: Some(5),
        });

        trip_delays.insert("stop4".to_string(), Delay {
            arrival_delay: Some(3),
            departure_delay: None,
        });

        let mut delaymap = HashMap::new();
        delaymap.insert("my-train".to_string(), trip_delays);

        let train = DelayMapTrain::from_gtfs(&trip, &Some(delaymap));

        check_delays(
            train,
            trip,
            vec![None, Some(0), Some(0), Some(3)],
            vec![Some(5), Some(5), Some(5), None]
        )
    }

    #[test]
    fn test_delay_negative() {
        let trip = create_trip(0);
        let mut trip_delays = HashMap::new();

        trip_delays.insert("stop1".to_string(), Delay {
            arrival_delay: None,
            departure_delay: Some(0),
        });

        trip_delays.insert("stop2".to_string(), Delay {
            arrival_delay: Some(-5),
            departure_delay: None,
        });

        trip_delays.insert("stop3".to_string(), Delay {
            arrival_delay: Some(0),
            departure_delay: None,
        });

        let mut delaymap = HashMap::new();
        delaymap.insert("my-train".to_string(), trip_delays);

        let train = DelayMapTrain::from_gtfs(&trip, &Some(delaymap));

        check_delays(
            train,
            trip,
            vec![None, Some(-5), Some(0), Some(0)],
            vec![Some(0), Some(0), Some(0), None]
        )
    }

    // ----- INTERPOLATION + DELAY -----
    #[test]
    fn test_interpolation_delay() {
        let trip = create_trip(80);
        let mut trip_delays = HashMap::new();

        trip_delays.insert("stop2".to_string(), Delay {
            arrival_delay: Some(60),
            departure_delay: Some(60),
        });

        let mut delaymap = HashMap::new();
        delaymap.insert("my-train".to_string(), trip_delays);

        let train = DelayMapTrain::from_gtfs(&trip, &Some(delaymap));

        assert_eq!(train.estimated_lat, 2.0);
        assert_eq!(train.estimated_lon, 2.0);
        assert_eq!(train.stop_index, 1);
        assert_eq!(train.is_stopped, false);
    }

    // ----- HELPERS -----
    fn check_delays(
        train: DelayMapTrain,
        trip: Trip,
        expected_arrival_delays: Vec<Option<i32>>,
        expected_departure_delays: Vec<Option<i32>>,
    ) {

        for i in 0..trip.stop_times.len() {
            assert_eq!(train.stops[i].arrival_delay, expected_arrival_delays[i]);
            assert_eq!(train.stops[i].departure_delay, expected_departure_delays[i]);

            assert_eq!(
                train.stops[i].arrival_timestamp,
                trip.stop_times[i].arrival_time
            );
            assert_eq!(
                train.stops[i].departure_timestamp,
                trip.stop_times[i].departure_time
            );
        }
    }

    fn create_stoptime(
        stop_id: &str,
        stop_name: &str,
        stop_lat: Option<f64>,
        stop_lon: Option<f64>,
        arrival_time: Option<u32>,
        departure_time: Option<u32>,
    ) -> StopTime {
        let mut stop = Stop::default();
        stop.name = stop_name.to_string();
        stop.latitude = stop_lat;
        stop.longitude = stop_lon;
        stop.id = stop_id.to_string();

        let mut stoptime = StopTime::default();
        stoptime.arrival_time = arrival_time;
        stoptime.departure_time = departure_time;

        stoptime.stop = Arc::new(stop);
        return stoptime;
    }

    fn create_trip(delta: i32) -> Trip {
        let local_datetime = Brussels.from_utc_datetime(&Utc::now().naive_utc());
        let local_timestamp = local_datetime.time().num_seconds_from_midnight();
        let t_zero: u32 = (local_timestamp as i32 - delta).try_into().unwrap();

        let mut trip = Trip::default();
        trip.trip_headsign = Some("My Train".to_string());
        trip.id = "my-train".to_string();
        trip.stop_times = vec![];
        trip.stop_times.push(create_stoptime(
            "stop1", "Stop 1", Some(0.0), Some(0.0), None, Some(t_zero + 0)
        ));
        trip.stop_times.push(create_stoptime(
            "stop2", "Stop 2", Some(3.0), Some(3.0), Some(t_zero + 60), Some(t_zero + 65)
        ));
        trip.stop_times.push(create_stoptime(
            "stop3", "Stop 3", Some(3.0), Some(6.0), Some(t_zero + 120), Some(t_zero + 130)
        ));
        trip.stop_times.push(create_stoptime(
            "stop4", "Stop 4", Some(6.0), Some(6.0), Some(t_zero + 180), None
        ));
        trip
    }

}
