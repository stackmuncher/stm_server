use crate::config::{validate_owner_id, Config};
use crate::{
    elastic,
    search_log::{send_to_sqs, SearchLog},
};
use chrono::Utc;
use html_data::{HtmlData, KeywordMetadata};
use regex::Regex;
use std::collections::HashMap;
use tracing::{debug, info, warn};

mod dev_profile;
mod dev_search;
mod gh_login_profile;
mod home;
pub(crate) mod html_data;
mod related;
mod stats;

const MAX_NUMBER_OF_VALID_SEARCH_TERMS: usize = 4;
const MAX_NUMBER_OF_SEARCH_TERMS_TO_CHECK: usize = 6;
const MAX_NUMBER_OF_LOC_PER_SEARCH_TERM: usize = 100_000;

/// Routes HTML requests to processing modules. Returns HTML response and TTL value in seconds.
pub(crate) async fn html(
    config: &Config,
    url_path: String,
    url_query: String,
    dev: Option<String>,
    headers: HashMap<String, String>,
) -> Result<HtmlData, ()> {
    // prepare a common structure for feeding into Tera templates
    let html_data = HtmlData {
        raw_search: url_query.clone(),
        related: None,
        devs: None,
        stack_stats: None,
        keywords: Vec::new(),
        keywords_meta: Vec::new(),
        langs: Vec::new(),
        keywords_str: None,
        template_name: "404.html".to_owned(),
        ttl: 600,
        http_resp_code: 404,
        meta_robots: None,
        login_str: None,
        owner_id_str: None,
        stats_jobs: None,
        headers,
        timestamp: Utc::now(),
        availability_tz: None,
        availability_tz_hrs: None,
        page_number: 1,
        results_from: 0,
        devs_per_page: Config::MAX_DEV_LISTINGS_PER_SEARCH_RESULT,
        max_pages: Config::MAX_PAGES_PER_SEARCH_RESULT,
        // this is a temporary plug until caching is implemented
        all_langs: config.all_langs.clone(),
    };

    // return 404 for requests that are too long or for some resource related to the static pages
    if url_path.len() > 100 || url_query.len() > 100 {
        warn!("Invalid request: {} / {}", url_path, url_query);
        return Ok(html_data);
    }
    if url_path.starts_with("/about/") || url_path.starts_with("/robots.txt") {
        warn!("Static resource request: {}", url_path);
        return Ok(html_data);
    }

    // is it a stats page?
    if url_path.trim_end_matches("/") == "/_stats" {
        // return stats page
        return Ok(stats::html(config, html_data).await?);
    }

    // is it a related keyword search?
    if url_path.trim_end_matches("/") == "/_related" {
        // return related keywords page
        return Ok(related::html(config, url_query, html_data).await?);
    }

    // check if there is a path - it can be the developer login
    // there shouldn't be any other paths at this stage
    if url_path.len() > 1 {
        // it must be a dev login that matches the one on github, e.g. rimutaka
        let login = url_path
            .trim()
            .trim_end_matches("/")
            .trim_start_matches("/")
            .trim()
            .to_string();

        // is it a valid format for a dev login?
        if config.no_sql_string_invalidation_regex.is_match(&login) {
            warn!("Invalid dev login: {}", url_path);
            return Ok(html_data);
        }

        // return dev profile page
        return gh_login_profile::html(config, login, html_data).await;
    }

    // check if dev ID was specified
    if let Some(owner_id) = dev {
        let owner_id = owner_id.trim().to_owned();

        // is it a valid format for a dev login?
        if !validate_owner_id(&owner_id) {
            warn!("Invalid owner_id: {} from {}", owner_id, url_query);
            return Ok(html_data);
        }

        // return dev profile page
        return dev_profile::html(config, owner_id, html_data).await;
    }

    // is there something in the query string?
    if url_query.len() > 1 {
        // extract the page number part from the query
        let (url_query, page_number, results_from) =
            extract_page_num_part_from_query(url_query, &config.page_num_terms_regex);
        let html_data = HtmlData {
            raw_search: url_query.clone(),
            page_number,
            results_from,
            ..html_data
        };

        // extract the timezone part from the query
        let (url_query, tz_hours, tz_offset) =
            extract_timezone_part_from_query(url_query, &config.timezone_terms_regex);

        // split the query into parts using a few common separators
        let search_terms = config
            .search_terms_regex
            .find_iter(&url_query)
            .map(|v| v.as_str().to_owned())
            .collect::<Vec<String>>();
        info!("Terms: {:?}", search_terms);

        // normalise and dedupe the search terms
        let mut search_terms = search_terms.iter().map(|v| v.to_lowercase()).collect::<Vec<String>>();
        search_terms.dedup();
        let search_terms = search_terms;

        // will contain values that matches language names
        let mut langs: Vec<(String, usize)> = Vec::new();
        // will contain the list of keywords to search for
        let mut keywords: Vec<String> = Vec::new();
        // every search term submitted by the user with the meta of how it was understood
        let mut keywords_meta: Vec<KeywordMetadata> = Vec::new();

        // check every search term for what type of a term it is
        for (search_term_idx, search_term) in search_terms.into_iter().enumerate() {
            // searches with a tailing or leading . should be cleaned up
            // it may be possible to have a lead/trail _, in python for example
            // I havn't seen a lead/trail - anywhere
            // Trailing + and # are OK, e.g. C# and C++
            let search_term = search_term
                .trim_matches('.')
                .trim_matches('-')
                .trim_start_matches("+")
                .trim_start_matches("#")
                .to_owned();

            // check if there is anything left after trimming
            if search_term.is_empty() {
                continue;
            }

            // extract LoC part / split into term and LoC
            let search_term = match extract_loc_part_from_search_term(search_term.clone()) {
                Some(v) => v,
                None => {
                    // this term is invalid and will be ignored
                    keywords_meta.push(KeywordMetadata {
                        search_term: search_term,
                        search_term_loc: 0,
                        es_keyword_count: 0,
                        es_package_count: 0,
                        es_language_count: 0,
                        unknown: true,
                        too_many: false,
                    });

                    continue;
                }
            };

            // limit the list of valid search terms to 4
            if search_term_idx >= MAX_NUMBER_OF_SEARCH_TERMS_TO_CHECK
                || keywords.len() + langs.len() >= MAX_NUMBER_OF_VALID_SEARCH_TERMS
            {
                // this term got no results and will be ignored
                keywords_meta.push(KeywordMetadata {
                    search_term: search_term.0,
                    search_term_loc: search_term.1,
                    es_keyword_count: 0,
                    es_package_count: 0,
                    es_language_count: 0,
                    unknown: false,
                    too_many: true,
                });

                continue;
            }

            // searching for a keyword is different from searching for a fully qualified package name
            // e.g. xml vs System.XML vs SomeVendor.XML
            let (fields, can_be_lang) = if search_term.0.contains(".") {
                // this is a fully qualified name and cannot be a language
                (vec!["report.tech.refs.k.keyword", "report.tech.pkgs.k.keyword"], false)
            } else {
                // this is a keyword, which may be all there is, but it will be in _kw field anyway
                // this can also be a language
                (
                    vec![
                        "report.tech.language.keyword",
                        "report.tech.refs_kw.k.keyword",
                        "report.tech.pkgs_kw.k.keyword",
                    ],
                    true,
                )
            };

            // get the doc counts for the term
            let counts = elastic::matching_doc_counts(
                &config.es_url,
                &config.dev_idx,
                fields,
                &search_term.0,
                &config.no_sql_string_invalidation_regex,
            )
            .await?;
            info!("search_term {}/{}: {:?}", search_term.0, search_term.1, counts);

            if can_be_lang {
                // there should be 3 search results if it can be a language

                // keyword counts may not be there is it's just the language
                let es_keyword_count = counts.iter().skip(1).map(|v| v).sum::<usize>();

                // store the metadata for this search term
                keywords_meta.push(KeywordMetadata {
                    search_term: search_term.0.clone(),
                    search_term_loc: search_term.1,
                    es_keyword_count: es_keyword_count,
                    es_package_count: 0,
                    es_language_count: counts[0],
                    unknown: (counts[0] + es_keyword_count) == 0,
                    too_many: false,
                });

                // extract useful terms to be used in the search
                // this may be a language
                if counts[0] > 0 {
                    langs.push((search_term.0, search_term.1));
                } else if es_keyword_count > 0 {
                    // add it to the list of keywords if there is still room
                    keywords.push(search_term.0);
                }
            } else if counts[0] > 0 || counts[1] > 0 {
                // only 2 results if it looks like a package

                // store the metadata for this search term
                keywords_meta.push(KeywordMetadata {
                    search_term: search_term.0.clone(),
                    search_term_loc: search_term.1,
                    es_keyword_count: counts[0],
                    es_package_count: counts[1],
                    es_language_count: 0,
                    unknown: (counts[0] + counts[1]) == 0,
                    too_many: false,
                });

                // .-notation, so can't be a language, but can be a keyword
                // add it to the list of keywords if there is still room
                keywords.push(search_term.0);
            } else {
                // this term got no results and will be ignored
                keywords_meta.push(KeywordMetadata {
                    search_term: search_term.0,
                    search_term_loc: search_term.1,
                    es_keyword_count: 0,
                    es_package_count: 0,
                    es_language_count: 0,
                    unknown: true,
                    too_many: false,
                });
            }
        }

        // prepare timezone availability data
        let (availability_tz, availability_tz_hrs) = if tz_hours > 0 {
            if tz_offset > 14 {
                (Some(["UTC-0", (24 - tz_offset).to_string().as_str()].concat()), Some(tz_hours))
            } else if tz_offset > 12 {
                (Some(["UTC-", (24 - tz_offset).to_string().as_str()].concat()), Some(tz_hours))
            } else if tz_offset > 9 {
                (Some(["UTC+", tz_offset.to_string().as_str()].concat()), Some(tz_hours))
            } else {
                (Some(["UTC+0", tz_offset.to_string().as_str()].concat()), Some(tz_hours))
            }
        } else {
            (None, None)
        };

        // update keyword metadata for the output
        // they should be sorted in the same order as the search terms, which were
        // sorted earlier
        // the sort order has to be enforced for URL consistency
        let html_data = HtmlData {
            keywords_meta,
            availability_tz,
            availability_tz_hrs,
            ..html_data
        };

        // run a keyword search for devs
        let html_data = dev_search::html(config, keywords, langs, tz_offset, tz_hours, html_data).await?;

        // log the search query and its results in a DB via SQS
        if !html_data.raw_search.is_empty() {
            send_to_sqs(&SearchLog::from(&html_data), &config.sqs_client, &config.search_log_sqs_url).await;
        }

        return Ok(html_data);
    }

    // return the homepage if there is nothing else
    return Ok(home::html(config, html_data).await?);
}

/// Extracts and validates the min number of lines included in the search as `rust:2000`.
/// Returns None if the value is invalid.
fn extract_loc_part_from_search_term(term: String) -> Option<(String, usize)> {
    // return the term as-is if no loc was provided
    if !term.contains(":") {
        return Some((term, 0));
    }

    // split at the first :
    if let Some((term, loc)) = term.split_once(":") {
        if term.is_empty() {
            // e.g. :1000
            return None;
        }
        // try to convert into a number
        if let Ok(loc) = usize::from_str_radix(loc, 10) {
            // limit the max value
            let loc = loc.min(MAX_NUMBER_OF_LOC_PER_SEARCH_TERM);

            return Some((term.to_owned(), loc));
        }
    }

    // it failed validation
    None
}

/// Extracts the page part from the query and returns:
/// * the query without the page part
/// * the page number and the FROM value for ES
/// All values are validated and can be plugged directly into a query.
fn extract_page_num_part_from_query(url_query: String, page_num_terms_regex: &Regex) -> (String, usize, usize) {
    debug!("Extracting page-num part from: {}", url_query);
    // is there any page number info?
    let captures = match page_num_terms_regex.captures(&url_query) {
        Some(v) => v,
        None => {
            info!("No page-num part - no captures");
            return (url_query, 1, 0);
        }
    };

    // get the time part of the query, if any
    let full_match = match captures.get(0) {
        Some(v) => v.as_str(),
        None => {
            info!("No page-num part - empty capture");
            return (url_query, 1, 0);
        }
    };

    info!(
        "Full page-num part: {}, captures: {}, new url_query: {}",
        full_match,
        captures.len(),
        url_query
    );

    // get the page number
    let page_number = match captures.get(1) {
        Some(v) => v.as_str(),
        None => {
            info!("No page-num part - no number");
            return (url_query, 1, 0);
        }
    };

    // remote the page part from the query
    let url_query = url_query.replace(full_match, " ").trim().to_string();

    // validate the number
    let page_number = match usize::from_str_radix(page_number, 10) {
        Ok(v) if v > 0 && v <= Config::MAX_PAGES_PER_SEARCH_RESULT => v,
        _ => {
            debug!("No page-num part - invalid number");
            return (url_query, 1, 0);
        }
    };

    // this value is needed for ES
    let results_from = (page_number - 1) * Config::MAX_DEV_LISTINGS_PER_SEARCH_RESULT;

    // all parts were collected and validated
    info!("Page-num part: {}, from: {}", page_number, results_from);
    (url_query, page_number, results_from)
}

#[test]
fn extract_page_num_part_from_query_test() {
    let config = Config::new();
    let rgx = config.page_num_terms_regex;

    let vals = vec![
        // valid values
        ("rust p:10", 10usize, 450usize, "rust"),
        ("p:10", 10, 450, ""),
        (" p:10", 10, 450, ""),
        ("p:10 ", 10, 450, ""),
        ("p:10 rust", 10, 450, "rust"),
        ("rust,p:10,", 10, 450, "rust"),
        // invalid values
        ("p:-100", 1, 0, "p:-100"),
        ("rust,p:100,", 1, 0, "rust"),
        ("rust,p:11114,", 1, 0, "rust"),
        ("rustp:10", 1, 0, "rustp:10"),
        ("p:10a", 1, 0, "p:10a"),
        (" kap:10", 1, 0, " kap:10"),
        ("-p:10 rust", 1, 0, "-p:10 rust"),
        ("p:10- rust", 1, 0, "p:10- rust"),
        ("rust.p:10", 1, 0, "rust.p:10"),
        (":p:10", 1, 0, ":p:10"),
        (" a:p:10", 1, 0, " a:p:10"),
        ("p:10#rust", 1, 0, "p:10#rust"),
    ];

    for val in vals {
        let res = extract_page_num_part_from_query(val.0.to_string(), &rgx);
        assert_eq!(val.1, res.1, "`{}`", val.0);
        assert_eq!(val.2, res.2, "`{}`", val.0);
        assert_eq!(val.3, &res.0, "`{}`", val.0);
    }
}

/// Extracts the timezone part from the query and returns:
/// * the query without the timezone part
/// * hours in the timezone
/// * hours of the timezone, +/-
/// All values are validated and can be plugged directly into a query.
fn extract_timezone_part_from_query(url_query: String, timezone_terms_regex: &Regex) -> (String, usize, usize) {
    debug!("Extracting TZ part from: {}", url_query);
    // is there any timezone info?
    let captures = match timezone_terms_regex.captures(&url_query) {
        Some(v) => v,
        None => {
            info!("No TZ part - no captures");
            return (url_query, 0, 0);
        }
    };

    // remove the tz info from the query
    let full_match = match captures.get(0) {
        Some(v) => v.as_str(),
        None => {
            info!("No TZ part - empty capture");
            return (url_query, 0, 0);
        }
    };

    debug!("Full TZ part: {}, captures: {}, new url_query: {}", full_match, captures.len(), url_query);

    // get the required number of hours
    let hours = match captures.get(1) {
        Some(v) => v.as_str(),
        None => {
            info!("No TZ part - no hrs");
            return (url_query, 0, 0);
        }
    };
    // validate the hours
    let hours = match usize::from_str_radix(hours, 10) {
        Ok(v) if v > 0 && v <= 24 => v,
        _ => {
            debug!("No TZ part - invalid hrs");
            return (url_query, 0, 0);
        }
    };

    // remote the UTC part from the query
    let updated_url_query = url_query.replace(full_match, " ");

    // get the timezone
    let tz = match captures.get(2) {
        Some(v) => v.as_str(),
        None => {
            // no UTC offset was specified, e.g. 4UTC
            debug!("TZ part: {}hrs, 0 offset", hours);
            return (updated_url_query, hours, 0);
        }
    };
    // validate the tz
    let tz = match i32::from_str_radix(tz, 10) {
        Ok(v) if v >= 0 && v <= 12 => v as usize,
        Ok(v) if v >= -12 && v < 0 => (24 + v) as usize,
        _ => {
            // almost there, but the TZ offset is invalid, so the entire TZ search is ignored
            debug!("No TZ part - invalid hrs");
            return (url_query, 0, 0);
        }
    };

    // all parts were collected and validated
    info!("TZ part: {}hrs, h{} offset", hours, tz);
    (updated_url_query, hours, tz)
}

#[test]
fn extract_timezone_part_from_query_test() {
    let config = Config::new();
    let rgx = config.timezone_terms_regex;

    let vals = vec![
        // positive offset
        ("5utc+03", 5usize, 3usize, " "),
        ("5utc+03 ", 5, 3, " "),
        (" 5utc+03", 5, 3, " "),
        (" 5utc+03 ", 5, 3, " "),
        ("rust 5utc+03", 5, 3, "rust "),
        ("5utc+03 rust", 5, 3, " rust"),
        ("rust 5utc+03 serde", 5, 3, "rust serde"),
        ("5utc+03a", 0, 0, "5utc+03a"),
        ("a5utc+03", 0, 0, "a5utc+03"),
        ("a5utc+03a", 0, 0, "a5utc+03a"),
        ("5utc+", 0, 0, "5utc+"),
        ("5utc+a", 0, 0, "5utc+a"),
        ("5utc+ ", 0, 0, "5utc+ "),
        ("5utc- ", 0, 0, "5utc- "),
        // negative offset
        ("5utc-03", 5, 21, " "),
        ("5utc-03 ", 5, 21, " "),
        (" 5utc-03", 5, 21, " "),
        (" 5utc-03 ", 5, 21, " "),
        ("rust 5utc-03", 5, 21, "rust "),
        ("5utc-03 rust", 5, 21, " rust"),
        ("rust 5utc-03 serde", 5, 21, "rust serde"),
        ("5utc-03a", 0, 0, "5utc-03a"),
        ("a5utc-03", 0, 0, "a5utc-03"),
        ("a5utc-03a", 0, 0, "a5utc-03a"),
        // no offset
        ("5utc", 5, 0, " "),
        ("5utc ", 5, 0, " "),
        (" 5utc", 5, 0, " "),
        (" 5utc ", 5, 0, " "),
        ("rust 5utc", 5, 0, "rust "),
        ("5utc rust", 5, 0, " rust"),
        ("rust 5utc serde", 5, 0, "rust serde"),
        ("rust 5utc+0 serde", 5, 0, "rust serde"),
        ("rust 5utc-0 serde", 5, 0, "rust serde"),
        ("rust 5utc+00 serde", 5, 0, "rust serde"),
        ("rust 5utc-00 serde", 5, 0, "rust serde"),
        ("5utca", 0, 0, "5utca"),
        ("a5utc", 0, 0, "a5utc"),
        ("a5utca", 0, 0, "a5utca"),
        // one tz digit
        ("5utc+3", 5, 3, " "),
        ("5utc+3 ", 5, 3, " "),
        (" 5utc+3", 5, 3, " "),
        (" 5utc+3 ", 5, 3, " "),
        ("rust 5utc+3", 5, 3, "rust "),
        ("5utc+3 rust", 5, 3, " rust"),
        ("rust 5utc+3 serde", 5, 3, "rust serde"),
        ("5utc+3a", 0, 0, "5utc+3a"),
        ("a5utc+3", 0, 0, "a5utc+3"),
        ("a5utc+3a", 0, 0, "a5utc+3a"),
        // UPPER-CASE
        ("5UTC+3", 5, 3, " "),
        ("5HRS@utc+3", 5, 3, " "),
        // optional @, hr@, hrs@
        ("5@utc+3", 5, 3, " "),
        ("5hrs@utc+3", 5, 3, " "),
        ("5hr@utc+3", 5, 3, " "),
        ("5h@utc+3", 5, 3, " "),
        ("5hrr@utc+3", 0, 0, "5hrr@utc+3"),
        ("5@@utc+3", 0, 0, "5@@utc+3"),
        ("@5utc+3", 0, 0, "@5utc+3"),
        // no match
        ("rust 5utc-x serde", 0, 0, "rust 5utc-x serde"),
        ("rust utc-5 serde", 0, 0, "rust utc-5 serde"),
        ("-5utc-3", 0, 0, "-5utc-3"),
        ("100utc+03", 0, 0, "100utc+03"),
        ("5utc-300", 0, 0, "5utc-300"),
        // hours bounds
        ("0utc+03", 0, 0, "0utc+03"),
        ("5utc+03", 5, 3, " "),
        ("20utc+03", 20, 3, " "),
        ("24utc+03", 24, 3, " "),
        ("25utc+03", 0, 0, "25utc+03"),
        ("10utc-11", 10, 13, " "),
        ("10utc-1", 10, 23, " "),
        ("10utc-12", 10, 12, " "),
        ("10utc-13", 0, 0, "10utc-13"),
        ("10utc+1", 10, 1, " "),
        ("10utc+12", 10, 12, " "),
        ("10utc+13", 0, 0, "10utc+13"),
    ];

    for val in vals {
        let res = extract_timezone_part_from_query(val.0.to_string(), &rgx);
        assert_eq!(val.1, res.1, "`{}`", val.0);
        assert_eq!(val.2, res.2, "`{}`", val.0);
        assert_eq!(val.3, &res.0, "`{}`", val.0);
    }
}
