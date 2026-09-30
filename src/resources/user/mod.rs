use clap::Args;

use crate::resources::common::ResourceCommand;

#[derive(Debug, Args)]
pub struct UserCommand {
    #[command(subcommand)]
    pub command: ResourceCommand,
}

pub fn run(
    command: UserCommand,
    profile: Option<&str>,
) -> Result<(), crate::resources::common::ResourceError> {
    crate::resources::common::run(
        "users",
        "user",
        include_str!("metadata.yaml"),
        command.command,
        profile,
    )
}
