use crate::html::html_data::HtmlData;
use chrono::Utc;
use rusoto_sqs::SqsClient;
use std::convert::From;
pub(crate) use stm_shared::elastic::types::SearchLog;
use stm_shared::elastic::types::{ESSource, ESSourceDev};
use tracing::error;

const HEADER_SOURCE_IP: &str = "x-forwarded-for";

/// Converts itself into a string and sends it to the specified queue.
/// Logs any errors on failure.
pub(crate) async fn send_to_sqs(search_log: &SearchLog, sqs_client: &SqsClient, sqs_queue_url: &String) {
    match serde_json::to_string(search_log) {
        Ok(payload) => {
            let _ = stm_shared::sqs::send(sqs_client, payload, sqs_queue_url).await;
        }
        Err(e) => {
            error!("Failed to serialize SearchLog: {}", e);
        }
    }
}

impl From<&HtmlData> for SearchLog {
    /// Extracts data needed for logging from `html_data`.
    /// Logs an error and returns what it can on error.
    fn from<'b>(html_data: &HtmlData) -> Self {
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
            lang: html_data.langs.iter().map(|(l, _)| l.clone()).collect::<Vec<String>>(),
            ip: source_ip,
            ts: html_data.timestamp.timestamp(),
            dur: Utc::now().timestamp_millis() - html_data.timestamp.timestamp_millis(),
            availability_tz: html_data.availability_tz.clone(),
            availability_tz_hrs: html_data.availability_tz_hrs,
            page_num: html_data.page_number,
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
