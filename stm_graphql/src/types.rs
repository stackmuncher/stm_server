use serde::{Deserialize, Serialize};
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
}

/// An inner member of ApiGatewayRequest
#[derive(Deserialize, Debug)]
pub(crate) struct ApiGatewayRequestQueryStringParameters {
    pub dev: Option<String>,
}

/// An inner member of ApiGatewayRequest
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ApiGatewayRequestContextHttp {
    pub method: String,
}

/// An inner member of ApiGatewayRequest
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ApiGatewayRequestContext {
    pub http: ApiGatewayRequestContextHttp,
}

/// The API Gateway request struct with fields of interest.
/// See `samples/options_request.json` file for a full request example.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ApiGatewayRequest {
    pub raw_path: String,
    pub raw_query_string: String,
    pub headers: HashMap<String, String>,
    pub query_string_parameters: Option<ApiGatewayRequestQueryStringParameters>,
    pub request_context: ApiGatewayRequestContext,
}
