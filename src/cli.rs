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
#[command(
    name = "redmine-cli",
    version,
    about = "Redmine command-line client",
    after_help = "Examples:\n  redmine-cli config list\n  redmine-cli issue get 123\n  redmine-cli attachment download 391795\n  redmine-cli search \"login error\""
)]
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
    #[command(
        about = "Manage saved Redmine connection profiles",
        after_help = "Example:\n  redmine-cli config set redminex --url https://redmine.example.com/"
    )]
    Config(config_cli::ConfigCommand),
    #[command(
        about = "Manage Redmine attachments",
        after_help = "Example:\n  redmine-cli attachment download 391795"
    )]
    Attachment(attachment::AttachmentCommand),
    #[command(
        about = "Manage Redmine issues",
        after_help = "Example:\n  redmine-cli issue get https://redmine.example.com/issues/123 --include attachments"
    )]
    Issue(issue::cli::IssueCommand),
    #[command(
        about = "Manage Redmine projects",
        after_help = "Example:\n  redmine-cli project get 1 --fields id,name,identifier"
    )]
    Project(project::ProjectCommand),
    #[command(
        about = "Manage Redmine users",
        after_help = "Example:\n  redmine-cli user list --limit 20"
    )]
    User(user::UserCommand),
    #[command(
        about = "Manage Redmine time entries",
        after_help = "Example:\n  redmine-cli time-entry list --project-id 1"
    )]
    TimeEntry(time_entry::TimeEntryCommand),
    #[command(
        about = "Manage project wiki pages",
        after_help = "Example:\n  redmine-cli wiki-page get 1"
    )]
    WikiPage(wiki_page::WikiPageCommand),
    #[command(
        about = "Search Redmine resources and issues",
        after_help = "Examples:\n  redmine-cli search \"login error\"\n  redmine-cli search \"customer\" --limit 20 --offset 20"
    )]
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
