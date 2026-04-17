use anyhow::Result;
use clap::Args;
use fsrs::DEFAULT_PARAMETERS;

use crate::output;

#[derive(Args)]
pub struct ParamsArgs {
    /// Show a specific parameter set (comma-separated). If omitted, shows default parameters
    #[arg(short, long, value_delimiter = ',')]
    pub values: Option<Vec<f32>>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

const PARAM_NAMES: [&str; 21] = [
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

pub fn run(args: ParamsArgs) -> Result<()> {
    let params = args.values.unwrap_or_else(|| DEFAULT_PARAMETERS.to_vec());

    output::print_with(&params, args.json, |r| {
        println!("FSRS Parameters ({} values):", r.len());
        for (i, &v) in r.iter().enumerate() {
            let name = if i < PARAM_NAMES.len() {
                PARAM_NAMES[i]
            } else {
                "unknown"
            };
            println!("  [{:2}] {:<38} {:.4}", i, name, v);
        }
    })
}
