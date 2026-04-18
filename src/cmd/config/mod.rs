mod parameters;
mod retention;

use anyhow::Result;
use clap::{Args, Subcommand};
use serde::Serialize;

use crate::config::{self, ConfigKey, ConfigStore};
use crate::output;

pub use parameters::ConfigParametersArgs;
pub use retention::ConfigRetentionArgs;

#[derive(Args)]
pub struct ConfigArgs {
    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Option<ConfigCommand>,
}

#[derive(Subcommand)]
pub enum ConfigCommand {
    /// Show or change the default FSRS parameters used by the CLI
    #[command(name = "parameters", alias = "params")]
    Parameters(ConfigParametersArgs),

    /// Show or change the default retention used by the CLI
    Retention(ConfigRetentionArgs),
}

#[derive(Serialize)]
struct ConfigOutput {
    config_file: String,
    defaults: ConfigDefaultsOutput,
}

#[derive(Serialize)]
struct ConfigDefaultsOutput {
    parameters: Vec<f32>,
    retention: f32,
}

pub fn run(args: ConfigArgs) -> Result<()> {
    match args.command {
        Some(ConfigCommand::Parameters(args)) => parameters::run(args),
        Some(ConfigCommand::Retention(args)) => retention::run(args),
        None => run_get_config(args.json),
    }
}

fn run_get_config(json: bool) -> Result<()> {
    let store = ConfigStore::load()?;
    let path = store.path().to_path_buf();
    let has_saved_parameters = store.has(ConfigKey::Parameters)?;
    let parameters = store.active_parameters()?;
    let retention = store.active_retention()?;

    let result = ConfigOutput {
        config_file: path.display().to_string(),
        defaults: ConfigDefaultsOutput {
            parameters,
            retention,
        },
    };

    output::print_with(&result, json, |r| {
        println!("CLI Config:");
        println!(
            "  File: {}",
            config::format_display_path_with_status(path.as_path())
        );
        println!(
            "  Parameters: {}",
            config::format_source_label(path.as_path(), has_saved_parameters)
        );
        parameters::print_parameters(&r.defaults.parameters);
        println!("  Retention: {:.2}", r.defaults.retention);
    })
}
