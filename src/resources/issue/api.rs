use std::borrow::Cow;

use redmine_api::api::issues::{
    GetIssue, Issue, IssueInclude, IssueListInclude, ListIssues, SortByColumn,
};
use redmine_api::api::{self, Endpoint, NoPagination, ResponsePage, ReturnsJsonResponse};
use reqwest::{Method, blocking::Client};
use serde_json::Value;
use thiserror::Error;
use url::Url;

use crate::config::Profile;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("redmine-api error: {0}")]
    Redmine(#[from] redmine_api::Error),
    #[error("invalid profile URL: {0}")]
    InvalidUrl(#[from] url::ParseError),
    #[error("invalid request JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid endpoint options: {0}")]
    Builder(String),
}

pub struct ApiClient {
    client: redmine_api::api::Redmine,
}

impl ApiClient {
    pub fn new(profile: &Profile) -> Result<Self, ApiError> {
        let http = Client::builder()
            .use_rustls_tls()
            .build()
            .map_err(redmine_api::Error::from)?;
        let url = Url::parse(&profile.url)?;
        let api_key = profile.api_key.as_deref().unwrap_or_default();
        Ok(Self {
            client: api::Redmine::new(http, url, api_key)?,
        })
    }

    pub fn get_issue(&self, id: u64, includes: &[IssueInclude]) -> Result<Value, ApiError> {
        let endpoint = GetIssue::builder()
            .id(id)
            .include(includes.to_vec())
            .build()
            .map_err(|error| ApiError::Builder(error.to_string()))?;
        let response: api::issues::IssueWrapper<Issue> =
            self.client.json_response_body(&endpoint)?;
        Ok(serde_json::to_value(response.issue)?)
    }

    pub fn list_issues(&self, filters: &IssueFilters) -> Result<ResponsePage<Value>, ApiError> {
        let mut builder = ListIssues::builder();
        if let Some(project_id) = filters.project_id {
            builder.project_id(vec![project_id]);
        }
        if let Some(tracker_id) = filters.tracker_id {
            builder.tracker_id(vec![tracker_id]);
        }
        if let Some(priority_id) = filters.priority_id {
            builder.priority_id(vec![priority_id]);
        }
        if let Some(status) = &filters.status {
            builder.status_id(status_filter(status));
        }
        if let Some(assignee_id) = filters.assigned_to_id {
            builder.assignee(api::issues::AssigneeFilter::TheseAssignees(vec![
                assignee_id,
            ]));
        }
        if let Some(subject) = &filters.subject {
            builder.subject(api::StringFieldFilter::SubStringMatch(subject.clone()));
        }
        if let Some(custom_fields) = &filters.custom_fields {
            builder.custom_field_filters(
                custom_fields
                    .iter()
                    .map(|(id, value)| api::CustomFieldFilter {
                        id: *id,
                        value: api::StringFieldFilter::ExactMatch(value.clone()),
                    })
                    .collect(),
            );
        }
        if !filters.includes.is_empty() {
            builder.include(filters.includes.clone());
        }
        if let Some(sort) = &filters.sort {
            builder.sort(vec![sort_filter(sort)]);
        }
        let endpoint = builder
            .build()
            .map_err(|error| ApiError::Builder(error.to_string()))?;
        let page = self.client.json_response_body_page::<_, Issue>(
            &endpoint,
            filters.offset,
            filters.limit,
        )?;
        Ok(ResponsePage {
            values: page
                .values
                .into_iter()
                .map(serde_json::to_value)
                .collect::<Result<_, _>>()?,
            total_count: page.total_count,
            offset: page.offset,
            limit: page.limit,
        })
    }

    pub fn create_issue(&self, payload: Value) -> Result<Value, ApiError> {
        let endpoint = JsonIssueEndpoint::new(Method::POST, "issues.json", payload);
        let response: api::issues::IssueWrapper<Value> =
            self.client.json_response_body(&endpoint)?;
        Ok(response.issue)
    }

    pub fn update_issue(&self, id: u64, payload: Value) -> Result<Value, ApiError> {
        let endpoint = JsonIssueEndpoint::new(Method::PUT, format!("issues/{id}.json"), payload);
        let response: api::issues::IssueWrapper<Value> =
            self.client.json_response_body(&endpoint)?;
        Ok(response.issue)
    }

    pub fn delete_issue(&self, id: u64) -> Result<(), ApiError> {
        let endpoint = api::issues::DeleteIssue::builder()
            .id(id)
            .build()
            .map_err(|error| ApiError::Builder(error.to_string()))?;
        self.client.ignore_response_body(&endpoint)?;
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct IssueFilters {
    pub project_id: Option<u64>,
    pub tracker_id: Option<u64>,
    pub priority_id: Option<u64>,
    pub status: Option<String>,
    pub assigned_to_id: Option<u64>,
    pub subject: Option<String>,
    pub custom_fields: Option<Vec<(u64, String)>>,
    pub includes: Vec<IssueListInclude>,
    pub sort: Option<String>,
    pub limit: u64,
    pub offset: u64,
}

fn status_filter(value: &str) -> api::issues::IssueStatusFilter {
    match value {
        "open" => api::issues::IssueStatusFilter::Open,
        "closed" => api::issues::IssueStatusFilter::Closed,
        "*" | "all" => api::issues::IssueStatusFilter::All,
        value => api::issues::IssueStatusFilter::TheseStatuses(
            value.split(',').filter_map(|id| id.parse().ok()).collect(),
        ),
    }
}

fn sort_filter(value: &str) -> SortByColumn {
    value.strip_suffix(":desc").map_or_else(
        || SortByColumn::Forward {
            column_name: value.to_owned(),
        },
        |column| SortByColumn::Reverse {
            column_name: column.to_owned(),
        },
    )
}

struct JsonIssueEndpoint {
    method: Method,
    endpoint: String,
    body: Value,
}

impl JsonIssueEndpoint {
    fn new(method: Method, endpoint: impl Into<String>, payload: Value) -> Self {
        Self {
            method,
            endpoint: endpoint.into(),
            body: serde_json::json!({"issue": payload}),
        }
    }
}

impl Endpoint for JsonIssueEndpoint {
    fn method(&self) -> Method {
        self.method.clone()
    }
    fn endpoint(&self) -> Cow<'static, str> {
        self.endpoint.clone().into()
    }
    fn body(&self) -> Result<Option<(&'static str, Vec<u8>)>, redmine_api::Error> {
        Ok(Some(("application/json", serde_json::to_vec(&self.body)?)))
    }
}
impl ReturnsJsonResponse for JsonIssueEndpoint {}
impl NoPagination for JsonIssueEndpoint {}
