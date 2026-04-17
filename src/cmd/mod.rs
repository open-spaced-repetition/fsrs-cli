mod benchmark;
mod evaluate;
mod evaluate_with_time_series_splits;
mod memory;
mod optimize;
mod params;
#[cfg(feature = "repl")]
pub mod repl;
mod schedule;
mod simulate;

use clap::{Parser, Subcommand};

#[cfg(feature = "serve")]
use crate::serve;

#[derive(Parser)]
#[command(
    name = "fsrs",
    version,
    about = "A CLI tool for FSRS (Free Spaced Repetition Scheduler)",
    long_about = "Optimize, schedule, evaluate, and simulate spaced repetition using the FSRS algorithm.\n\n\
                  Supports CSV-based parameter optimization and evaluation, memory state computation,\n\
                  scheduling with both date and interval modes, deck simulation, and an HTTP API server."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Schedule next review states for all four ratings
    Schedule(schedule::NextStatesArgs),

    /// Compute and inspect memory states
    #[command(subcommand)]
    Memory(memory::MemoryCommand),

    /// Optimize FSRS parameters from review history CSV
    Optimize(optimize::OptimizeArgs),

    /// Fast parameter estimation from review history CSV
    Benchmark(benchmark::BenchmarkArgs),

    /// Evaluate model fit against review history CSV
    Evaluate(evaluate::EvaluateArgs),

    /// Evaluate with time-series cross-validation (train/test splits)
    EvaluateWithTimeSeriesSplits(
        evaluate_with_time_series_splits::EvaluateWithTimeSeriesSplitsArgs,
    ),

    /// Run deck simulation and analysis
    #[command(subcommand)]
    Simulate(simulate::SimulateCommand),

    /// Show or inspect FSRS parameters
    Params(params::ParamsArgs),

    /// Start HTTP API server with OpenAPI docs and SSE support
    #[cfg(feature = "serve")]
    Serve(crate::serve::ServeArgs),

    /// Interactive REPL mode
    #[cfg(feature = "repl")]
    #[command(name = "repl", alias = "interactive")]
    Repl,
}

pub async fn run(cmd: Commands) -> anyhow::Result<()> {
    match cmd {
        Commands::Schedule(args) => schedule::run(args),
        Commands::Memory(cmd) => memory::run(cmd),
        Commands::Optimize(args) => optimize::run(args),
        Commands::Benchmark(args) => benchmark::run(args),
        Commands::Evaluate(args) => evaluate::run(args),
        Commands::EvaluateWithTimeSeriesSplits(args) => evaluate_with_time_series_splits::run(args),
        Commands::Simulate(cmd) => simulate::run(cmd),
        Commands::Params(args) => params::run(args),
        #[cfg(feature = "serve")]
        Commands::Serve(args) => serve::start(&args.host, args.port).await,
        #[cfg(feature = "repl")]
        Commands::Repl => unreachable!(),
    }
}
