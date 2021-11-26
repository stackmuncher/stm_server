use regex::Regex;
use tracing::warn;

/// Add the name of the ElasticSearch index to that env var
pub const ES_DEV_IDX_ENV: &str = "STM_HTML_ES_DEV_IDX";
/// Add the name of the ElasticSearch index to that env var
pub const ES_STATS_IDX_ENV: &str = "STM_HTML_ES_STATS_IDX";
/// Add the name of the ElasticSearch index to that env var
pub const ES_SEARCH_LOG_IDX_ENV: &str = "STM_HTML_ES_SEARCH_LOG_IDX";
/// Add the absolute ElasticSearch URL to that env var
pub const ES_URL_ENV: &str = "STM_HTML_ES_URL";

pub struct Config {
    /// Absolute ElasticSearch URL
    pub es_url: String,
    /// Name of `dev` index
    pub dev_idx: String,
    /// Name of `stats` index
    pub stats_idx: String,
    /// Name of `search_log` index
    pub search_log_idx: String,
    /// No-SQL field value validation regex - the value would be invalid if it's a match
    pub no_sql_string_invalidation_regex: Regex,
    /// Extracts individual search terms from the raw search string
    pub search_terms_regex: Regex,
    /// Extracts required working hours and the timezone from the search string
    /// E.g. 5utc+00, 5utc-0, 5utc, 5utc+03, 5utc-03
    /// * Capture group 1: hours in the timezone
    /// * Capture group 2: timezone, can be blank for UTC
    pub timezone_terms_regex: Regex,
}

/// A regex formula to extract search terms from the raw search string.
/// #### The extracted string is safe to be used inside another regex
/// The value validated by this string should not contain any chars that may be unsafe inside another regex.
/// Any such chars should be escape when that regex is constructed.
const SEARCH_TERM_REGEX: &str = r#"[#:\-._+0-9a-zA-Z]+"#;

/// A regex formula inverse to `SEARCH_TERM_REGEX` to invalidate anything that has invalid chars.
/// It is a redundant check in case an invalid value slipped past previous checks.
const NO_SQL_STRING_INVALIDATION_REGEX: &str = r#"[^#\-._+0-9a-zA-Z]"#;

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
            search_log_idx: std::env::var(ES_SEARCH_LOG_IDX_ENV)
                .expect(&format!("Missing {} env var with ES SEARCH LOG index name", ES_SEARCH_LOG_IDX_ENV))
                .trim()
                .to_string(),
            no_sql_string_invalidation_regex: Regex::new(NO_SQL_STRING_INVALIDATION_REGEX)
                .expect("Failed to compile no_sql_string_value_regex"),
            search_terms_regex: Regex::new(SEARCH_TERM_REGEX).expect("Failed to compile search_terms_regex"),
            timezone_terms_regex: Regex::new(r#"(?i)(?:[[:space:]]|^)(\d{1,2})(?:hrs@|hr@|h@|@)?utc([\+\-]\d{1,2})?(?:[[:space:]]|$)"#)
                .expect("Failed to compile timezone_terms_regex"),
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

/// This test is more for debugging the regex. It didn't work as expected and does not match the results shown in
/// https://2fd.github.io/rust-regex-playground/, probably due to different crate versions.
#[test]
fn timezone_terms_regex() {
    let config = Config::new();
    let rgx = config.timezone_terms_regex;

    println!("Test");

    let vals = vec![
        ("5utc+03", "5utc+03"),
        ("5utc+03 ", "5utc+03 "),
        (" 5utc+03", " 5utc+03"),
        (" 5utc+03 ", " 5utc+03 "),
        ("rust 5utc+03", " 5utc+03"),
        ("5utc+03 rust", "5utc+03 "),
        ("rust 5utc+03 rust", " 5utc+03 "),
        ("5utc+03a", ""),
        ("a5utc+03", ""),
        ("a5utc+03a", ""),
        ("5utc+", ""),
        ("5utc+a", ""),
        ("5utc+ ", ""),
        ("5utc- ", ""),
        // negative offset
        ("5utc-03", "5utc-03"),
        ("5utc-03 ", "5utc-03 "),
        (" 5utc-03", " 5utc-03"),
        (" 5utc-03 ", " 5utc-03 "),
        ("rust 5utc-03", " 5utc-03"),
        ("5utc-03 rust", "5utc-03 "),
        ("rust 5utc-03 rust", " 5utc-03 "),
        ("5utc-03a", ""),
        ("a5utc-03", ""),
        ("a5utc-03a", ""),
        // no offset
        ("5utc", "5utc"),
        ("5utc ", "5utc "),
        (" 5utc", " 5utc"),
        (" 5utc ", " 5utc "),
        ("rust 5utc", " 5utc"),
        ("5utc rust", "5utc "),
        ("rust 5utc rust", " 5utc "),
        ("5utca", ""),
        ("a5utc", ""),
        ("a5utca", ""),
        // positive offset
        ("5utc+3", "5utc+3"),
        ("5utc+3 ", "5utc+3 "),
        (" 5utc+3", " 5utc+3"),
        (" 5utc+3 ", " 5utc+3 "),
        ("rust 5utc+3", " 5utc+3"),
        ("5utc+3 rust", "5utc+3 "),
        ("rust 5utc+3 rust", " 5utc+3 "),
        ("5utc+3a", ""),
        ("a5utc+3", ""),
        ("a5utc+3a", ""),
        // optional @, hr@, hrs@
        ("5@utc+3", "5@utc+3"),
        ("5hrs@utc+3", "5hrs@utc+3"),
        ("5hr@utc+3", "5hr@utc+3"),
        ("5h@utc+3", "5h@utc+3"),
        ("5hrr@utc+3", ""),
        ("5@@utc+3", ""),
        ("@5utc+3", ""),
        // UPPER-CASE
        ("5UTC+3", "5UTC+3"),
    ];

    for val in vals {
        println!("---------------------");
        println!("`{}` / `{}`", val.0, val.1);
        if let Some(captures) = rgx.captures(val.0) {
            if let Some(full_match) = captures.get(0) {
                println!("#{}, 0: {}", captures.len(), full_match.as_str());
                assert_eq!(full_match.as_str(), val.1);
                continue;
            };
        };

        assert_eq!(val.1, "");
    }
}
