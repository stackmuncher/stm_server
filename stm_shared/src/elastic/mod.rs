//use elasticsearch::{http::transport::Transport, CountParts, Elasticsearch, SearchParts};
use hyper::{Body, Client, Request, Uri};
use hyper_rustls::HttpsConnectorBuilder;
use regex::Regex;
use rusoto_core::credential::AwsCredentials;
use rusoto_core::credential::{DefaultCredentialsProvider, ProvideAwsCredentials};
use rusoto_core::signature::SignedRequest;
use serde::Serialize;
use serde_json::Value;
use std::convert::TryInto;
use std::str::FromStr;
use tracing::{debug, error, info};

pub mod types;

/// A generic function for making signed(v4) API calls to AWS ES.
/// `es_api_endpoint` must be a fully qualified URL, e.g. https://x.ap-southeast-2.es.amazonaws.com/my_index/_search
async fn call_es_api_put(
    es_api_endpoint: String,
    aws_credentials: &AwsCredentials,
    payload: Vec<u8>,
) -> Result<(), ()> {
    // The URL will need to be split into parts to extract region, host, etc.
    let uri = Uri::from_maybe_shared(es_api_endpoint).expect("Invalid ES URL");

    // get the region from teh URL
    let region = uri
        .host()
        .expect("Missing host in ES URL")
        .trim_end_matches(".es.amazonaws.com");
    let (_, region) = region.split_at(region.rfind(".").expect("Invalid ES URL") + 1);
    let region = rusoto_core::Region::from_str(region).expect("Invalid region in the ES URL");

    // prepare the request
    let mut req = SignedRequest::new("PUT", "es", &region, uri.path());
    req.set_payload(Some(payload.to_owned()));
    req.set_hostname(Some(uri.host().expect("Missing host in ES URL").to_string()));

    // these headers are required by ES
    req.add_header("Content-Type", "application/json");

    // sign the request
    req.sign(&aws_credentials);

    // convert the signed request into an HTTP request we can send out
    let req: Request<Body> = match req.try_into() {
        Err(e) => {
            error!("Cannot convert signed request into hyper request. {}", e);
            return Err(());
        }
        Ok(v) => v,
    };

    debug!("Http rq: {:?}", req);

    // the response details are only useful if there was an error
    let res = match Client::builder()
        .build::<_, hyper::Body>(
            HttpsConnectorBuilder::new()
                .with_native_roots()
                .https_only()
                .enable_http1()
                .build(),
        )
        .request(req)
        .await
    {
        Ok(v) => v,
        Err(e) => {
            error!("ES request failed with {}", e);
            return Err(());
        }
    };

    // exit on success
    if res.status().is_success() {
        return Ok(());
    };

    // the rest of the code is uncovering the error
    error!("ES PUT failed. Status {}", res.status());

    // there should be at least some reason in the response - log what we can
    let buf = match hyper::body::to_bytes(res).await {
        Err(e) => {
            error!("Cannot convert response body to bytes. {}", e);
            return Err(());
        }
        Ok(v) => v,
    };

    super::log_http_body(&buf);

    Err(())
}

/// Put the JSON string into the specified ES index. The string must be deserialisable into a valid JSON.
pub async fn upload_serialized_object_to_es(
    es_url: &String,
    aws_credentials: &AwsCredentials,
    object_to_upload: Vec<u8>,
    object_id: &String,
    es_idx: &str,
) -> Result<(), ()> {
    info!("Uploading to ES idx {} as {}", es_idx, object_id);

    let es_api_endpoint = [es_url, "/", es_idx, "/_doc/", object_id].concat();
    call_es_api_put(es_api_endpoint, aws_credentials, object_to_upload).await?;

    info!("ES upload completed");

    Ok(())
}

/// Deserializes the object into JSON and uploads into the specified ES index.
pub async fn upload_object_to_es<T: Serialize>(
    es_url: String,
    aws_credentials: AwsCredentials,
    object_to_upload: T,
    object_id: String,
    es_idx: String,
) -> Result<(), ()> {
    // try to serialize the object
    let object_to_upload = match serde_json::to_vec(&object_to_upload) {
        Ok(v) => v,
        Err(e) => {
            error!("Failed to serialize {} for {}, error: {}", object_id, es_idx, e);
            return Err(());
        }
    };

    info!("Uploading id:{} to ES idx {}", object_id, es_idx);

    let es_api_endpoint = [es_url.as_ref(), "/", es_idx.as_ref(), "/_doc/", object_id.as_ref()].concat();
    call_es_api_put(es_api_endpoint, &aws_credentials, object_to_upload).await?;

    info!("ES upload completed");

    Ok(())
}

/// A generic function for making signed(v4) API calls to AWS ES.
/// `es_api_endpoint` must be a fully qualified URL, e.g. https://x.ap-southeast-2.es.amazonaws.com/my_index/_search
pub async fn call_es_api(es_api_endpoint: String, payload: Option<String>) -> Result<Value, ()> {
    // prepare METHOD and the payload in one step
    let (method, payload) = match payload {
        None => ("GET", None),
        Some(v) => ("POST", Some(v.as_bytes().to_owned())),
    };
    let payload_id = if payload.is_none() {
        0usize
    } else {
        payload.as_ref().unwrap().len()
    };
    info!("ES query {} started", payload_id);

    // The URL will need to be split into parts to extract region, host, etc.
    let uri = Uri::from_maybe_shared(es_api_endpoint).expect("Invalid ES URL");

    // get the region from teh URL
    let region = uri
        .host()
        .expect("Missing host in ES URL")
        .trim_end_matches(".es.amazonaws.com");
    let (_, region) = region.split_at(region.rfind(".").expect("Invalid ES URL") + 1);
    let region = rusoto_core::Region::from_str(region).expect("Invalid region in the ES URL");

    // prepare the request
    let mut req = SignedRequest::new(method, "es", &region, uri.path());
    req.set_payload(payload);
    req.set_hostname(Some(uri.host().expect("Missing host in ES URL").to_string()));

    // these headers are required by ES
    req.add_header("Content-Type", "application/json");

    // get AWS creds
    let provider = DefaultCredentialsProvider::new().expect("Cannot get default creds provider");
    let credentials = provider.credentials().await.expect("Cannot find creds");

    // sign the request
    req.sign(&credentials);

    // convert the signed request into an HTTP request we can send out
    let req: Request<Body> = req
        .try_into()
        .expect("Cannot convert signed request into hyper request");
    debug!("Http rq: {:?}", req);

    let res = Client::builder()
        .build::<_, hyper::Body>(
            HttpsConnectorBuilder::new()
                .with_native_roots()
                .https_only()
                .enable_http1()
                .build(),
        )
        .request(req)
        .await
        .expect("ES request failed");

    let status = res.status();

    // Concatenate the body stream into a single buffer...
    let buf = hyper::body::to_bytes(res)
        .await
        .expect("Cannot convert response body to bytes");

    info!("ES query {} response: {} bytes", payload_id, buf.len());

    // there should be at least some data returned
    if buf.is_empty() {
        error!("Empty body with status {}", status);
        return Err(());
    }

    // any status other than 200 is an error
    if !status.is_success() {
        error!("Status {}", status);
        super::log_http_body(&buf);
        return Err(());
    }

    // all responses should be JSON. If it's not JSON it's an error.
    let output = Ok(serde_json::from_slice::<Value>(&buf).expect("Failed to convert ES resp to JSON"));
    info!("ES query {} deserialized", payload_id);
    //info!("{}", output.as_ref().unwrap()); // for debugging
    output
}

/// Reads a single document by ID.
/// Returns `_source` as the root tag with `hits` and meta sections stripped off.
/// ```json
///   {
///     "_source" : {
///       "repo" : [
///         {
///           "ts" : 1615195803,
///           "iso" : "2021-03-08T09:30:03.966075280+00:00",
///           "c" : 1725617
///         }
///       ]
///     }
///   }
/// ```
pub async fn get_doc_by_id(
    es_url: &String,
    idx: &String,
    doc_id: &str,
    no_sql_string_invalidation_regex: &Regex,
) -> Result<Value, ()> {
    // validate field_value for possible no-sql injection
    if no_sql_string_invalidation_regex.is_match(doc_id) {
        error!("Invalid doc_id: {}", doc_id);
        return Err(());
    }

    let es_api_endpoint = [es_url.as_ref(), "/", idx, "/_doc/", doc_id, "?filter_path=_source"].concat();

    let es_response = call_es_api(es_api_endpoint, None).await?;

    Ok(es_response)
}
