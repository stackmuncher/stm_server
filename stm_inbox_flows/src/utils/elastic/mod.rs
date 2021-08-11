//use elasticsearch::{http::transport::Transport, CountParts, Elasticsearch, SearchParts};
use crate::config::Config;
use hyper::{Body, Client, Request, Uri};
use hyper_rustls::HttpsConnector;
use rusoto_core::credential::AwsCredentials;
use rusoto_core::signature::SignedRequest;
use serde::Serialize;
use std::convert::TryInto;
use std::str::FromStr;
use tracing::{debug, error, info};

pub(crate) mod types;

/// A generic function for making signed(v4) API calls to AWS ES.
/// `es_api_endpoint` must be a fully qualified URL, e.g. https://x.ap-southeast-2.es.amazonaws.com/my_index/_search
async fn call_es_api_put(es_api_endpoint: String, aws_credentials: &AwsCredentials, payload: String) -> Result<(), ()> {
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
    req.set_payload(Some(payload.as_bytes().to_owned()));
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
        .build::<_, hyper::Body>(HttpsConnector::with_native_roots())
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

    crate::utils::log_http_body(&buf);

    Err(())
}

/// Serializes `object_to_upload` and puts it into one of the indexes.
/// `object_to_upload` cannot be a primitive (e.g. String) - it must be a deserialisable object.
/// Use `upload_string_to_es` for uploading JSON that is already a String.
pub(crate) async fn upload_to_es<T>(
    config: &Config,
    object_to_upload: &T,
    object_id: &String,
    idx: &str,
) -> Result<(), ()>
where
    T: Serialize,
{
    info!("Uploading to ES idx {} as {}", idx, object_id);

    let es_api_endpoint = [config.es_url.as_ref(), "/", idx, "/_doc/", object_id].concat();

    let payload = match serde_json::to_string(object_to_upload) {
        Err(e) => {
            error!("Failed to serialize payload for {} due to {}", object_id, e);
            return Err(());
        }
        Ok(v) => v,
    };

    call_es_api_put(es_api_endpoint, config.aws_credentials(), payload).await?;

    info!("ES upload completed");

    Ok(())
}
