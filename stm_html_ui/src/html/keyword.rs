use super::html_data::HtmlData;
use crate::config::Config;
use crate::elastic;
use tracing::info;

/// Returns package names containing the keyword and engineers using them.
/// * timezone_offset: 0..23 where anything > 12 is the negative offset
/// * timezone_hours: number of hours worked in the timezone
/// The timezone part of the query is ignored if the hours are zero.
pub(crate) async fn html(
    config: &Config,
    keywords: Vec<String>,
    langs: Vec<(String, usize)>,
    timezone_offset: usize,
    timezone_hours: usize,
    html_data: HtmlData,
) -> Result<HtmlData, ()> {
    info!("Generating html-keyword");
    info!("KWs: {:?}", keywords);
    info!("Lang: {:?}", langs);

    // return a blank response if no valid keywords were extracted from the search terms
    if keywords.is_empty() && langs.is_empty() {
        return Ok(HtmlData {
            devs: None,
            keywords,
            langs,
            keywords_str: None,
            template_name: "keyword.html".to_owned(),
            ttl: 3600,
            http_resp_code: 404,
            meta_robots: Some("noindex".to_owned()),
            ..html_data
        });
    }

    // get the data from ES
    let devs = elastic::matching_devs(
        &config.es_url,
        &config.dev_idx,
        keywords.clone(),
        langs.clone(),
        timezone_offset,
        timezone_hours,
        &config.no_sql_string_invalidation_regex,
    )
    .await?;

    // pre-build search terms as a string for simplified presentation
    // it should present them all as a list, but for now it uses a simple string
    // languages come first
    let mut combined_search_terms = langs.iter().map(|(l,_)|l.clone()).collect::<Vec<String>>();
    for kw in &keywords {
        combined_search_terms.push(kw.clone());
    }
    let combined_search_terms = combined_search_terms.join(" + ");

    // any page with more than one language or more than one keyword should not be indexed
    // 1 lang + 1 kw is OK to index
    let meta_robots = if langs.len() > 1 || keywords.len() > 1 {
        Some("noindex".to_owned())
    } else {
        None
    };

    // put everything together for Tera
    let html_data = HtmlData {
        devs: Some(devs),
        keywords,
        langs,
        keywords_str: Some(combined_search_terms),
        template_name: "keyword.html".to_owned(),
        ttl: 600,
        http_resp_code: 200,
        meta_robots,
        ..html_data
    };

    Ok(html_data)
}
