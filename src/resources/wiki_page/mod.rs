use clap::Args;

use crate::resources::common::ResourceCommand;

#[derive(Debug, Args)]
#[command(about = "Manage project wiki pages")]
pub struct WikiPageCommand {
    #[command(subcommand)]
    pub command: ResourceCommand,
}

pub fn run(
    command: WikiPageCommand,
    profile: Option<&str>,
) -> Result<(), crate::resources::common::ResourceError> {
    crate::resources::common::run(
        "wiki_pages",
        "wiki_page",
        include_str!("metadata.yaml"),
        command.command,
        profile,
    )
}
