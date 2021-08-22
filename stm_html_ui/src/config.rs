use regex::Regex;
use tracing::warn;

/// Add the name of the ElasticSearch index to that env var
pub const ES_DEV_IDX_ENV: &str = "STM_HTML_ES_DEV_IDX";
/// Add the name of the ElasticSearch index to that env var
pub const ES_STATS_IDX_ENV: &str = "STM_HTML_ES_STATS_IDX";
/// Add the absolute ElasticSearch URL to that env var
pub const ES_URL_ENV: &str = "STM_HTML_ES_URL";

pub struct Config {
    /// Absolute ElasticSearch URL
    pub es_url: String,
    /// Name of `dev` index
    pub dev_idx: String,
    /// Name of `stats` index
    pub stats_idx: String,
    /// No-SQL field value validation regex - the value would be invalid if it's a match
    pub no_sql_string_invalidation_regex: Regex,
}

/// A regex formula to check for unsafe values to insert into another regex string.
/// It is stricter than no_sql_string_invalidation_regex and is to be compiled only in some cases
pub(crate) const SAFE_REGEX_SUBSTRING: &str = r#"[^#\-\._0-9a-zA-Z]"#;

impl Config {
    pub fn new() -> Self {
        Config {
            es_url: std::env::var(ES_URL_ENV)
                .expect(&format!("Missing {} env var with ElasticSearch URL", ES_URL_ENV))
                .trim()
                .trim_end_matches("/")
                .to_string(),
            dev_idx: std::env::var(ES_DEV_IDX_ENV)
                .expect(&format!("Missing {} env var with ES DEV index name", ES_DEV_IDX_ENV))
                .trim()
                .to_string(),
            stats_idx: std::env::var(ES_STATS_IDX_ENV)
                .expect(&format!("Missing {} env var with ES STATS index name", ES_STATS_IDX_ENV))
                .trim()
                .to_string(),
            no_sql_string_invalidation_regex: Regex::new(r#"[^#\-\._0-9a-zA-Z]"#)
                .expect("Failed to compile no_sql_string_value_regex"),
        }
    }
}

/// Returns TRUE if the owner_id decodes from base58 into exactly 256 bytes.
/// Logs a warning and returns FALSE otherwise.
/// TODO: this should be a shared utility function!!!
pub(crate) fn validate_owner_id(owner_id: &str) -> bool {
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