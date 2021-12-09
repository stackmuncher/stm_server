use crate::config::Config;
use crate::db::bots::IpLog;
use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use stm_shared::elastic::types::SearchLog;
use stm_shared::elastic::upload_object_to_es;
use stm_shared::pgsql::get_pg_client;
use stm_shared::sqs::delete_messages;
use stm_shared::FailureType;
use stm_shared::{aws_events, s3, sqs};
use tokio::time::{sleep, Duration};
use tokio_postgres::Client as PgClient;
use tracing::{error, info, warn};

/// Value: `demo/`
/// * s3://stm-www-logs-2q16ag89dl/demo/E2TC76QMLKRQVN.2021-03-09-02.334ce4b9.gz
const S3_KEY_PREFIX: &str = "demo/";

pub(crate) async fn read_www_logs(mut config: Config) {
    info!("Reading, parsing and deleting all CloudFront logs from S3.");

    let s3_key_prefix = String::from_str(S3_KEY_PREFIX).unwrap();

    let sqs_client = config
        .sqs_endpoints
        .sqs_client
        .as_ref()
        .expect("Failed to get SQS client. Check if AWS zone is configured correctly.")
        .clone();
    let www_logs_queue_url = config
        .sqs_endpoints
        .www_logs
        .as_ref()
        .expect("Failed to get URL of SQS queue for www-logs. Check the config.")
        .clone();

    let search_events_queue_url = config
        .sqs_endpoints
        .search_stats
        .as_ref()
        .expect("Failed to get URL of SQS queue for search stats. Check the config.")
        .clone();

    if sqs::purge_queue(&sqs_client, &www_logs_queue_url).await.is_err() {
        error!("Failed to purge the logs queue. No point proceeding.");
        std::process::exit(1);
    };

    // used to determine repeated errors and abort processing
    // it will only abort processing if N consecutive keys return ERROR
    // its goal is to react to NOTHING WORKS situation quickly
    let mut err_counter = 0usize;
    const MAX_CONSECUTIVE_ERRORS: usize = 10;

    // try to get the jobs DB client (postgres)
    // this line panics if the connection fails
    let pg_client = get_pg_client(&config.pg_con_str).await;

    // a holder of the last s3 key in the retrieved list for start-after
    let mut start_after = Some(String::new());

    // enter a loop of processing the backlog of files in S3 using LIST
    loop {
        // terminate the process if it keeps failing
        if err_counter >= MAX_CONSECUTIVE_ERRORS {
            error!("Too many errors. Exiting.");
            std::process::exit(1);
        }

        // how many errors there were at the beginning of the loop
        let loop_start_errors = err_counter;

        // renew the creds if needed
        config.renew_aws_credentials().await;

        // get as many records in one go as AWS can give (1000?)
        let (s3_objects, _) = match s3::list_up_to_10000_objects_from_s3(
            config.s3_client(),
            &config.s3_bucket_web_logs,
            &s3_key_prefix,
            &start_after,
        )
        .await
        {
            Err(_e) => {
                err_counter += 1;
                error!("Error count: {}, sleeping for 10s", err_counter);
                sleep(Duration::from_secs(10)).await;
                continue;
            }
            Ok(v) => v,
        };

        // exit on completion - no more messages left
        if s3_objects.is_empty() {
            info!("No more records in S3.");
            break;
        }

        // process all logs sequentially, but ignore the IPs returned by the function
        // hopefully the compiler will drop that branch of the code
        let s3_keys = s3_objects.into_iter().map(|v| v.key).collect::<Vec<String>>();
        // the next lot of keys start with this one
        if let Some(last_key) = s3_keys.last() {
            info!("Next search after: {}", last_key);
            start_after = Some(last_key.clone());
        };

        // process the full lot of logs in one call
        err_counter += process_www_logs(&config, &pg_client, s3_keys).await.1;

        // reset the error counter if the loop completed successfully with no errors
        if err_counter == loop_start_errors {
            err_counter = 0;
        }
    }

    // collect all known IP from this run
    let mut ip_cache = match IpLog::get_all_ips(&pg_client).await {
        Ok(v) => v.into_iter().collect::<HashSet<String>>(),
        Err(_) => {
            error!("Failed to get the list of all IPs from DB.");
            std::process::exit(1);
        }
    };

    info!("Retrieved {} IPs from DB", ip_cache.len());

    // enter an infinite loop of getting new logs and new search results
    // the logs are processed without a wait time - if there is a new log in the queue it is processed,
    // if not, the code moves onto the next 10 search stats events to ensure there is no bottleneck
    // logs arrive at a scheduled interval of 5 - 10 min
    // search events are added to the queue with every web request served by Lambdas and are more of a stream

    err_counter = 0;

    loop {
        // how many errors there were at the beginning of the loop
        let loop_start_errors = err_counter;

        // terminate the process if it keeps failing
        if err_counter >= MAX_CONSECUTIVE_ERRORS {
            error!("Too many errors. Exiting.");
            std::process::exit(1);
        }

        // renew the creds if needed
        config.renew_aws_credentials().await;

        // catchup on any new jobs that arrived in the log queue while we were processing the backlog
        let new_log_msgs =
            match sqs::SqsMessages::<aws_events::s3::S3Event>::get(&sqs_client, &www_logs_queue_url, 10, false).await {
                Ok(v) => v,
                Err(_) => {
                    error!("Failed to get messaged from www-logs queue.");
                    sleep(Duration::from_secs(60)).await;
                    err_counter += 1;
                    continue;
                }
            };

        // get the list of handles for delete the messages from the queue
        let receipt_handles = new_log_msgs.get_all_receipt_handles();
        let new_log_msgs_count = new_log_msgs.messages.len();

        // get the list of all S3 keys from received messages
        let s3_keys = new_log_msgs
            .messages
            .into_iter()
            .map(|msg| msg.message.get_all_s3_keys())
            .collect::<Vec<Vec<String>>>()
            .into_iter()
            .flatten()
            .collect::<Vec<String>>();

        info!("Received {} new search log msgs with {} keys", new_log_msgs_count, s3_keys.len());

        // there should be just one log per event, but the code is more elegant if we re-use multi-log processing
        let (new_ips, new_errors) = process_www_logs(&config, &pg_client, s3_keys).await;
        let cache_size = ip_cache.len();

        // add new IPs to the local cache
        for ip in new_ips {
            ip_cache.insert(ip);
        }
        info!(
            "Cache size: {}, new IPs added: {}, errors: {}/{}",
            ip_cache.len(),
            ip_cache.len() - cache_size,
            err_counter,
            new_errors
        );
        if new_errors == 0 {
            err_counter = 0
        } else {
            err_counter += new_errors;
            sleep(Duration::from_secs(60)).await;
            continue;
        }

        // delete the messages from www-logs queue
        if delete_messages(&sqs_client, &www_logs_queue_url, receipt_handles)
            .await
            .is_err()
        {
            err_counter += 1;
        };

        // process search events -------------------------------------------------------

        let new_search_events =
            match sqs::SqsMessages::<SearchLog>::get(&sqs_client, &search_events_queue_url, 10, true).await {
                Ok(v) => v,
                Err(_) => {
                    error!("Failed to get messages from search-events queue.");
                    sleep(Duration::from_secs(60)).await;
                    err_counter += 1;
                    continue;
                }
            };

        let new_search_events_count = new_search_events.messages.len();
        info!("Received {} new search event msgs", new_search_events_count);

        // get the list of handles for deleting the messages from the queue
        let receipt_handles = new_search_events.get_all_receipt_handles();

        // save events in ES if they are not from the list of known bot IPs
        for search_event in new_search_events.messages {
            if let Some(ip) = &search_event.message.ip {
                if !ip_cache.contains(ip) {
                    // push the payload into ES
                    let object_id = search_event.message.get_hash();
                    if upload_object_to_es::<SearchLog>(
                        config.es_url.clone(),
                        config.aws_credentials().clone(),
                        search_event.message,
                        object_id,
                        config.es_idx.search_log.clone(),
                    )
                    .await
                    .is_err()
                    {
                        err_counter += 1;
                    };
                }
            }
        }

        // delete event messages from the SQS queue
        if delete_messages(&sqs_client, &search_events_queue_url, receipt_handles)
            .await
            .is_err()
        {
            err_counter += 1;
        };

        // reset the error counter if work was done and the loop completed successfully with no errors
        if err_counter == loop_start_errors && (new_log_msgs_count > 0 || new_search_events_count > 0) {
            err_counter = 0;
        }
        info!("End of loop error count: {}", err_counter);
    }
}

/// Processes the collection of logs one at a time and returns a list of bot IPs and the number of errors it encountered.
/// Logs are saved in the DB and the S3 files are deleted if no errors were encountered.
async fn process_www_logs(config: &Config, pg_client: &PgClient, s3_keys: Vec<String>) -> (Vec<String>, usize) {
    // an output collector for merged IpLog records to minimize the DB load
    let mut new_ip_logs_per_loop: HashMap<String, IpLog> = HashMap::new();

    // a list of successfully processed log files that can be deleted
    let mut s3_keys_for_deletion: Vec<String> = Vec::new();

    // a list of new IPs to be added to cache
    let mut ip_cache: Vec<String> = Vec::new();

    // total number of errors per function call
    let mut error_counter = 0_usize;

    // process every file separately
    for s3_key in s3_keys {
        let ip_logs = match process_www_log(s3_key.clone(), &config).await {
            Ok(v) => v,
            Err(e) => match e {
                FailureType::DoNotRetry(v) => {
                    warn!("Deleting faulty log file: {}", v);
                    s3_keys_for_deletion.push(v);
                    continue;
                }
                FailureType::Retry(v) => {
                    warn!("Faulty log file will be retried: {}", v);
                    continue;
                }
            },
        };

        // merge IP records so that there is only one record per IP with the very first added_ts and
        // the very last latest_ts per bucket-list loop, which is up to 1000 files
        let new_ip_logs_per_loop_len_start = new_ip_logs_per_loop.len();
        for ip_log in ip_logs {
            match new_ip_logs_per_loop.get_mut(&ip_log.ip) {
                Some(v) => {
                    v.cnt += ip_log.cnt;
                    v.latest_ts = ip_log.latest_ts;
                }
                None => {
                    new_ip_logs_per_loop.insert(ip_log.ip.clone(), ip_log);
                }
            }
        }

        info!("New IPs from log: {}", new_ip_logs_per_loop.len() - new_ip_logs_per_loop_len_start);

        s3_keys_for_deletion.push(s3_key);
    }

    // add the IPs to the local cache
    for ip in new_ip_logs_per_loop.keys() {
        ip_cache.push(ip.clone());
    }

    // save merged IpLog data in the DB in one hit
    let new_ip_logs_per_loop = new_ip_logs_per_loop.into_iter().map(|(_, v)| v).collect::<Vec<IpLog>>();
    if IpLog::add_or_update(new_ip_logs_per_loop, &pg_client).await.is_err() {
        error_counter += 1;
    };

    // abort the loop if there were any errors
    if error_counter > 0 {
        return (ip_cache, error_counter);
    }

    // delete the S3 objects
    if s3::delete_from_s3(config.s3_client(), &config.s3_bucket_web_logs, s3_keys_for_deletion)
        .await
        .is_err()
    {
        error_counter += 1;
    }

    // expected to be zero
    (ip_cache, error_counter)
}

/// Read the specified log, check all the records inside and return a list of records to update in the DB.
async fn process_www_log(log_s3_key: String, config: &Config) -> Result<Vec<IpLog>, FailureType<String>> {
    info!("Log file: {}", log_s3_key);

    // get the log file as unzipped bytes
    let log = match s3::get_text_from_s3(config.s3_client(), &config.s3_bucket_web_logs, log_s3_key.clone(), true).await
    {
        Ok((v, _)) => v,
        Err(_) => return Err(FailureType::DoNotRetry(log_s3_key)),
    };

    // convert it into a string for splitting
    let log = match String::from_utf8(log) {
        Ok(v) => v,
        Err(e) => {
            error!("Cannot convert S3 object to UTF8 text: {}", e);
            return Err(FailureType::DoNotRetry(log_s3_key));
        }
    };

    // an output collector
    let mut ip_records: HashMap<String, IpLog> = HashMap::new();

    // process every line separately
    let mut line_counter = 0_usize;
    for log_line in log.lines() {
        line_counter += 1;

        // logs start with `#Version: 1.0`
        // headers start with `#Fields: `
        if log_line.starts_with("#") || log_line.is_empty() {
            continue;
        }

        // split into individual fields
        // 2021-03-09	08:16:17	HEL50-C2	499	77.88.5.23	GET	d2au2ee4m2bz32.cloudfront.net	/robots.txt	200	-	Mozilla/5.0%20(compatible;%20YandexBot/3.0;%20+http://yandex.com/bots)	-	-	Hit	20OKoifV_tfflrNoY6f8ArwRIKPh98GVZOcNNj5SARzIqPoe6m132g==	stackmuncher.com	https	199	0.002	-	TLSv1.3	TLS_AES_128_GCM_SHA256	Hit	HTTP/1.1	-	-	34060	0.002	Hit	text/plain	28	-	-
        let fields = log_line.split("\t").collect::<Vec<&str>>();

        // 32 tabs, 33 fields
        if fields.len() != 33 {
            error!("Invalid line with {} fields", fields.len());
            error!("{}", log_line);
            return Err(FailureType::DoNotRetry(log_s3_key));
        }

        // check the user agent
        if let Some(bot_pattern) = BOT_AGENTS.iter().find_map(|s| {
            if fields[10].to_lowercase().contains(s) {
                Some(s)
            } else {
                None
            }
        }) {
            // it's a bot
            info!("Bot pattern: {}", bot_pattern);
            info!("Bot UA: {}", fields[10]);
        } else {
            // not a bot - nothing else to be done
            info!("User UA: {}", fields[10]);
            continue;
        };

        // it's a bot - extract and log the details in the DB
        let date_time = match DateTime::parse_from_rfc3339(&[fields[0], "T", fields[1], "+00:00"].concat()) {
            Ok(v) => v.with_timezone(&Utc),
            Err(e) => {
                error!("Invalid timestamp: {}", e);
                error!("{}", log_line);
                return Err(FailureType::DoNotRetry(log_s3_key));
            }
        };

        // merge IP records so that there is only one record per IP with the very first added_ts and the very last latest_ts
        match ip_records.get_mut(fields[4]) {
            Some(v) => {
                v.cnt += 1;
                v.latest_ts = date_time.clone();
            }
            None => {
                ip_records.insert(
                    fields[4].to_string(),
                    IpLog {
                        ip: fields[4].to_string(),
                        cnt: 1,
                        added_ts: date_time.clone(),
                        latest_ts: date_time,
                    },
                );
            }
        }
    }

    // simplify HashMap -> Vec
    let ip_records = ip_records.into_iter().map(|(_, v)| v).collect::<Vec<IpLog>>();

    info!("Lines: {}, IpLogs extracted: {}", line_counter, ip_records.len());

    Ok(ip_records)
}

/// A list of lower-cased user-agent substrings from https://github.com/monperrus/crawler-user-agents/blob/master/crawler-user-agents.json
const BOT_AGENTS: [&str; 485] = [
    "ptst",
    "sentry",
    "ppingdom",
    "wget",
    "apache-httpclient",
    "curl",
    "lcc ",
    "php-curl-class",
    "crawler",
    "2ip.ru",
    "360spider",
    "a6-indexer",
    "aboundex",
    "acapbot",
    "acoonbot",
    "adbeat_bot",
    "addsearchbot",
    "addthis",
    "adidxbot",
    "admantx",
    "adsbot-google",
    "adscanner",
    "advbot",
    "ahc",
    "ahrefs",
    "aihitbot",
    "aiohttp",
    "aisearchbot",
    "alphabot",
    "amazon cloudfront",
    "amazonbot",
    "anderspinkbot",
    "antibot",
    "anyevent",
    "apercite",
    "apis-google",
    "appengine-google",
    "appinsights",
    "applebot",
    "arabot",
    "archive.org_bot",
    "archivebot",
    "aspiegelbot",
    "atom feed robot",
    "awariorssbot",
    "awariosmartbot",
    "axios",
    "b2b bot",
    "baidu-yunguance",
    "baiduspider",
    "bark[rr]owler",
    "bazqux",
    "bdcbot",
    "behloolbot",
    "betabot",
    "bidswitchbot",
    "biglotron",
    "bingbot",
    "bingpreview",
    "binlar",
    "bitbot",
    "bitlybot",
    "blackboard",
    "blexbot",
    "blogmurabot",
    "blogtraffic",
    "blp_bbot",
    "bnf.fr_bot",
    "bomborabot",
    "bot-pge.chlooe.com",
    "bot.araturka.com",
    "botify",
    "boxcarbot",
    "brainobot",
    "brandonbot",
    "brandverity",
    "btwebclient",
    "bubing",
    "bublupbot",
    "buck",
    "buzzbot",
    "bytespider",
    "caliperbot",
    "capsulechecker",
    "careerbot",
    "cc metadata scaper",
    "ccbot",
    "centurybot",
    "changedetection",
    "check_http",
    "checkmarknetwork",
    "chrome-lighthouse",
    "cincraw",
    "citeseerxbot",
    "clickagy",
    "cliqzbot",
    "cloudflare-alwaysonline",
    "coccoc",
    "collection@infegy.com",
    "contextad bot",
    "contxbot",
    "convera",
    "crunchbot",
    "crystalsemanticsbot",
    "curebot",
    "cutbot",
    "cxensebot",
    "cyberpatrol",
    "dareboost",
    "datafeedwatch",
    "datagnionbot",
    "datanyze",
    "dataprovider.com",
    "daum",
    "dcrawl",
    "deadlinkchecker",
    "deusu",
    "diffbot",
    "digg deeper",
    "digincore bot",
    "discobot",
    "discordbot",
    "disqus",
    "dnyzbot",
    "domain re-animator bot",
    "domains project",
    "domainstatsbot",
    "dotbot",
    "dragonbot",
    "drupact",
    "dubbotbot",
    "duckduckbot",
    "duckduckgo-favicons-bot",
    "ec2linkfinder",
    "edisterbot",
    "electricmonk",
    "elisabot",
    "embedly",
    "epicbot",
    "eright",
    "europarchive.org",
    "evc-batch",
    "everyonesocialbot",
    "exabot",
    "experibot",
    "extlinksbot",
    "eyeotabot",
    "ezid",
    "ezooms",
    "facebookexternalhit",
    "facebot",
    "fedoraplanet",
    "feedbot",
    "feedfetcher-google",
    "feedly",
    "feedspot",
    "feedvalidator",
    "femtosearchbot",
    "fetch",
    "fever",
    "finditanswersbot",
    "findlink",
    "findthatfile",
    "findxbot",
    "flamingo_searchengine",
    "flipboardproxy",
    "fluffy",
    "freewebmonitoring sitechecker",
    "freshrss",
    "friendica",
    "fuelbot",
    "fyrebot",
    "g00g1e.net",
    "g2 web services",
    "g2reader-bot",
    "genieo",
    "gigablast",
    "gigabot",
    "gnam gnam spider",
    "gnowitnewsbot",
    "go-http-client",
    "google favicon",
    "google web preview",
    "google-adwords-instant",
    "google-certificates-bridge",
    "google-physicalweb",
    "google-site-verification",
    "google-structured-data-testing-tool",
    "google-xrawler",
    "googlebot",
    "googlebot-image",
    "googlebot-mobile",
    "googlebot-news",
    "googlebot-video",
    "gowikibot",
    "grobbot",
    "grouphigh",
    "grub.org",
    "gslfbot",
    "gwene",
    "hatena",
    "headlesschrome",
    "heritrix",
    "http_get",
    "httpunit",
    "httpurlconnection",
    "httpx",
    "httrack",
    "hubspot",
    "ia_archiver",
    "icbot",
    "ichiro",
    "imrbot",
    "indeedbot",
    "infoobot",
    "integromedb",
    "intelium_bot",
    "interfaxscanbot",
    "ips-agent",
    "iskanie",
    "istellabot",
    "james bot",
    "jamie's spider",
    "jetslide",
    "jetty",
    "jobboersebot",
    "jooblebot",
    "jpg-newsbot",
    "jyxobot",
    "k7mlwcbot",
    "kemvibot",
    "kosmiobot",
    "landau-media-spider",
    "laserlikebot",
    "lb-spider",
    "leikibot",
    "libwww-perl",
    "linguee bot",
    "linkapediabot",
    "linkarchiver",
    "linkdex",
    "linkedinbot",
    "linkisbot",
    "lipperhey",
    "livelap[bb]ot",
    "lssbot",
    "ltx71",
    "luminator-robots",
    "mail.ru_bot",
    "mappydata",
    "mastodon",
    "mauibot",
    "mediapartners (googlebot)",
    "mediapartners-google",
    "mediatoolkitbot",
    "megaindex",
    "meltwaternews",
    "memorybot",
    "metajobbot",
    "metauri",
    "mindupbot",
    "miniflux",
    "mixnodecache",
    "mj12bot",
    "mlbot",
    "moatbot",
    "mojeekbot",
    "moodlebot",
    "moreover",
    "msnbot",
    "msrbot",
    "muckrack",
    "multiviewbot",
    "naver blog rssbot",
    "nerdbynature.bot",
    "nerdybot",
    "netcraftsurveyagent",
    "netresearchserver",
    "netvibes",
    "newsharecounts",
    "newspaper",
    "nextcloud",
    "niki-bot",
    "nimbostratus-bot",
    "ning",
    "ninja bot",
    "nixstatsbot",
    "nmap scripting engine",
    "ntentbot",
    "nutch",
    "nuzzel",
    "ocarinabot",
    "officestorebot",
    "okhttp",
    "omgili",
    "online-webceo-bot",
    "openhosebot",
    "openindexspider",
    "orangebot",
    "outbrain",
    "outclicksbot",
    "page2rss",
    "pagepeeker",
    "pandalytics",
    "panscient",
    "paperlibot",
    "pcore-http",
    "petalbot",
    "phantomjs",
    "phpcrawl",
    "pinterest.com.bot",
    "piplbot",
    "pocketparser",
    "postrank",
    "pr-cy.ru",
    "primalbot",
    "privacyawarebot",
    "proximic",
    "psbot",
    "pulsepoint",
    "purebot",
    "python-requests",
    "python-urllib",
    "qwantify",
    "rankactivelinkbot",
    "redditbot",
    "refindbot",
    "regionstuttgartbot",
    "retrevopageanalyzer",
    "ridderbot",
    "rivva",
    "rogerbot",
    "rssbot",
    "rssingbot",
    "rytebot",
    "s[ee][mm]rushbot",
    "safednsbot",
    "sbl-bot",
    "scoutjet",
    "scrapy",
    "screaming frog seo spider",
    "scribdbot",
    "searchatlas",
    "seekbot",
    "seewithkids",
    "semanticbot",
    "semanticscholarbot",
    "sentibot",
    "seobilitybot",
    "seokicks",
    "seoscanners",
    "serendeputybot",
    "serpstatbot",
    "seznambot",
    "simplepie",
    "simplescraper",
    "sitebot",
    "siteexplorer.info",
    "siteimprove.com",
    "skypeuripreview",
    "slack-imgproxy",
    "slackbot",
    "slurp",
    "smtbot",
    "snacktory",
    "socialrankiobot",
    "sogou",
    "sonic",
    "spbot",
    "speedy",
    "startmebot",
    "storygizebot",
    "streamline3bot",
    "summify",
    "superfeedr",
    "surdotlybot",
    "surveybot",
    "swimgbot",
    "sysomos",
    "taboolabot",
    "tagoobot",
    "tangibleebot",
    "telegrambot",
    "teoma",
    "theoldreader.com",
    "thinklab",
    "tigerbot",
    "tineye",
    "tiny tiny rss",
    "toplistbot",
    "toutiaospider",
    "traackr.com",
    "tracemyfile",
    "trendictionbot",
    "trendsmapresolver",
    "trove",
    "turnitinbot",
    "tweetedtimes",
    "tweetmemebot",
    "twengabot",
    "twingly",
    "twitterbot",
    "twurly",
    "um-ln",
    "upflow",
    "uptimebot.org",
    "uptimerobot",
    "urlappendbot",
    "ut-dorkbot",
    "validator.nu",
    "vebidoobot",
    "veoozbot",
    "viber",
    "vigil",
    "virustotal",
    "vkrobot",
    "vkshare",
    "voilabot",
    "voluumdsp-content-bot",
    "w3c_css_validator",
    "w3c_i18n-checker",
    "w3c_unicorn",
    "w3c_validator",
    "w3c-checklink",
    "w3c-mobileok",
    "wbsearchbot",
    "web-archive-net.com.bot",
    "webdatastats",
    "webmon ",
    "wesee:search",
    "whatsapp",
    "wocbot",
    "woobot",
    "wordupinfosearch",
    "woriobot",
    "wotbox",
    "www.uptime.com",
    "xenu link sleuth",
    "xovibot",
    "y!j",
    "yacybot",
    "yadirectfetcher",
    "yahoo link preview",
    "yak",
    "yandexaccessibilitybot",
    "yandexadnet",
    "yandexblogs",
    "yandexbot",
    "yandexcalendar",
    "yandexdirect",
    "yandexfavicons",
    "yandexfordomain",
    "yandeximageresizer",
    "yandeximages",
    "yandexmarket",
    "yandexmedia",
    "yandexmetrika",
    "yandexmobilebot",
    "yandexmobilescreenshotbot",
    "yandexnews",
    "yandexontodb",
    "yandexpagechecker",
    "yandexpartner",
    "yandexrca",
    "yandexscreenshotbot",
    "yandexsearchshop",
    "yandexsitelinks",
    "yandexspravbot",
    "yandextracker",
    "yandexturbo",
    "yandexverticals",
    "yandexvertis",
    "yandexvideo",
    "yandexwebmaster",
    "yanga",
    "yeti",
    "yisouspider",
    "yoozbot",
    "zabbix",
    "zenback bot",
    "zgrab",
    "zoombot",
    "zoominfobot",
    "zumbot",
    "zuperlistbot",
];
