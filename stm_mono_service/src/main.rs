use tracing::info;

mod config;
mod flows;
mod db;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
        // load the main config
    let config = config::Config::new().await;

    tracing_subscriber::fmt()
        .with_max_level(config.log_level.clone())
        .with_ansi(false)
        //.without_time()
        .init();

    info!("StackMuncher Mono-Service started: {:?}", config.flow);

    match config.flow {
 
        config::Flow::WwwLogReader => {
            flows::www_log_reader::read_www_logs(config).await;
        }

    }

    Ok(())
}
