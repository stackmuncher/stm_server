use super::html_data::HtmlData;
use crate::config::Config;
use crate::elastic;
use stm_shared::elastic as elastic_shared;
use tracing::info;

/// Returns the default home page
pub(crate) async fn html(config: &Config, html_data: HtmlData) -> Result<HtmlData, ()> {
    info!("Generating orgs-home");

    // get number of devs per technology
    let stack_stats =
        elastic_shared::search(&config.es_url, &config.org_idx, Some(elastic::SEARCH_VERIFIED_ORGS_PER_LANGUAGE))
            .await?;

    // combine everything together for Tera
    let html_data = HtmlData {
        stack_stats: Some(stack_stats),
        template_name: "orgs.html".to_owned(),
        ttl: 600,
        http_resp_code: 200,
        ..html_data
    };

    Ok(html_data)
}
