use super::html_data::{HtmlData, RelatedKeywords};
use crate::config::Config;
use crate::elastic;
use regex::Regex;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use stm_shared::elastic::types;
use tracing::info;

/// A struct to extract a report from _source.
#[derive(Deserialize, Debug)]
struct EngSource {
    report: Option<Report>,
}

/// A dummy Report struct that takes just one field we need out of many it has in the original structure.
#[derive(Deserialize, Debug)]
struct Report {
    tech: Option<Vec<Tech>>,
}

/// A dummy Tech struct that takes just 2 fields we need out of many it has in the original structure.
#[derive(Deserialize, Debug)]
struct Tech {
    refs_kw: Option<Vec<RelatedKeywords>>,
    pkgs_kw: Option<Vec<RelatedKeywords>>,
}

/// Returns the default home page
pub(crate) async fn html(config: &Config, html_data: HtmlData) -> Result<HtmlData, ()> {
    info!("Generating list of recently added/updated devs");

    // a query to grab a bunch of latest additions and updates to dev idx
    let devs = elastic::new_devs(&config.es_url, &config.dev_idx, html_data.results_from).await?;

    // combine everything together for Tera
    let html_data = HtmlData {
        related: Some(extract_keywords(&devs)),
        devs: Some(devs),
        template_name: "dev_search.html".to_owned(),
        ttl: 600,
        http_resp_code: 200,
        ..html_data
    };

    Ok(html_data)
}

/// Extracts ref_kw from all engineers and returns a unique list
fn extract_keywords(engineer_list: &Value) -> Vec<RelatedKeywords> {
    let mut collector: HashMap<String, usize> = HashMap::new();
    let rgx = Regex::new(r#"^_|[^\-_0-9a-zA-Z]"#).expect("Wrong _kw regex!");

    // the data we need is buried 10 levels deep - keep unwrapping until we are there
    let e_list_resp =
        serde_json::from_value::<types::ESSource<EngSource>>(engineer_list.clone()).expect("Cannot deser Eng List");

    for e_source in e_list_resp.hits.hits {
        // a report should always be present, but if it isn't we just skip the record
        if let Some(report) = e_source.source.report {
            // if tech is missing we just skip it as well
            if let Some(tech) = report.tech {
                for t in tech {
                    // code files like .cs and .rs have references (use ...)
                    if let Some(refs_kw) = t.refs_kw {
                        for kw in refs_kw {
                            // do not add rubbish ones, but log them for reference
                            if rgx.is_match(&kw.k) {
                                info!("Ignoring keyword: {}", kw.k);
                                continue;
                            }
                            // add the keyword to the list and increment its counter
                            *collector.entry(kw.k).or_insert(kw.c) += kw.c;
                        }
                    }

                    // project level files have packages like .csproj or Cargo.toml
                    // it's unlikely to have both, pkgs and refs
                    if let Some(refs_kw) = t.pkgs_kw {
                        // these are the keywords we are after
                        for kw in refs_kw {
                            // do not add rubbish ones, but log them for reference
                            if rgx.is_match(&kw.k) {
                                info!("Ignoring keyword: {}", kw.k);
                                continue;
                            }
                            // add the keyword to the list of increment its counter
                            *collector.entry(kw.k).or_insert(kw.c) += kw.c;
                        }
                    }
                }
            }
        }
    }

    // convert to a vector of `{k:"", c:""}`
    let mut ref_kws: Vec<RelatedKeywords> = collector
        .iter()
        .map(|(k, c)| RelatedKeywords {
            k: k.clone(),
            c: c.clone(),
        })
        .collect();

    info!("Dev keywords extracted: {}", ref_kws.len());

    // limit the number of keywords
    // they arrive in random order from ES and can be truncated at this point
    if ref_kws.len() > 500 {
        ref_kws = ref_kws.into_iter().take(500).collect();
    }

    // sort by keyword, case-insensitive
    ref_kws.sort_by(|a, b| a.k.to_lowercase().cmp(&b.k.to_lowercase()));

    ref_kws
}
