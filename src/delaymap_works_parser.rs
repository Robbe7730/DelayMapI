use crate::delaymap_works::DelayMapWorks;
use crate::delaymap_works::DelayMapURL;

use std::fmt;
use std::sync::RwLockReadGuard;

use serde::ser::StdError;

use gtfs_structures::Gtfs;

#[derive(Debug)]
pub struct DelayMapWorksParserError {
    message: String
}

impl fmt::Display for DelayMapWorksParserError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Could not parse Works: {}", self.message)
    }
}

impl StdError for DelayMapWorksParserError {}

pub struct DelayMapWorksParser {
    language: String,
    lines: Vec<String>,
    line_i: usize,
}

impl DelayMapWorksParser {
    pub fn new(
        language: Option<String>,
        content: String
    ) -> Self {
        Self {
            language: language.unwrap_or("en".to_string()),
            lines: content.clone().lines().map(|x| x.to_string()).collect(),
            line_i: 0
        }
    }

    pub fn next_line(&mut self) -> Option<String> {
        self.line_i += 1;
        self.lines.get(self.line_i - 1).map(|x| x.to_string())
    }

    pub fn parse_next(&mut self, gtfs: RwLockReadGuard<Gtfs>) -> Result<Option<DelayMapWorks>, DelayMapWorksParserError> {
        let mut maybe_line = self.next_line();

        if maybe_line == Some("himmessages=[".to_string()) || maybe_line == Some("\"himmessages\"=[".to_string()) {
            maybe_line = self.next_line();
        }

        if maybe_line == Some("]".to_string()) || maybe_line == Some("];".to_string()) {
            // No more messages
            return Ok(None);
        }

        if maybe_line != Some("{".to_string()) && maybe_line != Some(",{".to_string()) {
            return Err(DelayMapWorksParserError {
                message: format!("Invalid first line {:?}", maybe_line)
            })
        }

        let mut ret = DelayMapWorks::empty();

        while maybe_line != Some("}".to_string()) && maybe_line != None {
            let line = maybe_line.unwrap();
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
                    "id" => ret.id = value.to_string(),
                    "caption" => ret.name = value.to_string(),
                    "message" => ret.message = value.to_string(),
                    "pubstartdate_0" => ret.start_date = value.to_string(),
                    "pubstarttime_0" => ret.start_time = value.to_string(),
                    "pubenddate_0" => ret.end_date = value.to_string(),
                    "pubendtime_0" => ret.end_time = value.to_string(),
                    "impactstation_extId" => ret.impacted_station =
                            gtfs.get_stop_translated(value, &self.language)
                                .map(|stop| stop.into())
                                .ok(),
                    "urllist" => {
                        ret.urls = self.parse_urllist(line);
                    },
                    _ => {}
                }
            }
            maybe_line = self.next_line();
        }

        Ok(Some(ret))
    }

    fn parse_urllist(&mut self, line: String) -> Vec<DelayMapURL> {
        let mut urls = vec!();
        let mut urlline = line;
        let mut curr_url = DelayMapURL {
            label: "Link".to_string(),
            url: "#".to_string(),
        };
        while urlline != "]" {
            urlline = self.next_line().unwrap();
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
        urls
    }
}

#[cfg(test)]
mod tests {
    use std::sync::RwLock;
    use std::sync::Arc;

    use gtfs_structures::Stop;

    use lazy_static::lazy_static;

    lazy_static! {
        static ref GTFS: RwLock<Gtfs> = {
            let mut gtfs = Gtfs::default();
            gtfs.stops.insert("8831807".to_string(), Arc::new({
                let mut stop = Stop::default();
                stop.name = "Sint-Truiden".to_string();
                stop.latitude = Some(50.81762);
                stop.longitude = Some(5.17665);
                stop.id = "8831807".to_string();
                stop
            }));
            gtfs.stops.insert("8811445".to_string(), Arc::new({
                let mut stop = Stop::default();
                stop.name = "Groenendaal".to_string();
                stop.latitude = Some(50.7661);
                stop.longitude = Some(4.44949);
                stop.id = "8811445".to_string();
                stop
            }));
            RwLock::new(gtfs)
        };
    }

    use super::*;

    #[test]
    fn test_empty() {
        let content = "himmessages=[
]".to_string();
        let mut parser = DelayMapWorksParser::new(Some("en".to_string()), content);
        let res = parser.parse_next(GTFS.read().unwrap());
        assert!(res.is_ok());
        assert!(res.unwrap().is_none());
    }

    #[test]
    fn test_two_messages() {
        // These are 2 real messages from 19/06/2022
        let content = "{
\"id\":\"63381\"
,\"caption\":\"Landen - Sint-Truiden: Personen in de nabijheid van de sporen.\"
,\"lead\":\"OLGPA[56736]\"
,\"message\":\"De treinen rijden opnieuw normaal.<br />\"
,\"urllist\":[
]
,\"priority\":\"25\"
,\"loctype\":\"0\"
,\"status\":\"1\"
,\"startstation_extId\":\"8833605\"
,\"startstation_puic\":\"80\"
,\"endstation_extId\":\"8831807\"
,\"endstation_puic\":\"80\"
,\"impactstation_extId\":\"8831807\"
,\"impactstation_puic\":\"80\"
,\"startdate\":\"19.06.22\"
,\"starttime\":\"14:48\"
,\"enddate\":\"19.06.22\"
,\"endtime\":\"23:59\"
,\"tstart\":\"00:00\"
,\"tend\":\"23:59\"
,\"pubstartdate_0\":\"19.06.22\"
,\"pubstarttime_0\":\"14:48\"
,\"pubenddate_0\":\"19.06.22\"
,\"pubendtime_0\":\"23:59\"
,\"pubstartdate_3\":\"19.06.22\"
,\"pubstarttime_3\":\"17:15\"
,\"pubenddate_3\":\"19.06.22\"
,\"pubendtime_3\":\"23:59\"
}
,{
\"id\":\"63382\"
,\"caption\":\"Bosvoorde - Groenendaal: Storing aan de seinen.\"
,\"lead\":\"OLGPA[56737]\"
,\"message\":\"Tussen Bosvoorde en Groenendaal:<br />Vertragingen zijn mogelijk.<br />Onbepaalde duur van de storing.<br />Luister naar de aankondigingen, raadpleeg de infoschermen of plan uw reis via de NMBS-app of nmbs.be voor meer info.\"
,\"urllist\":[
]
,\"priority\":\"25\"
,\"loctype\":\"0\"
,\"status\":\"1\"
,\"startstation_extId\":\"8811437\"
,\"startstation_puic\":\"80\"
,\"endstation_extId\":\"8811445\"
,\"endstation_puic\":\"80\"
,\"impactstation_extId\":\"8811445\"
,\"impactstation_puic\":\"80\"
,\"startdate\":\"19.06.22\"
,\"starttime\":\"16:00\"
,\"enddate\":\"19.06.22\"
,\"endtime\":\"23:59\"
,\"tstart\":\"00:00\"
,\"tend\":\"23:59\"
,\"pubstartdate_0\":\"19.06.22\"
,\"pubstarttime_0\":\"16:00\"
,\"pubenddate_0\":\"19.06.22\"
,\"pubendtime_0\":\"23:59\"
,\"pubstartdate_3\":\"19.06.22\"
,\"pubstarttime_3\":\"17:15\"
,\"pubenddate_3\":\"19.06.22\"
,\"pubendtime_3\":\"23:59\"
}".to_string();
        let mut parser = DelayMapWorksParser::new(Some("en".to_string()), content);

        let res = parser.parse_next(GTFS.read().unwrap());
        assert!(res.is_ok());
        let res_unwrapped = res.unwrap();
        assert!(res_unwrapped.is_some());
        let first_message = res_unwrapped.unwrap();

        assert_eq!(first_message.id, "63381".to_string());
        assert_eq!(first_message.name, "Landen - Sint-Truiden: Personen in de nabijheid van de sporen.".to_string());
        assert_eq!(first_message.message, "De treinen rijden opnieuw normaal.<br />");
        assert!(first_message.impacted_station.is_some());
        assert_eq!(first_message.impacted_station.unwrap().name, "Sint-Truiden".to_string());
        assert_eq!(first_message.start_date, "19.06.22".to_string());
        assert_eq!(first_message.end_date, "19.06.22".to_string());
        assert_eq!(first_message.start_time, "14:48".to_string());
        assert_eq!(first_message.end_time, "23:59".to_string());
        assert!(first_message.urls.is_empty());

        let res2 = parser.parse_next(GTFS.read().unwrap());
        assert!(res2.is_ok());
        let res2_unwrapped = res2.unwrap();
        assert!(res2_unwrapped.is_some());
        let second_message = res2_unwrapped.unwrap();

        assert_eq!(second_message.id, "63382".to_string());
        assert_eq!(second_message.name, "Bosvoorde - Groenendaal: Storing aan de seinen.".to_string());
        assert_eq!(second_message.message, "Tussen Bosvoorde en Groenendaal:<br />Vertragingen zijn mogelijk.<br />Onbepaalde duur van de storing.<br />Luister naar de aankondigingen, raadpleeg de infoschermen of plan uw reis via de NMBS-app of nmbs.be voor meer info.");
        assert!(second_message.impacted_station.is_some());
        assert_eq!(second_message.impacted_station.unwrap().name, "Groenendaal".to_string());
        assert_eq!(second_message.start_date, "19.06.22".to_string());
        assert_eq!(second_message.end_date, "19.06.22".to_string());
        assert_eq!(second_message.start_time, "16:00".to_string());
        assert_eq!(second_message.end_time, "23:59".to_string());
        assert!(second_message.urls.is_empty());
    }

    #[test]
    fn test_urls() {
        // This is custom, as I don't have real examples at the time of writing
        let content="himmessages=[
{
\"id\":\"1\"
,\"urllist\":[
{
\"url\":\"http://example.com/\"
,\"label\":\"example\"
}
,{
\"url\":\"https://delaymap.robbevanherck.be/\"
,\"label\":\"DelayMap\"
}
]
}
]".to_string();
        let mut parser = DelayMapWorksParser::new(Some("en".to_string()), content);

        let res = parser.parse_next(GTFS.read().unwrap());
        assert!(res.is_ok());
        let res_unwrapped = res.unwrap();
        assert!(res_unwrapped.is_some());
        let first_message = res_unwrapped.unwrap();

        assert_eq!(first_message.id, "1".to_string());
        assert_eq!(first_message.urls.len(), 2);

        let first_url = &first_message.urls[0];
        let second_url = &first_message.urls[1];

        assert_eq!(first_url.url, "http://example.com/".to_string());
        assert_eq!(second_url.url, "https://delaymap.robbevanherck.be/".to_string());

        assert_eq!(first_url.label, "example");
        assert_eq!(second_url.label, "DelayMap");
    }
}
