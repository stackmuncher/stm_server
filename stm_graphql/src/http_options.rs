use std::collections::HashMap;

use crate::types::{ApiGatewayRequest, ApiGatewayResponse};
use tracing::info;

/// Returns an HTTP response specific to the OPTIONS request passed in.
/// See https://developer.mozilla.org/en-US/docs/Web/HTTP/Methods/OPTIONS
pub(crate) fn http_options_response(req: ApiGatewayRequest) -> ApiGatewayResponse {
    info!("OPTIONS: {:?}", req);

    // return a dummy response that allows everything and anything
    ApiGatewayResponse {
        is_base64_encoded: false,
        status_code: 204,
        headers: HashMap::new(),
        body: None,
    }
}
