use lambda_runtime::Error;
use log::warn;
use serde::{Deserialize, Serialize};
use tokio_postgres::{Client, NoTls, Row};
use tracing::{debug, error, info};

/// Corresponds to table t_commit_ownership
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub(crate) struct CommitOwnership {
    pub owner_id: String,
    pub project_id: String,
    pub commit_hash: String,
    pub commit_ts: i64,
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

/// Corresponds to `t_email_ownership` table
pub(crate) struct EmailOwnership {}

/// Corresponds to `t_dev` table
pub(crate) struct Dev {}

impl CommitOwnership {
    /// Returns a list of all matching commit details, incl project, owner and timestamp.
    /// Do not use with an empty `commit_hash`.
    pub(crate) async fn find_matching_commits(
        pg_client: &Client,
        commit_hashes: Vec<&String>,
    ) -> Result<Vec<CommitOwnership>, Error> {
        // make sure the list is not empty
        if commit_hashes.is_empty() {
            warn!("Empty list of commits to search for. It's a bug!");
            return Ok(Vec::new());
        }

        // get the data from PG
        let rows = match pg_client
            .query("select * from stm_find_projects_by_commits($1::varchar[])", &[&commit_hashes])
            .await
        {
            Ok(v) => v,
            Err(e) => {
                error!("stm_find_projects_by_commits for {} failed with {}", &commit_hashes[0], e);
                return Err(Error::from(e));
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
        owner_id: &String,
        project_id: &String,
        commit_hashes: &Vec<String>,
        commit_ts: &Vec<i64>,
    ) -> Result<(), Error> {
        // make sure the list is not empty
        if commit_hashes.is_empty() {
            warn!("Empty list of commits to add to DB. It's a bug!");
            return Ok(());
        }

        info!("Adding {} commits starting from {}", commit_hashes.len(), &commit_hashes[0]);

        // push the data to PG, log the result, nothing to return
        let rows = match pg_client
            .execute(
                "select stm_add_commits($1::varchar, $2::varchar, $3::varchar[], $4::bigint[])",
                &[owner_id, project_id, commit_hashes, commit_ts],
            )
            .await
        {
            Ok(v) => v,
            Err(e) => {
                error!("stm_add_commits failed with {}", e);
                return Err(Error::from(e));
            }
        };

        debug!("Rows updated: {}", rows);
        Ok(())
    }

    /// Returns the latest timestamp for the specified owner/project ids.
    /// Returns an error if the DB has no matching data.
    pub(crate) async fn get_latest_project_commit(
        pg_client: &Client,
        owner_id: &String,
        project_id: &String,
    ) -> Result<i64, Error> {
        // get the data from PG
        let rows = match pg_client
            .query(
                "select * from stm_get_latest_project_commit($1::varchar, $2::varchar)",
                &[owner_id, project_id],
            )
            .await
        {
            Ok(v) => v,
            Err(e) => {
                return Err(Error::from(format!(
                    "stm_get_latest_project_commit for {}/{} failed with {}",
                    owner_id, project_id, e
                )));
            }
        };

        // there should always be some commits for the project if it's requested
        if rows.is_empty() {
            return Err(Error::from(format!("No commits found for {}/{}", owner_id, project_id)));
        }

        // try to return the result if it can be converted into i64
        match rows[0].try_get(0) {
            Ok(v) => Ok(v),
            Err(e) => {
                return Err(Error::from(format!(
                    "Cannot convert latest commit ts to i64 for {}/{} with {}",
                    owner_id, project_id, e
                )));
            }
        }
    }
}

impl EmailOwnership {
    /// Associates an email address with a public key or updates `is_primary` flag for existing records.
    pub(crate) async fn add_email(
        pg_client: &Client,
        owner_id: &String,
        email: &String,
        is_primary: &bool,
    ) -> Result<(), Error> {
        info!("Adding email {}, primary: {}", email, is_primary);

        // push the data to PG, log the result, nothing to return
        let rows = match pg_client
            .execute(
                "select stm_add_email($1::varchar, $2::varchar, $3::boolean)",
                &[owner_id, email, is_primary],
            )
            .await
        {
            Ok(v) => v,
            Err(e) => {
                error!("stm_add_email failed with {}", e);
                return Err(Error::from(e));
            }
        };

        debug!("Rows updated: {}", rows);
        Ok(())
    }
}

impl Dev {
    /// Updates the developer record to make it selectable for report update after a new submission.
    pub(crate) async fn queue_up_for_update(pg_client: &Client, owner_id: &String, gh_login_gist_latest: &Option<String>) -> Result<(), Error> {
        info!("Queueing up report dev {}", owner_id);

        // push the data to PG, log the result, nothing to return
        let rows = match pg_client
            .execute("select stm_queue_up_dev_report($1::varchar, $2::varchar)", &[owner_id, gh_login_gist_latest])
            .await
        {
            Ok(v) => v,
            Err(e) => {
                error!("stm_queue_up_dev_report failed with {}", e);
                return Err(Error::from(e));
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
