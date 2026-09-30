use std::io::Read;

use clap::{Args, Subcommand};
use reqwest::{Method, blocking::Client};
use serde_json::{Map, Value, json};
use thiserror::Error;
use url::Url;

use crate::{
    config::{Config, ConfigError, Profile},
    output,
};

#[derive(Debug, Error)]
pub enum ResourceError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error("HTTP request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("invalid resource URL: {0}")]
    Url(#[from] url::ParseError),
    #[error("invalid JSON input: {0}")]
    Json(#[from] serde_json::Error),
    #[error("failed to read JSON from stdin")]
    ReadStdin,
    #[error("invalid metadata YAML: {0}")]
    Metadata(#[from] serde_yaml::Error),
    #[error("resource reference '{0}' is invalid")]
    Reference(String),
    #[error("missing required field '{0}'")]
    MissingField(String),
    #[error("request failed with HTTP {status}: {body}")]
    Http { status: u16, body: String },
}

#[derive(Debug, Subcommand)]
pub enum ResourceCommand {
    Get {
        reference: String,
        #[arg(long)]
        fields: Option<String>,
    },
    List {
        #[arg(long)]
        project_id: Option<u64>,
        #[arg(long)]
        user_id: Option<u64>,
        #[arg(long)]
        assigned_to_id: Option<u64>,
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        sort: Option<String>,
        #[arg(long)]
        fields: Option<String>,
        #[arg(long, default_value_t = 50)]
        limit: u64,
        #[arg(long, default_value_t = 0)]
        offset: u64,
    },
    Create(ResourceInput),
    Update {
        reference: String,
        #[command(flatten)]
        input: ResourceInput,
    },
    Delete {
        reference: String,
    },
    Fields,
    Schema,
}

#[derive(Debug, Args)]
pub struct ResourceInput {
    #[arg(long, conflicts_with = "stdin")]
    pub json: Option<String>,
    #[arg(long, conflicts_with = "json")]
    pub stdin: bool,
}

pub fn run(
    resource: &str,
    singular: &str,
    metadata: &str,
    command: ResourceCommand,
    profile: Option<&str>,
) -> Result<(), ResourceError> {
    if let ResourceCommand::Fields | ResourceCommand::Schema = command {
        let metadata: Value = serde_yaml::from_str(metadata)?;
        let section = if matches!(command, ResourceCommand::Fields) {
            "fields"
        } else {
            "schema"
        };
        output::success(metadata.get(section).cloned().unwrap_or(Value::Null));
        return Ok(());
    }

    let config = Config::load(None)?;
    let (profile_name, profile_data) = config.profile(profile)?;
    let client = ResourceClient::new(&profile_data)?;
    match command {
        ResourceCommand::Get { reference, fields } => {
            let id = parse_reference(resource, &reference)?;
            let response =
                client.request_json(Method::GET, &format!("{resource}/{id}.json"), None)?;
            let value = response.get(singular).cloned().unwrap_or(response);
            output::success(select_fields(value, fields.as_deref()));
        }
        ResourceCommand::List {
            project_id,
            user_id,
            assigned_to_id,
            status,
            sort,
            fields,
            limit,
            offset,
        } => {
            let mut query = vec![("limit", limit.to_string()), ("offset", offset.to_string())];
            if let Some(value) = project_id {
                query.push(("project_id", value.to_string()));
            }
            if let Some(value) = user_id {
                query.push(("user_id", value.to_string()));
            }
            if let Some(value) = assigned_to_id {
                query.push(("assigned_to_id", value.to_string()));
            }
            if let Some(value) = status {
                query.push(("status_id", value));
            }
            if let Some(value) = sort {
                query.push(("sort", value));
            }
            let response = client.request_json_with_query(
                Method::GET,
                &format!("{resource}.json"),
                None,
                &query,
            )?;
            let items = response
                .get(resource)
                .cloned()
                .unwrap_or(Value::Array(vec![]));
            let items = items
                .as_array()
                .map(|values| {
                    values
                        .iter()
                        .cloned()
                        .map(|value| select_fields(value, fields.as_deref()))
                        .collect()
                })
                .unwrap_or_default();
            let mut result = json!({"total_count": response.get("total_count"), "limit": limit, "offset": offset, "profile": profile_name});
            result[resource] = Value::Array(items);
            output::success(result);
        }
        ResourceCommand::Create(input) => {
            let payload = read_input(input)?;
            let response = client.request_json(
                Method::POST,
                &format!("{resource}.json"),
                Some(wrap_payload(singular, payload)),
            )?;
            output::success(response.get(singular).cloned().unwrap_or(response));
        }
        ResourceCommand::Update { reference, input } => {
            let id = parse_reference(resource, &reference)?;
            let payload = read_input(input)?;
            let response = client.request_json(
                Method::PUT,
                &format!("{resource}/{id}.json"),
                Some(wrap_payload(singular, payload)),
            )?;
            output::success(response.get(singular).cloned().unwrap_or(response));
        }
        ResourceCommand::Delete { reference } => {
            let id = parse_reference(resource, &reference)?;
            client.request_json(Method::DELETE, &format!("{resource}/{id}.json"), None)?;
            output::success(json!({"deleted": true, "id": id}));
        }
        ResourceCommand::Fields | ResourceCommand::Schema => unreachable!(),
    }
    Ok(())
}

struct ResourceClient {
    client: Client,
    base_url: Url,
    api_key: String,
}

impl ResourceClient {
    fn new(profile: &Profile) -> Result<Self, ResourceError> {
        Ok(Self {
            client: Client::builder().use_rustls_tls().build()?,
            base_url: Url::parse(&profile.url)?,
            api_key: profile.api_key.clone().unwrap_or_default(),
        })
    }

    fn request_json(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<Value, ResourceError> {
        self.request_json_with_query(method, path, body, &[])
    }

    fn request_json_with_query(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
        query: &[(&str, String)],
    ) -> Result<Value, ResourceError> {
        let mut url = self.base_url.join(path)?;
        url.query_pairs_mut()
            .extend_pairs(query.iter().map(|(key, value)| (*key, value.as_str())));
        let mut request = self
            .client
            .request(method, url)
            .header("X-Redmine-API-Key", &self.api_key)
            .header("Accept", "application/json");
        if let Some(body) = body {
            request = request.json(&body);
        }
        let response = request.send()?;
        let status = response.status();
        let text = response.text()?;
        if !status.is_success() {
            return Err(ResourceError::Http {
                status: status.as_u16(),
                body: text,
            });
        }
        if text.is_empty() {
            return Ok(Value::Null);
        }
        Ok(serde_json::from_str(&text)?)
    }
}

fn read_input(input: ResourceInput) -> Result<Value, ResourceError> {
    let text = if let Some(json) = input.json {
        json
    } else if input.stdin {
        let mut text = String::new();
        std::io::stdin()
            .read_to_string(&mut text)
            .map_err(|_| ResourceError::ReadStdin)?;
        text
    } else {
        "{}".to_owned()
    };
    Ok(serde_json::from_str(&text)?)
}

fn parse_reference(resource: &str, value: &str) -> Result<String, ResourceError> {
    if value.starts_with("http://") || value.starts_with("https://") {
        let url = Url::parse(value)?;
        return url
            .path_segments()
            .and_then(|segments| {
                let segments: Vec<_> = segments.collect();
                segments
                    .windows(2)
                    .find(|pair| pair[0] == resource)
                    .map(|pair| pair[1].to_owned())
            })
            .ok_or_else(|| ResourceError::Reference(value.to_owned()));
    }
    if value.is_empty() || value.contains('/') {
        return Err(ResourceError::Reference(value.to_owned()));
    }
    Ok(value.to_owned())
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

fn wrap_payload(key: &str, payload: Value) -> Value {
    let mut body = Map::new();
    body.insert(key.to_owned(), payload);
    Value::Object(body)
}
