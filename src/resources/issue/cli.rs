use std::io::Read;

use clap::{Args, Subcommand};
use serde_json::{Map, Value};
use thiserror::Error;

use crate::{
    config::{Config, ConfigError, parse_issue_reference},
    output,
    resources::issue::api::{ApiClient, IssueFilters},
};

#[derive(Debug, Error)]
pub enum IssueError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Api(#[from] super::api::ApiError),
    #[error("invalid JSON input: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid metadata YAML: {0}")]
    Metadata(#[from] serde_yaml::Error),
    #[error("failed to read JSON from stdin")]
    ReadStdin,
    #[error("missing required field '{0}'")]
    MissingField(String),
    #[error("custom fields must be a JSON object with numeric keys")]
    CustomFieldsObject,
    #[error("unsupported command")]
    Unsupported,
}

#[derive(Debug, Args)]
#[command(
    about = "Manage Redmine issues",
    after_help = "Examples:\n  redmine-cli -p prod issue get 123\n  redmine-cli issue get https://prod.example.com/issues/123\n  redmine-cli issue get 123 --include attachments,journals\n  redmine-cli issue list --status open --limit 20\n  redmine-cli issue list --project-id 1 --fields id,subject,status\n  redmine-cli issue create --project-id 1 --subject \"New issue\"\n  redmine-cli issue update 123 --json '{\"priority_id\":3}'\n  redmine-cli issue fields\n  redmine-cli issue schema"
)]
pub struct IssueCommand {
    #[command(subcommand)]
    pub command: IssueSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum IssueSubcommand {
    #[command(
        about = "Get an issue by ID or URL",
        after_help = "Examples:\n  redmine-cli issue get 123\n  redmine-cli issue get https://redmine.example.com/issues/123 --include attachments,journals"
    )]
    Get {
        #[arg(help = "Issue ID or Redmine URL")]
        reference: String,
        #[arg(long, help = "Return only these comma-separated fields")]
        fields: Option<String>,
        #[arg(long, value_delimiter = ',', help = "Related data to include")]
        include: Vec<String>,
    },
    #[command(
        about = "List issues with optional filters",
        after_help = "Examples:\n  redmine-cli issue list --status open --limit 20\n  redmine-cli issue list --project-id 1 --fields id,subject,status"
    )]
    List {
        #[arg(long, help = "Filter by project ID")]
        project_id: Option<u64>,
        #[arg(long, help = "Filter by tracker ID")]
        tracker_id: Option<u64>,
        #[arg(long, help = "Filter by priority ID")]
        priority_id: Option<u64>,
        #[arg(long, help = "Filter by status name or comma-separated status IDs")]
        status: Option<String>,
        #[arg(long, help = "Filter by assignee ID")]
        assigned_to_id: Option<u64>,
        #[arg(long, help = "Filter by subject substring")]
        subject: Option<String>,
        #[arg(long, help = "Filter custom fields as a JSON object of ID to value")]
        custom_fields: Option<String>,
        #[arg(long, value_delimiter = ',', help = "Related data to include")]
        include: Vec<String>,
        #[arg(long, help = "Sort by a column; append :desc for reverse order")]
        sort: Option<String>,
        #[arg(long, help = "Return only these comma-separated fields")]
        fields: Option<String>,
        #[arg(
            long,
            default_value_t = 50,
            help = "Maximum number of issues to return"
        )]
        limit: u64,
        #[arg(long, default_value_t = 0, help = "Number of issues to skip")]
        offset: u64,
    },
    #[command(
        about = "Create a new issue",
        after_help = "Examples:\n  redmine-cli issue create --project-id 1 --subject \"New issue\"\n  echo '{\"project_id\":1,\"subject\":\"New issue\"}' | redmine-cli issue create --stdin"
    )]
    Create(InputArgs),
    #[command(
        about = "Update an existing issue",
        after_help = "Examples:\n  redmine-cli issue update 123 --subject \"Updated subject\"\n  redmine-cli issue update 123 --json '{\"priority_id\":3}'"
    )]
    Update {
        #[arg(help = "Issue ID or Redmine URL")]
        reference: String,
        #[command(flatten)]
        input: InputArgs,
    },
    #[command(
        about = "Delete an issue",
        after_help = "Example:\n  redmine-cli issue delete 123"
    )]
    Delete {
        #[arg(help = "Issue ID or Redmine URL")]
        reference: String,
    },
    #[command(
        about = "Show issue response fields",
        after_help = "Example:\n  redmine-cli issue fields"
    )]
    Fields,
    #[command(
        about = "Show issue create/update schema",
        after_help = "Example:\n  redmine-cli issue schema"
    )]
    Schema,
}

#[derive(Debug, Args)]
pub struct InputArgs {
    #[arg(
        long,
        conflicts_with = "stdin",
        help = "JSON object containing supported issue fields"
    )]
    pub json: Option<String>,
    #[arg(
        long,
        conflicts_with = "json",
        help = "Read the JSON object from standard input"
    )]
    pub stdin: bool,
    #[arg(long, help = "Project ID; required when creating an issue")]
    pub project_id: Option<u64>,
    #[arg(long, help = "Issue subject")]
    pub subject: Option<String>,
    #[arg(long, help = "Issue description")]
    pub description: Option<String>,
    #[arg(long, help = "Tracker ID")]
    pub tracker_id: Option<u64>,
    #[arg(long, help = "Priority ID")]
    pub priority_id: Option<u64>,
    #[arg(long, help = "Assignee ID")]
    pub assigned_to_id: Option<u64>,
    #[arg(long, help = "Custom fields as a JSON object of ID to value")]
    pub custom_fields: Option<String>,
}

pub fn run(issue: IssueCommand, profile: Option<&str>) -> Result<(), IssueError> {
    if matches!(
        &issue.command,
        IssueSubcommand::Fields | IssueSubcommand::Schema
    ) {
        return run_issue_without_config(issue.command);
    }
    let config = Config::load(None)?;
    run_issue(&config, profile, issue.command)
}

fn run_issue_without_config(command: IssueSubcommand) -> Result<(), IssueError> {
    match command {
        IssueSubcommand::Fields => output::success(metadata_section("fields")?),
        IssueSubcommand::Schema => output::success(metadata_section("schema")?),
        _ => unreachable!("only metadata commands are handled without configuration"),
    }
    Ok(())
}

fn run_issue(
    config: &Config,
    profile: Option<&str>,
    command: IssueSubcommand,
) -> Result<(), IssueError> {
    match command {
        IssueSubcommand::Fields | IssueSubcommand::Schema => {
            unreachable!("metadata commands are handled before API configuration")
        }
        IssueSubcommand::Get {
            reference,
            fields,
            include,
        } => {
            let (client, id) = client_for_reference(config, profile, &reference)?;
            let includes: Vec<redmine_api::api::issues::IssueInclude> = include
                .into_iter()
                .filter_map(|item| parse_include(&item))
                .collect();
            output::success(select_fields(
                client.get_issue(id, &includes)?,
                fields.as_deref(),
            ));
        }
        IssueSubcommand::List {
            project_id,
            tracker_id,
            priority_id,
            status,
            assigned_to_id,
            subject,
            custom_fields,
            include,
            sort,
            fields,
            limit,
            offset,
        } => {
            let (name, profile_data) = config.profile(profile)?;
            let client = ApiClient::new(&profile_data)?;
            let custom_fields = custom_fields
                .as_deref()
                .map(parse_custom_fields)
                .transpose()?;
            let includes = include
                .into_iter()
                .filter_map(|item| parse_list_include(&item))
                .collect();
            let page = client.list_issues(&IssueFilters {
                project_id,
                tracker_id,
                priority_id,
                status,
                assigned_to_id,
                subject,
                custom_fields,
                includes,
                sort,
                limit,
                offset,
            });
            let page = page?;
            let issues = page
                .values
                .into_iter()
                .map(|issue| select_fields(issue, fields.as_deref()))
                .collect::<Vec<_>>();
            output::success(
                serde_json::json!({"total_count": page.total_count, "limit": page.limit, "offset": page.offset, "issues": issues, "profile": name}),
            );
        }
        IssueSubcommand::Create(input) => {
            let (_, profile_data) = config.profile(profile)?;
            let payload = read_input(input)?;
            if payload.get("project_id").is_none() {
                return Err(IssueError::MissingField("project_id".to_owned()));
            }
            output::success(ApiClient::new(&profile_data)?.create_issue(payload)?);
        }
        IssueSubcommand::Update { reference, input } => {
            let (client, id) = client_for_reference(config, profile, &reference)?;
            output::success(client.update_issue(id, read_input(input)?)?);
        }
        IssueSubcommand::Delete { reference } => {
            let (client, id) = client_for_reference(config, profile, &reference)?;
            client.delete_issue(id)?;
            output::success(serde_json::json!({"deleted": true, "id": id}));
        }
    }
    Ok(())
}

fn metadata_section(section: &str) -> Result<Value, IssueError> {
    let metadata: Value = serde_yaml::from_str(include_str!("metadata.yaml"))?;
    Ok(metadata.get(section).cloned().unwrap_or(Value::Null))
}

fn client_for_reference(
    config: &Config,
    profile: Option<&str>,
    reference: &str,
) -> Result<(ApiClient, u64), IssueError> {
    let parsed = parse_issue_reference(reference, profile, config)?;
    let (_, profile_data) = config.profile(Some(&parsed.profile))?;
    Ok((ApiClient::new(&profile_data)?, parsed.id))
}

fn read_input(input: InputArgs) -> Result<Value, IssueError> {
    let text = if let Some(json) = input.json {
        json
    } else if input.stdin {
        let mut text = String::new();
        std::io::stdin()
            .read_to_string(&mut text)
            .map_err(|_| IssueError::ReadStdin)?;
        text
    } else {
        "{}".to_owned()
    };
    let mut value: Map<String, Value> = serde_json::from_str(&text)?;
    if let Some(project_id) = input.project_id {
        value.insert("project_id".to_owned(), project_id.into());
    }
    if let Some(subject) = input.subject {
        value.insert("subject".to_owned(), subject.into());
    }
    if let Some(description) = input.description {
        value.insert("description".to_owned(), description.into());
    }
    if let Some(tracker_id) = input.tracker_id {
        value.insert("tracker_id".to_owned(), tracker_id.into());
    }
    if let Some(priority_id) = input.priority_id {
        value.insert("priority_id".to_owned(), priority_id.into());
    }
    if let Some(assigned_to_id) = input.assigned_to_id {
        value.insert("assigned_to_id".to_owned(), assigned_to_id.into());
    }
    if let Some(custom_fields) = input.custom_fields {
        let fields: Value = serde_json::from_str(&custom_fields)?;
        if !fields.is_object() {
            return Err(IssueError::CustomFieldsObject);
        }
        value.insert("custom_fields".to_owned(), fields);
    }
    Ok(Value::Object(value))
}

fn select_fields(value: Value, fields: Option<&str>) -> Value {
    let Some(fields) = fields else {
        return value;
    };
    let Some(object) = value.as_object() else {
        return value;
    };
    Value::Object(
        fields
            .split(',')
            .filter_map(|field| {
                let field = field.trim();
                object
                    .get(field)
                    .map(|value| (field.to_owned(), value.clone()))
            })
            .collect::<Map<_, _>>(),
    )
}

fn parse_custom_fields(value: &str) -> Result<Vec<(u64, String)>, IssueError> {
    let fields: Value = serde_json::from_str(value)?;
    let Some(fields) = fields.as_object() else {
        return Err(IssueError::CustomFieldsObject);
    };
    fields
        .iter()
        .map(|(id, value)| {
            let id = id.parse().map_err(|_| IssueError::CustomFieldsObject)?;
            let value = value
                .as_str()
                .ok_or(IssueError::CustomFieldsObject)?
                .to_owned();
            Ok((id, value))
        })
        .collect()
}

fn parse_include(value: &str) -> Option<redmine_api::api::issues::IssueInclude> {
    match value {
        "children" => Some(redmine_api::api::issues::IssueInclude::Children),
        "attachments" => Some(redmine_api::api::issues::IssueInclude::Attachments),
        "relations" => Some(redmine_api::api::issues::IssueInclude::Relations),
        "changesets" => Some(redmine_api::api::issues::IssueInclude::Changesets),
        "journals" => Some(redmine_api::api::issues::IssueInclude::Journals),
        "watchers" => Some(redmine_api::api::issues::IssueInclude::Watchers),
        "allowed_statuses" => Some(redmine_api::api::issues::IssueInclude::AllowedStatuses),
        _ => None,
    }
}

fn parse_list_include(value: &str) -> Option<redmine_api::api::issues::IssueListInclude> {
    match value {
        "relations" => Some(redmine_api::api::issues::IssueListInclude::Relations),
        "time_entries" => Some(redmine_api::api::issues::IssueListInclude::TimeEntries),
        _ => None,
    }
}
