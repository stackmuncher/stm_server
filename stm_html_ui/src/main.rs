#[cfg(not(debug_assertions))]
use lambda_runtime::handler_fn;

mod config;
mod elastic;
mod handler;
mod html;
mod search_log;

pub(crate) type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

#[tokio::main]
async fn main() -> Result<(), Error> {
    // init the logger with the specified level
    let tsub = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_ansi(false);
    // time is not needed in CloudWatch, but is useful in console
    #[cfg(not(debug_assertions))]
    let tsub = tsub.without_time();
    tsub.init();

    #[cfg(debug_assertions)]
    return proxy::run().await;

    // call the actual handler of the request
    #[cfg(not(debug_assertions))]
    return lambda_runtime::run(handler_fn(handler::my_handler)).await;
}

/// This module is only used for local debugging via SQS and should
/// not be deployed to Lambda.
#[cfg(debug_assertions)]
mod proxy {
    use lambda_runtime::Context;
    use rusoto_core::region::Region;
    use rusoto_sqs::{
        DeleteMessageRequest, ReceiveMessageRequest, SendMessageRequest, Sqs, SqsClient,
    };
    use serde::Deserialize;
    use serde_json::Value;
    use tracing::info;

    pub(crate) type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

    const AWS_REGION: Region = Region::UsEast1; // replace with your preferred region
    const REQUEST_QUEUE_URL_ENV: &str = "STM_HTML_LAMBDA_PROXY_REQ"; // add your queue URL there
    const RESPONSE_QUEUE_URL_ENV: &str = "STM_HTML_LAMBDA_PROXY_RESP"; // add your queue URL there

    #[derive(Deserialize, Debug)]
    struct RequestPayload {
        pub event: Value,
        pub ctx: Context,
    }

    pub(crate) async fn run() -> Result<(), Error> {
        #[cfg(debug_assertions)]
        loop {
            // get event and context details from the queue
            let (payload, receipt_handle) = get_input().await?;
            info!("New msg");
            // invoke the handler
            let response = crate::handler::my_handler(payload.event, payload.ctx).await?;

            // send back the response and delete the message from the queue
            send_output(response, receipt_handle).await?;
            info!("Msg sent");
        }
    }

    /// Read a message from the queue and return the payload as Lambda structures
    async fn get_input() -> Result<(RequestPayload, String), Error> {
        let client = SqsClient::new(AWS_REGION);

        // start listening to the response
        loop {
            let resp = client
                .receive_message(ReceiveMessageRequest {
                    max_number_of_messages: Some(1),
                    queue_url: std::env::var(REQUEST_QUEUE_URL_ENV)
                        .expect(&format!(
                            "Missing {} env var with the SQS request queue URL",
                            REQUEST_QUEUE_URL_ENV
                        ))
                        .trim()
                        .to_string(),
                    wait_time_seconds: Some(20),
                    ..Default::default()
                })
                .await?;

            // wait until a message arrives or the function is killed by AWS
            if resp.messages.is_none() {
                continue;
            }

            // an empty list returns when the queue wait time expires
            let msgs = resp.messages.expect("Failed to get list of messages");
            if msgs.len() == 0 {
                continue;
            }

            // the message receipt is needed to delete the message from the queue later
            let receipt_handle = msgs[0]
                .receipt_handle
                .as_ref()
                .expect("Failed to get msg receipt")
                .to_owned();

            // convert JSON encoded body into event + ctx structures as defined by Lambda Runtime
            let body = msgs[0].body.as_ref().expect("Failed to get message body");
            let payload: RequestPayload =
                serde_json::from_str(body).expect("Failed to deserialize msg body");

            return Ok((payload, receipt_handle));
        }
    }

    /// Send back the response and delete the message from the queue.
    async fn send_output(response: Value, receipt_handle: String) -> Result<(), Error> {
        let client = SqsClient::new(AWS_REGION);

        client
            .send_message(SendMessageRequest {
                message_body: response.to_string(),
                queue_url: std::env::var(RESPONSE_QUEUE_URL_ENV)
                    .expect(&format!(
                        "Missing {} env var with the SQS response queue URL",
                        RESPONSE_QUEUE_URL_ENV
                    ))
                    .trim()
                    .to_string(),
                ..Default::default()
            })
            .await?;

        // delete the request msg from the queue so it cannot be replayed again
        client
            .delete_message(DeleteMessageRequest {
                queue_url: std::env::var(REQUEST_QUEUE_URL_ENV)
                    .expect(&format!(
                        "Missing {} env var with the SQS request queue URL",
                        REQUEST_QUEUE_URL_ENV
                    ))
                    .trim()
                    .to_string(),
                receipt_handle,
            })
            .await?;

        Ok(())
    }
}
