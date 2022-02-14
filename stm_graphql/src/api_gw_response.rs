use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

/// The full response structure for sending back to API Gateway
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ApiGatewayResponse {
    pub is_base64_encoded: bool,
    pub status_code: u32,
    pub headers: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
}

impl ApiGatewayResponse {
    /// Converts itself to serde_json::Value
    /// # Panics
    /// May panic if the conversion fails.
    pub(crate) fn to_value(self) -> Value {
        serde_json::to_value(self).expect("Failed to serialize response")
    }

    /// Prepares the response with the status and HTML body. May panic if conversion of the response to Value fails.
    pub(crate) fn new(body: String, status_code: u32, ttl: u32) -> Value {
        let mut headers: HashMap<String, String> = HashMap::new();
        headers.insert("Content-Type".to_owned(), "application/json".to_owned());
        headers.insert("Cache-Control".to_owned(), ["max-age=".to_owned(), ttl.to_string()].concat());

        let resp = ApiGatewayResponse {
            is_base64_encoded: false,
            status_code,
            headers,
            body: Some(body),
        };

        resp.to_value()
    }
}
