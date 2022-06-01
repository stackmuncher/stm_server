//use elasticsearch::{http::transport::Transport, CountParts, Elasticsearch, SearchParts};
use std::fmt::Display;
use stm_shared::elastic::types as es_types;
use stm_shared::elastic::{
    call_es_api,
    validators::{escape_for_regex_fields, NO_SQL_STRING_INVALIDATION_REGEX},
};
use tracing::{error, info};

/// Maximum number of devs to return from ES in one hit
pub(crate) const MAX_DEV_LISTINGS_PER_SEARCH_RESULT: u32 = 50;

pub(crate) const QUERY_DEVS_PER_TECH: &str = r#"{"size":0,"aggs":{"agg":{"terms":{"field":"report.tech.language.keyword","size":1000,"order":{"_key":"asc"}}}}}"#;

/// Specifies ascending or descending search order.
pub(crate) enum EsSortDirection {
    Asc,
    Desc,
}

impl Display for EsSortDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EsSortDirection::Asc => write!(f, "asc"),
            EsSortDirection::Desc => write!(f, "desc"),
        }
    }
}

/// Specifies what field the results should be sorted by.
/// Uses `AscDesc` enum to specify the sort direction.
pub(crate) enum EsSortType {
    Newest,
    RecentlyActive,
}

impl Display for EsSortType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EsSortType::Newest => write!(f, "created_at"),
            EsSortType::RecentlyActive => write!(f, "report.last_contributor_commit_date_epoch"),
        }
    }
}

/// Describes the required experience level for the tech.
#[derive(juniper::GraphQLInputObject)]
pub(crate) struct TechExperience {
    /// Name of the tech, e.g. `c#`, case insensitive.
    pub tech: String,
    /// Band for the minimum number of lines of code. Valid values 1 and 2. Any other value is ignored and no LoC search clause is constructed.
    pub loc_band: Option<i32>,
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

/// Returns the count of matching docs from DEV idx depending on the params. The query is built to match the list of params and may vary in length and complexity.
/// Lang and KW params are checked for No-SQL injection.
/// * stack: a tuple of the keyword and the min number of lines for it, e.g. ("rust",1000)
/// * timezone_offset: 0..23 where anything > 12 is the negative offset
/// * timezone_hours: number of hours worked in the timezone
pub(crate) async fn matching_dev_count(
    es_url: &String,
    dev_idx: &String,
    stack: Vec<TechExperience>,
    pkgs: Vec<String>,
    timezone_offset: u32,
    timezone_hours: u32,
) -> Result<i32, ()> {
    // sample query
    // {"size":100,"track_scores":true,"query":{"bool":{"must":[{"match":{"report.tech.language.keyword":"rust"}},{"multi_match":{"query":"logger","fields":["report.tech.pkgs_kw.k.keyword","report.tech.refs_kw.k.keyword"]}},{"multi_match":{"query":"clap","fields":["report.tech.pkgs_kw.k.keyword","report.tech.refs_kw.k.keyword"]}},{"multi_match":{"query":"serde","fields":["report.tech.pkgs_kw.k.keyword","report.tech.refs_kw.k.keyword"]}}]}},"sort":[{"hireable":{"order":"desc"}},{"report.timestamp":{"order":"desc"}}]}

    // generate combined MUST clauses
    let clauses = matching_dev_clauses(stack, pkgs, timezone_offset, timezone_hours)?;

    // combine everything into a single query
    let query = [r#"{"query":{"bool":{"must":["#, &clauses, r#"]}}}"#].concat();

    // call the query
    let es_api_endpoint = [es_url.as_ref(), "/", dev_idx, "/_count"].concat();
    let es_response = call_es_api(es_api_endpoint, Some(query.to_string())).await?;

    let dev_count = match serde_json::from_value::<es_types::ESDocCount>(es_response) {
        Ok(v) => v.count,
        Err(e) => {
            error!("Failed to convert dev_count response with {}", e);
            return Err(());
        }
    };

    Ok(dev_count)
}

// not comprehensive
#[tokio::test]
async fn matching_dev_count_ok_test() {
    crate::config::init_logging();

    info!("matching_dev_count_ok_test");

    let config = crate::config::Config::new();

    let dev_count = matching_dev_count(
        &config.es_url,
        &config.dev_idx,
        vec![TechExperience {
            tech: "rust".to_string(),
            loc_band: Some(2),
        }],
        vec!["serde".to_string()],
        0,
        0,
    )
    .await
    .unwrap();

    assert!(dev_count > 100, "ES returned: {}", dev_count);
}

/// Returns the list of matching docs from DEV idx depending on the params. The query is built to match the list of params and may vary in length and complexity.
/// Lang and KW params are checked for No-SQL injection.
/// * stack: a tuple of the keyword and the min number of lines for it, e.g. ("rust",1000)
/// * timezone_offset: 0..23 where anything > 12 is the negative offset
/// * timezone_hours: number of hours worked in the timezone
pub(crate) async fn matching_dev_list(
    es_url: &String,
    dev_idx: &String,
    stack: Vec<TechExperience>,
    pkgs: Vec<String>,
    timezone_offset: u32,
    timezone_hours: u32,
    results_from: u32,
    sort_type: EsSortType,
    sort_direction: EsSortDirection,
) -> Result<Vec<es_types::GitHubUser>, ()> {
    // sample query
    // {"size":100,"track_scores":true,"query":{"bool":{"must":[{"match":{"report.tech.language.keyword":"rust"}},{"multi_match":{"query":"logger","fields":["report.tech.pkgs_kw.k.keyword","report.tech.refs_kw.k.keyword"]}},{"multi_match":{"query":"clap","fields":["report.tech.pkgs_kw.k.keyword","report.tech.refs_kw.k.keyword"]}},{"multi_match":{"query":"serde","fields":["report.tech.pkgs_kw.k.keyword","report.tech.refs_kw.k.keyword"]}}]}},"sort":[{"hireable":{"order":"desc"}},{"report.timestamp":{"order":"desc"}}]}

    // generate combined MUST clauses
    let clauses = matching_dev_clauses(stack, pkgs, timezone_offset, timezone_hours)?;

    // combine everything into a single query
    let query = [
        r#"{"size":"#,
        &MAX_DEV_LISTINGS_PER_SEARCH_RESULT.to_string(),
        r#","from": "#,
        &results_from.to_string(),
        r#","track_scores":true,"query":{"bool":{"must":["#,
        &clauses,
        r#"]}},"sort":[{""#,
        &sort_type.to_string(),
        r#"":{"order":""#,
        &sort_direction.to_string(),
        r#""}}]}"#,
    ]
    .concat();

    // call the query
    let es_api_endpoint = [es_url.as_ref(), "/", dev_idx, "/_search"].concat();
    let es_response = call_es_api(es_api_endpoint, Some(query.to_string())).await?;

    // ES returns devs reports wrapped into hits/hits that need to be unwrapped to get a pure list of reports
    let reports = match serde_json::from_value::<es_types::ESSource<es_types::GitHubUser>>(es_response) {
        Ok(v) => v
            .hits
            .hits
            .into_iter()
            .map(|hit| hit.source)
            .collect::<Vec<es_types::GitHubUser>>(),
        Err(e) => {
            error!("Failed to convert ESSource<es_types::GitHubUser> response with {}", e);
            return Err(());
        }
    };

    Ok(reports)
}

// not comprehensive
#[tokio::test]
async fn matching_dev_list_ok_test() {
    crate::config::init_logging();

    info!("matching_dev_list_ok_test");

    let config = crate::config::Config::new();

    let dev_list = matching_dev_list(
        &config.es_url,
        &config.dev_idx,
        vec![TechExperience {
            tech: "rust".to_string(),
            loc_band: Some(2),
        }],
        vec!["serde".to_string()],
        0,
        0,
        0,
        EsSortType::Newest,
        EsSortDirection::Desc,
    )
    .await
    .unwrap();

    std::fs::write(
        "samples/es-responses/matching_dev_list.json",
        serde_json::to_string_pretty(&dev_list).unwrap(),
    )
    .expect("Unable to write 'samples/es-responses/matching_dev_list.json' file");

    assert_eq!(dev_list.len(), MAX_DEV_LISTINGS_PER_SEARCH_RESULT as usize);
}

/// Returns the list of list of search clauses for ES query based on the supplied params.
/// Lang and KW params are checked for No-SQL injection.
/// * stack: a tuple of the keyword and the min number of lines for it, e.g. ("rust",1000)
/// * timezone_offset: 0..23 where anything > 12 is the negative offset
/// * timezone_hours: number of hours worked in the timezone
/// ## Returning type
/// Insert the value
/// ```
/// let query = [r#"{"query":{"bool":{"must":["#, &clauses, r#"]}}}"#].concat();
/// ```
fn matching_dev_clauses(
    stack: Vec<TechExperience>,
    pkgs: Vec<String>,
    timezone_offset: u32,
    timezone_hours: u32,
) -> Result<String, ()> {
    // sample query
    // {"size":100,"track_scores":true,"query":{"bool":{"must":[{"match":{"report.tech.language.keyword":"rust"}},{"multi_match":{"query":"logger","fields":["report.tech.pkgs_kw.k.keyword","report.tech.refs_kw.k.keyword"]}},{"multi_match":{"query":"clap","fields":["report.tech.pkgs_kw.k.keyword","report.tech.refs_kw.k.keyword"]}},{"multi_match":{"query":"serde","fields":["report.tech.pkgs_kw.k.keyword","report.tech.refs_kw.k.keyword"]}}]}},"sort":[{"hireable":{"order":"desc"}},{"report.timestamp":{"order":"desc"}}]}

    // a collector of must clauses
    let mut must_clauses: Vec<String> = Vec::new();

    // build language clause
    for lang in stack {
        // validate field_value for possible no-sql injection
        if NO_SQL_STRING_INVALIDATION_REGEX.is_match(&lang.tech) {
            error!("Invalid lang: {}", lang.tech);
            return Err(());
        }

        // get the actual number of lines for this language for this band
        let loc = lang.validated_loc();

        // language clause is different from keywords clause
        let clause = if lang.loc_band == Some(0) {
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
    for pkg in pkgs {
        // validate field_value for possible no-sql injection
        if NO_SQL_STRING_INVALIDATION_REGEX.is_match(&pkg) {
            error!("Invalid kw: {}", pkg);
            return Err(());
        }

        // query  pkgs and refs if the name is qualified or pkgs_kw and refs_kw if it's not
        let qual_unqual_clause = if pkg.contains(".") {
            r#"","fields":["report.tech.pkgs.k.keyword","report.tech.refs.k.keyword"]}}"#
        } else {
            r#"","fields":["report.keywords.keyword"]}}"#
        };

        // using multimatch because different techs have keywords in different places
        let clause = [r#"{"multi_match":{"query":""#, &pkg, qual_unqual_clause].concat();

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
    Ok(must_clauses.join(",").replace(" ", "").replace("\n", ""))
}

// not comprehensive
#[tokio::test]
async fn dev_clauses_test() {
    crate::config::init_logging();

    info!("dev_clauses_test");

    // test valid input
    let clauses = matching_dev_clauses(
        vec![TechExperience {
            tech: "rust".to_string(),
            loc_band: Some(2),
        }],
        vec!["serde".to_string()],
        4,
        10,
    )
    .unwrap();

    assert_eq!(
        &clauses,
        r#"{"nested":{"path":"report.tech","query":{"bool":{"must":[{"match":{"report.tech.language.keyword":"rust"}},{"range":{"report.tech.code_lines":{"gt":50000}}}]}}}},{"multi_match":{"query":"serde","fields":["report.keywords.keyword"]}},{"range":{"report.commit_time_histo.timezone_overlap_recent.h04":{"gte":10}}}"#,
        "Clauses: {}",
        clauses
    );

    // test invalid input
    let clauses = matching_dev_clauses(
        vec![TechExperience {
            tech: "rust`".to_string(),
            loc_band: Some(2),
        }],
        vec!["serde?".to_string()],
        0,
        10,
    );

    assert!(clauses.is_err(), "Clauses: {:?}", clauses);
}

/// Returns a list of keywords starting with the string in `starts_with`.
/// Min required length for the substring is 3 chars.
/// Input shorter than that returns None.
pub(crate) async fn keyword_suggester(
    es_url: &String,
    dev_idx: &String,
    starts_with: String,
) -> Result<Option<es_types::ESAggs>, ()> {
    // sample query
    // GET dev/_search
    // {
    //   "aggs": {
    //     "suggestions": {
    //       "terms": { "field": "report.tech.pkgs_kw.k.keyword" ,
    //         "include": "mon.*"}
    //     }
    //   },
    //   "size": 0
    // }

    // ignore queries that are too short
    let starts_with = starts_with.trim().to_lowercase();
    if starts_with.len() < 4 {
        error!("Short starts_with: {}", starts_with);
        return Ok(None);
    }

    // validate field_value for possible no-sql injection
    if NO_SQL_STRING_INVALIDATION_REGEX.is_match(&starts_with) {
        error!("Invalid starts_with: {}", starts_with);
        return Err(());
    }

    // combine everything into a single query
    let starts_with = escape_for_regex_fields(&starts_with);
    info!("Escaped kw: {}", starts_with);
    let query = [
        r#"{"aggs":{"agg":{"terms":{"field":"report.tech.pkgs_kw.k.keyword","include":""#,
        &starts_with,
        r#".*"}}},"size":0}"#,
    ]
    .concat();

    info!("{query}");

    // call the query
    let es_api_endpoint = [es_url.as_ref(), "/", dev_idx, "/_search"].concat();
    let es_response = call_es_api::<es_types::ESAggs>(es_api_endpoint, Some(query.to_string())).await?;

    // the response looks like this
    // "aggregations" : {
    //     "agg" : {
    //       "doc_count_error_upper_bound" : 19,
    //       "sum_other_doc_count" : 1351,
    //       "buckets" : ["key" : "mono","doc_count" : 15636},{"key" : "mongodb","doc_count" : 8505},{ ...

    // extract the list of keys in the order they appear in the response

    Ok(Some(es_response))
}

// not comprehensive
#[tokio::test]
async fn keyword_suggester_ok_test() {
    crate::config::init_logging();

    info!("keyword_suggester_ok_test");

    let config = crate::config::Config::new();

    let es_response = keyword_suggester(&config.es_url, &config.dev_idx, "mongo".to_string())
        .await
        .unwrap()
        .unwrap();

    let es_response = serde_json::to_string_pretty(&es_response).unwrap();

    std::fs::write("samples/es-responses/keyword_suggester.json", es_response.clone())
        .expect("Unable to write 'samples/es-responses/keyword_suggester.json' file");

    assert!(es_response.contains("mongodb"), "ES returned: {}", es_response);
}

// not comprehensive
#[tokio::test]
async fn keyword_suggester_too_short_test() {
    crate::config::init_logging();

    info!("keyword_suggester_too_short_test");

    let config = crate::config::Config::new();

    let es_response = keyword_suggester(&config.es_url, &config.dev_idx, "mon".to_string())
        .await
        .unwrap();

    assert!(es_response.is_none());
}

// not comprehensive
#[tokio::test]
async fn keyword_suggester_invalid_input_test() {
    crate::config::init_logging();

    info!("keyword_suggester_invalid_input_test");

    let config = crate::config::Config::new();

    assert!(keyword_suggester(&config.es_url, &config.dev_idx, r#".\=/(-)oir"#.to_string())
        .await
        .is_err());
}
