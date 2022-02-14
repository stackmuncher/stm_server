use crate::authorizer::validate_jwt;
use crate::handlers;
use crate::Error;
use crate::{api_gw_request::ApiGatewayRequest, api_gw_response::ApiGatewayResponse};
use juniper::http::GraphQLRequest;
use lambda_runtime::LambdaEvent;
use serde::Serialize;
use serde_json::Value;
use simple_error::SimpleError;
use std::collections::HashMap;
use stm_shared::elastic::types_aggregations::MyScalarValue;
use sysinfo::{RefreshKind, System, SystemExt};
use tracing::{error, info};
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
pub(crate) async fn my_handler(event: LambdaEvent<Value>) -> Result<Value, Error> {
    let mut sys = System::new_with_specifics(RefreshKind::with_memory(RefreshKind::new()));
    let (event, _ctx) = event.into_parts();
    info!("Event: {}", event);
    //info!("Context: {:?}", _ctx);

    log_memory_use(&mut sys, "Start");

    let api_request = serde_json::from_value::<ApiGatewayRequest>(event).expect("Failed to deser APIGW request");

    // log_memory_use(&mut sys, "API Req created");

    if api_request.request_context.http.method.to_uppercase().as_str() == "OPTIONS" {
        return Ok(crate::http_options_handler::http_options_response(api_request).to_value());
    }

    // decode possible URL path and query string
    info!("Raw path: {}, Query: {}", &api_request.raw_path, &api_request.raw_query_string);
    let url_path = decode(&api_request.raw_path).unwrap_or_default().trim().to_string();
    let url_query = decode(&api_request.raw_query_string)
        .unwrap_or_default()
        .trim()
        .to_string();
    let dev = match &api_request.query_string_parameters {
        None => None,
        Some(v) => v.dev.clone(),
    };
    info!("Decoded path: {}, query: {}, dev: {:?}", url_path, url_query, dev);

    // /schema request returns the GQL schema and does not require any auth
    if url_path == "schema" {
        return Ok(ApiGatewayResponse::new(handlers::home::get_schema(), 200, 3600));
    }

    let config = crate::config::Config::new();

    // get caller details from the JWT attached to the request
    // return HTTP 401 if the request is not authorized
    // why 401: https://stackoverflow.com/questions/3297048/403-forbidden-vs-401-unauthorized-http-responses
    let jwt = match validate_jwt(&api_request, &config) {
        Some(v) => v,
        None => {
            return Ok((ApiGatewayResponse {
                is_base64_encoded: false,
                status_code: 401,
                headers: HashMap::new(),
                body: None,
            })
            .to_value());
        }
    };

    info!("Caller: {:?}", jwt.email);

    // log_memory_use(&mut sys, "Config init");

    // extract the GQL request, which is Body of POST in JSON form
    // e.g. {"variables":{},"query":"{\n  devsPerLanguage {\n    aggregations {\n agg {\n buckets {\n key\n docCount\n __typename\n }\n __typename\n }\n __typename\n }\n __typename\n  }\n}"}
    let gql_request = match &api_request.body {
        Some(body) => match serde_json::from_str::<GraphQLRequest<MyScalarValue>>(body) {
            Ok(v) => v,
            Err(e) => {
                error!("Invalid GQL request: {} / {}", e, body);
                return Err(Box::new(SimpleError::new("Failed to get GQL data")));
            }
        },
        None => {
            error!("Empty GQL request");
            return Err(Box::new(SimpleError::new("Empty GQL request")));
        }
    };
    log_memory_use(&mut sys, "GQL Request extracted");
    // send the user request downstream for processing
    let gql_data = handlers::home::execute_gql(&config, gql_request).await;

    // measure memory consumption before unwrapping
    log_memory_use(&mut sys, "GQL response returned");
    let gql_data = match gql_data {
        Ok(v) => v,
        Err(_) => {
            return Err(Box::new(SimpleError::new("Failed to get GQL data")));
        }
    };

    info!("gql full: {} bytes", gql_data.len());
    let gql_data = minify::html::minify(&gql_data);
    info!("HTML mini: {} bytes", gql_data.len());

    log_memory_use(&mut sys, "GQL minified");

    // return back the result
    Ok(ApiGatewayResponse::new(gql_data, 200, 0))
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
