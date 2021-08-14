//use elasticsearch::{http::transport::Transport, CountParts, Elasticsearch, SearchParts};
use futures::future::{join3, join_all};
use hyper::{Body, Client, Request, Uri};
use hyper_rustls::HttpsConnector;
use regex::Regex;
use rusoto_core::credential::{DefaultCredentialsProvider, ProvideAwsCredentials};
use rusoto_signature::signature::SignedRequest;
use serde::Deserialize;
use serde_json::Value;
use std::str::FromStr;
use std::{collections::HashMap, convert::TryInto};
use tracing::{debug, error, info};

pub const SEARCH_TOP_USERS: &str = r#"{"size":24,"query":{"match":{"hireable":{"query":"true"}}},"sort":[{"report.timestamp":{"order":"desc"}}]}"#;
pub const SEARCH_ENGINEER_BY_LOGIN: &str = r#"{"query":{"term":{"login.keyword":{"value":"%"}}}}"#;

/// Member of ESHitsCount
#[derive(Deserialize)]
struct ESHitsCountTotals {
    value: usize,
}

/// Member of ESHitsCount
#[derive(Deserialize)]
struct ESHitsCountHits {
    total: ESHitsCountTotals,
}

/// Corresponds to ES response metadata
/// ```json
/// {
///     "took" : 652,
///     "timed_out" : false,
///     "_shards" : {
///         "total" : 5,
///         "successful" : 5,
///         "skipped" : 0,
///         "failed" : 0
///     },
///     "hits" : {
///         "total" : {
///         "value" : 0,
///         "relation" : "eq"
///         },
///         "max_score" : null,
///         "hits" : [ ]
///     }
/// }
/// ```
#[derive(Deserialize)]
struct ESHitsCount {
    hits: ESHitsCountHits,
}

/// Part of ESAggs
#[derive(Deserialize)]
struct ESAggsBucket {
    pub key: String,
    pub doc_count: usize,
}

/// Part of ESAggs
#[derive(Deserialize)]
struct ESAggsBuckets {
    pub buckets: Vec<ESAggsBucket>,
}

/// Part of ESAggs
#[derive(Deserialize)]
struct ESAggsAgg {
    pub agg: ESAggsBuckets,
}

/// A generic structure for ES aggregations result. Make sure the aggregation name is `aggs`.
/// ```json
///   {
///     "aggregations" : {
///       "agg" : {
///         "buckets" : [
///           {
///             "key" : "twilio",
///             "doc_count" : 597
///           }
///         ]
///       }
///     }
///   }
/// ```
#[derive(Deserialize)]
struct ESAggs {
    pub aggregations: ESAggsAgg,
}

/// Run a search with the provided query.
/// * es_url: elastucsearch url
/// * idx: ES index name
/// * query: the query text, if any for *_search* or `None` for *_count*
pub(crate) async fn search(
    es_url: &String,
    idx: &String,
    query: Option<&str>,
) -> Result<Value, ()> {
    if query.is_some() {
        let es_api_endpoint = [es_url.as_ref(), "/", idx, "/_search"].concat();
        return call_es_api(es_api_endpoint, Some(query.unwrap().to_string())).await;
    } else {
        let es_api_endpoint = [es_url.as_ref(), "/", idx, "/_count"].concat();
        return call_es_api(es_api_endpoint, None).await;
    }
}

/// Inserts a single param in the ES query in place of %. The param may be repeated within the query multiple times.
/// Panics if the param is unsafe for no-sql queries.
pub(crate) fn add_param(
    query: &str,
    param: String,
    no_sql_string_invalidation_regex: &Regex,
) -> String {
    // validate the param
    if no_sql_string_invalidation_regex.is_match(&param) {
        panic!("Unsafe param value: {}", param);
    }

    let mut modded_query = query.to_string();

    // loop through the query until there are no more % to replace
    while modded_query.contains("%") {
        let (left, right) =
            modded_query.split_at(modded_query.find("%").expect("Cannot split the query"));

        modded_query = [left, param.as_str(), &right[1..]].concat().to_string();
    }

    modded_query
}

/// A generic function for making signed(v4) API calls to AWS ES.
/// `es_api_endpoint` must be a fully qualified URL, e.g. https://x.ap-southeast-2.es.amazonaws.com/my_index/_search
pub(crate) async fn call_es_api(
    es_api_endpoint: String,
    payload: Option<String>,
) -> Result<Value, ()> {
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
    req.set_hostname(Some(
        uri.host().expect("Missing host in ES URL").to_string(),
    ));

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
        .build::<_, hyper::Body>(HttpsConnector::with_native_roots())
        .request(req)
        .await
        .expect("ES request failed");

    info!("ES query {} response arrived", payload_id);
    let status = res.status();

    // Concatenate the body stream into a single buffer...
    let buf = hyper::body::to_bytes(res)
        .await
        .expect("Cannot convert response body to bytes");

    // there should be at least some data returned
    if buf.is_empty() {
        error!("Empty body with status {}", status);
        return Err(());
    }

    // any status other than 200 is an error
    if !status.is_success() {
        error!("Status {}", status);
        log_http_body(&buf);
        return Err(());
    }

    // all responses should be JSON. If it's not JSON it's an error.
    let output =
        Ok(serde_json::from_slice::<Value>(&buf).expect("Failed to convert ES resp to JSON"));
    info!("ES query {} finished", payload_id);
    //info!("{}", output.as_ref().unwrap()); // for debugging
    output
}

/// Returns the number of ES docs that match the query. The field name is not validated or sanitized. 
/// Returns an error if the field value contains anything other than alphanumerics and `.-_`.
pub(crate) async fn matching_doc_count(
    es_url: &String,
    idx: &String,
    field: &str,
    field_value: &String,
    no_sql_string_invalidation_regex: &Regex,
) -> Result<usize, ()> {
    // validate field_value for possible no-sql injection
    if no_sql_string_invalidation_regex.is_match(field_value) {
        error!("Invalid field_value: {}", field_value);
        return Err(());
    }

    // the query must be build inside this fn to get a consistent response
    let query = [
        r#"{"query":{"match":{""#,
        field,
        r#"":""#,
        field_value,
        r#""}},"size":0}"#,
    ]
    .concat();

    let es_api_endpoint = [
        es_url.as_ref(),
        "/",
        idx,
        "/_search?filter_path=aggregations.total.buckets",
    ]
    .concat();
    let count = call_es_api(es_api_endpoint, Some(query.to_string())).await?;

    // extract the actual value from a struct like this
    // {
    //     "took" : 652,
    //     "timed_out" : false,
    //     "_shards" : {
    //       "total" : 5,
    //       "successful" : 5,
    //       "skipped" : 0,
    //       "failed" : 0
    //     },
    //     "hits" : {
    //       "total" : {
    //         "value" : 0,
    //         "relation" : "eq"
    //       },
    //       "max_score" : null,
    //       "hits" : [ ]
    //     }
    // }
    let count = match serde_json::from_value::<ESHitsCount>(count) {
        Ok(v) => v.hits.total.value,
        Err(e) => {
            error!(
                "Failed to doc count response for idx:{}, field: {}, value: {} with {}",
                idx, field, field_value, e
            );
            return Err(());
        }
    };

    Ok(count)
}

/// Executes multiple doc counts queries in parallel and returns the results in the same order.
/// Returns an error if any of the queries fail.
pub(crate) async fn matching_doc_counts(
    es_url: &String,
    idx: &String,
    fields: Vec<&str>,
    field_value: &String,
    no_sql_string_invalidation_regex: &Regex,
) -> Result<Vec<usize>, ()> {
    let mut futures: Vec<_> = Vec::new();

    for field in fields {
        futures.push(matching_doc_count(
            es_url,
            idx,
            field,
            field_value,
            no_sql_string_invalidation_regex,
        ));
    }

    // execute all searches in parallel and unwrap the results
    let mut counts: Vec<usize> = Vec::new();
    for count in join_all(futures).await {
        match count {
            Err(_) => {
                return Err(());
            }
            Ok(v) => {
                counts.push(v);
            }
        }
    }

    Ok(counts)
}

/// Logs the body as error!(), if possible.
pub(crate) fn log_http_body(body_bytes: &hyper::body::Bytes) {
    // log the body as-is if it's not too long
    if body_bytes.len() < 5000 {
        let s = match std::str::from_utf8(&body_bytes).to_owned() {
            Err(_e) => "The body is not UTF-8".to_string(),
            Ok(v) => v.to_string(),
        };
        error!("Response body: {}", s);
    } else {
        error!("Response is too long to log: {}B", body_bytes.len());
    }
}

/// Returns up to 24 matching docs from DEV idx depending on the params. The query is built to match the list of params.
/// Lang and KW params are checked for No-SQL injection.
pub(crate) async fn matching_devs(
    es_url: &String,
    dev_idx: &String,
    keywords: Vec<String>,
    langs: Vec<String>,
    no_sql_string_invalidation_regex: &Regex,
) -> Result<Value, ()> {
    // sample query
    // {"size":24,"track_scores":true,"query":{"bool":{"must":[{"match":{"report.tech.language.keyword":"rust"}},{"multi_match":{"query":"logger","fields":["report.tech.pkgs_kw.k.keyword","report.tech.refs_kw.k.keyword"]}},{"multi_match":{"query":"clap","fields":["report.tech.pkgs_kw.k.keyword","report.tech.refs_kw.k.keyword"]}},{"multi_match":{"query":"serde","fields":["report.tech.pkgs_kw.k.keyword","report.tech.refs_kw.k.keyword"]}}]}},"sort":[{"hireable":{"order":"desc"}},{"report.timestamp":{"order":"desc"}}]}

    // a collector of must clauses
    let mut must_clauses: Vec<String> = Vec::new();

    // build language clause
    for lang in langs {
        // validate field_value for possible no-sql injection
        if no_sql_string_invalidation_regex.is_match(&lang) {
            error!("Invalid lang: {}", lang);
            return Err(());
        }

        // language clause is different from keywords clause
        let clause = [
            r#"{"match":{"report.tech.language.keyword":""#,
            &lang,
            r#""}}"#,
        ]
        .concat();

        must_clauses.push(clause);
    }

    // build keywords clauses
    for keyword in keywords {
        // validate field_value for possible no-sql injection
        if no_sql_string_invalidation_regex.is_match(&keyword) {
            error!("Invalid keyword: {}", keyword);
            return Err(());
        }

        // query  pkgs and refs if the name is qualified or pkgs_kw and refs_kw if it's not
        let qual_unqual_clause = if keyword.contains(".") {
            r#"","fields":["report.tech.pkgs.k.keyword","report.tech.refs.k.keyword"]}}"#
        } else {
            r#"","fields":["report.tech.pkgs_kw.k.keyword","report.tech.refs_kw.k.keyword"]}}"#
        };

        // using multimatch because different techs have keywords in different places
        let clause = [r#"{"multi_match":{"query":""#, &keyword, qual_unqual_clause].concat();

        must_clauses.push(clause);
    }

    // combine the clauses
    let clauses = must_clauses.join(",");

    // combine everything into a single query
    let query = [
        r#"{"size":24,"track_scores":true,"query":{"bool":{"must":["#,
        &clauses,
        r#"]}},"sort":[{"hireable":{"order":"desc"}},{"report.timestamp":{"order":"desc"}}]}"#,
    ]
    .concat();

    // call the query
    let es_api_endpoint = [es_url.as_ref(), "/", dev_idx, "/_search"].concat();
    let es_response = call_es_api(es_api_endpoint, Some(query.to_string())).await?;

    Ok(es_response)
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
pub(crate) async fn get_doc_by_id(
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

    let es_api_endpoint = [
        es_url.as_ref(),
        "/",
        idx,
        "/_doc/",
        doc_id,
        "?filter_path=_source",
    ]
    .concat();

    let es_response = call_es_api(es_api_endpoint, None).await?;

    Ok(es_response)
}

/// Search related keywords and packages by a partial keyword, up to 100 of each.
/// Returns a combined list of keyword/populary count for refs_kw and pkgs_kw sorted alphabetically.
/// The keyword is checked for validity ([^\-_0-9a-zA-Z]) before inserting into the regex query.
/// Returns an error if the keyword has any extra characters or the queries fail.
pub(crate) async fn related_keywords(
    es_url: &String,
    idx: &String,
    keyword: &String,
) -> Result<Vec<(String, usize)>, ()> {
    // validate field_value for possible no-sql injection
    let rgx = Regex::new(crate::config::SAFE_REGEX_SUBSTRING)
        .expect("Failed to compile SAFE_REGEX_SUBSTRING");
    if rgx.is_match(&keyword) {
        error!("Invalid keyword: {}", keyword);
        return Err(());
    }

    // some keywords may contain #,. or -, which should be escaped in regex
    let keyword_escaped = keyword
        .replace("#", r#"\\#"#)
        .replace(".", r#"\\."#)
        .replace("-", r#"\\-"#);

    // send a joined query to ES
    let refs = r#"{"size":0,"aggregations":{"agg":{"terms":{"field":"report.tech.refs.k.keyword","size":50,"include":"(.*\\.)?%.*"}}}}"#;
    let refs = refs.replace("%", &keyword_escaped);
    let pkgs = r#"{"size":0,"aggregations":{"agg":{"terms":{"field":"report.tech.pkgs.k.keyword","size":50,"include":"(.*\\.)?%.*"}}}}"#;
    let pkgs = pkgs.replace("%", &keyword_escaped);
    let langs = r#"{"size":0,"aggregations":{"agg":{"terms":{"field":"report.tech.language.keyword","size":50,"include":"(.*\\.)?%.*"}}}}"#;
    let langs = langs.replace("%", &keyword_escaped);
    let (refs, pkgs, langs) = join3(
        search(es_url, idx, Some(&refs)),
        search(es_url, idx, Some(&pkgs)),
        search(es_url, idx, Some(&langs)),
    )
    .await;

    // extract the data from JSON
    let refs = match serde_json::from_value::<ESAggs>(refs?) {
        Err(e) => {
            error!("Cannot deser refs with {}", e);
            return Err(());
        }
        Ok(v) => v,
    };
    let pkgs = match serde_json::from_value::<ESAggs>(pkgs?) {
        Err(e) => {
            error!("Cannot pkgs refs with {}", e);
            return Err(());
        }
        Ok(v) => v,
    };
    let langs = match serde_json::from_value::<ESAggs>(langs?) {
        Err(e) => {
            error!("Cannot deser langs with {}", e);
            return Err(());
        }
        Ok(v) => v,
    };

    // extract refs into a hashmap
    let mut related = refs
        .aggregations
        .agg
        .buckets
        .into_iter()
        .map(|v| (v.key.to_lowercase(), v.doc_count))
        .collect::<HashMap<String, usize>>();

    // combine the refs counts with pkgs counts
    for bucket in pkgs.aggregations.agg.buckets {
        if let Some(doc_count) = related.get_mut(&bucket.key) {
            *doc_count += bucket.doc_count;
        } else {
            related.insert(bucket.key, bucket.doc_count);
        }
    }

    // repeat the same for languages
    for bucket in langs.aggregations.agg.buckets {
        if let Some(doc_count) = related.get_mut(&bucket.key) {
            *doc_count += bucket.doc_count;
        } else {
            related.insert(bucket.key, bucket.doc_count);
        }
    }

    // convert the combined hashmap into an array
    let mut related = related
        .into_iter()
        .map(|v| (v.0, v.1))
        .collect::<Vec<(String, usize)>>();

    // sort the result alphabetically
    related.sort_by(|a, b| b.1.cmp(&a.1));

    Ok(related)
}

/// Reads the latest N entries from the specified stats index, e.g. stm_stats_dev_job_counts.
/// Returns the entire response as JSON Value. The index must follow a certain pattern
/// with the top element the same as the name of the query. Any other format will fail
/// at Tera transform.
/// ```json
/// {
/// "stm_stats_dev_job_counts" : {
///     "iso" : "2021-04-29T10:32:17.660423+00:00",
///     "ts" : 1619692338,
///     ...
///   }
/// }
/// ```
/// The name of the IDX is included as a field in the query, but is NOT SANITIZED.
pub(crate) async fn get_stm_stats(es_url: &String, idx: &str, count: usize) -> Result<Value, ()> {
    // e.g. GET stm_stats_dev_job_counts/_search
    let es_api_endpoint = [es_url.as_ref(), "/", idx, "/_search"].concat();

    // insert the index name in the query
    let query = [
        r#"{"size":"#,
        count.to_string().as_str(),
        r#","query":{"match_all":{}},"sort":[{""#,
        idx,
        r#".ts":{"order":"desc"}}]}"#,
    ]
    .concat();

    let es_response = call_es_api(es_api_endpoint, Some(query)).await?;

    Ok(es_response)
}
