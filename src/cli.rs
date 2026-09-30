use clap::{Parser, Subcommand};
use thiserror::Error;

use crate::resources::{issue, project, time_entry, user, wiki_page};

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
    #[arg(short, long, global = true, env = "REDMINE_PROFILE")]
    pub profile: Option<String>,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Issue(issue::cli::IssueCommand),
    Project(project::ProjectCommand),
    User(user::UserCommand),
    TimeEntry(time_entry::TimeEntryCommand),
    WikiPage(wiki_page::WikiPageCommand),
    Search { query: String },
}

pub fn run(cli: Cli) -> Result<(), CliError> {
    match cli.command {
        Command::Search { .. } => Err(CliError::Unsupported),
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
