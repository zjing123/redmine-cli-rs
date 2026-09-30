use clap::{Parser, Subcommand};
use thiserror::Error;

use crate::config_cli;
use crate::output;
use crate::resources::{attachment, issue, project, time_entry, user, wiki_page};

#[derive(Debug, Error)]
pub enum CliError {
    #[error("{0}")]
    Resource(String),
    #[error("unsupported command")]
    Unsupported,
}

#[derive(Debug, Parser)]
#[command(name = "redmine-cli", version, about = "Redmine command-line client")]
pub struct Cli {
    #[arg(
        short,
        long,
        global = true,
        env = "REDMINE_PROFILE",
        help = "Saved Redmine profile to use"
    )]
    pub profile: Option<String>,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    #[command(about = "Manage saved Redmine connection profiles")]
    Config(config_cli::ConfigCommand),
    #[command(about = "Manage Redmine attachments")]
    Attachment(attachment::AttachmentCommand),
    #[command(about = "Manage Redmine issues")]
    Issue(issue::cli::IssueCommand),
    #[command(about = "Manage Redmine projects")]
    Project(project::ProjectCommand),
    #[command(about = "Manage Redmine users")]
    User(user::UserCommand),
    #[command(about = "Manage Redmine time entries")]
    TimeEntry(time_entry::TimeEntryCommand),
    #[command(about = "Manage project wiki pages")]
    WikiPage(wiki_page::WikiPageCommand),
    #[command(about = "Search Redmine resources and issues")]
    Search {
        #[arg(help = "Text to search for")]
        query: String,
        #[arg(
            long,
            default_value_t = 50,
            help = "Maximum number of results to return"
        )]
        limit: u64,
        #[arg(long, default_value_t = 0, help = "Number of results to skip")]
        offset: u64,
    },
}

pub fn run(cli: Cli) -> Result<(), CliError> {
    match cli.command {
        Command::Config(command) => {
            config_cli::run(command).map_err(|error| CliError::Resource(error.to_string()))
        }
        Command::Attachment(command) => attachment::run(command, cli.profile.as_deref())
            .map_err(|error| CliError::Resource(error.to_string())),
        Command::Search {
            query,
            limit,
            offset,
        } => {
            let config = crate::config::Config::load(None)
                .map_err(|error| CliError::Resource(error.to_string()))?;
            let (_, profile) = config
                .profile(cli.profile.as_deref())
                .map_err(|error| CliError::Resource(error.to_string()))?;
            let client = crate::redmine_client::RedmineClient::from_profile(&profile)
                .map_err(|error| CliError::Resource(error.to_string()))?;
            let result = client
                .search(query, limit, offset)
                .map_err(|error| CliError::Resource(error.to_string()))?;
            output::success(result);
            Ok(())
        }
        Command::Issue(issue) => issue::cli::run(issue, cli.profile.as_deref())
            .map_err(|error| CliError::Resource(error.to_string())),
        Command::Project(command) => project::run(command, cli.profile.as_deref())
            .map_err(|error| CliError::Resource(error.to_string())),
        Command::User(command) => user::run(command, cli.profile.as_deref())
            .map_err(|error| CliError::Resource(error.to_string())),
        Command::TimeEntry(command) => time_entry::run(command, cli.profile.as_deref())
            .map_err(|error| CliError::Resource(error.to_string())),
        Command::WikiPage(command) => wiki_page::run(command, cli.profile.as_deref())
            .map_err(|error| CliError::Resource(error.to_string())),
    }
}
