use crate::html::html_data::HtmlData;
use chrono::Utc;
use serde::Serialize;
use stm_shared::elastic::types::{ESSource, ESSourceDev};
use tracing::error;

/// A container for search results stats
#[derive(Serialize)]
pub(crate) struct SearchLog {
    /// The raw search string as entered by the user
    pub raw: String,
    /// Same as availability_tz in html_data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub availability_tz: Option<String>,
    /// Same as availability_tz_hrs in html_data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub availability_tz_hrs: Option<usize>,
    /// List of keywords extracted from the raw search
    pub kw: Vec<String>,
    /// A list of search terms matching known languages
    pub lang: Vec<String>,
    /// Source IP address
    pub ip: Option<String>,
    /// EPOCH of the timestamp
    pub ts: i64,
    /// Duration of the request in ms
    pub dur: i64,
    /// List of GH logins found in the response
    pub gh_logins: Vec<String>,
}

const HEADER_SOURCE_IP: &str = "x-forwarded-for";

impl SearchLog {
    /// Extracts data needed for logging from `html_data`.
    /// Logs an error and returns what it can on error.
    pub(crate) fn from_html_data<'b>(html_data: &HtmlData) -> Self {
        // prepare a no-results response
        let source_ip = match html_data.headers.get(HEADER_SOURCE_IP) {
            Some(v) => {
                // this header usually looks like "x-forwarded-for": "5.45.207.142, 64.252.75.128"
                // where the 2nd IP is the CF endpoint
                if let Some((part_1, _)) = v.split_once(",") {
                    // return the 2nd part - the viewer's IP
                    Some(part_1.trim().to_owned())
                } else {
                    // this should not be happening
                    Some(v.to_owned())
                }
            }
            // this should definitely not be happening
            None => None,
        };

        let search_log = SearchLog {
            gh_logins: Vec::new(),
            raw: html_data.raw_search.clone(),
            kw: html_data.keywords.clone(),
            lang: html_data.langs.clone(),
            ip: source_ip,
            ts: html_data.timestamp.timestamp(),
            dur: Utc::now().timestamp_millis() - html_data.timestamp.timestamp_millis(),
            availability_tz: html_data.availability_tz.clone(),
            availability_tz_hrs: html_data.availability_tz_hrs,
        };

        // get the list of dev logins from the ES response
        let gh_logins = match html_data.devs.as_ref() {
            Some(v) => v.clone(),
            None => {
                return search_log;
            }
        };

        let gh_logins = match serde_json::from_value::<ESSource<ESSourceDev>>(gh_logins) {
            Ok(v) => v,
            Err(e) => {
                error!("Cannot deser ES response: {}", e);
                return search_log;
            }
        };

        let gh_logins = gh_logins
            .hits
            .hits
            .into_iter()
            .filter_map(|es_hit| es_hit.source.login)
            .collect::<Vec<String>>();

        SearchLog {
            gh_logins,
            ..search_log
        }
    }
}
