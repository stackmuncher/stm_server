use hyper_rustls::HttpsConnector;
use rusoto_core::credential::DefaultCredentialsProvider;
use rusoto_core::HttpClient;
use rusoto_core::Region;
use rusoto_s3::S3Client;
use std::str::FromStr;
use std::time::Duration;
use crate::postgres::get_pg_client;
use regex::Regex;

/// All buckets are expected to be in the same region (STM_INBOX_S3_REGION)
/// E.g. `us-east-1`
pub const S3_REGION_ENV: &str = "STM_INBOX_S3_REGION";
/// The name of a bucket where new submissions from the members land (STM_INBOX_S3_BUCKET)
/// E.g. `stm-reports-prod`
pub const S3_INBOX_BUCKET_ENV: &str = "STM_INBOX_S3_BUCKET";
/// The submissions may sit in some folder, not the root (STM_INBOX_S3_BUCKET)
/// The storage tree with any additional folders is placed under this prefix.
/// E.g. `queue`, leading/trailing `/` are removed
pub const S3_INBOX_PREFIX_ENV: &str = "STM_INBOX_S3_PREFIX";
/// The full connection string for the Postgres DB
/// E.g. `host=stm-prod.xxxxxxxxx.us-east-1.rds.amazonaws.com dbname=aaa_bbb user=uuu_vvv password='*#blA()Bla' connect_timeout=15`
pub const PG_CONN_STR: &str = "STM_INBOX_PG_CON_STRING";
/// The bucket where processed member submissions are held (STM_MEMBER_REPORTS_S3_BUCKET)
/// E.g. `stm-member-prod`
pub const S3_MEMBER_REPORTS_BUCKET_ENV: &str = "STM_MEMBER_REPORTS_S3_BUCKET";
/// The member reports are likely to sit in subfolder, not the root (STM_MEMBER_REPORTS_S3_PREFIX)
/// The storage tree with any additional folders is placed under this prefix.
/// E.g. `reports`, leading/trailing `/` are removed
pub const S3_MEMBER_REPORTS_PREFIX_ENV: &str = "STM_MEMBER_REPORTS_S3_PREFIX";


/// A struct with all the config info passed around as a single param
pub struct Config {
    /// The name of the bucket for storing member reports before they are processed.
    /// E.g. `stm-subs-j5awwhv9pb9np7d`
    pub s3_inbox_bucket: String,
    /// The base prefix within the bucket to the root of the report storage.
    /// The storage tree with any additional folders is placed under this prefix.
    /// E.g. `queue`, leading/trailing `/` are removed
    pub s3_inbox_prefix: String,
    /// The name of the bucket for storing member reports after they were processed.
    /// E.g. `stm-subs-j5awwhv9pb9np7d`
    pub s3_report_bucket: String,
    /// The base prefix within the bucket to the root of the report storage.
    /// The storage tree with any additional folders is placed under this prefix.
    /// E.g. `queue`, leading/trailing `/` are removed
    pub s3_report_prefix: String,
    /// Contains an initialized S3 Client for reuse. Doesn't need to be public.
    pub s3_client: S3Client,
    /// An initialized Postgres client
    pub pg_client: tokio_postgres::Client,
    /// A compiled regex for validating 8-char commit hashes
    pub commit_hash_regex: Regex,
}

impl Config {
    /// Initializes a new Config struct from the environment. Panics on invalid config values.
    pub async fn new() -> Self {
        let s3_region = std::env::var(S3_REGION_ENV)
            .expect(&format!(
                "Missing {} env var with S3 region name, e.g. us-east-1",
                S3_REGION_ENV
            ))
            .trim()
            .to_string();

        let s3_region = Region::from_str(&s3_region).expect("Invalid S3 Region value. Must look like `us-east-1`.");

        let pg_connection_string = std::env::var(PG_CONN_STR)
        .expect(&format!(
            "Missing {} env var with Postgres DB connection string. E.g. `host=stm-prod.xxxxxxxxx.us-east-1.rds.amazonaws.com dbname=aaa_bbb user=uuu_vvv password='*#blA()Bla' connect_timeout=15`",
            PG_CONN_STR
        ))
        .trim()
        .trim_end_matches("/")
        .to_string();

        Config {
            s3_inbox_bucket: std::env::var(S3_INBOX_BUCKET_ENV)
                .expect(&format!(
                    "Missing {} env var with Inbox S3 bucket name, e.g. stm-subs-j5awwhv9pb9np7d",
                    S3_INBOX_BUCKET_ENV
                ))
                .trim()
                .trim_end_matches("/")
                .to_string(),
            s3_inbox_prefix: std::env::var(S3_INBOX_PREFIX_ENV)
                .expect(&format!(
                    "Missing {} env var with Inbox S3 prefix, e.g. `queue`",
                    S3_INBOX_PREFIX_ENV
                ))
                .trim()
                .trim_end_matches("/")
                .to_string(),
                s3_report_bucket: std::env::var(S3_MEMBER_REPORTS_BUCKET_ENV)
                .expect(&format!(
                    "Missing {} env var with member reports S3 bucket name, e.g. stm-subs-j5awwhv9pb9np7d",
                    S3_MEMBER_REPORTS_BUCKET_ENV
                ))
                .trim()
                .trim_end_matches("/")
                .to_string(),
            s3_report_prefix: std::env::var(S3_MEMBER_REPORTS_PREFIX_ENV)
                .expect(&format!(
                    "Missing {} env var with S3 prefix for member reports, e.g. `reports`",
                    S3_MEMBER_REPORTS_PREFIX_ENV
                ))
                .trim()
                .trim_end_matches("/")
                .to_string(),
            s3_client: generate_s3_client(s3_region),
            pg_client: get_pg_client(&pg_connection_string).await,
            commit_hash_regex: Regex::new("[a-z0-9]{8}").expect("Invalid commit_hash_regex. It's a bug.")
        }
    }
}

/// Generates an S3Client with custom settings to match AWS server defaults.
fn generate_s3_client(s3_region: Region) -> S3Client {
    let https_connector = HttpsConnector::with_native_roots();

    let cred_prov = DefaultCredentialsProvider::new().expect("Cannot unwrap DefaultCredentialsProvider");

    let mut builder = hyper::Client::builder();
    builder.pool_idle_timeout(Duration::from_secs(15));
    builder.http2_keep_alive_interval(Duration::from_secs(5));
    builder.http2_keep_alive_timeout(Duration::from_secs(3));

    let http_client = HttpClient::from_builder(builder, https_connector);

    S3Client::new_with(http_client, cred_prov, s3_region)
}
