use crate::config::Config;
use crate::elastic;
use html_data::{HtmlData, KeywordMetadata};
use regex::Regex;
use tracing::{info, warn};

mod dev_profile;
mod gh_login_profile;
mod home;
mod html_data;
mod keyword;
mod related;
mod stats;

const MAX_NUMBER_OF_VALID_SEARCH_TERMS: usize = 4;
const MAX_NUMBER_OF_SEARCH_TERMS_TO_CHECK: usize = 6;

/// Routes HTML requests to processing modules. Returns HTML response and TTL value in seconds.
pub(crate) async fn html(
    config: &Config,
    url_path: String,
    url_query: String,
    dev: Option<String>,
) -> Result<HtmlData, ()> {
    // prepare a common structure for feeding into Tera templates
    let html_data = HtmlData {
        raw_search: url_query.clone(),
        related: None,
        devs: None,
        keywords: Vec::new(),
        keywords_meta: Vec::new(),
        langs: Vec::new(),
        keywords_str: None,
        stats: None,
        template_name: "404.html".to_owned(),
        ttl: 600,
        http_resp_code: 404,
        meta_robots: None,
        login_str: None,
        owner_id_str: None,
        stats_jobs: None,
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
        if !config.owner_id_validation_regex.is_match(&owner_id) {
            warn!("Invalid owner_id: {} from {}", owner_id, url_query);
            return Ok(html_data);
        }

        // return dev profile page
        return dev_profile::html(config, owner_id, html_data).await;
    }

    // is there something in the query string?
    if url_query.len() > 1 {
        // split the query into parts using a few common separators
        let rgx = Regex::new(r#"[#\-\._0-9a-zA-Z]+"#).expect("Wrong search terms regex!");
        let search_terms = rgx
            .find_iter(&url_query)
            .map(|v| v.as_str().to_owned())
            .collect::<Vec<String>>();
        info!("Terms: {:?}", search_terms);

        // normalise and dedupe the search terms
        let mut search_terms = search_terms.iter().map(|v| v.to_lowercase()).collect::<Vec<String>>();
        search_terms.dedup();
        let search_terms = search_terms;

        // will contain values that matches language names
        let mut langs: Vec<String> = Vec::new();
        // will contain the list of keywords to search for
        let mut keywords: Vec<String> = Vec::new();
        // every search term submitted by the user with the meta of how it was understood
        let mut keywords_meta: Vec<KeywordMetadata> = Vec::new();

        // check every search term for what type of a term it is
        for (search_term_idx, search_term) in search_terms.into_iter().enumerate() {
            // searches with a tailing or leading . should be cleaned up
            // it may be possible to have a lead/trail _, maybe
            // I havn't seen a lead/trail - anywhere
            let search_term = search_term.trim_matches('.').trim_matches('-').to_owned();

            // limit the list of valid search terms to 4
            if search_term_idx >= MAX_NUMBER_OF_SEARCH_TERMS_TO_CHECK
                || keywords.len() + langs.len() >= MAX_NUMBER_OF_VALID_SEARCH_TERMS
            {
                // this term got no results and will be ignored
                keywords_meta.push(KeywordMetadata {
                    search_term: search_term,
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
            let (fields, can_be_lang) = if search_term.contains(".") {
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
                &search_term,
                &config.no_sql_string_invalidation_regex,
            )
            .await?;
            info!("search_term {}: {:?}", search_term, counts);

            if can_be_lang {
                // there are 3 search results if it can be a language

                // store the metadata for this search term
                keywords_meta.push(KeywordMetadata {
                    search_term: search_term.clone(),
                    es_keyword_count: counts[1] + counts[2],
                    es_package_count: 0,
                    es_language_count: counts[0],
                    unknown: (counts[0] + counts[1] + counts[2]) == 0,
                    too_many: false,
                });

                // extract useful terms to be used in the search
                // this may be a language
                if counts[0] > 0 {
                    langs.push(search_term);
                } else if counts[1] > 0 || counts[2] > 0 {
                    // add it to the list of keywords if there is still room
                    keywords.push(search_term);
                }
            } else if counts[0] > 0 || counts[1] > 0 {
                // only 2 results if it looks like a package

                // store the metadata for this search term
                keywords_meta.push(KeywordMetadata {
                    search_term: search_term.clone(),
                    es_keyword_count: counts[0],
                    es_package_count: counts[1],
                    es_language_count: 0,
                    unknown: (counts[0] + counts[1]) == 0,
                    too_many: false,
                });

                // .-notation, so can't be a language, but can be a keyword
                // add it to the list of keywords if there is still room
                keywords.push(search_term);
            } else {
                // this term got no results and will be ignored
                keywords_meta.push(KeywordMetadata {
                    search_term: search_term,
                    es_keyword_count: 0,
                    es_package_count: 0,
                    es_language_count: 0,
                    unknown: true,
                    too_many: false,
                });
            }
        }

        // update keyword metadata for the output
        // they should be sorted in the same order as the search terms, which were
        // sorted earlier
        // the sort order has to be enforced for URL consistency
        let html_data = HtmlData {
            keywords_meta,
            ..html_data
        };

        // run a keyword search
        return Ok(keyword::html(config, keywords, langs, html_data).await?);
    }

    // return the homepage if there is nothing else
    return Ok(home::html(config, html_data).await?);
}
