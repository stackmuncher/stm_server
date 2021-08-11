use chrono::Utc;
use regex::Regex;
use rusoto_core::credential::{AwsCredentials, DefaultCredentialsProvider, ProvideAwsCredentials};
use rusoto_s3::S3Client;
use serde::Deserialize;
pub use stackmuncher_lib::config::Config as CoreConfig;
use std::fs;
use std::str::FromStr;
use tracing::{info, warn};

mod deser;

// these two structs exist to allow impl of traits on external types
// drop them if there is a better way of doing impl
#[derive(Debug)]
pub(crate) struct Region(rusoto_core::region::Region);
#[derive(Debug)]
pub(crate) struct LogLevel(tracing::Level);

/// ### A list of ElasticSearch indexes.
/// It is not practical to have a separate ES instance every time there is
/// an index change, specially in dev environment. It is easier to have multiple index versions to co-exist
/// on the same instance.
#[derive(Debug, Deserialize)]
pub(crate) struct EsIdx {
    /// Contains repository-level reports
    pub repo: String,
    /// Contains individual contributor report as set by Git identity.
    /// Multiple contributor reports may be merged into a single developer report.
    pub contributor: String,
    /// Contains GitHub user details and a combined report for all contributor identities.
    pub dev: String,
    /// Contains some stats produced by `stats` flow.
    pub stats: String,
}

/// ### Params of DB-based job queues
#[derive(Debug, Deserialize)]
pub(crate) struct JobQueues {
    ///  A PgSQL connection string to the job queues for processing github logins, repos, devs and contributors.
    pub con_str: String,
    /// Maximum duration for an active DEV job before it is returned to the queue, in seconds.
    pub max_time_in_flight_dev: i64,
    /// Maximum duration for an active REPO job before it is returned to the queue, in seconds.
    pub max_time_in_flight_repo: i64,
    /// Maximum number of processing failures before the job stops being reprocessed.
    pub max_number_of_fails: i32,
}

/// Params for S3 inventory and file retention
#[derive(Debug, Deserialize)]
pub(crate) struct Inventory {
    ///  Do not download repositories larger than this size in bytes.
    pub max_repo_size_download: i64,
    /// Delete repositories larger than this size in bytes after processing.
    pub max_repo_size_keep: i64,
    /// Delay in milliseconds per login to throttle DB updates.
    pub delay_ms: u64,
    /// Number of logins to be processed concurrently during S3 inventory taking
    pub concurrent_logins: usize,
    /// The key to start inventory taking from - never read from the config - must come from CLI.
    /// The key should contain the full S3 prefix. E.g. `repos/rimutaka`.
    #[serde(skip)]
    pub resume_after: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct Config {
    /// Defaults to INFO
    pub log_level: LogLevel,
    /// Must be an existing bucket.
    pub s3_bucket_private_reports: String,
    /// S3 region defaults to US-EAST1 if no value was provided.
    pub s3_region: Region,
    /// ElastiSearch endpoint URL
    pub es_url: String,
    /// ElastiSearch index names
    pub es_idx: EsIdx,
    /// The flow to execute. Defaults to `GitHub`
    pub flow: Flow,
    /// DB connection string, timeouts and other properties required to interact with DB-based job queues.
    pub job_queues: JobQueues,
    /// Contains `stackmuncher::config::Config`, when applicable. The upstream code should always init this member for the downstream code to use `unwrap`.
    #[serde(skip)]
    pub core_config: Option<CoreConfig>,
    /// Contains an initialized S3 Client for reuse. Doesn't need to be public. It is retrieved using a function call.
    #[serde(skip)]
    s3_client_inner: Option<S3Client>,
    /// No-SQL field value validation regex - the value would be invalid if it's a match
    /// Doesn't need to be public. It is retrieved using a function call.
    #[serde(skip)]
    no_sql_param_invalidation_regex_inner: Option<Regex>,
    /// A shared copy of AWS creds for reuse elsewhere.
    /// Doesn't need to be public. It is retrieved using a function call.
    #[serde(skip)]
    aws_credentials: Option<AwsCredentials>,
    /// A compiled regex to validate a 44-char long owner id in base58 form.
    /// E.g. 9PdHabyyhf4KhHAE1SqdpnbAZEXTHhpkermwfPQcLeFK
    #[serde(skip)]
    pub owner_id_validation_regex: Option<Regex>,
}

/// Defines what flow is activated in the app when its launched
#[derive(Debug)]
pub(crate) enum Flow {
    DevQueue,
    Help,
}

impl Default for Flow {
    /// Returns Flow::Help
    fn default() -> Self {
        Flow::Help
    }
}

impl FromStr for Flow {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // I could not use `Config::CLI_MODES[0]` directly in match arms
        // this is a hack to use the values from the arrat instead of literals
        const S0: &str = Config::CLI_MODES[0];

        match s {
            S0 => Ok(Flow::DevQueue),
            _ => {
                if !s.is_empty() {
                    println!("Invalid flow type: {}", s);
                }
                Ok(Flow::Help)
            }
        }
    }
}

impl Config {
    /// The order of items in this array must correspond to the order of `impl FromStr for Flow`
    pub(crate) const CLI_MODES: [&'static str; 1] = ["dev_queue"];

    /// Inits values from ENV vars and the command line arguments
    pub(crate) async fn new() -> Self {
        // init the structure from config.json sitting in the working folder
        let conf = match fs::File::open("config.json") {
            Err(e) => {
                panic!("Cannot read config.json file. {}", e);
            }
            Ok(v) => v,
        };

        let mut config: Config = match serde_json::from_reader(conf) {
            Err(e) => {
                panic!("Cannot parse config.json. {}", e);
            }
            Ok(v) => v,
        };

        // check if there were any arguments passed to override the config file
        let mut args = std::env::args().peekable();
        loop {
            if let Some(arg) = args.next() {
                match arg.to_lowercase().as_str() {
                    "-l" => {
                        config.log_level = LogLevel(Config::string_to_log_level(
                            args.peek().expect("-l arg is missing one of [trace, debug, info]"),
                        ))
                    }
                    "-flow" => {
                        config.flow = match args.peek() {
                            Some(v) => Flow::from_str(v).expect("Flow::from_str should always unwrap"),
                            _ => {
                                panic!(
                                    "-flow is required with one of: {:?}, optional -l [trace, debug, info]",
                                    Config::CLI_MODES
                                );
                            }
                        }
                    }
                    _ => { //do nothing
                    }
                };
            } else {
                break;
            }
        }

        // this member must be set to Some() for the downstream code to unwrap without checking
        config.core_config = Some(CoreConfig::new_with_defaults(&config.log_level.0));

        // init a reusable S3 client
        config.s3_client_inner = Some(crate::utils::s3::generate_s3_client(&config));

        // pre-compile NOSQL param validation regex
        // A regex formula to check for unsafe values to insert into another regex string.
        // It is stricter than no_sql_string_invalidation_regex and is to be compiled only in some cases
        config.no_sql_param_invalidation_regex_inner =
            Some(Regex::new(r#"[^#\-\._0-9a-zA-Z]"#).expect("Failed to compile no_sql_string_value_regex"));

        // pre-compile owner id validation regex
        config.owner_id_validation_regex = Some(
            Regex::new(r#"^[123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz]{44}$"#)
                .expect("Failed to compile owner_id_validation_regex"),
        );

        // get AWS creds
        let provider = DefaultCredentialsProvider::new().expect("Cannot get default creds provider");
        config.aws_credentials = Some(provider.credentials().await.expect("Cannot find creds"));

        config
    }

    /// Unwraps `aws_credentials` member with an initialized AwsCredentials.
    pub(crate) fn aws_credentials(&self) -> &AwsCredentials {
        self.aws_credentials.as_ref().unwrap()
    }

    /// Checks if the the token in `aws_credentials` member is about to expire and tries to renew it.
    /// Panics if the creds cannot be renewed.
    pub(crate) async fn renew_aws_credentials(&mut self) {
        if let Some(creds) = &self.aws_credentials {
            if let Some(expiration) = creds.expires_at() {
                info!("AWS token expiration: {}", expiration.to_rfc3339());
                // renew if expires within the next 2 minutes
                // normally tokens have many hours of life
                if expiration.timestamp() - Utc::now().timestamp() > 120 {
                    return;
                }
            };
        };

        warn!("Renewing AWS token.");

        let provider = DefaultCredentialsProvider::new().expect("Cannot get default creds provider");
        self.aws_credentials = Some(provider.credentials().await.expect("Cannot find creds"));

        self.aws_credentials
            .as_ref()
            .expect("Cannot unwrap aws creds after renewal");
    }

    /// Unwraps `s3_client_inner` member with an initialized S3Client.
    pub(crate) fn s3_client(&self) -> &S3Client {
        self.s3_client_inner.as_ref().unwrap()
    }

    /// Returns the log level as struct. Defaults to INFO if none was provided. Panics if the value is invalid.
    pub(crate) fn string_to_log_level(s: &str) -> tracing::Level {
        if s.is_empty() {
            return tracing::Level::INFO;
        }
        let err_msg = &["Invalid tracing level value: ", s].concat();
        tracing::Level::from_str(s).expect(err_msg)
    }
}
