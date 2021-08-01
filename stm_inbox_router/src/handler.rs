use crate::config::Config;
use crate::s3::{get_bytes_from_s3, S3Event};
use flate2::read::GzDecoder;
use lambda_runtime::{Context, Error};
use log::info;
use serde_json::Value;
use stackmuncher_lib::report::Report;
use std::io::Read;
use tracing::{debug, error};

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
            return Err(Error::from(format!("Empty object key in the event details")));
        }
    };

    // required to ID the transaction in the log, otherwise it's not known which report failed
    info!("S3 key: {}", s3_key);

    // read and unzip the report from S3
    let report = get_bytes_from_s3(config, s3_key).await?;
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

    // get the list of commits a few levels down into the hierarchy
    let commit_list = match report
        .projects_included
        .iter()
        .next()
        .as_ref()
        .expect("Cannot unwrap included project. It's a bug.")
        .recent_project_commits
        .as_ref()
    {
        Some(v) => v,
        None => {
            info!("No commit details found.");
            return Ok(());
        }
    };

    info!("{}", commit_list.join(", "));

    Ok(())
}
