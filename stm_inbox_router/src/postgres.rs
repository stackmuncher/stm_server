use chrono::Utc;
use log::warn;
use serde::{Deserialize, Serialize};
use tokio_postgres::{Client, NoTls, Row};
use tracing::{debug, error, info};

/// Corresponds to table t_commit_ownership
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub(crate) struct CommitOwnership {
    pub owner_id: String,
    pub project_id: Option<String>,
    pub commit_hash: String,
    pub commit_ts: Option<chrono::DateTime<Utc>>,
}

impl From<&Row> for CommitOwnership {
    /// Creates a new structure from tokio_postgres::Row
    fn from(row: &Row) -> Self {
        Self {
            owner_id: row.get("owner_id"),
            project_id: row.get("project_id"),
            commit_hash: row.get("commit_hash"),
            commit_ts: row.get("commit_ts"),
        }
    }
}

impl CommitOwnership {
    /// Returns a list of all matching commit details, incl project, owner and timestamp.
    /// Do not use with an empty `commit_hash`.
    pub(crate) async fn find_matching_commits(
        pg_client: &Client,
        commit_hash: Vec<String>,
    ) -> Result<Vec<CommitOwnership>, ()> {
        // make sure the list is not empty
        if commit_hash.is_empty() {
            warn!("Empty list of commits to search for. It's a bug!");
            return Ok(Vec::new());
        }

        // get the data from PG
        let rows = match pg_client
            .query("select * from stm_find_projects_by_commits($1::varchar[])", &[&commit_hash])
            .await
        {
            Ok(v) => v,
            Err(e) => {
                error!("stm_find_projects_by_commits for {} failed with {}", &commit_hash[0], e);
                return Err(());
            }
        };

        // convert the PG rows into struct and return
        Ok(rows
            .iter()
            .map(|row| CommitOwnership::from(row))
            .collect::<Vec<CommitOwnership>>())
    }

    /// Adds a list of commits from a report.
    /// Every commit hash must have a corresponding timestamp or None.
    /// Do not use with en empty `commit_hash`.
    pub(crate) async fn add_commits(
        pg_client: &Client,
        owner_id: String,
        project_id: String,
        commit_hash: Vec<String>,
        commit_ts: Vec<Option<chrono::DateTime<Utc>>>,
    ) -> Result<(), ()> {
        // make sure the list is not empty
        if commit_hash.is_empty() {
            warn!("Empty list of commits to add to DB. It's a bug!");
            return Ok(());
        }

        info!("Adding {} commits starting from {}", commit_hash.len(), &commit_hash[0]);

        // push the data to PG, log the result, nothing to return
        let rows = match pg_client
            .execute(
                "select stm_add_commits($1::varchar, $2::varchar,$3::varchar,$4::timestamptz[])",
                &[&owner_id, &project_id, &commit_hash, &commit_ts],
            )
            .await
        {
            Ok(v) => v,
            Err(e) => {
                error!("stm_add_commits failed with {}", e);
                return Err(());
            }
        };

        debug!("Rows updated: {}", rows);
        Ok(())
    }
}

/// Prepare a client for Postgres connection. Panics if cannot connect to the PG DB.
pub(crate) async fn get_pg_client(connection_string: &String) -> tokio_postgres::Client {
    // try to connect to PG
    let (client, connection) = tokio_postgres::connect(connection_string, NoTls)
        .await
        .expect("Cannot connect to the DB.");

    // Spawn the object that performs the actual comms with the DB into its own thread.
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            error!("PG connection error: {}", e);
            panic!();
        }
    });
    debug!("client connected");

    // return the client to the caller
    client
}
