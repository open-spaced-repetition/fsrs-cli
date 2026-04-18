use anyhow::Result;
use clap::Args;
use fsrs::{FSRS, ItemProgress};
use indicatif::{ProgressBar, ProgressStyle};
use serde::Serialize;
use std::path::PathBuf;

use crate::config;
use crate::csv_parser::CsvArgs;
use crate::output;

#[derive(Args)]
pub struct EvaluateArgs {
    /// Path to review history CSV file
    #[arg(short, long)]
    pub csv: PathBuf,

    /// FSRS parameters to evaluate (comma-separated). If omitted, uses saved custom parameters when available
    #[arg(short, long, value_delimiter = ',')]
    pub parameters: Option<Vec<f32>>,

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

impl CsvArgs for EvaluateArgs {
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
struct EvaluateOutput {
    log_loss: f32,
    rmse_bins: f32,
}

fn make_progress_cb(json: bool) -> (Option<ProgressBar>, impl FnMut(ItemProgress) -> bool) {
    let pb = if json {
        None
    } else {
        let pb = ProgressBar::new(0);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} items")
                .unwrap()
                .progress_chars("#>-"),
        );
        Some(pb)
    };

    let pb_clone = pb.clone();
    let callback = move |p: ItemProgress| {
        if let Some(ref pb) = pb_clone {
            pb.set_length(p.total as u64);
            pb.set_position(p.current as u64);
        }
        true
    };
    (pb, callback)
}

pub fn run(args: EvaluateArgs) -> Result<()> {
    let items = args.load_items()?;
    let count = items.len();

    if !args.json {
        eprintln!("Loaded {} review items from CSV", count);
    }

    let parameters = config::resolve_parameters(args.parameters.clone())?;
    let fsrs = FSRS::new(&parameters)?;

    let (pb, progress_cb) = make_progress_cb(args.json);
    let eval = fsrs.evaluate(items, progress_cb)?;
    if let Some(pb) = pb {
        pb.finish_and_clear();
    }

    let result = EvaluateOutput {
        log_loss: eval.log_loss,
        rmse_bins: eval.rmse_bins,
    };

    output::print_with(&result, args.json, |r| {
        println!("Model Evaluation ({} items):", count);
        println!("  Log Loss:  {:.6}", r.log_loss);
        println!("  RMSE Bins: {:.6}", r.rmse_bins);
    })
}
