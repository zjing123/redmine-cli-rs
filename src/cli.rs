use clap::{Arg, CommandFactory, FromArgMatches, Parser, Subcommand};
use thiserror::Error;

use crate::auth_cli;
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
    version = concat!(env!("CARGO_PKG_VERSION"), " (Rust)"),
    about = "Redmine command-line client",
    next_line_help = false,
    term_width = 120,
    after_help = "This is a concise overview of the available commands.\nUse `redmine-cli <command> --help` for detailed options and examples."
)]
pub struct Cli {
    #[arg(
        short,
        long,
        global = true,
        env = "REDMINE_PROFILE",
        help = "Profile for API requests; defaults to REDMINE_PROFILE or active_profile. Full issue URLs can match a configured profile automatically"
    )]
    pub profile: Option<String>,
    #[command(subcommand)]
    pub command: Command,
}

impl Cli {
    pub fn parse_with_global_options_heading() -> Self {
        let mut command = Self::command();
        let profile = command
            .get_arguments()
            .find(|argument| argument.get_id().as_str() == "profile")
            .cloned();
        if let Some(profile) = profile {
            command = command
                .mut_subcommands(|subcommand| add_global_options_heading(subcommand, &profile));
        }
        let matches = command.get_matches();
        Self::from_arg_matches(&matches).unwrap_or_else(|error| error.exit())
    }
}

fn add_global_options_heading(mut command: clap::Command, profile: &Arg) -> clap::Command {
    command = command.arg(profile.clone().help_heading("Global Options"));
    command.mut_subcommands(|subcommand| add_global_options_heading(subcommand, profile))
}

#[derive(Debug, Subcommand)]
pub enum Command {
    #[command(
        about = "Inspect local CLI configuration",
        after_help = "Examples:\n  # Print the configuration file path\n  redmine-cli config path"
    )]
    Config(config_cli::ConfigCommand),
    #[command(
        about = "Manage authentication profiles",
        after_help = "Examples:\n  # Log in to a Redmine server\n  redmine-cli auth login\n\n  # List configured profiles\n  redmine-cli auth list\n\n  # Switch the active profile\n  redmine-cli auth switch work"
    )]
    Auth(auth_cli::AuthCommand),
    #[command(
        name = "attachments",
        alias = "attachment",
        about = "Manage Redmine attachments",
        after_help = "Examples:\n  # Download an attachment\n  redmine-cli attachments download 391795"
    )]
    Attachment(attachment::AttachmentCommand),
    #[command(
        name = "issues",
        alias = "issue",
        about = "Manage Redmine issues",
        after_help = "ID-based requests, list, create, update, and delete use -p/--profile, REDMINE_PROFILE, or active_profile. A full issue URL can auto-detect the profile.\nExamples:\n  # Fetch an issue by ID\n  redmine-cli -p prod issues get 123\n\n  # Fetch an issue by URL and auto-detect its profile\n  redmine-cli issues get https://prod.example.com/issues/123\n\n  # Include attachments and journals\n  redmine-cli -p prod issues get 123 --include attachments,journals\n\n  # List open issues\n  redmine-cli -p prod issues list --status open --limit 20\n\n  # Return selected fields\n  redmine-cli -p prod issues list --project-id 1 --fields id,subject,status\n\n  # Create an issue\n  redmine-cli -p prod issues create --project-id 1 --subject \"New issue\"\n\n  # Update an issue\n  redmine-cli -p prod issues update 123 --json '{\"priority_id\":3}'\n\n  # Show issue fields or schema\n  redmine-cli issues fields\n  redmine-cli issues schema"
    )]
    Issue(issue::cli::IssueCommand),
    #[command(
        name = "projects",
        alias = "project",
        about = "Manage Redmine projects",
        after_help = "Examples:\n  # Get a project and return selected fields\n  redmine-cli projects get 1 --fields id,name,identifier"
    )]
    Project(project::ProjectCommand),
    #[command(
        name = "users",
        alias = "user",
        about = "Manage Redmine users",
        after_help = "Examples:\n  # List users\n  redmine-cli users list --limit 20"
    )]
    User(user::UserCommand),
    #[command(
        name = "time",
        about = "Manage Redmine time entries",
        after_help = "Examples:\n  # List time entries for a project\n  redmine-cli time list --project-id 1"
    )]
    TimeEntry(time_entry::TimeEntryCommand),
    #[command(
        name = "wiki",
        about = "Manage project wiki pages",
        after_help = "Examples:\n  # Get a wiki page\n  redmine-cli wiki get 1"
    )]
    WikiPage(wiki_page::WikiPageCommand),
    #[command(
        about = "Search Redmine resources and issues",
        after_help = "Uses -p/--profile, REDMINE_PROFILE, or active_profile.\nExamples:\n  # Search Redmine for a phrase\n  redmine-cli -p prod search \"login error\"\n\n  # Fetch the next page of matching results\n  redmine-cli -p prod search \"customer\" --limit 20 --offset 20"
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
        Command::Auth(command) => auth_cli::run(command, cli.profile.as_deref())
            .map_err(|error| CliError::Resource(error.to_string())),
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
