use crate::config::Config;
use crate::dev_profile::DevProfile;
use crate::jobs::{wait_for_next_cycle, DevJob, FailureType};
use crate::utils;
use futures::stream::{FuturesUnordered, StreamExt};
use tokio::time::Instant;
use tokio_postgres::Client as PgClient;
use tracing::{debug, error, info, instrument, warn};
use utils::{pgsql::get_pg_client, s3};
use uuid::Uuid;

/// Limited by how many S3 requests can be handled at a time
const MAX_NUMBER_OF_ACTIVE_DEV_JOBS: usize = 20;
/// Limited by the max load can be put on PG and ES
const MIN_CYCLE_DURATION_IN_MS: u64 = 10000;
/// Limited by the max load can be put on PG and ES
const MAX_NUMBER_OF_DEV_JOBS_TO_QUEUE_UP: i32 = 100;

/// Generates a combined developer report by merging all existing repo reports for that login and stores it in ES.
/// The merge requests come from DB DevJob queue.
pub(crate) async fn merge_devs_reports(mut config: Config) {
    info!("Merging dev reports already stored in S3 and store the results in S3 + ES.");

    // used to determine repeated errors and abort processing
    let mut err_counter = 0usize;
    const MAX_CONSECUTIVE_ERRORS: usize = 10;

    // try to get the jobs DB client (postgres)
    // this line panics if the connection fails
    let pg_client = get_pg_client(&config.job_queues.con_str).await;

    // track the time it takes for a single cycle to complete
    let mut main_loop_start = Instant::now();
    // set to false by no-jobs cycle and to true when there are jobs
    let mut log_sleep_msg = true;

    // enter an infinite loop of getting new jobs from the queue
    loop {
        // terminate the process if it keeps failing
        if err_counter >= MAX_CONSECUTIVE_ERRORS {
            error!("Too many errors. Exiting.");
            std::process::exit(1);
        }

        // renew the creds if needed
        config.renew_aws_credentials().await;

        // generate a unique ID for the current lot of jobs retrieved from the queue
        // it will be needed to update the job status later
        let report_in_flight_id = uuid::Uuid::new_v4();

        let qmsgs = match DevJob::get_new_for_report_generation(
            &pg_client,
            &report_in_flight_id,
            MAX_NUMBER_OF_DEV_JOBS_TO_QUEUE_UP,
        )
        .await
        {
            Err(_e) => {
                err_counter += 1;
                error!("Attempt {}", err_counter);
                continue;
            }
            Ok(v) => v,
        };

        // check if there need to be a delay before the next jobs call
        let qmsgs_len = qmsgs.len();
        if qmsgs_len == 0 {
            wait_for_next_cycle(&main_loop_start, log_sleep_msg, MIN_CYCLE_DURATION_IN_MS).await;
            log_sleep_msg = false;
        } else {
            // process all dev jobs received from the queue
            err_counter = process_devs(qmsgs, &config, &pg_client, &report_in_flight_id).await;
            // sleep to the end of the cycle if there were fewer jobs than the max allowed
            // to reduce the load on the DB server - the job selection query is quite expensive
            if qmsgs_len < MAX_NUMBER_OF_ACTIVE_DEV_JOBS as usize {
                wait_for_next_cycle(&main_loop_start, true, MIN_CYCLE_DURATION_IN_MS).await;
            }
            log_sleep_msg = true;
        }

        main_loop_start = Instant::now();
    }
}

/// Processes devs from the list of jobs and returns the error counter
async fn process_devs(
    owner_ids: Vec<String>,
    config: &Config,
    pg_client: &PgClient,
    report_in_flight_id: &Uuid,
) -> usize {
    let mut err_counter = 0usize;

    let mut owner_ids = owner_ids;

    // put the max allowed number of jobs into one container
    // use `idx` to mark the job # in the log
    let mut dev_jobs: FuturesUnordered<_> = owner_ids
        .drain(..MAX_NUMBER_OF_ACTIVE_DEV_JOBS.min(owner_ids.len()))
        .enumerate()
        .map(|(idx, owner_id)| process_dev(owner_id, config, idx))
        .collect();

    // a job counter to identify the job in the log
    let mut idx = MAX_NUMBER_OF_ACTIVE_DEV_JOBS;

    // loop through the active dev jobs
    loop {
        match dev_jobs.next().await {
            Some(job_result) => {
                // a job was completed
                match job_result {
                    Err(e) => {
                        match e {
                            FailureType::DoNotRetry(owner_id) => {
                                let _ = DevJob::mark_failed(&pg_client, &owner_id, report_in_flight_id).await;
                            }
                            FailureType::Retry(_) => {
                                // failed - retry by requeueing it later
                            }
                        }
                        err_counter += 1;
                    }
                    Ok(owner_id) => {
                        // the job succeeded
                        err_counter = 0;

                        // mark the job as completed in the DB
                        let _ = DevJob::mark_completed(&pg_client, &owner_id, report_in_flight_id).await;
                    }
                }

                // top up the futures queue with either a user or an org until they run out
                if let Some(owner_id) = owner_ids.pop() {
                    let dev_job = process_dev(owner_id, config, idx);
                    dev_jobs.push(dev_job);
                    info!("Added job {}", idx);
                    idx += 1;
                }
            }
            None => {
                // no more jobs left in the futures queue
                info!("All dev jobs processed");
                break;
            }
        }
    }

    err_counter
}

/// Merge all existing dev reports for the specified owner_id. Param `idx` is only used to identify the job #
/// in async execution for logging. Returns `owner_id` in Ok or Err.
#[instrument(skip(owner_id, config), name = "pd")]
pub(crate) async fn process_dev(owner_id: String, config: &Config, idx: usize) -> Result<String, FailureType<String>> {
    let dev_s3_key = match s3::build_dev_s3_key_from_owner_id(config, &owner_id) {
        Err(()) => {
            // there is something wrong with the key - def no point retrying with the same input
            return Err(FailureType::DoNotRetry(owner_id));
        }
        Ok(v) => v,
    };

    info!("Processing s3 key {}", dev_s3_key);

    // get the list of all objects in the dev's folder in S3
    // the trailing "/" is needed to make it the exact path match, e.g. "repos/ddd" matches "repos/ddd-retail", but "repos/ddd/" will be the exact match
    let dev_s3_objects = match utils::s3::list_objects_from_s3(
        config.s3_client(),
        &config.s3_bucket_private_reports,
        dev_s3_key.clone(),
        None,
    )
    .await
    {
        Ok(v) => v,
        Err(_) => return Err(FailureType::Retry(owner_id)),
    };

    // this could not arise if there were failures in S3 - the user def has no reports
    if dev_s3_objects.len() == 0 {
        warn!("No user objects in S3.");
        return Err(FailureType::DoNotRetry(owner_id));
    }

    // collect all combined project reports in the dev's folder
    let mut project_reports: Vec<s3::S3ObjectProps> = Vec::new();
    for s3_object in dev_s3_objects {
        debug!("Considering {}", s3_object.key);
        // is this a combined project report?
        if s3::is_combined_project_report(&s3_object.key, &owner_id) {
            info!("{} report for merging", s3_object.key);
            project_reports.push(s3_object);
            continue;
        }
    }

    // a dev may have no reports if they were deleted between the time the job was scheduled and now
    if project_reports.is_empty() {
        info! {"Found no reports for dev {}",owner_id};
        return Err(FailureType::DoNotRetry(owner_id));
    }

    // extract the last modified date and the key and ignore any that have no or invalid last modified date
    let mut project_reports = project_reports
        .into_iter()
        .filter_map(|report_s3_props| match s3::parse_date_header(&Some(report_s3_props.last_modified)) {
            Err(_) => None,
            Ok(v) => Some((v, report_s3_props.key)),
        })
        .collect::<Vec<(i64, String)>>();

    // sort the reports by date in the scending order so that the latest report comes last and overwrites any privacy settings of the earlier reports
    project_reports.sort_unstable_by(|a, b| a.0.cmp(&b.0));
    let report_s3_keys = project_reports.into_iter().map(|v| v.1).collect::<Vec<String>>();

    // merge multiple reports into a single dev profile
    let dev_profile = DevProfile::from_contributor_reports(report_s3_keys, &config, &owner_id).await?;
    let serialized_profile = dev_profile.to_vec()?;

    // save the profile in S3
    dev_profile.save_in_s3(&config, &serialized_profile).await?;

    // save the same serialized profile in ES
    if utils::elastic::upload_serialized_object_to_es(config, serialized_profile, &owner_id, &config.es_idx.dev)
        .await
        .is_err()
    {
        return Err(FailureType::Retry(owner_id));
    }

    Ok(owner_id)
}
