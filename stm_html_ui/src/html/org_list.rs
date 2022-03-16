use super::html_data::HtmlData;
use crate::config::Config;
use crate::elastic;
use tracing::info;

/// Returns a list of top ORGs for a single TECH.
pub(crate) async fn html(config: &Config, langs: Vec<(String, usize)>, html_data: HtmlData) -> Result<HtmlData, ()> {
    info!("Generating ORG list for langs: {:?}, from: {}", langs, html_data.results_from);

    // there should be at least one language specified for this query to work
    // if more than one is specified then only the first one is used
    let lang = langs.first();

    // return a blank response if no valid languages were extracted from the search terms
    let lang = if lang.is_none() {
        return Ok(HtmlData {
            template_name: "org_list.html".to_owned(),
            ttl: 6000,
            http_resp_code: 404,
            meta_robots: Some("noindex".to_owned()),
            ..html_data
        });
    } else {
        // there should be at least one lang value
        lang.expect("Cannot unwrap the first lang element. It's a bug.")
            .0
            .clone()
    };

    // get the data from ES
    let devs = elastic::matching_orgs(
        &config.es_url,
        &config.org_idx,
        lang.clone(),
        html_data.results_from,
        &config.no_sql_string_invalidation_regex,
    )
    .await?;

    // put everything together for Tera
    let html_data = HtmlData {
        devs: Some(devs),
        langs: vec![(lang, 0)],
        template_name: "org_list.html".to_owned(),
        // the underlying data is slow to change
        ttl: 6000,
        http_resp_code: 200,
        ..html_data
    };

    Ok(html_data)
}
