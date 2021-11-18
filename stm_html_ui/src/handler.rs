use crate::{config::Config, html, Error};
use lambda_runtime::Context;
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tera::Tera;
use tracing::{info, warn};
use urlencoding::decode;

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ApiGatewayResponse {
    // #[serde(skip_serializing_if = "Option::is_none")]
    // cookies: Option<Vec<String>>,
    is_base64_encoded: bool,
    status_code: u32,
    headers: HashMap<String, String>,
    body: String,
}

#[derive(Deserialize, Debug)]
struct ApiGatewayQueryStringParameters {
    dev: Option<String>,
    //project: String
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ApiGatewayRequest {
    raw_path: String,
    raw_query_string: String,
    headers: HashMap<String, String>,
    query_string_parameters: Option<ApiGatewayQueryStringParameters>,
}

#[derive(RustEmbed)]
#[folder = "templates"]
struct Asset;

//pub(crate) async fn my_handler(event: Value, _ctx: Context) -> Result<Value, Error> {
pub(crate) async fn my_handler(event: Value, _ctx: Context) -> Result<Value, Error> {
    //info!("Event: {}", event);
    //info!("Context: {:?}", ctx);

    let api_request = serde_json::from_value::<ApiGatewayRequest>(event).expect("Failed to deser APIGW request");

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

    // get ElasticSearch URL and index names from env vars
    let config = Config::new();

    let tera = tera_init()?;

    // decode possible URL path and query string
    info!("Raw path: {}, Query: {}", &api_request.raw_path, &api_request.raw_query_string);
    let url_path = decode(&api_request.raw_path).unwrap_or_default().to_string();
    let url_query = decode(&api_request.raw_query_string).unwrap_or_default().to_string();
    let dev = match api_request.query_string_parameters {
        None => None,
        Some(v) => v.dev,
    };
    info!("Decoded path: {}, query: {}, dev: {:?}", url_path, url_query, dev);

    // send the user request downstream for processing
    let html_data = match html::html(&config, url_path, url_query, dev, api_request.headers).await {
        Ok(v) => v,
        Err(_) => return gw_response("Server Error".to_owned(), 500, 600),
    };

    // render the prepared data as HTML
    let html = tera
        .render(
            &html_data.template_name,
            &tera::Context::from_value(serde_json::to_value(&html_data).expect("Failed to serialize html_data"))
                .expect("Cannot serialize: tera::Context::from_value"),
        )
        .expect("Cannot render");
    info!("Rendered");

    // return back the result
    gw_response(html, html_data.http_resp_code, html_data.ttl)
}

/// Prepares the response with the status and HTML body. May fail and return an error.
fn gw_response(body: String, status_code: u32, ttl: u32) -> Result<Value, Error> {
    let mut headers: HashMap<String, String> = HashMap::new();
    headers.insert("Content-Type".to_owned(), "text/html".to_owned());
    headers.insert("Cache-Control".to_owned(), ["max-age=".to_owned(), ttl.to_string()].concat());

    let resp = ApiGatewayResponse {
        is_base64_encoded: false,
        status_code,
        headers,
        body,
    };

    Ok(serde_json::to_value(resp).expect("Failed to serialize response"))
}

/// Init Tera instance and load all HTML templates either from the file system
/// (debug) or the binary (release).
fn tera_init() -> Result<Tera, Error> {
    let mut tera = Tera::default();

    // loads the files from the fs or embedded strings
    // see https://github.com/pyros2097/rust-embed
    for file in Asset::iter() {
        let file: &str = &file;
        let content = Asset::get(file).expect("Cannot de-asset HTML");
        let content = std::str::from_utf8(&content.data).expect("Cannot convert HTML for str");

        tera.add_raw_template(file, content).expect("Cannot add raw template");
    }

    Ok(tera)
}
