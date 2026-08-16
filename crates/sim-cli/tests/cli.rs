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
fn phase9_fixture_is_identical_across_clean_processes_and_pinned() {
    // C9.7 clause 1. Two separate processes, and the constants pinned rather
    // than only compared with each other: `verify-phase1-determinism.sh` and
    // `verify-phase2-determinism.sh` only `cmp` two runs of one build, so a
    // change that moved a checksum consistently would pass both of them.
    //
    // The full 8,000-tick fixture, not a cheap short one. The horizon is the
    // point: `maturity_age_ticks` is 600 and founders spawn at age 0, so at
    // 500 ticks this world has had zero births and would pin nothing about
    // meiosis, structural mutation, or the schema-2 birth path.
    let args = [
        "fixture",
        "--ticks",
        "8000",
        "--phase2",
        "--genome2",
        "--seed",
        "0x5eedcafef00dbeef",
    ];
    let first = lifesim(&args);
    let second = lifesim(&args);
    assert!(first.status.success(), "stderr: {}", stderr(&first));
    assert_eq!(stdout(&first), stdout(&second));
    let line = stdout(&first);
    for expected in [
        "\"fixture_schema_version\":4",
        "\"phase\":\"phase9\"",
        "\"genome2_policy\":\"lifesim-genome-v2\"",
        "\"meiosis_policy\":\"lifesim-meiosis-v1\"",
        "\"structmut_policy\":\"lifesim-structmut-v1\"",
        "\"controller2_policy\":\"lifesim-controller-v2\"",
        "\"config_hash\":\"0x9abc0cd47914127f\"",
        "\"state_checksum\":\"0x5f0c4e95e4f5170f\"",
    ] {
        assert!(line.contains(expected), "missing {expected} in {line}");
    }
    // Non-vacuity. A fixture whose births, deaths, and applied structural
    // mutations are all zero is a control wearing a fixture's name, and the
    // checksum above would say nothing about it either way.
    // `duplications_applied` is separate from `structural_mutations_applied`
    // because the latter counts point mutation, which changes no structure.
    for forbidden in [
        "\"births_total\":0,",
        "\"paired_births_total\":0,",
        "\"deaths_total\":0,",
        "\"duplications_applied\":0,",
        "\"structural_mutations_rejected\":0}",
    ] {
        assert!(
            !line.contains(forbidden),
            "the Phase 9 fixture became vacuous ({forbidden}): {line}"
        );
    }

    // ...and the flag is refused rather than quietly implying --phase2, so a
    // schema-2 request can never silently produce a schema-1 world.
    let refused = lifesim(&["fixture", "--ticks", "10", "--genome2"]);
    assert!(!refused.status.success());
    assert!(stderr(&refused).contains("--genome2 requires --phase2"));
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

#[test]
fn phase11_trace_is_identical_across_clean_processes_and_pinned() {
    // C11.5's clean-process half, at a horizon this test can afford. The
    // 10^6-tick trace the criterion names is
    // `scripts/verify-phase11-determinism.sh`; what is checked here is that
    // the fixture exists, is pinned, is schema 5, and is not a control.
    //
    // 200,000 ticks rather than 1,000,000 because the whole point of the long
    // horizon is accumulation, and the accumulation clause below can be made
    // at any two horizons that differ. The script carries the criterion's
    // number.
    let args = [
        "fixture",
        "--ticks",
        "200000",
        "--phase2",
        "--genome2",
        "--plasticity",
        "--seed",
        "0x5eedcafef00dbeef",
    ];
    let first = lifesim(&args);
    let second = lifesim(&args);
    assert!(first.status.success(), "stderr: {}", stderr(&first));
    assert_eq!(stdout(&first), stdout(&second));
    let line = stdout(&first);
    for expected in [
        "\"fixture_schema_version\":7",
        "\"phase\":\"phase11\"",
        "\"plasticity_policy\":\"lifesim-plasticity-v2\"",
        "\"rule_registry\":1",
        "\"config_hash\":\"0xae34cd2b6f7a3e13\"",
        // One organism, alive at the end, with both edges plastic and the
        // learn phase having fired on every one of them: 2 edges x 200,000
        // ticks. Any of these at zero and the checksum above would still
        // reproduce - it would simply be a different constant.
        "\"population\":1",
        "\"plastic_edges_total\":2",
        "\"plasticity_updates_total\":400000",
        // **The split, and the reason the total above is not enough.** The
        // confirmatory campaign reported 1,109,373,897 updates of which
        // 95.43 percent were `StepKind::Static` - the early return for a
        // flagged edge whose rule is 0, taken before any gene is read - and
        // the finding was written up as "the mechanism executed a billion
        // times". A total that sums applied, static and refused answers
        // "how many plastic edges were visited", never "how many learned",
        // and only the first question has a bearing on whether anything
        // happened.
        //
        // This trace is the pure case and is pinned as such: every visit
        // applies, because the one organism's two edges carry a live rule
        // and a nonzero eta for all 200,000 ticks. A static count above
        // zero here would mean the fixture had started measuring the
        // early return instead of the arithmetic.
        "\"plasticity_updates_applied\":400000",
        "\"plasticity_updates_static\":0",
        // Zero forever, and asserted rather than assumed: a rule id outside
        // the registry is a genome-validation bug report, and this fixture
        // is where it would first show.
        "\"plasticity_updates_refused\":0",
        // Both plastic edges actually moved. `plastic_edges_total` says how
        // many edges *could* learn and this says how many *did*, which is the
        // distinction D-098 was published for want of: a mean over all the
        // edges that could learn reads zero whether none of them did or a
        // few of them did a great deal.
        "\"learned_edges_nonzero\":2",
        "\"plasticity_anomalies_total\":0",
        "\"controller_faults_total\":0}",
    ] {
        assert!(line.contains(expected), "missing {expected} in {line}");
    }
    assert!(
        !line.contains("\"mean_abs_learned_milli\":0,"),
        "the trace learned nothing: {line}"
    );
    // The count and the max are independent of the mean and of each other,
    // and each is asserted against the thing only it can catch. A max of zero
    // with a nonzero count would mean every delta is under 66 Q16 - a live
    // mechanism that never accumulates - which reproduces just as well as a
    // healthy one and would read as success in every other field on the line.
    assert!(
        !line.contains("\"max_abs_learned_milli\":0,"),
        "no edge learned by as much as one part in a thousand: {line}"
    );

    // **The accumulation clause.** A plastic edge with decay settles at an
    // equilibrium and then repeats one step forever; a fixture in that state
    // reproduces perfectly and measures nothing past the point it settled.
    // The first cut of this fixture did exactly that, with the input bound to
    // `energy_fraction`, which is constant in a world with no energy costs.
    // Two horizons with different learned magnitudes is what says the trace
    // is still moving.
    let mut short_args = args;
    short_args[2] = "20000";
    let short = lifesim(&short_args);
    let learned = |line: &str| {
        line.split("\"mean_abs_learned_milli\":")
            .nth(1)
            .and_then(|rest| rest.split(',').next())
            .expect("the field is in the line")
            .to_owned()
    };
    assert_ne!(
        learned(&line),
        learned(&stdout(&short)),
        "the trace reached a fixed point, so its horizon is decorative"
    );

    // The flag is refused rather than quietly implying its prerequisites, on
    // the same grounds --genome2 is.
    let refused = lifesim(&["fixture", "--ticks", "10", "--plasticity"]);
    assert!(!refused.status.success());
    assert!(stderr(&refused).contains("--plasticity requires --phase2 --genome2"));

    // ...and the Phase 9 fixture is untouched by the flag existing.
    let phase9 = lifesim(&[
        "fixture",
        "--ticks",
        "8000",
        "--phase2",
        "--genome2",
        "--seed",
        "0x5eedcafef00dbeef",
    ]);
    assert!(stdout(&phase9).contains("\"state_checksum\":\"0x5f0c4e95e4f5170f\""));
}
