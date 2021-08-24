use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::time::{interval, Duration, Instant};
use tokio_postgres::{Client, Row};
use tracing::{debug, error, info};
use uuid::Uuid;

/// Helps decide on the best course of action for job processing.
/// The error must include the job it relates to for the job to be marked in the queue
/// for re-processing
pub(crate) enum FailureType<T> {
    /// Networking errors, scheduling conflicts.
    Retry(T),
    /// Data errors - corrupt or missing files.
    DoNotRetry(T),
}

/// Corresponds to `t_dev` table. All SPs and the table creation reside in stm_inbox project for consistency.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub(crate) struct DevJob {
    pub owner_id: String,
    pub report_ts: Option<chrono::DateTime<Utc>>,
    pub report_in_flight_id: Option<Uuid>,
    pub report_in_flight_ts: Option<chrono::DateTime<Utc>>,
    pub report_fail_counter: i32,
    pub last_submission_ts: Option<chrono::DateTime<Utc>>,
    pub gh_login: Option<String>,
    pub gh_login_gist_validation: Option<String>,
    pub gh_login_validation_ts: Option<chrono::DateTime<Utc>>,
    pub gh_login_gist_latest: Option<String>,
}

impl From<&Row> for DevJob {
    /// Creates a new structure from tokio_postgres::Row
    fn from(row: &Row) -> Self {
        Self {
            owner_id: row.get("owner_id"),
            report_ts: row.get("report_ts"),
            report_in_flight_id: row.get("report_in_flight_id"),
            report_in_flight_ts: row.get("report_in_flight_ts"),
            report_fail_counter: row.get("report_fail_counter"),
            last_submission_ts: row.get("last_submission_ts"),
            gh_login: row.get("gh_login"),
            gh_login_gist_validation: row.get("gh_login_gist_validation"),
            gh_login_validation_ts: row.get("gh_login_validation_ts"),
            gh_login_gist_latest: row.get("gh_login_gist_latest"),
        }
    }
}

impl DevJob {
    /// Marks the developer record as successfully completed and a new dev report generated.
    /// All SPs and the table creation for this method reside in stm_inbox project for consistency.
    pub(crate) async fn mark_completed(
        pg_client: &Client,
        owner_id: &String,
        report_in_flight_id: &Uuid,
        gh_login: &Option<String>,
        gh_login_gist_validation: &Option<String>,
    ) -> Result<(), ()> {
        info!("Marking report dev completed {}", owner_id);

        // push the data to PG, log the result, nothing to return
        let rows = match pg_client
            .execute(
                "select stm_complete_dev_job($1::varchar, $2::uuid, $3::varchar, $4::varchar)",
                &[owner_id, report_in_flight_id, gh_login, gh_login_gist_validation],
            )
            .await
        {
            Ok(v) => v,
            Err(e) => {
                error!("stm_complete_dev_job failed with {}", e);
                return Err(());
            }
        };

        debug!("Rows updated: {}", rows);
        Ok(())
    }

    /// Marks the developer record as failed and no new dev report generated.
    /// All SPs and the table creation for this method reside in stm_inbox project for consistency.
    pub(crate) async fn mark_failed(
        pg_client: &Client,
        owner_id: &String,
        report_in_flight_id: &Uuid,
    ) -> Result<(), ()> {
        info!("Marking report dev completed {}", owner_id);

        // push the data to PG, log the result, nothing to return
        let rows = match pg_client
            .execute("select stm_give_up_on_dev($1::varchar, , $2::uuid)", &[owner_id, report_in_flight_id])
            .await
        {
            Ok(v) => v,
            Err(e) => {
                error!("stm_give_up_on_dev failed with {}", e);
                return Err(());
            }
        };

        debug!("Rows updated: {}", rows);
        Ok(())
    }

    /// Returns a list of owner_ids with new submissions or missing reports to generate a new combined report for each.
    /// All SPs and the table creation for this method reside in stm_inbox project for consistency.
    pub(crate) async fn get_new_for_report_generation(
        pg_client: &Client,
        report_in_flight_id: &Uuid,
        jobs_max: i32,
    ) -> Result<Vec<DevJob>, ()> {
        debug!("Getting dev report jobs for {}", report_in_flight_id);

        // get the data from PG
        let rows = match pg_client
            .query("select * from stm_get_dev_jobs($1::UUID, $2::integer)", &[report_in_flight_id, &jobs_max])
            .await
        {
            Ok(v) => v,
            Err(e) => {
                error!("stm_get_dev_jobs failed with {}", e);
                return Err(());
            }
        };

        // check if the result makes sense
        let row_count = rows.len();
        debug!("Rows: {}", row_count);

        Ok(rows.iter().map(|row| DevJob::from(row)).collect::<Vec<DevJob>>())
    }
}

/// Puts the thread to sleep for the remainder of the minimum time between job requests to a DB-based queue
/// Returns the current timestamp to be used as the start of the new cycle.
pub(crate) async fn wait_for_next_cycle(
    main_loop_start: &Instant,
    log_sleep_msg: bool,
    min_cycle_duration_in_ms: u64,
) -> Instant {
    let last_loop_duration = main_loop_start.elapsed();
    debug!("Last loop duration: {}", last_loop_duration.as_millis());

    let duration_min = Duration::from_millis(min_cycle_duration_in_ms);

    if last_loop_duration < duration_min {
        let wait_for_next_cycle = duration_min - last_loop_duration + Duration::from_millis(1);
        // only log this message once per no-jobs cycle to avoid log pollution
        if log_sleep_msg {
            info!("{}ms sleep delay between cycles", wait_for_next_cycle.as_millis());
        }
        let mut search_delay = interval(wait_for_next_cycle);
        search_delay.tick().await;
        search_delay.tick().await;
    }

    Instant::now()
}
