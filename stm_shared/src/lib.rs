use regex::Regex;
use tracing::{error, info, warn};

pub mod elastic;
pub mod pgsql;
pub mod s3;

/// Logs the body as error!(), if possible.
pub fn log_http_body(body_bytes: &hyper::body::Bytes) {
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

/// Logs and error and returns false if `gh_login` is empty or has any characters outside of the allowed range.
/// Otherwise returns true.
pub fn validate_gh_login_format(gh_login: &String, gh_login_invalidation_regex: &Regex) -> bool {
    // check if the login is save, even if we got it from GH
    if gh_login.is_empty() || gh_login.len() > 150 || gh_login_invalidation_regex.is_match(gh_login) {
        error!("Invalid GitHub Login format: {}", gh_login);
        false
    } else {
        true
    }
}

/// Returns TRUE if the owner_id decodes from base58 into exactly 256 bytes.
/// Logs a warning and returns FALSE otherwise.
/// TODO: this should be a shared utility function!!!
pub fn validate_owner_id(owner_id: &str) -> bool {
    match bs58::decode(owner_id).into_vec() {
        Err(e) => {
            warn!("Invalid owner_id: {}. Cannot decode from bs58: {}", owner_id, e);
            false
        }
        Ok(v) => {
            if v.len() == 32 {
                true
            } else {
                warn!("Invalid owner_id: {}. Decoded to {} bytes", owner_id, v.len());
                false
            }
        }
    }
}
