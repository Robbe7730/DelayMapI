use gtfs_structures::Stop;
use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DelayMapStop {
    name: String,
    lat: Option<f64>,
    lon: Option<f64>,
    stop_id: String,
}

impl From<Stop> for DelayMapStop {
    fn from(stop: Stop) -> Self {
        DelayMapStop {
            name: stop.name.clone(),
            lat: stop.latitude,
            lon: stop.longitude,
            stop_id: stop.id.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stop() {
        let mut stop = Stop::default();
        stop.name = "Stop 1".to_string();
        stop.latitude = Some(1.2);
        stop.longitude = Some(3.4);
        stop.id = "stop1".to_string();

        let delaymap_stop: DelayMapStop = stop.into();

        assert_eq!(delaymap_stop.name, "Stop 1".to_string());
        assert_eq!(delaymap_stop.lat, Some(1.2));
        assert_eq!(delaymap_stop.lon, Some(3.4));
        assert_eq!(delaymap_stop.stop_id, "stop1".to_string());
    }
}
