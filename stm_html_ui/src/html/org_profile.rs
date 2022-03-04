use super::html_data::HtmlData;
use crate::config::Config;
use crate::elastic;
use serde_json::Value;
use stm_shared::elastic as elastic_shared;
use tracing::{info, warn};

/// Returns the org profile. Expects a string in format of "_org/stackmuncher".
/// Leading and trailing slashes removed.
pub(crate) async fn html(config: &Config, url_path: String, html_data: HtmlData) -> Result<HtmlData, ()> {
    info!("Generating org profile");

    // remove _org/ prefix
    let login = url_path.trim_start_matches("org/").trim().to_string();

    // is it a valid format for an org login?
    if config.no_sql_string_invalidation_regex.is_match(&login) {
        warn!("Invalid org login: {}", url_path);
        return Ok(html_data);
    }

    let query =
        elastic::add_param(elastic::SEARCH_ENGINEER_BY_LOGIN, login.clone(), &config.no_sql_string_invalidation_regex);

    let devs = elastic_shared::search::<Value>(&config.es_url, &config.org_idx, Some(query.as_str())).await?;

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
        template_name: "org_profile.html".to_owned(),
        ttl: 3600,
        http_resp_code,
        login_str: Some(login),
        ..html_data
    };

    Ok(html_data)
}
