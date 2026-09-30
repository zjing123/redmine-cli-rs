use std::io::IsTerminal;

use clap::{Args, Subcommand};
use serde_json::json;
use thiserror::Error;

use crate::{
    config::{Config, ConfigError, Profile},
    output,
};

#[derive(Debug, Error)]
pub enum ConfigCliError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Client(#[from] crate::redmine_client::RedmineClientError),
    #[error("profile '{0}' already exists")]
    ProfileExists(String),
    #[error("profile '{0}' was not found")]
    ProfileMissing(String),
    #[error("an API key is required")]
    MissingCredentials,
    #[error("remove requires --yes")]
    ConfirmationRequired,
    #[error("a secret is required when standard input is not interactive")]
    NonInteractiveSecret,
    #[error("failed to read secret: {0}")]
    Secret(#[from] std::io::Error),
    #[error("profile name cannot be empty")]
    EmptyProfileName,
}

#[derive(Debug, Args)]
#[command(
    about = "Manage saved Redmine connection profiles",
    after_help = "Examples:\n  redmine-cli config set redminex --url https://redmine.example.com/\n  redmine-cli config test redminex\n  redmine-cli config list"
)]
pub struct ConfigCommand {
    #[command(subcommand)]
    pub command: ConfigSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum ConfigSubcommand {
    #[command(
        about = "List all saved profiles without exposing API keys",
        after_help = "Example:\n  redmine-cli config list"
    )]
    List,
    #[command(
        about = "Show one saved profile with its API key redacted",
        after_help = "Example:\n  redmine-cli config get redminex"
    )]
    Get {
        #[arg(help = "Profile name")]
        profile: String,
    },
    #[command(
        about = "Print the configuration file path",
        after_help = "Example:\n  redmine-cli config path"
    )]
    Path,
    #[command(
        about = "Validate the configuration file and all profiles",
        after_help = "Example:\n  redmine-cli config validate"
    )]
    Validate,
    #[command(
        about = "Create a new profile configuration",
        after_help = "Examples:\n  redmine-cli config set redminex --url https://redmine.example.com/\n  redmine-cli config set redminex --url https://redmine.example.com/ --api-key <API_KEY>"
    )]
    Set(ProfileArgs),
    #[command(
        about = "Update an existing profile configuration",
        after_help = "Example:\n  redmine-cli config update redminex --url https://redmine.example.com/"
    )]
    Update(ProfileArgs),
    #[command(
        about = "Test the profile against Redmine and show the authenticated user",
        after_help = "Example:\n  redmine-cli config test redminex"
    )]
    Test {
        #[arg(help = "Profile name")]
        profile: String,
    },
    #[command(
        about = "Remove an entire profile and its configuration",
        after_help = "Example:\n  redmine-cli config remove redminex --yes"
    )]
    Remove {
        #[arg(help = "Profile name")]
        profile: String,
        #[arg(long, help = "Confirm the removal")]
        yes: bool,
    },
}

#[derive(Debug, Args)]
pub struct ProfileArgs {
    #[arg(help = "Profile name")]
    pub profile: String,
    #[arg(long, help = "Redmine server URL")]
    pub url: Option<String>,
    #[arg(
        long,
        help = "Redmine API key; prompts securely when omitted for config set"
    )]
    pub api_key: Option<String>,
}

pub fn run(command: ConfigCommand) -> Result<(), ConfigCliError> {
    match command.command {
        ConfigSubcommand::List => {
            let config = Config::load(None)?;
            let profiles = config
                .profiles()
                .map(|(name, profile)| redacted_profile(name, profile))
                .collect::<Vec<_>>();
            output::success(json!(profiles));
        }
        ConfigSubcommand::Get { profile } => {
            let config = Config::load(None)?;
            let profile_data = config
                .profiles()
                .find(|(name, _)| name.as_str() == profile)
                .map(|(name, profile)| redacted_profile(name, profile))
                .ok_or(ConfigCliError::ProfileMissing(profile))?;
            output::success(profile_data);
        }
        ConfigSubcommand::Path => {
            let config = Config::load_or_default(None)?;
            output::success(json!({"path": config.path()}));
        }
        ConfigSubcommand::Validate => {
            let config = Config::load(None)?;
            config.validate()?;
            output::success(json!({"valid": true, "profiles": config.profile_names().count()}));
        }
        ConfigSubcommand::Set(args) => set_profile(args, false)?,
        ConfigSubcommand::Update(args) => set_profile(args, true)?,
        ConfigSubcommand::Test { profile } => {
            let config = Config::load(None)?;
            let (_, profile_data) = config.profile(Some(&profile))?;
            let client = crate::redmine_client::RedmineClient::from_profile(&profile_data)?;
            output::success(client.current_user()?);
        }
        ConfigSubcommand::Remove { profile, yes } => {
            if !yes {
                return Err(ConfigCliError::ConfirmationRequired);
            }
            let mut config = Config::load(None)?;
            config
                .remove_profile(&profile)
                .ok_or(ConfigCliError::ProfileMissing(profile.clone()))?;
            config.save()?;
            output::success(json!({"removed": profile}));
        }
    }
    Ok(())
}

fn set_profile(args: ProfileArgs, update: bool) -> Result<(), ConfigCliError> {
    if args.profile.is_empty() {
        return Err(ConfigCliError::EmptyProfileName);
    }
    let mut config = Config::load_or_default(None)?;
    let existing = config
        .profiles()
        .find(|(name, _)| name.as_str() == args.profile);
    if !update && existing.is_some() {
        return Err(ConfigCliError::ProfileExists(args.profile));
    }
    if update && existing.is_none() {
        return Err(ConfigCliError::ProfileMissing(args.profile));
    }

    let mut profile = existing
        .map(|(_, profile)| profile.clone())
        .unwrap_or(Profile {
            url: String::new(),
            api_key: None,
            _version: None,
        });
    if let Some(url) = args.url {
        profile.url = url;
    }
    if let Some(api_key) = args.api_key {
        profile.api_key = Some(api_key);
    } else if !update && profile.api_key.is_none() {
        profile.api_key = Some(read_secret("API Key: ")?);
    }
    if profile.url.is_empty() {
        return Err(ConfigError::MissingUrl(args.profile).into());
    }
    if profile.api_key.as_deref().is_none_or(str::is_empty) {
        return Err(ConfigCliError::MissingCredentials);
    }
    config.insert_profile(args.profile.clone(), profile);
    config.save()?;
    output::success(json!({"profile": args.profile, "saved": true}));
    Ok(())
}

fn read_secret(prompt: &str) -> Result<String, ConfigCliError> {
    if !std::io::stdin().is_terminal() {
        return Err(ConfigCliError::NonInteractiveSecret);
    }
    Ok(rpassword::prompt_password(prompt)?)
}

fn redacted_profile(name: &str, profile: &Profile) -> serde_json::Value {
    let authentication = if profile
        .api_key
        .as_deref()
        .is_some_and(|value| !value.is_empty())
    {
        "api_key"
    } else {
        "none"
    };
    json!({
        "name": name,
        "url": profile.url,
        "authentication": authentication,
    })
}
