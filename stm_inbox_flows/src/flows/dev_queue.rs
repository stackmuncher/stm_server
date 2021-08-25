use crate::config::Config;
use crate::dev_profile::{DevProfile, GitHubUser};
use crate::jobs::{wait_for_next_cycle, DevJob, FailureType};
use crate::utils;
use chrono::{Duration, Utc};
use futures::stream::{FuturesUnordered, StreamExt};
use tokio::time::Instant;
use tokio_postgres::Client as PgClient;
use tracing::{debug, error, info, instrument};
use utils::{pgsql::get_pg_client, s3};
use uuid::Uuid;

/// Limited by how many S3 requests can be handled at a time
const MAX_NUMBER_OF_ACTIVE_DEV_JOBS: usize = 20;
/// Limited by the max load can be put on PG and ES
const MIN_CYCLE_DURATION_IN_MS: u64 = 10000;
/// Limited by the max load can be put on PG and ES
const MAX_NUMBER_OF_DEV_JOBS_TO_QUEUE_UP: i32 = 100;
/// Validity period for gh_login revalidation
const GH_LOGIN_VALIDITY_PERIOD_DAYS: i64 = 30;

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
    dev_jobs: Vec<DevJob>,
    config: &Config,
    pg_client: &PgClient,
    report_in_flight_id: &Uuid,
) -> usize {
    let mut err_counter = 0usize;

    let mut dev_jobs = dev_jobs;

    // put the max allowed number of jobs into one container
    // use `idx` to mark the job # in the log
    let mut dev_jobs_futures: FuturesUnordered<_> = dev_jobs
        .drain(..MAX_NUMBER_OF_ACTIVE_DEV_JOBS.min(dev_jobs.len()))
        .enumerate()
        .map(|(idx, dev_job)| process_dev(dev_job, config, idx))
        .collect();

    // a job counter to identify the job in the log
    let mut idx = MAX_NUMBER_OF_ACTIVE_DEV_JOBS;

    // loop through the active dev jobs
    loop {
        match dev_jobs_futures.next().await {
            Some(job_result) => {
                // a job was completed
                match job_result {
                    Err(e) => {
                        match e {
                            FailureType::DoNotRetry(dev_job) => {
                                let _ = DevJob::mark_failed(&pg_client, &dev_job.owner_id, report_in_flight_id).await;
                            }
                            FailureType::Retry(_) => {
                                // failed - retry by requeueing it later
                            }
                        }
                        err_counter += 1;
                    }
                    Ok(dev_job) => {
                        // the job succeeded
                        err_counter = 0;

                        // mark the job as completed in the DB
                        let _ = DevJob::mark_completed(
                            &pg_client,
                            &dev_job.owner_id,
                            report_in_flight_id,
                            &dev_job.gh_login,
                            &dev_job.gh_login_gist_validation,
                        )
                        .await;
                    }
                }

                // top up the futures queue with either a user or an org until they run out
                if let Some(dev_job) = dev_jobs.pop() {
                    let dev_job = process_dev(dev_job, config, idx);
                    dev_jobs_futures.push(dev_job);
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
/// in async execution for logging. Returns an updated `DevJob` in Ok or Err.
#[instrument(skip(dev_job, config), name = "pd")]
pub(crate) async fn process_dev(dev_job: DevJob, config: &Config, idx: usize) -> Result<DevJob, FailureType<DevJob>> {
    // check if gh_login needs to be discovered or re-validated
    // this could be an async task, but it is not expected to be called often enough to warrant that
    let dev_job = add_gh_login(dev_job, config).await;

    // get a key for dev's private reports folder
    let dev_s3_key = match s3::build_dev_s3_key_from_owner_id(&dev_job.owner_id) {
        Err(()) => {
            // there is something wrong with the key - def no point retrying with the same input
            return Err(FailureType::DoNotRetry(dev_job));
        }
        Ok(v) => v,
    };

    info!("Processing private s3 key {}", dev_s3_key);

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
        Err(_) => return Err(FailureType::Retry(dev_job)),
    };

    // get the list of objects for GH repos/reports for dev'g GH login, if any
    let dev_gh_s3_objects = match &dev_job.gh_login {
        Some(gh_login) => {
            // get a key for dev's GitHub reports folder
            let dev_gh_s3_key = match s3::build_dev_s3_key_from_gh_login(gh_login, config.gh_login_invalidation_regex())
            {
                Err(()) => {
                    // there is something wrong with the key - def no point retrying with the same input
                    return Err(FailureType::DoNotRetry(dev_job));
                }
                Ok(v) => v,
            };

            info!("Processing GH s3 key {}", dev_gh_s3_key);

            // get the list of all objects in the dev's folder in S3
            // the trailing "/" is needed to make it the exact path match, e.g. "repos/ddd" matches "repos/ddd-retail", but "repos/ddd/" will be the exact match
            match utils::s3::list_objects_from_s3(
                config.s3_client(),
                &config.s3_bucket_gh_reports,
                dev_gh_s3_key.clone(),
                None,
            )
            .await
            {
                Ok(v) => v,
                Err(_) => return Err(FailureType::Retry(dev_job)),
            }
        }
        None => Vec::new(),
    };

    // collect all combined project reports in the dev's private folder
    let mut private_reports: Vec<s3::S3ObjectProps> = Vec::new();
    for s3_object in dev_s3_objects {
        debug!("Considering private: {}", s3_object.key);
        // is this a combined project report?
        if s3::is_combined_project_report(&s3_object.key, &dev_job.owner_id) {
            info!("{} privae report for merging", s3_object.key);
            private_reports.push(s3_object);
            continue;
        }
    }
    // extract the last modified date and the key and ignore any that have no or invalid last modified date
    // sort the reports by date in the scending order so that the latest report comes last and overwrites any privacy settings of the earlier reports
    let mut private_reports = private_reports
        .into_iter()
        .filter_map(|report_s3_props| match s3::parse_date_header(&Some(report_s3_props.last_modified)) {
            Err(_) => None,
            Ok(v) => Some((v, report_s3_props.key)),
        })
        .collect::<Vec<(i64, String)>>();
    // TODO: investigate if FuturesUnordered changes the sorting <<< POSSIBLE BUG!!! <<< POSSIBLE BUG!!! <<< POSSIBLE BUG!!! <<< POSSIBLE BUG!!!
    private_reports.sort_unstable_by(|a, b| a.0.cmp(&b.0));
    let private_report_s3_keys = private_reports.into_iter().map(|v| v.1).collect::<Vec<String>>();

    // collect all combined project reports in the dev's GH folder
    let mut gh_reports: Vec<s3::S3ObjectProps> = Vec::new();
    let mut gh_user_profile_s3_key: Option<String> = None;
    for s3_object in dev_gh_s3_objects {
        debug!("Considering gh: {}", s3_object.key);
        // is this a combined project report?
        if s3::is_gh_repo_report_name(&s3_object.key) {
            info!("{} gh report for merging", s3_object.key);
            gh_reports.push(s3_object);
            continue;
        } else if s3_object.key.ends_with(s3::S3_OBJ_NAME_GH_USER) {
            gh_user_profile_s3_key = Some(s3_object.key);
        }
    }
    // extract the last modified date and the key and ignore any that have no or invalid last modified date
    // sort the reports by date in the scending order so that the latest report comes last and overwrites any privacy settings of the earlier reports
    let mut gh_reports = gh_reports
        .into_iter()
        .filter_map(|report_s3_props| match s3::parse_date_header(&Some(report_s3_props.last_modified)) {
            Err(_) => None,
            Ok(v) => Some((v, report_s3_props.key)),
        })
        .collect::<Vec<(i64, String)>>();
    gh_reports.sort_unstable_by(|a, b| a.0.cmp(&b.0));
    let gh_report_s3_keys = gh_reports.into_iter().map(|v| v.1).collect::<Vec<String>>();

    // merge multiple reports into a single dev profile
    // a dev may have no reports if they were deleted between the time the job was scheduled and now
    // the merge will produce a dev profile with no reports
    let combined_report = match DevProfile::from_contributor_reports(
        private_report_s3_keys,
        gh_report_s3_keys,
        &config,
        &dev_job.owner_id,
    )
    .await
    {
        Ok(v) => v,
        Err(_) => {
            return Err(FailureType::DoNotRetry(dev_job));
        }
    };

    // load either GH User Profile or a trimmed down private profile, add the combined report to it and convert into Vec<u8>
    let (serialized_profile, es_object_id) = match gh_user_profile_s3_key {
        Some(gh_user_profile_s3_key) => {
            let mut profile = match GitHubUser::from_s3(config, gh_user_profile_s3_key).await {
                Ok(v) => v,
                Err(_) => {
                    error!("Failed to load user GitHub profile from S3");
                    return Err(FailureType::Retry(dev_job));
                }
            };
            profile.report = combined_report;
            (profile.to_vec(), profile.node_id.clone())
        }
        None => (DevProfile::new(combined_report, &dev_job.owner_id).to_vec(), dev_job.owner_id.clone()),
    };

    // check if we have a profile to save
    let serialized_profile = match serialized_profile {
        Ok(v) => v,
        Err(_) => {
            return Err(FailureType::DoNotRetry(dev_job));
        }
    };

    // save the same serialized profile in ES
    if utils::elastic::upload_serialized_object_to_es(config, serialized_profile, &es_object_id, &config.es_idx.dev)
        .await
        .is_err()
    {
        return Err(FailureType::Retry(dev_job));
    }

    Ok(dev_job)
}

/// Checks if there is new GitHub login validation ID or the previous ID is due for revalidation.
/// Returns the original DevJob is no changes were made or adds new GitHub login details.
async fn add_gh_login(dev_job: DevJob, config: &Config) -> DevJob {
    if dev_job.gh_login_gist_latest != dev_job.gh_login_gist_validation
        || dev_job.gh_login_validation_ts.is_none()
        || Utc::now()
            - dev_job
                .gh_login_validation_ts
                .expect("Cannot unwrap gh_login_validation_ts. It's a bug.")
            > Duration::days(GH_LOGIN_VALIDITY_PERIOD_DAYS)
    {
        let gh_login = crate::gh_login::get_validated_gist(
            &dev_job.gh_login_gist_latest,
            &dev_job.owner_id,
            config.gh_login_invalidation_regex(),
        )
        .await;

        info!("gh_login revalidation. Old: {:?}, new: {:?}", dev_job.gh_login, gh_login);

        // return DevJob with updated gh_login details, gh_login_validation_ts will be set by the stored procedure
        DevJob {
            gh_login,
            gh_login_gist_validation: dev_job.gh_login_gist_latest.clone(),
            ..dev_job
        }
    } else {
        dev_job
    }
}
