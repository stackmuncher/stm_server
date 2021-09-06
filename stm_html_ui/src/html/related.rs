use super::html_data::{HtmlData, RelatedKeywords};
use crate::config::Config;
use crate::elastic;
use tracing::{error, warn};

pub(crate) async fn html(
    config: &Config,
    keyword: String,
    html_data: HtmlData,
) -> Result<HtmlData, ()> {
    // is it a valid format for related keywords search?
    let keyword = keyword.trim().to_string();

    let html_data = HtmlData {
        related: Some(Vec::new()),
        keywords_str: Some(keyword.clone()),
        template_name: "related.html".to_owned(),
        ttl: 3600,
        http_resp_code: 200,
        meta_robots: Some("noindex".to_owned()),
        ..html_data
    };

    // check if the search term has any invalid chars - the string must be safe to include into another regex
    // inside an ES query
    if config.no_sql_string_invalidation_regex.is_match(&keyword) {
        warn!("Invalid keyword: {}", keyword);
        return Ok(html_data);
    }

    // get the data from ES
    let related = match elastic::related_keywords(&config.es_url, &config.dev_idx, &keyword, &config.no_sql_string_invalidation_regex).await {
        Err(_) => {
            // the UI shouldn't send any invalid keywords through, but the user or the bot may still try to submit
            // all sorts of values for search. Those should result in a 404 page.
            error!("Keyword search failed for {}", keyword);
            return Ok(html_data);
        }
        Ok(v) => v,
    };

    let related = related
        .into_iter()
        .map(|(k, c)| RelatedKeywords { k, c })
        .collect::<Vec<RelatedKeywords>>();

    // put everything together for Tera
    let html_data = HtmlData {
        related: Some(related),
        ..html_data
    };

    Ok(html_data)
}
