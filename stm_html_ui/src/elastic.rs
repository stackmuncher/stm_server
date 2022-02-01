//use elasticsearch::{http::transport::Transport, CountParts, Elasticsearch, SearchParts};
use crate::config::Config;
use futures::future::{join, join_all};
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;
use stm_shared::elastic::types as es_types;
use stm_shared::elastic::{call_es_api, search};
use tracing::error;

pub const SEARCH_ENGINEER_BY_LOGIN: &str = r#"{"query":{"term":{"login.keyword":{"value":"%"}}}}"#;
pub const SEARCH_DEV_BY_DOC_ID: &str = r#"{"query":{"term":{"_id":"%"}}}"#;
pub const SEARCH_ALL_LANGUAGES: &str =
    r#"{"size":0,"aggs":{"agg":{"terms":{"field":"report.tech.language.keyword","size":1000}}}}"#;

/// Inserts a single param in the ES query in place of %. The param may be repeated within the query multiple times.
/// Panics if the param is unsafe for no-sql queries.
pub(crate) fn add_param(query: &str, param: String, no_sql_string_invalidation_regex: &Regex) -> String {
    // validate the param
    if no_sql_string_invalidation_regex.is_match(&param) {
        panic!("Unsafe param value: {}", param);
    }

    let mut modded_query = query.to_string();

    // loop through the query until there are no more % to replace
    while modded_query.contains("%") {
        let (left, right) = modded_query.split_at(modded_query.find("%").expect("Cannot split the query"));

        modded_query = [left, param.as_str(), &right[1..]].concat().to_string();
    }

    modded_query
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

    let es_api_endpoint = [es_url.as_ref(), "/", idx, "/_search"].concat();
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
    let count = match serde_json::from_value::<es_types::ESHitsCount>(count) {
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
        futures.push(matching_doc_count(es_url, idx, field, field_value, no_sql_string_invalidation_regex));
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

/// Returns up to 100 matching docs from DEV idx depending on the params. The query is built to match the list of params.
/// Lang and KW params are checked for No-SQL injection.
/// * langs: a tuple of the keyword and the min number of lines for it, e.g. ("rust",1000)
/// * timezone_offset: 0..23 where anything > 12 is the negative offset
/// * timezone_hours: number of hours worked in the timezone
/// * results_from: a pagination value to be passed onto ES
pub(crate) async fn matching_devs(
    es_url: &String,
    dev_idx: &String,
    keywords: Vec<String>,
    langs: Vec<(String, usize)>,
    timezone_offset: usize,
    timezone_hours: usize,
    results_from: usize,
    no_sql_string_invalidation_regex: &Regex,
) -> Result<Value, ()> {
    // sample query
    // {"size":100,"track_scores":true,"query":{"bool":{"must":[{"match":{"report.tech.language.keyword":"rust"}},{"multi_match":{"query":"logger","fields":["report.tech.pkgs_kw.k.keyword","report.tech.refs_kw.k.keyword"]}},{"multi_match":{"query":"clap","fields":["report.tech.pkgs_kw.k.keyword","report.tech.refs_kw.k.keyword"]}},{"multi_match":{"query":"serde","fields":["report.tech.pkgs_kw.k.keyword","report.tech.refs_kw.k.keyword"]}}]}},"sort":[{"hireable":{"order":"desc"}},{"report.timestamp":{"order":"desc"}}]}

    // a collector of must clauses
    let mut must_clauses: Vec<String> = Vec::new();

    // build language clause
    for lang in langs {
        // validate field_value for possible no-sql injection
        if no_sql_string_invalidation_regex.is_match(&lang.0) {
            error!("Invalid lang: {}", lang.0);
            return Err(());
        }

        // language clause is different from keywords clause
        let clause = if lang.1 == 0 {
            // a simple clause with no line counts
            [r#"{"match":{"report.tech.language.keyword":""#, &lang.0, r#""}}"#].concat()
        } else {
            // LoC counts included in the query
            [
                r#"{
                "nested": {
                    "path": "report.tech",
                    "query": {
                      "bool": {
                        "must": [
                          {
                            "match": {
                              "report.tech.language.keyword": ""#,
                &lang.0,
                r#""
                            }
                          },
                          {
                            "range": {
                              "report.tech.code_lines": {
                                "gt": "#,
                &lang.1.to_string(),
                r#"
                              }
                            }
                          }
                        ]
                      }
                    }
                  }
                }"#,
            ]
            .concat()
            .replace(" ", "")
        };

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
            r#"","fields":["report.keywords.keyword"]}}"#
        };

        // using multimatch because different techs have keywords in different places
        let clause = [r#"{"multi_match":{"query":""#, &keyword, qual_unqual_clause].concat();

        must_clauses.push(clause);
    }

    // add timezone part
    if timezone_hours > 0 && timezone_hours <= 24 {
        let timezone_offset = if timezone_offset > 9 {
            ["h", &timezone_offset.to_string()].concat()
        } else {
            ["h0", &timezone_offset.to_string()].concat()
        };

        let clause = [
            r#"{"range":{"report.commit_time_histo.timezone_overlap_recent."#,
            &timezone_offset,
            r#"": {"gte": "#,
            &timezone_hours.to_string(),
            "}}}",
        ]
        .concat();

        error!("TZ clause: {}", clause);

        must_clauses.push(clause);
    }

    // combine the clauses
    let clauses = must_clauses.join(",");

    // combine everything into a single query
    let query = [
        r#"{"size":"#,
        &Config::MAX_DEV_LISTINGS_PER_SEARCH_RESULT.to_string(),
        r#","from": "#,
        &results_from.to_string(),
        r#","track_scores":true,"query":{"bool":{"must":["#,
        &clauses,
        r#"]}},"sort":[{"report.last_contributor_commit_date_epoch":{"order":"desc"}}]}"#,
    ]
    .concat();

    // call the query
    let es_api_endpoint = [es_url.as_ref(), "/", dev_idx, "/_search"].concat();
    let es_response = call_es_api(es_api_endpoint, Some(query.to_string())).await?;

    Ok(es_response)
}

/// Returns a list of 24 recently updated devs with publicly available email addresses.
/// ```json
/// {"size":24,"query":{"exists":{"field":"email"}},"sort":[{"updated_at":{"order":"desc"}}]}
/// ```
pub(crate) async fn new_devs(es_url: &String, dev_idx: &String, results_from: usize) -> Result<Value, ()> {
    // sample query
    // {"size":24,"query":{"match_all":{}},"sort":[{"updated_at":{"order":"desc"}}]}

    // combine everything into a single query
    let query = [
        r#"{"size":"#,
        &Config::MAX_DEV_LISTINGS_PER_SEARCH_RESULT.to_string(),
        r#","from": "#,
        &results_from.to_string(),
        r#","query":{"exists":{"field":"email"}},"sort":[{"updated_at":{"order":"desc"}}]}"#,
    ]
    .concat();

    // call the query
    let es_api_endpoint = [es_url.as_ref(), "/", dev_idx, "/_search"].concat();
    let es_response = call_es_api(es_api_endpoint, Some(query.to_string())).await?;

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
    regex_substring_invalidation: &Regex,
) -> Result<Vec<(String, usize)>, ()> {
    // validate field_value for possible no-sql injection
    if regex_substring_invalidation.is_match(&keyword) {
        error!("Invalid keyword: {}", keyword);
        return Err(());
    }

    // some keywords may contain #,. or -, which should be escaped in regex
    // ES regex search is case sensitive, but the data is all in lower-case
    // it is faster to make the KW lower case as well
    let keyword_escaped = keyword
        .to_lowercase()
        .replace("#", r#"\\#"#)
        .replace("#", r#"\\+"#)
        .replace(".", r#"\\."#)
        .replace("-", r#"\\-"#);

    // send a joined query to ES
    let refs = r#"{"size":0,"aggregations":{"agg":{"terms":{"field":"report.tech.refs.k.keyword","size":50,"include":"(.*\\.)?%.*"}}}}"#;
    let refs = refs.replace("%", &keyword_escaped);
    let pkgs = r#"{"size":0,"aggregations":{"agg":{"terms":{"field":"report.tech.pkgs.k.keyword","size":50,"include":"(.*\\.)?%.*"}}}}"#;
    let pkgs = pkgs.replace("%", &keyword_escaped);

    let (refs, pkgs) = join(search(es_url, idx, Some(&refs)), search(es_url, idx, Some(&pkgs))).await;

    // extract the data from JSON
    let refs = match serde_json::from_value::<es_types::ESAggs>(refs?) {
        Err(e) => {
            error!("Cannot deser refs with {}", e);
            return Err(());
        }
        Ok(v) => v,
    };
    let pkgs = match serde_json::from_value::<es_types::ESAggs>(pkgs?) {
        Err(e) => {
            error!("Cannot pkgs refs with {}", e);
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
