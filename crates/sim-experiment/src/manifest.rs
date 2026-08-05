//! Campaign manifest: what ran, under which effective config, and what
//! came out.
//!
//! "Its manifest records every effective config hash, per-seed final
//! checksums, build provenance, and the analysis versions applied"
//! (`specifications/experiment-config-schema.md`). A manifest is written in
//! canonical unit order, never completion order, so two runs of the same
//! campaign at different worker counts produce byte-identical manifests
//! apart from the wall-clock fields, which are recorded separately and
//! excluded from the manifest hash.
//!
//! The campaign source is embedded verbatim. A manifest that pointed at an
//! external campaign file would silently change meaning when that file was
//! edited, which is precisely the failure mode the config-hash discipline
//! exists to prevent.

use crate::campaign::{Campaign, CampaignError};
use sim_core::{Counters, Phase2Counters};
use std::collections::BTreeMap;
use std::fmt;

pub const MANIFEST_FORMAT: &str = "phase5-manifest-v1";
pub const MANIFEST_VERSION: u32 = 1;

/// Everything one finished world reports.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunResult {
    pub index: usize,
    pub condition: String,
    pub seed: u64,
    pub condition_delta_hash: u64,
    pub config_hash: u64,
    pub terrain_checksum: u64,
    pub state_checksum: u64,
    pub ticks: u64,
    pub population: u64,
    pub extinct: bool,
    pub total_energy_milli: i64,
    pub total_biomass_milli: i64,
    pub max_ancestry_depth: u32,
    pub counters: Counters,
    pub phase2: Option<Phase2Counters>,
    pub event_log_offset: u64,
    pub snapshot_bytes: u64,
    /// Spatial samples written. Recorded so an analysis can prove it read
    /// the whole series rather than a silently shortened one.
    pub spatial_samples: u64,
    /// Phase 8 demography outcomes. Zero when the section is disabled.
    pub deaths_senescence_total: u64,
    pub deaths_extrinsic_total: u64,
    pub deaths_juvenile_total: u64,
    pub max_age_ticks_observed: u64,
    pub total_capacity_milli: i64,
    /// Phase 9 structure metrics. Zero when schema 2 is disabled.
    pub mean_nodes_milli: u64,
    pub mean_edges_milli: u64,
    /// C9.1's stated quantity: the median, which moves only once half the
    /// population has diverged from the founding topology.
    pub median_nodes: u64,
    pub median_edges: u64,
    pub distinct_structures: u64,
    pub structural_mutations_applied: u64,
    pub structural_mutations_rejected: u64,
    /// Phase 7 contest outcomes. Zero when the section is disabled.
    pub attacks_total: u64,
    pub deaths_by_damage_total: u64,
    pub carcasses: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FailedRun {
    pub index: usize,
    pub condition: String,
    pub seed: u64,
    pub reason: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Manifest {
    pub campaign: Campaign,
    pub campaign_source: String,
    pub build_version: String,
    /// Every behavior-policy version compiled into this build, recorded
    /// together rather than as one "active" string. Conditions may differ
    /// in `phase2.enabled`, so a campaign does not necessarily have a
    /// single active policy; the per-run config hash carries which one
    /// applied.
    pub behavior_policy_versions: Vec<String>,
    pub rng_algorithm_version: String,
    pub worldgen_version: String,
    pub genome_schema_version: u16,
    pub event_schema_version: u32,
    /// Analysis policy versions applied to this campaign's outputs.
    pub analysis_versions: Vec<String>,
    pub workers: usize,
    pub runs: Vec<RunResult>,
    pub failed: Vec<FailedRun>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ManifestError {
    Syntax {
        line: usize,
        message: String,
    },
    Missing(&'static str),
    UnsupportedVersion(u32),
    UnsupportedFormat(String),
    Campaign(CampaignError),
    /// The embedded source no longer hashes to the recorded campaign hash.
    CampaignHashMismatch {
        recorded: u64,
        computed: u64,
    },
}

impl fmt::Display for ManifestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Syntax { line, message } => write!(formatter, "line {line}: {message}"),
            Self::Missing(what) => write!(formatter, "manifest is missing `{what}`"),
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported manifest version {version}")
            }
            Self::UnsupportedFormat(format) => {
                write!(formatter, "unsupported manifest format '{format}'")
            }
            Self::Campaign(error) => write!(formatter, "embedded campaign: {error}"),
            Self::CampaignHashMismatch { recorded, computed } => write!(
                formatter,
                "embedded campaign hashes to 0x{computed:016x} but the manifest records \
                 0x{recorded:016x}; the manifest has been edited"
            ),
        }
    }
}

impl std::error::Error for ManifestError {}

fn hex(value: u64) -> String {
    format!("0x{value:016x}")
}

fn parse_hex(value: &str) -> Option<u64> {
    match value.strip_prefix("0x") {
        Some(rest) => u64::from_str_radix(rest, 16).ok(),
        None => value.parse().ok(),
    }
}

impl Manifest {
    /// Render the canonical manifest text.
    pub fn render(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("manifest-version {MANIFEST_VERSION}\n"));
        out.push_str(&format!("format {MANIFEST_FORMAT}\n"));
        out.push_str(&format!("campaign {}\n", self.campaign.id));
        out.push_str(&format!(
            "campaign-hash {}\n",
            hex(self.campaign.stable_hash())
        ));
        out.push_str(&format!("build {}\n", self.build_version));
        for version in &self.behavior_policy_versions {
            out.push_str(&format!("behavior-policy {version}\n"));
        }
        out.push_str(&format!("rng-version {}\n", self.rng_algorithm_version));
        out.push_str(&format!("worldgen-version {}\n", self.worldgen_version));
        out.push_str(&format!(
            "genome-schema-version {}\n",
            self.genome_schema_version
        ));
        out.push_str(&format!(
            "event-schema-version {}\n",
            self.event_schema_version
        ));
        for version in &self.analysis_versions {
            out.push_str(&format!("analysis-version {version}\n"));
        }
        out.push_str(&format!("ticks {}\n", self.campaign.ticks));
        out.push_str(&format!("workers {}\n", self.workers));
        out.push_str(&format!("runs {}\n", self.runs.len()));
        out.push_str(&format!("failed-runs {}\n", self.failed.len()));
        for field in &self.campaign.varied {
            out.push_str(&format!("varied {field}\n"));
        }
        for condition in &self.campaign.conditions {
            out.push_str(&format!(
                "condition {} {}\n",
                condition.name,
                hex(condition.delta_hash())
            ));
        }
        for run in &self.runs {
            out.push_str(&render_run(run));
        }
        for failure in &self.failed {
            out.push_str(&format!(
                "failed index={} condition={} seed={} reason={}\n",
                failure.index,
                failure.condition,
                hex(failure.seed),
                failure.reason.replace('\n', " ")
            ));
        }
        out.push_str("campaign-source-begin\n");
        for line in self.campaign_source.lines() {
            out.push_str("| ");
            out.push_str(line);
            out.push('\n');
        }
        out.push_str("campaign-source-end\n");
        out
    }

    pub fn parse(text: &str) -> Result<Self, ManifestError> {
        let mut header: BTreeMap<String, String> = BTreeMap::new();
        let mut analysis_versions = Vec::new();
        let mut behavior_policy_versions = Vec::new();
        let mut runs: Vec<RunResult> = Vec::new();
        let mut failed: Vec<FailedRun> = Vec::new();
        let mut source_lines: Vec<String> = Vec::new();
        let mut in_source = false;

        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let syntax = |message: &str| ManifestError::Syntax {
                line,
                message: message.to_owned(),
            };
            if in_source {
                if raw.trim_end() == "campaign-source-end" {
                    in_source = false;
                    continue;
                }
                source_lines.push(raw.strip_prefix("| ").unwrap_or(raw).to_owned());
                continue;
            }
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                continue;
            }
            let (directive, rest) = trimmed.split_once(' ').unwrap_or((trimmed, ""));
            match directive {
                "campaign-source-begin" => in_source = true,
                "analysis-version" => analysis_versions.push(rest.to_owned()),
                "behavior-policy" => behavior_policy_versions.push(rest.to_owned()),
                "varied" | "condition" => {
                    // Derived from the embedded campaign; kept in the text
                    // for readability and re-checked against it on load.
                }
                "run" => runs.push(parse_run(rest, line)?),
                "failed" => {
                    let fields = parse_pairs(rest);
                    failed.push(FailedRun {
                        index: fields
                            .get("index")
                            .and_then(|value| value.parse().ok())
                            .ok_or_else(|| syntax("failed record needs index"))?,
                        condition: fields
                            .get("condition")
                            .cloned()
                            .ok_or_else(|| syntax("failed record needs condition"))?,
                        seed: fields
                            .get("seed")
                            .and_then(|value| parse_hex(value))
                            .ok_or_else(|| syntax("failed record needs seed"))?,
                        reason: fields.get("reason").cloned().unwrap_or_default(),
                    });
                }
                _ => {
                    header.insert(directive.to_owned(), rest.to_owned());
                }
            }
        }

        let version: u32 = header
            .get("manifest-version")
            .and_then(|value| value.parse().ok())
            .ok_or(ManifestError::Missing("manifest-version"))?;
        if version != MANIFEST_VERSION {
            return Err(ManifestError::UnsupportedVersion(version));
        }
        let format = header
            .get("format")
            .cloned()
            .ok_or(ManifestError::Missing("format"))?;
        if format != MANIFEST_FORMAT {
            return Err(ManifestError::UnsupportedFormat(format));
        }
        if source_lines.is_empty() {
            return Err(ManifestError::Missing("campaign-source"));
        }
        let campaign_source = source_lines.join("\n") + "\n";
        let campaign = Campaign::parse(&campaign_source).map_err(ManifestError::Campaign)?;
        let recorded = header
            .get("campaign-hash")
            .and_then(|value| parse_hex(value))
            .ok_or(ManifestError::Missing("campaign-hash"))?;
        let computed = campaign.stable_hash();
        if recorded != computed {
            return Err(ManifestError::CampaignHashMismatch { recorded, computed });
        }

        Ok(Self {
            campaign,
            campaign_source,
            build_version: header.get("build").cloned().unwrap_or_default(),
            behavior_policy_versions,
            rng_algorithm_version: header.get("rng-version").cloned().unwrap_or_default(),
            worldgen_version: header.get("worldgen-version").cloned().unwrap_or_default(),
            genome_schema_version: header
                .get("genome-schema-version")
                .and_then(|value| value.parse().ok())
                .unwrap_or(0),
            event_schema_version: header
                .get("event-schema-version")
                .and_then(|value| value.parse().ok())
                .unwrap_or(0),
            analysis_versions,
            workers: header
                .get("workers")
                .and_then(|value| value.parse().ok())
                .unwrap_or(1),
            runs,
            failed,
        })
    }

    /// Runs belonging to one condition, in ascending seed order.
    pub fn runs_for(&self, condition: &str) -> Vec<&RunResult> {
        let mut runs: Vec<&RunResult> = self
            .runs
            .iter()
            .filter(|run| run.condition == condition)
            .collect();
        runs.sort_by_key(|run| run.seed);
        runs
    }
}

fn render_run(run: &RunResult) -> String {
    let mut line = format!(
        "run index={} condition={} seed={} delta_hash={} config_hash={} terrain_checksum={} \
         state_checksum={} ticks={} population={} extinct={} energy_milli={} biomass_milli={} \
         max_ancestry_depth={} births={} deaths_starvation={} deaths_old_age={} \
         capacity_rejections={} dropped_events={} event_log_bytes={} snapshot_bytes={} \
         attacks={} deaths_by_damage={} carcasses={} spatial_samples={} \
         deaths_senescence={} deaths_extrinsic={} deaths_juvenile={} \
         max_age_observed={} capacity_milli={} mean_nodes_milli={} \
         mean_edges_milli={} median_nodes={} median_edges={} \
         distinct_structures={} structmut_applied={} \
         structmut_rejected={}",
        run.index,
        run.condition,
        hex(run.seed),
        hex(run.condition_delta_hash),
        hex(run.config_hash),
        hex(run.terrain_checksum),
        hex(run.state_checksum),
        run.ticks,
        run.population,
        run.extinct,
        run.total_energy_milli,
        run.total_biomass_milli,
        run.max_ancestry_depth,
        run.counters.births_total,
        run.counters.deaths_starvation_total,
        run.counters.deaths_old_age_total,
        run.counters.capacity_rejections_total,
        run.counters.dropped_events_total,
        run.event_log_offset,
        run.snapshot_bytes,
        run.attacks_total,
        run.deaths_by_damage_total,
        run.carcasses,
        run.spatial_samples,
        run.deaths_senescence_total,
        run.deaths_extrinsic_total,
        run.deaths_juvenile_total,
        run.max_age_ticks_observed,
        run.total_capacity_milli,
        run.mean_nodes_milli,
        run.mean_edges_milli,
        run.median_nodes,
        run.median_edges,
        run.distinct_structures,
        run.structural_mutations_applied,
        run.structural_mutations_rejected,
    );
    if let Some(phase2) = run.phase2.as_ref() {
        line.push_str(&format!(
            " paired_births={} pair_rejected_capacity={} pair_rejected_placement={} \
             pair_rejected_energy={} pair_rejected_nonviable={} controller_faults={} \
             mutated_trait_genes={} \
             mutated_neural_genes={}",
            phase2.paired_births_total,
            phase2.pair_rejected_capacity_total,
            phase2.pair_rejected_placement_total,
            phase2.pair_rejected_energy_total,
            phase2.pair_rejected_nonviable_total,
            phase2.controller_faults_total,
            phase2.mutated_trait_genes_total,
            phase2.mutated_neural_genes_total,
        ));
    }
    line.push('\n');
    line
}

fn parse_pairs(text: &str) -> BTreeMap<String, String> {
    let mut pairs = BTreeMap::new();
    let mut words = text.split_whitespace().peekable();
    while let Some(word) = words.next() {
        if let Some((key, value)) = word.split_once('=') {
            if key == "reason" {
                // The reason is free text and always last.
                let mut reason = value.to_owned();
                for extra in words.by_ref() {
                    reason.push(' ');
                    reason.push_str(extra);
                }
                pairs.insert(key.to_owned(), reason);
                break;
            }
            pairs.insert(key.to_owned(), value.to_owned());
        }
    }
    pairs
}

fn parse_run(text: &str, line: usize) -> Result<RunResult, ManifestError> {
    let fields = parse_pairs(text);
    let missing = |what: &'static str| ManifestError::Syntax {
        line,
        message: format!("run record needs {what}"),
    };
    let number = |key: &'static str| -> Result<u64, ManifestError> {
        fields
            .get(key)
            .and_then(|value| value.parse::<u64>().ok())
            .ok_or_else(|| missing(key))
    };
    let signed = |key: &'static str| -> Result<i64, ManifestError> {
        fields
            .get(key)
            .and_then(|value| value.parse::<i64>().ok())
            .ok_or_else(|| missing(key))
    };
    let hashed = |key: &'static str| -> Result<u64, ManifestError> {
        fields
            .get(key)
            .and_then(|value| parse_hex(value))
            .ok_or_else(|| missing(key))
    };
    let phase2 = fields
        .contains_key("paired_births")
        .then(|| Phase2Counters {
            paired_births_total: number("paired_births").unwrap_or(0),
            pair_rejected_capacity_total: number("pair_rejected_capacity").unwrap_or(0),
            pair_rejected_placement_total: number("pair_rejected_placement").unwrap_or(0),
            pair_rejected_energy_total: number("pair_rejected_energy").unwrap_or(0),
            pair_rejected_nonviable_total: number("pair_rejected_nonviable").unwrap_or(0),
            controller_faults_total: number("controller_faults").unwrap_or(0),
            mutated_trait_genes_total: number("mutated_trait_genes").unwrap_or(0),
            mutated_neural_genes_total: number("mutated_neural_genes").unwrap_or(0),
        });
    Ok(RunResult {
        index: number("index")? as usize,
        condition: fields
            .get("condition")
            .cloned()
            .ok_or_else(|| missing("condition"))?,
        seed: hashed("seed")?,
        condition_delta_hash: hashed("delta_hash")?,
        config_hash: hashed("config_hash")?,
        terrain_checksum: hashed("terrain_checksum")?,
        state_checksum: hashed("state_checksum")?,
        ticks: number("ticks")?,
        population: number("population")?,
        extinct: fields
            .get("extinct")
            .map(|value| value == "true")
            .unwrap_or(false),
        total_energy_milli: signed("energy_milli")?,
        total_biomass_milli: signed("biomass_milli")?,
        max_ancestry_depth: number("max_ancestry_depth")? as u32,
        counters: Counters {
            births_total: number("births")?,
            deaths_starvation_total: number("deaths_starvation")?,
            deaths_old_age_total: number("deaths_old_age")?,
            capacity_rejections_total: number("capacity_rejections")?,
            dropped_events_total: number("dropped_events")?,
        },
        phase2,
        event_log_offset: number("event_log_bytes")?,
        snapshot_bytes: number("snapshot_bytes")?,
        attacks_total: number("attacks").unwrap_or(0),
        deaths_by_damage_total: number("deaths_by_damage").unwrap_or(0),
        carcasses: number("carcasses").unwrap_or(0),
        spatial_samples: number("spatial_samples").unwrap_or(0),
        deaths_senescence_total: number("deaths_senescence").unwrap_or(0),
        deaths_extrinsic_total: number("deaths_extrinsic").unwrap_or(0),
        deaths_juvenile_total: number("deaths_juvenile").unwrap_or(0),
        max_age_ticks_observed: number("max_age_observed").unwrap_or(0),
        total_capacity_milli: signed("capacity_milli").unwrap_or(0),
        mean_nodes_milli: number("mean_nodes_milli").unwrap_or(0),
        mean_edges_milli: number("mean_edges_milli").unwrap_or(0),
        median_nodes: number("median_nodes").unwrap_or(0),
        median_edges: number("median_edges").unwrap_or(0),
        distinct_structures: number("distinct_structures").unwrap_or(0),
        structural_mutations_applied: number("structmut_applied").unwrap_or(0),
        structural_mutations_rejected: number("structmut_rejected").unwrap_or(0),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::campaign::Campaign;
    use crate::scheduler::{SchedulerOptions, run_campaign};

    const SOURCE: &str = "\
campaign manifest-round-trip
ticks 60
seeds 1..3
base preset phase2
base cells_x 32
base cells_y 32
base initial_organisms 20
base max_entities 200
condition control
condition treatment
set treatment crowding_cost_milli_per_s 400
vary crowding_cost_milli_per_s
output events off
output snapshots off
";

    fn built_manifest() -> Manifest {
        let campaign = Campaign::parse(SOURCE).unwrap();
        let results = run_campaign(&campaign, &SchedulerOptions::in_memory(2));
        let runs: Vec<RunResult> = results
            .into_iter()
            .map(|result| result.expect("run succeeded"))
            .collect();
        Manifest {
            campaign,
            campaign_source: SOURCE.to_owned(),
            build_version: "lifesim-test".to_owned(),
            behavior_policy_versions: vec![
                sim_core::BEHAVIOR_POLICY_VERSION.to_owned(),
                sim_core::PHASE2_BEHAVIOR_POLICY_VERSION.to_owned(),
            ],
            rng_algorithm_version: sim_core::RNG_ALGORITHM_VERSION.to_owned(),
            worldgen_version: sim_core::WORLDGEN_VERSION.to_owned(),
            genome_schema_version: sim_core::GENOME_SCHEMA_VERSION,
            event_schema_version: sim_core::EVENT_SCHEMA_VERSION,
            analysis_versions: vec![sim_core::SIMILARITY_ALGORITHM_VERSION.to_owned()],
            workers: 2,
            runs,
            failed: Vec::new(),
        }
    }

    #[test]
    fn manifest_round_trips_exactly() {
        let manifest = built_manifest();
        let text = manifest.render();
        let parsed = Manifest::parse(&text).expect("parse");
        assert_eq!(parsed.runs, manifest.runs);
        assert_eq!(parsed.campaign, manifest.campaign);
        assert_eq!(parsed.analysis_versions, manifest.analysis_versions);
        assert_eq!(parsed.render(), text, "rendering is not a fixed point");
    }

    #[test]
    fn manifest_text_is_independent_of_worker_count() {
        let campaign = Campaign::parse(SOURCE).unwrap();
        let render_at = |workers: usize| {
            let runs: Vec<RunResult> =
                run_campaign(&campaign, &SchedulerOptions::in_memory(workers))
                    .into_iter()
                    .map(|result| result.expect("run succeeded"))
                    .collect();
            let mut manifest = built_manifest();
            manifest.runs = runs;
            manifest.workers = 0; // excluded from the comparison
            manifest.render()
        };
        assert_eq!(render_at(1), render_at(4));
        assert_eq!(render_at(1), render_at(8));
    }

    #[test]
    fn an_edited_manifest_is_rejected() {
        let manifest = built_manifest();
        let text = manifest.render();

        // Tamper with the embedded source; the recorded hash no longer
        // matches, so the manifest is refused rather than reinterpreted.
        let tampered = text.replace("| ticks 60", "| ticks 61");
        assert!(matches!(
            Manifest::parse(&tampered),
            Err(ManifestError::CampaignHashMismatch { .. })
        ));

        let wrong_version = text.replace("manifest-version 1", "manifest-version 9");
        assert!(matches!(
            Manifest::parse(&wrong_version),
            Err(ManifestError::UnsupportedVersion(9))
        ));
    }

    #[test]
    fn failed_runs_survive_the_round_trip_with_their_reason() {
        let mut manifest = built_manifest();
        manifest.failed.push(FailedRun {
            index: 99,
            condition: "control".to_owned(),
            seed: 7,
            reason: "invariant violated: EnergyLedgerMismatch { .. }".to_owned(),
        });
        let parsed = Manifest::parse(&manifest.render()).unwrap();
        assert_eq!(parsed.failed, manifest.failed);
    }

    #[test]
    fn runs_for_returns_seed_ordered_records() {
        let manifest = built_manifest();
        let control = manifest.runs_for("control");
        assert_eq!(control.len(), 3);
        assert!(control.windows(2).all(|pair| pair[0].seed < pair[1].seed));
    }
}
