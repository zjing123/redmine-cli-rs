use clap::Args;

use crate::resources::common::ResourceCommand;

#[derive(Debug, Args)]
pub struct TimeEntryCommand {
    #[command(subcommand)]
    pub command: ResourceCommand,
}

pub fn run(
    command: TimeEntryCommand,
    profile: Option<&str>,
) -> Result<(), crate::resources::common::ResourceError> {
    crate::resources::common::run(
        "time_entries",
        "time_entry",
        include_str!("metadata.yaml"),
        command.command,
        profile,
    )
}
