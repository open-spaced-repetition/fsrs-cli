use anyhow::{Result, bail};
use clap::{Args, Subcommand};
use fsrs::DEFAULT_PARAMETERS;

use crate::config::{self, ConfigKey, ConfigStore, ConfigValue};
use crate::output;

#[derive(Args)]
pub struct ConfigParametersArgs {
    #[command(subcommand)]
    pub command: Option<ConfigParametersCommand>,
}

#[derive(Subcommand)]
pub enum ConfigParametersCommand {
    /// Show active FSRS parameters
    Get(GetParametersArgs),

    /// Save custom FSRS parameters as the CLI default
    Set(SetParametersArgs),

    /// Clear saved custom parameters and revert to built-in defaults
    Reset(ResetParametersArgs),
}

#[derive(Args, Default)]
pub struct GetParametersArgs {
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Args)]
pub struct SetParametersArgs {
    /// Custom FSRS parameters, e.g. "0.5,1.0,..." or "[0.5,1.0,...]"
    pub values: String,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Default)]
pub struct ResetParametersArgs {
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

const PARAMETER_NAMES: [&str; 21] = [
    "initial_stability_again",
    "initial_stability_hard",
    "initial_stability_good",
    "initial_stability_easy",
    "initial_difficulty",
    "difficulty_factor",
    "stability_decay",
    "mean_reversion_weight",
    "success_stability_base",
    "success_stability_decay",
    "success_stability_retrievability",
    "fail_stability_base",
    "fail_stability_difficulty",
    "fail_stability_stability",
    "fail_stability_retrievability",
    "fail_difficulty_factor",
    "fail_stability_factor",
    "short_term_stability_base",
    "short_term_stability_rating",
    "short_term_stability_history",
    "decay",
];

pub fn run(args: ConfigParametersArgs) -> Result<()> {
    match args
        .command
        .unwrap_or(ConfigParametersCommand::Get(GetParametersArgs::default()))
    {
        ConfigParametersCommand::Get(args) => run_get(args),
        ConfigParametersCommand::Set(args) => run_set(args),
        ConfigParametersCommand::Reset(args) => run_reset(args),
    }
}

pub(crate) fn print_parameters(parameters: &[f32]) {
    for (i, &v) in parameters.iter().enumerate() {
        let name = if i < PARAMETER_NAMES.len() {
            PARAMETER_NAMES[i]
        } else {
            "unknown"
        };
        println!("  [{:2}] {:<38} {:.4}", i, name, v);
    }
}

fn run_get(args: GetParametersArgs) -> Result<()> {
    let store = ConfigStore::load()?;
    let path = store.path().to_path_buf();
    let has_saved_parameters = store.has(ConfigKey::Parameters)?;
    let parameters = store.active_parameters()?;

    output::print_with(&parameters, args.json, |r| {
        println!(
            "FSRS Parameters ({} values, source: {}):",
            r.len(),
            config::format_source_label(&path, has_saved_parameters)
        );
        print_parameters(r);
    })
}

fn run_set(args: SetParametersArgs) -> Result<()> {
    let values = parse_parameter_values(&args.values)?;
    let mut store = ConfigStore::load()?;
    store.set(
        ConfigKey::Parameters,
        ConfigValue::Parameters(values.clone()),
    )?;
    let path = store.path().to_path_buf();
    store.save()?;

    output::print_with(&values, args.json, |r| {
        println!("Saved custom FSRS parameters.");
        println!(
            "Config file: {}",
            config::format_display_path_with_status(path.as_path())
        );
        print_parameters(r);
    })
}

fn run_reset(args: ResetParametersArgs) -> Result<()> {
    let mut store = ConfigStore::load()?;
    store.reset(ConfigKey::Parameters);
    let path = store.path().to_path_buf();
    store.save()?;
    let parameters = DEFAULT_PARAMETERS.to_vec();

    output::print_with(&parameters, args.json, |r| {
        println!("Reset FSRS parameters to built-in defaults.");
        println!(
            "Config file: {}",
            config::format_display_path_with_status(path.as_path())
        );
        print_parameters(r);
    })
}

fn parse_parameter_values(input: &str) -> Result<Vec<f32>> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        bail!("parameter list cannot be empty");
    }

    let inner = match (trimmed.starts_with('['), trimmed.ends_with(']')) {
        (true, true) => trimmed[1..trimmed.len() - 1].trim(),
        (true, false) | (false, true) => {
            bail!("parameter list must use matching '[' and ']' brackets")
        }
        (false, false) => trimmed,
    };

    if inner.is_empty() {
        bail!("parameter list cannot be empty");
    }

    inner
        .split(',')
        .map(|value| {
            let value = value.trim();
            if value.is_empty() {
                bail!("parameter list contains an empty value");
            }
            value
                .parse::<f32>()
                .map_err(|_| anyhow::anyhow!("invalid parameter value: {value}"))
        })
        .collect()
}
