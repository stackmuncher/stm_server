use rusoto_sqs::{
    DeleteMessageBatchRequest, DeleteMessageBatchRequestEntry, PurgeQueueRequest, ReceiveMessageRequest,
    SendMessageRequest, Sqs, SqsClient,
};
use serde::de::DeserializeOwned;
use tokio::time::{sleep, Duration};
use tracing::{error, info};

/// A single message of type T and its receipt handle for deleting the message later.
pub struct SqsMessage<T> {
    /// Deserialized message.
    pub message: T,
    /// Receipt handle for later deletion of the message.
    pub receipt_handle: String,
}

/// A batch of messages received from a queue.
pub struct SqsMessages<T> {
    pub messages: Vec<SqsMessage<T>>,
    pub queue_url: String,
}

/// Sends the payload to the specified queue.
pub async fn send(client: &SqsClient, payload: String, queue_url: &String) -> Result<(), ()> {
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

/// Purges the specified queue.
pub async fn purge_queue(client: &SqsClient, queue_url: &String) -> Result<(), ()> {
    info!("Purging {}", queue_url);

    if let Err(e) = client
        .purge_queue(PurgeQueueRequest {
            queue_url: queue_url.to_owned(),
            ..Default::default()
        })
        .await
    {
        error!("Failed to purge the queue: {}", e);
        return Err(());
    };

    info!("Purged");

    Ok(())
}

/// Returns just the key name, e.g. stm_jobs_new.fifo from the full URL
/// https://sqs.us-east-1.amazonaws.com/028534811986/stm_jobs_new.fifo
/// Panics on error
fn extract_queue_name(url: &String) -> String {
    url[url.rfind("/").expect(&format!("Invalid SQS URL: {}", url))..].to_string()
}

impl<T: DeserializeOwned> SqsMessages<T> {
    /// Read a list of messages from the queue and convert them into a structure.
    /// `number_of_msgs` must be between 1..10
    /// `wait_for_new_msgs`, true = wait indefinitely, false = exit if empty
    pub async fn get(
        client: &SqsClient,
        queue_url: &String,
        number_of_msgs: i64,
        wait_for_new_msgs: bool,
    ) -> Result<Self, ()> {
        info!("Getting SQS messages from {}", extract_queue_name(queue_url));

        let mut get_err_counter: usize = 0;

        // a container for returning back to the caller
        let mut sqs_messages = SqsMessages {
            messages: Vec::new(),
            queue_url: queue_url.clone(),
        };

        let wait_time = if wait_for_new_msgs { 20_i64 } else { 0 };

        // start listening to the response
        loop {
            if get_err_counter >= 10 {
                return Err(());
            }
            let resp = match client
                .receive_message(ReceiveMessageRequest {
                    max_number_of_messages: Some(number_of_msgs),
                    queue_url: queue_url.to_owned(),
                    wait_time_seconds: Some(wait_time),
                    ..Default::default()
                })
                .await
            {
                Err(e) => {
                    get_err_counter += 1;
                    error!("Failed to retrieve SQS messages. {}", e);
                    sleep(Duration::from_secs(10)).await;
                    continue;
                }
                Ok(v) => v,
            };

            // wait until a message arrives or the function is killed by AWS
            if resp.messages.is_none() {
                if wait_for_new_msgs {
                    continue;
                } else {
                    return Ok(sqs_messages);
                }
            }

            // an empty list returns when the queue wait time expires
            let msgs = match resp.messages {
                None => {
                    error!("Failed to get list of SQS messages");
                    continue;
                }
                Some(v) => v,
            };
            if msgs.len() == 0 {
                continue;
            }

            // process messages one at a time
            for msg in msgs {
                // convert JSON encoded body into event + ctx structures as defined by Lambda Runtime
                let body = match msg.body.as_ref() {
                    None => {
                        error!("Failed to get SQS message body");
                        continue;
                    }
                    Some(v) => v,
                };

                let message = match serde_json::from_str::<T>(body) {
                    Err(e) => {
                        error!("Failed to deserialize SQS msg body. {}", e);
                        continue;
                    }
                    Ok(v) => v,
                };

                // the message receipt is needed to delete the message from the queue later
                let receipt_handle = match msg.receipt_handle.as_ref() {
                    None => {
                        error!("Failed to get SQS msg receipt");
                        continue;
                    }
                    Some(v) => v.to_owned(),
                };

                sqs_messages.messages.push(SqsMessage {
                    message,
                    receipt_handle,
                });
            }

            info!("Fetched {} SQS msgs", sqs_messages.messages.len());

            return Ok(sqs_messages);
        }
    }

    /// Returns all receipt handles stored in the collection of messages.
    pub fn get_all_receipt_handles(&self) -> Vec<String> {
        self.messages
            .iter()
            .map(|msg| msg.receipt_handle.clone())
            .collect::<Vec<String>>()
    }
}

/// Delete specified messages from the queue using receipt ID stored in the message.
pub async fn delete_messages(client: &SqsClient, queue_url: &String, receipt_handles: Vec<String>) -> Result<(), ()> {
    if receipt_handles.is_empty() {
        info!("Deleting 0 msgs from SQS {}", extract_queue_name(queue_url));
        return Ok(());
    }
    info!("Deleting {} msgs from SQS {}", receipt_handles.len(), extract_queue_name(queue_url));

    let entries = receipt_handles
        .into_iter()
        .enumerate()
        .map(|(id, receipt_handle)| DeleteMessageBatchRequestEntry {
            id: id.to_string(),
            receipt_handle,
        })
        .collect::<Vec<DeleteMessageBatchRequestEntry>>();

    // delete the request msg from the queue so it cannot be replayed again
    if let Err(e) = client
        .delete_message_batch(DeleteMessageBatchRequest {
            queue_url: queue_url.clone(),
            entries,
        })
        .await
    {
        error!("Failed to delete one or more SQS messages. {}", e);
        return Err(());
    };

    Ok(())
}
