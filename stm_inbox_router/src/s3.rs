use crate::config::Config;
use chrono::Utc;
use futures_util::stream::TryStreamExt;
use lambda_runtime::Error;
use rusoto_s3::{GetObjectRequest, PutObjectRequest, S3};
use serde::Deserialize;
use tracing::{error, info};

/// Reuses the existing S3 client and calls `put_object` for the provided payload and config.
/// The reports are stored under `timestamp_pubkey.json`
/// They are just dumped there as fast as possible for later processing.
pub(crate) async fn upload_to_s3(config: &Config, report_bytes: Vec<u8>, pub_key: String) {
    // the public key is definitely a base58 string because it was decoded for signature validation,
    // so it's safe to be used in the object name as-is
    let report_name = [Utc::now().timestamp().to_string(), pub_key].join("_");

    // the resulting key looks like `queue/1621680890_7prBWD7pzYk2czeXZeXzjxjDQbnuka2RLShdW5AxWuk7.json`
    let s3_key: String = [&config.s3_inbox_prefix, "/", &report_name, ".gzip"].concat();

    info!("Uploading to S3 {}", s3_key);
    if let Err(e) = config
        .s3_client
        .put_object(PutObjectRequest {
            bucket: config.s3_inbox_bucket.clone(),
            key: s3_key.clone(),
            body: Some(report_bytes.into()),
            ..Default::default()
        })
        .await
    {
        error!("Uploading failed for {} with {}", s3_key, e);
        return;
    }
}

/// Return the contents of the object as non-empty String, otherwise return an error.
/// An empty object is an error.
/// * *missing_is_error*: set to true if the object must exist to log an ERROR if it's missing, otherwise it will log it as INFO
pub(crate) async fn get_bytes_from_s3(config: &Config, s3_key: String) -> Result<Vec<u8>, Error> {
    info!("Getting S3 object {}", s3_key);

    let s3_resp = match config
        .s3_client
        .get_object(GetObjectRequest {
            bucket: config.s3_inbox_bucket.clone(),
            key: s3_key.clone(),
            ..Default::default()
        })
        .await
    {
        Err(e) => {
            return Err(Error::from(e));
        }
        Ok(v) => v,
    };

    // try to extract a valid string from the response
    if let Some(s3_object) = s3_resp.body {
        // this step is required because we'll need a stream with Read trait, but it is not implemented in ByteStream
        // there may be a better way of converting it into a stream
        if let Ok(data) = s3_object.map_ok(|b| b.to_vec()).try_concat().await {
            if data.len() == 0 {
                return Err(Error::from("Zero length object."));
            }

            return Ok(data);
        }
    };

    Err(Error::from("Failed to get object contents."))
}

/// `S3Event` which wrap an array of `S3Event`Record
#[derive(Deserialize)]
pub(crate) struct S3Event {
    #[serde(rename = "Records")]
    pub records: Vec<S3EventRecord>,
}

/// `S3EventRecord` which wrap record data
#[derive(Deserialize)]
pub(crate) struct S3EventRecord {
    #[serde(rename = "eventSource")]
    pub event_source: Option<String>,
    #[serde(rename = "awsRegion")]
    pub aws_region: Option<String>,
    pub s3: S3Entity,
}

#[derive(Deserialize)]
pub(crate) struct S3Entity {
    pub bucket: S3Bucket,
    pub object: S3Object,
}

#[derive(Deserialize)]
pub(crate) struct S3Bucket {
    /// The bucket name, e.g. `stm-subs-kb2qskfheu`
    pub name: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct S3Object {
    /// S3 key, ex bucket name, e.g. `queue/1627801778_9PdHabyyhf4KhHAE1SqdpnbAZEXTHhpkermwfPQcLeFK.gzip`
    pub key: Option<String>,
    /// The object size in bytes, e.g. 7172
    pub size: Option<i64>,
}
