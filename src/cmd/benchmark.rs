use anyhow::Result;
use clap::Args;
use fsrs::{CombinedProgressState, ComputeParametersInput, benchmark};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use crate::csv_parser::CsvArgs;
use crate::output;

#[derive(Args)]
pub struct BenchmarkArgs {
    /// Path to review history CSV file
    #[arg(short, long)]
    pub csv: PathBuf,

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

impl CsvArgs for BenchmarkArgs {
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

pub fn run(args: BenchmarkArgs) -> Result<()> {
    let items = args.load_items()?;

    if !args.json {
        eprintln!("Loaded {} review items from CSV", items.len());
    }

    let input = ComputeParametersInput {
        train_set: items,
        progress: Some(CombinedProgressState::new_shared()),
        enable_short_term: true,
        num_relearning_steps: None,
    };

    let handle = thread::spawn(move || benchmark(input));

    if !args.json {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap(),
        );
        pb.set_message("Running benchmark...");
        pb.enable_steady_tick(Duration::from_millis(100));
        while !handle.is_finished() {
            thread::sleep(Duration::from_millis(50));
        }
        pb.finish_and_clear();
    }

    let parameters = handle.join().expect("benchmark thread panicked");

    output::print_with(&parameters, args.json, |r| {
        println!("Benchmark Parameters:");
        println!(
            "[{}]",
            r.iter()
                .map(|p| format!("{:.4}", p))
                .collect::<Vec<_>>()
                .join(", ")
        );
    })
}
