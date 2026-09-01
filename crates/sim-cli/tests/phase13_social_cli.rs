//! Phase 13 `lifesim social`: the C13.1 observer report reads a real
//! campaign's artifacts (manifest, event logs, spatial series, final
//! snapshots), joins the event log to the position series with the
//! fail-closed population check, recomputes the patch through the kernel's
//! own schedule accessor, and prints one line per world with the arrival
//! census and the reachability census side by side - no threshold, no
//! verdict (ADR-0016).
//!
//! The campaign is small (two arms, two seeds, 2,000 ticks) and nothing
//! here asserts what a criterion would *decide* - the assertions are about
//! the report's shape, its internal consistency (arrived + censored equals
//! the cohort, the patch cell is a real cell, the census population is the
//! run's population), and its refusals by name. That the command exits
//! zero at all is itself the strongest check in the file: it means the
//! event-log/spatial join reconciled the living population at every
//! sample tick of every world, on artifacts a real campaign wrote.

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

const CAMPAIGN: &str = "\
campaign social-cli
ticks 2000
workers 2
seeds 5301..5302
base preset phase2
base cells_x 64
base cells_y 64
base initial_organisms 120
base max_entities 4000
base genome2.enabled true
base genome2.mutation.point_q16 65535
base genome2.mutation.binding_q16 32768
base worldmod.enabled true
base worldmod.patch_enabled true
base worldmod.relocate_interval_ticks 500
base artifact.enabled true
base social.enabled true
condition A
condition C
set A social.perception_enabled true
set A social.signal_enabled true
set C social.perception_enabled false
set C social.signal_enabled false
vary social.perception_enabled
vary social.signal_enabled
output events on
output snapshots on
output actions off
output spatial 50
";

struct Campaign {
    directory: std::path::PathBuf,
}

impl Campaign {
    fn run(name: &str) -> Self {
        Self::run_spec(name, CAMPAIGN)
    }

    fn run_spec(name: &str, spec: &str) -> Self {
        let directory =
            std::env::temp_dir().join(format!("lifesim-social-cli-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("temp dir");
        let source = directory.join("campaign.txt");
        std::fs::write(&source, spec).expect("write campaign");
        let output = lifesim(&[
            "batch",
            "--campaign",
            source.to_str().unwrap(),
            "--output",
            directory.to_str().unwrap(),
            "--workers",
            "2",
        ]);
        assert!(output.status.success(), "batch: {}", stderr(&output));
        Self { directory }
    }

    fn manifest(&self) -> std::path::PathBuf {
        self.directory.join("manifest.txt")
    }

    fn report(&self, epoch: &str) -> Output {
        lifesim(&[
            "social",
            "--manifest",
            self.manifest().to_str().unwrap(),
            "--epoch",
            epoch,
        ])
    }

    fn contrast(&self, epochs: &str) -> Output {
        lifesim(&[
            "social-contrast",
            "--manifest",
            self.manifest().to_str().unwrap(),
            "--treatment",
            "A",
            "--baseline",
            "C",
            "--epochs",
            epochs,
            "--sesoi",
            "100",
            "--analysis-seed",
            "0x1373",
        ])
    }
}

impl Drop for Campaign {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

fn field(line: &str, key: &str) -> u64 {
    let needle = format!(" {key}=");
    let start = line
        .find(&needle)
        .unwrap_or_else(|| panic!("no {key} in: {line}"))
        + needle.len();
    let rest = &line[start..];
    let end = rest.find(' ').unwrap_or(rest.len());
    rest[..end]
        .parse()
        .unwrap_or_else(|_| panic!("{key} is not a number in: {line}"))
}

#[test]
fn the_report_joins_every_world_and_its_lines_are_internally_consistent() {
    let campaign = Campaign::run("report");
    let output = campaign.report("2");
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let text = stdout(&output);

    assert!(
        text.starts_with("social-report 1 campaign social-cli"),
        "{text}"
    );
    assert!(
        text.contains("detector lifesim-arrival-detector-v1 epoch 2"),
        "{text}"
    );
    let worlds: Vec<&str> = text
        .lines()
        .filter(|line| line.starts_with("world "))
        .collect();
    assert_eq!(worlds.len(), 4, "two arms, two seeds:\n{text}");
    for line in &worlds {
        // Epoch 2 at interval 500: the patch active on [1000,1500).
        assert!(line.contains(" window=[1000,1500) "), "{line}");
        // The cohort partitions exactly into arrived and censored.
        assert_eq!(
            field(line, "naive"),
            field(line, "arrived") + field(line, "censored"),
            "{line}"
        );
        // The patch centre is a real cell of the 64x64 grid.
        assert!(field(line, "patch_cell") < 64 * 64, "{line}");
        // A live population, censused from the snapshot's genomes.
        assert!(field(line, "population") > 0, "{line}");
        // Sampled at the campaign's own declared resolution.
        assert_eq!(field(line, "sample_interval"), 50, "{line}");
        // Speakers and hearers never exceed the population.
        let population = field(line, "population");
        assert!(field(line, "hearers") <= population, "{line}");
        assert!(field(line, "speakers") <= population, "{line}");
    }
    // Both arms are reduced: the report never decides, so C's worlds get
    // the same line A's do.
    assert_eq!(
        worlds
            .iter()
            .filter(|line| line.contains("condition=C"))
            .count(),
        2,
        "{text}"
    );
}

#[test]
fn the_report_refuses_epoch_zero_and_a_window_past_the_run_by_name() {
    let campaign = Campaign::run("refusals");
    let zero = campaign.report("0");
    assert!(!zero.status.success());
    assert!(stderr(&zero).contains("epoch 0"), "{}", stderr(&zero));

    // Epoch 10 puts the window at [5000,5500) in a 2,000-tick run: no
    // samples can exist there, and the refusal names the window.
    let past = campaign.report("10");
    assert!(!past.status.success());
    assert!(stderr(&past).contains("[5000,5500)"), "{}", stderr(&past));
}

#[test]
fn the_contrast_reduces_pairs_and_reports_the_absolute_directed_count() {
    let campaign = Campaign::run("contrast");
    let output = lifesim(&[
        "social-contrast",
        "--manifest",
        campaign.manifest().to_str().unwrap(),
        "--treatment",
        "A",
        "--baseline",
        "C",
        "--epochs",
        "1,2,3",
        "--sesoi",
        "100",
        "--analysis-seed",
        "0x1373",
    ]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let text = stdout(&output);
    assert!(
        text.starts_with("social-contrast 1 campaign social-cli"),
        "{text}"
    );
    // The plan echo carries the pre-registered inputs verbatim.
    assert!(
        text.contains("treatment A baseline C epochs [1, 2, 3] sesoi_milli 100"),
        "{text}"
    );
    // One line per world of each arm, usable or not, then one contrast.
    let world_lines = text
        .lines()
        .filter(|line| line.starts_with("world "))
        .count();
    assert_eq!(world_lines, 4, "{text}");
    let contrast = text
        .lines()
        .find(|line| line.starts_with("contrast "))
        .expect("contrast line");
    // Pairs + unusable accounts for both seeds; every named field parses.
    let pairs = field(contrast, "pairs");
    let unusable = field(contrast, "unusable_seeds");
    assert_eq!(pairs + unusable, 2, "{contrast}");
    let reaching = field(contrast, "reaching_absolute_directed");
    assert!(reaching <= pairs, "{contrast}");
    let p = field(contrast, "absolute_p_milli");
    assert!(p <= 1000, "{contrast}");

    // The same invocation with an epoch set that cannot fit the run
    // refuses by name rather than reducing a partial window.
    let past = lifesim(&[
        "social-contrast",
        "--manifest",
        campaign.manifest().to_str().unwrap(),
        "--treatment",
        "A",
        "--baseline",
        "C",
        "--epochs",
        "10",
        "--sesoi",
        "100",
    ]);
    assert!(!past.status.success());
    assert!(stderr(&past).contains("[5000,5500)"), "{}", stderr(&past));
}

/// A second campaign, tuned so naive organisms actually reach the patch: a
/// smaller world with a wider patch, run long enough for five relocation
/// epochs. The campaign above never produces a single arrival, so every
/// number the contrast derives from it is zero against zero - the shape
/// holds and the arithmetic says nothing.
const CAMPAIGN_ARRIVALS: &str = "\
campaign social-cli-arrivals
ticks 3000
workers 2
seeds 5301..5302
base preset phase2
base cells_x 24
base cells_y 24
base initial_organisms 120
base max_entities 4000
base genome2.enabled true
base genome2.mutation.point_q16 65535
base genome2.mutation.binding_q16 32768
base worldmod.enabled true
base worldmod.patch_enabled true
base worldmod.patch_radius_cells 5
base worldmod.relocate_interval_ticks 500
base artifact.enabled true
base social.enabled true
condition A
condition C
set A social.perception_enabled true
set A social.signal_enabled true
set C social.perception_enabled false
set C social.signal_enabled false
vary social.perception_enabled
vary social.signal_enabled
output events on
output snapshots on
output actions off
output spatial 50
";

/// The same campaign sampled once per relocation interval, so every epoch's
/// window holds exactly one spatial sample: the one at its opening tick.
fn coarse_campaign() -> String {
    CAMPAIGN_ARRIVALS.replace("output spatial 50\n", "output spatial 500\n")
}

fn signed_field(line: &str, key: &str) -> i64 {
    let needle = format!(" {key}=");
    let start = line
        .find(&needle)
        .unwrap_or_else(|| panic!("no {key} in: {line}"))
        + needle.len();
    let rest = &line[start..];
    let end = rest.find(' ').unwrap_or(rest.len());
    rest[..end]
        .parse()
        .unwrap_or_else(|_| panic!("{key} is not a number in: {line}"))
}

fn text_field<'a>(line: &'a str, key: &str) -> &'a str {
    let needle = format!(" {key}=");
    let start = line
        .find(&needle)
        .unwrap_or_else(|| panic!("no {key} in: {line}"))
        + needle.len();
    let rest = &line[start..];
    &rest[..rest.find(' ').unwrap_or(rest.len())]
}

/// The contrast's numbers are the arithmetic of the reports beside them,
/// never an independent second path to the same claim. Two identities are
/// checked against artifacts a real campaign wrote:
///
/// - each world's `fraction_milli` is the mean, over the epochs whose naive
///   cohort was non-empty, of the SAME per-epoch census the `social` report
///   prints - which pins the epoch window's bounds, the mean's denominator,
///   and the empty-cohort exclusion at once;
/// - the `contrast` line is the paired arithmetic of the world lines above
///   it - which pins which arm is the treatment and which the control.
///
/// The two guard assertions matter as much as the identities: with no
/// empty-cohort epoch the denominator is untested, and with no non-zero
/// paired difference the two arms are interchangeable.
#[test]
fn the_contrast_is_the_arithmetic_of_the_per_epoch_reports_and_the_world_lines() {
    let campaign = Campaign::run_spec("arithmetic", CAMPAIGN_ARRIVALS);
    let epochs = [1_u64, 2, 3, 4, 5];

    // Per (condition, seed): (naive cohort size, arrival fraction) by epoch.
    let mut per_epoch: std::collections::BTreeMap<(String, String), Vec<(u64, i64)>> =
        std::collections::BTreeMap::new();
    for epoch in epochs {
        let output = campaign.report(&epoch.to_string());
        assert!(
            output.status.success(),
            "epoch {epoch}: {}",
            stderr(&output)
        );
        for line in stdout(&output).lines().filter(|l| l.starts_with("world ")) {
            let key = (
                text_field(line, "condition").to_owned(),
                text_field(line, "seed").to_owned(),
            );
            per_epoch.entry(key).or_default().push((
                field(line, "naive"),
                signed_field(line, "arrival_fraction_milli"),
            ));
        }
    }
    assert_eq!(per_epoch.len(), 4, "two arms, two seeds");

    let output = campaign.contrast("1,2,3,4,5");
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let text = stdout(&output);

    let mut saw_an_empty_cohort_epoch = false;
    let mut fractions: std::collections::BTreeMap<(String, String), i64> =
        std::collections::BTreeMap::new();
    for line in text.lines().filter(|l| l.starts_with("world ")) {
        let key = (
            text_field(line, "condition").to_owned(),
            text_field(line, "seed").to_owned(),
        );
        let census = per_epoch.get(&key).expect("a report line for every world");
        let contributing: Vec<i64> = census
            .iter()
            .filter(|(naive, _)| *naive > 0)
            .map(|&(_, fraction)| fraction)
            .collect();
        assert!(!contributing.is_empty(), "{line}");
        if contributing.len() < epochs.len() {
            saw_an_empty_cohort_epoch = true;
        }
        assert_eq!(
            signed_field(line, "fraction_milli"),
            contributing.iter().sum::<i64>() / contributing.len() as i64,
            "{line}"
        );
        assert_eq!(
            field(line, "epochs_contributing"),
            contributing.len() as u64,
            "{line}"
        );
        fractions.insert(key, signed_field(line, "fraction_milli"));
    }
    assert!(
        saw_an_empty_cohort_epoch,
        "the epoch set must include an empty-cohort epoch, or neither the \
         mean's denominator nor the empty-cohort exclusion is tested:\n{text}"
    );

    let seeds: std::collections::BTreeSet<String> =
        fractions.keys().map(|(_, seed)| seed.clone()).collect();
    let differences: Vec<i64> = seeds
        .iter()
        .filter_map(|seed| {
            let treatment = fractions.get(&("A".to_owned(), seed.clone()))?;
            let control = fractions.get(&("C".to_owned(), seed.clone()))?;
            Some(treatment - control)
        })
        .collect();
    assert!(
        differences.iter().any(|difference| *difference != 0),
        "the campaign must produce a non-zero paired difference, or which arm \
         is the control cannot be seen:\n{text}"
    );
    let contrast = text
        .lines()
        .find(|line| line.starts_with("contrast "))
        .expect("contrast line");
    assert_eq!(
        field(contrast, "pairs"),
        differences.len() as u64,
        "{contrast}"
    );
    assert_eq!(
        signed_field(contrast, "mean_difference_milli"),
        differences.iter().sum::<i64>() / differences.len() as i64,
        "{contrast}"
    );
    assert_eq!(
        field(contrast, "positive_differences"),
        differences.iter().filter(|d| **d > 0).count() as u64,
        "{contrast}"
    );
}

/// A world whose every named epoch had an empty naive cohort is unusable:
/// named by seed, counted, and kept out of the pairs - never imputed as a
/// zero fraction. Sampled once per relocation interval, every epoch window
/// holds only its opening tick, where no organism born strictly after that
/// tick exists yet, so every world is unusable and the accounting shows.
#[test]
fn an_unusable_world_is_named_by_seed_and_kept_out_of_the_pairs() {
    let campaign = Campaign::run_spec("unusable", &coarse_campaign());
    let output = campaign.contrast("1,2");
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let text = stdout(&output);
    let unusable_worlds = text
        .lines()
        .filter(|line| line.starts_with("world ") && line.contains(" unusable "))
        .count();
    assert_eq!(unusable_worlds, 4, "{text}");
    let contrast = text
        .lines()
        .find(|line| line.starts_with("contrast "))
        .expect("contrast line");
    assert_eq!(field(contrast, "pairs"), 0, "{contrast}");
    assert_eq!(field(contrast, "unusable_seeds"), 2, "{contrast}");
}

/// The epoch window's lower bound is inclusive: a window holding only the
/// sample at its own opening tick is still a window, not an absent one. No
/// naive organism can appear at that tick - naive means born strictly after
/// it - so this bound is invisible in every count the contrast reports and
/// visible only in whether the window exists at all.
#[test]
fn a_window_holding_only_its_opening_sample_is_still_a_window() {
    let campaign = Campaign::run_spec("opening", &coarse_campaign());
    let output = campaign.contrast("1");
    assert!(
        output.status.success(),
        "epoch 1's window holds exactly the sample at its opening tick: {}",
        stderr(&output)
    );
}

/// The contrast refuses epoch 0 by name. Epoch 0's window opens at tick 0,
/// where "born strictly after the threshold" admits every founder and the
/// naive cohort would be the whole starting population. The refusal
/// precedes every read, so no campaign is needed to reach it.
#[test]
fn the_contrast_refuses_epoch_zero_by_name() {
    let output = lifesim(&[
        "social-contrast",
        "--manifest",
        "/nonexistent/manifest.txt",
        "--treatment",
        "A",
        "--baseline",
        "C",
        "--epochs",
        "0",
        "--sesoi",
        "100",
    ]);
    assert!(!output.status.success(), "{}", stdout(&output));
    assert!(
        stderr(&output).contains("none of them 0"),
        "{}",
        stderr(&output)
    );
}
