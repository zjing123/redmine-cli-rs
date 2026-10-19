use redmine_api::api::issues::{
    CreateIssue, GetIssue, Issue, IssueInclude, IssueListInclude, ListIssues, SortByColumn,
    UpdateIssue,
};
use redmine_api::api::{self, Endpoint, Pageable, QueryParams, ResponsePage, ReturnsJsonResponse};
use serde_json::Value;
use thiserror::Error;

use crate::{config::Profile, redmine_client::RedmineClient};

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("redmine-api error: {0}")]
    Redmine(#[from] redmine_api::Error),
    #[error(transparent)]
    Client(#[from] crate::redmine_client::RedmineClientError),
    #[error("invalid request JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid endpoint options: {0}")]
    Builder(String),
    #[error("unsupported or invalid issue field: {0}")]
    InvalidField(String),
}

pub struct ApiClient {
    client: RedmineClient,
}

impl ApiClient {
    pub fn new(profile: &Profile) -> Result<Self, ApiError> {
        Ok(Self {
            client: RedmineClient::from_profile(profile)?,
        })
    }

    pub fn get_issue(&self, id: u64, includes: &[IssueInclude]) -> Result<Value, ApiError> {
        let mut builder = GetIssue::builder();
        builder.id(id);
        if !includes.is_empty() {
            builder.include(includes.to_vec());
        }
        let endpoint = builder
            .build()
            .map_err(|error| ApiError::Builder(error.to_string()))?;
        let response: api::issues::IssueWrapper<Issue> =
            self.client.api().json_response_body(&endpoint)?;
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
        if let Some(assignee) = &filters.assignee {
            builder.assignee(assignee_filter(assignee)?);
        }
        if let Some(subject) = &filters.subject {
            builder.subject(api::StringFieldFilter::SubStringMatch(subject.clone()));
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
        let endpoint = FilteredIssueList {
            endpoint,
            filters: &filters.filters,
        };
        let page = self.client.api().json_response_body_page::<_, Issue>(
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
        let endpoint = create_endpoint(&payload)?;
        let response: api::issues::IssueWrapper<Value> =
            self.client.api().json_response_body(&endpoint)?;
        Ok(response.issue)
    }

    pub fn update_issue(&self, id: u64, payload: Value) -> Result<Value, ApiError> {
        let endpoint = update_endpoint(id, &payload)?;
        self.client.api().ignore_response_body(&endpoint)?;
        Ok(serde_json::json!({"id": id, "updated": true}))
    }

    pub fn delete_issue(&self, id: u64) -> Result<(), ApiError> {
        let endpoint = api::issues::DeleteIssue::builder()
            .id(id)
            .build()
            .map_err(|error| ApiError::Builder(error.to_string()))?;
        self.client.api().ignore_response_body(&endpoint)?;
        Ok(())
    }

    pub fn download_attachment(
        &self,
        attachment: &redmine_api::api::attachments::Attachment,
    ) -> Result<Vec<u8>, ApiError> {
        Ok(self.client.api().download_attachment(attachment)?)
    }
}

#[derive(Debug, Default)]
pub struct IssueFilters {
    pub project_id: Option<u64>,
    pub tracker_id: Option<u64>,
    pub priority_id: Option<u64>,
    pub status: Option<String>,
    pub assignee: Option<String>,
    pub subject: Option<String>,
    pub filters: Vec<(String, String)>,
    pub includes: Vec<IssueListInclude>,
    pub sort: Option<String>,
    pub limit: u64,
    pub offset: u64,
}

struct FilteredIssueList<'a> {
    endpoint: ListIssues,
    filters: &'a [(String, String)],
}

impl Endpoint for FilteredIssueList<'_> {
    fn method(&self) -> reqwest::Method {
        self.endpoint.method()
    }

    fn endpoint(&self) -> std::borrow::Cow<'static, str> {
        self.endpoint.endpoint()
    }

    fn parameters(&self) -> QueryParams<'_> {
        let mut parameters = self.endpoint.parameters();
        parameters.extend(
            self.filters
                .iter()
                .map(|(key, value)| (key.as_str(), value.as_str())),
        );
        parameters
    }

    fn body(&self) -> Result<Option<(&'static str, Vec<u8>)>, redmine_api::Error> {
        self.endpoint.body()
    }
}

impl ReturnsJsonResponse for FilteredIssueList<'_> {}

impl Pageable for FilteredIssueList<'_> {
    fn response_wrapper_key(&self) -> String {
        "issues".to_owned()
    }
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

fn assignee_filter(value: &str) -> Result<api::issues::AssigneeFilter, ApiError> {
    match value {
        "me" => Ok(api::issues::AssigneeFilter::Me),
        "none" => Ok(api::issues::AssigneeFilter::NoAssignee),
        id => id
            .parse::<u64>()
            .map(|id| api::issues::AssigneeFilter::TheseAssignees(vec![id]))
            .map_err(|_| ApiError::InvalidField(format!("invalid assignee: {value}"))),
    }
}

fn create_endpoint(payload: &Value) -> Result<CreateIssue<'static>, ApiError> {
    validate_fields(payload, CREATE_FIELDS)?;
    let object = payload
        .as_object()
        .ok_or_else(|| ApiError::InvalidField("payload must be a JSON object".to_owned()))?;
    let project_id = required_u64(object, "project_id")?;
    let mut builder = CreateIssue::builder();
    builder.project_id(project_id);
    set_common_create_fields(&mut builder, object)?;
    if let Some(value) = custom_fields(object, "custom_fields")? {
        builder.custom_fields(value);
    }
    if let Some(value) = u64_vec(object, "watcher_user_ids")? {
        builder.watcher_user_ids(value);
    }
    if let Some(value) = bool_field(object, "is_private")? {
        builder.is_private(value);
    }
    if let Some(value) = f64_field(object, "estimated_hours")? {
        builder.estimated_hours(value);
    }
    if let Some(value) = uploads(object, "uploads")? {
        builder.uploads(value);
    }
    builder
        .build()
        .map_err(|error| ApiError::Builder(error.to_string()))
}

fn update_endpoint(id: u64, payload: &Value) -> Result<UpdateIssue<'static>, ApiError> {
    validate_fields(payload, UPDATE_FIELDS)?;
    let object = payload
        .as_object()
        .ok_or_else(|| ApiError::InvalidField("payload must be a JSON object".to_owned()))?;
    let mut builder = UpdateIssue::builder();
    builder.id(id);
    set_common_update_fields(&mut builder, object)?;
    if let Some(value) = custom_fields(object, "custom_fields")? {
        builder.custom_fields(value);
    }
    if let Some(value) = u64_vec(object, "watcher_user_ids")? {
        builder.watcher_user_ids(value);
    }
    if let Some(value) = bool_field(object, "is_private")? {
        builder.is_private(value);
    }
    if let Some(value) = f64_field(object, "estimated_hours")? {
        builder.estimated_hours(value);
    }
    if let Some(value) = string_field(object, "notes")? {
        builder.notes(value.into());
    }
    if let Some(value) = bool_field(object, "private_notes")? {
        builder.private_notes(value);
    }
    if let Some(value) = uploads(object, "uploads")? {
        builder.uploads(value);
    }
    builder
        .build()
        .map_err(|error| ApiError::Builder(error.to_string()))
}

fn validate_fields(payload: &Value, allowed: &[&str]) -> Result<(), ApiError> {
    let object = payload
        .as_object()
        .ok_or_else(|| ApiError::InvalidField("payload must be a JSON object".to_owned()))?;
    if let Some(field) = object
        .keys()
        .find(|field| !allowed.contains(&field.as_str()))
    {
        return Err(ApiError::InvalidField(field.clone()));
    }
    Ok(())
}

macro_rules! set_common_numeric {
    ($builder:ident, $object:ident, $( $field:ident ),+ $(,)?) => {
        $(
            if let Some(value) = u64_field($object, stringify!($field))? {
                $builder.$field(value);
            }
        )+
    };
}

macro_rules! set_common_text {
    ($builder:ident, $object:ident, $( $field:ident ),+ $(,)?) => {
        $(
            if let Some(value) = string_field($object, stringify!($field))? {
                $builder.$field(value);
            }
        )+
    };
}

fn set_common_create_fields(
    builder: &mut redmine_api::api::issues::CreateIssueBuilder<'static>,
    object: &serde_json::Map<String, Value>,
) -> Result<(), ApiError> {
    set_common_numeric!(
        builder,
        object,
        tracker_id,
        status_id,
        priority_id,
        category_id,
        assigned_to_id,
        parent_issue_id
    );
    if let Some(value) = u64_field(object, "fixed_version_id")? {
        builder.version(value);
    }
    set_common_text!(builder, object, subject, description);
    Ok(())
}

fn set_common_update_fields(
    builder: &mut redmine_api::api::issues::UpdateIssueBuilder<'static>,
    object: &serde_json::Map<String, Value>,
) -> Result<(), ApiError> {
    set_common_numeric!(
        builder,
        object,
        project_id,
        tracker_id,
        status_id,
        priority_id,
        category_id,
        assigned_to_id,
        parent_issue_id
    );
    if let Some(value) = u64_field(object, "fixed_version_id")? {
        builder.version(value);
    }
    set_common_text!(builder, object, subject, description);
    Ok(())
}

fn required_u64(object: &serde_json::Map<String, Value>, field: &str) -> Result<u64, ApiError> {
    u64_field(object, field)?.ok_or_else(|| ApiError::InvalidField(format!("{field} is required")))
}

fn u64_field(
    object: &serde_json::Map<String, Value>,
    field: &str,
) -> Result<Option<u64>, ApiError> {
    object
        .get(field)
        .map(|value| {
            value.as_u64().ok_or_else(|| {
                ApiError::InvalidField(format!("{field} must be an unsigned integer"))
            })
        })
        .transpose()
}

fn u64_vec(
    object: &serde_json::Map<String, Value>,
    field: &str,
) -> Result<Option<Vec<u64>>, ApiError> {
    object
        .get(field)
        .map(|value| {
            value
                .as_array()
                .ok_or_else(|| ApiError::InvalidField(format!("{field} must be an array")))?
                .iter()
                .map(|item| {
                    item.as_u64().ok_or_else(|| {
                        ApiError::InvalidField(format!("{field} must contain unsigned integers"))
                    })
                })
                .collect()
        })
        .transpose()
}

fn string_field(
    object: &serde_json::Map<String, Value>,
    field: &str,
) -> Result<Option<String>, ApiError> {
    object
        .get(field)
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| ApiError::InvalidField(format!("{field} must be a string")))
        })
        .transpose()
}

fn bool_field(
    object: &serde_json::Map<String, Value>,
    field: &str,
) -> Result<Option<bool>, ApiError> {
    object
        .get(field)
        .map(|value| {
            value
                .as_bool()
                .ok_or_else(|| ApiError::InvalidField(format!("{field} must be a boolean")))
        })
        .transpose()
}

fn f64_field(
    object: &serde_json::Map<String, Value>,
    field: &str,
) -> Result<Option<f64>, ApiError> {
    object
        .get(field)
        .map(|value| {
            value
                .as_f64()
                .ok_or_else(|| ApiError::InvalidField(format!("{field} must be a number")))
        })
        .transpose()
}

fn custom_fields(
    object: &serde_json::Map<String, Value>,
    field: &str,
) -> Result<Option<Vec<redmine_api::api::custom_fields::CustomField<'static>>>, ApiError> {
    object
        .get(field)
        .map(|value| {
            if let Some(fields) = value.as_object() {
                return fields
                    .iter()
                    .map(|(id, value)| {
                        let id = id.parse().map_err(|_| {
                            ApiError::InvalidField("custom_fields keys must be numeric".to_owned())
                        })?;
                        let value = value.as_str().ok_or_else(|| {
                            ApiError::InvalidField(
                                "custom_fields values must be strings".to_owned(),
                            )
                        })?;
                        Ok(redmine_api::api::custom_fields::CustomField {
                            id,
                            name: None,
                            value: value.to_owned().into(),
                        })
                    })
                    .collect();
            }
            serde_json::from_value(value.clone()).map_err(|error| {
                ApiError::InvalidField(format!("custom_fields has invalid format: {error}"))
            })
        })
        .transpose()
}

fn uploads(
    object: &serde_json::Map<String, Value>,
    field: &str,
) -> Result<Option<Vec<redmine_api::api::issues::UploadedAttachment<'static>>>, ApiError> {
    object
        .get(field)
        .map(|value| {
            let entries = value
                .as_array()
                .ok_or_else(|| ApiError::InvalidField(format!("{field} must be an array")))?;
            entries
                .iter()
                .map(|entry| {
                    let object = entry.as_object().ok_or_else(|| {
                        ApiError::InvalidField(format!("{field} entries must be objects"))
                    })?;
                    Ok(redmine_api::api::issues::UploadedAttachment {
                        token: required_string(object, "token")?.into(),
                        filename: required_string(object, "filename")?.into(),
                        description: optional_string(object, "description")?.map(Into::into),
                        content_type: required_string(object, "content_type")?.into(),
                    })
                })
                .collect()
        })
        .transpose()
}

fn required_string(
    object: &serde_json::Map<String, Value>,
    field: &str,
) -> Result<String, ApiError> {
    optional_string(object, field)?
        .ok_or_else(|| ApiError::InvalidField(format!("{field} is required")))
}

fn optional_string(
    object: &serde_json::Map<String, Value>,
    field: &str,
) -> Result<Option<String>, ApiError> {
    object
        .get(field)
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| ApiError::InvalidField(format!("{field} must be a string")))
        })
        .transpose()
}

const CREATE_FIELDS: &[&str] = &[
    "project_id",
    "tracker_id",
    "status_id",
    "priority_id",
    "subject",
    "description",
    "category_id",
    "fixed_version_id",
    "assigned_to_id",
    "parent_issue_id",
    "custom_fields",
    "watcher_user_ids",
    "is_private",
    "estimated_hours",
    "uploads",
];

const UPDATE_FIELDS: &[&str] = &[
    "project_id",
    "tracker_id",
    "status_id",
    "priority_id",
    "subject",
    "description",
    "category_id",
    "fixed_version_id",
    "assigned_to_id",
    "parent_issue_id",
    "custom_fields",
    "watcher_user_ids",
    "is_private",
    "estimated_hours",
    "notes",
    "private_notes",
    "uploads",
];

#[cfg(test)]
mod tests {
    use super::assignee_filter;

    #[test]
    fn assignee_filter_supports_current_user() {
        assert_eq!(assignee_filter("me").unwrap().to_string(), "me");
    }

    #[test]
    fn assignee_filter_supports_unassigned() {
        assert_eq!(assignee_filter("none").unwrap().to_string(), "!*");
    }

    #[test]
    fn assignee_filter_supports_numeric_ids() {
        assert_eq!(assignee_filter("42").unwrap().to_string(), "42");
    }

    #[test]
    fn assignee_filter_rejects_unknown_names() {
        assert!(assignee_filter("alice").is_err());
    }
}
