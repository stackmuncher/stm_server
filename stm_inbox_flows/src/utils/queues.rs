use rusoto_core::region::Region;
use rusoto_sqs::{DeleteMessageRequest, ReceiveMessageRequest, SendMessageRequest, Sqs, SqsClient};
use rusoto_sqs::{SendMessageBatchRequest, SendMessageBatchRequestEntry};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tracing::{error, info, warn};

/// A unified type of SQS payload. The actual values are S3 keys.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) enum MsgPayload {
    Repo(String),
    GitHubLogin(String),
}

/// A unified payload message for all queues
#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct QueuedMsg {
    /// All enum values are S3 keys, excluding the bucket name.
    pub s3_keys: MsgPayload,
    /// Comes from AWS as part of an incoming message. Used to identify an existing message in the queue for deletion.
    #[serde(skip)]
    pub receipt_handle: Option<String>,
    /// Set by this app to have the queue URL handy for message deletion.
    #[serde(skip)]
    pub queue_url: Option<String>,
}

impl QueuedMsg {
    /// Prepares a message formatted for processing via gh_login or dev user queue.
    /// Value example `repos/01-Feli`. Trailing `/` is removed if present.
    pub(crate) fn new_gh_login_msg(s3_key_login: String) -> Self {
        Self {
            s3_keys: MsgPayload::GitHubLogin(s3_key_login.trim_end_matches("/").to_owned()),
            receipt_handle: None,
            queue_url: None,
        }
    }

    /// Prepares a message formatted for processing via the repo queue.
    /// Value example `repos/01-Feli/ClassProject`. Trailing `/` is removed if present.
    pub(crate) fn new_repo_msg(s3_key_repo: String) -> Self {
        Self {
            s3_keys: MsgPayload::Repo(s3_key_repo),
            receipt_handle: None,
            queue_url: None,
        }
    }

    /// Read a list of messages from the queue and convert into a structure.
    /// `number_of_msgs` must be between 1..10
    /// `wait_for_new_msgs`, true = wait indefinitely, false = exit if empty
    pub(crate) async fn get(
        queue_url: &String,
        number_of_msgs: i64,
        wait_for_new_msgs: bool,
    ) -> Result<Vec<Self>, ()> {
        let client = SqsClient::new(extract_region_from_sqs_url(queue_url)?);

        info!(
            "Getting SQS messages from {}",
            extract_queue_name(queue_url)
        );

        let mut get_err_counter: usize = 0;

        // start listening to the response
        loop {
            if get_err_counter >= 10 {
                return Err(());
            }
            let resp = match client
                .receive_message(ReceiveMessageRequest {
                    max_number_of_messages: Some(number_of_msgs),
                    queue_url: queue_url.to_owned(),
                    wait_time_seconds: Some(20),
                    ..Default::default()
                })
                .await
            {
                Err(e) => {
                    get_err_counter += 1;
                    error!("Failed to retrieve SQS messages. {}", e);
                    crate::utils::sleep(10).await;
                    continue;
                }
                Ok(v) => v,
            };

            // wait until a message arrives or the function is killed by AWS
            if resp.messages.is_none() {
                if wait_for_new_msgs {
                    continue;
                } else {
                    return Ok(Vec::new());
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

            let mut collector: Vec<QueuedMsg> = Vec::new();

            for msg in msgs {
                // convert JSON encoded body into event + ctx structures as defined by Lambda Runtime
                let body = match msg.body.as_ref() {
                    None => {
                        error!("Failed to get SQS message body");
                        continue;
                    }
                    Some(v) => v,
                };

                let mut qmsg: QueuedMsg = match serde_json::from_str(body) {
                    Err(e) => {
                        error!("Failed to deserialize SQS msg body. {}", e);
                        continue;
                    }
                    Ok(v) => v,
                };

                // the message receipt is needed to delete the message from the queue later
                qmsg.receipt_handle = match msg.receipt_handle.as_ref() {
                    None => {
                        error!("Failed to get SQS msg receipt");
                        continue;
                    }
                    Some(v) => Some(v.to_owned()),
                };
                qmsg.queue_url = Some(queue_url.clone());

                collector.push(qmsg);
            }

            info!("Fetched {} SQS msgs", collector.len());

            return Ok(collector);
        }
    }

    /// Sends itself to the specified queue. The message deduping key is a hash of the normalized payload (lowercase + sorted).
    pub(crate) async fn send(&self, queue_url: &String) -> Result<(), ()> {
        let client = SqsClient::new(extract_region_from_sqs_url(queue_url)?);

        info!("Sending msg to SQS {}", extract_queue_name(queue_url));

        if let Err(e) = client
            .send_message(SendMessageRequest {
                message_body: match serde_json::to_string(self) {
                    Err(e) => {
                        error!("Cannot serialize the SQS message {:?} due to {}", self, e);
                        return Err(());
                    }
                    Ok(v) => v,
                },
                queue_url: queue_url.to_owned(),
                message_deduplication_id: Some(self.calculate_deduplication_id()),
                ..Default::default()
            })
            .await
        {
            error!("Failed to send SQS msg {:?} due to {}", self, e);
            return Err(());
        };

        Ok(())
    }

    /// Sends multiple messages to the specified queue in batches. The batch size is limited to 256kB.
    /// It is extremely unlikely any batches will be anywhere that big.
    pub(crate) async fn send_batch(msgs: Vec<Self>, queue_url: &String) -> Result<(), ()> {
        let client = SqsClient::new(extract_region_from_sqs_url(queue_url)?);

        info!(
            "Sending a batch of {} msgs to SQS {}",
            msgs.len(),
            extract_queue_name(queue_url)
        );

        // prepare the individual entries
        let mut batch_entries: Vec<SendMessageBatchRequestEntry> = Vec::new();
        let mut batch_size = 0usize;
        for msg in msgs {
            // serialize the payload
            let message_body = match serde_json::to_string(&msg.s3_keys) {
                Err(e) => {
                    error!("Cannot serialize the SQS message {:?} due to {}", msg, e);
                    return Err(());
                }
                Ok(v) => v,
            };

            // send the batch now if the addition of this message will break the allowed limit of 256kB
            // we'll stay on the safe side and make it 200kB
            if batch_size + message_body.len() > 200_000 {
                warn!("A very large batch: {}", batch_size + message_body.len());

                if let Err(e) = client
                    .send_message_batch(SendMessageBatchRequest {
                        entries: batch_entries
                            .drain(0..)
                            .collect::<Vec<SendMessageBatchRequestEntry>>(),
                        queue_url: queue_url.to_owned(),
                    })
                    .await
                {
                    error!(
                        "Failed to send some of the SQS msgs from the batch due to {}",
                        e
                    );
                    return Err(());
                };

                batch_size = 0;
            }

            // add the current entry to the pending batch
            batch_size += message_body.len();
            batch_entries.push(SendMessageBatchRequestEntry {
                message_body,
                message_deduplication_id: Some(msg.calculate_deduplication_id()),
                ..Default::default()
            });
        }

        // send out what's pending, if anything
        if !batch_entries.is_empty() {
            if let Err(e) = client
                .send_message_batch(SendMessageBatchRequest {
                    entries: batch_entries,
                    queue_url: queue_url.to_owned(),
                })
                .await
            {
                error!(
                    "Failed to send some of the SQS msgs from the batch due to {}",
                    e
                );
                return Err(());
            };
        }

        Ok(())
    }

    /// Delete itself from the queue using receipt ID stored in the message.
    pub(crate) async fn delete(&self) -> Result<(), ()> {

        // this would be a bug if the queue URL is missing
        let queue_url = self.queue_url.clone().expect("Missing queue URL in SQS msg");

        let client = SqsClient::new(extract_region_from_sqs_url(&queue_url)?);

        info!("Deleting msg from SQS {}", extract_queue_name(&queue_url));

        // delete the request msg from the queue so it cannot be replayed again
        if let Err(e) = client
            .delete_message(DeleteMessageRequest {
                queue_url: queue_url.to_owned(),
                receipt_handle: match self.receipt_handle.as_ref() {
                    None => {
                        error!("Missing SQS msg ID");
                        return Err(());
                    }
                    Some(v) => v.clone(),
                },
            })
            .await
        {
            error!("Failed to delete the SQS message. {}", e);
            return Err(());
        };

        Ok(())
    }

    /// Calculates deduplication id as a hash of the normalized payload (lowercase + sorted).
    #[inline]
    fn calculate_deduplication_id(&self) -> String {
        match &self.s3_keys {
            MsgPayload::GitHubLogin(login_s3_key) => {
                stackmuncher::utils::hash_str_sha1(&login_s3_key.to_lowercase())
            }
            MsgPayload::Repo(repo_s3_key) => stackmuncher::utils::hash_str_sha1(repo_s3_key),
        }
    }
}

/// Extract the region from a URL like this
/// `https://sqs.ap-southeast-2.amazonaws.com/028534811986/LAMBDA_PROXY_REQ`
fn extract_region_from_sqs_url(url: &String) -> Result<Region, ()> {
    let no_prefix = url.trim_start_matches("https://sqs.");
    let region = no_prefix[..no_prefix
        .find(".")
        .expect(&format!("Invalid SQS URL: {}", no_prefix))]
        .to_string();

    Ok(Region::from_str(&region).expect(&format!("Cannot convert SQS region value: {}", region)))
}

/// Returns just the key name, e.g. stm_jobs_new.fifo from the full URL
/// https://sqs.us-east-1.amazonaws.com/028534811986/stm_jobs_new.fifo
/// Panics on error
fn extract_queue_name(url: &String) -> String {
    url[url.rfind("/").expect(&format!("Invalid SQS URL: {}", url))..].to_string()
}
