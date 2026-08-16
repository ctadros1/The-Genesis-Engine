//! Phase 12 `lifesim artifact`: the C12.1-C12.3 analysis reads a real
//! campaign's artifacts (manifest, event logs, final snapshots), echoes the
//! pre-registered plan, reduces every world, decides only the named arms,
//! prints the reachability census beside the verdicts, and refuses by name
//! when the snapshot's config did not survive the codec or an arm is not in
//! the campaign.
//!
//! The campaign is small (four arms, two seeds, 3,000 ticks) and nothing here
//! asserts what the criteria *decide* - a two-seed toy has no bearing on the
//! pre-registered thresholds and the assertions say so by testing the shape
//! of the report, its guards, and that the census is read from genomes that
//! the bind operator reached.

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
campaign artifact-cli
ticks 3000
workers 2
seeds 5201..5202
base preset phase2
base cells_x 64
base cells_y 64
base initial_organisms 120
base max_entities 4000
base genome2.enabled true
base genome2.mutation.point_q16 65535
base genome2.mutation.binding_q16 32768
base contest.enabled true
base worldmod.enabled true
base worldmod.patch_enabled false
base artifact.enabled true
condition A
condition B
condition C
condition D
set A artifact.max_composition_depth 4
set A artifact.ephemeral false
set A artifact.inert false
set B artifact.max_composition_depth 4
set B artifact.ephemeral true
set B artifact.inert false
set C artifact.max_composition_depth 4
set C artifact.ephemeral false
set C artifact.inert true
set D artifact.max_composition_depth 0
set D artifact.ephemeral false
set D artifact.inert false
vary artifact.max_composition_depth
vary artifact.ephemeral
vary artifact.inert
output events on
output snapshots on
output actions off
";

struct Campaign {
    directory: std::path::PathBuf,
}

impl Campaign {
    fn run(name: &str) -> Self {
        let directory =
            std::env::temp_dir().join(format!("lifesim-artifact-cli-{name}-{}", std::process::id()));
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

    fn analyse(&self, treatment: &str, control: &str, disabled: &str) -> Output {
        lifesim(&[
            "artifact",
            "--manifest",
            self.manifest().to_str().unwrap(),
            "--treatment",
            treatment,
            "--baseline",
            control,
            "--disabled",
            disabled,
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
    let start = line.find(&needle).unwrap_or_else(|| panic!("no {key} in: {line}")) + needle.len();
    let rest = &line[start..];
    let end = rest.find(' ').unwrap_or(rest.len());
    rest[..end].parse().unwrap_or_else(|_| panic!("{key} is not a number in: {line}"))
}

#[test]
fn the_report_echoes_the_plan_reduces_every_world_and_prints_the_census() {
    let campaign = Campaign::run("report");
    let output = campaign.analyse("A", "C", "D");
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let text = stdout(&output);

    assert!(text.starts_with("artifact analysis lifesim-artifact-analysis-v1 campaign artifact-cli"), "{text}");
    // The pre-registered constants are echoed, so a reader checks the bars
    // against the document rather than trusting the arithmetic.
    assert!(text.contains("plan sesoi_c121_ppm=10 bar_c121=20 bar_c122_lifetime=15 bar_c122_fitness=20"), "{text}");
    assert!(text.contains("exposure_floor_milli=50 exposure_min_organisms=20 sesoi_c123_milli=500 bar_c123=20"), "{text}");
    assert!(text.contains("seeds=2 analysis_seed=0xa11fac750b1ec751"), "{text}");

    // Eight world lines, each with the census fields; four reachability
    // lines, one per condition; the six verdict/paired lines.
    assert_eq!(text.matches("\nworld condition=").count() + usize::from(text.starts_with("world condition=")), 8, "{text}");
    for condition in ["A", "B", "C", "D"] {
        assert_eq!(text.matches(&format!("world condition={condition} ")).count(), 2, "{text}");
        assert_eq!(text.matches(&format!("reachability condition={condition} worlds=2 ")).count(), 1, "{text}");
    }
    for line in text.lines().filter(|line| line.starts_with("world condition=")) {
        for key in [
            "organism_ticks", "successes", "fires", "success_rate_ppm", "fire_rate_ppm",
            "placed_episodes", "organism_lifespans", "depth2_ever", "census_population",
            "binds_pick_up", "binds_place", "binds_combine", "pick_up_and_place",
            "pick_up_and_combine", "binding_applied",
        ] {
            let _ = field(line, key);
        }
        assert!(field(line, "organism_ticks") > 0, "a world ran no organism-ticks: {line}");
        assert!(field(line, "organism_lifespans") > 0, "no lifespan was observed: {line}");
    }
    assert!(text.contains("C12.1 count="), "{text}");
    assert!(text.contains("C12.1 success-rate A-C pairs=2 "), "{text}");
    assert!(text.contains("C12.1 fire-rate A-C (supplementary) pairs=2 "), "{text}");
    assert!(text.contains("C12.2 met="), "{text}");
    assert!(text.contains("C12.3 count="), "{text}");
    assert!(text.contains("C12.3 condition D zero by construction: true"), "{text}");
    assert!(text.contains("medians over A worlds:"), "{text}");

    // The census is read from genomes the bind operator actually reached:
    // at 0.5 per birth over 3,000 ticks the manifest's binding_applied is
    // nonzero somewhere, and the reachability line carries it. Without this
    // a census of all zeros would pass every shape assertion above while
    // reading nothing.
    let applied: u64 = text
        .lines()
        .filter(|line| line.starts_with("reachability condition="))
        .map(|line| field(line, "binding_applied_total"))
        .sum();
    assert!(applied > 0, "no bind operation was applied in any arm, so the census read unbound genomes:\n{text}");
}

#[test]
fn a_snapshot_whose_config_did_not_survive_the_codec_is_refused_by_name() {
    let campaign = Campaign::run("snapguard");
    let snapshot = campaign.directory.join("A-seed0000000000001451.alif");
    let bytes = std::fs::read(&snapshot).expect("snapshot exists");
    let (info, mut state) = sim_persist::decode_snapshot(&bytes).expect("snapshot decodes");
    assert_eq!(state.config.artifact.max_composition_depth, 4);
    // Flip a field the manifest's hash covers and nothing downstream reads
    // before the guard: the analysis would otherwise report condition A as
    // condition D.
    state.config.artifact.max_composition_depth = 0;
    let reencoded = sim_persist::encode_snapshot(
        &state,
        info.world_id,
        info.parent_world_id,
        info.state_checksum,
        &info.build_version,
        info.event_log_offset,
        None,
    )
    .expect("re-encodes");
    std::fs::write(&snapshot, reencoded).expect("write snapshot");

    let output = campaign.analyse("A", "C", "D");
    assert!(!output.status.success(), "stdout: {}", stdout(&output));
    let message = stderr(&output);
    assert!(
        message.contains("the snapshot's config hash") && message.contains("is not the manifest's"),
        "a different guard fired: {message}"
    );
}

#[test]
fn the_command_refuses_an_arm_it_cannot_find_and_a_missing_manifest() {
    let campaign = Campaign::run("arms");
    let output = campaign.analyse("A", "C", "Z");
    assert!(!output.status.success());
    assert!(stderr(&output).contains("no condition named 'Z'"), "{}", stderr(&output));
    let output = lifesim(&[
        "artifact",
        "--manifest",
        campaign.directory.join("absent.txt").to_str().unwrap(),
        "--treatment",
        "A",
        "--baseline",
        "C",
        "--disabled",
        "D",
    ]);
    assert!(!output.status.success());
    assert!(stdout(&output).is_empty(), "printed a report for a manifest that does not exist");
}
