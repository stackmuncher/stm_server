use rusoto_core::region::Region;
use rusoto_sqs::{SendMessageRequest, Sqs, SqsClient};
use std::str::FromStr;
use tracing::{error, info};

/// Sends the payload to the specified queue.
pub async fn send(payload: String, queue_url: &String) -> Result<(), ()> {
    let client = SqsClient::new(extract_region_from_sqs_url(queue_url)?);

    info!("Sending msg to SQS {}", extract_queue_name(queue_url));

    if let Err(e) = client
        .send_message(SendMessageRequest {
            message_body: payload,
            queue_url: queue_url.to_owned(),
            ..Default::default()
        })
        .await
    {
        error!("Failed to send SQS msg: {}", e);
        return Err(());
    };

    info!("Sent");

    Ok(())
}

/// Extract the region from a URL like this
/// `https://sqs.ap-southeast-2.amazonaws.com/028534811986/LAMBDA_PROXY_REQ`
fn extract_region_from_sqs_url(url: &String) -> Result<Region, ()> {
    let no_prefix = url.trim_start_matches("https://sqs.");
    let region = no_prefix[..no_prefix.find(".").expect(&format!("Invalid SQS URL: {}", no_prefix))].to_string();

    Ok(Region::from_str(&region).expect(&format!("Cannot convert SQS region value: {}", region)))
}

/// Returns just the key name, e.g. stm_jobs_new.fifo from the full URL
/// https://sqs.us-east-1.amazonaws.com/028534811986/stm_jobs_new.fifo
/// Panics on error
fn extract_queue_name(url: &String) -> String {
    url[url.rfind("/").expect(&format!("Invalid SQS URL: {}", url))..].to_string()
}
