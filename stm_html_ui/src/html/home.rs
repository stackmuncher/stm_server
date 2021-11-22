use super::html_data::{HtmlData, RelatedKeywords};
use crate::config::Config;
use crate::elastic;
use serde::Deserialize;
use stm_shared::elastic as elastic_shared;
use tracing::info;

#[derive(Deserialize, Debug)]
struct EngListResp {
    hits: EngListHits,
}

#[derive(Deserialize, Debug)]
struct EngListHits {
    hits: Vec<EngHit>,
}

#[derive(Deserialize, Debug)]
struct EngHit {
    #[serde(rename(deserialize = "_source"))]
    source: Option<EngSource>,
}

#[derive(Deserialize, Debug)]
struct EngSource {
    report: Option<Report>,
}

#[derive(Deserialize, Debug)]
struct Report {
    tech: Option<Vec<Tech>>,
}

#[derive(Deserialize, Debug)]
struct Tech {
    refs_kw: Option<Vec<RelatedKeywords>>,
    pkgs_kw: Option<Vec<RelatedKeywords>>,
}

/// Returns the default home page
pub(crate) async fn html(config: &Config, html_data: HtmlData) -> Result<HtmlData, ()> {
    info!("Generating html-home");

    // a query to grab a bunch of latest additions and updates to dev idx
    let stack_stats = elastic::search(&config.es_url, &config.dev_idx, Some(elastic::SEARCH_ALL_LANGUAGES));
    // a query to get latest stats
    // returns Stats struct wrapped in _source
    let stats = elastic_shared::get_doc_by_id(
        &config.es_url,
        &config.stats_idx,
        "latest_stats.json",
        &config.no_sql_string_invalidation_regex,
    );

    // get all the data the page needs from ES in one go with async requests
    let (stack_stats, stats) = futures::future::join(stack_stats, stats).await;
    let stack_stats = stack_stats?;
    let stats = stats?;

    // combine everything together for Tera
    let html_data = HtmlData {
        stack_stats: Some(stack_stats),
        stats: Some(stats),
        template_name: "home.html".to_owned(),
        ttl: 600,
        http_resp_code: 200,
        ..html_data
    };

    Ok(html_data)
}
