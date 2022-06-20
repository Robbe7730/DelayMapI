use crate::Delay;
use gtfs_structures::StopTime;

use serde::Serialize;

// Arrival_delay and _timestamp are only allowed to be None at the first station
// Departure_delay and _timestamp are only allowed to be None at the last station
#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DelayMapStopTime {
    pub name: String,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub arrival_delay: Option<i32>,
    pub arrival_timestamp: Option<u32>,
    pub departure_delay: Option<i32>,
    pub departure_timestamp: Option<u32>,
    pub stop_id: String,
}

impl DelayMapStopTime {
    pub fn from_gtfs(stoptime: &StopTime, delay: &Delay) -> DelayMapStopTime {
        DelayMapStopTime {
            name: stoptime.stop.name.clone(),
            lat: stoptime.stop.latitude,
            lon: stoptime.stop.longitude,
            arrival_delay: delay.arrival_delay,
            arrival_timestamp: stoptime.arrival_time,
            departure_delay: delay.departure_delay,
            departure_timestamp: stoptime.departure_time,
            stop_id: stoptime.stop.id.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use gtfs_structures::Stop;
    use std::sync::Arc;

    use super::*;

    #[test]
    fn test_from_gtfs_full() {
        let mut stoptime = StopTime::default();
        let mut stop = Stop::default();
        stop.name = "Stop 1".to_string();
        stop.latitude = Some(13.37);
        stop.longitude = Some(4.20);
        stop.id = "stop1_id".to_string();
        stoptime.stop = Arc::new(stop);
        stoptime.arrival_time = Some(123);
        stoptime.departure_time = Some(456);

        let delay = Delay {
            arrival_delay: Some(12),
            departure_delay: Some(34),
        };
        let dm_stoptime = DelayMapStopTime::from_gtfs(&stoptime, &delay);

        assert_eq!(dm_stoptime.name, "Stop 1");
        assert_eq!(dm_stoptime.lat, Some(13.37));
        assert_eq!(dm_stoptime.lon, Some(4.20));
        assert_eq!(dm_stoptime.arrival_delay, Some(12));
        assert_eq!(dm_stoptime.arrival_timestamp, Some(123));
        assert_eq!(dm_stoptime.departure_delay, Some(34));
        assert_eq!(dm_stoptime.departure_timestamp, Some(456));
        assert_eq!(dm_stoptime.stop_id, "stop1_id");
    }

    #[test]
    fn test_from_gtfs_empty() {
        let stoptime = StopTime::default();
        let delay = Delay {
            arrival_delay: None,
            departure_delay: None,
        };
        let dm_stoptime = DelayMapStopTime::from_gtfs(&stoptime, &delay);

        assert_eq!(dm_stoptime.name, "");
        assert_eq!(dm_stoptime.lat, None);
        assert_eq!(dm_stoptime.lon, None);
        assert_eq!(dm_stoptime.arrival_delay, None);
        assert_eq!(dm_stoptime.arrival_timestamp, None);
        assert_eq!(dm_stoptime.departure_delay, None);
        assert_eq!(dm_stoptime.departure_timestamp, None);
        assert_eq!(dm_stoptime.stop_id, "");
    }
}
