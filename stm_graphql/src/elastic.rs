//use elasticsearch::{http::transport::Transport, CountParts, Elasticsearch, SearchParts};
use futures::future::join;
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;
use stm_shared::elastic::types as es_types;
use stm_shared::elastic::{call_es_api, search};
use tracing::error;

pub const QUERY_DEVS_PER_TECH: &str = r#"{"size":0,"aggs":{"agg":{"terms":{"field":"report.tech.language.keyword","size":1000,"order":{"_key":"asc"}}}}}"#;

/// Describes the required experience level for the tech.
#[derive(juniper::GraphQLInputObject)]
pub(crate) struct TechExperience {
    /// Name of the tech, e.g. `c#`, case insensitive.
    tech: String,
    /// Band for the minimum number of lines of code. Valid values 1 and 2. Any other value is ignored and no LoC search clause is constructed.
    loc_band: Option<i32>,
    // /// Minimum number of years the tech was in use. Valid values 1-10. Any other value is ignored and no LoC search clause is constructed.
    // years: Option<i32>,
}

impl TechExperience {
    /// Returns a validated number of LoC for the specified band.
    /// TODO: add a param with averages per tech. E.g. 10,000 Dockerfile lines is not the same as 10,000 Rust lines
    pub(crate) fn validated_loc(&self) -> u64 {
        if let Some(band) = self.loc_band {
            // these are arbitrary numbers
            return match band {
                1 => 20_000,
                2 => 50_000,
                _ => 0,
            };
        }
        // no special LoC value was specified
        0
    }
}

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

/// Returns the count of matching docs from DEV idx depending on the params. The query is built to match the list of params and may vary in length and complexity.
/// Lang and KW params are checked for No-SQL injection.
/// * stack: a tuple of the keyword and the min number of lines for it, e.g. ("rust",1000)
/// * timezone_offset: 0..23 where anything > 12 is the negative offset
/// * timezone_hours: number of hours worked in the timezone
pub(crate) async fn matching_dev_count(
    es_url: &String,
    dev_idx: &String,
    keywords: Vec<String>,
    stack: Vec<TechExperience>,
    timezone_offset: u32,
    timezone_hours: u32,
    no_sql_string_invalidation_regex: &Regex,
) -> Result<Value, ()> {
    // sample query
    // {"query":{"bool":{"must":[{"match":{"report.tech.language.keyword":"rust"}},{"multi_match":{"query":"logger","fields":["report.tech.pkgs_kw.k.keyword","report.tech.refs_kw.k.keyword"]}},{"multi_match":{"query":"clap","fields":["report.tech.pkgs_kw.k.keyword","report.tech.refs_kw.k.keyword"]}},{"multi_match":{"query":"serde","fields":["report.tech.pkgs_kw.k.keyword","report.tech.refs_kw.k.keyword"]}}]}}}

    // a collector of must clauses
    let mut must_clauses: Vec<String> = Vec::new();

    // build language clause
    for lang in stack {
        // validate field_value for possible no-sql injection
        if no_sql_string_invalidation_regex.is_match(&lang.tech) {
            error!("Invalid lang: {}", lang.tech);
            return Err(());
        }

        // language clause is different from keywords clause
        let loc = lang.validated_loc();
        let clause = if loc == 0 {
            // a simple clause with no line counts
            [r#"{"match":{"report.tech.language.keyword":""#, &lang.tech, r#""}}"#].concat()
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
                &lang.tech,
                r#""
                            }
                          },
                          {
                            "range": {
                              "report.tech.code_lines": {
                                "gt": "#,
                &loc.to_string(),
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
    let query = [r#"{"query":{"bool":{"must":["#, &clauses, r#"]}}}"#].concat();

    // call the query
    let es_api_endpoint = [es_url.as_ref(), "/", dev_idx, "/_count"].concat();
    let es_response = call_es_api(es_api_endpoint, Some(query.to_string())).await?;

    Ok(es_response)
}

/*
/// Search related keywords and packages by a partial keyword, up to 100 of each.
/// Returns a combined list of keyword/populary count for refs_kw and pkgs_kw sorted alphabetically.
/// The keyword is checked for validity ([^\-_0-9a-zA-Z]) before inserting into the regex query.
/// Returns an error if the keyword has any extra characters or the queries fail.
pub(crate) async fn related_keywords(
    es_url: &String,
    idx: &String,
    keyword: &String,
    regex_substring_invalidation: &Regex,
) -> Result<Vec<(String, u64)>, ()> {
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
        .collect::<HashMap<String, u64>>();

    // combine the refs counts with pkgs counts
    for bucket in pkgs.aggregations.agg.buckets {
        if let Some(doc_count) = related.get_mut(&bucket.key) {
            *doc_count += bucket.doc_count;
        } else {
            related.insert(bucket.key, bucket.doc_count);
        }
    }

    // convert the combined hashmap into an array
    let mut related = related.into_iter().map(|v| (v.0, v.1)).collect::<Vec<(String, u64)>>();

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
*/
