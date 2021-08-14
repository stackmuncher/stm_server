use crate::config::Config;
use crate::jobs::FailureType;
use crate::utils;
use chrono::Utc;
use flate2::write::GzEncoder;
use flate2::Compression;
use futures::stream::{FuturesUnordered, StreamExt};
use serde::Serialize;
use stackmuncher_lib::report::Report;
use std::io::prelude::*;
use tracing::{error, info};
use utils::s3;

/// A developer profile with the stack report and some personal info
#[derive(Debug, Serialize)]
pub(crate) struct DevProfile {
    pub owner_id: String,
    pub name: Option<String>,
    pub blog: Option<String>,
    pub email: Option<String>,
    pub updated_at: String,
    #[serde(skip_deserializing)]
    pub report: Option<Report>,
}

impl DevProfile {
    /// Returns a serialized form of Self.
    pub(crate) fn to_vec(&self) -> Result<Vec<u8>, FailureType<String>> {
        // convert into json
        match serde_json::to_vec::<Self>(&self) {
            Err(_) => {
                error!("Cannot serialize dev profile.");
                Err(FailureType::DoNotRetry(self.owner_id.clone()))
            }
            Ok(v) => Ok(v),
        }
    }

    /// Merges all project reports from S3 into a single dev report, extracts the latest personal details and returns a complete developer profile
    pub(crate) async fn from_contributor_reports(
        report_s3_keys: Vec<String>,
        config: &Config,
        owner_id: &String,
    ) -> Result<Self, FailureType<String>> {
        info!("Merging {} dev reports into a profile for {}", report_s3_keys.len(), owner_id);

        // put all the S3 requests into one futures container
        let mut s3_jobs: FuturesUnordered<_> = report_s3_keys
            .into_iter()
            .map(|s3_key| s3::get_text_from_s3(&config.s3_client(), &config.s3_bucket_private_reports, s3_key, true))
            .collect();

        // a container for a list of reports as raw bytes retrieved from S3
        let mut s3_resp: Vec<(Vec<u8>, String)> = Vec::new();
        loop {
            match s3_jobs.next().await {
                Some(result) => {
                    if let Ok((contents, s3_key)) = result {
                        s3_resp.push((contents, s3_key));
                    };
                }
                None => {
                    // no more jobs left in the futures queue
                    break;
                }
            }
        }

        // merge all user reports into one
        let mut combined_report: Option<Report> = None;
        for (report, s3_key) in s3_resp {
            match serde_json::from_slice::<Report>(report.as_slice()) {
                Ok(other_report) => {
                    let mut other_report = other_report.abridge();
                    // add S3 key to the project overview, otherwise there is no way of telling which overview belongs to which project
                    // there should be just one project overview in each of these reports because they are project reports
                    other_report.owner_id = Some(owner_id.clone());
                    other_report.project_id = Some(s3::split_key_into_parts(&s3_key).1);
                    other_report.github_user_name = None;
                    other_report.github_repo_name = None;
                    info!(
                        "Project ID: {:?}, epoch: {:?}",
                        other_report.project_id, other_report.last_contributor_commit_date_epoch
                    );
                    combined_report = Report::merge(combined_report, other_report);
                }
                Err(e) => {
                    error!("Cannot convert S3report into struct {}", e);
                    // it would be good to know which report that is, but it's too big to log
                }
            }
        }

        // it's possible there are no reports in the user struct
        if combined_report.is_none() {
            error!("No merged report was produced.");
            return Err(FailureType::DoNotRetry(owner_id.clone()));
        }

        let dev_profile = DevProfile {
            updated_at: Utc::now().to_rfc3339(),
            name: combined_report.as_ref().unwrap().public_name.clone(),
            blog: combined_report.as_ref().unwrap().public_contact.clone(),
            report: combined_report,
            email: None,
            owner_id: owner_id.clone(),
        };

        Ok(dev_profile)
    }

    /// GZips the json form of the profile and saves it in S3 using a fixed file name in the owner's folder.
    pub(crate) async fn save_in_s3(
        &self,
        config: &Config,
        serialized_profile: &Vec<u8>,
    ) -> Result<(), FailureType<String>> {
        let s3_key = [
            s3::S3_FOLDER_DEV_REPORTS,
            "/",
            &self.owner_id,
            "/",
            s3::S3_DEV_PROFILE_FILE_NAME,
        ]
        .concat();

        // gzip the json
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        if let Err(e) = encoder.write_all(&serialized_profile) {
            error!("Cannot gzip the profile due to {}", e);
            return Err(FailureType::DoNotRetry(self.owner_id.clone()));
        };
        let profile_json = match encoder.finish() {
            Err(e) => {
                error!("Cannot finish gzipping the profile due to {}", e);
                return Err(FailureType::DoNotRetry(self.owner_id.clone()));
            }

            Ok(v) => v,
        };

        // write the report details to S3
        if s3::upload_to_s3(&config.s3_client(), &config.s3_bucket_private_reports, s3_key, profile_json)
            .await
            .is_err()
        {
            return Err(FailureType::Retry(self.owner_id.clone()));
        };

        info!("Upload completed");

        Ok(())
    }
}
