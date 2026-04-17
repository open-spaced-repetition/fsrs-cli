use anyhow::Result;
use clap::Args;
use fsrs::{CombinedProgressState, ComputeParametersInput, evaluate_with_time_series_splits};
use indicatif::{ProgressBar, ProgressStyle};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::csv_parser::CsvArgs;
use crate::output;

#[derive(Args)]
pub struct EvaluateWithTimeSeriesSplitsArgs {
    /// Path to review history CSV file
    #[arg(short, long)]
    pub csv: PathBuf,

    /// Freeze short-term memory parameters
    #[arg(long)]
    pub freeze_short_term: bool,

    /// Number of relearning steps
    #[arg(long)]
    pub num_relearning_steps: Option<usize>,

    /// IANA timezone name (e.g. "Asia/Shanghai", "America/New_York"). Defaults to UTC
    #[arg(long, default_value = "UTC")]
    pub timezone: String,

    /// Hour at which a new day starts (e.g. 4 means 4:00 AM). Shifts the day boundary for timestamp-based data
    #[arg(long, default_value_t = 0)]
    pub day_cutoff: i64,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

impl CsvArgs for EvaluateWithTimeSeriesSplitsArgs {
    fn csv_path(&self) -> &std::path::Path {
        &self.csv
    }
    fn timezone_str(&self) -> &str {
        &self.timezone
    }
    fn day_cutoff_hours(&self) -> i64 {
        self.day_cutoff
    }
}

#[derive(Serialize)]
struct EvaluateWithTimeSeriesSplitsOutput {
    log_loss: f32,
    rmse_bins: f32,
}

pub fn run(args: EvaluateWithTimeSeriesSplitsArgs) -> Result<()> {
    let items = args.load_items()?;
    let count = items.len();

    if !args.json {
        eprintln!("Loaded {} review items from CSV", count);
        eprintln!("Running time-series cross-validation...");
    }

    let progress = CombinedProgressState::new_shared();

    let input = ComputeParametersInput {
        train_set: items,
        progress: Some(Arc::clone(&progress)),
        enable_short_term: !args.freeze_short_term,
        num_relearning_steps: args.num_relearning_steps,
    };

    let json = args.json;
    let progress_ref = Arc::clone(&progress);
    let handle = thread::spawn(move || evaluate_with_time_series_splits(input, |_| true));

    if !json {
        let pb = ProgressBar::new(100);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} ({msg})")
                .unwrap()
                .progress_chars("#>-"),
        );
        pb.set_message("cross-validating...");

        loop {
            let state = progress_ref.lock().unwrap();
            let total = state.total();
            let current = state.current();
            let finished = state.finished();
            drop(state);

            if total > 0 {
                pb.set_length(total as u64);
                pb.set_position(current as u64);
            }

            if finished || handle.is_finished() {
                pb.finish_with_message("done");
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
    }

    let eval = handle.join().expect("cross-validate thread panicked")?;

    let result = EvaluateWithTimeSeriesSplitsOutput {
        log_loss: eval.log_loss,
        rmse_bins: eval.rmse_bins,
    };

    output::print_with(&result, json, |r| {
        println!("Time-Series Cross-Validation ({} items):", count);
        println!("  Log Loss:  {:.6}", r.log_loss);
        println!("  RMSE Bins: {:.6}", r.rmse_bins);
    })
}
