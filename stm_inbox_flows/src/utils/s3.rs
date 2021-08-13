use crate::config::Config;
use flate2::read::GzDecoder;
use futures::stream::TryStreamExt;
use hyper_rustls::HttpsConnector;
use rusoto_core::credential::DefaultCredentialsProvider;
use rusoto_core::HttpClient;
use rusoto_s3::{GetObjectRequest, ListObjectsV2Request, PutObjectRequest, S3Client, S3};
use std::io::Read;
use std::time::Duration;
use tracing::{error, info, warn};

/// An S3 prefix for dev reports organized by owner_id/project_id
pub(crate) const S3_FOLDER_DEV_REPORTS: &str = "reports";
pub(crate) const S3_COMBINED_DEV_REPORT_FILE_NAME: &str = "report.gz";
pub(crate) const S3_DEV_PROFILE_FILE_NAME: &str = "profile.gz";

/// Contains some of the object properties returned by S3 ListObjectV2
/// There are also size, owner and etag props that were not included
#[derive(Clone)]
pub(crate) struct S3ObjectProps {
    pub key: String,
    /// Defaults to the beginning of time 1970-01-01T00:00:00.000Z
    pub last_modified: String,
    /// Size in bytes as reported by S3
    pub size: i64,
}

/// Returns a list of all keys matching the specified prefix. Makes multiple API calls to AWS if the list is longer than 1000 objects.
/// There is no limit on the number of objects, so potentially it can gobble up all the memory.
/// Returns an error if any of the API calls fail.
pub(crate) async fn list_objects_from_s3(
    s3_client: &S3Client,
    s3_bucket: &String,
    s3_key_prefix: String,
    start_after: Option<String>,
) -> Result<Vec<S3ObjectProps>, ()> {
    // a container for the combined list from multiple calls to AWS API
    let mut all_objects: Vec<S3ObjectProps> = Vec::new();
    // hold the last object in the list as the starting point for the next request
    let mut start_after = start_after;

    // keep getting lots of up to 1000 objects until done
    loop {
        if let Ok((s3_objects_lot, is_truncated)) =
            list_up_to_10000_objects_from_s3(s3_client, s3_bucket, &s3_key_prefix, &start_after).await
        {
            // exit if nothing was retrieved
            if s3_objects_lot.is_empty() {
                break;
            }

            // avoid unnecessary move via the iterator and just assign the result to the output container on the first call
            if all_objects.is_empty() {
                all_objects = s3_objects_lot;
            } else {
                // add the result to the total
                all_objects.extend(s3_objects_lot);
            }

            // exit the loop when the end of the list was reached
            if !is_truncated {
                break;
            }

            // there are more objects to list - prepare the starting point for the next call
            start_after = Some(all_objects.last().as_ref().unwrap().key.clone());

            // continue to the next call ...
        } else {
            return Err(());
        }
    }

    Ok(all_objects)
}

/// Returns a list of up to 1000 keys matching the specified prefix and a flag indicating if the result was truncated.
async fn list_up_to_10000_objects_from_s3(
    s3_client: &S3Client,
    s3_bucket: &String,
    s3_key_prefix: &String,
    start_after: &Option<String>,
) -> Result<(Vec<S3ObjectProps>, bool), ()> {
    info!("Getting list of S3 objects for {}", s3_key_prefix);

    let s3_resp = match s3_client
        .list_objects_v2(ListObjectsV2Request {
            bucket: s3_bucket.clone(),
            prefix: Some(s3_key_prefix.clone()),
            start_after: start_after.clone(),
            ..Default::default()
        })
        .await
    {
        Err(e) => {
            error!("Failed to get a list S3 objects in {}. {}", s3_key_prefix, e);
            return Err(());
        }
        Ok(v) => v,
    };

    // the output collector
    let mut key_list: Vec<S3ObjectProps> = Vec::new();

    // loop through the response, if any and collect the keys
    if let Some(s3_objects) = s3_resp.contents {
        info!(
            "Objects found: {}, truncated: {}",
            s3_objects.len(),
            s3_resp.is_truncated.unwrap_or_default()
        );
        for obj in s3_objects {
            if obj.key.is_none() {
                warn!("Empty object key - this should not happen.");
            } else {
                if obj.last_modified.is_none() {
                    warn!("Empty last_modified - this should not happen.");
                }
                key_list.push(S3ObjectProps {
                    key: obj.key.unwrap(),
                    last_modified: obj.last_modified.unwrap_or("1970-01-01T00:00:00.000Z".to_owned()),
                    size: match obj.size {
                        Some(v) => v,
                        None => 0,
                    },
                });
            }
        }
    };

    Ok((key_list, s3_resp.is_truncated.unwrap_or_default()))
}

/// Returns the contents of the object as a non-empty String + it's S3 key, otherwise return an error.
/// An empty object is an error.
/// * *missing_is_error*: set to true if the object must exist to log an ERROR if it's missing, otherwise it will log it as INFO
pub(crate) async fn get_text_from_s3(
    s3_client: &S3Client,
    s3_bucket: &String,
    s3_key: String,
    missing_is_error: bool,
) -> Result<(Vec<u8>, String), ()> {
    info!("Getting S3 object {}", s3_key);

    let s3_resp = match s3_client
        .get_object(GetObjectRequest {
            bucket: s3_bucket.clone(),
            key: s3_key.clone(),
            ..Default::default()
        })
        .await
    {
        Err(e) => {
            if missing_is_error {
                error!("Failed to get S3 object {}. {}", s3_key, e);
            } else {
                info!("Failed to get S3 object {}. {}", s3_key, e);
            }
            return Err(());
        }
        Ok(v) => v,
    };

    // try to extract a valid string from the response
    if let Some(s3_object) = s3_resp.body {
        if let Ok(data) = s3_object.map_ok(|b| b.to_vec()).try_concat().await {
            if data.len() == 0 {
                error!("Zero length object.");
                return Err(());
            }

            // check if the contents are gzipped
            if data.len() > 2 && data[0] == 0x1f && data[1] == 0x8b {
                let mut decoder = GzDecoder::new(data.as_slice());
                let mut buffer: Vec<u8> = Vec::new();
                let len = match decoder.read_to_end(&mut buffer) {
                    Ok(v) => v,
                    Err(e) => {
                        error!("Failed to unzip S3 object {}. {}", s3_key, e);
                        return Err(());
                    }
                };

                info!("Unzipped to {} bytes", len);

                return Ok((buffer, s3_key));
            }

            return Ok((data, s3_key));
        }
    };

    error!("Failed to get object contents.");
    Err(())
}

/// Uploads the payload to S3.
pub(crate) async fn upload_to_s3(
    s3_client: &S3Client,
    s3_bucket: &String,
    s3_key: String,
    payload: Vec<u8>,
) -> Result<(), ()> {
    info!("Uploading to S3: {}", s3_key);
    if let Err(e) = s3_client
        .put_object(PutObjectRequest {
            bucket: s3_bucket.clone(),
            key: s3_key,
            body: Some(payload.into()),
            ..Default::default()
        })
        .await
    {
        error!("Uploading failed: {}", e);
        return Err(());
    }

    Ok(())
}

/// Generates an S3Client with custom settings to match AWS server defaults.
/// AWS times out idle connections after 20s as per https://aws.amazon.com/premiumsupport/knowledge-center/s3-socket-connection-timeout-error/
/// We need to sync the idle time of our client with that setting.
pub(crate) fn generate_s3_client(config: &Config) -> S3Client {
    let https_connector = HttpsConnector::with_native_roots();

    let cred_prov = DefaultCredentialsProvider::new().expect("Cannot unwrap DefaultCredentialsProvider");

    let mut builder = hyper::Client::builder();
    builder.pool_idle_timeout(Duration::from_secs(15));
    builder.http2_keep_alive_interval(Duration::from_secs(5));
    builder.http2_keep_alive_timeout(Duration::from_secs(3));

    let http_client = HttpClient::from_builder(builder, https_connector);

    S3Client::new_with(http_client, cred_prov, config.s3_region.clone())
}

/// Returns an S3 key for the dev with the specified `owner_id` or an Err if the owner id does not match the required format.
/// The key includes a trailing `/` to make sure that the match is exact because `report/abc` will match `report/abc/` and `report/abcd/`.
/// The validation is to enforce zero-trust with other parts of the system,
/// but it is unlikely that the owner_id is invalid because it is validated many times elsewhere.
pub(crate) fn build_dev_s3_key_from_owner_id(config: &Config, owner_id: &String) -> Result<String, ()> {
    // validate the owner id, which should be a base58 encoded string of 44 chars
    if !config
        .owner_id_validation_regex
        .as_ref()
        .expect("Missing owner_id_validation_regex. It's a bug.")
        .is_match(owner_id)
    {
        error!("Invalid owner id: {}", owner_id);
        return Err(());
    }

    Ok([S3_FOLDER_DEV_REPORTS, "/", owner_id, "/"].concat())
}

/// Returns true if the key points at an object in `reports_prefix/owner_id/project_id/combined_report_name`.
/// The combined report name is the same for everyone and comes from `S3_COMBINED_DEV_REPORT_NAME` constant.
/// E.g. `reports/9PdHabyyhf4KhHAE1SqdpnbAZEXTHhpkermwfPQcLeFK/FZ8zezMFji6VXcWEDxckwy/report.gz`
pub(crate) fn is_combined_project_report(s3_key: &String, owner_id: &String) -> bool {
    // trim the ending part of the key as it's the one most likely to differ
    let trimmed_end = s3_key.trim_end_matches(&["/", S3_COMBINED_DEV_REPORT_FILE_NAME].concat());
    // return false if nothing was trimmed - it's obviously not a match
    if trimmed_end.len() == s3_key.len() {
        return false;
    }

    // check if the front part of the key is a match
    let trimmed_front = trimmed_end.trim_start_matches(&[S3_FOLDER_DEV_REPORTS, "/", owner_id, "/"].concat());
    // return false if nothing was trimmed - it's obviously not a match
    if trimmed_front.len() == trimmed_end.len() {
        return false;
    }

    // the only remaining part should be a project ID
    if trimmed_front.contains("/") {
        return false;
    }

    // by exclusion, it must be the right key
    true
}

/// Converts an rfc2822 date used by S3 into a timestamp or returns an error.
/// The date should look like Mon, 15 Oct 2012 21:58:07 GMT.
pub(crate) fn parse_date_header(header: &Option<String>) -> Result<i64, ()> {
    // there is some data re. the object - check `last modified` header.
    if let Some(last_modified) = header {
        match chrono::DateTime::parse_from_rfc3339(&last_modified) {
            Ok(last_mod) => {
                return Ok(last_mod.timestamp());
            }
            Err(e) => {
                error!("Invalid date in last_modified header: {} / {}", last_modified, e);
                return Err(());
            }
        }
    } else {
        error!("last_modified header is missing");
        return Err(());
    }
}

/// Splits the S3 key into _owner_ and _project_ IDs by looking at the S3 key from the end of the string.
/// E.g. `some_prefix/9PdHabyyhf4KhHAE1SqdpnbAZEXTHhpkermwfPQcLeFK/NeYatzas1FrogKLDe2nBG8/1628730164_d6f8b0fea106c94f185ae246a2cd43fac1b1c3b0.gz`
/// -> `9PdHabyyhf4KhHAE1SqdpnbAZEXTHhpkermwfPQcLeFK` and `9PdHabyyhf4KhHAE1SqdpnbAZEXTHhpkermwfPQcLeFK` using PathBuf.
/// #### This only works on full keys that include the object name.
/// # Panics
/// Panics if the string has less than 4 parts: prefix, owner, project and object.
pub(crate) fn split_key_into_parts(s3_key: &String) -> (String, String) {
    let mut parts = s3_key.split("/").collect::<Vec<&str>>();
    if parts.len() < 4 {
        panic!("Invalid S3 key: {}. It's a bug.", s3_key);
    }

    let _file_name = parts
        .pop()
        .expect(&format!("Failed to extract file name from S3 Key: {}. it's a bug", s3_key));
    let project_id = parts
        .pop()
        .expect(&format!("Failed to extract project ID from S3 Key: {}. it's a bug", s3_key));
    let owner_id = parts
        .pop()
        .expect(&format!("Failed to extract owner ID from S3 Key: {}. it's a bug", s3_key));

    (owner_id.to_owned(), project_id.to_owned())
}
