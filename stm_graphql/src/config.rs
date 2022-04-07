use regex::Regex;
use rusoto_core::region::Region as AwsRegion;
use rusoto_sqs::SqsClient;
use std::str::FromStr;

/// Add the name of the ElasticSearch index to that env var
const ES_DEV_IDX_ENV: &str = "STM_HTML_ES_DEV_IDX";
/// Add the name of the ElasticSearch index to that env var
const ES_STATS_IDX_ENV: &str = "STM_HTML_ES_STATS_IDX";
/// Add the absolute ElasticSearch URL to that env var
const ES_URL_ENV: &str = "STM_HTML_ES_URL";
/// Add URL of the SQS queue for search results stats to that env var
const SQS_SEARCH_STATS_URL: &str = "STM_HTML_SQS_SEARCH_STATS_URL";
/// The N-portion of JWK supplied by AUTH0.
/// Extracted from https://stackmuncher.us.auth0.com/.well-known/jwks.json
const JWK_N: &str = "STM_UI_AUTH0_PUB_KEY_N";
/// The E-portion of JWK supplied by AUTH0.
/// Extracted from https://stackmuncher.us.auth0.com/.well-known/jwks.json
const JWK_E: &str = "STM_UI_AUTH0_PUB_KEY_E";

pub struct Config {
    pub aws_region: AwsRegion,
    /// Absolute ElasticSearch URL
    pub es_url: String,
    /// Name of `dev` index
    pub dev_idx: String,
    /// Name of `stats` index
    pub stats_idx: String,
    /// SQS URL for logging search results
    pub search_log_sqs_url: String,
    /// Extracts required working hours and the timezone from the search string
    /// E.g. 5utc+00, 5utc-0, 5utc, 5utc+03, 5utc-03
    /// * Capture group 1: hours in the timezone
    /// * Capture group 2: timezone, can be blank for UTC
    pub timezone_terms_regex: Regex,
    /// Extracts the page part from the query, e.g. p:1 or p:15
    pub page_num_terms_regex: Regex,
    /// A compiled regex that returns a match if the library name is invalid and should not be searched for
    pub library_name_invalidation_regex: Regex,
    /// SQS client for `aws_region`
    pub sqs_client: SqsClient,
    /// A fixed list of known techs/languages.
    /// This is a temporary plug until caching is implemented.
    pub all_langs: Vec<String>,
    /// A value for JWK n param taken from an env var.
    pub jwk_n: String,
    /// A value for JWK e param taken from an env var.
    pub jwk_e: String,
}

impl Config {
    /// The maximum number of dev listings per page of search results
    pub const MAX_DEV_LISTINGS_PER_SEARCH_RESULT: usize = 50;

    /// The maximum number of pages allowed in search.
    /// Check HTML templates if changing the limits on page numbers
    /// 20 is hardcoded in some of the logic there
    pub const MAX_PAGES_PER_SEARCH_RESULT: usize = 20;

    pub fn new() -> Self {
        let aws_region = AwsRegion::from_str(
            std::env::var("AWS_REGION")
                .expect(&format!("Missing AWS_REGION env var with the AWS region, e.g. us-east-1"))
                .trim()
                .trim_end_matches("/"),
        )
        .expect("Invalid value in AWS_REGION env var. Expecting `us-east-1` format.");

        Config {
            aws_region: aws_region.clone(),

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
            search_log_sqs_url: std::env::var(SQS_SEARCH_STATS_URL)
                .expect(&format!("Missing {} env var with SQS SEARCH_STATS URL", SQS_SEARCH_STATS_URL))
                .trim()
                .to_string(),
            timezone_terms_regex: Regex::new(
                r#"(?i)(?:[[:space:]]|^)(\d{1,2})(?:hrs@|hr@|h@|@)?utc([\+\-]\d{1,2})?(?:[[:space:]]|$)"#,
            )
            .expect("Failed to compile timezone_terms_regex"),
            page_num_terms_regex: Regex::new(r#"(?:[ ,]|^)p:(\d+)(?:[ ,]|$)"#)
                .expect("Failed to compile page_num_terms_regex"),
            library_name_invalidation_regex: Regex::new(r#"[^[:alnum:]\.\-_]"#)
                .expect("Failed to compile library_name_invalidation_regex"),
            sqs_client: SqsClient::new(aws_region),
            all_langs: vec![
                "C#",
                "C++",
                "CSS",
                "DevOps",
                "Docker",
                "eRuby",
                "Go",
                "HAML",
                "HTML",
                "Java",
                "JavaScript",
                "Jupyter",
                "Kotlin",
                "Makefile",
                "Markdown",
                "PowerShell",
                "Puppet",
                "Python",
                "ReactJS",
                "restructuredText",
                "Ruby",
                "Rust",
                "SCSS",
                "Shell",
                "SQL",
                "Terraform",
                "TypeScript",
                "VueJS",
            ]
            .into_iter()
            .map(|s| s.to_owned())
            .collect(),
            jwk_n: std::env::var(JWK_N)
                .expect(&format!("Missing {} env var with JWK n value", JWK_N))
                .trim()
                .to_string(),
            jwk_e: std::env::var(JWK_E)
                .expect(&format!("Missing {} env var with JWK e value", JWK_E))
                .trim()
                .to_string(),
        }
    }
}

// /// Returns TRUE if the owner_id decodes from base58 into exactly 256 bytes.
// /// Logs a warning and returns FALSE otherwise.
// /// TODO: this should be a shared utility function!!!
// pub(crate) fn validate_owner_id(owner_id: &str) -> bool {
//     match bs58::decode(owner_id).into_vec() {
//         Err(e) => {
//             warn!("Invalid owner_id: {}. Cannot decode from bs58: {}", owner_id, e);
//             false
//         }
//         Ok(v) => {
//             if v.len() == 32 {
//                 true
//             } else {
//                 warn!("Invalid owner_id: {}. Decoded to {} bytes", owner_id, v.len());
//                 false
//             }
//         }
//     }
// }

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

/// Attempts to initialize logging at INFO level. It is specially useful for test
/// functions as a shortcut for logging initializing. This Fn is safe to call multiple times.
pub(crate) fn init_logging() {
    let tsub = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_ansi(false);

    if tsub.try_init().is_ok() {
        tracing::info!("tracing_subscriber initialized");
    }
}
