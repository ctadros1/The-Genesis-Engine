//! Phase 11 `lifesim plasticity`: the analysis reads a real campaign's
//! artifacts, and its two provenance guards are pinned by their own
//! diagnostics.
//!
//! # Why the two guards are tested separately, and pinned by message
//!
//! Both compare something against the run record's `config_hash`: the action
//! log's header, and the config the snapshot restored. **Two layers that can
//! refuse the same input** is exactly the shape that lets a guard be deleted
//! without any test failing - an assertion that the command merely failed
//! passes whichever one fired. So each is reached by an input the other cannot
//! see (a patched manifest for the near one, a re-encoded snapshot for the far
//! one) and each is asserted on the text it alone prints.
//!
//! The far guard is the one that matters most for the report's honesty. An
//! earlier generation of this command family used `.ok()` on the restore and
//! reported "no organism was mature" when the truth was "the config did not
//! survive the codec". Here the equivalent silent failure would be a zero
//! allele census read as "plasticity never arose", which is the opposite
//! conclusion from "the marker gate was lost in the codec".

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
campaign plasticity-cli
ticks 6000
workers 2
seeds 5101..5103
base preset phase2
base cells_x 64
base cells_y 64
base initial_organisms 120
base max_entities 4000
base genome2.enabled true
base genome2.mutation.point_q16 65535
base probe.enabled true
base probe.action_census_enabled true
base probe.marker_locus_enabled true
base worldmod.enabled true
base worldmod.patch_enabled true
base worldmod.relocate_interval_ticks 1000
base worldmod.patch_radius_cells 8
base plasticity.enabled true
base genome2.mutation.plasticity_enabled true
condition A
condition B
condition N
set A worldmod.patch_capacity_scale_q16 262144
set B worldmod.patch_capacity_scale_q16 262144
set B plasticity.enabled false
set B genome2.mutation.plasticity_enabled false
set N worldmod.patch_capacity_scale_q16 262144
set N worldmod.patch_enabled false
vary plasticity.enabled
vary genome2.mutation.plasticity_enabled
vary worldmod.patch_enabled
output events off
output snapshots on
output actions 250
";

struct Campaign {
    directory: std::path::PathBuf,
}

impl Campaign {
    fn run(name: &str) -> Self {
        let directory = std::env::temp_dir().join(format!(
            "lifesim-plasticity-cli-{name}-{}",
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

    fn analyse(&self) -> Output {
        lifesim(&[
            "plasticity",
            "--manifest",
            self.manifest().to_str().unwrap(),
            "--treatment",
            "A",
            "--baseline",
            "B",
            "--burn-in",
            "1000",
        ])
    }
}

impl Drop for Campaign {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

#[test]
fn the_report_reduces_every_condition_and_decides_only_the_named_pair() {
    let campaign = Campaign::run("report");
    let output = campaign.analyse();
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let text = stdout(&output);

    assert!(text.starts_with("plasticity-report 1 campaign plasticity-cli"));
    assert!(text.contains("analysis-version lifesim-plasticity-analysis-v1"));
    assert!(text.contains("census-policy lifesim-action-census-v1"));
    assert!(text.contains("columns rest,move_ahead,turn_left,turn_right,eat,mate,attack"));
    // The plan is echoed so a reader can check the bars against the campaign
    // source rather than trusting the program's arithmetic.
    assert!(text.contains("drift_margin_milli=25"));
    assert!(text.contains("permutations=199"));

    // Every condition is reduced; only the named pair is decided.
    assert_eq!(text.matches("condition A worlds=3").count(), 1);
    assert_eq!(text.matches("condition B worlds=3").count(), 1);
    assert_eq!(text.matches("condition N worlds=3").count(), 1);
    assert!(text.contains("criterion C11.1 treatment=A control=B"));
    assert!(text.contains("criterion C11.2 treatment=A control=B"));

    // Nine world lines and nine drift lines, and the shift statistic was
    // actually computable in the two scheduled conditions - a report of
    // refusals would satisfy every assertion above.
    assert_eq!(text.matches("world condition=").count(), 9);
    assert_eq!(text.matches("drift condition=").count(), 9);
    assert!(
        text.lines()
            .filter(|line| line.starts_with("world condition=A")
                || line.starts_with("world condition=B"))
            .all(|line| line.contains("boundaries=")),
        "a scheduled world refused, so nothing above was tested:\n{text}"
    );
    // **Condition N runs no relocation schedule at all, and is refused by
    // name rather than scored.** Without this the analysis would happily
    // manufacture boundaries in a world where nothing ever moved and report
    // whatever the correlation happened to be - a null with no event in it,
    // which is the most misleading result this command could print.
    assert_eq!(
        text.lines()
            .filter(|line| line.starts_with("world condition=N"))
            .filter(|line| line.contains("refused=no_schedule"))
            .count(),
        3,
        "a schedule-free world was analysed as if it had a schedule:\n{text}"
    );
    assert!(text.contains("condition N worlds=3 extinct=0 shifted=0 associated=0"));
    assert!(
        text.contains("shift_refused=3"),
        "the refusals were not counted in the condition summary:\n{text}"
    );
    // ...and the instrument moved. `varying_columns=0` everywhere would mean
    // C11.1 was measured against a constant, which is the trap Phase 11's
    // substrate was rebuilt to avoid.
    assert!(
        text.lines()
            .any(|line| line.starts_with("world ") && !line.contains(" varying_columns=0 ")),
        "no world's action columns varied:\n{text}"
    );
    // The marker locus reached the genomes, or C11.2's control never ran.
    assert!(
        text.lines()
            .any(|line| line.starts_with("drift ") && !line.contains(" marker_alleles=0 ")),
        "no world carried a marker allele:\n{text}"
    );
}

#[test]
fn an_action_log_from_another_world_is_refused_by_the_guard_that_names_the_log() {
    // The near guard. Patching the manifest's recorded config hash makes the
    // action log's header disagree with the run record, and this guard is the
    // first that can see it.
    let campaign = Campaign::run("logguard");
    let path = campaign.manifest();
    let text = std::fs::read_to_string(&path).expect("manifest");
    // Overwrite the sixteen hex digits in place: prepending would produce a
    // value the manifest parser rejects, and a parse refusal is a different
    // guard from the one under test.
    let marker = "config_hash=0x";
    let at = text
        .find(marker)
        .expect("the manifest records a config hash")
        + marker.len();
    let mut patched = text.clone();
    patched.replace_range(at..at + 16, "deadbeefdeadbeef");
    assert_ne!(patched, text, "the manifest had no config hash to patch");
    std::fs::write(&path, patched).expect("write manifest");

    let output = campaign.analyse();
    assert!(!output.status.success(), "stdout: {}", stdout(&output));
    let message = stderr(&output);
    assert!(
        message.contains("the action log's config hash"),
        "a different guard fired: {message}"
    );
}

#[test]
fn a_snapshot_whose_config_did_not_survive_the_codec_is_refused_by_name() {
    // The far guard, reached by an input the near one cannot see: the manifest
    // and the action log are untouched and only the snapshot's stored config
    // is changed, so this is the only layer that can refuse.
    //
    // `probe.marker_locus_enabled` is the field flipped on purpose. Losing it
    // in the codec produces an empty marker census, and an empty marker census
    // read without this guard says "the drift control never moved" when the
    // truth is "the drift control was never enabled in the restored world".
    let campaign = Campaign::run("snapguard");
    let snapshot = campaign.directory.join("A-seed00000000000013ed.alif");
    let bytes = std::fs::read(&snapshot).expect("snapshot exists");
    let (info, mut state) = sim_persist::decode_snapshot(&bytes).expect("snapshot decodes");
    assert!(
        state.config.probe.marker_locus_enabled,
        "the campaign did not enable the marker, so flipping it proves nothing"
    );
    state.config.probe.marker_locus_enabled = false;
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

    let output = campaign.analyse();
    assert!(!output.status.success(), "stdout: {}", stdout(&output));
    let message = stderr(&output);
    assert!(
        message.contains("the config did not survive the snapshot"),
        "a different guard fired: {message}"
    );
    assert!(
        message.contains("probe.marker_locus_enabled=false"),
        "the guard did not name the field that changed: {message}"
    );
}

#[test]
fn the_command_refuses_a_condition_it_cannot_find_and_a_self_contrast() {
    let campaign = Campaign::run("names");
    let manifest = campaign.manifest();
    let manifest = manifest.to_str().unwrap();
    let missing = lifesim(&[
        "plasticity",
        "--manifest",
        manifest,
        "--treatment",
        "A",
        "--baseline",
        "nope",
    ]);
    assert!(!missing.status.success());
    assert!(stderr(&missing).contains("no condition named 'nope'"));

    let same = lifesim(&[
        "plasticity",
        "--manifest",
        manifest,
        "--treatment",
        "A",
        "--baseline",
        "A",
    ]);
    assert!(!same.status.success());
    assert!(stderr(&same).contains("name the same condition"));
}
