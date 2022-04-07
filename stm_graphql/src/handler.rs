use crate::authorizer::validate_jwt;
use crate::graphql;
use crate::Error;
use crate::{api_gw_request::ApiGatewayRequest, api_gw_response::ApiGatewayResponse};
use juniper::http::GraphQLRequest;
use lambda_runtime::LambdaEvent;
use serde::Serialize;
use serde_json::Value;
use simple_error::SimpleError;
use std::collections::HashMap;
use stm_shared::graphql::RustScalarValue;
use sysinfo::{RefreshKind, System, SystemExt};
use tracing::{error, info};

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
    // info!("Event: {}", event);
    //info!("Context: {:?}", _ctx);

    log_memory_use(&mut sys, "Start");

    let api_request = serde_json::from_value::<ApiGatewayRequest>(event).expect("Failed to deser API GW request");

    // browsers send OPTIONS request to check for CORS before doing cross-domain xHTTP queries
    // this part can have its own dedicated responder configured via API GW
    if api_request.request_context.http.method.to_uppercase().as_str() == "OPTIONS" {
        return Ok(crate::http_options_handler::http_options_response(api_request).to_value());
    }

    // a GET requests returns the GQL schema and does not require any auth
    if api_request.request_context.http.method.to_uppercase().as_str() == "GET" {
        let mut headers: HashMap<String, String> = HashMap::new();
        headers.insert("Content-Type".to_owned(), "text/plain".to_owned());
        headers.insert("Cache-Control".to_owned(), "max-age=600".to_owned());

        return Ok(ApiGatewayResponse {
            is_base64_encoded: false,
            status_code: 200,
            headers,
            body: Some(graphql::get_schema()),
        }
        .to_value());
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

    // extract the GQL request, which is Body of POST in JSON form
    // e.g. {"variables":{},"query":"{\n  devsPerLanguage {\n    aggregations {\n agg {\n buckets {\n key\n docCount\n __typename\n }\n __typename\n }\n __typename\n }\n __typename\n  }\n}"}
    let gql_request = match &api_request.body {
        Some(body) => match serde_json::from_str::<GraphQLRequest<RustScalarValue>>(body) {
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
    let gql_data = graphql::execute_gql(&config, gql_request).await;
    log_memory_use(&mut sys, "GQL response returned");

    let gql_data = match gql_data {
        // the .1 member indicates if there were any errors during the execution
        // this is of no importance here and can be ignored
        Ok((v, _)) => v,
        Err(_) => {
            return Err(Box::new(SimpleError::new("Failed to get GQL response data")));
        }
    };

    info!("Resp size: {} bytes", gql_data.len());

    // return back the result
    Ok(ApiGatewayResponse::new(gql_data, 200, 600))
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
