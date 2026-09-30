use std::{fs::OpenOptions, io::Write, path::PathBuf};

use clap::{Args, Subcommand};
use thiserror::Error;

use crate::{
    config::{Config, ConfigError},
    output,
    redmine_client::{RedmineClient, RedmineClientError},
};

#[derive(Debug, Error)]
pub enum AttachmentError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Client(#[from] RedmineClientError),
    #[error("cannot determine the home directory")]
    HomeDirectory,
    #[error("failed to create download directory '{path}': {source}")]
    CreateDirectory {
        path: String,
        source: std::io::Error,
    },
    #[error("failed to create output file '{path}': {source}")]
    CreateFile {
        path: String,
        source: std::io::Error,
    },
    #[error("failed to write output file '{path}': {source}")]
    WriteFile {
        path: String,
        source: std::io::Error,
    },
}

#[derive(Debug, Args)]
#[command(
    about = "Manage Redmine attachments",
    after_help = "Example:\n  redmine-cli attachment download 391795"
)]
pub struct AttachmentCommand {
    #[command(subcommand)]
    pub command: AttachmentSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum AttachmentSubcommand {
    #[command(
        about = "Download an attachment to a local file",
        after_help = "Examples:\n  redmine-cli -p redminex attachment download 391795\n  redmine-cli -p redminex attachment download 391795 --output ./image.png"
    )]
    Download {
        #[arg(help = "Attachment ID")]
        id: u64,
        #[arg(long, help = "Output file path; defaults to the attachment filename")]
        output: Option<PathBuf>,
    },
}

pub fn run(command: AttachmentCommand, profile: Option<&str>) -> Result<(), AttachmentError> {
    let config = Config::load(None)?;
    let AttachmentSubcommand::Download { id, output } = command.command;
    let (_, profile_data) = config.profile(profile)?;
    let client = RedmineClient::from_profile(&profile_data)?;
    let attachment = client.attachment(id)?;
    let path = if let Some(output) = output {
        output
    } else {
        let home = std::env::var_os("HOME").ok_or(AttachmentError::HomeDirectory)?;
        let downloads = PathBuf::from(home).join("Downloads");
        std::fs::create_dir_all(&downloads).map_err(|source| AttachmentError::CreateDirectory {
            path: downloads.display().to_string(),
            source,
        })?;
        downloads.join(&attachment.filename)
    };
    let data = client.download_attachment(&attachment)?;
    let path_display = path.display().to_string();
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|source| AttachmentError::CreateFile {
            path: path_display.clone(),
            source,
        })?;
    file.write_all(&data)
        .map_err(|source| AttachmentError::WriteFile {
            path: path_display.clone(),
            source,
        })?;
    output::success(serde_json::json!({
        "id": attachment.id,
        "filename": attachment.filename,
        "path": path_display,
        "size": data.len(),
    }));
    Ok(())
}
