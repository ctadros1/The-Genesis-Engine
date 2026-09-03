//! Offline analysis over campaign artifacts (Phase 7).
//!
//! **Analysis observes; it never instructs** (ADR-0016). That rule is
//! enforced structurally rather than by discipline: nothing in the
//! workspace depends on this crate except the CLI, so no statistic computed
//! here has a path to a rule, an input channel, a config trigger, or an
//! intervention.
//!
//! Everything here is exact and deterministic. Indices are integer counts
//! reported in milli-units; resampling draws come from the kernel's named
//! streams keyed on a recorded analysis seed. A report is a pure function
//! of the campaign artifacts plus that seed.

mod arrival;
mod assortment;
mod development;
mod artifact;
mod communities;
mod conjunction;
mod demography;
mod era;
mod fidelity;
mod lineage;
mod morph;
mod paired;
mod plasticity;
mod power;
mod recognition;
pub mod social;
mod spatial;
mod structure;
mod tradition;

pub use assortment::{ASSORTMENT_POLICY_VERSION, AssortmentCensus, assortment_census};
pub use development::{
    DEVELOPMENT_POLICY_VERSION, DevelopmentCensus, DevelopmentError, development_census,
};
pub use arrival::{
    ARRIVAL_DETECTOR_VERSION, ArrivalCensus, ArrivalError, IndividualArrival, PatchSpec,
    arrival_census,
};
pub use communities::{COMMUNITIES_VERSION, CommunityPlan, WorldCommunities, world_communities};
pub use fidelity::{
    FIDELITY_VERSION, FidelityBin, FidelityPlan, KINSHIP_ONE_Q32, Pedigree, RELATEDNESS_BIN_COUNT,
    RELATEDNESS_BIN_EDGES_Q32, WorldFidelity, world_fidelity,
};
pub use recognition::{RECOGNITION_VERSION, RecognitionPlan, WorldRecognition, world_recognition};
pub use tradition::{
    TRADITION_VERSION, TraditionFinding, TraditionPlan, WorldTraditions, world_traditions,
};

pub use artifact::{
    ARTIFACT_ANALYSIS_VERSION, ArtifactPlan, ArtifactReport, BindingCensus, LivingOrganism,
    Verdict as ArtifactVerdict, WorldArtifact, WorldInputs, binding_census,
    decide as decide_artifact, render as render_artifact, world_artifact,
};
pub use conjunction::{
    AlleleConjunctionCensus, ArmConjunction, CONJUNCTION_CENSUS_VERSION, Conjunction,
    ExpressedConjunctionCensus, LearnedStateCensus, WorldConjunction, allele_conjunction_census,
    conjunction_of, expressed_conjunction_census, learned_state_census, modulator_satisfied,
    render as render_conjunction, rule_reads_coefficients, summarise as summarise_conjunction,
};
pub use demography::{
    DEMOGRAPHY_INDEX_VERSION, WorldDemography, spearman_milli, thermal_match_rho_milli,
    world_demography,
};
pub use lineage::{LINEAGE_INDEX_VERSION, WorldLineage, world_lineage};
pub use era::{
    Boundary as EraBoundary, ERA_VERSION, EraError, EraPlan, FEATURE_COUNT, FEATURE_NAMES,
    FeatureGates, SYNTHETIC_FIXTURE_VERSION, SyntheticSpec, WindowFeatures, WorldEra,
    render_header, render_world, segment, synthetic_log, world_era,
};
pub use morph::{
    MORPH_ANALYSIS_VERSION, MorphOutcome, MorphPlan, MorphSample, PERMUTATIONS, WorldMorph,
    consequence_of, pairs_of as morph_pairs, parse_series, permutation_p95_milli,
    render as render_morph, rho_of, stability as morph_stability, summarise as summarise_morph,
};
pub use paired::{
    BOOTSTRAP_RESAMPLES, Direction, PAIRED_STATS_VERSION, Pair, PairedResult,
    binomial_upper_tail_milli, compare, median_milli,
};
pub use plasticity::{
    AlleleCensus, Boundary, PLASTICITY_ANALYSIS_VERSION, PlasticityOutcome, PlasticityPlan,
    ShiftRefusal, ShiftResult, Verdict, WindowDistance, WorldPlasticity, allele_census, boundaries,
    contrast as plasticity_contrast, pairs_of as plasticity_pairs, rates_milli,
    render as render_plasticity, summarise as summarise_plasticity, window_distance, world_shift,
};
pub use power::{
    POWER_TRIALS, POWER_VERSION, PowerPoint, observed_success_rate_milli, power_curve, required_at,
    smallest_adequate,
};
pub use sim_core::preferred_temperature_milli;
pub use spatial::{
    COARSE_SCALE_CELLS, FINE_SCALE_CELLS, IndexRefusal, QuadratGrid, SPATIAL_INDEX_VERSION,
    WorldIndex, world_index,
};
pub use structure::{
    STRUCTURE_ANALYSIS_VERSION, StabilityReport, StructureOutcome, StructurePlan, WorldStructure,
    pairs_of as structure_pairs, render as render_structure, stability, stability_count,
    summarise as summarise_structure, worlds_for as structure_worlds,
};

use sim_experiment::Manifest;
use sim_persist::decode_spatial;
use std::path::Path;

/// Analysis parameters that must be fixed before a confirmatory campaign
/// runs. They are recorded verbatim in the report so a reader can check
/// that they were not chosen after seeing the data.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SpatialPlan {
    /// Samples at or before this tick are discarded as the opening
    /// transient, when layout still reflects founder placement.
    pub burn_in_ticks: u64,
    pub fine_scale_cells: u32,
    pub coarse_scale_cells: u32,
    /// Smallest effect of interest, milli-units on the relative scale.
    /// `100` is a 10 percent change.
    pub sesoi_milli: i64,
    /// Per-world probability of reaching the SESOI under the null, used for
    /// the exact binomial p-value. Prespecified with the SESOI.
    ///
    /// `500` is the conservative value for a directed rule: under any
    /// symmetric null at most half the worlds can move in the prespecified
    /// direction, so a p-value computed against it cannot flatter the
    /// result. The pilot's near-null contrasts came in at or below that
    /// rate, which is the check that it is not optimistic.
    pub null_rate_milli: u32,
    /// The direction the effect is expected to take, fixed from the pilot
    /// before the confirmatory campaign runs.
    pub direction: Direction,
    /// Worlds that must reach the SESOI for the criterion to pass.
    pub required_worlds: usize,
    /// Seed for the bootstrap. Recorded; the interval is a function of the
    /// data and this value only.
    pub analysis_seed: u64,
}

impl Default for SpatialPlan {
    fn default() -> Self {
        Self {
            burn_in_ticks: 5_000,
            fine_scale_cells: FINE_SCALE_CELLS,
            coarse_scale_cells: COARSE_SCALE_CELLS,
            sesoi_milli: 100,
            null_rate_milli: 500,
            direction: Direction::Decrease,
            required_worlds: 20,
            analysis_seed: 0x5eed_cafe_f00d_beef,
        }
    }
}

/// One world's spatial result, or the typed reason there is none.
#[derive(Clone, Debug, PartialEq)]
pub struct WorldOutcome {
    pub condition: String,
    pub seed: u64,
    pub population: u64,
    pub extinct: bool,
    pub index: Result<WorldIndex, String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpatialReport {
    pub campaign_id: String,
    pub plan: SpatialPlan,
    pub index_version: String,
    pub stats_version: String,
    pub worlds: Vec<WorldOutcome>,
    /// One entry per (treatment, control) contrast requested.
    pub contrasts: Vec<Contrast>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Contrast {
    pub treatment: String,
    pub control: String,
    /// Seeds where either side had no defined index, with the reason. These
    /// are exclusions correlated with the treatment, so they are reported
    /// rather than dropped quietly.
    pub unusable_seeds: Vec<(u64, String)>,
    pub aggregation: PairedResult,
    pub encounter: PairedResult,
}

#[derive(Clone, Debug)]
pub enum AnalysisError {
    Io(String),
    Decode(String),
    /// The sample file's terrain does not match the terrain the manifest
    /// records for that run, so the file and the run are not the same world.
    TerrainMismatch {
        seed: u64,
        condition: String,
        recorded: u64,
        regenerated: u64,
    },
    /// The sample file's config hash does not match the run's.
    ConfigMismatch {
        seed: u64,
        condition: String,
        recorded: u64,
        found: u64,
    },
    /// The file holds fewer samples than the manifest says were written.
    SampleCountMismatch {
        seed: u64,
        condition: String,
        recorded: u64,
        found: u64,
    },
    UnknownCondition(String),
    Campaign(String),
}

impl std::fmt::Display for AnalysisError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

/// Compute the spatial index for every run in a manifest.
///
/// Terrain is regenerated from the manifest's own embedded campaign source
/// and checked against both the run record and the sample file's header
/// before its land mask is used. Three independent records of the same
/// world have to agree, or the analysis refuses: a land mask taken from the
/// wrong map would silently renormalize every index computed from it.
pub fn analyse_worlds(
    manifest: &Manifest,
    directory: &Path,
    plan: &SpatialPlan,
) -> Result<Vec<WorldOutcome>, AnalysisError> {
    let mut outcomes = Vec::with_capacity(manifest.runs.len());
    for run in &manifest.runs {
        let condition = manifest
            .campaign
            .conditions
            .iter()
            .find(|candidate| candidate.name == run.condition)
            .ok_or_else(|| AnalysisError::UnknownCondition(run.condition.clone()))?;
        let config = manifest
            .campaign
            .config_for(condition, run.seed)
            .map_err(|error| AnalysisError::Campaign(error.to_string()))?;
        let terrain = sim_core::generate_terrain(&config)
            .map_err(|error| AnalysisError::Campaign(error.to_string()))?;
        if terrain.terrain_checksum != run.terrain_checksum {
            return Err(AnalysisError::TerrainMismatch {
                seed: run.seed,
                condition: run.condition.clone(),
                recorded: run.terrain_checksum,
                regenerated: terrain.terrain_checksum,
            });
        }

        let path = directory.join(format!(
            "{}.alss",
            sim_experiment::run_stem(&run.condition, run.seed)
        ));
        let bytes = std::fs::read(&path)
            .map_err(|error| AnalysisError::Io(format!("{}: {error}", path.display())))?;
        let scan =
            decode_spatial(&bytes).map_err(|error| AnalysisError::Decode(error.to_string()))?;
        if scan.info.config_hash != run.config_hash {
            return Err(AnalysisError::ConfigMismatch {
                seed: run.seed,
                condition: run.condition.clone(),
                recorded: run.config_hash,
                found: scan.info.config_hash,
            });
        }
        if scan.info.terrain_checksum != run.terrain_checksum {
            return Err(AnalysisError::TerrainMismatch {
                seed: run.seed,
                condition: run.condition.clone(),
                recorded: run.terrain_checksum,
                regenerated: scan.info.terrain_checksum,
            });
        }
        if scan.samples.len() as u64 != run.spatial_samples {
            return Err(AnalysisError::SampleCountMismatch {
                seed: run.seed,
                condition: run.condition.clone(),
                recorded: run.spatial_samples,
                found: scan.samples.len() as u64,
            });
        }

        let index = world_index(
            &terrain,
            &scan.samples,
            plan.burn_in_ticks,
            config.cell_size_m,
            plan.fine_scale_cells,
            plan.coarse_scale_cells,
        )
        .map_err(|refusal| refusal.to_string());

        outcomes.push(WorldOutcome {
            condition: run.condition.clone(),
            seed: run.seed,
            population: run.population,
            extinct: run.extinct,
            index,
        });
    }
    Ok(outcomes)
}

/// Which of the two indices a paired comparison is about.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IndexKind {
    Aggregation,
    Encounter,
}

/// Seed-matched pairs for one index, for feeding a power analysis.
pub fn pairs_for(
    outcomes: &[WorldOutcome],
    treatment: &str,
    control: &str,
    kind: IndexKind,
) -> Vec<Pair> {
    let mut seeds: Vec<u64> = outcomes
        .iter()
        .filter(|outcome| outcome.condition == treatment)
        .map(|outcome| outcome.seed)
        .collect();
    seeds.sort_unstable();
    seeds.dedup();

    let value = |index: &WorldIndex| match kind {
        IndexKind::Aggregation => index.aggregation_milli,
        IndexKind::Encounter => index.encounter_milli,
    };
    seeds
        .into_iter()
        .filter_map(|seed| {
            let find = |condition: &str| {
                outcomes
                    .iter()
                    .find(|outcome| outcome.condition == condition && outcome.seed == seed)
            };
            match (
                find(treatment)?.index.as_ref(),
                find(control)?.index.as_ref(),
            ) {
                (Ok(t), Ok(c)) => Some(Pair {
                    seed,
                    treatment_milli: value(t),
                    control_milli: value(c),
                }),
                _ => None,
            }
        })
        .collect()
}

/// Build the seed-paired contrast between two conditions.
pub fn contrast(
    outcomes: &[WorldOutcome],
    treatment: &str,
    control: &str,
    plan: &SpatialPlan,
) -> Contrast {
    let mut aggregation_pairs = Vec::new();
    let mut encounter_pairs = Vec::new();
    let mut unusable = Vec::new();

    let seeds: Vec<u64> = {
        let mut seeds: Vec<u64> = outcomes
            .iter()
            .filter(|outcome| outcome.condition == treatment)
            .map(|outcome| outcome.seed)
            .collect();
        seeds.sort_unstable();
        seeds.dedup();
        seeds
    };

    for seed in seeds {
        let find = |condition: &str| {
            outcomes
                .iter()
                .find(|outcome| outcome.condition == condition && outcome.seed == seed)
        };
        match (find(treatment), find(control)) {
            (Some(t), Some(c)) => match (&t.index, &c.index) {
                (Ok(treatment_index), Ok(control_index)) => {
                    aggregation_pairs.push(Pair {
                        seed,
                        treatment_milli: treatment_index.aggregation_milli,
                        control_milli: control_index.aggregation_milli,
                    });
                    encounter_pairs.push(Pair {
                        seed,
                        treatment_milli: treatment_index.encounter_milli,
                        control_milli: control_index.encounter_milli,
                    });
                }
                (Err(reason), _) => unusable.push((seed, format!("{treatment}: {reason}"))),
                (_, Err(reason)) => unusable.push((seed, format!("{control}: {reason}"))),
            },
            _ => unusable.push((seed, "missing run".to_owned())),
        }
    }

    Contrast {
        treatment: treatment.to_owned(),
        control: control.to_owned(),
        unusable_seeds: unusable,
        aggregation: compare(
            &aggregation_pairs,
            plan.sesoi_milli,
            plan.null_rate_milli,
            plan.direction,
            plan.analysis_seed,
        ),
        encounter: compare(
            &encounter_pairs,
            plan.sesoi_milli,
            plan.null_rate_milli,
            plan.direction,
            plan.analysis_seed ^ 0x9e37_79b9,
        ),
    }
}

/// Render a report as plain text. The format is deliberately flat and
/// greppable, matching the campaign manifest.
pub fn render(report: &SpatialReport) -> String {
    let mut out = String::new();
    out.push_str("spatial-report 1\n");
    out.push_str(&format!("campaign {}\n", report.campaign_id));
    out.push_str(&format!("index_version {}\n", report.index_version));
    out.push_str(&format!("stats_version {}\n", report.stats_version));
    out.push_str(&format!(
        "plan burn_in_ticks={} fine_scale_cells={} coarse_scale_cells={} sesoi_milli={} \
         null_rate_milli={} direction={} required_worlds={} analysis_seed={:#018x} bootstrap={}\n",
        report.plan.burn_in_ticks,
        report.plan.fine_scale_cells,
        report.plan.coarse_scale_cells,
        report.plan.sesoi_milli,
        report.plan.null_rate_milli,
        report.plan.direction.name(),
        report.plan.required_worlds,
        report.plan.analysis_seed,
        BOOTSTRAP_RESAMPLES,
    ));

    for world in &report.worlds {
        match &world.index {
            Ok(index) => out.push_str(&format!(
                "world condition={} seed={:#018x} population={} extinct={} samples={} \
                 observations={} ordered_pairs={} coarse_pairs={} fine_pairs={} \
                 aggregation_milli={} fine_milli={} encounter_milli={}\n",
                world.condition,
                world.seed,
                world.population,
                world.extinct,
                index.samples_used,
                index.observations,
                index.ordered_pairs,
                index.coarse_pairs,
                index.fine_pairs,
                index.aggregation_milli,
                index.fine_milli,
                index.encounter_milli,
            )),
            Err(reason) => out.push_str(&format!(
                "world condition={} seed={:#018x} population={} extinct={} refused={}\n",
                world.condition, world.seed, world.population, world.extinct, reason
            )),
        }
    }

    for contrast in &report.contrasts {
        for (label, result) in [
            ("aggregation", &contrast.aggregation),
            ("encounter", &contrast.encounter),
        ] {
            out.push_str(&format!(
                "contrast index={} treatment={} control={} pairs={} reaching_sesoi_any={} \
                 reaching_sesoi_directed={} required={} positive={} p_milli={} \
                 mean_diff_milli={} median_diff_milli={} ci95_milli=[{},{}] \
                 mean_rel_milli={} ci90_rel_milli=[{},{}] equivalent={} passes={}\n",
                label,
                contrast.treatment,
                contrast.control,
                result.pairs,
                result.reaching_sesoi,
                result.reaching_sesoi_directed,
                required_at(result.pairs, report.plan.required_worlds),
                result.positive_differences,
                result.sesoi_p_value_milli,
                result.mean_difference_milli,
                result.median_difference_milli,
                result.ci_low_milli,
                result.ci_high_milli,
                result.mean_relative_milli,
                result.relative_ci_low_milli,
                result.relative_ci_high_milli,
                result.equivalent,
                result.reaching_sesoi_directed
                    >= required_at(result.pairs, report.plan.required_worlds),
            ));
        }
        for (seed, reason) in &contrast.unusable_seeds {
            out.push_str(&format!(
                "unusable treatment={} control={} seed={:#018x} reason={}\n",
                contrast.treatment, contrast.control, seed, reason
            ));
        }
    }
    out
}
