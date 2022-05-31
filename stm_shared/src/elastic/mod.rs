//use elasticsearch::{http::transport::Transport, CountParts, Elasticsearch, SearchParts};
use hyper::{header::HeaderValue, Body, Client, Request, Uri};
use hyper_rustls::HttpsConnectorBuilder;
use regex::Regex;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use tracing::{debug, error, info};

pub mod types;
pub mod types_aggregations;
pub mod types_hits;
pub mod types_search_log;
pub mod types_source;
pub mod validators;

/// A generic function for making signed(v4) API calls to AWS ES.
/// `es_api_endpoint` must be a fully qualified URL, e.g. https://x.ap-southeast-2.es.amazonaws.com/my_index/_search
async fn call_es_api_put(es_api_endpoint: String, payload: Vec<u8>) -> Result<(), ()> {
    // The URL will need to be split into parts to extract region, host, etc.
    let uri = Uri::from_maybe_shared(es_api_endpoint).expect("Invalid ES URL");

    // prepare a request with Content-Type header required by ES
    let req = match Request::builder().uri(uri).method("PUT").body(Body::from(payload)) {
        Ok(mut v) => {
            v.headers_mut()
                .insert("Content-Type", HeaderValue::from_static("application/json"));
            v
        }
        Err(e) => {
            error!("Invalid payload. {}", e);
            return Err(());
        }
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
    object_to_upload: Vec<u8>,
    object_id: &String,
    es_idx: &str,
) -> Result<(), ()> {
    info!("Uploading to ES idx {} as {}", es_idx, object_id);

    let es_api_endpoint = [es_url, "/", es_idx, "/_doc/", object_id].concat();
    call_es_api_put(es_api_endpoint, object_to_upload).await?;

    info!("ES upload completed");

    Ok(())
}

/// Deserializes the object into JSON and uploads into the specified ES index.
pub async fn upload_object_to_es<T: Serialize>(
    es_url: String,
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
    call_es_api_put(es_api_endpoint, object_to_upload).await?;

    info!("ES upload completed");

    Ok(())
}

/// A generic function for making signed(v4) API calls to AWS ES.
/// `es_api_endpoint` must be a fully qualified URL, e.g. https://x.ap-southeast-2.es.amazonaws.com/my_index/_search
pub async fn call_es_api<T: DeserializeOwned>(es_api_endpoint: String, payload: Option<String>) -> Result<T, ()> {
    // prepare METHOD and the payload in one step
    let (method, payload_id, payload) = match payload {
        None => ("GET", 0usize, Body::empty()),
        Some(v) => ("POST", v.len(), Body::from(v)),
    };

    info!("ES query {} started", payload_id);

    // The URL will need to be split into parts to extract region, host, etc.
    let uri = Uri::from_maybe_shared(es_api_endpoint).expect("Invalid ES URL");

    // prepare a request with Content-Type header required by ES
    let req = match Request::builder().uri(uri).method(method).body(payload) {
        Ok(mut v) => {
            v.headers_mut()
                .insert("Content-Type", HeaderValue::from_static("application/json"));
            v
        }
        Err(e) => {
            error!("Invalid payload. {}", e);
            return Err(());
        }
    };

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
    let output = Ok(serde_json::from_slice::<T>(&buf).expect("Failed to convert ES resp to a type"));
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

/// Run a search with the provided query.
/// * es_url: elastucsearch url
/// * idx: ES index name
/// * query: the query text, if any for *_search* or `None` for *_count*
pub async fn search<T: DeserializeOwned>(es_url: &String, idx: &String, query: Option<&str>) -> Result<T, ()> {
    if query.is_some() {
        let es_api_endpoint = [es_url.as_ref(), "/", idx, "/_search"].concat();
        return call_es_api::<T>(es_api_endpoint, Some(query.unwrap().to_string())).await;
    } else {
        let es_api_endpoint = [es_url.as_ref(), "/", idx, "/_count"].concat();
        return call_es_api::<T>(es_api_endpoint, None).await;
    }
}
