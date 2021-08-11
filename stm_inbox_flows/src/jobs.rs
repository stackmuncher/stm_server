use tokio::time::{interval, Duration, Instant};
use tokio_postgres::Client;
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
pub(crate) struct DevJob {}

impl DevJob {
    /// Marks the developer record as successfully completed and a new dev report generated.
    /// All SPs and the table creation for this method reside in stm_inbox project for consistency.
    pub(crate) async fn mark_completed(
        pg_client: &Client,
        owner_id: &String,
        report_in_flight_id: &Uuid,
    ) -> Result<(), ()> {
        info!("Marking report dev completed {}", owner_id);

        // push the data to PG, log the result, nothing to return
        let rows = match pg_client
            .execute("select stm_complete_dev_job($1::varchar, , $2::uuid)", &[owner_id, report_in_flight_id])
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
    ) -> Result<Vec<String>, ()> {
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

        Ok(rows
            .into_iter()
            .filter_map(|row| match row.try_get(0) {
                Ok(v) => Some(v),
                Err(e) => {
                    error!("Cannot get owner_id from stm_get_dev_jobs for {} with {}", report_in_flight_id, e);
                    None
                }
            })
            .collect::<Vec<String>>())
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
