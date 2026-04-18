use anyhow::Result;
use clap::{Args, Subcommand};
use fsrs::{SimulatorConfig, expected_workload, optimal_retention, simulate};
use serde::Serialize;

use crate::config;
use crate::output;

#[derive(Subcommand)]
pub enum SimulateCommand {
    /// Run a deck simulation
    Run(SimulateRunArgs),

    /// Find the optimal retention rate
    OptimalRetention(OptimalRetentionArgs),

    /// Estimate expected workload for new cards
    Workload(WorkloadArgs),
}

#[derive(Args)]
pub struct SimulateRunArgs {
    /// Total number of cards in the deck
    #[arg(long, default_value_t = 10000)]
    pub deck_size: usize,

    /// Simulation duration in days
    #[arg(long, default_value_t = 365)]
    pub learn_span: usize,

    /// Maximum time cost per day in seconds
    #[arg(long, default_value_t = 1800.0)]
    pub max_cost_perday: f32,

    /// Maximum interval in days
    #[arg(long, default_value_t = 36500.0)]
    pub max_ivl: f32,

    /// Desired retention rate (0.70-0.95)
    #[arg(short, long)]
    pub retention: Option<f32>,

    /// New cards per day limit
    #[arg(long)]
    pub learn_limit: Option<usize>,

    /// Review cards per day limit
    #[arg(long)]
    pub review_limit: Option<usize>,

    /// Random seed for reproducibility
    #[arg(long)]
    pub seed: Option<u64>,

    /// FSRS parameters as comma-separated floats. If omitted, uses saved custom parameters when available
    #[arg(short, long, value_delimiter = ',')]
    pub parameters: Option<Vec<f32>>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Args)]
pub struct OptimalRetentionArgs {
    /// Total number of cards in the deck
    #[arg(long, default_value_t = 10000)]
    pub deck_size: usize,

    /// Simulation duration in days
    #[arg(long, default_value_t = 365)]
    pub learn_span: usize,

    /// Maximum time cost per day in seconds
    #[arg(long, default_value_t = 1800.0)]
    pub max_cost_perday: f32,

    /// Maximum interval in days
    #[arg(long, default_value_t = 36500.0)]
    pub max_ivl: f32,

    /// New cards per day limit
    #[arg(long)]
    pub learn_limit: Option<usize>,

    /// Review cards per day limit
    #[arg(long)]
    pub review_limit: Option<usize>,

    /// FSRS parameters as comma-separated floats. If omitted, uses saved custom parameters when available
    #[arg(short, long, value_delimiter = ',')]
    pub parameters: Option<Vec<f32>>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Args)]
pub struct WorkloadArgs {
    /// Desired retention rate (0.70-0.95)
    #[arg(short, long)]
    pub retention: Option<f32>,

    /// Total number of cards in the deck
    #[arg(long, default_value_t = 10000)]
    pub deck_size: usize,

    /// Simulation duration in days
    #[arg(long, default_value_t = 365)]
    pub learn_span: usize,

    /// Maximum time cost per day in seconds
    #[arg(long, default_value_t = 1800.0)]
    pub max_cost_perday: f32,

    /// New cards per day limit
    #[arg(long)]
    pub learn_limit: Option<usize>,

    /// Review cards per day limit
    #[arg(long)]
    pub review_limit: Option<usize>,

    /// FSRS parameters as comma-separated floats. If omitted, uses saved custom parameters when available
    #[arg(short, long, value_delimiter = ',')]
    pub parameters: Option<Vec<f32>>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Serialize)]
struct SimulateOutput {
    total_reviews: usize,
    total_learned: usize,
    avg_reviews_per_day: f32,
    avg_cost_per_day: f32,
    total_cost: f32,
    final_memorized: f32,
    days: usize,
}

#[derive(Serialize)]
struct OptimalRetentionOutput {
    optimal_retention: f32,
}

#[derive(Serialize)]
struct WorkloadOutput {
    expected_workload: f32,
    retention: f32,
}

pub fn run(cmd: SimulateCommand) -> Result<()> {
    match cmd {
        SimulateCommand::Run(args) => run_simulate(args),
        SimulateCommand::OptimalRetention(args) => run_optimal_retention(args),
        SimulateCommand::Workload(args) => run_workload(args),
    }
}

fn build_config(
    deck_size: usize,
    learn_span: usize,
    max_cost_perday: f32,
    max_ivl: f32,
    learn_limit: Option<usize>,
    review_limit: Option<usize>,
) -> SimulatorConfig {
    SimulatorConfig {
        deck_size,
        learn_span,
        max_cost_perday,
        max_ivl,
        learn_limit: learn_limit.unwrap_or(usize::MAX),
        review_limit: review_limit.unwrap_or(usize::MAX),
        ..Default::default()
    }
}

fn run_simulate(args: SimulateRunArgs) -> Result<()> {
    let parameters = config::resolve_parameters(args.parameters)?;
    let retention = config::resolve_retention(args.retention)?;
    let config = build_config(
        args.deck_size,
        args.learn_span,
        args.max_cost_perday,
        args.max_ivl,
        args.learn_limit,
        args.review_limit,
    );

    if !args.json {
        eprintln!(
            "Running simulation: {} cards, {} days, {:.0}% retention...",
            args.deck_size,
            args.learn_span,
            retention * 100.0
        );
    }

    let result = simulate(&config, &parameters, retention, args.seed, None)?;

    let total_reviews: usize = result.review_cnt_per_day.iter().sum();
    let total_learned: usize = result.learn_cnt_per_day.iter().sum();
    let total_cost: f32 = result.cost_per_day.iter().sum();
    let days = result.review_cnt_per_day.len();
    let final_memorized = *result.memorized_cnt_per_day.last().unwrap_or(&0.0);

    let out = SimulateOutput {
        total_reviews,
        total_learned,
        avg_reviews_per_day: if days > 0 {
            total_reviews as f32 / days as f32
        } else {
            0.0
        },
        avg_cost_per_day: if days > 0 {
            total_cost / days as f32
        } else {
            0.0
        },
        total_cost,
        final_memorized,
        days,
    };

    output::print_with(&out, args.json, |r| {
        println!("Simulation Results ({} days):", r.days);
        println!("  Total reviews:       {}", r.total_reviews);
        println!("  Total learned:       {}", r.total_learned);
        println!("  Final memorized:     {:.0}", r.final_memorized);
        println!("  Avg reviews/day:     {:.1}", r.avg_reviews_per_day);
        println!(
            "  Avg cost/day:        {:.1}s ({:.1}min)",
            r.avg_cost_per_day,
            r.avg_cost_per_day / 60.0
        );
        println!(
            "  Total cost:          {:.0}s ({:.1}hr)",
            r.total_cost,
            r.total_cost / 3600.0
        );
    })
}

fn run_optimal_retention(args: OptimalRetentionArgs) -> Result<()> {
    let parameters = config::resolve_parameters(args.parameters)?;
    let config = build_config(
        args.deck_size,
        args.learn_span,
        args.max_cost_perday,
        args.max_ivl,
        args.learn_limit,
        args.review_limit,
    );

    if !args.json {
        eprintln!("Searching for optimal retention...");
    }

    let retention = optimal_retention(&config, &parameters, |_| true, None, None)?;

    let result = OptimalRetentionOutput {
        optimal_retention: retention,
    };

    output::print_with(&result, args.json, |r| {
        println!(
            "Optimal Retention: {:.4} ({:.1}%)",
            r.optimal_retention,
            r.optimal_retention * 100.0
        );
    })
}

fn run_workload(args: WorkloadArgs) -> Result<()> {
    let parameters = config::resolve_parameters(args.parameters)?;
    let retention = config::resolve_retention(args.retention)?;
    let config = build_config(
        args.deck_size,
        args.learn_span,
        args.max_cost_perday,
        36500.0,
        args.learn_limit,
        args.review_limit,
    );

    let workload = expected_workload(&parameters, retention, &config)?;

    let result = WorkloadOutput {
        expected_workload: workload,
        retention,
    };

    output::print_with(&result, args.json, |r| {
        println!("Expected Workload:");
        println!("  Retention: {:.1}%", r.retention * 100.0);
        println!("  Workload:  {:.2}s per new card", r.expected_workload);
    })
}
