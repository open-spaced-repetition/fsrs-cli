use anyhow::{Result, bail};
use clap::Args;
use fsrs::{FSRS, MemoryState};

use crate::config;
use crate::output;

#[derive(Args)]
pub struct NextStatesArgs {
    /// Desired retention rate (0.70-0.95)
    #[arg(short, long)]
    pub retention: Option<f32>,

    /// Days elapsed since last review
    #[arg(short, long, default_value_t = 0)]
    pub ivl: u32,

    /// Current stability (omit for new card)
    #[arg(short, long)]
    pub stability: Option<f32>,

    /// Current difficulty (omit for new card)
    #[arg(long)]
    pub difficulty: Option<f32>,

    /// FSRS parameters as comma-separated floats. If omitted, uses saved custom parameters when available
    #[arg(short, long, value_delimiter = ',')]
    pub parameters: Option<Vec<f32>>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

pub fn run(args: NextStatesArgs) -> Result<()> {
    let parameters = config::resolve_parameters(args.parameters)?;
    let retention = config::resolve_retention(args.retention)?;
    let fsrs = FSRS::new(&parameters)?;

    let current_state = match (args.stability, args.difficulty) {
        (Some(s), Some(d)) => Some(MemoryState {
            stability: s,
            difficulty: d,
        }),
        (None, None) => None,
        _ => bail!(
            "Both --stability and --difficulty must be provided together, or omit both for a new card"
        ),
    };

    let states = fsrs.next_states(current_state, retention, args.ivl)?;

    output::print_with(&states, args.json, |r| {
        println!(
            "Next States (retention={:.0}%, elapsed={}d):",
            retention * 100.0,
            args.ivl
        );
        println!(
            "  Again: interval={:.1}d, stability={:.4}, difficulty={:.4}",
            r.again.interval, r.again.memory.stability, r.again.memory.difficulty
        );
        println!(
            "  Hard:  interval={:.1}d, stability={:.4}, difficulty={:.4}",
            r.hard.interval, r.hard.memory.stability, r.hard.memory.difficulty
        );
        println!(
            "  Good:  interval={:.1}d, stability={:.4}, difficulty={:.4}",
            r.good.interval, r.good.memory.stability, r.good.memory.difficulty
        );
        println!(
            "  Easy:  interval={:.1}d, stability={:.4}, difficulty={:.4}",
            r.easy.interval, r.easy.memory.stability, r.easy.memory.difficulty
        );
    })
}
