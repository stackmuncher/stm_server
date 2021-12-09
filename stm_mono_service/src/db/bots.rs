use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio_postgres::{Client, Row};
use tracing::{debug, error, info};

/// Corresponds to table t_ip_log
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub(crate) struct IpLog {
    /// IP v4 or v6 as string
    pub ip: String,
    /// Number of times the IP was encountered.
    /// * from DB: the value stored in the DB
    /// * to DB: the value to add to what is in the DB
    pub cnt: i64,
    /// Set when the the record was first created.
    pub added_ts: chrono::DateTime<Utc>,
    /// Set on the last update.
    pub latest_ts: chrono::DateTime<Utc>,
}

impl From<&Row> for IpLog {
    /// Creates a new structure from tokio_postgres::Row
    fn from(row: &Row) -> Self {
        Self {
            ip: row.get("ip"),
            cnt: row.get("cnt"),
            added_ts: row.get("ts_added"),
            latest_ts: row.get("ts_latest"),
        }
    }
}

impl IpLog {
    /// Adds an IP address to t_ip_log table or updates the existing one.
    /// * `added_ts`, `latest_ts` - set both to the TS of the log record
    /// * `cnt` - set it to the number of times the IP was encountered in the log file for adding the value to what
    /// is already in the DB.
    ///
    /// The log reader does not know when the IP was first added, so this field is only set on INSERT.
    /// `latest_ts` always updates the record.
    pub(crate) async fn add_or_update(ip_logs: Vec<IpLog>, pg_client: &Client) -> Result<(), ()> {
        info!("Adding {} IPs", ip_logs.len());
        if ip_logs.is_empty() {
            return Ok(());
        }

        for ip_log in &ip_logs {
            info!("IP record: {} / {} / {} / {}", ip_log.ip, ip_log.cnt, ip_log.added_ts, ip_log.latest_ts);
        }

        // collect the columns into their own arrays
        let ip = ip_logs.iter().map(|v| v.ip.clone()).collect::<Vec<String>>();
        let cnt = ip_logs.iter().map(|v| v.cnt.clone()).collect::<Vec<i64>>();
        let added_ts = ip_logs
            .iter()
            .map(|v| v.added_ts.clone())
            .collect::<Vec<DateTime<Utc>>>();
        let latest_ts = ip_logs
            .iter()
            .map(|v| v.latest_ts.clone())
            .collect::<Vec<DateTime<Utc>>>();

        // push the data to PG, log the result, nothing to return
        let rows = match pg_client
            .execute(
                "select stm_add_ip_log($1::varchar[], $2::bigint[], $3::timestamptz[], $4::timestamptz[])",
                &[&ip, &cnt, &added_ts, &latest_ts],
            )
            .await
        {
            Ok(v) => v,
            Err(e) => {
                error!("stm_add_ip_log failed with {}", e);
                return Err(());
            }
        };

        debug!("Rows updated: {}", rows);
        Ok(())
    }

    /// Returns a list of all IPs from t_ip_log.
    pub(crate) async fn get_all_ips(pg_client: &Client) -> Result<Vec<String>, ()> {
        debug!("Getting all IPs");

        // get the data from PG
        let rows = match pg_client.query("select * from stm_get_all_ips()", &[]).await {
            Ok(v) => v,
            Err(e) => {
                error!("stm_get_all_ips failed with {}", e);
                return Err(());
            }
        };

        // check if the result makes sense
        let row_count = rows.len();
        debug!("Rows: {}", row_count);

        Ok(rows.iter().map(|row| row.get(0)).collect::<Vec<String>>())
    }
}
