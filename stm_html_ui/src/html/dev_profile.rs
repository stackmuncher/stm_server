use super::html_data::HtmlData;
use crate::config::Config;
use crate::elastic;
use tracing::info;

/// Returns the developer profile. Expects a valid pub key of the dev
pub(crate) async fn html(config: &Config, owner_id: String, html_data: HtmlData) -> Result<HtmlData, ()> {
    info!("Generating html-dev");

    let query =
        elastic::add_param(elastic::SEARCH_DEV_BY_DOC_ID, owner_id.clone(), &config.no_sql_string_invalidation_regex);

    let devs = elastic::search(&config.es_url, &config.dev_idx, Some(&query)).await?;

    // default response code
    let mut http_resp_code = 404_u32;

    // check if the dev profile was found to return 200
    if let Some(v) = devs.get("hits") {
        if let Some(v) = v.get("hits") {
            if v.is_array() && v.get(0).is_some() {
                // content found - return 200
                http_resp_code = 200;
            }
        }
    }

    let html_data = HtmlData {
        devs: Some(devs),
        template_name: "dev.html".to_owned(),
        ttl: 3600,
        http_resp_code,
        owner_id_str: Some(owner_id),
        meta_robots: Some("noindex".to_owned()),
        ..html_data
    };

    Ok(html_data)
}
