# fsrs-cli

A CLI tool for [FSRS](https://github.com/open-spaced-repetition/fsrs-rs) (Free Spaced Repetition Scheduler). Optimize, evaluate, schedule, and simulate spaced repetition from the command line.

## Install

```bash
cargo install --git https://github.com/open-spaced-repetition/fsrs-cli
```

Or build from source:

```bash
git clone https://github.com/open-spaced-repetition/fsrs-cli
cd fsrs-cli
cargo build --release
```

## Quick Start

```bash
# Show active parameters
fsrs config

# Schedule next review for a new card
fsrs schedule --json

# Optimize parameters from review history
fsrs optimize --csv revlog.csv --timezone Asia/Shanghai --day-cutoff 4

# Evaluate model fit
fsrs evaluate --csv revlog.csv --json

# Start HTTP API server
fsrs serve
```

## Commands

### schedule

Get next review states for all four ratings.

```bash
# New card with active parameters and retention
fsrs schedule --json

# Existing card
fsrs schedule -s 10.0 --difficulty 5.0 --ivl 3 --json

# Custom retention
fsrs schedule --retention 0.85 --json
```

### memory

Compute and inspect memory states.

```bash
# Compute memory state from review history
# Format: rating:delta_t (e.g. 3:0 means rating=3, delta_t=0)
fsrs memory state -H "3:0,3:1,3:5,2:10" --json

# Show historical memory states after each review
fsrs memory history -H "3:0,3:1,3:5,2:10" --json

# Calculate retrievability
fsrs memory retrievability -s 10.0 -i 5.0 --json

# Convert SM2 parameters to FSRS
fsrs memory from-sm2 --ease-factor 2.5 --interval 30.0 --json
```

### optimize

Optimize FSRS parameters from review history CSV.

```bash
fsrs optimize --csv revlog.csv --json
fsrs optimize --csv revlog.csv --timezone Asia/Shanghai --day-cutoff 4
```

### benchmark

Fast parameter estimation (no gradient descent).

```bash
fsrs benchmark --csv revlog.csv --json
```

### evaluate

Evaluate model fit against review history.

```bash
fsrs evaluate --csv revlog.csv --json
```

### evaluate-with-time-series-splits

Evaluate with time-series cross-validation.

```bash
fsrs evaluate-with-time-series-splits --csv revlog.csv --json
```

### simulate

Run deck simulation and analysis.

```bash
# Run simulation
fsrs simulate run --deck-size 100 --learn-span 30 --seed 42 --json

# Find optimal retention
fsrs simulate optimal-retention --json

# Estimate workload
fsrs simulate workload --retention 0.9 --json
```

### config

Inspect or change CLI defaults.

```bash
# Show active parameters
# If you've saved custom parameters before, this prints the saved set.
fsrs config --json

# Save custom parameters as the CLI default
fsrs config parameters set 0.5,1.0,2.0,6.0,5.0,0.8,3.0,0.001,1.8,0.2,0.8,1.5,0.06,0.26,1.6,0.6,1.9,0.5,0.09,0.07,0.15

# Bracketed form is also supported
# Quote it in shells like zsh to avoid glob expansion
fsrs config parameters set "[0.5,1.0,2.0,6.0,5.0,0.8,3.0,0.001,1.8,0.2,0.8,1.5,0.06,0.26,1.6,0.6,1.9,0.5,0.09,0.07,0.15]"

# Reset back to built-in defaults
fsrs config parameters reset

# Save custom retention as the CLI default
fsrs config retention set 0.85

# Show saved retention
fsrs config retention get --json

# Reset retention back to built-in defaults
fsrs config retention reset
```

Commands that accept `--parameters` or `--retention` will automatically use saved config defaults when the flag is omitted.

### serve

Start HTTP API server with Swagger docs.

```bash
fsrs serve
fsrs serve --port 8080
```

Open `http://localhost:7320/docs` for Swagger UI. Import `http://localhost:7320/api-docs/openapi.json` into Postman.

The API supports JSON body, CSV file upload (multipart/form-data), and SSE streaming for long-running tasks.

### repl

Interactive mode with tab completion and history.

```bash
fsrs repl
```

## CSV Format

The CSV file follows the [Review Logs Schema](https://github.com/open-spaced-repetition/fsrs-optimizer?tab=readme-ov-file#review-logs-schema) from fsrs-optimizer.

## License

Apache-2.0
