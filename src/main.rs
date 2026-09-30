use clap::Parser;
use redmine_cli::{cli, output};

fn main() {
    let cli = cli::Cli::parse();
    if let Err(error) = cli::run(cli) {
        output::failure(&error.to_string());
        std::process::exit(1);
    }
}
