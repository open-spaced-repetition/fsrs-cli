use anyhow::{Result, bail};
use clap::{Args, Subcommand};
use fsrs::{FSRS, FSRSItem, FSRSReview, MemoryState};

use crate::config;
use crate::output;

#[derive(Subcommand)]
pub enum MemoryCommand {
    /// Compute memory state from a review history
    State(StateArgs),

    /// Compute historical memory states after each review
    History(HistoryArgs),

    /// Calculate current retrievability
    Retrievability(RetrievabilityArgs),

    /// Convert SM2 parameters to FSRS memory state
    FromSm2(FromSm2Args),
}

#[derive(Args)]
pub struct StateArgs {
    /// Review history as "rating:delta_t" pairs, e.g. "3:0,3:1,3:5,2:10"
    #[arg(short = 'H', long)]
    pub history: String,

    /// Starting stability (for truncated history)
    #[arg(long)]
    pub starting_stability: Option<f32>,

    /// Starting difficulty (for truncated history)
    #[arg(long)]
    pub starting_difficulty: Option<f32>,

    /// FSRS parameters as comma-separated floats. If omitted, uses saved custom parameters when available
    #[arg(short, long, value_delimiter = ',')]
    pub parameters: Option<Vec<f32>>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Args)]
pub struct HistoryArgs {
    /// Review history as "rating:delta_t" pairs, e.g. "3:0,3:1,3:5,2:10"
    #[arg(short = 'H', long)]
    pub history: String,

    /// Starting stability (for truncated history)
    #[arg(long)]
    pub starting_stability: Option<f32>,

    /// Starting difficulty (for truncated history)
    #[arg(long)]
    pub starting_difficulty: Option<f32>,

    /// FSRS parameters as comma-separated floats. If omitted, uses saved custom parameters when available
    #[arg(short, long, value_delimiter = ',')]
    pub parameters: Option<Vec<f32>>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Args)]
pub struct RetrievabilityArgs {
    /// Memory stability
    #[arg(short, long)]
    pub stability: f32,

    /// Memory difficulty
    #[arg(short = 'D', long, default_value_t = 0.0)]
    pub difficulty: f32,

    /// Days elapsed since last review
    #[arg(short, long)]
    pub ivl: f32,

    /// Decay parameter (default: FSRS v6 decay)
    #[arg(long)]
    pub decay: Option<f32>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Args)]
pub struct FromSm2Args {
    /// SM2 ease factor (e.g., 2.5)
    #[arg(short, long)]
    pub ease_factor: f32,

    /// Current interval in days
    #[arg(short, long)]
    pub interval: f32,

    /// SM2 retention rate (e.g., 0.9)
    #[arg(short = 'R', long, default_value_t = 0.9)]
    pub sm2_retention: f32,

    /// FSRS parameters as comma-separated floats. If omitted, uses saved custom parameters when available
    #[arg(short, long, value_delimiter = ',')]
    pub parameters: Option<Vec<f32>>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

pub fn run(cmd: MemoryCommand) -> Result<()> {
    match cmd {
        MemoryCommand::State(args) => run_state(args),
        MemoryCommand::History(args) => run_history(args),
        MemoryCommand::Retrievability(args) => run_retrievability(args),
        MemoryCommand::FromSm2(args) => run_from_sm2(args),
    }
}

fn parse_review_history(input: &str) -> Result<Vec<FSRSReview>> {
    let mut reviews = Vec::new();
    for pair in input.split(',') {
        let parts: Vec<&str> = pair.trim().split(':').collect();
        if parts.len() != 2 {
            bail!(
                "Invalid review format '{}'. Expected 'rating:delta_t'",
                pair
            );
        }
        let rating: u32 = parts[0]
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid rating: {}", parts[0]))?;
        let delta_t: u32 = parts[1]
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid delta_t: {}", parts[1]))?;
        if !(1..=4).contains(&rating) {
            bail!("Rating must be 1-4, got {}", rating);
        }
        reviews.push(FSRSReview { rating, delta_t });
    }
    if reviews.is_empty() {
        bail!("Review history cannot be empty");
    }
    Ok(reviews)
}

fn make_starting_state(
    stability: Option<f32>,
    difficulty: Option<f32>,
) -> Result<Option<MemoryState>> {
    match (stability, difficulty) {
        (Some(s), Some(d)) => Ok(Some(MemoryState {
            stability: s,
            difficulty: d,
        })),
        (None, None) => Ok(None),
        _ => bail!("Both --starting-stability and --starting-difficulty must be provided together"),
    }
}

fn run_state(args: StateArgs) -> Result<()> {
    let reviews = parse_review_history(&args.history)?;
    let starting_state = make_starting_state(args.starting_stability, args.starting_difficulty)?;
    let parameters = config::resolve_parameters(args.parameters)?;
    let fsrs = FSRS::new(&parameters)?;

    let item = FSRSItem { reviews };
    let state = fsrs.memory_state(item, starting_state)?;

    output::print_with(&state, args.json, |r| {
        println!("Memory State:");
        println!("  Stability:  {:.4}", r.stability);
        println!("  Difficulty: {:.4}", r.difficulty);
    })
}

fn run_history(args: HistoryArgs) -> Result<()> {
    let reviews = parse_review_history(&args.history)?;
    let starting_state = make_starting_state(args.starting_stability, args.starting_difficulty)?;
    let parameters = config::resolve_parameters(args.parameters)?;
    let fsrs = FSRS::new(&parameters)?;

    let item = FSRSItem {
        reviews: reviews.clone(),
    };
    let states = fsrs.historical_memory_states(item, starting_state)?;

    output::print_with(&states, args.json, |r| {
        println!("Historical Memory States:");
        println!("{:<6} {:<12} {:<12}", "Index", "Stability", "Difficulty");
        for (i, s) in r.iter().enumerate() {
            println!("{:<6} {:<12.4} {:<12.4}", i, s.stability, s.difficulty);
        }
    })
}

fn run_retrievability(args: RetrievabilityArgs) -> Result<()> {
    let decay = args.decay.unwrap_or(fsrs::FSRS6_DEFAULT_DECAY);
    let state = MemoryState {
        stability: args.stability,
        difficulty: args.difficulty,
    };
    let r = fsrs::current_retrievability(state, args.ivl, decay);

    output::print_with(&r, args.json, |r| {
        println!("Retrievability: {:.4} ({:.1}%)", r, r * 100.0);
    })
}

fn run_from_sm2(args: FromSm2Args) -> Result<()> {
    let parameters = config::resolve_parameters(args.parameters)?;
    let fsrs = FSRS::new(&parameters)?;

    let state = fsrs.memory_state_from_sm2(args.ease_factor, args.interval, args.sm2_retention)?;

    output::print_with(&state, args.json, |r| {
        println!("Converted from SM2:");
        println!("  Stability:  {:.4}", r.stability);
        println!("  Difficulty: {:.4}", r.difficulty);
    })
}
