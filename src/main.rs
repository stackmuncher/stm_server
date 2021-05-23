use crate::config::Config;
use lambda_runtime::Error;

mod config;
mod handler;
mod s3;

/// Boilerplate Lambda runtime code with conditional debug proxy
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

    // prepare cached config to save on the init time
    // it may backfire if the S3 connector token expire while it is being cached
    // let's hope that the function gets recycled before that happens
    // if the token expire we'll get an error in the log and loose the report because there is no retry
    let config_owned = Config::new();
    let config_shared = &config_owned;

    // call the proxy - development only
    #[cfg(debug_assertions)]
    return proxy::run(config_shared).await;

    // call the actual handler of the request - production only
    #[cfg(not(debug_assertions))]
    return lambda_runtime::run(lambda_runtime::handler_fn(
        move |event: serde_json::Value, ctx: lambda_runtime::Context| async move {
            handler::my_handler(event, ctx, config_shared).await
        },
    ))
    .await;
}

/// This module is only used for local debugging via SQS and will
/// not be deployed to Lambda if compiled with `--release`.
#[cfg(debug_assertions)]
mod proxy {
    use crate::config::Config;
    use lambda_runtime::{Context, Error};
    use rusoto_core::region::Region;
    use rusoto_sqs::{DeleteMessageRequest, ReceiveMessageRequest, SendMessageRequest, Sqs, SqsClient};
    use serde::Deserialize;
    use serde_json::Value;
    use tracing::info;

    // these are specific to a particular account - modify as needed for development
    // they should probably be taken out into a separate file
    const AWS_REGION: Region = Region::UsEast1;
    const REQUEST_QUEUE_URL: &str = "https://sqs.us-east-1.amazonaws.com/028534811986/STM_INBOX_LAMBDA_PROXY_REQ";
    const RESPONSE_QUEUE_URL: &str = "https://sqs.us-east-1.amazonaws.com/028534811986/STM_INBOX_LAMBDA_PROXY_RESP";

    #[derive(Deserialize, Debug)]
    struct RequestPayload {
        pub event: Value,
        pub ctx: Context,
    }

    pub(crate) async fn run(config: &Config) -> Result<(), Error> {
        loop {
            // get event and context details from the queue
            let (payload, receipt_handle) = get_input().await?;
            info!("New msg");
            // invoke the handler
            let response = crate::handler::my_handler(payload.event, payload.ctx, config).await?;

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
                    queue_url: REQUEST_QUEUE_URL.to_string(),
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
            let payload: RequestPayload = serde_json::from_str(body).expect("Failed to deserialize msg body");

            return Ok((payload, receipt_handle));
        }
    }

    /// Send back the response and delete the message from the queue.
    async fn send_output(response: Value, receipt_handle: String) -> Result<(), Error> {
        let client = SqsClient::new(AWS_REGION);

        client
            .send_message(SendMessageRequest {
                message_body: response.to_string(),
                queue_url: RESPONSE_QUEUE_URL.to_string(),
                ..Default::default()
            })
            .await?;

        // delete the request msg from the queue so it cannot be replayed again
        client
            .delete_message(DeleteMessageRequest {
                queue_url: REQUEST_QUEUE_URL.to_string(),
                receipt_handle,
            })
            .await?;

        Ok(())
    }
}
