use crate::delaymap_stop::DelayMapStop;

use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DelayMapWorks {
    pub id: String,
    pub name: String,
    pub message: String,
    pub impacted_station: Option<DelayMapStop>,
    pub start_date: String,
    pub end_date: String,
    pub start_time: String,
    pub end_time: String,
    pub urls: Vec<DelayMapURL>,
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
pub struct DelayMapURL {
    pub url: String,
    pub label: String,
}
