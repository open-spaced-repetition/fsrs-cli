use anyhow::Result;
use clap::{Args, Subcommand};

use crate::config::{self, ConfigKey, ConfigStore, ConfigValue};
use crate::output;

#[derive(Args)]
pub struct ConfigRetentionArgs {
    #[command(subcommand)]
    pub command: Option<ConfigRetentionCommand>,
}

#[derive(Subcommand)]
pub enum ConfigRetentionCommand {
    /// Show active retention
    Get(GetRetentionArgs),

    /// Save custom retention as the CLI default
    Set(SetRetentionArgs),

    /// Clear saved retention and revert to built-in defaults
    Reset(ResetRetentionArgs),
}

#[derive(Args, Default)]
pub struct GetRetentionArgs {
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Args)]
pub struct SetRetentionArgs {
    /// Retention value, e.g. 0.9
    pub value: f32,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Default)]
pub struct ResetRetentionArgs {
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

pub fn run(args: ConfigRetentionArgs) -> Result<()> {
    match args
        .command
        .unwrap_or(ConfigRetentionCommand::Get(GetRetentionArgs::default()))
    {
        ConfigRetentionCommand::Get(args) => run_get(args),
        ConfigRetentionCommand::Set(args) => run_set(args),
        ConfigRetentionCommand::Reset(args) => run_reset(args),
    }
}

fn run_get(args: GetRetentionArgs) -> Result<()> {
    let store = ConfigStore::load()?;
    let path = store.path().to_path_buf();
    let has_saved_retention = store.has(ConfigKey::Retention)?;
    let retention = store.active_retention()?;

    output::print_with(&retention, args.json, |r| match r {
        retention => {
            println!(
                "Retention: {:.4} (source: {})",
                retention,
                config::format_source_label(path.as_path(), has_saved_retention)
            );
        }
    })
}

fn run_set(args: SetRetentionArgs) -> Result<()> {
    let mut store = ConfigStore::load()?;
    store.set(ConfigKey::Retention, ConfigValue::Retention(args.value))?;
    let path = store.path().to_path_buf();
    store.save()?;

    output::print_with(&args.value, args.json, |r| {
        println!("Saved custom retention.");
        println!(
            "Config file: {}",
            config::format_display_path_with_status(path.as_path())
        );
        println!("Retention: {:.1}", r);
    })
}

fn run_reset(args: ResetRetentionArgs) -> Result<()> {
    let mut store = ConfigStore::load()?;
    store.reset(ConfigKey::Retention);
    let path = store.path().to_path_buf();
    store.save()?;

    output::print_with(&serde_json::Value::Null, args.json, |_| {
        println!("Reset retention to built-in defaults.");
        println!(
            "Config file: {}",
            config::format_display_path_with_status(path.as_path())
        );
    })
}
