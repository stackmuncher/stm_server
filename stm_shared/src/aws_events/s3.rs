// this code was uplifted from https://github.com/LegNeato/aws-lambda-events/blob/master/aws_lambda_events/src/generated/s3.rs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Top level `S3Event` which wrap an array of `S3Event`Record
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct S3Event {
    #[serde(rename = "Records")]
    pub records: Vec<S3EventRecord>,
}

impl S3Event {
    /// Digs out the key of the very first record in the list, if any.
    /// Normally there is only 1 record for event arriving into Lambda and SQS
    pub fn get_first_s3_key(self) -> Option<String> {
        for record in self.records {
            if let Some(key) = record.s3.object.key {
                return Some(key);
            }
        }
        None
    }

    /// Returns a list of all S3 keys included in the event
    pub fn get_all_s3_keys(self) -> Vec<String> {
        let mut keys: Vec<String> = Vec::new();

        keys.reserve(self.records.len());

        for record in self.records {
            if let Some(key) = record.s3.object.key {
                keys.push(key);
            }
        }

        keys
    }
}

/// `S3EventRecord` which wrap record data
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct S3EventRecord {
    #[serde(default)]
    #[serde(rename = "eventVersion")]
    pub event_version: Option<String>,
    #[serde(default)]
    #[serde(rename = "eventSource")]
    pub event_source: Option<String>,
    #[serde(default)]
    #[serde(rename = "awsRegion")]
    pub aws_region: Option<String>,
    #[serde(rename = "eventTime")]
    pub event_time: DateTime<Utc>,
    #[serde(default)]
    #[serde(rename = "eventName")]
    pub event_name: Option<String>,
    #[serde(rename = "userIdentity")]
    pub principal_id: S3UserIdentity,
    #[serde(rename = "requestParameters")]
    pub request_parameters: S3RequestParameters,
    #[serde(default)]
    #[serde(rename = "responseElements")]
    pub response_elements: HashMap<String, String>,
    pub s3: S3Entity,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct S3UserIdentity {
    #[serde(default)]
    #[serde(rename = "principalId")]
    pub principal_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct S3RequestParameters {
    #[serde(default)]
    #[serde(rename = "sourceIPAddress")]
    pub source_ip_address: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct S3Entity {
    #[serde(default)]
    #[serde(rename = "s3SchemaVersion")]
    pub schema_version: Option<String>,
    #[serde(default)]
    #[serde(rename = "configurationId")]
    pub configuration_id: Option<String>,
    pub bucket: S3Bucket,
    pub object: S3Object,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct S3Bucket {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(rename = "ownerIdentity")]
    pub owner_identity: S3UserIdentity,
    /// nolint: stylecheck
    #[serde(default)]
    pub arn: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct S3Object {
    #[serde(default)]
    pub key: Option<String>,
    pub size: Option<i64>,
    #[serde(default)]
    #[serde(rename = "urlDecodedKey")]
    pub url_decoded_key: Option<String>,
    #[serde(default)]
    #[serde(rename = "versionId")]
    pub version_id: Option<String>,
    #[serde(default)]
    #[serde(rename = "eTag")]
    pub e_tag: Option<String>,
    #[serde(default)]
    pub sequencer: Option<String>,
}
