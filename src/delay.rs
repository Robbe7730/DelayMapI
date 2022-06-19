use serde::Serialize;
use crate::gtfs_realtime::TripUpdate_StopTimeUpdate;

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Delay {
    pub arrival_delay: i32,
    pub departure_delay: i32,
}

impl From<TripUpdate_StopTimeUpdate> for Delay {
    fn from(update: TripUpdate_StopTimeUpdate) -> Delay {
        Delay {
            departure_delay: update.departure.into_option().map(|x| x.get_delay()).unwrap_or(0),
            arrival_delay: update.arrival.into_option().map(|x| x.get_delay()).unwrap_or(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::gtfs_realtime::TripUpdate_StopTimeEvent;

    use super::*;

    #[test]
    fn test_from_all_none() {
        let update = TripUpdate_StopTimeUpdate::new();
        let delay = Delay::from(update);
        assert_eq!(delay.arrival_delay, 0);
        assert_eq!(delay.departure_delay, 0);
    }

    #[test]
    fn test_from_some() {
        let mut update = TripUpdate_StopTimeUpdate::new();
        // No delay set = 0 delay
        update.set_arrival(TripUpdate_StopTimeEvent::new());
        let mut departure_delay = TripUpdate_StopTimeEvent::new();
        departure_delay.set_delay(25);
        update.set_departure(departure_delay);

        let delay = Delay::from(update);
        assert_eq!(delay.arrival_delay, 0);
        assert_eq!(delay.departure_delay, 25);
    }
}
