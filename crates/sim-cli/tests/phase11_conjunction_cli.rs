//! Phase 11 follow-up `lifesim conjunction`: the descriptive census reads a
//! real campaign's snapshots, labels itself, and refuses a snapshot that is
//! not what the manifest says it is.
//!
//! # What is tested here and what is tested in the unit tests
//!
//! `sim-analysis::conjunction`'s unit tests own the counting: which of the
//! four conditions is load-bearing, the Oja exemption, the modulator gate,
//! the depth histogram, and the learned-state counts against a mean that
//! rounds to zero. All of them are mutation-tested there against synthetic
//! genomes, where a planted answer is exact.
//!
//! What cannot be tested there is that the numbers describe the world that
//! actually ran. Three things have to hold and each is asserted below against
//! a campaign this test executes:
//!
//! - the report says on its face that it is a census and not a test, because
//!   a table of counts detached from that line reads exactly like a result;
//! - the census walks a genome that the codec really produced, so the guard
//!   is pinned by the diagnostic **only it** prints - not by "the command
//!   failed", which would pass whichever of several layers fired;
//! - the re-compiled plan agrees with the restored world's own
//!   `plastic_edges_total`. That equality is the whole warrant for reading
//!   the expressed census as "what the learn phase visited" rather than as
//!   "what this analysis imagines it visited", and it is printed on every
//!   line so a reader can check it without trusting the program.

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

/// A small two-arm campaign with the plasticity factor crossed exactly as the
/// confirmatory campaign crosses it, so the B arm carries no learned-state
/// section at all and the "absent, not zero" path is exercised.
const CAMPAIGN: &str = "\
campaign conjunction-cli
ticks 12000
workers 2
seeds 5201..5202
base preset phase2
base cells_x 96
base cells_y 96
base initial_organisms 200
base max_entities 12000
base genome2.enabled true
base genome2.mutation.point_q16 65535
base probe.enabled true
base probe.marker_locus_enabled true
base plasticity.max_plastic_edges 32
base plasticity.plastic_edge_cost_milli_per_s 2
condition A
condition B
set A plasticity.enabled true
set A genome2.mutation.plasticity_enabled true
set B plasticity.enabled false
set B genome2.mutation.plasticity_enabled false
vary plasticity.enabled
vary genome2.mutation.plasticity_enabled
output events off
output snapshots on
";

struct Campaign {
    directory: std::path::PathBuf,
}

impl Campaign {
    fn run(name: &str) -> Self {
        let directory = std::env::temp_dir().join(format!(
            "lifesim-conjunction-cli-{name}-{}",
            std::process::id()
        ));
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

    fn census(&self) -> Output {
        lifesim(&[
            "conjunction",
            "--manifest",
            self.manifest().to_str().unwrap(),
        ])
    }
}

impl Drop for Campaign {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

/// Read `key=value` off a report line, as an integer.
fn field(line: &str, key: &str) -> u64 {
    let needle = format!(" {key}=");
    let at = line
        .find(&needle)
        .unwrap_or_else(|| panic!("no {key} in: {line}"))
        + needle.len();
    let rest = &line[at..];
    let end = rest.find(' ').unwrap_or(rest.len());
    rest[..end]
        .parse()
        .unwrap_or_else(|_| panic!("{key} is not an integer in: {line}"))
}

#[test]
fn the_census_labels_itself_and_its_counts_are_internally_consistent() {
    let campaign = Campaign::run("report");
    let output = campaign.census();
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let text = stdout(&output);

    assert!(text.starts_with("conjunction-census 1 campaign conjunction-cli"));
    assert!(text.contains("analysis-version lifesim-conjunction-census-v1"));
    // **The label is load-bearing.** This command is run after the criteria
    // are decided, on the same data, so a report of it that read as a test
    // would be a threshold chosen with the answer in view.
    assert!(
        text.contains("DESCRIPTIVE CENSUS") && text.contains("Not a hypothesis test"),
        "the report does not say what it is:\n{text}"
    );
    // The census must not print anything shaped like a decision. Checked on
    // the report's data lines only: the header says "no threshold, no null,
    // no verdict" and searching the whole text for the word would find its
    // own disclaimer.
    for line in text
        .lines()
        .filter(|line| !line.starts_with("kind ") && !line.starts_with("conditions "))
    {
        for forbidden in ["criterion", "verdict", "UNMET", " MET", "bar=", "p_milli="] {
            assert!(
                !line.contains(forbidden),
                "the census printed something that reads as a decision: {line}"
            );
        }
    }

    // Four worlds, each with an allele line, an expressed line and a learned
    // line; two arms.
    assert_eq!(text.matches("world condition=").count(), 4);
    assert_eq!(text.matches("expressed condition=").count(), 4);
    assert_eq!(text.matches("learned condition=").count(), 4);

    for line in text.lines().filter(|line| line.starts_with("world ")) {
        // Every allele lands in exactly one rung of the depth histogram, or
        // the histogram is not a partition and a plateau read off it means
        // nothing.
        let depth: u64 = (0..5)
            .map(|rung| field(line, &format!("depth{rung}")))
            .sum();
        assert_eq!(
            depth,
            field(line, "edge_alleles"),
            "depth partition: {line}"
        );
        // The rule histogram is over the same alleles.
        let rules: u64 = (0..5).map(|rule| field(line, &format!("rule{rule}"))).sum();
        assert_eq!(rules, field(line, "edge_alleles"), "rule histogram: {line}");
        // Every marginal is bounded by the total and the conjunction by every
        // marginal: a count of complete alleles larger than the count of
        // flagged ones would be arithmetic nobody could read.
        for marginal in ["flagged", "non_static", "eta_positive", "drive"] {
            assert!(
                field(line, marginal) <= field(line, "edge_alleles"),
                "{line}"
            );
            assert!(
                field(line, "full_conjunction") <= field(line, marginal),
                "full_conjunction exceeds {marginal}: {line}"
            );
        }
        assert!(field(line, "full_conjunction_gated") <= field(line, "full_conjunction"));
        assert!(field(line, "edge_alleles") <= field(line, "loci"));
        assert_eq!(field(line, "out_of_registry"), 0);
    }

    // **The equality that makes the expressed census mean anything.** The
    // analysis re-expresses and re-compiles every genome; if that did not
    // reproduce the plan the learn phase ran, every expressed number would be
    // about a different network.
    for line in text.lines().filter(|line| line.starts_with("expressed ")) {
        assert_eq!(
            field(line, "plastic_edges"),
            field(line, "metric_plastic_edges"),
            "the recompiled plan disagrees with the world: {line}"
        );
    }

    // The A arm ran the mechanism and the B arm carries no section at all -
    // which is not the same as a section of zeros, and the report has to say
    // which. Without this the "absent" path could return zeros forever.
    assert_eq!(
        text.lines()
            .filter(|line| line.starts_with("learned condition=B"))
            .filter(|line| line.contains("section=absent"))
            .count(),
        2,
        "the plasticity-disabled arm should carry no learned section:\n{text}"
    );
    assert_eq!(
        text.lines()
            .filter(|line| line.starts_with("learned condition=A"))
            .filter(|line| line.contains("section=present"))
            .count(),
        2
    );
    let arm_a = text
        .lines()
        .find(|line| line.starts_with("arm A "))
        .expect("an A arm line");
    // The mutation gate reached the plasticity genes at all. Every count in
    // this report would be zero in a world where point mutation could not
    // write them, and a report of zeros would be indistinguishable from the
    // finding it exists to state.
    assert!(
        field(arm_a, "flagged") > 0,
        "no plasticity gene moved in the treatment arm, so nothing above was \
         measured against a live mechanism:\n{arm_a}"
    );
    // ...and a flagged allele actually became a plastic edge in a compiled
    // plan, so the expressed and learned halves ran against a real learn
    // phase rather than against an empty list.
    let arm_a_expressed = text
        .lines()
        .find(|line| line.starts_with("arm-expressed A "))
        .expect("an A expressed arm line");
    assert!(
        field(arm_a_expressed, "plastic_edges") > 0,
        "no edge compiled plastic in the treatment arm:\n{arm_a_expressed}"
    );
    let arm_a_learned = text
        .lines()
        .find(|line| line.starts_with("arm-learned A "))
        .expect("an A learned arm line");
    assert!(
        field(arm_a_learned, "rows") > 0 && field(arm_a_learned, "metric_updates") > 0,
        "the learn phase never ran in the treatment arm:\n{arm_a_learned}"
    );
    let arm_b = text
        .lines()
        .find(|line| line.starts_with("arm B "))
        .expect("a B arm line");
    assert_eq!(field(arm_b, "flagged"), 0);
    assert_eq!(field(arm_b, "full_conjunction"), 0);
    assert!(field(arm_b, "edge_alleles") > 0, "the B arm had no edges");
}

#[test]
fn a_snapshot_whose_config_did_not_survive_the_codec_is_refused_by_this_command_s_own_name() {
    // `command_plasticity` has a guard of the same shape, so "the command
    // failed" would prove nothing about this one. The diagnostic asserted
    // below is unique to the census: it names the conclusion that would have
    // been drawn wrongly here - an unassembled phenotype - rather than the
    // empty allele census the other command would have reported.
    let campaign = Campaign::run("snapguard");
    let snapshot = campaign.directory.join("A-seed0000000000001451.alif");
    let bytes = std::fs::read(&snapshot).expect("snapshot exists");
    let (info, mut state) = sim_persist::decode_snapshot(&bytes).expect("snapshot decodes");
    // The field flipped is the one whose loss would silently empty this
    // report: with the budget gone, no edge compiles plastic and every
    // expressed count reads zero for a reason that is about the codec.
    assert_eq!(state.config.plasticity.max_plastic_edges, 32);
    state.config.plasticity.max_plastic_edges = 31;
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

    let output = campaign.census();
    assert!(!output.status.success(), "stdout: {}", stdout(&output));
    let message = stderr(&output);
    assert!(
        message.contains(
            "a zero conjunction count here would mean a codec defect and not an \
                          unassembled phenotype"
        ),
        "a different guard fired: {message}"
    );
    assert!(
        message.contains("max_plastic_edges=31"),
        "the guard did not name the field that changed: {message}"
    );
}

#[test]
fn the_command_refuses_a_missing_manifest_rather_than_printing_an_empty_census() {
    let output = lifesim(&["conjunction"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("conjunction requires --manifest"));
}
