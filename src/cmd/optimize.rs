use anyhow::Result;
use clap::Args;
use fsrs::{CombinedProgressState, ComputeParametersInput, compute_parameters};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::csv_parser::CsvArgs;
use crate::output;

#[derive(Args)]
pub struct OptimizeArgs {
    /// Path to review history CSV file
    #[arg(short, long)]
    pub csv: PathBuf,

    /// Freeze short-term memory parameters (disable short-term optimization)
    #[arg(long)]
    pub freeze_short_term: bool,

    /// Number of relearning steps
    #[arg(long)]
    pub num_relearning_steps: Option<usize>,

    /// IANA timezone name (e.g. "Asia/Shanghai", "America/New_York"). Defaults to UTC
    #[arg(long, default_value = "UTC")]
    pub timezone: String,

    /// Hour at which a new day starts (e.g. 4 means 4:00 AM)
    #[arg(long, default_value_t = 0)]
    pub day_cutoff: i64,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

impl CsvArgs for OptimizeArgs {
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

pub fn run(args: OptimizeArgs) -> Result<()> {
    let items = args.load_items()?;

    if !args.json {
        eprintln!("Loaded {} review items from CSV", items.len());
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
    let handle = thread::spawn(move || compute_parameters(input));

    if !json {
        let pb = ProgressBar::new(100);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} ({msg})")
                .unwrap()
                .progress_chars("#>-"),
        );
        pb.set_message("optimizing...");

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

    let parameters = handle
        .join()
        .map_err(|_| anyhow::anyhow!("optimize thread panicked"))??;

    output::print_with(&parameters, json, |r| {
        println!("Optimized Parameters:");
        println!(
            "[{}]",
            r.iter()
                .map(|p| format!("{:.4}", p))
                .collect::<Vec<_>>()
                .join(", ")
        );
    })
}
