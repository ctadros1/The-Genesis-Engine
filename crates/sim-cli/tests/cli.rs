//! CLI integration tests. Each invocation is a separate clean process, so
//! the fixture equality test doubles as a clean-process determinism check.

use std::process::{Command, Output};

fn lifesim(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_lifesim"))
        .args(args)
        .output()
        .expect("spawn lifesim")
}

const SMALL_WORLD: &[&str] = &[
    "--cells-x",
    "64",
    "--cells-y",
    "64",
    "--organisms",
    "50",
    "--max-entities",
    "500",
];

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn fixture_is_identical_across_clean_processes() {
    let mut args = vec!["fixture", "--ticks", "300"];
    args.extend_from_slice(SMALL_WORLD);
    let first = lifesim(&args);
    let second = lifesim(&args);
    assert!(first.status.success(), "stderr: {}", stderr(&first));
    assert_eq!(stdout(&first), stdout(&second));
    let line = stdout(&first);
    assert!(line.contains("\"fixture_schema_version\":2"));
    assert!(line.contains("\"state_checksum\":\"0x"));
    assert!(line.contains("\"terrain_checksum\":\"0x"));
}

#[test]
fn fixture_differs_for_different_seed() {
    let mut base = vec!["fixture", "--ticks", "100"];
    base.extend_from_slice(SMALL_WORLD);
    let mut other = base.clone();
    other.extend_from_slice(&["--seed", "0x1234"]);
    let first = lifesim(&base);
    let second = lifesim(&other);
    assert!(first.status.success() && second.status.success());
    assert_ne!(stdout(&first), stdout(&second));
}

#[test]
fn run_reports_summary_and_verifies_pause() {
    let mut args = vec![
        "run",
        "--ticks",
        "200",
        "--pause-at",
        "100",
        "--pause-ticks",
        "7",
    ];
    args.extend_from_slice(SMALL_WORLD);
    let output = lifesim(&args);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let line = stdout(&output);
    assert!(line.contains("\"run_schema_version\":2"));
    assert!(line.contains("\"final_tick\":200"));
    assert!(line.contains("\"paused_ticks_verified\":7"));
    assert!(line.contains("\"invariants\":\"ok\""));
    assert!(line.contains("\"behavior_policy\":\"phase1-behavior-v1\""));
}

#[test]
fn metrics_output_matches_prometheus_schema() {
    let mut args = vec!["run", "--ticks", "50", "--metrics-out", "-"];
    args.extend_from_slice(SMALL_WORLD);
    let output = lifesim(&args);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let text = stdout(&output);
    for expected in [
        "# TYPE lifesim_organisms gauge",
        "lifesim_organisms{world=\"local\",life_state=\"alive\"} ",
        "# TYPE lifesim_births_total counter",
        "lifesim_deaths_total{world=\"local\",cause=\"starvation\"} ",
        "lifesim_deaths_total{world=\"local\",cause=\"old_age\"} ",
        "lifesim_food_biomass{world=\"local\",biome=\"all\"} ",
        "# TYPE lifesim_tick_duration_seconds histogram",
        "lifesim_tick_duration_seconds_bucket{world=\"local\",le=\"+Inf\"} 50",
        "lifesim_tick_duration_seconds_count{world=\"local\"} 50",
        "lifesim_tick_rate_hz{world=\"local\"} ",
    ] {
        assert!(text.contains(expected), "missing metric line: {expected}");
    }
}

#[test]
fn inspect_reports_worldgen_summary() {
    let mut args = vec!["inspect"];
    args.extend_from_slice(SMALL_WORLD);
    let output = lifesim(&args);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let line = stdout(&output);
    assert!(line.contains("\"inspect_schema_version\":1"));
    assert!(line.contains("\"habitable_cells\":"));
    assert!(line.contains("\"land_fraction_q16\":"));
}

#[test]
fn invalid_inputs_are_rejected() {
    // Unknown option.
    let output = lifesim(&["run", "--ticks", "10", "--bogus", "1"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("unknown option"));

    // Missing required --ticks.
    let output = lifesim(&["run"]);
    assert!(!output.status.success());

    // Invalid config: zero organisms.
    let output = lifesim(&["run", "--ticks", "10", "--organisms", "0"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("invalid initial organism count"));

    // Invalid config: initial organisms above capacity ceiling.
    let output = lifesim(&[
        "run",
        "--ticks",
        "10",
        "--organisms",
        "100",
        "--max-entities",
        "50",
    ]);
    assert!(!output.status.success());

    // Invalid seed literal.
    let output = lifesim(&["fixture", "--seed", "zzz"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("invalid seed"));

    // Unknown subcommand prints usage.
    let output = lifesim(&["frobnicate"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("usage:"));
}

#[test]
fn reproduction_can_be_disabled() {
    let mut args = vec!["run", "--ticks", "100", "--no-reproduction"];
    args.extend_from_slice(SMALL_WORLD);
    let output = lifesim(&args);
    assert!(output.status.success());
    assert!(stdout(&output).contains("\"births_total\":0"));
}

#[test]
fn phase2_fixture_is_identical_across_clean_processes_and_versioned() {
    let mut args = vec!["fixture", "--ticks", "300", "--phase2"];
    args.extend_from_slice(SMALL_WORLD);
    let first = lifesim(&args);
    let second = lifesim(&args);
    assert!(first.status.success(), "stderr: {}", stderr(&first));
    assert_eq!(stdout(&first), stdout(&second));
    let line = stdout(&first);
    assert!(line.contains("\"fixture_schema_version\":3"));
    assert!(line.contains("\"phase\":\"phase2\""));
    assert!(line.contains("\"behavior_policy\":\"phase2-behavior-v1\""));
    assert!(line.contains("\"genome_policy\":\"lifesim-genome-v1\""));
    assert!(line.contains("\"controller_policy\":\"lifesim-controller-v1\""));

    // Phase 2 must not disturb the Phase 1 fixture: the same invocation
    // without --phase2 keeps schema 2 and a different lineage.
    let mut phase1_args = vec!["fixture", "--ticks", "300"];
    phase1_args.extend_from_slice(SMALL_WORLD);
    let phase1 = lifesim(&phase1_args);
    assert!(stdout(&phase1).contains("\"fixture_schema_version\":2"));
    assert_ne!(stdout(&phase1), stdout(&first));
}

#[test]
fn phase2_run_reports_phase2_fields_and_metrics() {
    let mut args = vec!["run", "--ticks", "150", "--phase2", "--metrics-out", "-"];
    args.extend_from_slice(SMALL_WORLD);
    let output = lifesim(&args);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("\"behavior_policy\":\"phase2-behavior-v1\""));
    assert!(text.contains("\"phase2_enabled\":true"));
    assert!(text.contains("\"controller_faults_total\":0"));
    assert!(text.contains("lifesim_paired_births_total{world=\"local\"} "));
    assert!(text.contains("lifesim_pair_rejected_total{world=\"local\",reason=\"capacity\"} "));
    assert!(text.contains("lifesim_controller_faults_total{world=\"local\"} "));
}

#[test]
fn analyze_requires_phase2_and_reports_versioned_output() {
    let mut args = vec!["analyze", "--ticks", "100", "--phase2"];
    args.extend_from_slice(SMALL_WORLD);
    let output = lifesim(&args);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let line = stdout(&output);
    assert!(line.contains("\"algorithm\":\"lifesim-similarity-v1\""));
    assert!(line.contains("\"cluster_count\":"));
    assert!(line.contains("\"analysis_runtime_microseconds\":"));

    let mut bad_args = vec!["analyze", "--ticks", "10"];
    bad_args.extend_from_slice(SMALL_WORLD);
    let rejected = lifesim(&bad_args);
    assert!(!rejected.status.success());
    assert!(stderr(&rejected).contains("requires --phase2"));
}
