use crate::config::Config;
use tracing::info;

/// Output a help message
pub(crate) fn print_help_msg() {
    info!(
        "Required param: -flow with one of {}",
        Config::CLI_MODES.join(", ")
    );
    info!("Optional param: -l for logging with one of [trace, debug, info, error]. Defaults to [info].");
    info!(
        "Requires config.json in the same folder as the app. See config-schema.json for details."
    );
}
