use serde::Deserialize;
use std::fs;
use std::str::FromStr;
mod deser;
use chrono::Utc;
use rusoto_core::credential::{AwsCredentials, DefaultCredentialsProvider, ProvideAwsCredentials};
use rusoto_s3::S3Client;
use rusoto_sqs::SqsClient;
use std::process::exit;
use tracing::{info, warn};

// these two structs exist to allow impl of traits on external types
// drop them if there is a better way of doing impl
#[derive(Debug)]
pub(crate) struct Region(rusoto_core::region::Region);
#[derive(Debug)]
pub(crate) struct LogLevel(tracing::Level);

/// A list of ElasticSearch indexes.
#[derive(Debug, Deserialize)]
pub(crate) struct EsIdx {
    /// Contains search queries and what was returned in response.
    pub search_log: String,
}

/// URLs of SQS queues
#[derive(Deserialize)]
pub(crate) struct SqsEndpoints {
    /// S3 notifications of new www-logs from CloudFront
    pub www_logs: Option<String>,
    /// User searches and returned results are logged in this queue.
    pub search_stats: Option<String>,
    /// A reusable SQS client for `aws_region`.
    /// Remember to update its init code if new queues are added to the list above.
    #[serde(skip)]
    pub sqs_client: Option<SqsClient>,
}

#[derive(Deserialize)]
pub(crate) struct Config {
    /// Defaults to INFO
    pub log_level: LogLevel,
    /// Must be an existing bucket.
    pub s3_bucket_web_logs: String,
    /// AWS region defaults to US-EAST1 if no value was provided.
    /// It is reused for all services.
    pub aws_region: Region,
    /// URLs of SQS queues.
    pub sqs_endpoints: SqsEndpoints,
    /// Contents of Auth header. Calculated from user+key.
    pub es_url: String,
    /// ElastiSearch index names.
    pub es_idx: EsIdx,
    /// Postgres connection string.
    pub pg_con_str: String,
    /// The flow to execute. Defaults to `GitHub`
    pub flow: Flow,
    /// Contains an initialized S3 Client for reuse. Doesn't need to be public. It is retrieved using a getter.
    #[serde(skip)]
    s3_client_inner: Option<S3Client>,
    /// A shared copy of AWS creds for reuse elsewhere.
    /// Doesn't need to be public. It is retrieved using a function call.
    #[serde(skip)]
    aws_credentials: Option<AwsCredentials>,
}

/// Defines what flow is activated in the app when its launched
#[derive(Debug)]
pub(crate) enum Flow {
    WwwLogReader,
}

impl FromStr for Flow {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // I could not use `Config::CLI_MODES[0]` directly in match arms
        // this is a hack to use the values from the array instead of literals
        const S0: &str = Config::CLI_MODES[0];

        match s {
            S0 => Ok(Flow::WwwLogReader),
            _ => {
                println!("Invalid flow type: {}", s);
                exit(1);
            }
        }
    }
}

impl Config {
    /// The order of items in this array must correspond to the order of `impl FromStr for Flow`
    pub(crate) const CLI_MODES: [&'static str; 1] = ["www_log_reader"];

    /// Inits values from ENV vars and the command line arguments
    pub(crate) async fn new() -> Self {
        // init the structure from config.json sitting in the working folder
        let conf = match fs::File::open("config.json") {
            Err(e) => {
                println!("Cannot read config.json file. {}", e);
                exit(1);
            }
            Ok(v) => v,
        };

        let mut config: Config = match serde_json::from_reader(conf) {
            Err(e) => {
                println!("Cannot parse config.json. {}", e);
                exit(1);
            }
            Ok(v) => v,
        };

        // check if there were any arguments passed to override the config file
        let mut args = std::env::args().peekable();
        loop {
            if let Some(arg) = args.next() {
                match arg.to_lowercase().as_str() {
                    "--log" => {
                        config.log_level = LogLevel(Config::string_to_log_level(
                            args.peek().expect("--log arg is missing one of [trace, debug, info]"),
                        ))
                    }
                    "--flow" => {
                        config.flow = match args.peek() {
                            Some(v) => Flow::from_str(v).expect("Flow::from_str should always unwrap"),
                            _ => {
                                println!(
                                    "--flow is required with one of: {:?}, optional -l [trace, debug, info]",
                                    Config::CLI_MODES
                                );
                                exit(1);
                            }
                        }
                    }
                    _ => {
                        //do nothing
                    }
                };
            } else {
                break;
            }
        }

        // init a reusable S3 client
        config.s3_client_inner = Some(stm_shared::s3::generate_s3_client(&config.aws_region));

        // init a reusable SQS client if there are any SQS endpoints
        if config.sqs_endpoints.search_stats.is_some() || config.sqs_endpoints.www_logs.is_some() {
            config.sqs_endpoints.sqs_client = Some(SqsClient::new(config.aws_region.0.clone()));
        }

        // get AWS creds
        let provider = DefaultCredentialsProvider::new().expect("Cannot get default creds provider");
        config.aws_credentials = Some(provider.credentials().await.expect("Cannot find creds"));

        config
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
