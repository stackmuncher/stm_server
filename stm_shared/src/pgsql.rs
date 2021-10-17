use tokio_postgres::NoTls;
use tracing::{debug, error};

/// Prepare a client for Postgres connection. Panics if cannot connect to the PG DB.
pub async fn get_pg_client(connection_string: &String) -> tokio_postgres::Client {
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
