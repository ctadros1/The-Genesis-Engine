//! Phase 5 CLI integration tests: campaigns, manifests, comparison
//! refusals, event logs, and fixture preservation.
//!
//! Covers the user-facing half of A5.6 (conditions are distinguishable by
//! construction and the report refuses to aggregate anything else) and
//! A5.7 (both fixtures reproduce from clean processes under the new
//! execution paths).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn lifesim(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_lifesim"))
        .args(args)
        .output()
        .expect("spawn lifesim")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn scratch_dir(name: &str) -> PathBuf {
    let directory =
        std::env::temp_dir().join(format!("lifesim-phase5-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).expect("scratch dir");
    directory
}

fn write_campaign(directory: &Path, text: &str) -> PathBuf {
    let path = directory.join("campaign.txt");
    fs::write(&path, text).expect("write campaign");
    path
}

/// Two conditions, four seeds, small worlds. `basal_cost_milli_per_s`
/// applies to every organism every tick, so the treatment is guaranteed to
/// bite rather than merely to change the config hash.
const CAMPAIGN: &str = "\
campaign cli-pilot
ticks 300
seeds 1 2 5 6
base preset phase2
base cells_x 64
base cells_y 64
base initial_organisms 40
base max_entities 400
condition control
condition costly
set costly basal_cost_milli_per_s 160
vary basal_cost_milli_per_s
output events on
output snapshots on
";

fn run_batch(directory: &Path, campaign: &Path, workers: &str) -> (Output, PathBuf) {
    let output_dir = directory.join(format!("out{workers}"));
    let output = lifesim(&[
        "batch",
        "--campaign",
        campaign.to_str().unwrap(),
        "--output",
        output_dir.to_str().unwrap(),
        "--workers",
        workers,
    ]);
    (output, output_dir)
}

#[test]
fn batch_runs_a_campaign_and_report_compares_it() {
    let directory = scratch_dir("batch");
    let campaign = write_campaign(&directory, CAMPAIGN);
    let (output, output_dir) = run_batch(&directory, &campaign, "4");
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let summary = stdout(&output);
    assert!(summary.contains("\"worlds\":8"), "{summary}");
    assert!(summary.contains("\"failed\":0"), "{summary}");

    // Every declared world produced both artifacts.
    for condition in ["control", "costly"] {
        for seed in [1_u64, 2, 5, 6] {
            let stem = format!("{condition}-seed{seed:016x}");
            assert!(
                output_dir.join(format!("{stem}.alif")).exists(),
                "missing snapshot for {stem}"
            );
            assert!(
                output_dir.join(format!("{stem}.alev")).exists(),
                "missing event log for {stem}"
            );
        }
    }

    let manifest = output_dir.join("manifest.txt");
    let report = lifesim(&["report", "--manifest", manifest.to_str().unwrap()]);
    assert!(report.status.success(), "stderr: {}", stderr(&report));
    let text = stdout(&report);
    assert!(text.contains("campaign cli-pilot"));
    assert!(text.contains("baseline control"));
    assert!(text.contains("varied basal_cost_milli_per_s"));
    assert!(text.contains("No significance test"));

    // Every event log the campaign wrote is complete.
    let log = output_dir.join("control-seed0000000000000001.alev");
    let verify = lifesim(&["verify-events", log.to_str().unwrap()]);
    assert!(verify.status.success(), "stderr: {}", stderr(&verify));
    assert!(stdout(&verify).contains("\"status\":\"complete\""));
}

#[test]
fn a5_6_batch_output_is_identical_at_every_worker_count() {
    // The manifest records the world results; only the `workers` line and
    // the wall-clock summary may differ between these two runs.
    let directory = scratch_dir("workers");
    let campaign = write_campaign(&directory, CAMPAIGN);
    let (first, first_dir) = run_batch(&directory, &campaign, "1");
    let (second, second_dir) = run_batch(&directory, &campaign, "8");
    assert!(first.status.success(), "stderr: {}", stderr(&first));
    assert!(second.status.success(), "stderr: {}", stderr(&second));

    let strip_workers = |path: &Path| -> String {
        fs::read_to_string(path.join("manifest.txt"))
            .expect("manifest")
            .lines()
            .filter(|line| !line.starts_with("workers "))
            .collect::<Vec<_>>()
            .join("\n")
    };
    assert_eq!(
        strip_workers(&first_dir),
        strip_workers(&second_dir),
        "the manifest depends on worker count"
    );
}

#[test]
fn a5_6_two_conditions_that_are_the_same_experiment_are_refused() {
    let directory = scratch_dir("twins");
    let campaign = write_campaign(
        &directory,
        "campaign twins\nticks 10\nseeds 1\ncondition control\ncondition treatment\n",
    );
    let (output, _) = run_batch(&directory, &campaign, "1");
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("one experiment under two names"),
        "stderr: {}",
        stderr(&output)
    );
}

#[test]
fn a5_6_an_undeclared_difference_between_conditions_is_refused() {
    let directory = scratch_dir("undeclared");
    let campaign = write_campaign(
        &directory,
        "campaign sloppy\nticks 10\nseeds 1\ncondition control\ncondition treatment\n\
         set treatment crowding_threshold 9\n",
    );
    let (output, _) = run_batch(&directory, &campaign, "1");
    assert!(!output.status.success());
    let message = stderr(&output);
    assert!(message.contains("crowding_threshold"), "stderr: {message}");
    assert!(
        message.contains("vary crowding_threshold"),
        "stderr: {message}"
    );
}

#[test]
fn report_refuses_an_edited_manifest_rather_than_reinterpreting_it() {
    let directory = scratch_dir("edited");
    let campaign = write_campaign(&directory, CAMPAIGN);
    let (batch, output_dir) = run_batch(&directory, &campaign, "2");
    assert!(batch.status.success(), "stderr: {}", stderr(&batch));

    let manifest = output_dir.join("manifest.txt");
    let text = fs::read_to_string(&manifest).expect("manifest");
    // Edit the embedded campaign so it no longer matches its recorded hash.
    let tampered = text.replace("| ticks 300", "| ticks 301");
    assert_ne!(tampered, text, "the edit did not apply");
    fs::write(&manifest, tampered).expect("write");

    let report = lifesim(&["report", "--manifest", manifest.to_str().unwrap()]);
    assert!(!report.status.success());
    assert!(
        stderr(&report).contains("has been edited"),
        "stderr: {}",
        stderr(&report)
    );
}

#[test]
fn a_campaign_whose_seeds_cannot_all_generate_worlds_is_refused_before_running() {
    // Seeds 3, 4, and 5 fail land-fraction validation at 48x48. Running
    // 3 of 6 declared worlds would be a different design from the declared
    // one, so the campaign is refused rather than quietly reduced.
    let directory = scratch_dir("preflight");
    let campaign = write_campaign(
        &directory,
        "campaign holes\nticks 50\nseeds 1..6\nbase cells_x 48\nbase cells_y 48\n\
         base initial_organisms 40\nbase max_entities 400\ncondition only\n",
    );
    let (output, output_dir) = run_batch(&directory, &campaign, "2");
    assert!(!output.status.success());
    let message = stderr(&output);
    assert!(message.contains("cannot be generated"), "stderr: {message}");
    assert!(
        message.contains("a design you did not declare"),
        "stderr: {message}"
    );
    assert!(
        !output_dir.join("manifest.txt").exists(),
        "a refused campaign must not leave a manifest"
    );
}

#[test]
fn verify_events_reports_a_torn_log_without_repairing_it() {
    let directory = scratch_dir("torn");
    let log = directory.join("run.alev");
    // These parameters produce a steady stream of births and deaths. A
    // smaller or shorter world can finish with an empty log, and truncating
    // an empty log tests the header path rather than the segment path,
    // which is not what this test is for.
    let run = lifesim(&[
        "run",
        "--ticks",
        "3000",
        "--cells-x",
        "128",
        "--cells-y",
        "128",
        "--organisms",
        "200",
        "--max-entities",
        "3000",
        "--event-log",
        log.to_str().unwrap(),
    ]);
    assert!(run.status.success(), "stderr: {}", stderr(&run));

    let whole = lifesim(&["verify-events", log.to_str().unwrap()]);
    assert!(whole.status.success());
    let intact = stdout(&whole);
    assert!(intact.contains("\"status\":\"complete\""), "{intact}");
    assert!(
        !intact.contains("\"segments\":0,"),
        "the log has no segments, so there is nothing to tear: {intact}"
    );

    // Truncate mid-segment, the shape a crash between write and sync leaves.
    let bytes = fs::read(&log).expect("read log");
    fs::write(&log, &bytes[..bytes.len() - 4]).expect("truncate");
    let torn = lifesim(&["verify-events", log.to_str().unwrap()]);
    assert!(!torn.status.success(), "a torn log must not report success");
    let report = stdout(&torn);
    assert!(report.contains("\"status\":\"truncated"), "{report}");
    // The valid prefix is reported, and it is smaller than the file.
    assert!(report.contains("\"bytes_valid\":"), "{report}");
}

#[test]
fn fields_lists_every_settable_config_field() {
    let output = lifesim(&["fields"]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let text = stdout(&output);
    for expected in [
        "cells_x",
        "crowding_threshold",
        "phase2.enabled",
        "phase2.cluster_sample_max",
    ] {
        assert!(text.contains(expected), "missing field {expected}");
    }
    // The seed axis is never a settable field.
    assert!(!text.contains("world_seed"));
}

#[test]
fn a5_7_recording_an_event_log_does_not_change_the_phase_1_fixture() {
    let directory = scratch_dir("fixture1");
    let log = directory.join("phase1.alev");
    let output = lifesim(&[
        "run",
        "--ticks",
        "500",
        "--event-log",
        log.to_str().unwrap(),
    ]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(
        stdout(&output).contains("\"state_checksum\":\"0x1e3158a26afd3b39\""),
        "phase 1 fixture changed: {}",
        stdout(&output)
    );
    // The log exists and is complete, so the equality is not because
    // logging silently did nothing.
    let verify = lifesim(&["verify-events", log.to_str().unwrap()]);
    assert!(verify.status.success(), "stderr: {}", stderr(&verify));
    assert!(stdout(&verify).contains("\"status\":\"complete\""));
}

#[test]
fn a5_7_recording_an_event_log_does_not_change_the_phase_2_fixture() {
    let directory = scratch_dir("fixture2");
    let log = directory.join("phase2.alev");
    let output = lifesim(&[
        "run",
        "--ticks",
        "500",
        "--phase2",
        "--event-log",
        log.to_str().unwrap(),
    ]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(
        stdout(&output).contains("\"state_checksum\":\"0xff9dfcff5dffbf42\""),
        "phase 2 fixture changed: {}",
        stdout(&output)
    );
    let verify = lifesim(&["verify-events", log.to_str().unwrap()]);
    assert!(verify.status.success(), "stderr: {}", stderr(&verify));
}

#[test]
fn a5_7_a_campaign_reproduces_both_fixtures() {
    // The scheduler is an execution path like any other: a world run
    // through it must be the same world.
    let directory = scratch_dir("fixture-batch");
    let campaign = write_campaign(
        &directory,
        "campaign fixtures\nticks 500\nseeds 0x5eedcafef00dbeef\n\
         condition phase1\ncondition phase2\nset phase2 phase2.enabled true\n\
         vary phase2.enabled\noutput events off\noutput snapshots off\n",
    );
    let (output, output_dir) = run_batch(&directory, &campaign, "2");
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let manifest = fs::read_to_string(output_dir.join("manifest.txt")).expect("manifest");
    assert!(
        manifest.contains("state_checksum=0x1e3158a26afd3b39"),
        "phase 1 fixture not reproduced through the scheduler"
    );
    assert!(
        manifest.contains("state_checksum=0xff9dfcff5dffbf42"),
        "phase 2 fixture not reproduced through the scheduler"
    );
}
