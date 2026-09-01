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
        let directory =
            std::env::temp_dir().join(format!("lifesim-social-cli-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("temp dir");
        let source = directory.join("campaign.txt");
        std::fs::write(&source, CAMPAIGN).expect("write campaign");
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
