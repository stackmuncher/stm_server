use super::html_data::HtmlData;
use crate::config::Config;
use crate::elastic;
use tracing::info;

/// Returns the default home page
pub(crate) async fn html(config: &Config, html_data: HtmlData) -> Result<HtmlData, ()> {
    info!("Generating html-home");

    // get number of devs per technology
    let stack_stats = elastic::search(&config.es_url, &config.dev_idx, Some(elastic::SEARCH_ALL_LANGUAGES)).await?;

    // combine everything together for Tera
    let html_data = HtmlData {
        stack_stats: Some(stack_stats),
        template_name: "home.html".to_owned(),
        ttl: 600,
        http_resp_code: 200,
        ..html_data
    };

    Ok(html_data)
}
