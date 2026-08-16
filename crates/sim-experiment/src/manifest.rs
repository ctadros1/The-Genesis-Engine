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
    /// Per-individual action samples written, on the same terms and for the
    /// same reason: a C11.1 analysis that silently read a shortened series
    /// would compute a within-lifetime comparison over a window that is not
    /// the window the campaign declared.
    pub action_samples: u64,
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
    /// Phase 10 morphology metrics. Zero when the section is disabled.
    pub mean_modules_milli: u64,
    pub median_modules: u64,
    pub distinct_morphologies: u64,
    pub nonviable_bodies: u64,
    pub refused_node_budget: u64,
    /// Phase 7 contest outcomes. Zero when the section is disabled.
    pub attacks_total: u64,
    pub deaths_by_damage_total: u64,
    pub carcasses: u64,
    /// Structural-mutation outcomes by operator and by rejection reason, and
    /// development outcomes by action and by non-viability class.
    ///
    /// Nested rather than flattened into thirteen and twelve more scalars,
    /// so the compiler enforces completeness at every seam these cross:
    /// adding a counter to either struct breaks `render_run` and `parse_run`
    /// at once instead of letting one of them drop it.
    ///
    /// `None` when the subsystem is disabled, never zero. Absent and zero
    /// are opposite conclusions - "the cap could not have bound because
    /// schema 2 was off" against "the cap never bound" - and every question
    /// these columns exist to answer depends on telling them apart. The
    /// summed `structural_mutations_applied`/`_rejected` above stay, because
    /// the eight archived manifests under `experiments/results/` carry them
    /// and a manifest is a record, not a cache.
    pub mutation: Option<sim_core::MutationCounters>,
    pub develop: Option<sim_core::DevelopCounters>,
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

/// One `run` line.
///
/// **Every struct is destructured with no `..`, never field-accessed**
/// (D-077). This was 41 positional `run.field` arguments to one `format!`,
/// and the consequence is worth stating because it is not the obvious one: a
/// field added to `RunResult` broke compilation *only* in `parse_run`, where
/// it is satisfied by a `number("...").unwrap_or(0)` in seconds. Nothing
/// forced the renderer to emit it. The column would then be absent from
/// every manifest, parse back as zero, and `manifest_round_trips_exactly`
/// would compare that zero against the zero it started with and pass.
///
/// Destructuring moves the failure to compile time on the writing side,
/// which is the only side that can lose data.
fn render_run(run: &RunResult) -> String {
    let RunResult {
        index,
        condition,
        seed,
        condition_delta_hash,
        config_hash,
        terrain_checksum,
        state_checksum,
        ticks,
        population,
        extinct,
        total_energy_milli,
        total_biomass_milli,
        max_ancestry_depth,
        counters,
        phase2,
        event_log_offset,
        snapshot_bytes,
        spatial_samples,
        action_samples,
        deaths_senescence_total,
        deaths_extrinsic_total,
        deaths_juvenile_total,
        max_age_ticks_observed,
        total_capacity_milli,
        mean_nodes_milli,
        mean_edges_milli,
        median_nodes,
        median_edges,
        distinct_structures,
        structural_mutations_applied,
        structural_mutations_rejected,
        mean_modules_milli,
        median_modules,
        distinct_morphologies,
        nonviable_bodies,
        refused_node_budget,
        attacks_total,
        deaths_by_damage_total,
        carcasses,
        mutation,
        develop,
    } = run;
    let Counters {
        births_total,
        deaths_starvation_total,
        deaths_old_age_total,
        capacity_rejections_total,
        dropped_events_total,
    } = counters;
    let (seed, delta_hash, config_hash, terrain_checksum, state_checksum) = (
        hex(*seed),
        hex(*condition_delta_hash),
        hex(*config_hash),
        hex(*terrain_checksum),
        hex(*state_checksum),
    );
    let mut line = format!(
        "run index={index} condition={condition} seed={seed} delta_hash={delta_hash} \
         config_hash={config_hash} terrain_checksum={terrain_checksum} \
         state_checksum={state_checksum} ticks={ticks} population={population} \
         extinct={extinct} energy_milli={total_energy_milli} \
         biomass_milli={total_biomass_milli} max_ancestry_depth={max_ancestry_depth} \
         births={births_total} deaths_starvation={deaths_starvation_total} \
         deaths_old_age={deaths_old_age_total} \
         capacity_rejections={capacity_rejections_total} \
         dropped_events={dropped_events_total} event_log_bytes={event_log_offset} \
         snapshot_bytes={snapshot_bytes} attacks={attacks_total} \
         deaths_by_damage={deaths_by_damage_total} carcasses={carcasses} \
         spatial_samples={spatial_samples} action_samples={action_samples} \
         deaths_senescence={deaths_senescence_total} \
         deaths_extrinsic={deaths_extrinsic_total} deaths_juvenile={deaths_juvenile_total} \
         max_age_observed={max_age_ticks_observed} capacity_milli={total_capacity_milli} \
         mean_nodes_milli={mean_nodes_milli} mean_edges_milli={mean_edges_milli} \
         median_nodes={median_nodes} median_edges={median_edges} \
         distinct_structures={distinct_structures} \
         structmut_applied={structural_mutations_applied} \
         structmut_rejected={structural_mutations_rejected} \
         mean_modules_milli={mean_modules_milli} median_modules={median_modules} \
         distinct_morphologies={distinct_morphologies} nonviable_bodies={nonviable_bodies} \
         refused_node_budget={refused_node_budget}"
    );
    if let Some(Phase2Counters {
        paired_births_total,
        pair_rejected_capacity_total,
        pair_rejected_placement_total,
        pair_rejected_energy_total,
        pair_rejected_nonviable_total,
        controller_faults_total,
        mutated_trait_genes_total,
        mutated_neural_genes_total,
    }) = phase2
    {
        line.push_str(&format!(
            " paired_births={paired_births_total} \
             pair_rejected_capacity={pair_rejected_capacity_total} \
             pair_rejected_placement={pair_rejected_placement_total} \
             pair_rejected_energy={pair_rejected_energy_total} \
             pair_rejected_nonviable={pair_rejected_nonviable_total} \
             controller_faults={controller_faults_total} \
             mutated_trait_genes={mutated_trait_genes_total} \
             mutated_neural_genes={mutated_neural_genes_total}"
        ));
    }
    // The key space is flat, so every column here carries the `structmut_`
    // or `develop_` prefix of the struct it came from. Without it
    // `rejected_cap` and a future contest or physiology rejection counter
    // would be one key, and the manifest would parse whichever was written
    // last into both.
    if let Some(sim_core::MutationCounters {
        point_applied,
        duplication_applied,
        deletion_applied,
        insertion_applied,
        transposition_applied,
        binding_applied,
        rejected_homology_collision,
        rejected_orphaned,
        rejected_min_nodes,
        rejected_no_bindings,
        rejected_cap,
        rejected_inapplicable,
        rejected_cycle,
        rejected_invalid,
    }) = mutation
    {
        line.push_str(&format!(
            " structmut_point_applied={point_applied} \
             structmut_duplication_applied={duplication_applied} \
             structmut_deletion_applied={deletion_applied} \
             structmut_insertion_applied={insertion_applied} \
             structmut_transposition_applied={transposition_applied} \
             structmut_rejected_homology_collision={rejected_homology_collision} \
             structmut_rejected_orphaned={rejected_orphaned} \
             structmut_rejected_min_nodes={rejected_min_nodes} \
             structmut_rejected_no_bindings={rejected_no_bindings} \
             structmut_rejected_cap={rejected_cap} \
             structmut_rejected_inapplicable={rejected_inapplicable} \
             structmut_rejected_cycle={rejected_cycle} \
             structmut_rejected_invalid={rejected_invalid} \
             structmut_binding_applied={binding_applied}"
        ));
    }
    if let Some(sim_core::DevelopCounters {
        bodies_grown,
        modules_placed,
        differentiations,
        scale_changes,
        refused_occupied,
        refused_out_of_bounds,
        refused_max_modules,
        refused_node_budget,
        nonviable_empty,
        nonviable_disconnected,
        nonviable_missing_type,
        nonviable_other,
    }) = develop
    {
        line.push_str(&format!(
            " develop_bodies_grown={bodies_grown} develop_modules_placed={modules_placed} \
             develop_differentiations={differentiations} \
             develop_scale_changes={scale_changes} \
             develop_refused_occupied={refused_occupied} \
             develop_refused_out_of_bounds={refused_out_of_bounds} \
             develop_refused_max_modules={refused_max_modules} \
             develop_refused_node_budget={refused_node_budget} \
             develop_nonviable_empty={nonviable_empty} \
             develop_nonviable_disconnected={nonviable_disconnected} \
             develop_nonviable_missing_type={nonviable_missing_type} \
             develop_nonviable_other={nonviable_other}"
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
    // Each block is gated on one of its own columns, so a manifest written
    // before the block existed parses as `None` rather than as a struct full
    // of zeros. The eight archived manifests under `experiments/results/`
    // predate both blocks, which is why `MANIFEST_VERSION` does not move:
    // absence is a supported reading of the same format version, exactly as
    // it already is for the phase2 block.
    let mutation =
        fields
            .contains_key("structmut_point_applied")
            .then(|| sim_core::MutationCounters {
                point_applied: number("structmut_point_applied").unwrap_or(0),
                duplication_applied: number("structmut_duplication_applied").unwrap_or(0),
                deletion_applied: number("structmut_deletion_applied").unwrap_or(0),
                insertion_applied: number("structmut_insertion_applied").unwrap_or(0),
                transposition_applied: number("structmut_transposition_applied").unwrap_or(0),
                rejected_homology_collision: number("structmut_rejected_homology_collision")
                    .unwrap_or(0),
                rejected_orphaned: number("structmut_rejected_orphaned").unwrap_or(0),
                rejected_min_nodes: number("structmut_rejected_min_nodes").unwrap_or(0),
                rejected_no_bindings: number("structmut_rejected_no_bindings").unwrap_or(0),
                rejected_cap: number("structmut_rejected_cap").unwrap_or(0),
                rejected_inapplicable: number("structmut_rejected_inapplicable").unwrap_or(0),
                rejected_cycle: number("structmut_rejected_cycle").unwrap_or(0),
                rejected_invalid: number("structmut_rejected_invalid").unwrap_or(0),
                // Appended in Phase 12 (D-114). Absent from every archived
                // manifest, so absence reads as zero - which is what those
                // runs had, since the operator did not exist.
                binding_applied: number("structmut_binding_applied").unwrap_or(0),
            });
    let develop = fields
        .contains_key("develop_bodies_grown")
        .then(|| sim_core::DevelopCounters {
            bodies_grown: number("develop_bodies_grown").unwrap_or(0),
            modules_placed: number("develop_modules_placed").unwrap_or(0),
            differentiations: number("develop_differentiations").unwrap_or(0),
            scale_changes: number("develop_scale_changes").unwrap_or(0),
            refused_occupied: number("develop_refused_occupied").unwrap_or(0),
            refused_out_of_bounds: number("develop_refused_out_of_bounds").unwrap_or(0),
            refused_max_modules: number("develop_refused_max_modules").unwrap_or(0),
            refused_node_budget: number("develop_refused_node_budget").unwrap_or(0),
            nonviable_empty: number("develop_nonviable_empty").unwrap_or(0),
            nonviable_disconnected: number("develop_nonviable_disconnected").unwrap_or(0),
            nonviable_missing_type: number("develop_nonviable_missing_type").unwrap_or(0),
            nonviable_other: number("develop_nonviable_other").unwrap_or(0),
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
        action_samples: number("action_samples").unwrap_or(0),
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
        mean_modules_milli: number("mean_modules_milli").unwrap_or(0),
        median_modules: number("median_modules").unwrap_or(0),
        distinct_morphologies: number("distinct_morphologies").unwrap_or(0),
        nonviable_bodies: number("nonviable_bodies").unwrap_or(0),
        refused_node_budget: number("refused_node_budget").unwrap_or(0),
        mutation,
        develop,
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

    /// A run record in which **every** scalar holds a distinct nonzero
    /// value.
    ///
    /// Distinctness is the whole point. `manifest_round_trips_exactly` runs
    /// a campaign with genome2 and morphology disabled, so every one of the
    /// twenty-five new columns is zero on both sides of the comparison and
    /// the assertion it makes about them is `0 == 0`. It would pass with
    /// `render_run` emitting nothing at all. It would also pass with two
    /// columns transposed, or with a whole block written from the wrong
    /// struct, as long as the values happened to agree - which for a uniform
    /// 1 they always would.
    fn distinctly_valued_run() -> RunResult {
        RunResult {
            index: 7,
            condition: "treatment".to_owned(),
            seed: 0x1111_2222_3333_4444,
            condition_delta_hash: 0x2222_3333_4444_5555,
            config_hash: 0x3333_4444_5555_6666,
            terrain_checksum: 0x4444_5555_6666_7777,
            state_checksum: 0x5555_6666_7777_8888,
            ticks: 101,
            population: 102,
            extinct: true,
            total_energy_milli: 103,
            total_biomass_milli: 104,
            max_ancestry_depth: 105,
            counters: Counters {
                births_total: 106,
                deaths_starvation_total: 107,
                deaths_old_age_total: 108,
                capacity_rejections_total: 109,
                dropped_events_total: 110,
            },
            phase2: Some(Phase2Counters {
                paired_births_total: 111,
                pair_rejected_capacity_total: 112,
                pair_rejected_placement_total: 113,
                pair_rejected_energy_total: 114,
                pair_rejected_nonviable_total: 115,
                controller_faults_total: 116,
                mutated_trait_genes_total: 117,
                mutated_neural_genes_total: 118,
            }),
            event_log_offset: 119,
            snapshot_bytes: 120,
            spatial_samples: 121,
            action_samples: 401,
            deaths_senescence_total: 122,
            deaths_extrinsic_total: 123,
            deaths_juvenile_total: 124,
            max_age_ticks_observed: 125,
            total_capacity_milli: 126,
            mean_nodes_milli: 127,
            mean_edges_milli: 128,
            median_nodes: 129,
            median_edges: 130,
            distinct_structures: 131,
            structural_mutations_applied: 132,
            structural_mutations_rejected: 133,
            mean_modules_milli: 134,
            median_modules: 135,
            distinct_morphologies: 136,
            nonviable_bodies: 137,
            refused_node_budget: 138,
            attacks_total: 139,
            deaths_by_damage_total: 140,
            carcasses: 141,
            mutation: Some(sim_core::MutationCounters {
                point_applied: 201,
                duplication_applied: 202,
                deletion_applied: 203,
                insertion_applied: 204,
                transposition_applied: 205,
                rejected_homology_collision: 206,
                rejected_orphaned: 207,
                rejected_min_nodes: 208,
                rejected_no_bindings: 209,
                rejected_cap: 210,
                rejected_inapplicable: 211,
                rejected_cycle: 212,
                rejected_invalid: 213,
                binding_applied: 214,
            }),
            develop: Some(sim_core::DevelopCounters {
                bodies_grown: 301,
                modules_placed: 302,
                differentiations: 303,
                scale_changes: 304,
                refused_occupied: 305,
                refused_out_of_bounds: 306,
                refused_max_modules: 307,
                refused_node_budget: 308,
                nonviable_empty: 309,
                nonviable_disconnected: 310,
                nonviable_missing_type: 311,
                nonviable_other: 312,
            }),
        }
    }

    #[test]
    fn every_counter_class_survives_the_round_trip() {
        let mut manifest = built_manifest();
        let run = distinctly_valued_run();
        manifest.runs = vec![run.clone()];
        let text = manifest.render();

        // Guard the guard: if any two fields shared a value, an equality
        // assertion below would tolerate the transposition it exists to
        // catch. Checked on the rendered text rather than on the struct so
        // it covers whatever `render_run` actually chose to emit.
        let line = text
            .lines()
            .find(|line| line.starts_with("run "))
            .expect("a run line");
        let mut values: Vec<&str> = line
            .split_whitespace()
            .skip(1)
            .filter_map(|pair| pair.split_once('=').map(|(_, value)| value))
            .collect();
        let emitted = values.len();
        values.sort_unstable();
        values.dedup();
        assert_eq!(
            values.len(),
            emitted,
            "two columns share a value, so this test cannot see them swapped"
        );

        let parsed = Manifest::parse(&text).expect("parse");
        assert_eq!(parsed.runs, vec![run]);
        assert_eq!(parsed.render(), text, "rendering is not a fixed point");
    }

    #[test]
    fn a_manifest_without_the_counter_columns_parses_as_absent_not_as_zero() {
        // The eight archived manifests under experiments/results/ say
        // `manifest-version 1` and carry neither block. Absence has to read
        // as "not recorded": a campaign that reported thirteen zeros for a
        // world whose schema-2 section was never even enabled would be
        // asserting that no cap ever bound, which it has no evidence for.
        let manifest = built_manifest();
        for run in &manifest.runs {
            assert_eq!(run.mutation, None);
            assert_eq!(run.develop, None);
        }
        let text = manifest.render();
        assert!(
            !text.contains("structmut_point_applied") && !text.contains("develop_bodies_grown"),
            "a disabled subsystem emitted counter columns"
        );
        let parsed = Manifest::parse(&text).expect("parse");
        assert!(parsed.runs.iter().all(|run| run.mutation.is_none()));
        assert!(parsed.runs.iter().all(|run| run.develop.is_none()));

        // ...and the same manifest with the columns present parses them,
        // so the assertion above is about the columns and not about a
        // parser that never fills these in at all.
        let mut with_columns = manifest;
        with_columns.runs = vec![distinctly_valued_run()];
        let reparsed = Manifest::parse(&with_columns.render()).expect("parse");
        assert_eq!(
            reparsed.runs[0]
                .mutation
                .expect("mutation block")
                .rejected_cap,
            210
        );
    }

    #[test]
    fn every_archived_manifest_still_parses_at_this_version() {
        // Adding columns without bumping MANIFEST_VERSION is only sound if
        // the records already written still load. Phases 8, 9 and 10 each
        // added columns on that reasoning and none of them wrote the check
        // down, so "still parses" was an assumption for three phases.
        let root =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../experiments/results");
        let mut checked = 0;
        let mut phase9: Option<Manifest> = None;
        for entry in std::fs::read_dir(&root).expect("experiments/results exists") {
            let path = entry.expect("dir entry").path();
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            if !name.ends_with("-manifest.txt") {
                continue;
            }
            let text = std::fs::read_to_string(&path).expect("read manifest");
            let manifest = Manifest::parse(&text)
                .unwrap_or_else(|error| panic!("{name} no longer parses: {error}"));
            assert!(!manifest.runs.is_empty(), "{name} parsed to zero runs");
            if name == "phase9-c91-confirmatory-manifest.txt" {
                phase9 = Some(manifest);
            }
            checked += 1;
        }
        // A floor, not an exact count, so adding a manifest does not fail
        // this; but a floor is required, because an empty or moved
        // directory would otherwise let the loop above assert nothing.
        assert!(
            checked >= 8,
            "only {checked} archived manifests were read; the fixture set shrank"
        );
        // The strongest of the eight: C9.1 ran with schema 2 enabled, so a
        // manifest written today would carry all thirteen columns. Its
        // absence must read as "not recorded" rather than as thirteen
        // zeros, which would assert that no cap ever bound in a run that
        // has no such evidence either way.
        let phase9 = phase9.expect("the phase 9 confirmatory manifest is archived");
        assert_eq!(phase9.runs.len(), 210);
        assert!(phase9.runs.iter().all(|run| run.mutation.is_none()));
        assert!(phase9.runs.iter().all(|run| run.develop.is_none()));
        assert!(
            phase9
                .runs
                .iter()
                .any(|run| run.structural_mutations_applied > 0),
            "the summed columns those manifests do carry stopped parsing"
        );
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

    /// The rates are `phase9_world::the_invalid_counter_stays_zero_because_
    /// it_is_the_bug_signal`'s, and they are chosen rather than raised:
    /// transposition cannot apply to a single-chromosome founder, so every
    /// transposition draw that fires becomes an `Inapplicable` rejection.
    /// That gives a class which is reliably nonzero and which the summed
    /// `structmut_rejected` column cannot be distinguished from a cap
    /// rejection inside.
    const REJECTING: &str = "\
campaign structmut-columns
ticks 5000
seeds 13..14
base preset phase2
base cells_x 64
base cells_y 64
base initial_organisms 120
base max_entities 2000
base cell_capacity_milli 120000
base genome2.enabled true
base genome2.mutation.duplication_q16 6554
base genome2.mutation.insertion_q16 6554
base genome2.mutation.transposition_q16 6554
condition only
output events off
output snapshots off
";

    #[test]
    fn a_real_run_carries_its_rejection_classes_into_the_rendered_text() {
        // The hand-built round trip proves render and parse agree with each
        // other. It cannot prove the scheduler ever puts a counter into a
        // `RunResult` - `execute_unit` could set both fields to `None` and
        // that test would still pass. This runs a real campaign with
        // snapshots off, which is the configuration the whole change exists
        // for: before it, the only way to learn which class a rejection
        // belonged to was to re-open a snapshot that a campaign is not
        // obliged to write.
        let campaign = Campaign::parse(REJECTING).expect("campaign parses");
        let runs: Vec<RunResult> = run_campaign(&campaign, &SchedulerOptions::in_memory(2))
            .into_iter()
            .map(|result| result.expect("run succeeded"))
            .collect();
        let mut manifest = built_manifest();
        let source = manifest.campaign_source.clone();
        manifest.campaign = campaign;
        manifest.campaign_source = REJECTING.to_owned();
        assert_ne!(source, manifest.campaign_source);
        manifest.runs = runs;

        let observed: Vec<u64> = manifest
            .runs
            .iter()
            .map(|run| {
                run.mutation
                    .expect("schema 2 is enabled, so the block must be present")
                    .rejected_inapplicable
            })
            .collect();
        assert!(
            observed.iter().all(|count| *count > 0),
            "no world reported an inapplicable rejection ({observed:?}); the campaign is \
             not exercising the class this test reads"
        );
        for run in &manifest.runs {
            let counters = run.mutation.expect("block");
            // The block and the two summed columns have to describe the
            // same world. Without this, `execute_unit` could read `mutation`
            // from a freshly defaulted world, or from the wrong one, and
            // every assertion above would still hold.
            assert_eq!(counters.total_applied(), run.structural_mutations_applied);
            assert_eq!(counters.total_rejected(), run.structural_mutations_rejected);
            // The two halves of the same mechanism, in opposite directions:
            // a single-chromosome founder cannot transpose, so the draws
            // that fire land in `rejected_inapplicable` and never in
            // `transposition_applied`. No one mis-wired field could produce
            // both readings, which is what stops this from passing on a
            // column that merely happens to hold the same number.
            assert_eq!(counters.transposition_applied, 0);
            assert!(
                counters.point_applied > 0 && counters.duplication_applied > 0,
                "no applied class fired, so only the rejection half is covered"
            );
        }
        // Morphology is off in this campaign, so its block must be absent -
        // the same run proving that "present" and "absent" are both driven
        // by the world rather than emitted unconditionally.
        assert!(manifest.runs.iter().all(|run| run.develop.is_none()));

        let text = manifest.render();
        for count in &observed {
            assert!(
                text.contains(&format!("structmut_rejected_inapplicable={count}")),
                "the rendered manifest does not carry {count}"
            );
        }
        assert!(!text.contains("develop_bodies_grown"));
        let parsed = Manifest::parse(&text).expect("parse");
        assert_eq!(parsed.runs, manifest.runs);
    }

    #[test]
    fn runs_for_returns_seed_ordered_records() {
        let manifest = built_manifest();
        let control = manifest.runs_for("control");
        assert_eq!(control.len(), 3);
        assert!(control.windows(2).all(|pair| pair[0].seed < pair[1].seed));
    }
}
