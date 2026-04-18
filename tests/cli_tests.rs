use assert_cmd::Command;
use predicates::prelude::*;
use serde::Deserialize;
use std::io::Write;
use std::path::Path;
use tempfile::{NamedTempFile, TempDir};

fn cmd() -> Command {
    Command::cargo_bin("fsrs").unwrap()
}

fn cmd_with_config(config_dir: &Path) -> Command {
    let mut command = cmd();
    command
        .env("FSRS_CLI_CONFIG_DIR", config_dir)
        .env("XDG_CONFIG_HOME", config_dir)
        .env("HOME", config_dir)
        .env("APPDATA", config_dir)
        .env("LOCALAPPDATA", config_dir);
    command
}

fn run_json<T: for<'de> Deserialize<'de>>(args: &[&str]) -> T {
    let output = cmd().args(args).output().unwrap();
    assert!(output.status.success(), "Command failed: {:?}", args);
    serde_json::from_slice(&output.stdout).expect("Invalid JSON output")
}

fn run_json_with_config<T: for<'de> Deserialize<'de>>(config_dir: &Path, args: &[&str]) -> T {
    let output = cmd_with_config(config_dir).args(args).output().unwrap();
    assert!(output.status.success(), "Command failed: {:?}", args);
    serde_json::from_slice(&output.stdout).expect("Invalid JSON output")
}

fn assert_approx(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-3,
        "expected {expected}, got {actual}"
    );
}

#[derive(Deserialize)]
struct MemoryState {
    stability: f64,
    difficulty: f64,
}

#[derive(Deserialize)]
struct ItemState {
    memory: MemoryState,
    interval: f64,
}

#[derive(Deserialize)]
struct NextStates {
    again: ItemState,
    hard: ItemState,
    good: ItemState,
    easy: ItemState,
}

#[derive(Deserialize)]
struct EvaluateOutput {
    log_loss: f64,
    rmse_bins: f64,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct SimulateOutput {
    total_reviews: u64,
    total_learned: u64,
    avg_reviews_per_day: f64,
    avg_cost_per_day: f64,
    total_cost: f64,
    final_memorized: f64,
    days: u64,
}

#[derive(Deserialize)]
struct WorkloadOutput {
    expected_workload: f64,
    retention: f64,
}

#[derive(Deserialize)]
struct ConfigOutput {
    config_file: String,
    defaults: ConfigDefaultsOutput,
}

#[derive(Deserialize)]
struct ConfigDefaultsOutput {
    parameters: Vec<f64>,
    retention: f64,
}

// ============================================================
// CLI argument parsing tests
// ============================================================

#[test]
fn test_help_output() {
    cmd().arg("--help").assert().success().stdout(
        predicate::str::contains("schedule")
            .and(predicate::str::contains("memory"))
            .and(predicate::str::contains("optimize"))
            .and(predicate::str::contains("evaluate"))
            .and(predicate::str::contains("simulate"))
            .and(predicate::str::contains("config")),
    );
}

#[test]
fn test_version_output() {
    cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("fsrs"));
}

#[test]
fn test_no_args_shows_help() {
    cmd()
        .assert()
        .failure()
        .stderr(predicate::str::contains("Usage"));
}

// ============================================================
// Schedule command tests
// ============================================================

#[test]
fn test_next_states_new_card() {
    let v: NextStates = run_json(&["schedule", "--json"]);
    assert_approx(v.again.memory.stability, 0.212);
    assert_approx(v.again.memory.difficulty, 6.4133);
    assert_approx(v.hard.memory.stability, 1.2931);
    assert_approx(v.good.memory.stability, 2.3065);
    assert_approx(v.easy.memory.stability, 8.2956);
    // new card at 0.9 retention: interval == stability
    assert_approx(v.again.interval, 0.212);
    assert_approx(v.good.interval, 2.3065);
}

#[test]
fn test_next_states_existing_card() {
    let v: NextStates = run_json(&[
        "schedule",
        "-s",
        "10.0",
        "--difficulty",
        "5.0",
        "--ivl",
        "3",
        "--json",
    ]);
    assert_approx(v.again.memory.stability, 1.2588);
    assert_approx(v.hard.memory.stability, 15.038);
    assert_approx(v.good.memory.stability, 18.3772);
    assert_approx(v.easy.memory.stability, 25.6897);
    assert_approx(v.easy.interval, 25.6897);
}

#[test]
fn test_next_states_custom_retention() {
    let v: NextStates = run_json(&["schedule", "--retention", "0.85", "--json"]);
    // at 0.85 retention, intervals > stability
    assert_approx(v.again.interval, 0.4042);
    assert_approx(v.good.interval, 4.3972);
}

#[test]
fn test_next_states_human_readable() {
    cmd()
        .args([
            "schedule",
            "-s",
            "10.0",
            "--difficulty",
            "5.0",
            "--ivl",
            "3",
        ])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Next States")
                .and(predicate::str::contains("Again"))
                .and(predicate::str::contains("Good")),
        );
}

// ============================================================
// Invalid input tests
// ============================================================

#[test]
fn test_next_states_stability_without_difficulty() {
    cmd()
        .args(["schedule", "--stability", "10.0"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Both --stability and --difficulty",
        ));
}

// ============================================================
// Memory command tests
// ============================================================

#[test]
fn test_memory_state() {
    let v: MemoryState = run_json(&["memory", "state", "-H", "3:0,3:1,3:5,2:10", "--json"]);
    assert_approx(v.stability, 46.781);
    assert_approx(v.difficulty, 4.7437);
}

#[test]
fn test_memory_state_with_starting_state() {
    let v: MemoryState = run_json(&[
        "memory",
        "state",
        "-H",
        "3:5,2:10",
        "--starting-stability",
        "5.0",
        "--starting-difficulty",
        "5.0",
        "--json",
    ]);
    assert_approx(v.stability, 31.1913);
    assert_approx(v.difficulty, 6.6595);
}

#[test]
fn test_memory_history() {
    let v: Vec<MemoryState> = run_json(&["memory", "history", "-H", "3:0,3:1,3:5,2:10", "--json"]);
    assert_eq!(v.len(), 4);
    assert_approx(v[0].stability, 2.3065);
    assert_approx(v[0].difficulty, 2.118);
    assert_approx(v[3].stability, 46.781);
    assert_approx(v[3].difficulty, 4.7437);
}

#[test]
fn test_memory_history_human_readable() {
    cmd()
        .args(["memory", "history", "-H", "3:0,3:1,3:5,2:10"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Historical Memory States")
                .and(predicate::str::contains("Index")),
        );
}

#[test]
fn test_retrievability() {
    let v: f64 = run_json(&[
        "memory",
        "retrievability",
        "-s",
        "10.0",
        "-i",
        "5.0",
        "--json",
    ]);
    assert_approx(v, 0.9403);
}

#[test]
fn test_from_sm2() {
    let v: MemoryState = run_json(&[
        "memory",
        "from-sm2",
        "--ease-factor",
        "2.5",
        "--interval",
        "30.0",
        "--json",
    ]);
    assert_approx(v.stability, 30.0);
    assert_approx(v.difficulty, 6.0934);
}

#[test]
fn test_memory_state_invalid_history() {
    cmd()
        .args(["memory", "state", "-H", "invalid"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid review format"));
}

#[test]
fn test_memory_state_invalid_rating_in_history() {
    cmd()
        .args(["memory", "state", "-H", "5:0,3:1"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Rating must be 1-4"));
}

#[test]
fn test_memory_state_starting_stability_without_difficulty() {
    cmd()
        .args([
            "memory",
            "state",
            "-H",
            "3:0,3:1",
            "--starting-stability",
            "5.0",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Both --starting-stability and --starting-difficulty",
        ));
}

// ============================================================
// Config command tests
// ============================================================

#[test]
fn test_parameters_default() {
    let config_dir = TempDir::new().unwrap();
    let v: Vec<f64> = run_json_with_config(
        config_dir.path(),
        &["config", "parameters", "get", "--json"],
    );
    assert_eq!(v.len(), 21);
    assert_approx(v[0], 0.212);
    assert_approx(v[4], 6.4133);
    assert_approx(v[20], 0.1542);
}

#[test]
fn test_config_default_json() {
    let config_dir = TempDir::new().unwrap();
    let v: ConfigOutput = run_json_with_config(config_dir.path(), &["config", "--json"]);
    assert!(v.config_file.contains("config.json"));
    assert_eq!(v.defaults.parameters.len(), 21);
    assert_approx(v.defaults.parameters[0], 0.212);
    assert_approx(v.defaults.retention, 0.9);
}

#[test]
fn test_config_default_human_output_shows_quoted_path_and_builtin_retention() {
    let config_dir = TempDir::new().unwrap();
    let config_root = config_dir.path().join("config dir with spaces");
    std::fs::create_dir_all(&config_root).unwrap();
    let config_file = config_root.join("config.json");

    cmd_with_config(&config_root)
        .args(["config"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains(format!("File: \"{}\" (not created)", config_file.display()))
                .and(predicate::str::contains("Retention: 0.90")),
        );
}

#[test]
fn test_config_human_output_marks_file_as_created_after_save() {
    let config_dir = TempDir::new().unwrap();
    let config_root = config_dir.path().join("config dir with spaces");
    std::fs::create_dir_all(&config_root).unwrap();
    let config_file = config_root.join("config.json");

    cmd_with_config(&config_root)
        .args(["config", "retention", "set", "0.85"])
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "Config file: \"{}\" (created)",
            config_file.display()
        )));

    cmd_with_config(&config_root)
        .args(["config"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains(format!("File: \"{}\" (created)", config_file.display()))
                .and(predicate::str::contains("Retention: 0.85")),
        );
}

#[test]
fn test_parameters_save_and_reuse_custom_values() {
    let config_dir = TempDir::new().unwrap();
    let custom = "0.5,1.0,2.0,6.0,5.0,0.8,3.0,0.001,1.8,0.2,0.8,1.5,0.06,0.26,1.6,0.6,1.9,0.5,0.09,0.07,0.15";

    let saved: Vec<f64> = run_json_with_config(
        config_dir.path(),
        &["config", "parameters", "set", custom, "--json"],
    );
    assert_eq!(saved.len(), 21);
    assert_approx(saved[0], 0.5);
    assert_approx(saved[4], 5.0);

    let active: Vec<f64> = run_json_with_config(
        config_dir.path(),
        &["config", "parameters", "get", "--json"],
    );
    assert_approx(active[0], 0.5);
    assert_approx(active[4], 5.0);

    let schedule: NextStates = run_json_with_config(config_dir.path(), &["schedule", "--json"]);
    assert_approx(schedule.again.memory.stability, 0.5);
    assert_approx(schedule.hard.memory.stability, 1.0);
    assert_approx(schedule.good.memory.stability, 2.0);
    assert_approx(schedule.easy.memory.stability, 6.0);
    assert_approx(schedule.again.memory.difficulty, 5.0);
}

#[test]
fn test_parameters_reset_restores_defaults() {
    let config_dir = TempDir::new().unwrap();
    let custom = "0.5,1.0,2.0,6.0,5.0,0.8,3.0,0.001,1.8,0.2,0.8,1.5,0.06,0.26,1.6,0.6,1.9,0.5,0.09,0.07,0.15";

    cmd_with_config(config_dir.path())
        .args(["config", "parameters", "set", custom, "--json"])
        .assert()
        .success();

    let reset: Vec<f64> = run_json_with_config(
        config_dir.path(),
        &["config", "parameters", "reset", "--json"],
    );
    assert_eq!(reset.len(), 21);
    assert_approx(reset[0], 0.212);

    let schedule: NextStates = run_json_with_config(config_dir.path(), &["schedule", "--json"]);
    assert_approx(schedule.again.memory.stability, 0.212);
    assert_approx(schedule.again.memory.difficulty, 6.4133);
}

#[test]
fn test_parameters_human_readable() {
    cmd()
        .args(["config", "parameters"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("FSRS Parameters")
                .and(predicate::str::contains("initial_stability_again")),
        );
}

#[test]
fn test_config_parameters_set_rejects_invalid_values() {
    let config_dir = TempDir::new().unwrap();

    cmd_with_config(config_dir.path())
        .args([
            "config",
            "parameters",
            "set",
            "0.5,1.0,2.0,6.0,5.0,0.8,3.0,0.001,1.8,0.2,0.8,1.5,0.06,0.26,1.6,0.6,1.9,0.5,0.09,0.07",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("expected 21 FSRS parameters"));
}

#[test]
fn test_config_parameters_set_rejects_non_finite_values() {
    let config_dir = TempDir::new().unwrap();

    cmd_with_config(config_dir.path())
        .args([
            "config",
            "parameters",
            "set",
            "0.5,1.0,2.0,6.0,5.0,0.8,3.0,0.001,1.8,0.2,0.8,1.5,0.06,0.26,1.6,0.6,1.9,0.5,0.09,0.07,NaN",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("FSRS parameters must be finite numbers"));
}

#[test]
fn test_config_parameters_set_accepts_bracketed_values() {
    let config_dir = TempDir::new().unwrap();

    let saved: Vec<f64> = run_json_with_config(
        config_dir.path(),
        &[
            "config",
            "parameters",
            "set",
            "[0.5,1.0,2.0,6.0,5.0,0.8,3.0,0.001,1.8,0.2,0.8,1.5,0.06,0.26,1.6,0.6,1.9,0.5,0.09,0.07,0.15]",
            "--json",
        ],
    );

    assert_eq!(saved.len(), 21);
    assert_approx(saved[0], 0.5);
    assert_approx(saved[20], 0.15);
}

#[test]
fn test_config_retention_set_get_reset() {
    let config_dir = TempDir::new().unwrap();

    let default_retention: f64 =
        run_json_with_config(config_dir.path(), &["config", "retention", "get", "--json"]);
    assert_approx(default_retention, 0.9);

    let saved: f64 = run_json_with_config(
        config_dir.path(),
        &["config", "retention", "set", "0.85", "--json"],
    );
    assert_approx(saved, 0.85);

    let active: f64 =
        run_json_with_config(config_dir.path(), &["config", "retention", "get", "--json"]);
    assert_approx(active, 0.85);

    let config: ConfigOutput = run_json_with_config(config_dir.path(), &["config", "--json"]);
    assert_approx(config.defaults.retention, 0.85);

    let reset: serde_json::Value = run_json_with_config(
        config_dir.path(),
        &["config", "retention", "reset", "--json"],
    );
    assert!(reset.is_null());

    let active: f64 =
        run_json_with_config(config_dir.path(), &["config", "retention", "get", "--json"]);
    assert_approx(active, 0.9);

    let config: ConfigOutput = run_json_with_config(config_dir.path(), &["config", "--json"]);
    assert_approx(config.defaults.retention, 0.9);
}

#[test]
fn test_schedule_uses_saved_retention() {
    let config_dir = TempDir::new().unwrap();

    cmd_with_config(config_dir.path())
        .args(["config", "retention", "set", "0.85", "--json"])
        .assert()
        .success();

    let v: NextStates = run_json_with_config(config_dir.path(), &["schedule", "--json"]);
    assert_approx(v.again.interval, 0.4042);
    assert_approx(v.good.interval, 4.3972);
}

#[test]
fn test_simulate_workload_uses_saved_retention() {
    let config_dir = TempDir::new().unwrap();

    cmd_with_config(config_dir.path())
        .args(["config", "retention", "set", "0.85", "--json"])
        .assert()
        .success();

    let v: WorkloadOutput =
        run_json_with_config(config_dir.path(), &["simulate", "workload", "--json"]);
    assert_approx(v.expected_workload, 120.4056);
    assert_approx(v.retention, 0.85);
}

// ============================================================
// Simulate command tests
// ============================================================

#[test]
fn test_simulate_workload() {
    let v: WorkloadOutput = run_json(&["simulate", "workload", "--json"]);
    assert_approx(v.expected_workload, 155.2765);
    assert_approx(v.retention, 0.9);
}

#[test]
fn test_simulate_workload_custom_retention() {
    let v: WorkloadOutput = run_json(&["simulate", "workload", "--retention", "0.85", "--json"]);
    assert_approx(v.expected_workload, 120.4056);
    assert_approx(v.retention, 0.85);
}

#[test]
fn test_simulate_run() {
    let v: SimulateOutput = run_json(&[
        "simulate",
        "run",
        "--deck-size",
        "100",
        "--learn-span",
        "30",
        "--seed",
        "42",
        "--json",
    ]);
    assert_eq!(v.total_reviews, 373);
    assert_eq!(v.total_learned, 100);
    assert_eq!(v.days, 30);
    assert_approx(v.final_memorized, 94.9646);
    assert_approx(v.total_cost, 9540.501);
}

#[test]
fn test_simulate_run_human_readable() {
    cmd()
        .args([
            "simulate",
            "run",
            "--deck-size",
            "100",
            "--learn-span",
            "30",
            "--seed",
            "42",
        ])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Simulation Results")
                .and(predicate::str::contains("Total reviews")),
        );
}

#[test]
fn test_simulate_run_with_limits() {
    let v: SimulateOutput = run_json(&[
        "simulate",
        "run",
        "--deck-size",
        "50",
        "--learn-span",
        "10",
        "--learn-limit",
        "5",
        "--review-limit",
        "20",
        "--seed",
        "42",
        "--json",
    ]);
    assert_eq!(v.total_reviews, 65);
    assert_eq!(v.total_learned, 50);
    assert_eq!(v.days, 10);
    assert_approx(v.final_memorized, 48.3928);
}

// ============================================================
// CSV parsing and validation tests
// ============================================================

fn create_revlog_csv() -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        "card_id,review_time,review_rating,review_state,review_duration"
    )
    .unwrap();
    writeln!(file, "1001,1000000000000,3,0,5000").unwrap();
    writeln!(file, "1001,1000086400000,3,1,4000").unwrap();
    writeln!(file, "1001,1000518400000,3,2,3000").unwrap();
    writeln!(file, "1001,1001382400000,2,2,8000").unwrap();
    writeln!(file, "1002,1000000000000,3,0,6000").unwrap();
    writeln!(file, "1002,1000172800000,3,1,3500").unwrap();
    writeln!(file, "1002,1000604800000,4,2,2000").unwrap();
    writeln!(file, "1003,1000000000000,1,0,10000").unwrap();
    writeln!(file, "1003,1000086400000,3,1,5000").unwrap();
    writeln!(file, "1003,1000259200000,3,2,4000").unwrap();
    file
}

fn create_invalid_csv() -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "foo,bar,baz").unwrap();
    writeln!(file, "1,2,3").unwrap();
    file
}

#[test]
fn test_optimize_with_csv_missing_file() {
    cmd()
        .args(["optimize", "--csv", "/nonexistent/file.csv"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn test_evaluate_with_csv_missing_file() {
    cmd()
        .args(["evaluate", "--csv", "/nonexistent/file.csv"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn test_optimize_invalid_csv_format() {
    let file = create_invalid_csv();
    cmd()
        .args(["optimize", "--csv", file.path().to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("CSV deserialize error"));
}

#[test]
fn test_evaluate_invalid_csv_format() {
    let file = create_invalid_csv();
    cmd()
        .args(["evaluate", "--csv", file.path().to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("CSV deserialize error"));
}

#[test]
fn test_evaluate_revlog_csv() {
    let file = create_revlog_csv();
    let v: EvaluateOutput =
        run_json(&["evaluate", "--csv", file.path().to_str().unwrap(), "--json"]);
    assert_approx(v.log_loss, 0.0824);
    assert_approx(v.rmse_bins, 0.0848);
}

#[test]
fn test_benchmark_revlog_csv() {
    let file = create_revlog_csv();
    let v: Vec<f64> = run_json(&[
        "benchmark",
        "--csv",
        file.path().to_str().unwrap(),
        "--json",
    ]);
    assert_eq!(v.len(), 21);
    assert_approx(v[0], 1.2422);
    assert_approx(v[1], 1.7280);
    assert_approx(v[2], 2.4086);
    assert_approx(v[3], 2.9497);
    assert_approx(v[20], 0.1517);
}
