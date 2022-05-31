use hyper_rustls::HttpsConnector;
use rusoto_core::credential::DefaultCredentialsProvider;
use rusoto_core::HttpClient;
use rusoto_core::Region;
use rusoto_s3::S3Client;
use std::str::FromStr;
use std::time::Duration;

/// Name of a required env variable (STM_INBOX_S3_REGION)
/// E.g. `us-east-1`
pub const S3_REGION_ENV: &str = "STM_INBOX_S3_REGION";
/// Name of a required env variable (STM_INBOX_S3_BUCKET)
/// E.g. `stm-reports-prod`
pub const S3_BUCKET_ENV: &str = "STM_INBOX_S3_BUCKET";
/// Name of a required env variable (STM_INBOX_S3_BUCKET)
/// The storage tree with any additional folders is placed under this prefix.
/// E.g. `queue`, leading/trailing `/` are removed
pub const S3_PREFIX_ENV: &str = "STM_INBOX_S3_PREFIX";

/// A struct with all the config info passed around as a single param
pub struct Config {
    /// The name of the bucket for storing contributor reports.
    /// E.g. `stm-subs-j5awwhv9pb9np7d`
    pub s3_bucket: String,
    /// The base prefix within the bucket to the root of the report storage.
    /// The storage tree with any additional folders is placed under this prefix.
    /// E.g. `queue`, leading/trailing `/` are removed
    pub s3_prefix: String,
    /// Contains an initialized S3 Client for reuse. Doesn't need to be public.
    pub s3_client: S3Client,
}

impl Config {
    /// Initializes a new Config struct from the environment. Panics on invalid config values.
    pub fn new() -> Self {
        let s3_region = std::env::var(S3_REGION_ENV)
            .expect(&format!("Missing {} env var with S3 region name, e.g. us-east-1", S3_REGION_ENV))
            .trim()
            .to_string();

        let s3_region = Region::from_str(&s3_region).expect("Invalid S3 Region value. Must look like `us-east-1`.");

        Config {
            s3_bucket: std::env::var(S3_BUCKET_ENV)
                .expect(&format!(
                    "Missing {} env var with S3 bucket name, e.g. stm-subs-j5awwhv9pb9np7d",
                    S3_BUCKET_ENV
                ))
                .trim()
                .trim_end_matches("/")
                .to_string(),
            s3_prefix: std::env::var(S3_PREFIX_ENV)
                .expect(&format!("Missing {} env var with S3 prefix, e.g. `queue`", S3_PREFIX_ENV))
                .trim()
                .trim_end_matches("/")
                .to_string(),
            s3_client: generate_s3_client(s3_region),
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
