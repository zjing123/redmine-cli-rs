use std::{
    fs::OpenOptions,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};

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
    #[error("filter must use KEY=VALUE format with a non-empty key and value: {0}")]
    InvalidFilter(String),
    #[error("unsupported command")]
    Unsupported,
    #[error("configuration is read-only; create, update, and delete operations are disabled")]
    ReadOnly,
    #[error("failed to create attachment directory '{path}': {source}")]
    CreateAttachmentDirectory {
        path: String,
        source: std::io::Error,
    },
    #[error("attachment filename is not a safe local filename: {0}")]
    InvalidAttachmentFilename(String),
    #[error("failed to create attachment file '{path}': {source}")]
    CreateAttachmentFile {
        path: String,
        source: std::io::Error,
    },
    #[error("failed to write attachment file '{path}': {source}")]
    WriteAttachmentFile {
        path: String,
        source: std::io::Error,
    },
}

#[derive(Debug, Args)]
#[command(
    about = "Manage Redmine issues",
    after_help = "ID-based requests, list, create, update, and delete use -p/--profile, REDMINE_PROFILE, or active_profile. A full issue URL can auto-detect the profile.\nExamples:\n  # Get an issue by ID\n  redmine-cli -p prod issue get 123\n\n  # Get an issue by URL and auto-detect the profile\n  redmine-cli issue get https://prod.example.com/issues/123\n\n  # List open issues\n  redmine-cli -p prod issue list --status open --limit 20\n\n  # Create an issue\n  redmine-cli -p prod issue create --project-id 1 --subject \"New issue\"\n\n  # Update an issue\n  redmine-cli -p prod issue update 123 --json '{\"priority_id\":3}'\n\n  # Show the issue response fields\n  redmine-cli issue fields\n\n  # Show the issue create/update schema\n  redmine-cli issue schema"
)]
pub struct IssueCommand {
    #[command(subcommand)]
    pub command: IssueSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum IssueSubcommand {
    #[command(
        about = "Get an issue by ID or URL",
        after_help = "Examples:\n  # Numeric issue IDs use -p/--profile, REDMINE_PROFILE, or active_profile\n  redmine-cli -p prod issue get 123\n\n  # A full issue URL can automatically select a matching configured profile\n  redmine-cli issue get https://redmine.example.com/issues/123\n\n  # Return only selected fields\n  redmine-cli -p prod issue get 123 --fields id,subject,status\n\n  # Include journals, children, relations, and attachments\n  redmine-cli -p prod issue get 123 --include journals,children,relations,attachments\n\n  # Use the related-data shortcut flags\n  redmine-cli -p prod issue get 123 --journals --children --relations --attachments\n\n  # Download all attachments to a directory\n  redmine-cli -p prod issue get 123 --download-attachments ./downloads/123\n\n  # Select fields and include journals and attachments using a URL\n  redmine-cli issue get https://redmine.example.com/issues/123 --fields id,subject --journals --attachments"
    )]
    Get {
        #[arg(help = "Issue ID or Redmine URL")]
        reference: String,
        #[arg(long, help = "Return only these comma-separated fields")]
        fields: Option<String>,
        #[arg(long, value_delimiter = ',', help = "Related data to include")]
        include: Vec<String>,
        #[arg(long, help = "Shortcut for --include journals")]
        journals: bool,
        #[arg(long, help = "Shortcut for --include children")]
        children: bool,
        #[arg(long, help = "Shortcut for --include relations")]
        relations: bool,
        #[arg(long, help = "Shortcut for --include attachments")]
        attachments: bool,
        #[arg(
            long,
            value_name = "DIRECTORY",
            help = "Download all issue attachments to this directory"
        )]
        download_attachments: Option<PathBuf>,
    },
    #[command(
        about = "List issues with optional filters",
        next_line_help = false,
        term_width = 120,
        after_help = "Examples:\n  # List open issues assigned to the current user\n  redmine-cli issues list --project-id 1709 --status open --assignee me\n\n  # Filter by assignee ID, tracker, priority, and subject text\n  redmine-cli issues list --assignee 42 --tracker-id 2 --priority-id 4 --subject \"登录\"\n\n  # Use repeatable Redmine filters, including custom fields\n  redmine-cli issues list --filter status_id=1 --filter cf_12=frontend\n\n  # Return only selected fields\n  redmine-cli issues list --project-id 1709 --fields id,subject,status\n\n  # Return a compact preset of common fields\n  redmine-cli issues list --short\n\n  # Include related data, sort by update time, and paginate\n  redmine-cli issues list --include attachments,relations --sort updated_on:desc --limit 20 --offset 0"
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
        #[arg(
            long,
            value_name = "ASSIGNEE",
            help = "Filter by assignee ID, 'me' for the current user, or 'none' for unassigned issues"
        )]
        assignee: Option<String>,
        #[arg(long, help = "Filter by subject substring")]
        subject: Option<String>,
        #[arg(
            long = "filter",
            value_name = "KEY=VALUE",
            help = "Filter by a Redmine issue field; repeatable (custom fields: cf_ID=VALUE)"
        )]
        filters: Vec<String>,
        #[arg(long, value_delimiter = ',', help = "Related data to include")]
        include: Vec<String>,
        #[arg(long, help = "Sort by a column; append :desc for reverse order")]
        sort: Option<String>,
        #[arg(long, help = "Return only these comma-separated fields")]
        fields: Option<String>,
        #[arg(
            long,
            conflicts_with = "fields",
            help = "Return a compact set of issue fields"
        )]
        short: bool,
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
        after_help = "Examples:\n  # Create an issue with flags\n  redmine-cli issue create --project-id 1 --subject \"New issue\"\n\n  # Create an issue from JSON on standard input\n  echo '{\"project_id\":1,\"subject\":\"New issue\"}' | redmine-cli issue create --stdin"
    )]
    Create(InputArgs),
    #[command(
        about = "Update an existing issue",
        after_help = "Examples:\n  # Update an issue subject\n  redmine-cli issue update 123 --subject \"Updated subject\"\n\n  # Update issue fields from JSON\n  redmine-cli issue update 123 --json '{\"priority_id\":3}'"
    )]
    Update {
        #[arg(help = "Issue ID or Redmine URL")]
        reference: String,
        #[command(flatten)]
        input: InputArgs,
    },
    #[command(
        about = "Delete an issue",
        after_help = "Examples:\n  # Delete an issue by ID\n  redmine-cli issue delete 123"
    )]
    Delete {
        #[arg(help = "Issue ID or Redmine URL")]
        reference: String,
    },
    #[command(
        about = "Show issue response fields",
        after_help = "Examples:\n  # Show fields available in issue responses\n  redmine-cli issue fields"
    )]
    Fields,
    #[command(
        about = "Show issue create/update schema",
        after_help = "Examples:\n  # Show the issue create/update schema\n  redmine-cli issue schema"
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
    #[arg(long = "assignee", value_name = "ID", help = "Assignee ID")]
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
    let mutation_profile = match &issue.command {
        IssueSubcommand::Create(_) => Some(config.profile(profile)?.0),
        IssueSubcommand::Update { reference, .. } | IssueSubcommand::Delete { reference } => {
            Some(parse_issue_reference(reference, profile, &config)?.profile)
        }
        _ => None,
    };
    if let Some(profile_name) = mutation_profile
        && config.is_read_only_for(&profile_name)
    {
        return Err(IssueError::ReadOnly);
    }
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
            journals,
            children,
            relations,
            attachments,
            download_attachments,
        } => {
            let (client, id) = client_for_reference(config, profile, &reference)?;
            let include = merge_issue_includes(
                include,
                journals,
                children,
                relations,
                attachments,
                download_attachments.is_some(),
            );
            let includes: Vec<redmine_api::api::issues::IssueInclude> = include
                .into_iter()
                .filter_map(|item| parse_include(&item))
                .collect();
            let mut issue = client.get_issue(id, &includes)?;
            if let Some(directory) = download_attachments {
                download_issue_attachments(&client, &mut issue, &directory)?;
            }
            output::success(select_fields(issue, fields.as_deref()));
        }
        IssueSubcommand::List {
            project_id,
            tracker_id,
            priority_id,
            status,
            assignee,
            subject,
            filters,
            include,
            sort,
            fields,
            short,
            limit,
            offset,
        } => {
            let (name, profile_data) = config.profile(profile)?;
            let client = ApiClient::new(&profile_data)?;
            let filters = filters
                .iter()
                .map(|filter| {
                    let (key, value) = filter
                        .split_once('=')
                        .ok_or_else(|| IssueError::InvalidFilter(filter.clone()))?;
                    if key.is_empty() || value.is_empty() {
                        return Err(IssueError::InvalidFilter(filter.clone()));
                    }
                    Ok((key.to_owned(), value.to_owned()))
                })
                .collect::<Result<Vec<_>, IssueError>>()?;
            let includes = include
                .into_iter()
                .filter_map(|item| parse_list_include(&item))
                .collect();
            let page = client.list_issues(&IssueFilters {
                project_id,
                tracker_id,
                priority_id,
                status,
                assignee,
                subject,
                filters,
                includes,
                sort,
                limit,
                offset,
            });
            let page = page?;
            let projection = fields.or_else(|| {
                short.then(|| "id,subject,status,priority,assigned_to,updated_on".to_owned())
            });
            let issues = page
                .values
                .into_iter()
                .map(|issue| select_fields(issue, projection.as_deref()))
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

fn merge_issue_includes(
    mut includes: Vec<String>,
    journals: bool,
    children: bool,
    relations: bool,
    attachments: bool,
    download_attachments: bool,
) -> Vec<String> {
    for (name, enabled) in [
        ("journals", journals),
        ("children", children),
        ("relations", relations),
        ("attachments", attachments || download_attachments),
    ] {
        if enabled && !includes.iter().any(|include| include == name) {
            includes.push(name.to_owned());
        }
    }
    includes
}

fn download_issue_attachments(
    client: &ApiClient,
    issue: &mut Value,
    directory: &Path,
) -> Result<(), IssueError> {
    let directory_display = directory.display().to_string();
    std::fs::create_dir_all(directory).map_err(|source| IssueError::CreateAttachmentDirectory {
        path: directory_display,
        source,
    })?;
    let attachments: Vec<redmine_api::api::attachments::Attachment> = issue
        .get("attachments")
        .cloned()
        .map(serde_json::from_value)
        .transpose()?
        .unwrap_or_default();
    for attachment in &attachments {
        let filename = safe_attachment_filename(&attachment.filename)?;
        let path = directory.join(filename);
        let path_display = path.display().to_string();
        let data = client.download_attachment(attachment)?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|source| IssueError::CreateAttachmentFile {
                path: path_display.clone(),
                source,
            })?;
        file.write_all(&data)
            .map_err(|source| IssueError::WriteAttachmentFile {
                path: path_display,
                source,
            })?;
    }
    Ok(())
}

fn safe_attachment_filename(filename: &str) -> Result<&str, IssueError> {
    let mut components = Path::new(filename).components();
    match (components.next(), components.next()) {
        (Some(Component::Normal(_)), None) if !filename.contains('\\') => Ok(filename),
        _ => Err(IssueError::InvalidAttachmentFilename(filename.to_owned())),
    }
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

#[cfg(test)]
mod tests {
    use super::{merge_issue_includes, safe_attachment_filename};

    #[test]
    fn merge_issue_includes_adds_shortcuts_without_duplicates() {
        let includes = merge_issue_includes(
            vec!["attachments".to_owned(), "watchers".to_owned()],
            true,
            false,
            true,
            true,
            false,
        );
        assert_eq!(
            includes,
            ["attachments", "watchers", "journals", "relations"]
        );
    }

    #[test]
    fn safe_attachment_filename_rejects_parent_path() {
        assert!(safe_attachment_filename("../secret.txt").is_err());
    }
}
