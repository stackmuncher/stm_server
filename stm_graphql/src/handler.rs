use crate::types::{ApiGatewayRequest, ApiGatewayResponse};
use crate::Error;
use lambda_runtime::LambdaEvent;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use sysinfo::{RefreshKind, System, SystemExt};
use tracing::{info, warn};
use urlencoding::decode;

/// A blank error structure to return to the runtime. No messages are required because all necessary information has already been logged.
/// The API Gateway will return 500 which may be picked up by CloudFront and converted into a nice looking 500 page.
#[derive(Debug, Serialize)]
struct HandlerError {}

impl std::error::Error for HandlerError {}

impl std::fmt::Display for HandlerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}

//pub(crate) async fn my_handler(event: Value, _ctx: Context) -> Result<Value, Error> {
pub(crate) async fn my_handler(event: LambdaEvent<Value>) -> Result<Value, lambda_runtime::Error> {
    let mut sys = System::new_with_specifics(RefreshKind::with_memory(RefreshKind::new()));
    let (event, _ctx) = event.into_parts();
    info!("Event: {}", event);
    //info!("Context: {:?}", _ctx);

    log_memory_use(&mut sys, "Start");

    let api_request = serde_json::from_value::<ApiGatewayRequest>(event).expect("Failed to deser APIGW request");

    // log_memory_use(&mut sys, "API Req created");

    if api_request.request_context.http.method.to_uppercase().as_str() == "OPTIONS" {
        return Ok(crate::http_options::http_options_response(api_request).to_value());
    }

    // if Authorization env var is present check if it matches Authorization header
    // this is done for basic protection against direct calls to the api bypassing CloudFront
    if let Ok(auth_var) = std::env::var("Authorization") {
        let auth_header = match api_request.headers.get("authorization") {
            Some(v) => v.clone(),
            None => String::new(),
        };

        if auth_var != auth_header {
            warn!("Unauthorized. Header: {}", auth_header);
            return gw_response("Unauthorized".to_owned(), 403, 3600);
        }
    } else {
        #[cfg(debug_assertions)]
        warn!("No Authorization env var - all requests are allowed");
    };

    // log_memory_use(&mut sys, "Config init");

     // decode possible URL path and query string
    info!("Raw path: {}, Query: {}", &api_request.raw_path, &api_request.raw_query_string);
    let url_path = decode(&api_request.raw_path).unwrap_or_default().trim().to_string();
    let url_query = decode(&api_request.raw_query_string)
        .unwrap_or_default()
        .trim()
        .to_string();
    let dev = match api_request.query_string_parameters {
        None => None,
        Some(v) => v.dev,
    };
    info!("Decoded path: {}, query: {}, dev: {:?}", url_path, url_query, dev);

    // send the user request downstream for processing
    let gql_data = r#"{
        "data": {
          "test": "Hello Vue!"
        }
      }"#;

    log_memory_use(&mut sys, "GQL data returned");

    info!("gql full: {} bytes", gql_data.len());
    let gql_data = minify::html::minify(&gql_data);
    info!("HTML mini: {} bytes", gql_data.len());

    log_memory_use(&mut sys, "GQL minified");

    // return back the result
    gw_response(gql_data, 200, 0)
}

/// Prepares the response with the status and HTML body. May fail and return an error.
fn gw_response(body: String, status_code: u32, ttl: u32) -> Result<Value, Error> {
    let mut headers: HashMap<String, String> = HashMap::new();
    headers.insert("Content-Type".to_owned(), "application/json".to_owned());
    headers.insert("Cache-Control".to_owned(), ["max-age=".to_owned(), ttl.to_string()].concat());

    let resp = ApiGatewayResponse {
        is_base64_encoded: false,
        status_code,
        headers,
        body: Some(body),
    };

    Ok(resp.to_value())
}

/// Logs current memory use and the delta from the previous sample.
fn log_memory_use(sys: &mut System, msg: &str) {
    let used = sys.used_memory() as i64;
    let swap = sys.used_swap() as i64;

    sys.refresh_memory();

    info!(
        "RAM total KB: {}, used {}/{}, tot swap: {}, used swap: {}/{} - {}",
        sys.total_memory(),
        sys.used_memory(),
        sys.used_memory() as i64 - used,
        sys.total_swap(),
        sys.used_swap(),
        sys.used_swap() as i64 - swap,
        msg,
    );
}
