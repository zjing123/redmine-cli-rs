use redmine_api::api;
use reqwest::blocking::Client;
use serde_json::{Value, json};
use thiserror::Error;
use url::Url;

use crate::config::Profile;

#[derive(Debug, Error)]
pub enum RedmineClientError {
    #[error("redmine-api error: {0}")]
    Redmine(#[from] redmine_api::Error),
    #[error("HTTP client error: {0}")]
    Request(#[from] reqwest::Error),
    #[error("invalid profile URL: {0}")]
    InvalidUrl(#[from] url::ParseError),
    #[error("invalid endpoint options: {0}")]
    Builder(String),
}

pub struct RedmineClient {
    client: redmine_api::api::Redmine,
}

impl RedmineClient {
    pub fn from_profile(profile: &Profile) -> Result<Self, RedmineClientError> {
        let http = Client::builder().use_rustls_tls().build()?;
        let url = Url::parse(&profile.url)?;
        Ok(Self {
            client: api::Redmine::new(http, url, profile.api_key.as_deref().unwrap_or_default())?,
        })
    }

    pub fn current_user(&self) -> Result<Value, RedmineClientError> {
        let endpoint = api::users::GetUser::builder()
            .build()
            .map_err(|error| RedmineClientError::Builder(error.to_string()))?;
        let mut response: Value = self.client.json_response_body(&endpoint)?;
        if let Some(user) = response.get_mut("user").and_then(Value::as_object_mut) {
            user.remove("api_key");
        }
        Ok(response.get("user").cloned().unwrap_or(response))
    }

    pub fn search(
        &self,
        query: String,
        limit: u64,
        offset: u64,
    ) -> Result<Value, RedmineClientError> {
        let endpoint = api::search::Search::builder()
            .q(query)
            .build()
            .map_err(|error| RedmineClientError::Builder(error.to_string()))?;
        let page = self
            .client
            .json_response_body_page::<_, Value>(&endpoint, offset, limit)?;
        Ok(json!({
            "results": page.values,
            "total_count": page.total_count,
            "offset": page.offset,
            "limit": page.limit,
        }))
    }
}
