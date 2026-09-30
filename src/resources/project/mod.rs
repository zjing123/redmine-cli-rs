use clap::Args;

use crate::resources::common::ResourceCommand;

#[derive(Debug, Args)]
pub struct ProjectCommand {
    #[command(subcommand)]
    pub command: ResourceCommand,
}

pub fn run(
    command: ProjectCommand,
    profile: Option<&str>,
) -> Result<(), crate::resources::common::ResourceError> {
    crate::resources::common::run(
        "projects",
        "project",
        include_str!("metadata.yaml"),
        command.command,
        profile,
    )
}
