use super::html_data::HtmlData;
use crate::config::Config;
use crate::elastic;
use futures::future::join_all;
use serde::Serialize;
use serde_json::Value;

#[derive(Serialize)]
pub(crate) struct Stats {
    stm_stats_dev_job_counts: Value,
    stm_stats_repo_job_counts: Value,
    stm_stats_report_success_counts: Value,
    stm_stats_report_generation_time_avg: Value,
    stm_stats_report_fail_counts: Value,
}

pub(crate) async fn html(config: &Config, html_data: HtmlData) -> Result<HtmlData, ()> {
    // get the data from ES
    let stm_stats_dev_job_counts =
        elastic::get_stm_stats(&config.es_url, "stm_stats_dev_job_counts", 60);
    let stm_stats_repo_job_counts =
        elastic::get_stm_stats(&config.es_url, "stm_stats_repo_job_counts", 60);
    let stm_stats_report_success_counts =
        elastic::get_stm_stats(&config.es_url, "stm_stats_report_success_counts", 12);
    let stm_stats_report_generation_time_avg =
        elastic::get_stm_stats(&config.es_url, "stm_stats_report_generation_time_avg", 12);
    let stm_stats_report_fail_counts =
        elastic::get_stm_stats(&config.es_url, "stm_stats_report_fail_counts", 12);

    // run the queries concurrently
    let jobs = vec![
        stm_stats_dev_job_counts,
        stm_stats_repo_job_counts,
        stm_stats_report_success_counts,
        stm_stats_report_generation_time_avg,
        stm_stats_report_fail_counts,
    ];
    let mut response = join_all(jobs).await;
    response.reverse();

    // put everything together into a structure
    // expect has to be repeated twice because it is Option<Result<Value>> after pop()
    let stats_jobs = Stats {
        stm_stats_dev_job_counts: response
            .pop()
            .expect("stm_stats_dev_job_counts ES query failed")
            .expect("stm_stats_dev_job_counts ES query failed"),
        stm_stats_repo_job_counts: response
            .pop()
            .expect("stm_stats_repo_job_counts ES query failed")
            .expect("stm_stats_repo_job_counts ES query failed"),
        stm_stats_report_success_counts: response
            .pop()
            .expect("stm_stats_report_success_counts ES query failed")
            .expect("stm_stats_report_success_counts ES query failed"),
        stm_stats_report_generation_time_avg: response
            .pop()
            .expect("stm_stats_report_generation_time_avg ES query failed")
            .expect("stm_stats_report_generation_time_avg ES query failed"),
        stm_stats_report_fail_counts: response
            .pop()
            .expect("stm_stats_report_fail_counts ES query failed")
            .expect("stm_stats_report_fail_counts ES query failed"),
    };

    // put everything together for Tera
    let html_data = HtmlData {
        stats_jobs: Some(stats_jobs),
        template_name: "stats.html".to_owned(),
        ttl: 600,
        http_resp_code: 200,
        ..html_data
    };

    Ok(html_data)
}
