use super::html_data::HtmlData;
use crate::config::Config;
use crate::elastic;
use tracing::info;

/// Returns the developer profile. Expects a valid login
pub(crate) async fn html(
    config: &Config,
    login: String,
    html_data: HtmlData,
) -> Result<HtmlData, ()> {
    info!("Generating html-dev");
    let query = elastic::add_param(
        elastic::SEARCH_ENGINEER_BY_LOGIN,
        login.clone(),
        &config.no_sql_string_invalidation_regex,
    );

    let html_data = HtmlData {
        devs: Some(elastic::search(&config.es_url, &config.dev_idx, Some(query.as_str())).await?),
        template_name: "dev.html".to_owned(),
        ttl: 3600,
        http_resp_code: 200,
        login_str: Some(login),
        ..html_data
    };

    Ok(html_data)
}
