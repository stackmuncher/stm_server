use serde::Deserialize;
use std::collections::HashMap;

/// The API Gateway request struct with fields of interest.
/// See `samples/options_request.json` file for a full request example.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ApiGatewayRequest {
    pub headers: HashMap<String, String>,
    pub request_context: inner_types::ApiGatewayRequestContext,
    pub body: Option<String>,
}

/// A wrapper for inner types that are unlikely to be used on their own to hide them from IDE prompts.
pub(crate) mod inner_types {
    use serde::Deserialize;

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
}
