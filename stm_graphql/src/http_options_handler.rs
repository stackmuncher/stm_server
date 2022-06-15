use crate::{api_gw_request::ApiGatewayRequest, api_gw_response::ApiGatewayResponse};
use std::collections::HashMap;
use tracing::{debug, info};

/// Returns a dummy HTTP response to HTTP OPTIONS request. All request headers are ignored.
/// CORS headers are handled by the API Gateway and will overwrite what is prepared here if configured in the API GW.
///
/// See https://developer.mozilla.org/en-US/docs/Web/HTTP/Methods/OPTIONS
pub(crate) fn http_options_response(req: ApiGatewayRequest) -> ApiGatewayResponse {
    info!("HTTP OPTIONS");
    debug!("OPTIONS: {:?}", req);

    // return a dummy response that allows everything and anything
    ApiGatewayResponse {
        is_base64_encoded: false,
        status_code: 204,
        headers: HashMap::new(),
        body: None,
    }
}
