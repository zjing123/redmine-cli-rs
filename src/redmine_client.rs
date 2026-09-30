use std::borrow::Cow;

use redmine_api::api::{self, Endpoint, NoPagination, QueryParams, Redmine, ReturnsJsonResponse};
use redmine_api::reqwest::{Method, blocking::Client};
use serde_json::{Value, json};
use thiserror::Error;
use url::Url;

use crate::config::Profile;

#[derive(Debug, Error)]
pub enum RedmineClientError {
    #[error("redmine-api error: {0}")]
    Redmine(#[from] redmine_api::Error),
    #[error("HTTP client error: {0}")]
    Request(#[from] redmine_api::reqwest::Error),
    #[error("invalid profile URL: {0}")]
    InvalidUrl(#[from] url::ParseError),
    #[error("invalid endpoint options: {0}")]
    Builder(String),
}

pub struct RedmineClient {
    pub(crate) client: Redmine,
}

impl RedmineClient {
    pub fn from_profile(profile: &Profile) -> Result<Self, RedmineClientError> {
        let http = Client::builder().tls_backend_rustls().build()?;
        let url = Url::parse(&profile.url)?;
        Ok(Self {
            client: api::Redmine::new(http, url, profile.api_key.as_deref().unwrap_or_default())?,
        })
    }

    pub(crate) fn api(&self) -> &Redmine {
        &self.client
    }

    pub(crate) fn request_json(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
        query: &[(&str, String)],
    ) -> Result<Value, RedmineClientError> {
        let endpoint = ResourceEndpoint {
            method,
            path: path.to_owned(),
            body,
            query: query
                .iter()
                .map(|(key, value)| ((*key).to_owned(), value.clone()))
                .collect(),
        };
        Ok(self.client.json_response_body(&endpoint)?)
    }

    pub(crate) fn delete(&self, path: &str) -> Result<(), RedmineClientError> {
        let endpoint = ResourceEndpoint {
            method: Method::DELETE,
            path: path.to_owned(),
            body: None,
            query: Vec::new(),
        };
        self.client.ignore_response_body(&endpoint)?;
        Ok(())
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

struct ResourceEndpoint {
    method: Method,
    path: String,
    body: Option<Value>,
    query: Vec<(String, String)>,
}

impl ReturnsJsonResponse for ResourceEndpoint {}
impl NoPagination for ResourceEndpoint {}

impl Endpoint for ResourceEndpoint {
    fn method(&self) -> Method {
        self.method.clone()
    }

    fn endpoint(&self) -> Cow<'static, str> {
        Cow::Owned(self.path.clone())
    }

    fn parameters(&self) -> QueryParams<'_> {
        let mut params = QueryParams::default();
        for (key, value) in &self.query {
            params.push(key.as_str(), value.as_str());
        }
        params
    }

    fn body(&self) -> Result<Option<(&'static str, Vec<u8>)>, redmine_api::Error> {
        self.body
            .as_ref()
            .map(|body| serde_json::to_vec(body).map(|data| ("application/json", data)))
            .transpose()
            .map_err(redmine_api::Error::from)
    }
}
