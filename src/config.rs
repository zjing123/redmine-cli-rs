use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;
use thiserror::Error;
use url::Url;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("configuration file not found: {0}")]
    Missing(PathBuf),
    #[error("failed to read configuration: {0}")]
    Read(#[from] std::io::Error),
    #[error("invalid configuration YAML: {0}")]
    Parse(#[from] serde_yaml::Error),
    #[error("profile '{0}' was not found")]
    ProfileNotFound(String),
    #[error("profile '{0}' has no url")]
    MissingUrl(String),
    #[error(
        "profile '{0}' has no api_key; username/password authentication is not supported by redmine-api yet"
    )]
    MissingApiKey(String),
    #[error("no profile matches URL '{0}'")]
    NoUrlMatch(String),
    #[error("invalid Redmine URL '{0}'")]
    InvalidUrl(String),
    #[error("invalid issue reference '{0}'")]
    InvalidIssueReference(String),
}

#[derive(Debug, Clone, Deserialize)]
pub struct Profile {
    pub url: String,
    #[serde(alias = "key")]
    pub api_key: Option<String>,
    #[serde(rename = "username")]
    pub _username: Option<String>,
    #[serde(rename = "password")]
    pub _password: Option<String>,
    #[serde(rename = "version")]
    pub _version: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawConfig {
    #[serde(default)]
    profiles: BTreeMap<String, Profile>,
}

#[derive(Debug)]
pub struct Config {
    raw: RawConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssueReference {
    pub profile: String,
    pub id: u64,
}

impl Config {
    pub fn load(path: Option<&Path>) -> Result<Self, ConfigError> {
        let path = path
            .map(PathBuf::from)
            .or_else(|| env::var_os("REDMINE_CONFIG").map(PathBuf::from))
            .unwrap_or_else(default_path);
        if !path.exists() {
            return Err(ConfigError::Missing(path));
        }
        let raw = serde_yaml::from_str(&fs::read_to_string(path)?)?;
        Ok(Self { raw })
    }

    pub fn profile(&self, name: Option<&str>) -> Result<(String, Profile), ConfigError> {
        let selected = name
            .map(str::to_owned)
            .or_else(|| env::var("REDMINE_PROFILE").ok())
            .ok_or_else(|| ConfigError::ProfileNotFound("(profile is required)".to_owned()))?;
        let profile = self
            .raw
            .profiles
            .get(&selected)
            .cloned()
            .ok_or_else(|| ConfigError::ProfileNotFound(selected.clone()))?;
        validate_profile(&selected, &profile)?;
        Ok((selected, profile))
    }

    pub fn match_issue_url(&self, value: &str) -> Result<IssueReference, ConfigError> {
        let input =
            Url::parse(value).map_err(|_| ConfigError::InvalidIssueReference(value.to_owned()))?;
        let segments: Vec<_> = input
            .path_segments()
            .ok_or_else(|| ConfigError::InvalidIssueReference(value.to_owned()))?
            .collect();
        let id = segments
            .windows(2)
            .find(|pair| pair[0] == "issues")
            .and_then(|pair| pair[1].parse::<u64>().ok())
            .ok_or_else(|| ConfigError::InvalidIssueReference(value.to_owned()))?;
        let mut best: Option<(String, usize)> = None;
        for (name, profile) in &self.raw.profiles {
            validate_profile(name, profile)?;
            let base = Url::parse(&profile.url)
                .map_err(|_| ConfigError::InvalidUrl(profile.url.clone()))?;
            if base.scheme() != input.scheme()
                || base.host_str() != input.host_str()
                || base.port_or_known_default() != input.port_or_known_default()
            {
                continue;
            }
            let base_path = base.path().trim_end_matches('/');
            let path_matches = base_path.is_empty()
                || input.path() == base_path
                || input.path().starts_with(&format!("{base_path}/"));
            if path_matches
                && best
                    .as_ref()
                    .is_none_or(|(_, length)| base_path.len() > *length)
            {
                best = Some((name.clone(), base_path.len()));
            }
        }
        best.map(|(profile, _)| IssueReference { profile, id })
            .ok_or_else(|| ConfigError::NoUrlMatch(value.to_owned()))
    }
}

fn default_path() -> PathBuf {
    env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".config/redmine-cli/config.yaml")
}

fn validate_profile(name: &str, profile: &Profile) -> Result<(), ConfigError> {
    if profile.url.is_empty() {
        return Err(ConfigError::MissingUrl(name.to_owned()));
    }
    if profile.api_key.as_deref().is_none_or(str::is_empty) {
        return Err(ConfigError::MissingApiKey(name.to_owned()));
    }
    Ok(())
}

pub fn parse_issue_reference(
    value: &str,
    profile: Option<&str>,
    config: &Config,
) -> Result<IssueReference, ConfigError> {
    if value.starts_with("http://") || value.starts_with("https://") {
        return config.match_issue_url(value);
    }
    let id = value
        .parse()
        .map_err(|_| ConfigError::InvalidIssueReference(value.to_owned()))?;
    let profile = profile
        .map(str::to_owned)
        .or_else(|| env::var("REDMINE_PROFILE").ok())
        .ok_or_else(|| {
            ConfigError::ProfileNotFound(
                "(profile is required for numeric issue references)".to_owned(),
            )
        })?;
    Ok(IssueReference { profile, id })
}
