use std::io::{self, IsTerminal, Write};

use clap::{Args, Subcommand};
use serde_json::json;
use thiserror::Error;

use crate::{
    config::{Config, ConfigError, Profile},
    output,
    redmine_client::{RedmineClient, RedmineClientError},
};

#[derive(Debug, Error)]
pub enum AuthError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Client(#[from] RedmineClientError),
    #[error("server URL is required")]
    MissingServer,
    #[error("API key is required")]
    MissingApiKey,
    #[error("profile name cannot be empty")]
    EmptyProfileName,
    #[error("interactive input is unavailable; provide the required flags")]
    NonInteractive,
    #[error("failed to read input: {0}")]
    Input(#[from] io::Error),
    #[error("profile removal requires confirmation in an interactive terminal")]
    ConfirmationRequired,
}

#[derive(Debug, Args)]
#[command(
    about = "Manage Redmine authentication profiles",
    after_help = "Examples:\n  # Add or update a profile without prompts\n  redmine-cli auth login --name work -k <API_KEY> -s https://redmine.example.com/\n\n  # Start interactive login; the profile name defaults from the server hostname\n  redmine-cli auth login\n\n  # List configured profiles\n  redmine-cli auth list"
)]
pub struct AuthCommand {
    #[command(subcommand)]
    pub command: AuthSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum AuthSubcommand {
    #[command(
        about = "List configured authentication profiles",
        after_help = "Examples:\n  # List profiles and their server URLs\n  redmine-cli auth list"
    )]
    List,
    #[command(
        about = "Log in to a Redmine server and save a profile",
        after_help = "Examples:\n  # Create or update a profile without prompts\n  redmine-cli auth login --name work -k <API_KEY> -s https://redmine.example.com/\n\n  # Enter missing values interactively; the name defaults from the server hostname\n  redmine-cli auth login"
    )]
    Login {
        #[arg(
            long,
            value_name = "NAME",
            help = "Profile name; defaults to the server hostname"
        )]
        name: Option<String>,
        #[arg(short = 'k', long, value_name = "API_KEY", help = "Redmine API key")]
        api_key: Option<String>,
        #[arg(short = 's', long, value_name = "SERVER", help = "Redmine server URL")]
        server: Option<String>,
    },
    #[command(
        about = "Remove a profile",
        after_help = "Examples:\n  # Remove a named profile after confirmation\n  redmine-cli auth logout work\n\n  # Remove the active profile after confirmation\n  redmine-cli auth logout"
    )]
    Logout {
        #[arg(
            id = "profile-name",
            value_name = "PROFILE",
            help = "Profile to remove; defaults to the active profile"
        )]
        profile: Option<String>,
    },
    #[command(
        about = "Show the active profile and authenticated user",
        after_help = "Examples:\n  # Show the active profile and current Redmine user\n  redmine-cli auth status\n\n  # Check a specific profile\n  redmine-cli -p work auth status"
    )]
    Status,
    #[command(
        about = "Switch the active profile",
        after_help = "Examples:\n  # Set a profile as active\n  redmine-cli auth switch work\n\n  # Choose a profile interactively\n  redmine-cli auth switch"
    )]
    Switch {
        #[arg(
            id = "profile-name",
            value_name = "PROFILE",
            help = "Profile to activate"
        )]
        profile: Option<String>,
    },
}

pub fn run(command: AuthCommand, selected_profile: Option<&str>) -> Result<(), AuthError> {
    match command.command {
        AuthSubcommand::List => list_profiles()?,
        AuthSubcommand::Login {
            name,
            api_key,
            server,
        } => login(name, api_key, server)?,
        AuthSubcommand::Logout { profile } => logout(profile.as_deref(), selected_profile)?,
        AuthSubcommand::Status => show_status(selected_profile)?,
        AuthSubcommand::Switch { profile } => switch_profile(profile)?,
    }
    Ok(())
}

fn config() -> Result<Config, AuthError> {
    Ok(Config::load_or_default(None)?)
}

fn list_profiles() -> Result<(), AuthError> {
    let config = config()?;
    let profiles = config
        .profiles()
        .map(|(name, profile)| {
            json!({
                "name": name,
                "server": &profile.url,
                "active": config.active_profile() == Some(name.as_str()),
                "read_only": config.is_read_only_for(name),
            })
        })
        .collect::<Vec<_>>();
    output::success(json!({
        "active_profile": config.active_profile(),
        "profiles": profiles,
    }));
    Ok(())
}

fn login(
    name: Option<String>,
    api_key: Option<String>,
    server: Option<String>,
) -> Result<(), AuthError> {
    let interactive = server
        .as_deref()
        .is_none_or(|value| value.trim().is_empty())
        || api_key
            .as_deref()
            .is_none_or(|value| value.trim().is_empty());
    let server = match server.filter(|value| !value.trim().is_empty()) {
        Some(server) => server,
        None => prompt_line("Redmine server URL: ")?,
    };
    if server.trim().is_empty() {
        return Err(AuthError::MissingServer);
    }

    let default_name = profile_name_from_server(&server)?;
    let name = match name {
        Some(name) => name,
        None if interactive => prompt_profile_name(&default_name)?,
        None => default_name,
    };
    if name.trim().is_empty() {
        return Err(AuthError::EmptyProfileName);
    }

    let api_key = match api_key.filter(|value| !value.trim().is_empty()) {
        Some(api_key) => api_key,
        None => {
            if !io::stdin().is_terminal() {
                return Err(AuthError::NonInteractive);
            }
            rpassword::prompt_password("API key: ")?
        }
    };
    if api_key.trim().is_empty() {
        return Err(AuthError::MissingApiKey);
    }

    let mut config = config()?;
    let read_only = config
        .profiles()
        .find(|(existing_name, _)| existing_name.as_str() == name)
        .and_then(|(_, profile)| profile.read_only);
    let profile = Profile {
        url: server,
        api_key: Some(api_key),
        _version: None,
        read_only,
    };

    let client = RedmineClient::from_profile(&profile)?;
    let user = client.current_user()?;
    let server = profile.url.clone();
    config.insert_profile(name.clone(), profile);
    if config.active_profile().is_none() {
        config.set_active_profile(&name)?;
    }
    config.save()?;

    output::success(json!({
        "profile": name,
        "server": server,
        "user": user,
        "saved": true,
    }));
    Ok(())
}

fn show_status(profile: Option<&str>) -> Result<(), AuthError> {
    let config = config()?;
    let (name, profile) = config.profile(profile)?;
    let user = RedmineClient::from_profile(&profile)?.current_user()?;
    output::success(json!({
        "profile": name,
        "server": profile.url,
        "user": user,
    }));
    Ok(())
}

fn switch_profile(profile: Option<String>) -> Result<(), AuthError> {
    let mut config = config()?;
    let name = match profile {
        Some(name) => name,
        None => select_profile(&config)?,
    };
    config.set_active_profile(&name)?;
    config.save()?;
    output::success(json!({"active_profile": name}));
    Ok(())
}

fn logout(profile: Option<&str>, selected_profile: Option<&str>) -> Result<(), AuthError> {
    let mut config = config()?;
    let name = profile
        .or(selected_profile)
        .or_else(|| config.active_profile())
        .ok_or_else(|| ConfigError::ProfileNotFound("(no active profile)".to_owned()))?
        .to_owned();
    let removed = config
        .remove_profile(&name)
        .ok_or_else(|| ConfigError::ProfileNotFound(name.clone()))?;

    if !io::stdin().is_terminal() {
        return Err(AuthError::ConfirmationRequired);
    }
    print!("Remove profile {name:?} ({})? [y/N] ", removed.url);
    io::stdout().flush()?;
    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    if !answer.trim().eq_ignore_ascii_case("y") {
        output::success(json!({"removed": false, "profile": name}));
        return Ok(());
    }

    config.save()?;
    output::success(json!({"removed": true, "profile": name}));
    Ok(())
}

fn select_profile(config: &Config) -> Result<String, AuthError> {
    let profiles = config.profile_names().collect::<Vec<_>>();
    if profiles.is_empty() {
        return Err(ConfigError::ProfileNotFound("(no profiles configured)".to_owned()).into());
    }
    if !io::stdin().is_terminal() {
        return Err(AuthError::NonInteractive);
    }

    for (index, name) in profiles.iter().enumerate() {
        println!("{}. {}", index + 1, name);
    }
    let answer = prompt_line("Select profile number or name: ")?;
    if let Ok(index) = answer.parse::<usize>()
        && let Some(name) = profiles.get(index.saturating_sub(1))
    {
        return Ok((**name).clone());
    }
    if profiles.iter().any(|name| name.as_str() == answer) {
        return Ok(answer);
    }
    Err(ConfigError::ProfileNotFound(answer).into())
}

fn prompt_line(prompt: &str) -> Result<String, AuthError> {
    if !io::stdin().is_terminal() {
        return Err(AuthError::NonInteractive);
    }
    print!("{prompt}");
    io::stdout().flush()?;
    let mut value = String::new();
    io::stdin().read_line(&mut value)?;
    Ok(value.trim().to_owned())
}

fn prompt_profile_name(default: &str) -> Result<String, AuthError> {
    let name = prompt_line(&format!("Profile name [{default}]: "))?;
    Ok(if name.is_empty() {
        default.to_owned()
    } else {
        name
    })
}

fn profile_name_from_server(server: &str) -> Result<String, AuthError> {
    let url = url::Url::parse(server).map_err(|_| ConfigError::InvalidUrl(server.to_owned()))?;
    let host = url
        .host_str()
        .ok_or_else(|| ConfigError::InvalidUrl(server.to_owned()))?;
    let hostname = host.strip_prefix("www.").unwrap_or(host);
    Ok(hostname.replace('.', "-"))
}
