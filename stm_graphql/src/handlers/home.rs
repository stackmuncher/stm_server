use crate::config::Config;
use crate::elastic;
use stm_shared::elastic as elastic_shared;
use tracing::info;
use serde_json::Value;

/// Returns the default home page
pub(crate) async fn language_stats(config: &Config) -> Result<String, ()> {
    info!("Generating html-home");

    // get number of devs per technology
    // let stack_stats =
    //     elastic_shared::search(&config.es_url, &config.dev_idx, Some(elastic::SEARCH_ALL_LANGUAGES)).await?;



    //Ok(stack_stats)
    Ok(String::new())
}
