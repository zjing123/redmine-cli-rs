use clap::{Args, Subcommand};
use serde_json::json;
use thiserror::Error;

use crate::{config::Config, output};

#[derive(Debug, Error)]
pub enum ConfigCliError {
    #[error(transparent)]
    Config(#[from] crate::config::ConfigError),
}

#[derive(Debug, Args)]
#[command(
    about = "Inspect local Redmine CLI configuration",
    after_help = "Examples:\n  # Print the configuration file path\n  redmine-cli config path"
)]
pub struct ConfigCommand {
    #[command(subcommand)]
    pub command: ConfigSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum ConfigSubcommand {
    #[command(
        about = "Print the configuration file path",
        after_help = "Examples:\n  # Print the configuration file path\n  redmine-cli config path"
    )]
    Path,
}

pub fn run(command: ConfigCommand) -> Result<(), ConfigCliError> {
    match command.command {
        ConfigSubcommand::Path => {
            let config = Config::load_or_default(None)?;
            output::success(json!({"path": config.path()}));
        }
    }
    Ok(())
}
