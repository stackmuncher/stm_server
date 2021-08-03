use crate::config::Config;
use crate::postgres::CommitOwnership;
use crate::s3::{copy_within_s3, delete_s3_object, get_bytes_from_s3, S3Event, REPORT_FILE_EXT_IN_S3};
use bs58;
use flate2::read::GzDecoder;
use lambda_runtime::{Context, Error};
use log::info;
use serde_json::Value;
use stackmuncher_lib::report::Report;
use std::collections::{HashMap, HashSet};
use std::io::Read;
use tracing::{debug, error, warn};

pub(crate) async fn my_handler(event: Value, ctx: Context, config: &Config) -> Result<(), Error> {
    // these 2 lines are for debugging only to see the raw request
    debug!("Event: {}", event);
    debug!("Context: {:?}", ctx);

    // convert the event into a struct
    let event = match serde_json::from_value::<S3Event>(event) {
        Ok(v) => v,
        Err(e) => {
            error!("Cannot deser S3 event with {}", e);
            return Err(Error::from(e));
        }
    };

    // the number of records should always be 1
    if event.records.len() != 1 {
        return Err(Error::from(format!("Wrong number of S3 records: {}", event.records.len())));
    }

    // the object key should always be present for this type of call
    let s3_key = match event.records[0].s3.object.key.as_ref() {
        Some(v) => v.clone(),
        None => {
            return Err(Error::from("Empty object key in the event details"));
        }
    };

    // required to ID the transaction in the log, otherwise it's not known which report failed
    info!("S3 key: {}", s3_key);

    // extract the owner id from a key like this `queue/1621680890_7prBWD7pzYk2czeXZeXzjxjDQbnuka2RLShdW5AxWuk7.gzip`
    let owner_id = match s3_key.split("_").last() {
        Some(v) => v,
        None => {
            return Err(Error::from("Failed to split the key at _ as pub_key.ext"));
        }
    };
    let owner_id = match owner_id.split(".").next() {
        Some(v) => v.to_owned(),
        None => {
            return Err(Error::from("Failed to split the key at . as pub_key.ext"));
        }
    };

    // check if the object has any contents
    if event.records[0].s3.object.size.unwrap_or_default() == 0 {
        return Err(Error::from(format!("Zero-sized object: {}", s3_key)));
    }

    info!("OwnerID: {}", owner_id);

    // read and unzip the report from S3
    let report = get_bytes_from_s3(config, s3_key.clone()).await?;
    let mut decoder = GzDecoder::new(report.as_slice());
    let mut buffer: Vec<u8> = Vec::new();
    let len = decoder.read_to_end(&mut buffer)?;

    info!("Decoded {} bytes", len);

    // load the file into a report struct
    let report = serde_json::from_slice::<Report>(buffer.as_slice())?;

    // check if there is just one project included in the report
    if report.projects_included.len() != 1 {
        error!("Wrong number of projects in the report: {}", report.projects_included.len());
        return Ok(());
    }

    // validate the latest commit SHA1
    let latest_report_commit_sha1 = report.last_contributor_commit_sha1.unwrap_or_default();
    if latest_report_commit_sha1.len() != 40 || !config.commit_hash_regex_full.is_match(&latest_report_commit_sha1) {
        // something's off here - no point proceeding
        error!("Invalid latest report commit: {}", latest_report_commit_sha1);
        return Ok(());
    }

    // get the list of commits a few levels down into the hierarchy
    let commit_list = match report
        .projects_included
        .iter()
        .next()
        .as_ref()
        .expect("Cannot unwrap included project. It's a bug.")
        .commits
        .as_ref()
    {
        Some(v) => v,
        None => {
            info!("No commit details found.");
            return Ok(());
        }
    };

    debug!("{}", commit_list.join(", "));

    // split the commits into hash and timestamp parts
    let mut valid_commits: HashMap<String, i64> = HashMap::new();
    // a valid commit looks like this: 7474684a_1595904770
    // anything else is either a bug or some other kind of data corruption
    for commit in commit_list {
        let split = commit.split("_").collect::<Vec<&str>>();
        if split.len() == 2 && split[0].len() == 8 && config.commit_hash_regex_short.is_match(split[0]) {
            // there should be no commits with no dates
            if let Ok(ts) = i64::from_str_radix(split[1], 10) {
                valid_commits.insert(split[0].to_string(), ts);
            } else {
                // something's off here - no point processing this report any further
                error!("Invalid commit date: {}", commit);
                return Ok(());
            };
        } else {
            // something's off here - no point processing this report any further
            error!("Invalid commit: {}", commit);
            return Ok(());
        }
    }

    // get a list of hashes to search for existing projects
    let commit_hashes_for_search = valid_commits
        .keys()
        .take(valid_commits.keys().len().min(50))
        .collect::<Vec<&String>>();

    // search for project matches by commit
    let commit_ownerships = CommitOwnership::find_matching_commits(&config.pg_client, commit_hashes_for_search).await?;

    info!("Found {} matching commits in PG", commit_ownerships.len());

    // collect matching project IDs
    let mut project_ids: HashSet<String> = HashSet::new();
    for ownership in commit_ownerships {
        // check if the commits match on the date as well
        if let Some(commit_ts) = valid_commits.get(&ownership.commit_hash) {
            if commit_ts == &ownership.commit_ts {
                project_ids.insert(ownership.project_id);
            }
        }
    }

    // a Vec is easier to work with
    let mut project_ids = project_ids.into_iter().collect::<Vec<String>>();
    info!("Found matching projects: {}", project_ids.join(","));

    // get or generate the project ID
    let project_id = match project_ids.len() {
        0 => {
            // generate a new one
            bs58::encode(uuid::Uuid::new_v4().as_bytes()).into_string()
        }
        1 => {
            // use existing
            project_ids.pop().expect("Failed to unwrap project_id")
        }
        _ => {
            // resolve conflicts, but just log an error for now
            error!("Project ID conflict resolution is not implemented.");
            return Ok(());
        }
    };

    info!("ProjectID: {}", project_id);

    // split all all known commits into commit/timestamp and add them all to the DB
    let mut commit_hashes: Vec<String> = Vec::new();
    let mut commit_timestamps: Vec<i64> = Vec::new();
    for commit in valid_commits {
        commit_hashes.push(commit.0);
        commit_timestamps.push(commit.1);
    }
    CommitOwnership::add_commits(&config.pg_client, &owner_id, &project_id, &commit_hashes, &commit_timestamps).await?;

    // check if this report is the latest known for this project
    let latest_report_commit_ts = report.last_contributor_commit_date_epoch.unwrap_or_default();
    let latest_project_commit_ts =
        CommitOwnership::get_latest_project_commit(&config.pg_client, &owner_id, &project_id).await?;

    if latest_report_commit_ts < latest_project_commit_ts {
        warn!(
            "Out of order report for {}/{}. Latest commit ts in PG: {}, report: {}",
            owner_id, project_id, latest_project_commit_ts, latest_report_commit_ts
        );
    }

    // it is the latest - move it to the member's folder
    // the source has the timestamp of the submission in the name, but the dest should have the timestamp of the last commit
    copy_within_s3(
        config,
        s3_key.clone(),
        [
            config.s3_report_prefix.as_str(),
            "/",
            owner_id.as_str(),
            "/",
            project_id.as_str(),
            "/",
            latest_report_commit_ts.to_string().as_str(),
            "_",
            latest_report_commit_sha1.as_str(),
            REPORT_FILE_EXT_IN_S3,
        ]
        .concat(),
    )
    .await?;

    // copy it again as the latest report with a predefined file name
    copy_within_s3(
        config,
        s3_key.clone(),
        [
            config.s3_report_prefix.as_str(),
            "/",
            owner_id.as_str(),
            "/",
            project_id.as_str(),
            "/report",
            REPORT_FILE_EXT_IN_S3,
        ]
        .concat(),
    )
    .await?;

    // delete the submission from inbox queue
    delete_s3_object(config, s3_key.clone()).await?;

    Ok(())
}
