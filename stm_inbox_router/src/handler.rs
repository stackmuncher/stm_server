use crate::config::Config;
use crate::postgres::{CommitOwnership, Dev, EmailOwnership};
use crate::s3::{copy_within_s3, delete_s3_object, get_bytes_from_s3, S3Event, REPORT_FILE_EXT_IN_S3};
use bs58;
use flate2::read::GzDecoder;
use futures::stream::{FuturesUnordered, StreamExt};
use lambda_runtime::{Context, Error};
use log::info;
use serde_json::Value;
use stackmuncher_lib::report::Report;
use std::collections::{HashMap, HashSet};
use std::io::Read;
use tracing::{debug, error, warn};
use unicode_segmentation::UnicodeSegmentation;

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

    // extract the owner id from a key like this `queue/1621680890_7prBWD7pzYk2czeXZeXzjxjDQbnuka2RLShdW5AxWuk7.gz`
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

    // this should already be validated, but check just in case
    if !validate_owner_id(&owner_id) {
        return Err(Error::from(format!("Invalid owner_id length: {}", owner_id)));
    }

    // read and unzip the report from S3
    let report = get_bytes_from_s3(config, s3_key.clone()).await?;
    let mut decoder = GzDecoder::new(report.as_slice());
    let mut buffer: Vec<u8> = Vec::new();
    let len = decoder.read_to_end(&mut buffer)?;

    info!("Decoded {} bytes", len);

    // load the file into a report struct
    let report = serde_json::from_slice::<Report>(buffer.as_slice())?;

    // compile the full list of user emails and mark the primary email as such
    // the primary email may or may not be in the list of git IDs
    let mut user_emails = report
        .git_ids_included
        .iter()
        .filter_map(|email| {
            if let Some(email) = validate_email_address(email) {
                Some((email, false))
            } else {
                warn!("Invalid email: {}", email);
                None
            }
        })
        .collect::<HashMap<String, bool>>();
    // add the primary email, if any
    // An empty string means NO CONTACT - see https://github.com/stackmuncher/stm_server/issues/16
    if let Some(email) = &report.primary_email {
        if let Some(email) = validate_email_address(email) {
            user_emails.insert(email, true);
        } else {
            warn!("Invalid primary email: {}", email);
        }
    }

    // make a list of email jobs to update the DB
    let mut email_jobs: FuturesUnordered<_> = user_emails
        .iter()
        .map(|email| EmailOwnership::add_email(&config.pg_client, &owner_id, &email.0, email.1))
        .collect();

    // validate the latest commit SHA1
    let last_contributor_commit_sha1 = report.last_contributor_commit_sha1.unwrap_or_default();
    if last_contributor_commit_sha1.len() != 40
        || !config.commit_hash_regex_full.is_match(&last_contributor_commit_sha1)
    {
        // something's off here - no point proceeding
        error!("Invalid latest report commit: {}", last_contributor_commit_sha1);
        return Ok(());
    }

    // get the list of recent project commits
    let commit_list = match report.recent_project_commits.as_ref() {
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
        if let Some(commit) = validate_short_commit_hash(commit, config) {
            valid_commits.insert(commit.0, commit.1);
        } else {
            // something's off here - no point processing this report any further
            error!("Invalid commit: {}", commit);
            return Ok(());
        }
    }

    // get a list of hashes to search for existing projects
    // hashmap should give us a randomized list, but we may need to adjust its size
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

    // move it to the member's folder
    // the source has the timestamp of the submission in the name, but the dest should have the timestamp of the last commit
    let copy_with_ts = copy_within_s3(
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
            last_contributor_commit_sha1.as_str(),
            REPORT_FILE_EXT_IN_S3,
        ]
        .concat(),
    );

    // short-circuit the processing here if it's an out of order report
    // it may be possible that someone makes a commit with a date 100 years ahead in the future
    // then that commit will always be the latest and the project will never update according to this logic
    if latest_report_commit_ts < latest_project_commit_ts {
        warn!(
            "Out of order report for {}/{}. Latest commit ts in PG: {}, report: {}",
            owner_id, project_id, latest_project_commit_ts, latest_report_commit_ts
        );

        // copy the report with the timestamp, but do not update the latest report for the project
        // because this one arrived out of order
        copy_with_ts.await?;
        delete_s3_object(config, s3_key.clone()).await?;
        return Ok(());
    }

    // copy it again as the latest report with a predefined file name
    let copy_latest = copy_within_s3(
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
    );

    // copy both concurrently
    let copy_results = futures::join!(copy_with_ts, copy_latest);
    if copy_results.0.is_err() || copy_results.1.is_err() {
        return Err(Error::from("Failed to copy reports."));
    }

    // mark the developer record for re-processing
    Dev::queue_up_for_update(&config.pg_client, &owner_id, &report.gh_validation_id).await?;

    // drive email insertion jobs to completion
    let mut email_addition_failed = false;
    loop {
        match email_jobs.next().await {
            Some(job_result) => {
                // a job was completed
                if job_result.is_err() {
                    email_addition_failed = true;
                }
            }
            None => {
                // no more jobs left in the futures queue
                info!("All repo jobs processed");
                break;
            }
        }
    }

    if email_addition_failed {
        return Err(Error::from("Failed to add one or more email addresses to t_email_ownership"));
    }

    // delete the submission from inbox queue
    delete_s3_object(config, s3_key.clone()).await?;

    Ok(())
}

/// Returns a cleaned up and normalized email address or None if the address doesn't seem to be deliverable.
/// The length must be between 4 and 150 unicode chars. This validation is specific for the purpose of this module and the DB constraints.
fn validate_email_address(email: &String) -> Option<String> {
    let email = email.trim();
    let email = email.trim().to_lowercase();
    if email.len() < 4
        || email.split("@").count() != 2
        || email.contains(" ")
        || email.contains("\n")
        || email.contains("\r")
        || email.contains("\t")
        || email.contains("\\")
        || email.contains("\0")
    {
        return None;
    }

    // Postgres DB does not allow more than 150 unicode chars per email.
    // A longer than that email address is probably meaningless and would get stuck in the pipes.
    let unicode_char_count = email.graphemes(true).count();
    if unicode_char_count > 150 {
        return None;
    }

    Some(email.to_lowercase())
}

/// Returns a tuple with a valid commit hash and a timestamp if they seem to be valid.
/// The timestamp can actually be any valid i64 number and not a realistic date.
fn validate_short_commit_hash(commit_hash_with_ts: &String, config: &Config) -> Option<(String, i64)> {
    let split = commit_hash_with_ts.split("_").collect::<Vec<&str>>();
    if split.len() != 2 {
        return None;
    }

    if split[0].len() != 8 || !config.commit_hash_regex_short.is_match(split[0]) {
        return None;
    }

    // there should be no commits with no dates
    if let Ok(ts) = i64::from_str_radix(split[1], 10) {
        return Some((split[0].to_string(), ts));
    }

    None
}

/// Returns TRUE if the owner_id decodes from base58 into exactly 256 bytes.
/// Logs a warning and returns FALSE otherwise.
/// TODO: this should be a shared utility function!!!
fn validate_owner_id(owner_id: &str) -> bool {
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
