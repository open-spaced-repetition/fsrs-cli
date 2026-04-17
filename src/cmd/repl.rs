use anyhow::Result;
use clap::{CommandFactory, Parser};
use rustyline::completion::{Completer, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{CompletionType, Config, Context, Editor, Helper};

use super::Cli;

struct FsrsHelper {
    /// (command_name, subcommand_names)
    commands: Vec<(String, Vec<String>)>,
}

impl FsrsHelper {
    fn new() -> Self {
        let cmd = Cli::command();
        let commands = cmd
            .get_subcommands()
            .map(|sub| {
                let name = sub.get_name().to_string();
                let children: Vec<String> = sub
                    .get_subcommands()
                    .map(|c| c.get_name().to_string())
                    .collect();
                (name, children)
            })
            .chain([("quit".to_string(), vec![]), ("exit".to_string(), vec![])])
            .collect();
        Self { commands }
    }

    fn match_candidates(names: &[String], prefix: &str) -> Vec<Pair> {
        names
            .iter()
            .filter(|n| n.starts_with(prefix))
            .map(|n| Pair {
                display: n.to_string(),
                replacement: n.to_string(),
            })
            .collect()
    }
}

impl Completer for FsrsHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let line = &line[..pos];
        let parts: Vec<&str> = line.split_whitespace().collect();
        let top_names: Vec<String> = self.commands.iter().map(|(n, _)| n.clone()).collect();

        match parts.len() {
            0 => Ok((0, Self::match_candidates(&top_names, ""))),
            1 if !line.ends_with(' ') => {
                let prefix = parts[0];
                Ok((pos - prefix.len(), Self::match_candidates(&top_names, prefix)))
            }
            _ => {
                let cmd = parts[0];
                if let Some((_, children)) = self.commands.iter().find(|(n, _)| n == cmd) {
                    if children.is_empty() {
                        return Ok((pos, vec![]));
                    }
                    let prefix = if line.ends_with(' ') {
                        ""
                    } else {
                        parts.last().unwrap_or(&"")
                    };
                    let start = pos - prefix.len();
                    Ok((start, Self::match_candidates(children, prefix)))
                } else {
                    Ok((pos, vec![]))
                }
            }
        }
    }
}

impl Hinter for FsrsHelper {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, _ctx: &Context<'_>) -> Option<String> {
        if pos < line.len() {
            return None;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();

        let (names, prefix): (Vec<&String>, &str) = match parts.len() {
            0 => return None,
            1 if !line.ends_with(' ') => {
                (self.commands.iter().map(|(n, _)| n).collect(), parts[0])
            }
            2 if !line.ends_with(' ') => {
                let cmd = parts[0];
                match self.commands.iter().find(|(n, _)| n == cmd) {
                    Some((_, children)) if !children.is_empty() => {
                        (children.iter().collect(), parts[1])
                    }
                    _ => return None,
                }
            }
            _ => return None,
        };

        names
            .iter()
            .find(|n| n.starts_with(prefix) && n.as_str() != prefix)
            .map(|n| n[prefix.len()..].to_string())
    }
}

impl Highlighter for FsrsHelper {
    fn highlight_hint<'h>(&self, hint: &'h str) -> std::borrow::Cow<'h, str> {
        std::borrow::Cow::Owned(format!("\x1b[90m{hint}\x1b[0m"))
    }
}
impl Validator for FsrsHelper {}
impl Helper for FsrsHelper {}

fn history_path() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(|h| std::path::PathBuf::from(h).join(".fsrs_history"))
}

pub async fn run() -> Result<()> {
    eprintln!("FSRS interactive mode. Tab for completion, 'quit' or Ctrl+D to exit.");
    eprintln!();

    let config = Config::builder()
        .completion_type(CompletionType::List)
        .build();
    let mut rl = Editor::with_config(config)?;
    rl.set_helper(Some(FsrsHelper::new()));

    if let Some(path) = history_path() {
        let _ = std::fs::create_dir_all(path.parent().unwrap());
        let _ = rl.load_history(&path);
    }

    loop {
        let line = match rl.readline("fsrs> ") {
            Ok(line) => line,
            Err(
                rustyline::error::ReadlineError::Eof | rustyline::error::ReadlineError::Interrupted,
            ) => {
                eprintln!("Bye!");
                break;
            }
            Err(e) => {
                eprintln!("Error: {e}");
                break;
            }
        };

        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let _ = rl.add_history_entry(line);

        if matches!(line, "quit" | "exit") {
            eprintln!("Bye!");
            break;
        }

        let args = match shellwords::split(line) {
            Ok(args) => args,
            Err(e) => {
                eprintln!("Error: {e}");
                continue;
            }
        };

        let mut full_args = vec!["fsrs".to_string()];
        full_args.extend(args);

        match Cli::try_parse_from(&full_args) {
            Ok(cli) => {
                if matches!(cli.command, super::Commands::Repl) {
                    eprintln!("Already in REPL mode.");
                } else if matches!(cli.command, super::Commands::Serve(_)) {
                    eprintln!("Cannot start server from REPL. Run 'fsrs serve' directly.");
                } else if let Err(e) = super::run(cli.command).await {
                    eprintln!("Error: {e}");
                }
            }
            Err(e) => {
                let _ = e.print();
            }
        }
    }

    if let Some(path) = history_path() {
        let _ = rl.save_history(&path);
    }

    Ok(())
}
