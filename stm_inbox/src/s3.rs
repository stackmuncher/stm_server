use crate::config::Config;
use chrono::Utc;
use rusoto_s3::{PutObjectRequest, S3};
use tracing::{error, info};

/// Reuses the existing S3 client and calls `put_object` for the provided payload and config.
/// The reports are stored under `timestamp_pubkey.json`
/// They are just dumped there as fast as possible for later processing.
pub(crate) async fn upload_to_s3(config: &Config, report_bytes: Vec<u8>, pub_key: String) {
    // the public key is definitely a base58 string because it was decoded for signature validation,
    // so it's safe to be used in the object name as-is
    let report_name = [Utc::now().timestamp().to_string(), pub_key].join("_");

    // the resulting key looks like `queue/1621680890_7prBWD7pzYk2czeXZeXzjxjDQbnuka2RLShdW5AxWuk7.gzip`
    let s3_key: String = [&config.s3_prefix, "/", &report_name, ".gzip"].concat();

    info!("Uploading to S3 {}", s3_key);
    if let Err(e) = config
        .s3_client
        .put_object(PutObjectRequest {
            bucket: config.s3_bucket.clone(),
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
