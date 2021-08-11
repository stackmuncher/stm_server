use tracing::{error, info};

pub(crate) mod elastic;
pub(crate) mod pgsql;
pub(crate) mod s3;

/// Logs the body as error!(), if possible.
pub(crate) fn log_http_body(body_bytes: &hyper::body::Bytes) {
    if body_bytes.is_empty() {
        error!("Empty response body.");
        return;
    }

    // log the body as-is if it's not too long
    if body_bytes.len() < 3000 {
        let s = match std::str::from_utf8(&body_bytes).to_owned() {
            Err(_e) => "The body is not UTF-8".to_string(),
            Ok(v) => v.to_string(),
        };
        info!("Response body: {}", s);
    } else {
        info!("Response is too long to log: {}B", body_bytes.len());
    }
}
