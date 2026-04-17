mod cmd;
mod csv_parser;
mod output;
#[cfg(feature = "serve")]
mod serve;

use anyhow::Result;
use clap::Parser;
use cmd::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    #[cfg(feature = "repl")]
    if matches!(cli.command, Commands::Repl) {
        return cmd::repl::run().await;
    }
    cmd::run(cli.command).await
}
