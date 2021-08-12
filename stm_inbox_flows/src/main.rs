use tracing::info;

mod config;
mod flows;
mod jobs;
mod utils;
mod dev_profile;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    // load the main config
    let config = config::Config::new().await;

    tracing_subscriber::fmt()
        .with_max_level(config.log_level.clone())
        .with_ansi(false)
        //.without_time()
        .init();

    info!("StackMuncher-GH started: {:?}", config.flow);

    match config.flow {
        config::Flow::DevQueue => {
            flows::dev_queue::merge_devs_reports(config).await;
        }

        config::Flow::Help => {
            flows::help::print_help_msg();
        }
    }

    Ok(())
}
