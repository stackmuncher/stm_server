use crate::config::Config;
use chrono::Utc;
use futures::stream::{FuturesUnordered, StreamExt};
use serde::{Deserialize, Serialize};
use stackmuncher_lib::report::Report;
use stm_shared::s3;
use tracing::{error, info};

/// A private developer profile with the stack report and some personal info
#[derive(Debug, Serialize)]
pub(crate) struct DevProfile {
    pub owner_id: String,
    pub updated_at: String,
    #[serde(skip_deserializing)]
    pub report: Option<Report>,
}

/// Reflects the structure used by GitHub API.
/// TODO: copied from stm-gh project. Must be shared.
#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct GitHubUser {
    pub login: String,
    pub id: i32,
    pub node_id: String,
    pub avatar_url: Option<String>,
    pub name: Option<String>,
    pub company: Option<String>,
    pub blog: Option<String>,
    pub location: Option<String>,
    pub email: Option<String>,
    pub hireable: Option<bool>,
    pub bio: Option<String>,
    pub twitter_username: Option<String>,
    pub public_repos: i32,
    pub public_gists: i32,
    pub followers: i32,
    pub following: i32,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip_deserializing)]
    pub report: Option<Report>,
}

impl GitHubUser {
    /// Loads itself from an S3 file
    pub(crate) async fn from_s3(config: &Config, s3_key: String) -> Result<Self, ()> {
        // try to get the file from S3
        let profile = match s3::get_text_from_s3(&config.s3_client(), &config.s3_bucket_gh_reports, s3_key, true).await
        {
            Ok(v) => v.0,
            Err(_) => {
                return Err(());
            }
        };
        // convert bytes into struct
        match serde_json::from_slice::<GitHubUser>(profile.as_slice()) {
            Ok(v) => Ok(v),
            Err(e) => {
                error!("Cannot convert GitHubUser into struct {}", e);
                Err(())
            }
        }
    }

    /// Returns a serialized form of Self.
    /// All errors are fatal. Do not retry with the same data.
    pub(crate) fn to_vec(&self) -> Result<Vec<u8>, ()> {
        // convert into json
        match serde_json::to_vec::<Self>(&self) {
            Err(_) => {
                error!("Cannot serialize GitHubUser profile.");
                Err(())
            }
            Ok(v) => Ok(v),
        }
    }
}

impl DevProfile {
    /// Returns a serialized form of Self.
    /// All errors are fatal. Do not retry with the same data.
    pub(crate) fn to_vec(&self) -> Result<Vec<u8>, ()> {
        // convert into json
        match serde_json::to_vec::<Self>(&self) {
            Err(_) => {
                error!("Cannot serialize dev profile.");
                Err(())
            }
            Ok(v) => Ok(v),
        }
    }

    /// Returns itself with the report embedded
    pub(crate) fn new(combined_report: Option<Report>, owner_id: &String) -> Self {
        DevProfile {
            updated_at: Utc::now().to_rfc3339(),
            report: combined_report,
            owner_id: owner_id.clone(),
        }
    }

    /// Merges all project reports from S3 into a single dev report.
    /// All errors are fatal. Do Not Retry with the same data.
    pub(crate) async fn from_contributor_reports(
        private_report_s3_keys: Vec<String>,
        gh_report_s3_keys: Vec<String>,
        config: &Config,
        owner_id: &String,
    ) -> Result<Option<Report>, ()> {
        info!(
            "Merging dev reports into a profile for {}. Private: {}, GH: {}",
            owner_id,
            private_report_s3_keys.len(),
            gh_report_s3_keys.len(),
        );

        // put all the S3 requests into 2 separate futures containers
        let mut private_s3_jobs: FuturesUnordered<_> = private_report_s3_keys
            .into_iter()
            .map(|s3_key| s3::get_text_from_s3(&config.s3_client(), &config.s3_bucket_private_reports, s3_key, true))
            .collect();

        let mut gh_s3_jobs: FuturesUnordered<_> = gh_report_s3_keys
            .into_iter()
            .map(|s3_key| s3::get_text_from_s3(&config.s3_client(), &config.s3_bucket_gh_reports, s3_key, true))
            .collect();

        // both private and GH repo collection should be running by now
        // they need to be separate because there is no other reliable way to tell which report is private and which is from GH
        // not until project matching is implemented

        // collect all GH reports retrieved from S3 as (REPORT , S3_KEY, FROM_GH(true))
        let mut s3_resp: Vec<(Report, String, bool)> = Vec::new();
        loop {
            match gh_s3_jobs.next().await {
                Some(result) => {
                    if let Ok((contents, s3_key)) = result {
                        let report = match serde_json::from_slice::<Report>(contents.as_slice()) {
                            Ok(v) => v,
                            Err(e) => {
                                error!("Cannot convert S3report {} into struct {}", s3_key, e);
                                // something's wrong with the report - skip
                                continue;
                            }
                        };
                        s3_resp.push((report, s3_key, true));
                    };
                }
                None => {
                    // no more jobs left in the futures queue
                    break;
                }
            }
        }

        // collect all private reports into the same container as GH
        loop {
            match private_s3_jobs.next().await {
                Some(result) => {
                    if let Ok((contents, s3_key)) = result {
                        let report = match serde_json::from_slice::<Report>(contents.as_slice()) {
                            Ok(v) => v,
                            Err(e) => {
                                error!("Cannot convert S3report {} into struct {}", s3_key, e);
                                // something's wrong with the report - skip
                                continue;
                            }
                        };
                        s3_resp.push((report, s3_key, false));
                    };
                }
                None => {
                    // no more jobs left in the futures queue
                    break;
                }
            }
        }

        // sort the s3 response so that the latest comes first
        s3_resp.sort_unstable_by(|a, b| b.0.timestamp.cmp(&a.0.timestamp));

        // merge all user gh reports into one
        let mut combined_report: Option<Report> = None;
        for (other_report, s3_key, is_gh) in s3_resp {
            let mut other_report = other_report.abridge();
            if is_gh {
                // gh user and repo name should uniquely identify the project
                other_report.owner_id = Some(owner_id.clone());
                other_report.project_id = None;

                info!(
                    "GH report: {:?}/{:?}, epoch: {:?}",
                    other_report.github_user_name,
                    other_report.github_repo_name,
                    other_report.last_contributor_commit_date_epoch
                );
            } else {
                // add S3 key to the project overview, otherwise there is no way of telling which overview belongs to which project
                // there should be just one project overview in each of these reports because they are project reports
                other_report.owner_id = Some(owner_id.clone());
                other_report.project_id = Some(s3::split_key_into_parts(&s3_key).1);
                other_report.github_user_name = None;
                other_report.github_repo_name = None;
                info!(
                    "Private report: {:?}, epoch: {:?}",
                    other_report.project_id, other_report.last_contributor_commit_date_epoch
                );
            }

            combined_report = Report::merge(combined_report, other_report);
        }

        Ok(combined_report)
    }
}
