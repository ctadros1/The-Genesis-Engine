//! World state and the deterministic tick.
//!
//! Organism storage is struct-of-arrays kept sorted by stable entity ID.
//! Births append strictly increasing IDs and removal preserves order, so
//! index order always equals entity-ID order; `check_invariants` verifies it.

use crate::actioncensus::ActionCensus;
use crate::checksum::Fnv1a64;
use crate::climate::{Biome, ClimateError, ClimateWorld};
use crate::config::{ConfigError, SimConfig};
use crate::contest::{Carcass, ContestState};
use crate::controller::{
    self, OUT_AVOID, OUT_EAT, OUT_FOLLOW, OUT_MATE, OUT_REST, OUT_THROTTLE, OUT_TURN, cos_bam_q15,
    sin_bam_q15,
};
use crate::genome::{
    CONTROLLER_INPUTS, Genome, Phenotype, VariationPolicy, VariationSummary, recombine,
};
use crate::learnstate::LearnState;
use crate::morphstate::MorphologyState;
use crate::origin::{self, OriginError};
use crate::phase2::{
    PairRejectReason, PendingChild, Phase2Counters, Phase2State, SENSOR_RANGE_MAX_M,
};
use crate::physiology::{HazardOutcome, PhysiologyState};
use crate::plasticity::{self, EdgeSignals, LearnedState};
use crate::rng::{RngSystem, named_random};
use crate::schema2::Schema2State;
use crate::structmut::MutationCounters;
use crate::terrainmod::{
    LAYER_CAPACITY_SCALE, LAYER_COUNT, LAYER_TRAVERSABLE, ModOutcome, TerrainModState,
    scale_capacity, value_in_domain,
};
use crate::worldgen::{self, Terrain, WorldGenError};
use std::fmt;

/// Q16 constant, kept local for fixed-point arithmetic.
const Q16: i64 = 65536;

/// Diagonal step scale: floor(1024 / sqrt(2)) over 1024.
const DIAGONAL_NUMERATOR: i64 = 724;
const DIAGONAL_DENOMINATOR: i64 = 1024;

/// Offspring are placed within this radius of the parent (policy v1).
const BIRTH_RADIUS_M: i64 = 2;
const BIRTH_PLACEMENT_ATTEMPTS: u32 = 8;

/// Deterministic jitter: one movement draw in eight overrides the food
/// gradient with a random direction (policy v1).
const JITTER_MODULUS: u64 = 8;

/// Bounded per-tick event buffer; overflow increments a deterministic
/// counter instead of growing without limit. The buffer is cleared at the
/// start of every tick, so caller drain behavior can never influence
/// simulation state or checksums.
///
/// Exported because the event-log codec caps a declared segment count
/// against it before allocating: a file claiming more events in one tick
/// than this kernel can produce did not come from this kernel.
pub const MAX_EVENTS_PER_TICK: usize = 4_096;

/// Movement directions: index 0 is "stay"; 1..=8 are the 8 neighbors in
/// row-major scan order (the documented deterministic tie order).
const DIRECTIONS: [(i8, i8); 9] = [
    (0, 0),
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];

mod artifact_tick;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TickPhase {
    Commands,
    Environment,
    SpatialIndex,
    Sense,
    /// Controller evaluation (Phase 2). Empty when Phase 2 is disabled;
    /// the phase boundary stays so per-phase timing is comparable.
    Controllers,
    Apply,
    /// Plastic-edge updates and their energy cost (Phase 11). Empty when the
    /// plasticity section is disabled; the phase boundary is emitted anyway,
    /// so per-phase timing stays comparable between a plasticity world and
    /// the world it is being compared against. The cost of that is a
    /// benchmark schema increment, which is cheaper than a benchmark table
    /// whose rows mean different things.
    Learn,
    Lifecycle,
    Finalize,
}

impl TickPhase {
    pub const ALL: [TickPhase; 9] = [
        TickPhase::Commands,
        TickPhase::Environment,
        TickPhase::SpatialIndex,
        TickPhase::Sense,
        TickPhase::Controllers,
        TickPhase::Apply,
        TickPhase::Learn,
        TickPhase::Lifecycle,
        TickPhase::Finalize,
    ];

    pub fn name(self) -> &'static str {
        match self {
            TickPhase::Commands => "commands",
            TickPhase::Environment => "environment",
            TickPhase::SpatialIndex => "spatial_index",
            TickPhase::Sense => "sense",
            TickPhase::Controllers => "controllers",
            TickPhase::Apply => "apply",
            TickPhase::Learn => "learn",
            TickPhase::Lifecycle => "lifecycle",
            TickPhase::Finalize => "finalize",
        }
    }
}

/// Host-side hook for timing phases. The kernel never reads a clock; a
/// caller may implement this with wall-clock instrumentation.
pub trait TickObserver {
    fn phase_started(&mut self, _phase: TickPhase) {}
    fn phase_finished(&mut self, _phase: TickPhase) {}
}

pub struct NoopObserver;

impl TickObserver for NoopObserver {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeathCause {
    Starvation,
    OldAge,
    /// Health depleted by damage (Phase 7). Terminal and idempotent like
    /// every other cause.
    Damage,
    /// Age-dependent hazard (Phase 8). Distinct from `OldAge`, which is the
    /// hard `max_age_ticks` cutoff senescence replaces: one is a hazard an
    /// organism lost a draw against, the other is a wall.
    Senescence,
    /// Non-food extrinsic hazard (Phase 8).
    Extrinsic,
}

impl DeathCause {
    pub fn name(self) -> &'static str {
        match self {
            DeathCause::Starvation => "starvation",
            DeathCause::OldAge => "old_age",
            DeathCause::Damage => "damage",
            DeathCause::Senescence => "senescence",
            DeathCause::Extrinsic => "extrinsic",
        }
    }
}

/// Event payloads. Version 1 covered Birth/Death/CapacityRejected/
/// Extinction; version 2 added the Phase 2 variants; version 3 adds the
/// Phase 7 contest variants; version 4 adds the structural-mutation
/// rejection (C9.6); version 5 adds the plasticity fault (Phase 11); version
/// 6 adds the nine Phase 12 object variants. Every increment is additive:
/// earlier payloads are unchanged. Reading events never alters simulation
/// state.
pub const EVENT_SCHEMA_VERSION: u32 = 6;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EventKind {
    Birth {
        id: u64,
        parent_id: u64,
    },
    Death {
        id: u64,
        cause: DeathCause,
    },
    CapacityRejected {
        parent_id: u64,
    },
    Extinction,
    /// Paired-parent child creation (Phase 2). Parent IDs are immutable;
    /// the variation counts audit the bounded numeric changes applied to
    /// the recombined record.
    PairedBirth {
        id: u64,
        parent_a: u64,
        parent_b: u64,
        genome_hash: u64,
        invest_a_milli: i64,
        invest_b_milli: i64,
        mutated_trait_genes: u32,
        mutated_neural_genes: u32,
    },
    /// A mutually selected pair failed validation after selection.
    PairRejected {
        parent_a: u64,
        parent_b: u64,
        reason: PairRejectReason,
    },
    /// A controller evaluation neutralized non-finite values this tick.
    ControllerFault {
        id: u64,
        faults: u32,
    },
    /// One landed attack (Phase 7).
    Damage {
        attacker: u64,
        target: u64,
        raw_milli: i64,
        applied_milli: i64,
        health_milli: i64,
    },
    /// Death by health depletion, carrying the attacker that finished it.
    DeathByDamage {
        id: u64,
        attacker: u64,
    },
    CarcassCreated {
        id: u64,
        source: u64,
        energy_milli: i64,
    },
    CarcassConsumed {
        id: u64,
        consumer: u64,
        energy_milli: i64,
    },
    /// A structural mutation operator was attempted on a child genome and
    /// refused (Phase 9, C9.6).
    ///
    /// The counters answer "how often did this class of rejection happen
    /// across the run", which is what a campaign needs; this answers "which
    /// child, at which tick, on which operator", which is what a *diagnosis*
    /// needs and no aggregate can reconstruct. Both are required: a cap that
    /// binds must reject, count, **and** event.
    ///
    /// Every typed rejection is carried, not only `RejectReason::Cap`. The
    /// reason field makes the cap subset filterable, and the classes that
    /// are expected rather than alarming (`Inapplicable`, `Cycle`) are
    /// exactly the ones whose *rate* is worth watching in a log.
    StructuralMutationRejected {
        /// The child whose mutation was refused. It is still born; the
        /// operator reverted, so the genome is its unmutated recombinant.
        child_id: u64,
        /// One of the `OP_*` codes in `structmut`.
        operator: u8,
        reason: crate::structmut::RejectReason,
    },
    /// One or more plastic-edge updates produced a non-finite delta this tick
    /// and were neutralized to zero (Phase 11).
    ///
    /// Follows the `ControllerFault` policy exactly, down to the shape:
    /// neutralize, count, event the **per-tick delta** rather than the
    /// lifetime total, never panic, and never let the value reach the
    /// checksum. Expected to stay at zero - the genome validator bounds every
    /// coefficient and activations are clamped - so a nonzero rate here is a
    /// bug report about validation, which is precisely why it is evented
    /// rather than only counted.
    PlasticityFault {
        id: u64,
        faults: u32,
    },
    /// An object entered the table (Phase 12). `cause` is an
    /// `artifact::CAUSE_*` value; `parent_id` is the fractured parent or the
    /// carcass's source organism, else 0.
    ObjectCreated {
        id: u64,
        material_id: u16,
        cause: u8,
        mass_milli: i64,
        energy_milli: i64,
        parent_id: u64,
    },
    /// An object left the table. `cause` is `artifact::DestroyCause::id`.
    ObjectDestroyed {
        id: u64,
        cause: u8,
    },
    /// An organism took a free object into its hold. `cell` is where the
    /// object was, so a log can say which cells were emptied.
    ObjectPickedUp {
        id: u64,
        holder: u64,
        cell: u32,
    },
    /// A held object returned to the world: dropped at the holder's position,
    /// placed into the faced cell, or dropped by a death. `cell` is where it
    /// landed, so C12.2's placed-object episodes and their cells can be
    /// reconstructed from the log without a per-tick object sample.
    ObjectReleased {
        id: u64,
        holder: u64,
        placed: bool,
        cell: u32,
    },
    /// One strike on an object, with the force it contributed. Every strike
    /// events; the fracture, if any, events as `ObjectDestroyed` after the
    /// forces on the target are summed.
    ObjectStruck {
        striker: u64,
        target: u64,
        force_q16: u32,
    },
    /// A strike on the terrain cell underfoot that extracted material; the
    /// object it created events as `ObjectCreated` at the end of the pass.
    TerrainStruck {
        striker: u64,
        cell: u32,
        volume_milli: i64,
        material_id: u16,
    },
    /// A held object and a free target became one composite.
    ObjectCombined {
        composite: u64,
        held: u64,
        target: u64,
        combiner: u64,
        depth: u8,
        joint_q16: u32,
    },
    /// An organism assimilated energy from an object.
    ObjectConsumed {
        id: u64,
        consumer: u64,
        energy_milli: i64,
    },
    /// An action was refused. `action` is `artifact::ObjectAction::id`,
    /// `reason` is `artifact::RefuseReason::id`. Every refusal events, so
    /// "fired" and "succeeded" are separable in the log and a cap that binds
    /// is visible (C12.7).
    ObjectActionRefused {
        id: u64,
        action: u8,
        reason: u8,
    },
    /// An organism's object history, emitted once at its death: ticks spent
    /// standing in a cell with a live placed object, ticks spent holding
    /// something, its age, and the capacity band of its birth cell. C12.2's
    /// per-organism record, paired by id with its births in the same log.
    /// Observation only; nothing in the tick reads it.
    ObjectExposure {
        id: u64,
        exposure_ticks: u64,
        carry_ticks: u64,
        age_ticks: u64,
        birth_band: u8,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Event {
    pub tick: u64,
    pub kind: EventKind,
}

/// Explicit energy/biomass ledger. Every transfer path records here so the
/// conservation invariant is exact integer arithmetic.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ledger {
    pub initial_energy_milli: i128,
    pub assimilated_milli: i128,
    pub spent_milli: i128,
    pub removed_at_death_milli: i128,
    pub initial_biomass_milli: i128,
    pub grown_milli: i128,
    pub consumed_biomass_milli: i128,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Counters {
    pub births_total: u64,
    pub deaths_starvation_total: u64,
    pub deaths_old_age_total: u64,
    pub capacity_rejections_total: u64,
    pub dropped_events_total: u64,
}

/// Compact read-only render record for observer streaming. Never carries
/// genome or controller matrices.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RenderEntity {
    pub id: u64,
    pub x_fp: i32,
    pub y_fp: i32,
    pub heading_bam: u16,
    pub mature: bool,
    pub pigment_hue_q8: u8,
    pub pigment_pattern_q8: u8,
    pub body_scale_q8: u8,
    pub energy_frac_q8: u8,
}

/// Bounded on-demand organism detail (HTTP inspector path).
#[derive(Clone, Copy, Debug)]
pub struct OrganismDetail {
    pub id: u64,
    pub x_fp: i32,
    pub y_fp: i32,
    pub energy_milli: i64,
    pub age_ticks: u64,
    pub cooldown_ticks: u64,
    pub phase2: Option<Phase2Detail>,
}

#[derive(Clone, Copy, Debug)]
pub struct Phase2Detail {
    pub heading_bam: u16,
    pub speed_milli: i64,
    pub trait_genes: [f32; crate::genome::TRAIT_COUNT],
    pub phenotype: Phenotype,
    pub parents: [u64; 2],
    pub ancestry_depth: u32,
    pub child_count: u32,
    pub birth_tick: u64,
    pub genome_hash: u64,
}

/// One organism's expressed structure and encoded genome size.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StructureSample {
    pub nodes: u32,
    pub edges: u32,
    pub genome_bytes: u32,
}

/// One organism's body size paired with what it has achieved.
///
/// C10.3's consequence clause needs both halves of this pair from the same
/// organism, and no existing view supplies them together. Observation only:
/// it hands out counts and returns nothing to the kernel (ADR-0016).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MorphologySample {
    pub modules: u32,
    pub child_count: u32,
    pub age_ticks: u64,
    /// Past its own maturity age. C10.3 restricts the correlation to mature
    /// organisms because a juvenile has had no opportunity to reproduce, and
    /// including them would measure age rather than morphology.
    pub mature: bool,
}

/// One organism's learned state, summarized (Phase 11).
///
/// **`sum_abs_learned_q16 == 0` is exactly "every plastic edge is at zero"**,
/// which is what makes C11.4 assertable directly rather than inferred from a
/// population mean: a mean of zero is also what you get from two organisms
/// whose deltas cancel, and a birth-reset test that could pass that way would
/// be no test at all.
///
/// Observation only (ADR-0016): it hands out numbers and returns nothing to
/// the kernel. A per-organism view rather than a summary for the reason
/// `structure_census` gives - C11.2 asks whether plasticity spread through
/// the population, and no aggregate can answer that.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LearnedSample {
    pub plastic_edges: u32,
    pub sum_abs_learned_q16: i64,
    pub max_abs_learned_q16: i32,
    pub sum_abs_trace_q16: i64,
    pub faults: u32,
    pub age_ticks: u64,
}

/// One organism's action histogram (Phase 11).
///
/// **The per-individual unit C11.1 is defined on.** A population histogram
/// answers a different question: births and deaths between two observations
/// make selection a complete explanation of any shift in it, so it cannot
/// distinguish "these organisms changed" from "different organisms are alive
/// now". `id` and `age_ticks` are carried with the counts precisely so an
/// analysis can verify it is comparing the *same* organism across two
/// samples rather than the same array slot.
///
/// Counts are **cumulative over the organism's life**, not per-window. A
/// window is the difference of two samples, which is strictly more
/// information than a reset and - unlike a reset - costs the world nothing.
///
/// Observation only (ADR-0016): it hands out numbers and instructs nothing.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ActionSample {
    pub id: u64,
    pub age_ticks: u64,
    /// Ticks in each class, indexed by [`crate::ActionClass`] discriminant.
    pub counts: [u32; crate::actioncensus::ACTION_CLASS_COUNT],
}

/// One organism's neutral marker alleles (Phase 11).
///
/// C11.2's drift control, reported per organism rather than as a population
/// summary for the reason `learned_census` gives: the criterion asks whether
/// a *distribution* shifted more than drift, and no aggregate can answer
/// that. Both haplotypes' alleles are summarized rather than blended,
/// because blending is expression and this locus has none - and because the
/// spread between the two alleles is heterozygosity, which is the quantity a
/// drift model is stated in.
///
/// Observation only (ADR-0016).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MarkerSample {
    pub id: u64,
    /// Marker loci carried across both haplotypes. Two in a founder; more
    /// after a duplication, fewer after a deletion, exactly as for an edge.
    pub alleles: u32,
    /// Sum of the marker values across those alleles, in milli.
    pub sum_value_milli: u32,
    /// Alleles carrying `MARKER_FLAG_NEUTRAL`, the control for the count of
    /// edges carrying `EDGE_FLAG_PLASTIC`.
    pub set_alleles: u32,
}

/// Point-in-time observable metrics (pure data; no clock). The Phase 2
/// fields are zero when Phase 2 is disabled.
#[derive(Clone, Copy, Debug)]
pub struct MetricsSnapshot {
    pub tick: u64,
    pub population: u64,
    pub births_total: u64,
    pub deaths_starvation_total: u64,
    pub deaths_old_age_total: u64,
    pub capacity_rejections_total: u64,
    pub dropped_events_total: u64,
    pub total_energy_milli: i64,
    pub total_biomass_milli: i64,
    pub extinct: bool,
    pub phase2_enabled: bool,
    pub paired_births_total: u64,
    pub pair_rejected_capacity_total: u64,
    pub pair_rejected_placement_total: u64,
    pub pair_rejected_energy_total: u64,
    pub controller_faults_total: u64,
    pub max_ancestry_depth: u32,
    pub contest_enabled: bool,
    pub attacks_total: u64,
    pub deaths_by_damage_total: u64,
    /// Phase 8. Zero when the physiology section is disabled.
    pub physiology_enabled: bool,
    pub deaths_senescence_total: u64,
    pub deaths_extrinsic_total: u64,
    pub deaths_juvenile_total: u64,
    pub mean_cumulative_hazard_q16: i64,
    pub max_age_ticks_observed: u64,
    /// Sum of every cell's effective capacity: the environmental carrying
    /// capacity C8.2 compares realized population against. Reported always,
    /// because "population divided by the memory guard" is the artifact
    /// this phase exists to stop measuring.
    pub total_capacity_milli: i64,
    /// Phase 9. Zero when the schema-2 section is disabled.
    pub genome2_enabled: bool,
    /// Mean expressed node and edge count, milli-units: the sensitive
    /// detector of any structural change at all.
    pub mean_nodes_milli: u64,
    pub mean_edges_milli: u64,
    /// Median expressed node and edge count, whole counts. C9.1's stated
    /// quantity, which asks the stricter question of whether structural
    /// change reached half the population.
    pub median_nodes: u64,
    pub median_edges: u64,
    /// Distinct `(node count, edge count)` pairs among living organisms.
    pub distinct_structures: u64,
    pub structural_mutations_applied: u64,
    pub structural_mutations_rejected: u64,
    /// Phase 10. Zero when the morphology section is disabled.
    pub morphology_enabled: bool,
    pub mean_modules_milli: u64,
    pub median_modules: u64,
    /// Distinct whole-body signatures among the living. C10.3's divergence
    /// measure, and a signature rather than a module count because A13 says
    /// novelty is not progress: two equal-sized bodies of different tissue
    /// are genuinely different morphologies, and counting modules alone
    /// would miss it.
    pub distinct_morphologies: u64,
    pub bodies_grown: u64,
    pub nonviable_bodies: u64,
    pub refused_node_budget: u64,
    pub carcasses: u64,
    pub total_carcass_energy_milli: i64,
    /// Phase 11. Zero when the plasticity section is disabled.
    pub plasticity_enabled: bool,
    /// Plastic edges across the living population, and the mean fraction of
    /// an organism's expressed edges that are plastic, in milli. C11.2 reads
    /// the second against the founder distribution: the count alone moves
    /// with population and with topology growth, so a rising count in a
    /// population that grew its networks is not evidence of anything.
    pub plastic_edges_total: u64,
    pub mean_plastic_fraction_milli: u64,
    /// Mean absolute learned delta over every plastic edge alive, in milli
    /// weight units against a clamp of 8000. A population full of plastic
    /// edges whose deltas are all zero is C11.1's null with C11.2's positive.
    ///
    /// **This is not on its own the "did anything actually learn" number and
    /// must never again be read as one.** It is a mean over every plastic
    /// edge alive, so a handful of edges that learned substantially inside a
    /// large population truncates it to zero. Read it with the two fields
    /// below or not at all.
    pub mean_abs_learned_milli: u64,
    /// The count and the extreme, beside the mean, because the mean alone
    /// published a false claim. Phase 11's confirmatory campaign reported
    /// `mean_abs_learned_milli` = 0 in all 30 treatment worlds and concluded
    /// that the mechanism "moved no weight by as much as one part in a
    /// thousand"; a census of the same snapshots found 25 of 48,119 plastic
    /// edges holding a nonzero learned weight, the largest at 229 milli
    /// (D-098). The mean was arithmetically correct and the conclusion drawn
    /// from it was wrong, which is the failure mode a second number prevents.
    ///
    /// This is the same split, for the same reason, that separated
    /// `plasticity_faults_total` and `plasticity_saturations_total` out of
    /// `plasticity_anomalies_total` under D-074.
    pub learned_edges_nonzero: u64,
    pub max_abs_learned_milli: u64,
    /// Plastic edges **visited**, and the anomaly half - faults plus clamp
    /// saturations. Runaway plasticity destabilizing controllers into noise is
    /// a named risk of this phase and this is its measurement.
    ///
    /// **"Visited", not "applied", and the difference was 95.43 percent.**
    /// This is `PlasticityCounters::total_evaluated` - applied plus static
    /// plus refused - and the confirmatory findings read its 1,109,373,897 as
    /// "the mechanism executed a billion times" when 1.06 billion of those
    /// were `StepKind::Static`: the early return for a flagged edge whose rule
    /// is 0, taken before any gene is read. One number that can only answer
    /// one question, read as answering both (D-098). The three dispositions
    /// below are what actually answer it, and this sum is kept beside them
    /// rather than replaced, exactly as `plasticity_anomalies_total` was kept
    /// when D-074 split it.
    pub plasticity_updates_total: u64,
    /// Updates that **moved learned state**: `StepKind::Applied`.
    ///
    /// This is the number a report means when it says the mechanism ran, and
    /// the one applied-step pricing charges against (D-107). A run with a
    /// large `plasticity_updates_total` and a zero here is a world that
    /// carried the machinery and never used it - which is what the Phase 11
    /// campaign measured, and could not say.
    pub plasticity_updates_applied: u64,
    /// Flagged edges whose rule is Static, returning before any gene is read.
    ///
    /// Not noise: a rule-0 plastic edge still pays the per-edge energy cost,
    /// so this is how a campaign tells "plasticity was selected down by
    /// turning the rule off" from "plasticity was selected down by dropping
    /// the flag".
    pub plasticity_updates_static: u64,
    /// Rule ids outside the registry. Expected to stay at zero forever; a
    /// nonzero value is a genome-validation bug report, so it is reported
    /// separately rather than folded into either number above - D-074's
    /// lesson that a bug signal summed into a busy counter stops being a
    /// signal.
    pub plasticity_updates_refused: u64,
    pub plasticity_anomalies_total: u64,
    /// Split out of `plasticity_anomalies_total`, which sums both, because
    /// the halves mean opposite things: a saturation is the clamp working
    /// and is expected in any run where anything learned, while a fault is a
    /// non-finite value that should be unreachable and is a bug report. A
    /// single total is how a bug signal becomes noise (D-074).
    pub plasticity_faults_total: u64,
    pub plasticity_saturations_total: u64,
    /// Flagged edges refused by `max_plastic_edges`. C11.7 sets that cap from
    /// measurement, and a population sitting hard against it looks exactly
    /// like a population that evolved that much plasticity without this.
    pub plastic_edges_over_cap: u64,
    /// Energy charged for plastic edges over the run, milli-EU. Already
    /// inside `Ledger::spent_milli`; reported separately because "the ledger
    /// balances" and "plasticity cost what we think" are different claims.
    pub plasticity_cost_milli: i64,
    /// Phase 12. Zero when the mutable-world section is disabled.
    pub worldmod_enabled: bool,
    /// Stored overrides, in total and per layer in layer-id order. Per layer
    /// as well as in total because the layers mean unrelated things and a
    /// single count is how a full traversability layer hides behind an empty
    /// material one.
    pub worldmod_overrides: u64,
    pub worldmod_overrides_by_layer: [u64; LAYER_COUNT as usize],
    /// Relocations of the resource patch that have run.
    pub worldmod_relocations: u64,
    /// Biomass trimmed because a modification lowered a cell's capacity below
    /// its standing biomass, and the number of cells it came off.
    ///
    /// **The quantity that defines this phase's control arm.** A control
    /// running the identical schedule at scale 1.0 reports zero here and a
    /// treatment does not, which is what makes their standing biomass
    /// comparable; a schedule-free control would differ by this sink for
    /// reasons unrelated to anything being measured.
    pub worldmod_capacity_loss_milli: i64,
    pub worldmod_cells_trimmed: u64,
    /// Modification writes refused for any reason - cap, occupancy, or an
    /// invalid layer/cell/value. C12.7: a run silently pressed against a cap
    /// must be visible in its report.
    pub worldmod_refusals: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InvariantViolation {
    EntityOrder,
    PositionInvalid {
        id: u64,
    },
    EnergyOutOfBounds {
        id: u64,
        energy_milli: i64,
    },
    AgeOutOfBounds {
        id: u64,
        age_ticks: u64,
    },
    BiomassOutOfBounds {
        cell: usize,
        biomass_milli: i64,
    },
    EnergyLedgerMismatch {
        expected: i128,
        actual: i128,
    },
    BiomassLedgerMismatch {
        expected: i128,
        actual: i128,
    },
    PopulationAccounting {
        expected: i128,
        actual: i128,
    },
    EntityIdAllocation {
        expected: u64,
        actual: u64,
    },
    Phase2Desync {
        organisms: usize,
        phase2: usize,
    },
    InvalidGenome {
        id: u64,
    },
    AncestryInvalid {
        id: u64,
    },
    ControllerStateInvalid {
        id: u64,
    },
    ContestDesync {
        organisms: usize,
        contest: usize,
    },
    /// The Phase 8 physiology arrays fell out of lockstep with the organism
    /// arrays. Every parallel-array subsystem needs this check: a missed
    /// push on one of the two birth paths is invisible until it panics with
    /// an index out of bounds several thousand ticks later.
    PhysiologyDesync {
        organisms: usize,
        physiology: usize,
    },
    /// The Phase 9 schema-2 arrays fell out of lockstep.
    Schema2Desync {
        organisms: usize,
        schema2: usize,
    },
    /// A schema-2 genome in world state failed validation. The mutation
    /// operators produce valid records by construction, so this is a bug
    /// report rather than a runtime condition.
    Schema2Invalid {
        id: u64,
    },
    MorphologyDesync {
        organisms: usize,
        morphology: usize,
    },
    /// The Phase 11 learned-state arrays fell out of lockstep with the
    /// organism arrays, or one organism's row is not the length its compiled
    /// plan says it should be.
    ///
    /// Both halves matter and the second is the one a length check on the
    /// outer array would miss: a row of the wrong width would let an organism
    /// index another organism's learned delta, or index past the end. This is
    /// the price D2 records for keeping learned state in its own
    /// `Option<LearnState>` instead of on `Schema2State` - lockstep is
    /// maintained by hand, so it is asserted rather than argued.
    LearnDesync {
        organisms: usize,
        learn: usize,
    },
    /// A learned delta or eligibility trace outside the clamp the update
    /// arithmetic promises. `accumulate_clamped` cannot produce one, so this
    /// is a restore or a future initialization policy, not the tick.
    LearnBounds {
        id: u64,
    },
    /// The Phase 11 action-census rows fell out of lockstep with the organism
    /// arrays.
    ///
    /// The same obligation every parallel-array subsystem carries, and here
    /// the failure it prevents is *quiet* rather than loud: `record` indexes
    /// by organism, so a short array panics, but a long one - a missed
    /// `retain` - would silently attribute a dead organism's history to
    /// whichever organism compacted into its slot. That is a per-individual
    /// series that looks exactly like a per-individual series and is not one,
    /// which is the single worst outcome available to C11.1.
    ActionCensusDesync {
        organisms: usize,
        census: usize,
    },
    ContestStateInvalid {
        id: u64,
    },
    CarcassOrder,
    CarcassLedgerMismatch {
        expected: i128,
        actual: i128,
    },
    /// The Phase 12 modification set is not strictly ascending by
    /// `(layer_id, cell_index)`, duplicates a key, or has ragged arrays.
    ///
    /// The counterpart of `EntityOrder` for the other sorted array in the
    /// world, and it is checked for the same reason: sortedness is what makes
    /// application a deterministic ordered scan and what makes a logical
    /// modification set have exactly one encoding. `set` and `clear` cannot
    /// break it, so the path this defends is a restore decoding an untrusted
    /// payload - which is the path that matters, because a modification
    /// section is the one part of a Phase 12 save that is not regenerated.
    TerrainModOrder {
        index: usize,
    },
    /// A modification's layer id, cell index, or value is outside its
    /// documented domain. Same provenance as `TerrainModOrder`: the writers
    /// validate, so this is a decoded payload or a future producer.
    TerrainModBounds {
        index: usize,
    },
    /// Phase 12 object table defect, named by `artifact::TableViolation`.
    ObjectTable {
        violation: crate::artifact::TableViolation,
    },
    /// The per-organism object arrays are not in lockstep with the population.
    ObjectDesync {
        organisms: usize,
        held: usize,
    },
    /// The held-list cache disagrees with `holder_id`.
    ObjectHeldMismatch,
    /// More objects than `artifact.max_objects`.
    ObjectCap {
        objects: usize,
        cap: u32,
    },
    /// An object held by an organism that does not exist.
    ObjectHolderDead {
        id: u64,
        holder: u64,
    },
    /// A genome bound to a channel this world's registry version does not
    /// offer (ADR-0028 section 7).
    ChannelNotOffered {
        id: u64,
        registry_version: u16,
    },
}

impl fmt::Display for InvariantViolation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for InvariantViolation {}

/// Fill absent expressed traits with the midpoint.
///
/// A schema-2 genome may lack a trait locus entirely - deletion can remove
/// one from both haplotypes - and the phenotype mapping needs a value. The
/// midpoint is chosen because it is the neutral point of every trait's
/// range; zero would silently push the organism to one extreme and look
/// like selection.
pub(crate) fn resolve_traits(
    expressed: &[Option<f32>; crate::genome::TRAIT_COUNT],
) -> [f32; crate::genome::TRAIT_COUNT] {
    let mut out = [0.5_f32; crate::genome::TRAIT_COUNT];
    for (slot, value) in expressed.iter().enumerate() {
        if let Some(value) = value {
            out[slot] = *value;
        }
    }
    out
}

#[derive(Clone, Debug)]
pub enum NewWorldError {
    Config(ConfigError),
    WorldGen(WorldGenError),
    Climate(ClimateError),
    Origin(OriginError),
}

impl fmt::Display for NewWorldError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(error) => write!(formatter, "invalid config: {error}"),
            Self::WorldGen(error) => write!(formatter, "world generation failed: {error}"),
            Self::Climate(error) => write!(formatter, "climate generation failed: {error}"),
            Self::Origin(error) => write!(formatter, "founder generation failed: {error}"),
        }
    }
}

impl std::error::Error for NewWorldError {}

#[derive(Clone, Debug)]
pub struct World {
    config: SimConfig,
    config_hash: u64,
    terrain: Terrain,
    tick: u64,
    paused: bool,
    extinct: bool,
    next_entity_id: u64,

    // Organism SoA, always sorted by ID.
    ids: Vec<u64>,
    x_fp: Vec<i32>,
    y_fp: Vec<i32>,
    energy_milli: Vec<i64>,
    age_ticks: Vec<u64>,
    cooldown_ticks: Vec<u64>,

    // Dynamic environment.
    biomass_milli: Vec<i64>,

    // Spatial buckets over organism positions.
    buckets: Vec<Vec<u32>>,
    buckets_x: u32,
    buckets_y: u32,
    bucket_size_fp: i32,

    // Per-tick intent buffers (reused; not part of logical state).
    intent_direction: Vec<u8>,
    intent_crowded: Vec<bool>,
    intent_reproduce: Vec<bool>,
    pending_births: Vec<(u64, i32, i32)>,

    ledger: Ledger,
    counters: Counters,
    events: Vec<Event>,

    // Derived per-tick constants.
    step_fp: i32,
    basal_cost_tick: i64,
    move_cost_tick: i64,
    crowding_cost_tick: i64,
    intake_tick: i64,
    /// Energy one plastic edge costs per tick, in **thousandths of a
    /// milli-EU** - the numerator `milli_per_s * dt_ms`, kept undivided.
    ///
    /// Every other per-tick cost here divides by 1000 and truncates, and for
    /// them that is right: they are charged once per organism and the
    /// truncation is a fixed fraction of a large number. Plasticity is
    /// charged per *edge*, so the truncation lands on a small number many
    /// times over, and at the shipped rate of 2 milli/s with `dt_ms = 100`
    /// it landed on **zero** - a plastic edge was free, and 10 milli/s was
    /// the cheapest rate that charged anything at all. The division happens
    /// at the debit site instead, against a carried remainder, so the price
    /// of an edge can be a fraction of a milli. Zero when plasticity is
    /// disabled.
    plastic_edge_cost_milli_thousandths: i64,
    crowding_radius_fp: i64,

    /// Phase 2 state; `None` exactly when `config.phase2.enabled` is false.
    phase2: Option<Phase2State>,

    /// Phase 6 climate; `None` exactly when `config.climate.enabled` is
    /// false, so a disabled world takes the exact Phase 1/2 code paths.
    climate: Option<ClimateWorld>,

    /// Genomes produced by a non-default origin, consumed when Phase 2
    /// state is built. Empty on the default path, which draws founder
    /// genomes from `GenomeInit` exactly as before.
    founder_genomes: Vec<Genome>,

    /// Phase 7 contest; `None` exactly when `config.contest.enabled` is
    /// false, so a disabled world takes the Phase 2 code paths.
    contest: Option<ContestState>,
    physiology: Option<PhysiologyState>,
    schema2: Option<Schema2State>,
    morphology: Option<MorphologyState>,
    /// Phase 11 learned state; `None` exactly when
    /// `config.plasticity.enabled` is false, so a disabled world compiles no
    /// plastic edge, runs an empty learn phase, and appends nothing to the
    /// checksum.
    learn: Option<LearnState>,
    /// Phase 12 terrain modification set; `None` exactly when
    /// `config.worldmod.enabled` is false.
    ///
    /// The `None` arm is what preserves four fixtures. Both composed
    /// accessors match on this option and their `None` arms are the
    /// pre-Phase-12 expressions unchanged, character for character, so a
    /// disabled world does not merely *compute* the same capacity - it runs
    /// the same code.
    worldmod: Option<TerrainModState>,
    /// Phase 11 per-organism action counts; `None` exactly when
    /// `config.probe.action_census_enabled` is false, so a world without the
    /// probe writes no counter, stores no row, and appends nothing to the
    /// checksum.
    ///
    /// **Written by `apply_phase2` and read by nobody in the tick.** That is
    /// the property the five fixtures assert and the reason this can exist at
    /// all without being an intervention (ADR-0016).
    action_census: Option<ActionCensus>,
    /// Phase 12 objects; `None` exactly when `config.artifact.enabled` is
    /// false, so a world without the section offers no object channel, runs
    /// no object pass, appends nothing to the checksum, and takes the Phase 7
    /// carcass path (ADR-0028).
    objects: Option<crate::artifact::ObjectState>,
}

impl World {
    pub fn new(config: SimConfig) -> Result<Self, NewWorldError> {
        config.validate().map_err(NewWorldError::Config)?;
        let terrain = worldgen::generate(&config).map_err(NewWorldError::WorldGen)?;

        let biomass_milli: Vec<i64> = terrain
            .capacity_milli
            .iter()
            .map(|&capacity| (capacity * i64::from(config.initial_biomass_q16)) >> 16)
            .collect();

        // Bucket size must cover the largest interaction range so a 3x3
        // bucket ring always suffices. Phase 2 adds genome-driven sensing
        // (bounded by SENSOR_RANGE_MAX_M) and pairing range.
        let bucket_size_m = if config.phase2.enabled {
            config
                .crowding_radius_m
                .max(SENSOR_RANGE_MAX_M)
                .max(config.phase2.pairing_range_m)
        } else {
            config.crowding_radius_m
        };
        let bucket_size_fp = (bucket_size_m as i32) * crate::FP_PER_METER;
        // Both operands are positive; signed div_ceil is not yet stable.
        let buckets_x = ((config.world_extent_x_fp() + bucket_size_fp - 1) / bucket_size_fp) as u32;
        let buckets_y = ((config.world_extent_y_fp() + bucket_size_fp - 1) / bucket_size_fp) as u32;

        let dt = i64::from(config.dt_ms);
        let step_fp = (i64::from(config.speed_mps_q16) * dt * i64::from(crate::FP_PER_METER)
            / (Q16 * 1000)) as i32;

        let mut world = Self {
            config_hash: config.stable_hash(),
            terrain,
            tick: 0,
            paused: false,
            extinct: false,
            next_entity_id: 1,
            ids: Vec::new(),
            x_fp: Vec::new(),
            y_fp: Vec::new(),
            energy_milli: Vec::new(),
            age_ticks: Vec::new(),
            cooldown_ticks: Vec::new(),
            biomass_milli,
            buckets: (0..(buckets_x as usize) * (buckets_y as usize))
                .map(|_| Vec::new())
                .collect(),
            buckets_x,
            buckets_y,
            bucket_size_fp,
            intent_direction: Vec::new(),
            intent_crowded: Vec::new(),
            intent_reproduce: Vec::new(),
            pending_births: Vec::new(),
            ledger: Ledger::default(),
            counters: Counters::default(),
            events: Vec::new(),
            step_fp,
            basal_cost_tick: config.basal_cost_milli_per_s * dt / 1000,
            move_cost_tick: config.move_cost_milli_per_s * dt / 1000,
            crowding_cost_tick: config.crowding_cost_milli_per_s * dt / 1000,
            intake_tick: config.intake_rate_milli_per_s * dt / 1000,
            plastic_edge_cost_milli_thousandths: if config.plasticity.enabled {
                config.plasticity.plastic_edge_cost_milli_per_s * dt
            } else {
                0
            },
            crowding_radius_fp: i64::from(config.crowding_radius_m)
                * i64::from(crate::FP_PER_METER),
            phase2: None,
            climate: None,
            founder_genomes: Vec::new(),
            contest: None,
            physiology: None,
            schema2: None,
            morphology: None,
            learn: None,
            action_census: None,
            objects: None,
            // Built here rather than after the population, because it is
            // empty either way: the relocation schedule first fires at tick
            // `relocate_interval_ticks`, so a freshly generated world's
            // terrain is exactly its baseline. That is deliberate - founder
            // placement, initial biomass seeding, and the worldgen
            // validations are all generation-time properties, and a patch
            // that existed at tick 0 would make a world's biomes depend on a
            // schedule rather than on its seed.
            worldmod: config
                .worldmod
                .enabled
                .then(crate::terrainmod::TerrainModState::default),
            config,
        };

        // Climate is built before the population so founders are placed in
        // a classified world and capacity already reflects biomes.
        if world.config.climate.enabled {
            let climate =
                ClimateWorld::new(&world.terrain, &world.config).map_err(NewWorldError::Climate)?;
            // Initial biomass follows the effective (biome-scaled) capacity,
            // not the raw elevation capacity.
            for cell in 0..world.biomass_milli.len() {
                let capacity = climate.capacity_milli(&world.terrain, &world.config.climate, cell);
                world.biomass_milli[cell] =
                    (capacity * i64::from(world.config.initial_biomass_q16)) >> 16;
            }
            world.climate = Some(climate);
        }

        // The default origin takes the exact Phase 1/2 founder path, which
        // is what preserves both fixtures; any other origin goes through the
        // Phase 6 generator.
        if origin::is_default_origin(&world.config.origin) {
            world.spawn_initial_population();
        } else {
            let biome: Vec<Biome> = world
                .climate
                .as_ref()
                .map(|climate| climate.state.biome.clone())
                .unwrap_or_default();
            let founders = origin::generate_founders(&world.config, &world.terrain, &biome)
                .map_err(NewWorldError::Origin)?;
            world.place_founders(founders);
        }
        if world.config.phase2.enabled {
            let mut state = Phase2State::with_capacity(world.ids.len());
            for index in 0..world.ids.len() {
                let id = world.ids[index];
                let genome = match world.founder_genomes.get(index) {
                    Some(genome) => genome.clone(),
                    None => Genome::founder(world.config.world_seed, id),
                };
                let genome_hash = genome.stable_hash();
                let phenotype = Phenotype::derive(&genome);
                let heading = (named_random(world.config.world_seed, 0, RngSystem::Spawn, id, 3)
                    & 0xffff) as u16;
                state.push_organism(Some(genome), genome_hash, phenotype, heading, [0, 0], 0, 0);
            }
            world.phase2 = Some(state);
        }
        world.founder_genomes = Vec::new();
        if world.config.contest.enabled {
            let mut contest = ContestState::with_capacity(world.ids.len());
            if let Some(p2) = world.phase2.as_ref() {
                for phenotype in &p2.phenotypes {
                    contest.push_organism(ContestState::health_max_milli(
                        &world.config.contest,
                        phenotype.body_scale_milli,
                    ));
                }
            }
            world.contest = Some(contest);
        }
        if world.config.genome2.enabled {
            let mut schema2 = Schema2State::with_capacity(world.ids.len());
            let budget = world.config.plasticity_budget();
            let mut learn = world
                .config
                .plasticity
                .enabled
                .then(|| LearnState::with_capacity(world.ids.len()));
            let morphology_config = world.config.morphology;
            let mut morphology = morphology_config
                .enabled
                .then(|| MorphologyState::with_capacity(world.ids.len()));
            let marker_enabled =
                world.config.probe.enabled && world.config.probe.marker_locus_enabled;
            if let Some(p2) = world.phase2.as_mut() {
                for index in 0..world.ids.len() {
                    // A morphology world's founder carries the one-rule growth
                    // program as well; a schema-2 world's founder is
                    // byte-identical to what it was before Phase 10.
                    let genome = if morphology_config.enabled {
                        crate::schema2::founder_with_morphology(p2.genomes[index].traits())
                    } else {
                        crate::schema2::founder_from_traits(p2.genomes[index].traits())
                    };
                    // The neutral marker is layered on last, exactly as the
                    // growth program is, so a founder in a world without the
                    // probe section is byte-identical to what it was before
                    // this line existed.
                    let genome = if marker_enabled {
                        crate::schema2::with_marker_locus(genome)
                    } else {
                        genome
                    };
                    let traits = resolve_traits(&genome.express_traits());
                    p2.phenotypes[index] = match morphology.as_mut() {
                        Some(state) => {
                            state.push_organism(&genome, &morphology_config).map_err(
                                |failure| {
                                    NewWorldError::Config(
                                    crate::config::ConfigError::PhysiologyRange(
                                        match failure {
                                            crate::morphology::ViabilityFailure::Empty => {
                                                "founder body is empty"
                                            }
                                            crate::morphology::ViabilityFailure::Disconnected => {
                                                "founder body is disconnected"
                                            }
                                            _ => "founder body is not viable",
                                        },
                                        index as i64,
                                    ),
                                )
                                },
                            )?;
                            let derived = state.derived[state.derived.len() - 1];
                            crate::genome::Phenotype::from_body(&traits, &derived, &state.reference)
                        }
                        None => crate::genome::Phenotype::from_traits(&traits),
                    };
                    if !schema2.push_organism(genome, budget) {
                        return Err(NewWorldError::Config(
                            crate::config::ConfigError::PhysiologyRange(
                                "founder genome does not compile",
                                index as i64,
                            ),
                        ));
                    }
                    // Founders start at zero on every plastic edge for the
                    // same reason children do: learned state is never
                    // inherited and never seeded. Pushed here, in the same
                    // loop as the plan it is sized from, because the length
                    // has to be the plan's and reading it from anywhere else
                    // is how the two arrays drift apart.
                    if let Some(state) = learn.as_mut() {
                        state.push_organism(schema2.plastic_edges(index));
                    }
                }
                // Schema 1's flat genomes play no part in a schema-2 world;
                // keeping them would double the memory and invite code to
                // read a genome the organism does not have. The hashes stay,
                // now identifying the schema-2 genome.
                p2.genomes.clear();
                for (index, hash) in p2.genome_hashes.iter_mut().enumerate() {
                    *hash = crate::checksum::fnv1a64(&schema2.genomes[index].encode());
                }
            }
            world.schema2 = Some(schema2);
            world.morphology = morphology;
            world.learn = learn;
        }
        if world.config.physiology.enabled {
            let mut physiology = PhysiologyState::with_capacity(world.ids.len());
            for _ in 0..world.ids.len() {
                physiology.push_organism();
            }
            world.physiology = Some(physiology);
        }
        // The action census is built last and independently of every genome
        // section: a row is a histogram, not a function of the organism's
        // genome, so it needs nothing from schema 2 and is sized from the
        // population alone. Founders start at zero for the reason children
        // do - `push_organism` takes no initial value.
        if world.config.probe.enabled && world.config.probe.action_census_enabled {
            let mut census = ActionCensus::with_capacity(world.ids.len());
            for _ in 0..world.ids.len() {
                census.push_organism();
            }
            world.action_census = Some(census);
        }
        // Phase 12 objects: an empty table and one held-list per founder.
        // Sized from the population like the census; nothing about a founder
        // decides anything here.
        if world.config.artifact.enabled {
            let mut objects = crate::artifact::ObjectState::with_capacity(world.ids.len());
            objects.band_thresholds = world.capacity_band_thresholds();
            for index in 0..world.ids.len() {
                let cell = world.cell_of(world.x_fp[index], world.y_fp[index]);
                let band = objects.band_of(world.terrain.capacity_milli[cell]);
                objects.push_organism(band);
            }
            world.objects = Some(objects);
        }
        world.ledger.initial_energy_milli = world
            .energy_milli
            .iter()
            .map(|&energy| i128::from(energy))
            .sum();
        world.ledger.initial_biomass_milli = world
            .biomass_milli
            .iter()
            .map(|&biomass| i128::from(biomass))
            .sum();
        Ok(world)
    }

    /// Materialize generated founders. Entity IDs arrive already allocated
    /// in canonical `(group, draw_index)` order, so this only places them.
    fn place_founders(&mut self, founders: Vec<crate::origin::Founder>) {
        let cell_fp = i64::from(self.config.cell_size_fp());
        let seed = self.config.world_seed;
        let mut founder_genomes = Vec::with_capacity(founders.len());
        for founder in founders {
            let cell_x = (founder.cell % self.terrain.cells_x as usize) as i64;
            let cell_y = (founder.cell / self.terrain.cells_x as usize) as i64;
            let jitter_x = named_random(seed, 0, RngSystem::FounderSeed, founder.entity_id, 4_096);
            let jitter_y = named_random(seed, 0, RngSystem::FounderSeed, founder.entity_id, 4_097);
            let x = cell_x * cell_fp + 1 + (jitter_x % (cell_fp - 2) as u64) as i64;
            let y = cell_y * cell_fp + 1 + (jitter_y % (cell_fp - 2) as u64) as i64;
            self.ids.push(founder.entity_id);
            self.x_fp.push(x as i32);
            self.y_fp.push(y as i32);
            self.energy_milli.push(self.config.initial_energy_milli);
            self.age_ticks.push(0);
            self.cooldown_ticks.push(0);
            founder_genomes.push(founder.genome);
        }
        self.next_entity_id = self.ids.len() as u64 + 1;
        self.founder_genomes = founder_genomes;
    }

    fn spawn_initial_population(&mut self) {
        let habitable: Vec<usize> = (0..self.terrain.cell_count())
            .filter(|&index| self.terrain.capacity_milli[index] > 0)
            .collect();
        let cell_fp = i64::from(self.config.cell_size_fp());
        for organism in 1..=u64::from(self.config.initial_organisms) {
            let seed = self.config.world_seed;
            let cell_draw = named_random(seed, 0, RngSystem::Spawn, organism, 0);
            let cell = habitable[(cell_draw % habitable.len() as u64) as usize];
            let cell_x = (cell % self.terrain.cells_x as usize) as i64;
            let cell_y = (cell / self.terrain.cells_x as usize) as i64;
            let jitter_x = named_random(seed, 0, RngSystem::Spawn, organism, 1);
            let jitter_y = named_random(seed, 0, RngSystem::Spawn, organism, 2);
            let x = cell_x * cell_fp + 1 + (jitter_x % (cell_fp - 2) as u64) as i64;
            let y = cell_y * cell_fp + 1 + (jitter_y % (cell_fp - 2) as u64) as i64;
            self.ids.push(organism);
            self.x_fp.push(x as i32);
            self.y_fp.push(y as i32);
            self.energy_milli.push(self.config.initial_energy_milli);
            self.age_ticks.push(0);
            self.cooldown_ticks.push(0);
        }
        self.next_entity_id = u64::from(self.config.initial_organisms) + 1;
    }

    // --- Accessors -------------------------------------------------------

    pub fn config(&self) -> &SimConfig {
        &self.config
    }

    pub fn config_hash(&self) -> u64 {
        self.config_hash
    }

    pub fn terrain(&self) -> &Terrain {
        &self.terrain
    }

    pub fn tick_number(&self) -> u64 {
        self.tick
    }

    pub fn population(&self) -> usize {
        self.ids.len()
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    pub fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
    }

    pub fn is_extinct(&self) -> bool {
        self.extinct
    }

    pub fn counters(&self) -> Counters {
        self.counters
    }

    pub fn ledger(&self) -> Ledger {
        self.ledger
    }

    pub fn total_energy_milli(&self) -> i64 {
        self.energy_milli.iter().sum()
    }

    pub fn total_biomass_milli(&self) -> i64 {
        self.biomass_milli.iter().sum()
    }

    /// Events produced by the most recent tick. The buffer is replaced at
    /// the start of every tick regardless of whether it was read, so host
    /// read patterns can never change simulation state.
    pub fn events(&self) -> &[Event] {
        &self.events
    }

    pub fn metrics(&self) -> MetricsSnapshot {
        let phase2 = self.phase2.as_ref();
        let phase2_counters = phase2.map(|p2| p2.counters).unwrap_or_default();
        MetricsSnapshot {
            tick: self.tick,
            population: self.ids.len() as u64,
            births_total: self.counters.births_total,
            deaths_starvation_total: self.counters.deaths_starvation_total,
            deaths_old_age_total: self.counters.deaths_old_age_total,
            capacity_rejections_total: self.counters.capacity_rejections_total,
            dropped_events_total: self.counters.dropped_events_total,
            total_energy_milli: self.total_energy_milli(),
            total_biomass_milli: self.total_biomass_milli(),
            extinct: self.extinct,
            phase2_enabled: phase2.is_some(),
            paired_births_total: phase2_counters.paired_births_total,
            pair_rejected_capacity_total: phase2_counters.pair_rejected_capacity_total,
            pair_rejected_placement_total: phase2_counters.pair_rejected_placement_total,
            pair_rejected_energy_total: phase2_counters.pair_rejected_energy_total,
            controller_faults_total: phase2_counters.controller_faults_total,
            max_ancestry_depth: phase2.map(|p2| p2.max_depth()).unwrap_or(0),
            genome2_enabled: self.schema2.is_some(),
            mean_nodes_milli: self
                .schema2
                .as_ref()
                .map_or(0, |state| state.mean_structure_milli().0),
            mean_edges_milli: self
                .schema2
                .as_ref()
                .map_or(0, |state| state.mean_structure_milli().1),
            median_nodes: self
                .schema2
                .as_ref()
                .map_or(0, |state| state.median_structure().0),
            median_edges: self
                .schema2
                .as_ref()
                .map_or(0, |state| state.median_structure().1),
            distinct_structures: self
                .schema2
                .as_ref()
                .map_or(0, |state| state.distinct_structures() as u64),
            structural_mutations_applied: self
                .schema2
                .as_ref()
                .map_or(0, |state| state.counters.total_applied()),
            structural_mutations_rejected: self
                .schema2
                .as_ref()
                .map_or(0, |state| state.counters.total_rejected()),
            morphology_enabled: self.morphology.is_some(),
            mean_modules_milli: self
                .morphology
                .as_ref()
                .map_or(0, |state| state.mean_modules_milli()),
            median_modules: self
                .morphology
                .as_ref()
                .map_or(0, |state| state.median_modules()),
            distinct_morphologies: self
                .morphology
                .as_ref()
                .map_or(0, |state| state.distinct_morphologies() as u64),
            bodies_grown: self
                .morphology
                .as_ref()
                .map_or(0, |state| state.counters.bodies_grown),
            nonviable_bodies: self
                .morphology
                .as_ref()
                .map_or(0, |state| state.counters.total_nonviable()),
            refused_node_budget: self
                .morphology
                .as_ref()
                .map_or(0, |state| state.counters.refused_node_budget),
            physiology_enabled: self.physiology.is_some(),
            deaths_senescence_total: self
                .physiology
                .as_ref()
                .map_or(0, |p| p.deaths_senescence_total),
            deaths_extrinsic_total: self
                .physiology
                .as_ref()
                .map_or(0, |p| p.deaths_extrinsic_total),
            deaths_juvenile_total: self
                .physiology
                .as_ref()
                .map_or(0, |p| p.deaths_juvenile_total),
            mean_cumulative_hazard_q16: self.physiology.as_ref().map_or(0, |p| {
                if p.cumulative_hazard_q16.is_empty() {
                    0
                } else {
                    (p.cumulative_hazard_q16
                        .iter()
                        .map(|&value| i128::from(value))
                        .sum::<i128>()
                        / p.cumulative_hazard_q16.len() as i128) as i64
                }
            }),
            max_age_ticks_observed: self.age_ticks.iter().copied().max().unwrap_or(0),
            total_capacity_milli: (0..self.terrain.cell_count())
                .map(|cell| self.effective_capacity_milli(cell))
                .sum(),
            contest_enabled: self.contest.is_some(),
            attacks_total: self.contest.as_ref().map_or(0, |c| c.attacks_total),
            deaths_by_damage_total: self
                .contest
                .as_ref()
                .map_or(0, |c| c.deaths_by_damage_total),
            carcasses: self
                .contest
                .as_ref()
                .map_or(0, |c| c.carcasses.len() as u64),
            total_carcass_energy_milli: self
                .contest
                .as_ref()
                .map_or(0, |c| c.total_carcass_energy_milli() as i64),
            plasticity_enabled: self.learn.is_some(),
            plastic_edges_total: self.learn.as_ref().map_or(0, |l| l.total_plastic_edges()),
            mean_plastic_fraction_milli: match (self.learn.as_ref(), self.schema2.as_ref()) {
                (Some(learn), Some(schema2)) => {
                    learn.mean_plastic_fraction_milli(&schema2.edges_per_organism())
                }
                _ => 0,
            },
            mean_abs_learned_milli: self
                .learn
                .as_ref()
                .map_or(0, |l| l.mean_abs_learned_milli()),
            learned_edges_nonzero: self.learn.as_ref().map_or(0, |l| l.count_nonzero_learned()),
            max_abs_learned_milli: self.learn.as_ref().map_or(0, |l| l.max_abs_learned_milli()),
            plasticity_updates_total: self
                .learn
                .as_ref()
                .map_or(0, |l| l.counters.total_evaluated()),
            plasticity_updates_applied: self
                .learn
                .as_ref()
                .map_or(0, |l| l.counters.updates_applied),
            plasticity_updates_static: self.learn.as_ref().map_or(0, |l| l.counters.updates_static),
            plasticity_updates_refused: self
                .learn
                .as_ref()
                .map_or(0, |l| l.counters.updates_refused),
            plasticity_anomalies_total: self
                .learn
                .as_ref()
                .map_or(0, |l| l.counters.total_anomalies()),
            plasticity_faults_total: self.learn.as_ref().map_or(0, |l| l.counters.total_faults()),
            plasticity_saturations_total: self
                .learn
                .as_ref()
                .map_or(0, |l| l.counters.total_saturations()),
            // Read off the plans rather than the learned state: an edge over
            // the cap has no learned-state row by definition, so counting it
            // there would report zero forever.
            plastic_edges_over_cap: match (self.learn.as_ref(), self.schema2.as_ref()) {
                (Some(_), Some(schema2)) => schema2.plastic_over_cap(),
                _ => 0,
            },
            plasticity_cost_milli: self.learn.as_ref().map_or(0, |l| l.cost_milli as i64),
            worldmod_enabled: self.worldmod.is_some(),
            worldmod_overrides: self.worldmod.as_ref().map_or(0, |s| s.len() as u64),
            worldmod_overrides_by_layer: self
                .worldmod
                .as_ref()
                .map_or([0; LAYER_COUNT as usize], |s| s.layer_counts()),
            worldmod_relocations: self.worldmod.as_ref().map_or(0, |s| s.counters.relocations),
            worldmod_capacity_loss_milli: self.worldmod_capacity_loss_milli() as i64,
            worldmod_cells_trimmed: self
                .worldmod
                .as_ref()
                .map_or(0, |s| s.counters.cells_trimmed),
            worldmod_refusals: self.worldmod.as_ref().map_or(0, |s| s.counters.refusals()),
        }
    }

    /// Phase 2 counters (zeroed default when Phase 2 is disabled).
    pub fn phase2_counters(&self) -> Phase2Counters {
        self.phase2
            .as_ref()
            .map(|p2| p2.counters)
            .unwrap_or_default()
    }

    pub fn phase2_enabled(&self) -> bool {
        self.phase2.is_some()
    }

    /// Ancestry summary for one organism: (parents, depth, child count,
    /// birth tick, genome hash). None when Phase 2 is disabled or the ID is
    /// not alive.
    pub fn ancestry_of(&self, id: u64) -> Option<([u64; 2], u32, u32, u64, u64)> {
        let p2 = self.phase2.as_ref()?;
        let index = self.index_of(id)?;
        Some((
            p2.parents[index],
            p2.depth[index],
            p2.child_count[index],
            p2.birth_tick[index],
            p2.genome_hashes[index],
        ))
    }

    pub(crate) fn phase2_state(&self) -> Option<&Phase2State> {
        self.phase2.as_ref()
    }

    pub(crate) fn climate_state(&self) -> Option<&ClimateWorld> {
        self.climate.as_ref()
    }

    pub(crate) fn contest_state(&self) -> Option<&ContestState> {
        self.contest.as_ref()
    }

    /// Read-only view of Phase 8 physiology state, `None` when disabled.
    pub(crate) fn physiology_state(&self) -> Option<&PhysiologyState> {
        self.physiology.as_ref()
    }

    /// Read-only view of Phase 9 schema-2 state, `None` when disabled.
    pub(crate) fn schema2_state(&self) -> Option<&Schema2State> {
        self.schema2.as_ref()
    }

    pub(crate) fn morphology_state(&self) -> Option<&MorphologyState> {
        self.morphology.as_ref()
    }

    /// Read-only view of Phase 11 learned state, `None` when the plasticity
    /// section is disabled.
    ///
    /// `export_state` reads it *with* `schema2_state`, never alone: the
    /// learned rows are positional and the edge identity that names each slot
    /// lives in the compiled plan, so a saved row without the plan it was
    /// sized against says nothing about which edge learned what.
    pub(crate) fn action_census_state(&self) -> Option<&ActionCensus> {
        self.action_census.as_ref()
    }

    pub(crate) fn learn_state(&self) -> Option<&LearnState> {
        self.learn.as_ref()
    }

    pub fn genome2_enabled(&self) -> bool {
        self.schema2.is_some()
    }

    pub fn morphology_enabled(&self) -> bool {
        self.morphology.is_some()
    }

    /// This organism's energy capacity, milli-EU.
    ///
    /// Without morphology this is the global config value and every caller
    /// behaves exactly as it did, which is what keeps the schema-1 and
    /// schema-2 fixtures intact. With morphology it is the config floor plus
    /// what the body's storage modules confer.
    ///
    /// Storage has to buy *something*. Every module costs mass and upkeep,
    /// and mass costs speed, so a storage module that conferred no capacity
    /// would be strictly deleterious - an authored disadvantage, and the
    /// morphospace would simply exclude a tissue type the registry claims to
    /// offer.
    fn energy_capacity_of(&self, index: usize) -> i64 {
        match self.morphology.as_ref() {
            Some(state) if index < state.derived.len() => state.energy_capacity_milli(index),
            _ => self.config.energy_max_milli,
        }
    }

    /// Per-organism expressed structure and encoded genome size, in entity-ID
    /// order. Empty when schema 2 is disabled.
    ///
    /// The world-level metrics carry a mean, a median, and a distinct count,
    /// which are three summaries of one distribution; C9.1 asks whether
    /// structure spread through the population and C9.8 asks what the tail
    /// costs to store, and neither question can be answered from a summary.
    /// This is observation, never instruction: it hands out counts and
    /// returns nothing to the kernel.
    pub fn structure_census(&self) -> Vec<StructureSample> {
        let Some(state) = self.schema2.as_ref() else {
            return Vec::new();
        };
        state
            .plans
            .iter()
            .zip(state.genomes.iter())
            .map(|(plan, genome)| StructureSample {
                nodes: plan.node_count() as u32,
                edges: plan.edge_count() as u32,
                genome_bytes: genome.encode().len() as u32,
            })
            .collect()
    }

    /// Per-organism body size against reproductive success, in entity-ID
    /// order. Empty when morphology is disabled.
    pub fn morphology_census(&self) -> Vec<MorphologySample> {
        let (Some(state), Some(p2)) = (self.morphology.as_ref(), self.phase2.as_ref()) else {
            return Vec::new();
        };
        state
            .bodies
            .iter()
            .enumerate()
            .map(|(index, body)| MorphologySample {
                modules: body.len() as u32,
                child_count: p2.child_count[index],
                age_ticks: self.age_ticks[index],
                mature: self.age_ticks[index] >= p2.phenotypes[index].maturity_ticks,
            })
            .collect()
    }

    /// Per-organism learned state, in entity-ID order. Empty when the
    /// plasticity section is disabled.
    ///
    /// This is the accessor C11.4 is asserted through. A newborn's entry has
    /// `sum_abs_learned_q16 == 0` on every plastic edge it carries, whatever
    /// its parents had learned, and `plastic_edges > 0` is what stops that
    /// zero from being the trivial zero of an organism with nothing to learn.
    pub fn learned_census(&self) -> Vec<LearnedSample> {
        let Some(learn) = self.learn.as_ref() else {
            return Vec::new();
        };
        (0..learn.len())
            .map(|index| {
                let learned = &learn.learned_q16[index];
                let trace = &learn.trace_q16[index];
                LearnedSample {
                    plastic_edges: learned.len() as u32,
                    sum_abs_learned_q16: learned
                        .iter()
                        .map(|value| i64::from(value.unsigned_abs()))
                        .sum(),
                    max_abs_learned_q16: learned
                        .iter()
                        .map(|value| value.saturating_abs())
                        .max()
                        .unwrap_or(0),
                    sum_abs_trace_q16: trace
                        .iter()
                        .map(|value| i64::from(value.unsigned_abs()))
                        .sum(),
                    faults: learn.faults[index],
                    age_ticks: self.age_ticks[index],
                }
            })
            .collect()
    }

    /// Effective weight of every plastic edge alive, genome weight plus
    /// learned delta after the clamp.
    ///
    /// Materialized rather than summarized because the property C11.5 states
    /// is a bound on **every** edge, and a mean or a max computed inside the
    /// kernel would be the same code under test checking itself.
    pub fn plastic_effective_weights(&self) -> Vec<f32> {
        let (Some(learn), Some(schema2)) = (self.learn.as_ref(), self.schema2.as_ref()) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for index in 0..learn.len().min(schema2.len()) {
            for (slot, edge) in schema2.plans[index].plastic_edges.iter().enumerate() {
                out.push(plasticity::effective_weight(
                    edge.weight,
                    learn.learned_q16[index][slot],
                ));
            }
        }
        out
    }

    /// Per-organism action histograms, in entity-ID order. Empty when the
    /// action census is disabled.
    ///
    /// Modelled on `morphology_census` and `learned_census`, and observation
    /// only on the same terms (ADR-0016): it returns numbers and instructs
    /// nothing. Nothing in the kernel calls it, and a value it returned could
    /// not reach a rule if it did - there is no path from here back into a
    /// tick phase.
    pub fn action_census(&self) -> Vec<ActionSample> {
        let Some(census) = self.action_census.as_ref() else {
            return Vec::new();
        };
        (0..census.len())
            .map(|index| ActionSample {
                id: self.ids[index],
                age_ticks: self.age_ticks[index],
                counts: census.counts[index],
            })
            .collect()
    }

    /// Zero every organism's action histogram: the probe boundary C11.1's
    /// before/after comparison is defined against.
    ///
    /// **This is a state change and it moves the state checksum**, which is
    /// correct: a world whose counters were zeroed at a given tick is not the
    /// world whose were not, and a boundary that left no trace would be one a
    /// replay could not reproduce. It is called by no tick phase and by no
    /// sampling path - the artifact records cumulative rows and differences
    /// them - so it is available to a scripted probe without any measurement
    /// depending on it.
    ///
    /// No-op when the census is disabled, rather than an error: a caller that
    /// asks a world with no instrument to reset it has asked for nothing to
    /// happen, and that is what happens.
    pub fn reset_action_census(&mut self) {
        if let Some(census) = self.action_census.as_mut() {
            census.reset();
        }
    }

    /// Census-wide action counters, or `None` when the census is disabled.
    pub fn action_census_counters(&self) -> Option<crate::actioncensus::ActionCensusCounters> {
        self.action_census.as_ref().map(|census| census.counters)
    }

    /// The object table, when the artifact section is enabled. Read-only.
    pub fn object_table(&self) -> Option<&crate::artifact::ObjectTable> {
        self.objects.as_ref().map(|objects| &objects.table)
    }

    pub(crate) fn object_state(&self) -> Option<&crate::artifact::ObjectState> {
        self.objects.as_ref()
    }

    /// The artifact half's counters; `None` when the section is disabled,
    /// never zeros (D-090's absence-not-zero contract).
    pub fn object_counters(&self) -> Option<crate::artifact::ObjectCounters> {
        self.objects.as_ref().map(|objects| objects.table.counters)
    }

    pub fn object_ledger(&self) -> Option<crate::artifact::ObjectLedger> {
        self.objects.as_ref().map(|objects| objects.table.ledger)
    }

    /// The six object cues (17-22: present, distance, bearing, heft,
    /// hardness, carried load) written for organism `index` in the last
    /// `Sense` phase; `None` without the section or out of range. Read-only,
    /// for tests and diagnostics: the values are what the controller saw.
    pub fn object_perception(&self, index: usize) -> Option<[f32; 6]> {
        self.objects
            .as_ref()
            .and_then(|objects| objects.perception.get(index).copied())
    }

    pub fn artifact_enabled(&self) -> bool {
        self.objects.is_some()
    }

    /// Quintile boundaries of baseline capacity over habitable cells: the
    /// stratifier for C12.2's matched comparison, a pure function of the
    /// terrain.
    pub(crate) fn capacity_band_thresholds(&self) -> [i64; 4] {
        crate::artifact::ObjectState::band_thresholds_of(
            self.terrain
                .capacity_milli
                .iter()
                .copied()
                .filter(|&capacity| capacity > 0)
                .collect(),
        )
    }

    /// Per-organism neutral marker alleles, in entity-ID order. Empty when no
    /// organism carries a marker locus, which includes every world with the
    /// probe section disabled.
    ///
    /// Observation only (ADR-0016). Derived from the genome on demand rather
    /// than cached in a parallel array: a cache would be a fourth thing to
    /// keep in lockstep for a quantity read once per sample, and a stale copy
    /// would make the drift control wrong in exactly the direction that looks
    /// like a result.
    pub fn marker_census(&self) -> Vec<MarkerSample> {
        let Some(schema2) = self.schema2.as_ref() else {
            return Vec::new();
        };
        schema2
            .genomes
            .iter()
            .enumerate()
            .map(|(index, genome)| {
                let alleles = genome.marker_alleles();
                MarkerSample {
                    id: self.ids[index],
                    alleles: alleles.len() as u32,
                    sum_value_milli: alleles
                        .iter()
                        .map(|(_, value, _)| (value.clamp(0.0, 1.0) * 1_000.0) as u32)
                        .sum(),
                    set_alleles: alleles
                        .iter()
                        .filter(|(_, _, flags)| flags & crate::genome2::MARKER_FLAG_NEUTRAL != 0)
                        .count() as u32,
                }
            })
            .collect()
    }

    /// Structural-mutation outcomes broken out by operator and by rejection
    /// reason. `None` when schema 2 is disabled.
    ///
    /// The aggregate applied/rejected pair in [`MetricsSnapshot`] cannot
    /// distinguish "duplication never fired" from "duplication fired and was
    /// rejected every time", and a null result about structural evolution
    /// means opposite things in those two worlds. This doc comment sat on
    /// `morphology_census` for two phases because the two were adjacent and
    /// nothing binds a comment to the item it describes.
    pub fn mutation_counters(&self) -> Option<MutationCounters> {
        self.schema2.as_ref().map(|state| state.counters)
    }

    /// Development outcomes broken out by action and by non-viability class.
    /// `None` when morphology is disabled.
    ///
    /// `morphology_state` is `pub(crate)`, so a campaign that wants to know
    /// which refusal bound - occupied cell, lattice edge, module cap, node
    /// budget - has no other way to ask. The aggregate `nonviable_bodies` in
    /// [`MetricsSnapshot`] collapses four classes into one number, and
    /// "bodies are hitting the module cap" and "bodies are growing into
    /// walls" call for opposite changes to a config.
    pub fn develop_counters(&self) -> Option<crate::develop::DevelopCounters> {
        self.morphology.as_ref().map(|state| state.counters)
    }

    pub fn physiology_enabled(&self) -> bool {
        self.physiology.is_some()
    }

    pub fn contest_enabled(&self) -> bool {
        self.contest.is_some()
    }

    pub fn climate_enabled(&self) -> bool {
        self.climate.is_some()
    }

    /// Read-only biome classification per cell (row-major). Empty when the
    /// climate section is disabled. Observers and analysis only; nothing in
    /// the tick reads a biome label to grant or deny a capability.
    pub fn biome_cells(&self) -> &[crate::climate::Biome] {
        match self.climate.as_ref() {
            Some(climate) => &climate.state.biome,
            None => &[],
        }
    }

    /// Cells per biome in registry order; all zero when disabled.
    pub fn biome_histogram(&self) -> [u32; crate::climate::BIOME_COUNT] {
        self.climate
            .as_ref()
            .map_or([0; crate::climate::BIOME_COUNT], |climate| {
                climate.biome_histogram()
            })
    }

    /// Temperature at one cell for the current tick, milli-degrees. `None`
    /// when the climate section is disabled.
    pub fn temperature_milli(&self, cell: usize) -> Option<i32> {
        let climate = self.climate.as_ref()?;
        Some(crate::climate::ClimateState::temperature_milli(
            &climate.base,
            &self.config.climate,
            cell,
            self.tick,
        ))
    }

    /// Read-only moisture field (row-major, milli-units). Empty when the
    /// climate section is disabled.
    pub fn moisture_cells(&self) -> &[i64] {
        match self.climate.as_ref() {
            Some(climate) => &climate.state.moisture_milli,
            None => &[],
        }
    }

    pub(crate) fn organism_ids(&self) -> &[u64] {
        &self.ids
    }

    /// Read-only view of living organism IDs, ascending. Observers and tests
    /// only; the tick never iterates through this.
    pub fn organism_ids_view(&self) -> &[u64] {
        &self.ids
    }

    /// The cell a fixed-point position falls in. The tick's own `cell_of`,
    /// exposed so a caller outside this module - a terrain-modification
    /// producer, or a test asking which cell an organism stands on - derives
    /// the index the same way the tick does rather than reimplementing the
    /// clamp and getting the rim wrong.
    pub fn cell_index_of(&self, x_fp: i32, y_fp: i32) -> usize {
        self.cell_of(x_fp, y_fp)
    }

    fn index_of(&self, id: u64) -> Option<usize> {
        self.ids.binary_search(&id).ok()
    }

    // --- Save/restore support (see save.rs) ------------------------------

    pub(crate) fn next_entity_id_value(&self) -> u64 {
        self.next_entity_id
    }

    pub(crate) fn positions_x(&self) -> &[i32] {
        &self.x_fp
    }

    pub(crate) fn positions_y(&self) -> &[i32] {
        &self.y_fp
    }

    pub(crate) fn energies(&self) -> &[i64] {
        &self.energy_milli
    }

    pub(crate) fn ages(&self) -> &[u64] {
        &self.age_ticks
    }

    pub(crate) fn cooldowns(&self) -> &[u64] {
        &self.cooldown_ticks
    }

    /// Swap in restored logical state. Only `World::from_state` calls this,
    /// after full validation; invariants are re-checked afterwards.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn replace_logical_state(
        &mut self,
        tick: u64,
        paused: bool,
        extinct: bool,
        next_entity_id: u64,
        ids: Vec<u64>,
        x_fp: Vec<i32>,
        y_fp: Vec<i32>,
        energy_milli: Vec<i64>,
        age_ticks: Vec<u64>,
        cooldown_ticks: Vec<u64>,
        biomass_milli: Vec<i64>,
        ledger: Ledger,
        counters: Counters,
        phase2: Option<Phase2State>,
        climate: Option<ClimateWorld>,
        contest: Option<ContestState>,
        physiology: Option<PhysiologyState>,
        schema2: Option<Schema2State>,
        morphology: Option<MorphologyState>,
        learn: Option<LearnState>,
        worldmod: Option<TerrainModState>,
        action_census: Option<ActionCensus>,
        objects: Option<crate::artifact::ObjectState>,
    ) {
        self.tick = tick;
        self.paused = paused;
        self.extinct = extinct;
        self.next_entity_id = next_entity_id;
        self.ids = ids;
        self.x_fp = x_fp;
        self.y_fp = y_fp;
        self.energy_milli = energy_milli;
        self.age_ticks = age_ticks;
        self.cooldown_ticks = cooldown_ticks;
        self.biomass_milli = biomass_milli;
        self.ledger = ledger;
        self.counters = counters;
        self.phase2 = phase2;
        self.climate = climate;
        self.contest = contest;
        self.physiology = physiology;
        self.schema2 = schema2;
        self.morphology = morphology;
        // Learned state comes from the save, like every other subsystem here.
        //
        // It did not, for one stage: this rebuilt the rows from the restored
        // plans and zeroed them, because the `SaveState` section did not exist
        // yet. That was structurally valid - the rows were the width the plans
        // said, so the desync invariant passed and the first learn phase did
        // not panic - and it was still a restore that silently reset the one
        // piece of world state that cannot be recomputed from the genome. The
        // caller now validates every value against its clamp and every stored
        // edge id against the rebuilt plan's plastic edges *before* this
        // point, so what arrives here is checked, not trusted.
        self.learn = learn;
        // The terrain delta comes from the save for the same reason the
        // learned state does: it cannot be recomputed from anything. A fresh
        // `World::new` built an *empty* set here, and for one stage that empty
        // set is what a restore installed - which dropped every override
        // silently in a control world (scale 1.0 composes to the baseline, so
        // nothing failed) and failed closed only by accident in a treatment
        // world, where biomass grown into a raised capacity exceeded the
        // baseline and tripped `BiomassOutOfBounds`. A fail-open in the arm
        // whose whole point is that it changes nothing measurable is the worst
        // possible place for one. The caller has already checked ordering,
        // uniqueness, and every value's domain, and verifies the composed
        // checksum immediately after this returns.
        self.worldmod = worldmod;
        // From the save, like the learned state and the terrain delta, and
        // for the identical reason: a lifetime's action counts cannot be
        // recomputed from anything the save carries. Rebuilding them empty
        // here would restore a world that quietly disagrees with the one it
        // was saved from about every organism's history - and, unlike the
        // learned state, nothing downstream would ever notice, because
        // nothing in the tick reads them.
        self.action_census = action_census;
        // From the save, like everything above: an object's position,
        // integrity, holder and provenance have no source but the file. The
        // caller checked the table and rebuilt the held lists; the cell index
        // is rebuilt on the next tick's `SpatialIndex` phase.
        self.objects = objects;
        self.events.clear();
        for bucket in &mut self.buckets {
            bucket.clear();
        }
    }

    /// Read-only dynamic food field (row-major cells, milli-biomass).
    pub fn biomass_cells(&self) -> &[i64] {
        &self.biomass_milli
    }

    /// Compact render records for every organism inside the bounds, in
    /// stable entity-ID order. Pure read-only view for observers; deep
    /// details use `organism_detail`. Phase 1 worlds report neutral render
    /// traits and heading zero.
    pub fn render_entities_in(
        &self,
        x0_fp: i32,
        y0_fp: i32,
        x1_fp: i32,
        y1_fp: i32,
        out: &mut Vec<RenderEntity>,
    ) {
        out.clear();
        let p2 = self.phase2.as_ref();
        for index in 0..self.ids.len() {
            let x = self.x_fp[index];
            let y = self.y_fp[index];
            if x < x0_fp || x > x1_fp || y < y0_fp || y > y1_fp {
                continue;
            }
            let energy_frac_q8 =
                ((self.energy_milli[index].clamp(0, self.energy_capacity_of(index)) * 255)
                    / self.energy_capacity_of(index).max(1)) as u8;
            let record = match p2 {
                Some(p2) => {
                    let traits = match self.schema2.as_ref() {
                        Some(state) => resolve_traits(&state.genomes[index].express_traits()),
                        None => *p2.genomes[index].traits(),
                    };
                    let traits = &traits;
                    RenderEntity {
                        id: self.ids[index],
                        x_fp: x,
                        y_fp: y,
                        heading_bam: p2.heading_bam[index],
                        mature: self.age_ticks[index] >= p2.phenotypes[index].maturity_ticks,
                        pigment_hue_q8: (traits[crate::genome::GENE_PIGMENT_HUE] * 255.0) as u8,
                        pigment_pattern_q8: (traits[crate::genome::GENE_PIGMENT_PATTERN] * 255.0)
                            as u8,
                        body_scale_q8: (traits[crate::genome::GENE_BODY_SCALE] * 255.0) as u8,
                        energy_frac_q8,
                    }
                }
                None => RenderEntity {
                    id: self.ids[index],
                    x_fp: x,
                    y_fp: y,
                    heading_bam: 0,
                    mature: self.age_ticks[index] >= self.config.maturity_age_ticks,
                    pigment_hue_q8: 128,
                    pigment_pattern_q8: 128,
                    body_scale_q8: 128,
                    energy_frac_q8,
                },
            };
            out.push(record);
        }
    }

    /// Bounded on-demand detail for one living organism.
    pub fn organism_detail(&self, id: u64) -> Option<OrganismDetail> {
        let index = self.index_of(id)?;
        let phase2 = self.phase2.as_ref().map(|p2| Phase2Detail {
            heading_bam: p2.heading_bam[index],
            speed_milli: p2.speed_milli[index],
            trait_genes: match self.schema2.as_ref() {
                Some(state) => resolve_traits(&state.genomes[index].express_traits()),
                None => *p2.genomes[index].traits(),
            },
            phenotype: p2.phenotypes[index],
            parents: p2.parents[index],
            ancestry_depth: p2.depth[index],
            child_count: p2.child_count[index],
            birth_tick: p2.birth_tick[index],
            genome_hash: p2.genome_hashes[index],
        });
        Some(OrganismDetail {
            id,
            x_fp: self.x_fp[index],
            y_fp: self.y_fp[index],
            energy_milli: self.energy_milli[index],
            age_ticks: self.age_ticks[index],
            cooldown_ticks: self.cooldown_ticks[index],
            phase2,
        })
    }

    // --- Tick ------------------------------------------------------------

    pub fn step(&mut self) {
        self.step_with_observer(&mut NoopObserver);
    }

    /// Advance one tick through the canonical phase order. A paused world
    /// advances zero ticks and produces zero state change.
    pub fn step_with_observer(&mut self, observer: &mut impl TickObserver) {
        if self.paused {
            return;
        }
        let next_tick = self.tick + 1;
        self.events.clear();

        // Canonical phase 1: apply validated queued commands. Phase 1 defined
        // no in-tick commands (pause/resume act between ticks) and the phase
        // stayed explicit so the canonical order was stable; Phase 12 is the
        // first thing to put work in it. The relocating resource patch is a
        // command the world issues to itself on a schedule, and it lands here
        // so the new capacity is in place before `Environment` grows into it
        // on the same tick. Empty and free when the section is disabled.
        observer.phase_started(TickPhase::Commands);
        self.relocate_patch(next_tick);
        observer.phase_finished(TickPhase::Commands);

        observer.phase_started(TickPhase::Environment);
        self.step_climate(next_tick);
        self.grow_food();
        // Phase 12 artifact half: material yield regenerates on its cadence.
        // Empty and free when the section is disabled.
        self.regenerate_yield(next_tick);
        observer.phase_finished(TickPhase::Environment);

        observer.phase_started(TickPhase::SpatialIndex);
        self.build_spatial_index();
        self.rebuild_object_index();
        observer.phase_finished(TickPhase::SpatialIndex);

        observer.phase_started(TickPhase::Sense);
        if self.phase2.is_some() {
            self.sense_phase2();
        } else {
            self.sense(next_tick);
        }
        // Object cues, read from the index built this tick. Empty when the
        // section is disabled.
        self.sense_objects();
        observer.phase_finished(TickPhase::Sense);

        observer.phase_started(TickPhase::Controllers);
        if self.phase2.is_some() {
            self.controllers_phase2(next_tick);
        }
        observer.phase_finished(TickPhase::Controllers);

        observer.phase_started(TickPhase::Apply);
        if self.phase2.is_some() {
            self.apply_phase2(next_tick);
        } else {
            self.apply_intents(next_tick);
        }
        self.contest_phase(next_tick);
        // Phase 12 artifact half: the five actions and object consumption,
        // after contest so a strike lands on the same positions an attack
        // does. Empty when the section is disabled.
        self.artifact_phase(next_tick);
        observer.phase_finished(TickPhase::Apply);

        // The boundary is emitted whether or not the section is enabled, so
        // per-phase timing is comparable between conditions A and B of this
        // phase's design. A phase that appeared only when enabled would make
        // the two runs' benchmark records different shapes.
        observer.phase_started(TickPhase::Learn);
        self.learn_phase(next_tick);
        observer.phase_finished(TickPhase::Learn);

        observer.phase_started(TickPhase::Lifecycle);
        self.lifecycle(next_tick);
        observer.phase_finished(TickPhase::Lifecycle);

        observer.phase_started(TickPhase::Finalize);
        self.tick = next_tick;
        observer.phase_finished(TickPhase::Finalize);
    }

    /// Carrying capacity for one cell after biome scaling and after any
    /// stored capacity override.
    ///
    /// Without the climate section the baseline is the terrain's own
    /// elevation-derived capacity, and without the worldmod section the
    /// override arm does not exist, so the Phase 1/2 arithmetic is unchanged
    /// byte for byte. **The `None` arm below is the whole of C12.8 for this
    /// accessor**: it returns the identical expression a pre-Phase-12 build
    /// evaluated, so a disabled world is not merely numerically equal, it
    /// runs the same code.
    pub fn effective_capacity_milli(&self, cell: usize) -> i64 {
        let baseline = match self.climate.as_ref() {
            Some(climate) => climate.capacity_milli(&self.terrain, &self.config.climate, cell),
            None => self.terrain.capacity_milli[cell],
        };
        match self.worldmod.as_ref() {
            Some(state) => match state.get(LAYER_CAPACITY_SCALE, cell as u32) {
                Some(scale) => scale_capacity(baseline, scale),
                None => baseline,
            },
            None => baseline,
        }
    }

    /// Whether a cell may be moved into, after any stored traversability
    /// override.
    ///
    /// The override is bidirectional: it can block a land cell and it can
    /// permit a water cell. Absent - which is every cell in a world without
    /// the section, and every unmodified cell in a world with it - this is
    /// `Terrain::land`, the expression every movement, birth-placement, and
    /// invariant site read directly before Phase 12.
    ///
    /// **Layer 0 has no producer yet.** Blocking objects and digging are
    /// artifact-half actions. The consumer is written now, and every read
    /// site routed through it now, because routing a read site is where the
    /// mistakes are: a site left on `Terrain::land` would be a cell an
    /// organism could walk into after the world said it could not, and it
    /// would be invisible until the artifact half started writing the layer.
    pub fn effective_traversable(&self, cell: usize) -> bool {
        match self.worldmod.as_ref() {
            Some(state) => match state.get(LAYER_TRAVERSABLE, cell as u32) {
                Some(value) => value != 0,
                None => self.terrain.land[cell],
            },
            None => self.terrain.land[cell],
        }
    }

    /// Read-only view of the modification set. `None` when the section is
    /// disabled.
    pub fn worldmod_state(&self) -> Option<&TerrainModState> {
        self.worldmod.as_ref()
    }

    /// Biomass removed because a modification lowered a cell's capacity below
    /// its standing biomass. Zero without the section.
    ///
    /// **This is the number the zero-magnitude control is defined by.** A
    /// control arm running the identical relocation schedule at scale 1.0
    /// keeps this at exactly zero while the treatment's climbs, which is what
    /// makes the two arms comparable on standing biomass at all.
    pub fn worldmod_capacity_loss_milli(&self) -> i128 {
        self.worldmod
            .as_ref()
            .map_or(0, |state| state.capacity_loss_milli)
    }

    /// The composed terrain checksum: baseline plus every override.
    ///
    /// A **full recompute**, on demand, over every cell. No tick calls it;
    /// see `TerrainModState::composed_checksum` for why an incremental
    /// version of an FNV-1a chain does not exist and what is owed to the
    /// specification because of that. Equals `terrain().terrain_checksum`
    /// exactly when the section is disabled or the set is empty, which is
    /// what the format 3 to format 4 migration writes.
    pub fn composed_terrain_checksum(&self) -> u64 {
        match self.worldmod.as_ref() {
            Some(state) => state.composed_checksum(&self.terrain),
            None => self.terrain.terrain_checksum,
        }
    }

    /// Apply one terrain modification, enforcing every policy the layer has.
    ///
    /// **The single entry point for terrain modification**, used by the
    /// relocating schedule below and, when the artifact half lands, by
    /// organism actions. Public so the safety policies are reachable from a
    /// test rather than only from a producer that does not exist yet: a
    /// refusal path nothing can call is a refusal path nothing has checked.
    ///
    /// `value` of `None` clears the override and returns the cell to
    /// baseline. Callers must invoke this in ascending `(layer_id,
    /// cell_index)` order within a tick, which is what the specification
    /// requires of the per-tick modification buffer and what makes two
    /// organisms editing the same cell compose in a fixed order regardless of
    /// which was visited first.
    ///
    /// The two policies that live here rather than in `TerrainModState`,
    /// because both need the *world* and not just the set:
    ///
    /// 1. **A cell an organism is standing on may not become
    ///    non-traversable** (`ModOutcome::RefusedOccupied` states the policy
    ///    and the two resolutions it was chosen over), in either
    ///    direction: blocking a land cell and un-permitting a water cell
    ///    strand an organism identically.
    /// 2. **Lowering a capacity below the standing biomass trims the excess
    ///    and ledgers it.** Copied in shape from `ClimateWorld::step`, which
    ///    has the same problem for the same reason: `check_invariants`
    ///    refuses biomass above capacity with no tolerance, so the excess has
    ///    to go somewhere, and the only honest somewhere is a named sink
    ///    inside the conservation identity. Raising a capacity needs nothing:
    ///    the new headroom fills through `grow_food`'s ordinary
    ///    `grown_milli` term.
    pub fn apply_terrain_modification(
        &mut self,
        layer: u8,
        cell: usize,
        value: Option<i64>,
    ) -> ModOutcome {
        let Some(state) = self.worldmod.as_mut() else {
            return ModOutcome::RefusedInvalid;
        };
        if layer >= LAYER_COUNT || cell >= self.terrain.capacity_milli.len() {
            state.counters.refused_invalid += 1;
            return ModOutcome::RefusedInvalid;
        }
        if let Some(value) = value
            && !value_in_domain(layer, value)
        {
            state.counters.refused_invalid += 1;
            return ModOutcome::RefusedInvalid;
        }
        if layer == LAYER_TRAVERSABLE {
            // Would the composed view of this cell be non-traversable after
            // the write? Computed from the write rather than from the state,
            // because a clear reverts to the baseline and a set does not.
            let after = match value {
                Some(value) => value != 0,
                None => self.terrain.land[cell],
            };
            if !after && self.cell_is_occupied(cell) {
                if let Some(state) = self.worldmod.as_mut() {
                    state.counters.refused_occupied += 1;
                }
                return ModOutcome::RefusedOccupied;
            }
        }
        let cap = match layer {
            LAYER_TRAVERSABLE => self.config.worldmod.max_traversable_overrides,
            LAYER_CAPACITY_SCALE => self.config.worldmod.max_capacity_overrides,
            _ => self.config.worldmod.max_material_overrides,
        };
        let state = self.worldmod.as_mut().expect("checked above");
        let outcome = match value {
            Some(value) => state.set(layer, cell as u32, value, cap),
            None => state.clear(layer, cell as u32),
        };
        if layer == LAYER_CAPACITY_SCALE
            && matches!(
                outcome,
                ModOutcome::Inserted | ModOutcome::Replaced | ModOutcome::Cleared
            )
        {
            self.trim_biomass_to_capacity(cell);
        }
        outcome
    }

    /// Remove biomass a capacity change left stranded above the new ceiling,
    /// into the modification set's own loss sink.
    fn trim_biomass_to_capacity(&mut self, cell: usize) {
        let capacity = self.effective_capacity_milli(cell);
        let biomass = self.biomass_milli[cell];
        if biomass <= capacity {
            return;
        }
        let excess = biomass - capacity;
        self.biomass_milli[cell] = capacity;
        if let Some(state) = self.worldmod.as_mut() {
            state.capacity_loss_milli += i128::from(excess);
            state.counters.cells_trimmed += 1;
        }
    }

    /// Whether any living organism currently occupies `cell`.
    ///
    /// A linear scan over positions. That is the right cost here and would be
    /// the wrong cost in a loop: the only caller is the traversability write
    /// path, which has no producer yet, and the artifact half's bulk producer
    /// must build an occupancy map once per batch rather than call this per
    /// cell. Said here because the linear scan is exactly the kind of thing a
    /// later caller copies without reading.
    fn cell_is_occupied(&self, cell: usize) -> bool {
        (0..self.ids.len()).any(|index| self.cell_of(self.x_fp[index], self.y_fp[index]) == cell)
    }

    /// Where the resource patch sits during `epoch`, as a baseline cell
    /// index. `None` for epoch 0, which is the interval before the first
    /// relocation and has no patch.
    ///
    /// A pure function of `(world_seed, epoch)`, and drawn over **baseline**
    /// habitable cells rather than composed ones. That is deliberate and it
    /// is the difference between an environment and a feedback loop: if the
    /// draw ranged over composed capacity, where the patch goes next would
    /// depend on where it has been, and on whatever the organisms have done
    /// to the terrain - which would make the schedule an authored response to
    /// the population rather than a property of the world.
    fn patch_centre_cell(&self, epoch: u64) -> Option<usize> {
        if epoch == 0 {
            return None;
        }
        let habitable = u64::from(self.terrain.habitable_cells);
        if habitable == 0 {
            return None;
        }
        let draw = named_random(
            self.config.world_seed,
            epoch * self.config.worldmod.relocate_interval_ticks,
            RngSystem::TerrainMod,
            epoch,
            0,
        );
        // The nth habitable cell in ascending index order. A scan rather
        // than a precomputed table: it runs once per relocation, not once
        // per tick, and a table would be derived state to keep in lockstep
        // with terrain for no gain.
        let mut remaining = draw % habitable;
        for cell in 0..self.terrain.capacity_milli.len() {
            if self.terrain.capacity_milli[cell] > 0 {
                if remaining == 0 {
                    return Some(cell);
                }
                remaining -= 1;
            }
        }
        None
    }

    /// The habitable cells covered by a patch centred on `centre`, ascending.
    ///
    /// Water and zero-capacity cells are excluded rather than written with an
    /// inert override: an override on a zero-capacity cell composes to zero
    /// whatever the scale, so it would cost an entry, a checksum, and a
    /// snapshot byte to say nothing.
    fn patch_cells(&self, centre: usize) -> Vec<u32> {
        let radius = self.config.worldmod.patch_radius_cells as i64;
        let cells_x = i64::from(self.terrain.cells_x);
        let cells_y = i64::from(self.terrain.cells_y);
        let centre_x = (centre as i64) % cells_x;
        let centre_y = (centre as i64) / cells_x;
        let mut cells = Vec::new();
        for offset_y in -radius..=radius {
            let cell_y = centre_y + offset_y;
            if cell_y < 0 || cell_y >= cells_y {
                continue;
            }
            for offset_x in -radius..=radius {
                let cell_x = centre_x + offset_x;
                if cell_x < 0 || cell_x >= cells_x {
                    continue;
                }
                let cell = (cell_y * cells_x + cell_x) as usize;
                if self.terrain.capacity_milli[cell] > 0 {
                    cells.push(cell as u32);
                }
            }
        }
        cells
    }

    /// Move the resource patch, if this tick is a relocation tick.
    ///
    /// Runs in the `Commands` phase, which had no work in it before: a
    /// declarative environmental schedule is a command the world issues to
    /// itself, and putting it there means it lands before `Environment` grows
    /// food into the new capacity in the same tick.
    ///
    /// The write list is the **union** of the leaving patch's cells and the
    /// arriving patch's, applied in ascending cell order with each cell's
    /// final value decided before any write. Applying a clear pass and then a
    /// set pass would leave the overlap's outcome dependent on pass order,
    /// which is exactly the ordering ambiguity the specification's ascending
    /// `(layer_id, cell_index)` rule exists to remove.
    fn relocate_patch(&mut self, next_tick: u64) {
        let worldmod = self.config.worldmod;
        if !worldmod.enabled || !worldmod.patch_enabled {
            return;
        }
        if !next_tick.is_multiple_of(worldmod.relocate_interval_ticks) {
            return;
        }
        let epoch = next_tick / worldmod.relocate_interval_ticks;
        let leaving = self
            .patch_centre_cell(epoch - 1)
            .map(|centre| self.patch_cells(centre))
            .unwrap_or_default();
        let arriving = self
            .patch_centre_cell(epoch)
            .map(|centre| self.patch_cells(centre))
            .unwrap_or_default();
        let scale = i64::from(worldmod.patch_capacity_scale_q16);

        // Merge two ascending, duplicate-free lists into one ascending list
        // of (cell, final value). A cell in both lists takes the arriving
        // value; a cell only in the leaving list returns to baseline.
        let mut writes: Vec<(u32, Option<i64>)> =
            Vec::with_capacity(leaving.len() + arriving.len());
        let (mut left, mut right) = (0_usize, 0_usize);
        while left < leaving.len() || right < arriving.len() {
            let take_left = right >= arriving.len()
                || (left < leaving.len() && leaving[left] <= arriving[right]);
            if take_left {
                let cell = leaving[left];
                if right < arriving.len() && arriving[right] == cell {
                    writes.push((cell, Some(scale)));
                    right += 1;
                } else {
                    writes.push((cell, None));
                }
                left += 1;
            } else {
                writes.push((arriving[right], Some(scale)));
                right += 1;
            }
        }
        for (cell, value) in writes {
            self.apply_terrain_modification(LAYER_CAPACITY_SCALE, cell as usize, value);
        }
        if let Some(state) = self.worldmod.as_mut() {
            state.counters.relocations += 1;
        }
    }

    /// Advance moisture and, on the configured cadence, reclassify biomes.
    /// Empty when the climate section is disabled, so the environment phase
    /// costs exactly what it did before Phase 6.
    fn step_climate(&mut self, next_tick: u64) {
        if let Some(mut climate) = self.climate.take() {
            climate.step(
                &self.terrain,
                &self.config.climate,
                next_tick,
                &mut self.biomass_milli,
            );
            self.climate = Some(climate);
        }
    }

    /// Logistic regrowth with a one-milli seeding floor so grazed cells
    /// recover (policy v1). Exact integer arithmetic; ledger-recorded.
    fn grow_food(&mut self) {
        let rate = i128::from(self.config.growth_rate_q16_per_s);
        let dt = i128::from(self.config.dt_ms);
        for index in 0..self.biomass_milli.len() {
            let capacity = self.effective_capacity_milli(index);
            if capacity <= 0 {
                continue;
            }
            let biomass = self.biomass_milli[index];
            if biomass >= capacity {
                continue;
            }
            let logistic =
                i128::from(biomass) * i128::from(capacity - biomass) / i128::from(capacity);
            let growth = (logistic * rate * dt / (i128::from(Q16) * 1000)) as i64;
            let growth = growth.max(1).min(capacity - biomass);
            self.biomass_milli[index] = biomass + growth;
            self.ledger.grown_milli += i128::from(growth);
        }
    }

    fn build_spatial_index(&mut self) {
        for bucket in &mut self.buckets {
            bucket.clear();
        }
        for index in 0..self.ids.len() {
            let bucket = self.bucket_of(self.x_fp[index], self.y_fp[index]);
            self.buckets[bucket].push(index as u32);
        }
    }

    fn bucket_of(&self, x_fp: i32, y_fp: i32) -> usize {
        let bucket_x = (x_fp / self.bucket_size_fp).min(self.buckets_x as i32 - 1) as usize;
        let bucket_y = (y_fp / self.bucket_size_fp).min(self.buckets_y as i32 - 1) as usize;
        bucket_y * self.buckets_x as usize + bucket_x
    }

    fn cell_of(&self, x_fp: i32, y_fp: i32) -> usize {
        let cell_fp = self.config.cell_size_fp();
        let cell_x = (x_fp / cell_fp).min(self.terrain.cells_x as i32 - 1) as usize;
        let cell_y = (y_fp / cell_fp).min(self.terrain.cells_y as i32 - 1) as usize;
        cell_y * self.terrain.cells_x as usize + cell_x
    }

    fn sense(&mut self, next_tick: u64) {
        let population = self.ids.len();
        self.intent_direction.clear();
        self.intent_direction.resize(population, 0);
        self.intent_crowded.clear();
        self.intent_crowded.resize(population, false);
        self.intent_reproduce.clear();
        self.intent_reproduce.resize(population, false);

        let cells_x = self.terrain.cells_x as i32;
        let cells_y = self.terrain.cells_y as i32;
        let cell_fp = self.config.cell_size_fp();

        for index in 0..population {
            // Food gradient over the 3x3 cell neighborhood; strict
            // improvement over the current cell wins, ties resolved by the
            // fixed DIRECTIONS scan order.
            let cell_x = self.x_fp[index] / cell_fp;
            let cell_y = self.y_fp[index] / cell_fp;
            let own_cell = self.cell_of(self.x_fp[index], self.y_fp[index]);
            let mut best_biomass = self.biomass_milli[own_cell];
            let mut best_direction = 0_u8;
            for (direction_index, (dx, dy)) in DIRECTIONS.iter().enumerate().skip(1) {
                let neighbor_x = cell_x + i32::from(*dx);
                let neighbor_y = cell_y + i32::from(*dy);
                if neighbor_x < 0
                    || neighbor_x >= cells_x
                    || neighbor_y < 0
                    || neighbor_y >= cells_y
                {
                    continue;
                }
                let neighbor = (neighbor_y as usize) * cells_x as usize + neighbor_x as usize;
                // Composed, not baseline: the gradient scan must not steer an
                // organism toward a cell the movement pass will then refuse
                // to let it enter, which is what a raw `terrain.land` read
                // here would do once layer 0 has a producer.
                if !self.effective_traversable(neighbor) {
                    continue;
                }
                if self.biomass_milli[neighbor] > best_biomass {
                    best_biomass = self.biomass_milli[neighbor];
                    best_direction = direction_index as u8;
                }
            }

            // Deterministic jitter draw (policy v1).
            let draw = named_random(
                self.config.world_seed,
                next_tick,
                RngSystem::Movement,
                self.ids[index],
                0,
            );
            if draw.is_multiple_of(JITTER_MODULUS) {
                best_direction = ((draw >> 3) % DIRECTIONS.len() as u64) as u8;
            }
            self.intent_direction[index] = best_direction;

            // Crowding via spatial buckets.
            self.intent_crowded[index] =
                self.neighbor_count(index) >= self.config.crowding_threshold;

            // Reproduction readiness (validated again in apply).
            self.intent_reproduce[index] = self.config.reproduction_enabled
                && self.age_ticks[index] >= self.config.maturity_age_ticks
                && self.cooldown_ticks[index] == 0
                && self.energy_milli[index] >= self.config.repro_threshold_milli;
        }
    }

    /// Count other organisms within the crowding radius using the bucket
    /// index. Bucket size equals the radius, so a 3x3 bucket ring covers it.
    fn neighbor_count(&self, index: usize) -> u32 {
        let x = i64::from(self.x_fp[index]);
        let y = i64::from(self.y_fp[index]);
        let radius_squared = self.crowding_radius_fp * self.crowding_radius_fp;
        let bucket_x = (self.x_fp[index] / self.bucket_size_fp).min(self.buckets_x as i32 - 1);
        let bucket_y = (self.y_fp[index] / self.bucket_size_fp).min(self.buckets_y as i32 - 1);
        let mut count = 0_u32;
        for neighbor_y in (bucket_y - 1).max(0)..=(bucket_y + 1).min(self.buckets_y as i32 - 1) {
            for neighbor_x in (bucket_x - 1).max(0)..=(bucket_x + 1).min(self.buckets_x as i32 - 1)
            {
                let bucket = (neighbor_y as usize) * self.buckets_x as usize + neighbor_x as usize;
                for &candidate in &self.buckets[bucket] {
                    let candidate = candidate as usize;
                    if candidate == index {
                        continue;
                    }
                    let dx = i64::from(self.x_fp[candidate]) - x;
                    let dy = i64::from(self.y_fp[candidate]) - y;
                    if dx * dx + dy * dy <= radius_squared {
                        count += 1;
                    }
                }
            }
        }
        count
    }

    fn apply_intents(&mut self, next_tick: u64) {
        let population = self.ids.len();
        let extent_x = self.config.world_extent_x_fp();
        let extent_y = self.config.world_extent_y_fp();

        // Movement pass (stable ID order).
        let mut moved = vec![false; population];
        for index in 0..population {
            let (dx, dy) = DIRECTIONS[self.intent_direction[index] as usize];
            if dx == 0 && dy == 0 {
                continue;
            }
            let step = i64::from(self.step_fp);
            let (step_x, step_y) = if dx != 0 && dy != 0 {
                (
                    i64::from(dx) * step * DIAGONAL_NUMERATOR / DIAGONAL_DENOMINATOR,
                    i64::from(dy) * step * DIAGONAL_NUMERATOR / DIAGONAL_DENOMINATOR,
                )
            } else {
                (i64::from(dx) * step, i64::from(dy) * step)
            };
            let new_x =
                (i64::from(self.x_fp[index]) + step_x).clamp(0, i64::from(extent_x) - 1) as i32;
            let new_y =
                (i64::from(self.y_fp[index]) + step_y).clamp(0, i64::from(extent_y) - 1) as i32;
            // A move into water is rejected: the organism stays and pays no
            // movement cost (policy v1). Phase 12 makes "water" the composed
            // view rather than the baseline one, so a blocked land cell
            // refuses the same way and a permitted water cell admits.
            if self.effective_traversable(self.cell_of(new_x, new_y)) {
                self.x_fp[index] = new_x;
                self.y_fp[index] = new_y;
                moved[index] = true;
            }
        }

        // Cost pass: basal + movement + crowding, floored at zero energy so
        // the ledger records exactly what was spent.
        for (index, &did_move) in moved.iter().enumerate() {
            let mut cost = self.basal_cost_tick;
            if did_move {
                cost += self.move_cost_tick;
            }
            if self.intent_crowded[index] {
                cost += self.crowding_cost_tick;
            }
            let paid = cost.min(self.energy_milli[index]);
            self.energy_milli[index] -= paid;
            self.ledger.spent_milli += i128::from(paid);
        }

        // Feeding pass: shared-cell contention resolves in stable ID order.
        let assimilation = i64::from(self.config.assimilation_q16);
        for index in 0..population {
            if self.energy_milli[index] <= 0 {
                continue;
            }
            let cell = self.cell_of(self.x_fp[index], self.y_fp[index]);
            let available = self.biomass_milli[cell];
            if available <= 0 {
                continue;
            }
            let remaining_capacity = self.energy_capacity_of(index) - self.energy_milli[index];
            if remaining_capacity <= 0 {
                continue;
            }
            let intake_cap = remaining_capacity * Q16 / assimilation;
            let intake = available.min(self.intake_tick).min(intake_cap);
            if intake <= 0 {
                continue;
            }
            let gain = intake * assimilation / Q16;
            self.biomass_milli[cell] -= intake;
            self.energy_milli[index] += gain;
            self.ledger.consumed_biomass_milli += i128::from(intake);
            self.ledger.assimilated_milli += i128::from(gain);
        }

        // Reproduction pass: energy is debited only after a valid placement
        // and capacity check, so a failed birth costs nothing.
        self.pending_births.clear();
        if self.config.reproduction_enabled {
            let total_cost = self.config.offspring_energy_milli + self.config.repro_overhead_milli;
            for index in 0..population {
                if !self.intent_reproduce[index]
                    || self.energy_milli[index] < self.config.repro_threshold_milli
                {
                    continue;
                }
                let Some((birth_x, birth_y)) = self.find_birth_position(
                    next_tick,
                    self.ids[index],
                    self.x_fp[index],
                    self.y_fp[index],
                ) else {
                    continue;
                };
                if population + self.pending_births.len() >= self.config.max_entities as usize {
                    self.counters.capacity_rejections_total += 1;
                    // The cooldown also applies to a rejected attempt so a
                    // capped world does not retry every tick (policy v1).
                    self.cooldown_ticks[index] = self.config.repro_cooldown_ticks;
                    let parent_id = self.ids[index];
                    self.push_event(next_tick, EventKind::CapacityRejected { parent_id });
                    continue;
                }
                self.energy_milli[index] -= total_cost;
                self.ledger.spent_milli += i128::from(self.config.repro_overhead_milli);
                self.cooldown_ticks[index] = self.config.repro_cooldown_ticks;
                self.pending_births
                    .push((self.ids[index], birth_x, birth_y));
            }
        }

        // Aging pass.
        for index in 0..population {
            self.age_ticks[index] += 1;
            self.cooldown_ticks[index] = self.cooldown_ticks[index].saturating_sub(1);
        }
    }

    // --- Phase 2 tick phases (phase2-behavior-v1) -------------------------

    /// Gather bounded normalized sensors for every organism in stable ID
    /// order. Unavailable channels use documented neutral values.
    fn sense_phase2(&mut self) {
        let mut p2 = self.phase2.take().expect("phase2 state present");
        let population = self.ids.len();
        p2.inputs.clear();
        p2.inputs.resize(population, [0.0; CONTROLLER_INPUTS]);
        self.intent_crowded.clear();
        self.intent_crowded.resize(population, false);

        let cells_x = self.terrain.cells_x as i32;
        let cells_y = self.terrain.cells_y as i32;
        let cell_fp = self.config.cell_size_fp();
        // Snapshot the contest fields the reserved channels read. Taken once
        // and read-only for the whole phase, so perception cannot observe a
        // partially updated tick.
        let contest_view: Option<(Vec<i64>, Vec<i64>, Vec<i64>)> =
            self.contest.as_ref().map(|contest| {
                let max: Vec<i64> = p2
                    .phenotypes
                    .iter()
                    .map(|phenotype| {
                        crate::contest::ContestState::health_max_milli(
                            &self.config.contest,
                            phenotype.body_scale_milli,
                        )
                    })
                    .collect();
                (
                    contest.health_milli.clone(),
                    contest.recent_damage_milli.clone(),
                    max,
                )
            });
        let threat_range_fp = i64::from(self.config.contest.attack_range_m.max(1) * 4)
            * i64::from(crate::FP_PER_METER);

        for index in 0..population {
            let inputs = &mut p2.inputs[index];
            let phenotype = &p2.phenotypes[index];

            // 1: energy fraction; 2: health neutral; 3: age fraction.
            inputs[0] =
                self.energy_milli[index] as f32 / self.energy_capacity_of(index).max(1) as f32;
            // Reserved channel 2: health. Neutral 1.0 without the contest
            // section, which is what keeps the Phase 2 fixture exact.
            inputs[1] = match contest_view.as_ref() {
                Some((health, _, max)) => {
                    (health[index] as f32 / max[index].max(1) as f32).clamp(0.0, 1.0)
                }
                None => 1.0,
            };
            inputs[2] =
                (self.age_ticks[index] as f32 / self.config.max_age_ticks as f32).clamp(0.0, 1.0);

            // 4/5: food gradient from the 3x3 cell scan.
            let cell_x = self.x_fp[index] / cell_fp;
            let cell_y = self.y_fp[index] / cell_fp;
            let own_cell = self.cell_of(self.x_fp[index], self.y_fp[index]);
            let mut best_biomass = self.biomass_milli[own_cell];
            let mut best_dx = 0_i32;
            let mut best_dy = 0_i32;
            for (dx, dy) in DIRECTIONS.iter().skip(1) {
                let neighbor_x = cell_x + i32::from(*dx);
                let neighbor_y = cell_y + i32::from(*dy);
                if neighbor_x < 0
                    || neighbor_x >= cells_x
                    || neighbor_y < 0
                    || neighbor_y >= cells_y
                {
                    continue;
                }
                let neighbor = (neighbor_y as usize) * cells_x as usize + neighbor_x as usize;
                // Composed, for the reason the schema-1 scan above is.
                if !self.effective_traversable(neighbor) {
                    continue;
                }
                if self.biomass_milli[neighbor] > best_biomass {
                    best_biomass = self.biomass_milli[neighbor];
                    best_dx = i32::from(*dx);
                    best_dy = i32::from(*dy);
                }
            }
            let diagonal = best_dx != 0 && best_dy != 0;
            let component = if diagonal { 0.70710677_f32 } else { 1.0 };
            inputs[3] = best_dx as f32 * component;
            inputs[4] = best_dy as f32 * component;

            // 6: terrain suitability (own-cell capacity fraction).
            //
            // **Left on the raw baseline capacity, deliberately, and this is
            // a pre-existing inconsistency rather than a Phase 12 choice.**
            // This sensor has read `terrain.capacity_milli` since Phase 2, so
            // it has ignored the climate section's biome scaling since Phase
            // 6: in a climate world an organism already senses the elevation
            // capacity of its cell, not the capacity that actually feeds it.
            // Routing it through `effective_capacity_milli` would fix that
            // and would change the input vector of **every climate-enabled
            // world from Phase 6 onward**, which is a behavior change to a
            // perception channel - a policy-version matter with its own
            // control, not a drive-by inside a phase about terrain storage.
            // Recorded here and reported; not changed.
            inputs[5] = (self.terrain.capacity_milli[own_cell] as f32
                / self.config.cell_capacity_milli as f32)
                .clamp(0.0, 1.0);

            // 7/8: nearest organism proximity and relative heading proxy.
            let range_fp = phenotype.sensor_range_milli * i64::from(crate::FP_PER_METER) / 1000;
            if let Some((distance_squared, nearest)) = self.nearest_within(index, range_fp) {
                let range = range_fp as f32;
                let distance = (distance_squared as f32).sqrt();
                let sensitivity = phenotype.sensor_sensitivity_milli as f32 / 1000.0;
                inputs[6] = ((1.0 - distance / range) * sensitivity).clamp(0.0, 1.0);
                let heading = p2.heading_bam[index];
                let heading_x = i64::from(cos_bam_q15(heading));
                let heading_y = i64::from(sin_bam_q15(heading));
                let delta_x = i64::from(self.x_fp[nearest]) - i64::from(self.x_fp[index]);
                let delta_y = i64::from(self.y_fp[nearest]) - i64::from(self.y_fp[index]);
                let cross = heading_x * delta_y - heading_y * delta_x;
                let norm = (delta_x.abs() + delta_y.abs()).max(1);
                inputs[7] = (cross as f32 / (32768.0 * norm as f32)).clamp(-1.0, 1.0);
            }

            // 9: crowding (also drives the crowding cost, as in Phase 1).
            let neighbor_count = self.neighbor_count(index);
            self.intent_crowded[index] = neighbor_count >= self.config.crowding_threshold;
            inputs[8] =
                (neighbor_count as f32 / (2.0 * self.config.crowding_threshold as f32)).min(1.0);

            // 10-12: threat, temperature comfort, moisture comfort are not
            // simulated in Phase 2; documented neutral zeros.

            // 13: speed fraction; 14: last turn rate.
            // Reserved channel 10: local threat. Nearest conspecific scaled
            // by its relative body size, a perceptible phenotype cue. There
            // is deliberately no genotype-distance channel: ADR-0022 A3
            // forbids direct access to genetic distance, pedigree, or
            // observer labels, so kin recognition must be solvable from what
            // an organism can actually see or not at all.
            if contest_view.is_some()
                && let Some((distance, other)) = self.nearest_within(index, threat_range_fp)
            {
                let their_scale = p2.phenotypes[other].body_scale_milli as f32;
                let own_scale = phenotype.body_scale_milli.max(1) as f32;
                let closeness =
                    1.0 - (distance as f32 / threat_range_fp.max(1) as f32).clamp(0.0, 1.0);
                inputs[9] = (closeness * (their_scale / own_scale)).clamp(0.0, 1.0);
            }
            // Reserved channel 16: recent damage fraction.
            if let Some((_, recent, max)) = contest_view.as_ref() {
                inputs[15] = (recent[index] as f32 / max[index].max(1) as f32).clamp(0.0, 1.0);
            }
            inputs[12] =
                (p2.speed_milli[index] as f32 / phenotype.max_speed_milli as f32).clamp(0.0, 1.0);
            inputs[13] = p2.last_turn[index];

            // 15: reproductive readiness.
            let ready = self.age_ticks[index] >= phenotype.maturity_ticks
                && self.cooldown_ticks[index] == 0
                && self.energy_milli[index] >= self.config.phase2.pairing_energy_threshold_milli;
            inputs[14] = if ready { 1.0 } else { 0.0 };

            // 16: recent damage neutral zero (no combat in Phase 2).

            // 17-20: memory values.
            inputs[16..20].copy_from_slice(&p2.memory[index]);
        }
        self.phase2 = Some(p2);
    }

    /// Nearest other organism within `range_fp`, deterministic tie-break by
    /// (distance, entity ID). Bucket size covers the maximum sensor range.
    fn nearest_within(&self, index: usize, range_fp: i64) -> Option<(i64, usize)> {
        let x = i64::from(self.x_fp[index]);
        let y = i64::from(self.y_fp[index]);
        let range_squared = range_fp * range_fp;
        let bucket_x = (self.x_fp[index] / self.bucket_size_fp).min(self.buckets_x as i32 - 1);
        let bucket_y = (self.y_fp[index] / self.bucket_size_fp).min(self.buckets_y as i32 - 1);
        let mut best: Option<(i64, u64, usize)> = None;
        for neighbor_y in (bucket_y - 1).max(0)..=(bucket_y + 1).min(self.buckets_y as i32 - 1) {
            for neighbor_x in (bucket_x - 1).max(0)..=(bucket_x + 1).min(self.buckets_x as i32 - 1)
            {
                let bucket = (neighbor_y as usize) * self.buckets_x as usize + neighbor_x as usize;
                for &candidate in &self.buckets[bucket] {
                    let candidate = candidate as usize;
                    if candidate == index {
                        continue;
                    }
                    let dx = i64::from(self.x_fp[candidate]) - x;
                    let dy = i64::from(self.y_fp[candidate]) - y;
                    let distance_squared = dx * dx + dy * dy;
                    if distance_squared > range_squared {
                        continue;
                    }
                    let key = (distance_squared, self.ids[candidate], candidate);
                    if best.is_none_or(|current| (key.0, key.1) < (current.0, current.1)) {
                        best = Some(key);
                    }
                }
            }
        }
        best.map(|(distance_squared, _, candidate)| (distance_squared, candidate))
    }

    /// Evaluate every controller into bounded intents. Controllers request
    /// actions only; the resolver validates them in the apply phase.
    fn controllers_phase2(&mut self, next_tick: u64) {
        let attack_threshold =
            self.config.contest.attack_threshold_q16 as f32 / crate::config::Q16_ONE as f32;
        if let Some(contest) = self.contest.as_mut() {
            contest.intent_attack.clear();
            contest.intent_attack.resize(self.ids.len(), false);
        }
        let mut p2 = self.phase2.take().expect("phase2 state present");
        let population = self.ids.len();
        p2.intent_turn.clear();
        p2.intent_turn.resize(population, 0.0);
        p2.intent_speed_milli.clear();
        p2.intent_speed_milli.resize(population, 0);
        p2.intent_eat.clear();
        p2.intent_eat.resize(population, false);
        p2.intent_mate.clear();
        p2.intent_mate.resize(population, false);
        p2.next_memory.clear();
        p2.next_memory.resize(population, [0.0; 4]);

        let eat_threshold = self.config.phase2.eat_threshold_q16 as f32 / 65536.0;
        let mate_threshold = self.config.phase2.mate_threshold_q16 as f32 / 65536.0;
        let rest_threshold = self.config.phase2.rest_threshold_q16 as f32 / 65536.0;

        // Schema 2 evaluates the organism's own evolved graph and maps its
        // action channels onto the same twelve output slots topology 1
        // produced, so everything below this point is schema-agnostic.
        let mut schema2 = self.schema2.take();
        // Learned deltas are read here and written only in the learn phase,
        // so evaluation sees a value that has been stable since the previous
        // tick's learn phase. An empty slice in a world with no plasticity
        // section, where no compiled edge carries a slot to index with.
        let learn = self.learn.take();
        // Phase 12 objects: the cues the gather reads for channels 17..=22
        // and the request slots 113..=117 become intents. Taken for the loop
        // and restored after it, like the schema-2 and learned state.
        let mut objects = self.objects.take();
        let action_threshold = self.config.artifact.action_threshold_q16 as f32 / 65536.0;
        for index in 0..population {
            let output = match schema2.as_mut() {
                Some(state) => {
                    let inputs = p2.inputs[index];
                    let cues: [f32; 6] = objects
                        .as_ref()
                        .and_then(|objects| objects.perception.get(index).copied())
                        .unwrap_or([0.0; 6]);
                    let before = state.activations[index].faults;
                    let mut requests = std::mem::take(&mut state.requests);
                    let learned: &[i32] = match learn.as_ref() {
                        Some(learn) => &learn.learned_q16[index],
                        None => &[],
                    };
                    crate::controller2::evaluate(
                        &state.plans[index],
                        &mut state.activations[index],
                        learned,
                        &|channel_id| {
                            // Channel IDs 1..=16 are the sixteen sensory
                            // inputs in `inputs[0..16]`; 17..=22 are the
                            // Phase 12 object cues, zero in a world without
                            // the section (and unbindable there).
                            crate::schema2::SENSE_CHANNELS
                                .iter()
                                .position(|candidate| *candidate == channel_id)
                                .map(|slot| inputs[slot])
                                .or_else(|| {
                                    (crate::registry::CHANNEL_OBJECT_PRESENT
                                        ..=crate::registry::CHANNEL_CARRIED_LOAD)
                                        .contains(&channel_id)
                                        .then(|| {
                                            cues[usize::from(
                                                channel_id
                                                    - crate::registry::CHANNEL_OBJECT_PRESENT,
                                            )]
                                        })
                                })
                                .unwrap_or(0.0)
                        },
                        &mut requests,
                    );
                    let outputs = crate::schema2::outputs_from_requests(&requests);
                    // A bound object channel above the threshold is a
                    // request; its value in milli is the claim priority. An
                    // unbound channel is absent from `requests` and is
                    // therefore never requested.
                    if let Some(objects) = objects.as_mut()
                        && let Some(intent) = objects.intents.get_mut(index)
                    {
                        let request = |channel: u16| -> Option<i32> {
                            requests
                                .binary_search_by_key(&channel, |(id, _)| *id)
                                .ok()
                                .map(|slot| requests[slot].1)
                                .filter(|value| *value > action_threshold)
                                .map(|value| (value * 1000.0) as i32)
                        };
                        intent.pick_up = request(crate::registry::CHANNEL_PICK_UP);
                        intent.drop = request(crate::registry::CHANNEL_DROP);
                        intent.place = request(crate::registry::CHANNEL_PLACE);
                        intent.strike = request(crate::registry::CHANNEL_STRIKE);
                        intent.combine = request(crate::registry::CHANNEL_COMBINE);
                    }
                    state.requests = requests;
                    crate::controller::ControllerOutput {
                        outputs,
                        faults: state.activations[index].faults.saturating_sub(before),
                    }
                }
                None => controller::evaluate(&p2.genomes[index], &p2.inputs[index]),
            };
            if output.faults > 0 {
                p2.counters.controller_faults_total += u64::from(output.faults);
                let id = self.ids[index];
                self.push_event(
                    next_tick,
                    EventKind::ControllerFault {
                        id,
                        faults: output.faults,
                    },
                );
            }
            let phenotype = &p2.phenotypes[index];

            // Steering: turn output plus approach/avoid bias toward the
            // nearest organism (input 8 holds the relative-heading proxy).
            let approach = phenotype.approach_milli as f32 / 1000.0;
            let bias = (output.outputs[OUT_FOLLOW] - output.outputs[OUT_AVOID])
                * approach
                * p2.inputs[index][7];
            p2.intent_turn[index] = (output.outputs[OUT_TURN] + bias).clamp(-1.0, 1.0);

            // Throttle maps [-1, 1] onto [0, 1] so organisms have baseline
            // mobility; the rest channel is the explicit opt-out
            // (phase2-behavior-v1 output mapping).
            let resting = output.outputs[OUT_REST] > rest_threshold;
            let throttle = if resting {
                0.0
            } else {
                (output.outputs[OUT_THROTTLE] + 1.0) * 0.5
            };
            p2.intent_speed_milli[index] = (throttle * phenotype.max_speed_milli as f32) as i64;

            p2.intent_eat[index] = output.outputs[OUT_EAT] > eat_threshold;
            p2.intent_mate[index] = output.outputs[OUT_MATE] > mate_threshold;
            // OUT_ATTACK stops being a no-op when the contest section is
            // enabled; without it the channel stays exactly as inert as it
            // was, which is what preserves the Phase 2 fixture.
            if let Some(contest) = self.contest.as_mut() {
                contest.intent_attack[index] = output.outputs[crate::controller::OUT_ATTACK]
                    > attack_threshold
                    && self.cooldown_ticks[index] == 0;
            }
            p2.next_memory[index] = controller::next_memory(&output);
        }
        // Prior-state buffers advance only after **every** organism has been
        // evaluated, exactly as schema 1's memory values become next-tick
        // memory only after all controller evaluation completes. Doing it
        // inline would let a later organism read a neighbour's current
        // activation through a delayed edge.
        if let Some(state) = schema2.as_mut() {
            for activation in &mut state.activations {
                crate::controller2::commit(activation);
            }
        }
        self.schema2 = schema2;
        self.learn = learn;
        self.objects = objects;
        self.phase2 = Some(p2);
    }

    /// Resolve Phase 2 intents: memory commit, movement, costs, feeding,
    /// pairing, aging. All order-sensitive passes run in stable ID order.
    fn apply_phase2(&mut self, next_tick: u64) {
        let mut p2 = self.phase2.take().expect("phase2 state present");
        let population = self.ids.len();
        let extent_x = self.config.world_extent_x_fp();
        let extent_y = self.config.world_extent_y_fp();
        let dt = i64::from(self.config.dt_ms);

        // Memory commit (phase-separated from evaluation).
        for index in 0..population {
            p2.memory[index] = p2.next_memory[index];
        }

        // **Per-organism action counting (C11.1's substrate).**
        //
        // Placed here, at the top of the resolver, because this is the last
        // point at which every intent for this tick exists and none has been
        // consumed: the movement pass below overwrites `speed_milli` with
        // what the organism *achieved*, and the feeding pass reads
        // `intent_eat` against biomass that may not be there. C11.1 asks what
        // the organism did, not what the world let it do - an organism that
        // learned to head for the patch and arrived to find it moved has
        // changed its behaviour, and a counter driven by realized outcomes
        // would score that as no change at all.
        //
        // This block writes and never reads. No value below depends on it,
        // no draw is taken, and the census is not consulted by any phase, so
        // it cannot alter the trajectory - which is what the fixtures assert.
        if let Some(census) = self.action_census.as_mut() {
            let attacks = self
                .contest
                .as_ref()
                .map(|contest| contest.intent_attack.as_slice());
            for index in 0..population {
                let attack = attacks
                    .and_then(|flags| flags.get(index))
                    .copied()
                    .unwrap_or(false);
                census.record(
                    index,
                    p2.intent_turn[index],
                    p2.intent_speed_milli[index],
                    p2.intent_eat[index],
                    p2.intent_mate[index],
                    attack,
                );
            }
        }

        // Movement pass.
        let max_turn = self.config.phase2.max_turn_per_tick_bam as f32;
        let mut moved = vec![false; population];
        for (index, moved_flag) in moved.iter_mut().enumerate() {
            let turn = p2.intent_turn[index];
            let delta_bam = (turn * max_turn) as i32;
            p2.heading_bam[index] =
                (i32::from(p2.heading_bam[index]) + delta_bam).rem_euclid(65536) as u16;
            p2.last_turn[index] = turn;

            let speed = p2.intent_speed_milli[index];
            if speed <= 0 {
                p2.speed_milli[index] = 0;
                continue;
            }
            let heading = p2.heading_bam[index];
            let step = speed * dt * i64::from(crate::FP_PER_METER);
            let step_x = step * i64::from(cos_bam_q15(heading)) / (1000 * 1000 * 32768);
            let step_y = step * i64::from(sin_bam_q15(heading)) / (1000 * 1000 * 32768);
            let new_x =
                (i64::from(self.x_fp[index]) + step_x).clamp(0, i64::from(extent_x) - 1) as i32;
            let new_y =
                (i64::from(self.y_fp[index]) + step_y).clamp(0, i64::from(extent_y) - 1) as i32;
            // Composed, exactly as the schema-1 movement pass is. Phase 12
            // adds one entry check beside it: a free object heavy enough to
            // block refuses entry to its cell and nothing else (ADR-0028
            // section 12). `cell_blocked_by_object` is `false` without the
            // section, so the pre-Phase-12 arithmetic is untouched.
            let target_cell = self.cell_of(new_x, new_y);
            if self.effective_traversable(target_cell)
                && (target_cell == self.cell_of(self.x_fp[index], self.y_fp[index])
                    || !self.cell_blocked_by_object(target_cell))
            {
                self.x_fp[index] = new_x;
                self.y_fp[index] = new_y;
                p2.speed_milli[index] = speed;
                *moved_flag = true;
            } else {
                // Rejected moves keep position and pay no movement cost.
                p2.speed_milli[index] = 0;
            }
        }

        // Cost pass: metabolism scales with the genome-derived multipliers,
        // and from Phase 8 also with body mass (allometry) and with the
        // distance from the organism's preferred temperature.
        let physiology_config = self.config.physiology;
        let mut allometric_added = 0_i128;
        let mut thermal_added = 0_i128;
        for (index, &did_move) in moved.iter().enumerate() {
            let phenotype = &p2.phenotypes[index];
            let mut cost = self.basal_cost_tick * phenotype.basal_mult_milli / 1000;
            if physiology_config.enabled {
                let linear = cost;
                cost =
                    cost * crate::physiology::allometry_multiplier_milli(
                        &physiology_config,
                        phenotype.body_scale_milli,
                    ) / 1000;
                allometric_added += i128::from(cost - linear);
                if let Some(temperature) =
                    self.temperature_milli(self.cell_of(self.x_fp[index], self.y_fp[index]))
                {
                    let thermal = crate::physiology::thermal_cost_milli(
                        &physiology_config,
                        phenotype.thermal_pref_milli,
                        temperature,
                        self.config.dt_ms,
                    );
                    cost += thermal;
                    thermal_added += i128::from(thermal);
                }
            }
            if did_move {
                let speed_frac_q16 =
                    (p2.speed_milli[index] << 16) / phenotype.max_speed_milli.max(1);
                let speed_squared_q16 = (speed_frac_q16 * speed_frac_q16) >> 16;
                let mut move_cost =
                    self.move_cost_tick * phenotype.body_scale_milli * speed_squared_q16
                        / (1000 * 65536);
                // Phase 12: carried mass multiplies the movement cost by
                // `1 + carry_move_cost_q16 * carried / capacity`. Exactly
                // one for an organism holding nothing, and the branch is
                // not taken at all without the section, so the pre-Phase-12
                // cost is untouched.
                if self.objects.is_some() {
                    let carried = self.held_mass_milli(index);
                    if carried > 0 {
                        let capacity = self.carry_capacity_milli(index);
                        let extra_q16 = i64::from(self.config.artifact.carry_move_cost_q16)
                            * carried
                            / capacity;
                        move_cost += move_cost * extra_q16 >> 16;
                    }
                }
                cost += move_cost;
            }
            // Phase 12: holding costs whether or not the holder moved
            // (review 15.6, "carrying indefinitely has a cost").
            if self.objects.is_some() {
                let carried = self.held_mass_milli(index);
                if carried > 0 {
                    let capacity = self.carry_capacity_milli(index);
                    cost += self.config.artifact.hold_cost_milli_per_s
                        * i64::from(self.config.dt_ms)
                        * carried
                        / (1000 * capacity);
                }
            }
            if self.intent_crowded[index] {
                cost += self.crowding_cost_tick;
            }
            let paid = cost.min(self.energy_milli[index]);
            self.energy_milli[index] -= paid;
            self.ledger.spent_milli += i128::from(paid);
        }
        if let Some(physiology) = self.physiology.as_mut() {
            physiology.allometric_cost_milli += allometric_added;
            physiology.thermal_cost_milli += thermal_added;
        }

        // Feeding pass: requires an eat request; intake scales with the
        // diet-affinity multiplier. Stable ID order resolves contention.
        let assimilation = i64::from(self.config.assimilation_q16);
        for index in 0..population {
            if !p2.intent_eat[index] || self.energy_milli[index] <= 0 {
                continue;
            }
            let cell = self.cell_of(self.x_fp[index], self.y_fp[index]);
            let available = self.biomass_milli[cell];
            if available <= 0 {
                continue;
            }
            let remaining_capacity = self.energy_capacity_of(index) - self.energy_milli[index];
            if remaining_capacity <= 0 {
                continue;
            }
            let intake_rate = self.intake_tick * p2.phenotypes[index].intake_mult_milli / 1000;
            let intake_cap = remaining_capacity * Q16 / assimilation;
            let intake = available.min(intake_rate).min(intake_cap);
            if intake <= 0 {
                continue;
            }
            let gain = intake * assimilation / Q16;
            self.biomass_milli[cell] -= intake;
            self.energy_milli[index] += gain;
            self.ledger.consumed_biomass_milli += i128::from(intake);
            self.ledger.assimilated_milli += i128::from(gain);
        }

        // Pairing pass: deterministic greedy selection in stable ID order.
        p2.pending.clear();
        if self.config.reproduction_enabled {
            self.resolve_pairs(next_tick, &mut p2);
        }

        // Aging pass.
        for index in 0..population {
            self.age_ticks[index] += 1;
            self.cooldown_ticks[index] = self.cooldown_ticks[index].saturating_sub(1);
        }

        self.phase2 = Some(p2);
    }

    /// Paired-parent selection and child creation. Requirements: mutual
    /// bounded mate intent, maturity, energy, completed cooldown, pairing
    /// range, trait compatibility, capacity, and valid placement. Energy is
    /// debited only after every check passes.
    fn resolve_pairs(&mut self, next_tick: u64, p2: &mut Phase2State) {
        let population = self.ids.len();
        let phase2_config = self.config.phase2;
        // Copied out once: `self.morphology` is borrowed mutably inside the
        // pairing loop, so the config cannot be read through `self` there.
        let morphology_config = self.config.morphology;
        let range_fp = i64::from(phase2_config.pairing_range_m) * i64::from(crate::FP_PER_METER);
        let range_squared = range_fp * range_fp;
        let compatibility = phase2_config.compatibility_threshold_q16 as f32 / 65536.0;
        let mut paired = vec![false; population];

        let eligible = |p2: &Phase2State,
                        cooldowns: &[u64],
                        ages: &[u64],
                        energies: &[i64],
                        index: usize|
         -> bool {
            p2.intent_mate[index]
                && ages[index] >= p2.phenotypes[index].maturity_ticks
                && cooldowns[index] == 0
                && energies[index] >= phase2_config.pairing_energy_threshold_milli
        };

        for index in 0..population {
            if paired[index]
                || !eligible(
                    p2,
                    &self.cooldown_ticks,
                    &self.age_ticks,
                    &self.energy_milli,
                    index,
                )
            {
                continue;
            }
            // Find the nearest unpaired, mutually willing, compatible mate.
            let x = i64::from(self.x_fp[index]);
            let y = i64::from(self.y_fp[index]);
            let bucket_x = (self.x_fp[index] / self.bucket_size_fp).min(self.buckets_x as i32 - 1);
            let bucket_y = (self.y_fp[index] / self.bucket_size_fp).min(self.buckets_y as i32 - 1);
            let mut best: Option<(i64, u64, usize)> = None;
            for neighbor_y in (bucket_y - 1).max(0)..=(bucket_y + 1).min(self.buckets_y as i32 - 1)
            {
                for neighbor_x in
                    (bucket_x - 1).max(0)..=(bucket_x + 1).min(self.buckets_x as i32 - 1)
                {
                    let bucket =
                        (neighbor_y as usize) * self.buckets_x as usize + neighbor_x as usize;
                    for &candidate in &self.buckets[bucket] {
                        let candidate = candidate as usize;
                        if candidate == index || paired[candidate] {
                            continue;
                        }
                        if !eligible(
                            p2,
                            &self.cooldown_ticks,
                            &self.age_ticks,
                            &self.energy_milli,
                            candidate,
                        ) {
                            continue;
                        }
                        let dx = i64::from(self.x_fp[candidate]) - x;
                        let dy = i64::from(self.y_fp[candidate]) - y;
                        let distance_squared = dx * dx + dy * dy;
                        if distance_squared > range_squared {
                            continue;
                        }
                        let distance = match self.schema2.as_ref() {
                            Some(state) => crate::schema2::compatibility_distance(
                                &state.genomes[index],
                                &state.genomes[candidate],
                            ),
                            None => {
                                p2.genomes[index].normalized_distance(&p2.genomes[candidate], 0)
                            }
                        };
                        if distance > compatibility {
                            continue;
                        }
                        let key = (distance_squared, self.ids[candidate], candidate);
                        if best.is_none_or(|current| (key.0, key.1) < (current.0, current.1)) {
                            best = Some(key);
                        }
                    }
                }
            }
            let Some((_, _, partner)) = best else {
                continue;
            };

            let parent_a = self.ids[index];
            let parent_b = self.ids[partner];
            let cooldown_a = p2.phenotypes[index].cooldown_ticks;
            let cooldown_b = p2.phenotypes[partner].cooldown_ticks;

            // Capacity check before any cost.
            if population + p2.pending.len() >= self.config.max_entities as usize {
                p2.counters.pair_rejected_capacity_total += 1;
                self.cooldown_ticks[index] = cooldown_a;
                self.cooldown_ticks[partner] = cooldown_b;
                paired[index] = true;
                paired[partner] = true;
                self.push_event(
                    next_tick,
                    EventKind::PairRejected {
                        parent_a,
                        parent_b,
                        reason: PairRejectReason::Capacity,
                    },
                );
                continue;
            }

            // Deterministic child identity for all child-keyed draws.
            let child_id = self.next_entity_id + p2.pending.len() as u64;

            // Placement near the parent midpoint.
            let mid_x = ((i64::from(self.x_fp[index]) + i64::from(self.x_fp[partner])) / 2) as i32;
            let mid_y = ((i64::from(self.y_fp[index]) + i64::from(self.y_fp[partner])) / 2) as i32;
            let Some((birth_x, birth_y)) =
                self.find_birth_position(next_tick, child_id, mid_x, mid_y)
            else {
                p2.counters.pair_rejected_placement_total += 1;
                self.cooldown_ticks[index] = cooldown_a;
                self.cooldown_ticks[partner] = cooldown_b;
                paired[index] = true;
                paired[partner] = true;
                self.push_event(
                    next_tick,
                    EventKind::PairRejected {
                        parent_a,
                        parent_b,
                        reason: PairRejectReason::Placement,
                    },
                );
                continue;
            };

            // Energy: child energy is the mean of the two genome-derived
            // investments; parent A (lower ID) pays the rounding remainder.
            let child_energy =
                (p2.phenotypes[index].invest_milli + p2.phenotypes[partner].invest_milli) / 2;
            let invest_b = child_energy / 2;
            let invest_a = child_energy - invest_b;
            let overhead = phase2_config.pairing_overhead_milli;
            let pay_a = invest_a + overhead;
            let pay_b = invest_b + overhead;
            if self.energy_milli[index] < pay_a || self.energy_milli[partner] < pay_b {
                p2.counters.pair_rejected_energy_total += 1;
                self.cooldown_ticks[index] = cooldown_a;
                self.cooldown_ticks[partner] = cooldown_b;
                paired[index] = true;
                paired[partner] = true;
                self.push_event(
                    next_tick,
                    EventKind::PairRejected {
                        parent_a,
                        parent_b,
                        reason: PairRejectReason::Energy,
                    },
                );
                continue;
            }

            // Recombination with bounded variation, keyed by child ID.
            let policy = VariationPolicy {
                probability_q16: phase2_config.variation_probability_q16,
                trait_sigma_q16: phase2_config.variation_trait_sigma_q16,
                neural_sigma_q16: phase2_config.variation_neural_sigma_q16,
            };
            // Schema 2 replaces per-gene independent choice with meiosis
            // plus structural mutation. `child_genome2` is `Some` exactly
            // when the schema-2 section is enabled.
            // Carried out of the `schema2` borrow so the rejections can be
            // evented once `state` is released; `MutationReport` is `Copy`
            // and fixed-size, so this costs nothing.
            let mut mutation_report = crate::structmut::MutationReport::default();
            let (genome, variation, child_genome2, genome_hash, phenotype) =
                match self.schema2.as_mut() {
                    Some(state) => {
                        let mut child = crate::meiosis::recombine(
                            (&state.genomes[index], parent_a),
                            (&state.genomes[partner], parent_b),
                            &self.config.genome2.meiosis,
                            self.config.world_seed,
                            next_tick,
                            child_id,
                        );
                        // Mutate only a viable recombinant. The operators
                        // validate their own output and revert on failure,
                        // so handing them an already-invalid genome makes
                        // every one of them report *its input* as invalid -
                        // which is how `rejected_invalid` filled up with
                        // dangling references that insertion had not caused.
                        // An operator's rejection counter is only readable
                        // if the operator is the one thing that could have
                        // caused it. Skipping the draws costs nothing:
                        // streams are keyed by draw index, not consumed in
                        // sequence, so an unmutated child leaves every other
                        // organism's draws exactly where they were.
                        if child.validate_structure(&self.config.genome2.caps).is_ok() {
                            mutation_report = crate::structmut::mutate(
                                &mut child,
                                &self.config.genome2.mutation,
                                &self.config.genome2.caps,
                                &mut state.counters,
                                self.config.world_seed,
                                next_tick,
                                child_id,
                                self.config.plasticity_rule_draw_count(),
                                self.config.channel_registry_version(),
                            );
                        }
                        let hash = crate::checksum::fnv1a64(&child.encode());
                        let traits = resolve_traits(&child.express_traits());
                        (
                            None,
                            VariationSummary::default(),
                            Some(child),
                            hash,
                            Phenotype::from_traits(&traits),
                        )
                    }
                    None => {
                        let (genome, variation): (Genome, VariationSummary) = recombine(
                            &p2.genomes[index],
                            &p2.genomes[partner],
                            policy,
                            self.config.world_seed,
                            next_tick,
                            child_id,
                        );
                        let hash = genome.stable_hash();
                        let phenotype = Phenotype::derive(&genome);
                        (Some(genome), variation, None, hash, phenotype)
                    }
                };

            // C9.6: emitted here rather than after the viability checks
            // below, because the rejection happened and was counted whether
            // or not this pairing goes on to produce a child. Emitting it
            // only for surviving children would make the event and the
            // counter disagree, and the counter is in the checksum.
            for (operator, reason) in mutation_report.rejections() {
                self.push_event(
                    next_tick,
                    EventKind::StructuralMutationRejected {
                        child_id,
                        operator,
                        reason,
                    },
                );
            }
            // **Meiosis can produce a genome that is not viable, and nothing
            // upstream checks.** Crossover cuts at an arbitrary point, so a
            // gamete can carry an edge whose node stayed on the other side
            // of the cut; the mutation operators validate their own output
            // but never the recombinant they were handed. Refused here,
            // alongside the other pairing rejections and before either
            // parent pays, so the failure costs a mating opportunity rather
            // than corrupting the ledger.
            if let Some(child) = child_genome2.as_ref()
                && child.validate_structure(&self.config.genome2.caps).is_err()
            {
                p2.counters.pair_rejected_nonviable_total += 1;
                self.push_event(
                    next_tick,
                    EventKind::PairRejected {
                        parent_a,
                        parent_b,
                        reason: PairRejectReason::Nonviable,
                    },
                );
                continue;
            }

            // **Development runs here, at pairing time**, for the same reason
            // the genome check above does: a child whose body will not work
            // costs a mating opportunity rather than the energy ledger, and
            // the phenotype the rest of the tick reads has to come from the
            // body rather than from trait genes. The grown body travels with
            // the pending child so development runs once per birth.
            let mut child_body = None;
            let mut phenotype = phenotype;
            if let Some(state) = self.morphology.as_mut()
                && let Some(child) = child_genome2.as_ref()
            {
                match crate::develop::develop(
                    child,
                    morphology_config.lattice,
                    &morphology_config.caps,
                    &mut state.counters,
                ) {
                    Ok(body) => {
                        let derived = body.derive();
                        // **C10.7: brain costs body.** A controller larger
                        // than the body's neural tissue can support is not a
                        // controller this organism can run, so the child is
                        // refused rather than having its network trimmed - a
                        // trimmed network is one no genome encoded, and the
                        // same fail-closed rule the structural caps follow.
                        //
                        // This is what makes the coupling structural rather
                        // than stipulated: growing a bigger brain requires
                        // growing neural modules, which cost mass and a
                        // fourth-power upkeep, and nothing anywhere states
                        // that a big brain is good or bad.
                        let budget = morphology_config.base_node_budget + derived.node_budget();
                        let nodes = child.express_network().nodes.len() as u32;
                        if nodes > budget {
                            state.counters.refused_node_budget += 1;
                            p2.counters.pair_rejected_nonviable_total += 1;
                            self.push_event(
                                next_tick,
                                EventKind::PairRejected {
                                    parent_a,
                                    parent_b,
                                    reason: PairRejectReason::Nonviable,
                                },
                            );
                            continue;
                        }
                        let traits = resolve_traits(&child.express_traits());
                        phenotype = Phenotype::from_body(&traits, &derived, &state.reference);
                        child_body = Some(body);
                    }
                    Err(_) => {
                        // The class is already counted by `DevelopCounters`;
                        // this counts the *pairing* that failed because of it.
                        p2.counters.pair_rejected_nonviable_total += 1;
                        self.push_event(
                            next_tick,
                            EventKind::PairRejected {
                                parent_a,
                                parent_b,
                                reason: PairRejectReason::Nonviable,
                            },
                        );
                        continue;
                    }
                }
            }

            let heading = (named_random(
                self.config.world_seed,
                next_tick,
                RngSystem::Reproduction,
                child_id,
                100,
            ) & 0xffff) as u16;

            self.energy_milli[index] -= pay_a;
            self.energy_milli[partner] -= pay_b;
            self.ledger.spent_milli += i128::from(2 * overhead);
            self.cooldown_ticks[index] = cooldown_a;
            self.cooldown_ticks[partner] = cooldown_b;
            p2.child_count[index] += 1;
            p2.child_count[partner] += 1;
            p2.counters.mutated_trait_genes_total += u64::from(variation.mutated_trait_genes);
            p2.counters.mutated_neural_genes_total += u64::from(variation.mutated_neural_genes);
            let depth = p2.depth[index].max(p2.depth[partner]).saturating_add(1);
            paired[index] = true;
            paired[partner] = true;
            p2.pending.push(PendingChild {
                parent_a,
                parent_b,
                genome,
                genome2: child_genome2,
                genome_hash,
                body: child_body,
                phenotype,
                x_fp: birth_x,
                y_fp: birth_y,
                heading_bam: heading,
                energy_milli: child_energy,
                invest_a_milli: invest_a,
                invest_b_milli: invest_b,
                depth,
                variation,
            });
        }
    }

    fn find_birth_position(
        &self,
        next_tick: u64,
        parent_id: u64,
        parent_x_fp: i32,
        parent_y_fp: i32,
    ) -> Option<(i32, i32)> {
        let radius_fp = BIRTH_RADIUS_M * i64::from(crate::FP_PER_METER);
        let span = (2 * radius_fp + 1) as u64;
        let extent_x = i64::from(self.config.world_extent_x_fp());
        let extent_y = i64::from(self.config.world_extent_y_fp());
        for attempt in 0..BIRTH_PLACEMENT_ATTEMPTS {
            let draw_x = named_random(
                self.config.world_seed,
                next_tick,
                RngSystem::Reproduction,
                parent_id,
                attempt * 2,
            );
            let draw_y = named_random(
                self.config.world_seed,
                next_tick,
                RngSystem::Reproduction,
                parent_id,
                attempt * 2 + 1,
            );
            let offset_x = (draw_x % span) as i64 - radius_fp;
            let offset_y = (draw_y % span) as i64 - radius_fp;
            let x = (i64::from(parent_x_fp) + offset_x).clamp(0, extent_x - 1) as i32;
            let y = (i64::from(parent_y_fp) + offset_y).clamp(0, extent_y - 1) as i32;
            // Composed: a child may not be placed where its parent could not
            // walk. Founder placement, by contrast, stays on the baseline -
            // it is a generation-time property, and composing it would make a
            // world's founding depend on a schedule.
            if self.effective_traversable(self.cell_of(x, y)) {
                return Some((x, y));
            }
        }
        None
    }

    /// Move a configured share of a dying organism's remaining energy into
    /// a carcass. The share can never exceed what the organism had, which is
    /// what C7.4 checks.
    fn spawn_carcass(&mut self, next_tick: u64, index: usize) {
        let Some(contest) = self.contest.as_ref() else {
            return;
        };
        let share = self.config.contest.carcass_energy_q16;
        if share == 0 {
            return;
        }
        // Phase 12 artifact half: the carcass is an object with a fresh id
        // rather than a `ContestState` carcass, and the contest table is
        // bypassed entirely. Without the section this branch is not taken
        // and the Phase 7 path below runs byte for byte (ADR-0028 section 9).
        if self.objects.is_some() {
            let remaining = self.energy_milli[index].max(0);
            let energy = (remaining * i64::from(share)) >> 16;
            if energy <= 0 {
                return;
            }
            self.spawn_carcass_object(next_tick, index, energy);
            return;
        }
        // At the cap the oldest carcass is dropped, and the loss is ledgered
        // as decay rather than silently discarded. "Oldest" is by creation
        // tick with the lower ID breaking ties, not by table position: the
        // table is ordered by ID, and an older organism can die later, so
        // position says nothing about age.
        if contest.carcasses.len() >= self.config.contest.max_carcasses as usize
            && let Some(contest) = self.contest.as_mut()
            && !contest.carcasses.is_empty()
        {
            let victim = contest
                .carcasses
                .iter()
                .enumerate()
                .min_by_key(|(_, carcass)| (carcass.created_tick, carcass.id))
                .map(|(index, _)| index)
                .unwrap_or(0);
            let dropped = contest.carcasses.remove(victim);
            contest.carcass_decayed_milli += i128::from(dropped.energy_milli);
        }
        let remaining = self.energy_milli[index].max(0);
        let energy = (remaining * i64::from(share)) >> 16;
        if energy <= 0 {
            return;
        }
        let id = self.ids[index];
        let (x_fp, y_fp) = (self.x_fp[index], self.y_fp[index]);
        if let Some(contest) = self.contest.as_mut() {
            // Inserted in sorted position, not appended: entity IDs are
            // assigned at birth, so a carcass created later can carry a
            // lower ID than one created earlier. The table is canonically
            // ordered by ID so its checksum is order-free.
            let carcass = Carcass {
                id,
                x_fp,
                y_fp,
                energy_milli: energy,
                created_tick: next_tick,
            };
            let position = contest
                .carcasses
                .binary_search_by_key(&id, |existing| existing.id)
                .unwrap_or_else(|position| position);
            contest.carcasses.insert(position, carcass);
            contest.carcass_created_milli += i128::from(energy);
        }
        self.push_event(
            next_tick,
            EventKind::CarcassCreated {
                id,
                source: id,
                energy_milli: energy,
            },
        );
    }

    /// Resolve attacks, apply healing, decay recent damage, and decay
    /// carcasses. Empty when the contest section is disabled, so the tick
    /// costs exactly what it did before Phase 7.
    fn contest_phase(&mut self, next_tick: u64) {
        if self.contest.is_none() {
            return;
        }
        let dt = i64::from(self.config.dt_ms);
        let contest_config = self.config.contest;
        let population = self.ids.len();
        let range_fp = i64::from(contest_config.attack_range_m) * i64::from(crate::FP_PER_METER);

        // --- Attacks. Intents were set during the controller phase. ---
        // Damage is computed against the *previous* health of every target
        // and applied afterwards, so two organisms attacking each other in
        // the same tick resolve simultaneously and symmetrically rather than
        // in visit order.
        let mut damage_to = vec![0_i64; population];
        let mut attacks = Vec::new();
        if let Some(contest) = self.contest.as_ref() {
            for index in 0..population {
                if !contest.intent_attack.get(index).copied().unwrap_or(false) {
                    continue;
                }
                // Nearest valid target within range, ties by (distance, ID),
                // which is the standard contention policy.
                let Some((_, target)) = self.nearest_within(index, range_fp) else {
                    continue;
                };
                attacks.push((index, target));
            }
        }
        for &(attacker, target) in &attacks {
            let (attacker_scale, target_scale) = match self.phase2.as_ref() {
                Some(p2) => (
                    p2.phenotypes[attacker].body_scale_milli,
                    p2.phenotypes[target].body_scale_milli,
                ),
                None => (1_000, 1_000),
            };
            let raw = ContestState::damage_milli(
                &contest_config,
                self.config.world_seed,
                next_tick,
                self.ids[attacker],
                self.ids[target],
                attacker_scale,
                target_scale,
            );
            damage_to[target] += raw;
            // The attack costs its attacker whether or not it kills.
            let cost = contest_config
                .attack_cost_milli
                .min(self.energy_milli[attacker]);
            self.energy_milli[attacker] -= cost;
            self.ledger.spent_milli += i128::from(cost);
            let (attacker_id, target_id) = (self.ids[attacker], self.ids[target]);
            if let Some(contest) = self.contest.as_mut() {
                contest.attacks_total += 1;
            }
            let health_after = self.contest.as_ref().map_or(0, |contest| {
                contest.health_milli[target] - damage_to[target]
            });
            self.push_event(
                next_tick,
                EventKind::Damage {
                    attacker: attacker_id,
                    target: target_id,
                    raw_milli: raw,
                    applied_milli: raw,
                    health_milli: health_after,
                },
            );
        }

        // --- Apply damage, heal, decay. ---
        let heal_per_tick = contest_config.heal_milli_per_s * dt / 1_000;
        // Hoisted out of the loop as a *fraction*, not an absolute: with
        // morphology the capacity differs per organism, so an absolute floor
        // computed from the global would let a small-storage organism heal
        // below what the policy intends and forbid a large-storage one from
        // healing at all.
        let heal_energy_floor_q16 = i64::from(contest_config.heal_energy_floor_q16);
        let decay_per_tick = i64::from(contest_config.damage_decay_q16_per_s) * dt / 1_000;
        let mut heal_spend = vec![0_i64; population];
        if let Some(contest) = self.contest.as_mut() {
            for (index, &applied) in damage_to.iter().enumerate() {
                if applied > 0 {
                    contest.health_milli[index] -= applied;
                    contest.recent_damage_milli[index] += applied;
                    contest.damage_dealt_milli += i128::from(applied);
                }
                // Recent damage decays toward zero.
                let decay = (contest.recent_damage_milli[index] * decay_per_tick) >> 16;
                contest.recent_damage_milli[index] = (contest.recent_damage_milli[index]
                    - decay.max(if decay > 0 { 1 } else { 0 }))
                .max(0);
            }
        }
        // Healing is a separate pass so it reads settled health.
        let max_health: Vec<i64> = match self.phase2.as_ref() {
            Some(p2) => p2
                .phenotypes
                .iter()
                .map(|phenotype| {
                    ContestState::health_max_milli(&contest_config, phenotype.body_scale_milli)
                })
                .collect(),
            None => vec![contest_config.base_health_milli; population],
        };
        if heal_per_tick > 0
            && let Some(contest) = self.contest.as_mut()
        {
            for index in 0..population {
                let deficit = max_health[index] - contest.health_milli[index];
                if deficit <= 0 || contest.health_milli[index] <= 0 {
                    continue;
                }
                let restored = heal_per_tick.min(deficit);
                let cost = (restored * i64::from(contest_config.heal_energy_cost_q16)) >> 16;
                heal_spend[index] = (restored, cost).1;
                contest.health_milli[index] += restored;
                contest.healed_milli += i128::from(restored);
            }
        }
        for (index, &spend) in heal_spend.iter().enumerate() {
            if spend <= 0 {
                continue;
            }
            let energy_floor = (self.energy_capacity_of(index) * heal_energy_floor_q16) >> 16;
            if self.energy_milli[index] <= energy_floor {
                // Could not afford it after all: undo the restoration.
                if let Some(contest) = self.contest.as_mut() {
                    let restored =
                        (spend << 16) / i64::from(contest_config.heal_energy_cost_q16).max(1);
                    contest.health_milli[index] -= restored;
                    contest.healed_milli -= i128::from(restored);
                }
                continue;
            }
            let cost = spend.min(self.energy_milli[index]);
            self.energy_milli[index] -= cost;
            self.ledger.spent_milli += i128::from(cost);
        }

        // --- Carcass consumption and decay. ---
        let reach_fp = i64::from(contest_config.carcass_reach_m) * i64::from(crate::FP_PER_METER);
        let assimilation = i64::from(self.config.assimilation_q16);
        let decay_q16 = i64::from(contest_config.carcass_decay_q16_per_s) * dt / 1_000;
        let mut consumed_events = Vec::new();
        if let Some(mut contest) = self.contest.take() {
            // Organisms consume in ascending ID order; each carcass is
            // visited in ascending carcass ID order. Both orders are
            // canonical, so contested consumption resolves deterministically.
            for index in 0..population {
                if self.energy_milli[index] >= self.energy_capacity_of(index) {
                    continue;
                }
                let (x, y) = (i64::from(self.x_fp[index]), i64::from(self.y_fp[index]));
                for carcass in contest.carcasses.iter_mut() {
                    if carcass.energy_milli <= 0 {
                        continue;
                    }
                    let dx = i64::from(carcass.x_fp) - x;
                    let dy = i64::from(carcass.y_fp) - y;
                    if dx * dx + dy * dy > reach_fp * reach_fp {
                        continue;
                    }
                    let room = self.energy_capacity_of(index) - self.energy_milli[index];
                    let take = carcass.energy_milli.min(self.intake_tick).max(0);
                    if take <= 0 {
                        continue;
                    }
                    let gained = ((take * assimilation) >> 16).min(room);
                    if gained <= 0 {
                        continue;
                    }
                    // Raw energy leaves the carcass; the assimilated part
                    // enters the organism, mirroring the plant path exactly.
                    let raw = (gained << 16) / assimilation.max(1);
                    let raw = raw.min(carcass.energy_milli);
                    carcass.energy_milli -= raw;
                    contest.carcass_consumed_milli += i128::from(raw);
                    self.energy_milli[index] += gained;
                    self.ledger.assimilated_milli += i128::from(gained);
                    // The unassimilated remainder is a loss from the carcass
                    // pool and is ledgered as decay so nothing vanishes.
                    contest.carcass_decayed_milli += i128::from(raw - gained);
                    contest.carcass_consumed_milli -= i128::from(raw - gained);
                    consumed_events.push((carcass.id, self.ids[index], gained));
                    break;
                }
            }
            // Decay, then drop empties.
            for carcass in contest.carcasses.iter_mut() {
                let decay = ((carcass.energy_milli * decay_q16) >> 16).max(if decay_q16 > 0 {
                    1
                } else {
                    0
                });
                let decay = decay.min(carcass.energy_milli);
                carcass.energy_milli -= decay;
                contest.carcass_decayed_milli += i128::from(decay);
            }
            contest.carcasses.retain(|carcass| carcass.energy_milli > 0);
            self.contest = Some(contest);
        }
        for (id, consumer, energy_milli) in consumed_events {
            self.push_event(
                next_tick,
                EventKind::CarcassConsumed {
                    id,
                    consumer,
                    energy_milli,
                },
            );
        }
    }

    /// Update every plastic edge and charge for it (Phase 11).
    ///
    /// Runs after `apply` and reads only values already committed for this
    /// tick: the presynaptic value each plastic edge actually read, captured
    /// during evaluation; the postsynaptic activation the same evaluation
    /// produced; and the modulator node's activation from that same update.
    /// It writes only learned state and energy. **No organism's learning
    /// reads another organism's state at all**, current-tick or prior, so
    /// Rule 4 is satisfied by construction rather than by ordering: the
    /// per-organism loop below could run in any order and produce the same
    /// answer, and it runs in index order anyway because index order is
    /// entity-ID order and the events it emits are ordered.
    ///
    /// **Zero allocation per organism per tick.** Nothing here builds a
    /// buffer; the learned-state rows were sized at birth and are written in
    /// place.
    ///
    /// # Nothing in this function knows how well an organism is doing
    ///
    /// The inputs to `plasticity::step` are four activations and a genome
    /// weight. Energy, age, offspring count, and food are not among them and
    /// must never be: what gates a modulated update is the organism's own
    /// evolved modulatory node, and this function cannot tell a modulator
    /// that fires on food from one that fires on a wall.
    fn learn_phase(&mut self, next_tick: u64) {
        let Some(mut learn) = self.learn.take() else {
            return;
        };
        // Both are taken out of `self` so the loop can hold a plan while
        // `push_event` and the energy debit take `&mut self`. The established
        // idiom in this file, and the reason `controllers_phase2` does it.
        let Some(schema2) = self.schema2.take() else {
            // Unreachable: validation refuses plasticity without genome2. The
            // state goes back rather than being dropped, because a silent
            // `None` here would be a desync at the next invariant check.
            self.learn = Some(learn);
            return;
        };
        let cost_per_edge_thousandths = self.plastic_edge_cost_milli_thousandths;
        let price_moved_only = self.config.plasticity.price_moved_edges_only;

        for index in 0..self.ids.len() {
            let plan = &schema2.plans[index];
            let activation = &schema2.activations[index];
            let learned_row = &mut learn.learned_q16[index];
            let trace_row = &mut learn.trace_q16[index];
            let before = learn.faults[index];
            // The moat's charge basis, counted in the same pass that does the
            // work. Zero when the moat is off, and unread in that case.
            let mut moved_edges = 0_i64;
            // Ascending `homology_id`: `plastic_edges` was built in the
            // compile pass over `network.edges`, which is already in that
            // order, so this loop inherits the spec's update order instead of
            // re-deriving it (Rule 6's discipline applied to the learn pass).
            for (slot, edge) in plan.plastic_edges.iter().enumerate() {
                let learned_q16 = learned_row[slot];
                let signals = EdgeSignals {
                    // The value this edge read during evaluation. Not
                    // `prior[source]`: `commit` has already run, so both
                    // activation buffers hold this tick's values and a
                    // delayed edge's actual input is only in the capture.
                    pre: activation.plastic_pre(slot),
                    post: activation.values[edge.target as usize],
                    // An edge with no usable modulator is handed 0.0, which
                    // makes rules 3 and 4 inert rather than always-on.
                    modulator: if edge.modulator == crate::controller2::NO_MODULATOR {
                        0.0
                    } else {
                        activation.values[edge.modulator as usize]
                    },
                    w_eff: plasticity::effective_weight(edge.weight, learned_q16),
                };
                let outcome = plasticity::step(
                    edge.rule,
                    signals,
                    LearnedState {
                        learned_q16,
                        trace_q16: trace_row[slot],
                    },
                );
                // `step` has already neutralized a non-finite delta to zero
                // and reported it; counting and eventing it is this loop's
                // half of the controller-fault policy.
                learn.counters.record(&outcome);
                // **State movement, not `StepKind::Applied`.** `Applied` means
                // the rule form ran and the state was rewritten *possibly to
                // the same value*, and the only non-`Applied` kinds are the
                // rule-0 early return and an unreachable refusal - so under
                // `live_rule_zero`, which removes rule 0, every edge is
                // `Applied` and a moat priced on it would charge exactly what
                // the unpriced engine charges. That would make the moat a
                // no-op in both arms where the chain is on and collapse the
                // 2x2's fourth arm into its second. D-107 named this failure
                // in advance; the campaign pre-registration records the
                // correction.
                if outcome.state
                    != (LearnedState {
                        learned_q16,
                        trace_q16: trace_row[slot],
                    })
                {
                    moved_edges += 1;
                }
                if outcome.fault {
                    learn.faults[index] = learn.faults[index].saturating_add(1);
                }
                learned_row[slot] = outcome.state.learned_q16;
                trace_row[slot] = outcome.state.trace_q16;
            }
            let faults = learn.faults[index] - before;
            if faults > 0 {
                let id = self.ids[index];
                self.push_event(next_tick, EventKind::PlasticityFault { id, faults });
            }

            // The energy cost, **charged after the update and never before**.
            // The order cannot change what was learned - energy is not an
            // input to any rule and must not become one - so the choice is
            // about what the cost means: an organism pays for the plastic
            // edges it carried through this tick, having exercised them. It
            // also means an organism whose last energy goes on plasticity
            // still learned this tick and dies in `lifecycle` afterwards,
            // rather than being silently exempted from a cost it could not
            // afford.
            //
            // Every plastic edge pays, including a rule-0 edge that writes
            // nothing: the cost is the price of carrying the machinery, and
            // charging only edges that moved would make "turn the rule off"
            // a free way to keep the flag.
            // **Exact, with the sub-milli remainder carried rather than
            // discarded.** The owed amount is in thousandths of a milli; it
            // is added to what this organism already owed, divided once, and
            // the leftover kept. So `n` edges at a rate that rounds to zero
            // per tick still cost `n * rate * dt / 1000` milli over time,
            // and the total charged is never more than one milli behind the
            // true cost. Truncating each tick instead - which is what every
            // other cost in this file does, correctly, because they are
            // charged once per organism against a large number - made a
            // plastic edge free at the shipped rate.
            // **The price basis is an arm of D-107's 2x2, so it is a config
            // read and not a constant.** With the moat off this is
            // `plastic_edges.len()` exactly as it has always been, so no
            // fixture moves; with it on, an edge that wrote nothing this tick
            // pays nothing this tick.
            let charged_edges = if price_moved_only {
                moved_edges
            } else {
                plan.plastic_edges.len() as i64
            };
            let owed =
                charged_edges * cost_per_edge_thousandths + i64::from(learn.cost_remainder[index]);
            let cost = owed / 1_000;
            // Non-negative by construction: both terms are non-negative, so
            // the remainder needs no sign handling and stays inside 0..1000.
            learn.cost_remainder[index] = (owed % 1_000) as u32;
            if cost > 0 {
                // `min(cost, energy)` with the paired ledger add, never a
                // debit without one: `check_invariants` compares the ledger
                // to the summed energy with **no tolerance**, so an unledgered
                // milli is a hard failure rather than a rounding note.
                let paid = cost.min(self.energy_milli[index]);
                self.energy_milli[index] -= paid;
                self.ledger.spent_milli += i128::from(paid);
                learn.cost_milli += i128::from(paid);
            }
        }

        self.schema2 = Some(schema2);
        self.learn = Some(learn);
    }

    fn lifecycle(&mut self, next_tick: u64) {
        // Death marks in stable order. Starvation is checked before age
        // (documented tie policy).
        let population = self.ids.len();
        let mut dead = vec![false; population];
        // Health depletion is checked first when contest is enabled: it is
        // the most specific cause, and a death has exactly one.
        let depleted: Vec<bool> = match self.contest.as_ref() {
            Some(contest) => contest
                .health_milli
                .iter()
                .map(|&health| health <= 0)
                .collect(),
            None => vec![false; population],
        };
        // Phase 8 competing risks, drawn before the cause cascade so a
        // hazard death is attributable to the hazard that caused it. The
        // draws happen for every living organism, so which ones are
        // consulted below cannot depend on the cascade's order.
        let physiology_config = self.config.physiology;
        let maturity_ticks: Vec<u64> = match self.phase2.as_ref() {
            Some(p2) => p2
                .phenotypes
                .iter()
                .map(|phenotype| phenotype.maturity_ticks)
                .collect(),
            None => vec![self.config.maturity_age_ticks; population],
        };
        let juvenile: Vec<bool> = (0..population)
            .map(|index| self.age_ticks[index] < maturity_ticks[index])
            .collect();
        let mut hazards = vec![HazardOutcome::Survives; population];
        if self.physiology.is_some() {
            let mut applied_hazard = vec![0_i64; population];
            for index in 0..population {
                let (outcome, applied) = crate::physiology::hazard_draw(
                    &physiology_config,
                    self.config.world_seed,
                    next_tick,
                    self.ids[index],
                    self.age_ticks[index],
                    maturity_ticks[index],
                    self.config.dt_ms,
                );
                hazards[index] = outcome;
                applied_hazard[index] = applied;
            }
            if let Some(physiology) = self.physiology.as_mut() {
                for (index, applied) in applied_hazard.into_iter().enumerate() {
                    physiology.cumulative_hazard_q16[index] =
                        physiology.cumulative_hazard_q16[index].saturating_add(applied);
                }
            }
        }
        // Senescence replaces the hard cutoff rather than joining it: with
        // the hazard live, `max_age_ticks` would truncate the very tail the
        // hazard exists to shape, and C8.5's lifespan comparison would
        // measure the cutoff instead of the evolved trait.
        let hard_age_cutoff = !physiology_config.enabled || !physiology_config.senescence_enabled;
        for (index, dead_flag) in dead.iter_mut().enumerate() {
            let cause = if depleted[index] {
                Some(DeathCause::Damage)
            } else if self.energy_milli[index] <= 0 {
                Some(DeathCause::Starvation)
            } else if hazards[index] == HazardOutcome::Senescence {
                Some(DeathCause::Senescence)
            } else if hazards[index] == HazardOutcome::Extrinsic {
                Some(DeathCause::Extrinsic)
            } else if hard_age_cutoff && self.age_ticks[index] >= self.config.max_age_ticks {
                Some(DeathCause::OldAge)
            } else {
                None
            };
            let Some(cause) = cause else { continue };
            *dead_flag = true;
            match cause {
                DeathCause::Starvation => self.counters.deaths_starvation_total += 1,
                DeathCause::OldAge => self.counters.deaths_old_age_total += 1,
                DeathCause::Damage => {
                    if let Some(contest) = self.contest.as_mut() {
                        contest.deaths_by_damage_total += 1;
                    }
                }
                // Counted in the physiology section rather than in
                // `Counters`, so the Phase 1/2 checksum field list is
                // untouched -- the same rule Phase 7 followed for damage.
                DeathCause::Senescence | DeathCause::Extrinsic => {
                    let was_juvenile = juvenile[index];
                    if let Some(physiology) = self.physiology.as_mut() {
                        if cause == DeathCause::Senescence {
                            physiology.deaths_senescence_total += 1;
                        } else {
                            physiology.deaths_extrinsic_total += 1;
                        }
                        if was_juvenile {
                            physiology.deaths_juvenile_total += 1;
                        }
                    }
                }
            }
            self.ledger.removed_at_death_milli += i128::from(self.energy_milli[index]);
            let id = self.ids[index];
            self.push_event(next_tick, EventKind::Death { id, cause });
            if cause == DeathCause::Damage {
                self.push_event(next_tick, EventKind::DeathByDamage { id, attacker: 0 });
            }
            // A carcass takes a configured share of the dead organism's
            // remaining energy. That energy is already counted as removed
            // from the organism pool above, so the carcass pool has its own
            // exact ledger and can never exceed its source.
            self.spawn_carcass(next_tick, index);
        }
        // Phase 12: a dead organism drops what it holds at its position,
        // before compaction moves its slot. Runs once, after the death loop,
        // so the event order within a tick is every death (with its carcass),
        // then every drop, in ascending id - stated here so it is a documented
        // order rather than a discovered one. Empty without the section.
        self.drop_held_on_death(next_tick, &dead);

        if dead.iter().any(|&flag| flag) {
            retain_by_flags(&mut self.ids, &dead);
            retain_by_flags(&mut self.x_fp, &dead);
            retain_by_flags(&mut self.y_fp, &dead);
            retain_by_flags(&mut self.energy_milli, &dead);
            retain_by_flags(&mut self.age_ticks, &dead);
            retain_by_flags(&mut self.cooldown_ticks, &dead);
            if let Some(p2) = self.phase2.as_mut() {
                p2.retain(&dead);
            }
            if let Some(contest) = self.contest.as_mut() {
                contest.retain(&dead);
            }
            if let Some(physiology) = self.physiology.as_mut() {
                physiology.retain(&dead);
            }
            if let Some(state) = self.schema2.as_mut() {
                state.retain(&dead);
            }
            if let Some(state) = self.morphology.as_mut() {
                state.retain(&dead);
            }
            // A dead organism takes its learned state with it. There is
            // nothing for it to survive into: learned state is per-organism
            // and per-edge, and a child starts at zero.
            if let Some(state) = self.learn.as_mut() {
                state.retain(&dead);
            }
            // A dead organism takes its action history with it. The history
            // is the individual's, and C11.1's unit of measurement is the
            // individual: a row that outlived its organism would be attached
            // to whichever organism compacted into its slot, which is the one
            // failure that would make a per-individual series read like a
            // per-individual series and be a population average.
            if let Some(state) = self.action_census.as_mut() {
                state.retain(&dead);
            }
            if let Some(state) = self.objects.as_mut() {
                state.retain_organisms(&dead);
            }
        }

        // Births append after removal; IDs stay strictly increasing.
        let births: Vec<(u64, i32, i32)> = std::mem::take(&mut self.pending_births);
        for (parent_id, x, y) in births {
            let id = self.next_entity_id;
            self.next_entity_id += 1;
            self.ids.push(id);
            self.x_fp.push(x);
            self.y_fp.push(y);
            self.energy_milli.push(self.config.offspring_energy_milli);
            self.age_ticks.push(0);
            self.cooldown_ticks.push(0);
            if let Some(physiology) = self.physiology.as_mut() {
                physiology.push_organism();
            }
            if let Some(contest) = self.contest.as_mut() {
                contest.push_organism(self.config.contest.base_health_milli);
            }
            // Unreachable in practice: this is the schema-1 asexual birth
            // path, which only runs when Phase 2 is disabled, and plasticity
            // requires genome2 which requires Phase 2. Pushed anyway, with no
            // plastic edges because there is no schema-2 plan to size a row
            // from, so lockstep is maintained by construction instead of by
            // an argument about which paths can coexist. D2's cost is exactly
            // this bookkeeping, and the way it goes wrong is a branch nobody
            // thought could be taken.
            if let Some(state) = self.learn.as_mut() {
                state.push_organism(0);
            }
            if let Some(state) = self.action_census.as_mut() {
                state.push_organism();
            }
            let birth_capacity = self.terrain.capacity_milli[self.cell_of(x, y)];
            if let Some(state) = self.objects.as_mut() {
                let band = state.band_of(birth_capacity);
                state.push_organism(band);
            }
            self.counters.births_total += 1;
            self.push_event(next_tick, EventKind::Birth { id, parent_id });
        }

        // Phase 2 paired births. Children act starting next tick.
        if let Some(mut p2) = self.phase2.take() {
            let pending: Vec<PendingChild> = std::mem::take(&mut p2.pending);
            for child in pending {
                // **Admit the schema-2 organism before anything else is
                // pushed.** A child whose merged network will not compile is
                // refused rather than admitted, exactly as a malformed
                // genome is - but the refusal has to happen before the core
                // arrays grow, or the refusal is itself the corruption.
                //
                // This block used to push `ids`, positions, energy and age
                // first and `continue` afterwards, under a comment asserting
                // the arrays stayed in lockstep. They did not: the organism
                // arrays grew by one and the phase-2 arrays did not, and the
                // next sense phase indexed `phenotypes` out of bounds. It
                // took a merged-network zero-delay cycle to reach, which
                // `validate_structure` now rejects outright, so this path
                // should be unreachable - which is exactly why it must be
                // counted rather than trusted.
                let budget = self.config.plasticity_budget();
                if let (Some(state), Some(genome2)) = (self.schema2.as_mut(), child.genome2.clone())
                    && !state.push_organism(genome2, budget)
                {
                    // The parents already paid at pairing time and the
                    // investment was riding on this child, so refusing the
                    // birth without booking that energy would leave the
                    // ledger short by exactly the child's endowment. A
                    // failed pregnancy costs what it cost.
                    self.ledger.spent_milli += i128::from(child.energy_milli);
                    p2.counters.pair_rejected_nonviable_total += 1;
                    continue;
                }
                if let (Some(state), Some(body)) = (self.morphology.as_mut(), child.body.clone()) {
                    state.push_body(body);
                }
                // **C11.4, at the only path a child can enter by.** The row
                // is sized from the plan that was just compiled for this
                // child and is zero on every plastic edge, whatever its
                // parents had learned. `LearnState::push_organism` takes no
                // initial value, so there is nowhere for a parent's delta to
                // be passed even by mistake - which is what makes "reset at
                // birth" an invariant rather than a default someone could
                // later parameterize.
                if let (Some(state), Some(schema2)) = (self.learn.as_mut(), self.schema2.as_ref()) {
                    state.push_organism(schema2.plastic_edges(schema2.len() - 1));
                }
                // Pushed **after** the schema-2 refusal above and before the
                // core arrays grow, for the reason the block's own comment
                // gives: a refusal that happens after the arrays have grown
                // is itself the corruption.
                if let Some(state) = self.action_census.as_mut() {
                    state.push_organism();
                }
                let birth_capacity =
                    self.terrain.capacity_milli[self.cell_of(child.x_fp, child.y_fp)];
                if let Some(state) = self.objects.as_mut() {
                    let band = state.band_of(birth_capacity);
                    state.push_organism(band);
                }
                let id = self.next_entity_id;
                self.next_entity_id += 1;
                self.ids.push(id);
                self.x_fp.push(child.x_fp);
                self.y_fp.push(child.y_fp);
                self.energy_milli.push(child.energy_milli);
                self.age_ticks.push(0);
                self.cooldown_ticks.push(0);
                p2.push_organism(
                    child.genome,
                    child.genome_hash,
                    child.phenotype,
                    child.heading_bam,
                    [child.parent_a, child.parent_b],
                    child.depth,
                    next_tick,
                );
                if let Some(physiology) = self.physiology.as_mut() {
                    physiology.push_organism();
                }
                if let Some(contest) = self.contest.as_mut() {
                    contest.push_organism(ContestState::health_max_milli(
                        &self.config.contest,
                        child.phenotype.body_scale_milli,
                    ));
                }
                self.counters.births_total += 1;
                p2.counters.paired_births_total += 1;
                self.push_event(
                    next_tick,
                    EventKind::PairedBirth {
                        id,
                        parent_a: child.parent_a,
                        parent_b: child.parent_b,
                        genome_hash: child.genome_hash,
                        invest_a_milli: child.invest_a_milli,
                        invest_b_milli: child.invest_b_milli,
                        mutated_trait_genes: child.variation.mutated_trait_genes,
                        mutated_neural_genes: child.variation.mutated_neural_genes,
                    },
                );
            }
            self.phase2 = Some(p2);
        }

        // Phase 12: passive object decay, after births so a carcass created
        // this tick decays from the next. Empty without the section.
        self.decay_objects(next_tick);

        // Extinction is a latched, single-event transition. The world stays
        // valid, observable, and pausable.
        if self.ids.is_empty() && !self.extinct {
            self.extinct = true;
            self.push_event(next_tick, EventKind::Extinction);
        }
    }

    fn push_event(&mut self, tick: u64, kind: EventKind) {
        if self.events.len() >= MAX_EVENTS_PER_TICK {
            self.counters.dropped_events_total += 1;
            return;
        }
        self.events.push(Event { tick, kind });
    }

    // --- Checksum and invariants ------------------------------------------

    /// Deterministic checksum over the complete logical state. Computed on
    /// demand; the tick itself does not pay for it.
    pub fn state_checksum(&self) -> u64 {
        let mut hasher = Fnv1a64::new();
        hasher.update(b"lifesim-state-v1");
        hasher.update_u64(self.config_hash);
        hasher.update_u64(self.config.world_seed);
        hasher.update_u64(self.tick);
        hasher.update(&[u8::from(self.paused), u8::from(self.extinct)]);
        hasher.update_u64(self.next_entity_id);
        hasher.update_u64(self.counters.births_total);
        hasher.update_u64(self.counters.deaths_starvation_total);
        hasher.update_u64(self.counters.deaths_old_age_total);
        hasher.update_u64(self.counters.capacity_rejections_total);
        hasher.update_u64(self.counters.dropped_events_total);
        hasher.update_i128(self.ledger.initial_energy_milli);
        hasher.update_i128(self.ledger.assimilated_milli);
        hasher.update_i128(self.ledger.spent_milli);
        hasher.update_i128(self.ledger.removed_at_death_milli);
        hasher.update_i128(self.ledger.initial_biomass_milli);
        hasher.update_i128(self.ledger.grown_milli);
        hasher.update_i128(self.ledger.consumed_biomass_milli);
        for index in 0..self.ids.len() {
            hasher.update_u64(self.ids[index]);
            hasher.update_i32(self.x_fp[index]);
            hasher.update_i32(self.y_fp[index]);
            hasher.update_i64(self.energy_milli[index]);
            hasher.update_u64(self.age_ticks[index]);
            hasher.update_u64(self.cooldown_ticks[index]);
        }
        for &biomass in &self.biomass_milli {
            hasher.update_i64(biomass);
        }
        hasher.update_u64(self.terrain.terrain_checksum);
        // Phase 2 state is hashed only when it exists, so phase2-disabled
        // worlds produce the exact Phase 1 checksums.
        if let Some(p2) = self.phase2.as_ref() {
            p2.hash_into(&mut hasher);
        }
        // Rule 8: a new subsystem appends a tagged section only when its
        // state exists, so a climate-disabled world hashes exactly as it did
        // before Phase 6.
        if let Some(climate) = self.climate.as_ref() {
            climate.hash_into(&mut hasher);
        }
        if let Some(contest) = self.contest.as_ref() {
            contest.hash_into(&mut hasher);
        }
        if let Some(state) = self.schema2.as_ref() {
            state.hash_into(&mut hasher);
        }
        // Phase 10: only the developmental counters. Bodies are a pure
        // function of genomes, and genomes are already hashed above, so
        // hashing bodies would add no discriminating power - divergent
        // bodies imply divergent genomes and the section above catches it.
        if let Some(state) = self.morphology.as_ref() {
            state.hash_into(&mut hasher);
        }
        if let Some(physiology) = self.physiology.as_ref() {
            physiology.hash_into(&mut hasher);
        }
        // Phase 11, **appended at the very end**. The shipped order above
        // (phase2, climate, contest, schema2, morphology, physiology) already
        // deviates from the order Rule 8's table lists, and reordering it to
        // match would move every existing checksum for no gain. Appending is
        // what Rule 8 actually guarantees: a section added to the end never
        // changes the checksum of a world that lacks it, and a world without
        // the plasticity section hashes exactly as it did before Phase 11.
        if let Some(learn) = self.learn.as_ref() {
            learn.hash_into(&mut hasher);
        }
        // Phase 12, appended after Phase 11's for the same reason Phase 11's
        // was appended after everything else: a section added to the end
        // never changes the checksum of a world that lacks it, and four
        // fixtures lack this one.
        //
        // The **composed terrain checksum is deliberately not hashed here**.
        // It is a pure function of `terrain_checksum` - already hashed above -
        // and the modification set - hashed on the line below - so it would
        // add no discriminating power at a cost of one pass over every cell
        // per checksum call. That is the same argument that keeps developed
        // bodies out of the Phase 10 section. It is still computed and
        // verified where it means something: in the save, against the file.
        if let Some(worldmod) = self.worldmod.as_ref() {
            worldmod.hash_into(&mut hasher);
        }
        // Phase 11's measurement section, appended after Phase 12's for the
        // reason each of the last three was appended after its predecessor:
        // a section added to the end never changes the checksum of a world
        // that lacks it, and five fixtures lack this one.
        //
        // **Hashed rather than left out, and the consequence is deliberate.**
        // A lifetime's action counts have no source but the save - they are
        // accumulated from intents computed from stored activations, and
        // re-deriving them would need the run replayed from tick zero. That
        // is `learnstate.rs`'s argument verbatim. It costs one real thing:
        // `reset_action_census` moves the checksum, so a probe boundary is
        // part of the replay lineage. That is the honest outcome - a world
        // whose counters were zeroed at tick 5,000 is not the world whose
        // were not - and it is why the sampling path records cumulative rows
        // and never resets.
        if let Some(census) = self.action_census.as_ref() {
            census.hash_into(&mut hasher);
        }
        // Phase 12 objects, **appended last** - after the action census, not
        // in the position `determinism-extensions.md`'s Rule 8 table lists
        // `lifesim-object-state-v1` (before terrainmod). Appending never
        // moves a world that lacks the section; inserting would move every
        // worldmod world, and the table is amended rather than obeyed
        // (ADR-0028 section 13).
        if let Some(objects) = self.objects.as_ref() {
            objects.table.hash_into(&mut hasher);
        }
        hasher.finish()
    }

    /// Verify structural, bounds, and conservation invariants exactly.
    pub fn check_invariants(&self) -> Result<(), InvariantViolation> {
        if self.ids.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(InvariantViolation::EntityOrder);
        }
        for index in 0..self.ids.len() {
            let x = self.x_fp[index];
            let y = self.y_fp[index];
            if x < 0
                || y < 0
                || x >= self.config.world_extent_x_fp()
                || y >= self.config.world_extent_y_fp()
                // Composed. The permit direction is why this cannot stay on
                // the baseline: an organism legally standing on a water cell
                // that a modification made traversable would otherwise be an
                // invariant violation. The block direction is safe because
                // `apply_terrain_modification` refuses to make an occupied
                // cell non-traversable, so this check is strictly stronger
                // than the one it replaces rather than weaker.
                || !self.effective_traversable(self.cell_of(x, y))
            {
                return Err(InvariantViolation::PositionInvalid {
                    id: self.ids[index],
                });
            }
            let energy = self.energy_milli[index];
            if energy < 0 || energy > self.energy_capacity_of(index) {
                return Err(InvariantViolation::EnergyOutOfBounds {
                    id: self.ids[index],
                    energy_milli: energy,
                });
            }
            // With senescence live the hard cutoff is gone, so an organism
            // older than `max_age_ticks` is expected rather than invalid.
            // The bound that still holds is that it is alive at all, which
            // the hazard guarantees probabilistically and this cannot
            // assert.
            if !(self.config.physiology.enabled && self.config.physiology.senescence_enabled)
                && self.age_ticks[index] >= self.config.max_age_ticks
            {
                return Err(InvariantViolation::AgeOutOfBounds {
                    id: self.ids[index],
                    age_ticks: self.age_ticks[index],
                });
            }
        }
        for (cell, &biomass) in self.biomass_milli.iter().enumerate() {
            if biomass < 0 || biomass > self.effective_capacity_milli(cell) {
                return Err(InvariantViolation::BiomassOutOfBounds {
                    cell,
                    biomass_milli: biomass,
                });
            }
        }
        // Energy conservation: offspring transfers are internal to the
        // organism pool, so only external sources/sinks appear here.
        let expected_energy = self.ledger.initial_energy_milli + self.ledger.assimilated_milli
            - self.ledger.spent_milli
            - self.ledger.removed_at_death_milli;
        let actual_energy: i128 = self
            .energy_milli
            .iter()
            .map(|&energy| i128::from(energy))
            .sum();
        if expected_energy != actual_energy {
            return Err(InvariantViolation::EnergyLedgerMismatch {
                expected: expected_energy,
                actual: actual_energy,
            });
        }
        // Capacity loss is a genuine sink: biomass removed because a cell's
        // biome became less productive. It is ledgered rather than
        // discarded, so conservation stays exact.
        let capacity_loss = self
            .climate
            .as_ref()
            .map_or(0, |climate| climate.capacity_loss_milli);
        // Phase 12 opens a second sink of the same shape and it is a separate
        // term rather than an addition to the climate one. Two reasons, and
        // the first is the ordinary one: a climate-disabled world has no
        // `ClimateWorld` to put it on. The second is that they answer
        // different questions - "biomes drifted and the world got poorer" and
        // "something edited the terrain" - and a single total is how a signal
        // becomes noise (D-074). The control arm of the relocating patch is
        // *defined* by this term staying at zero.
        let worldmod_capacity_loss = self.worldmod_capacity_loss_milli();
        let expected_biomass = self.ledger.initial_biomass_milli + self.ledger.grown_milli
            - self.ledger.consumed_biomass_milli
            - capacity_loss
            - worldmod_capacity_loss;
        let actual_biomass: i128 = self
            .biomass_milli
            .iter()
            .map(|&biomass| i128::from(biomass))
            .sum();
        if expected_biomass != actual_biomass {
            return Err(InvariantViolation::BiomassLedgerMismatch {
                expected: expected_biomass,
                actual: actual_biomass,
            });
        }
        let damage_deaths = self
            .contest
            .as_ref()
            .map_or(0, |contest| contest.deaths_by_damage_total);
        // Phase 8 causes live in the physiology section's counters, so they
        // have to be subtracted here too. Leaving them out is exactly the
        // omission this invariant exists to catch, and it did.
        let hazard_deaths = self.physiology.as_ref().map_or(0, |physiology| {
            physiology.deaths_senescence_total + physiology.deaths_extrinsic_total
        });
        let expected_population = i128::from(self.config.initial_organisms)
            + i128::from(self.counters.births_total)
            - i128::from(self.counters.deaths_starvation_total)
            - i128::from(self.counters.deaths_old_age_total)
            - i128::from(damage_deaths)
            - i128::from(hazard_deaths);
        if expected_population != self.ids.len() as i128 {
            return Err(InvariantViolation::PopulationAccounting {
                expected: expected_population,
                actual: self.ids.len() as i128,
            });
        }
        // Objects draw from the same counter (Rule 2), so the identity gains
        // a term the day the first object exists and is unchanged before.
        let objects_allocated = self
            .objects
            .as_ref()
            .map_or(0, |objects| objects.table.objects_allocated_total);
        let expected_next = u64::from(self.config.initial_organisms)
            + self.counters.births_total
            + objects_allocated
            + 1;
        if expected_next != self.next_entity_id {
            return Err(InvariantViolation::EntityIdAllocation {
                expected: expected_next,
                actual: self.next_entity_id,
            });
        }
        // Phase 9 structural invariant. Every parallel-array subsystem needs
        // one: a missed push on a birth path is invisible until it panics
        // thousands of ticks later.
        if let Some(state) = self.schema2.as_ref() {
            if state.len() != self.ids.len()
                || state.plans.len() != self.ids.len()
                || state.activations.len() != self.ids.len()
            {
                return Err(InvariantViolation::Schema2Desync {
                    organisms: self.ids.len(),
                    schema2: state.len(),
                });
            }
            if let Err(index) = crate::schema2::validate_all(state, &self.config.genome2.caps) {
                return Err(InvariantViolation::Schema2Invalid {
                    id: self.ids[index],
                });
            }
        }
        // Phase 11, on the same terms and with one extra clause. The outer
        // length is the ordinary lockstep check every parallel-array
        // subsystem needs; the per-row width is the one a length check alone
        // would miss, and it is the difference between an organism reading
        // its own learned delta and reading a neighbour's.
        if let Some(learn) = self.learn.as_ref() {
            if learn.len() != self.ids.len() {
                return Err(InvariantViolation::LearnDesync {
                    organisms: self.ids.len(),
                    learn: learn.len(),
                });
            }
            if let Some(schema2) = self.schema2.as_ref() {
                for index in 0..learn.len() {
                    if learn.plastic_edges(index) != schema2.plastic_edges(index) {
                        return Err(InvariantViolation::LearnDesync {
                            organisms: schema2.plastic_edges(index),
                            learn: learn.plastic_edges(index),
                        });
                    }
                }
            }
            // Every stored value inside the clamp. The tick cannot produce a
            // violation - `accumulate_clamped` clamps - so this defends the
            // restore path and any future initialization policy, both of
            // which would otherwise put an out-of-range value into
            // `effective_weight` and into the checksum.
            if let Some(index) = learn.bounds_violation() {
                return Err(InvariantViolation::LearnBounds {
                    id: self.ids[index],
                });
            }
        }
        // Phase 11's measurement array, on the ordinary lockstep terms.
        if let Some(census) = self.action_census.as_ref()
            && census.len() != self.ids.len()
        {
            return Err(InvariantViolation::ActionCensusDesync {
                organisms: self.ids.len(),
                census: census.len(),
            });
        }
        // Phase 12. Not a lockstep check - the modification set is indexed by
        // cell, not by organism - but the same idea applied to the two
        // properties the rest of the phase is built on top of. Sortedness and
        // uniqueness make application deterministic and the encoding unique;
        // the domains make the composition arithmetic total. Both are checked
        // rather than trusted because a restore writes this array from a
        // payload, and the payload is the one part of a Phase 12 save that
        // cannot be regenerated and compared.
        if let Some(worldmod) = self.worldmod.as_ref() {
            if let Some(index) = worldmod.order_violation() {
                return Err(InvariantViolation::TerrainModOrder { index });
            }
            if let Some(index) = worldmod.bounds_violation(self.terrain.cell_count()) {
                return Err(InvariantViolation::TerrainModBounds { index });
            }
        }
        // Phase 12 objects. Table structure, ledger identities, and the
        // derived caches, all checked rather than trusted because a restore
        // decodes the table from a payload. The genome check is the registry
        // gate of ADR-0028 section 7 from the world's side: a genome bound to
        // a channel this world does not offer is a defect whatever wrote it.
        if let Some(objects) = self.objects.as_ref() {
            if let Some(violation) = objects
                .table
                .violation(self.config.artifact.max_composition_depth.min(255) as u8)
            {
                return Err(InvariantViolation::ObjectTable { violation });
            }
            if objects.held.len() != self.ids.len() || objects.intents.len() != self.ids.len() {
                return Err(InvariantViolation::ObjectDesync {
                    organisms: self.ids.len(),
                    held: objects.held.len(),
                });
            }
            if !objects.held_is_consistent(&self.ids) {
                return Err(InvariantViolation::ObjectHeldMismatch);
            }
            if objects.table.free_count() > self.config.artifact.max_objects as usize
                || objects.table.len() > self.config.artifact.max_objects as usize
            {
                return Err(InvariantViolation::ObjectCap {
                    objects: objects.table.len(),
                    cap: self.config.artifact.max_objects,
                });
            }
            for (index, holder) in objects.table.holder_id.iter().enumerate() {
                if *holder != 0 && self.ids.binary_search(holder).is_err() {
                    return Err(InvariantViolation::ObjectHolderDead {
                        id: objects.table.ids[index],
                        holder: *holder,
                    });
                }
            }
        }
        if let Some(schema2) = self.schema2.as_ref() {
            let offered = self.config.channel_registry_version();
            for (index, genome) in schema2.genomes.iter().enumerate() {
                if !genome.bindings_offered_by(offered) {
                    return Err(InvariantViolation::ChannelNotOffered {
                        id: self.ids[index],
                        registry_version: offered,
                    });
                }
            }
        }
        // Phase 10, on the same terms. Bodies are derived rather than stored,
        // which makes a desync *more* likely to go unnoticed rather than
        // less: nothing on the save path would ever reveal it.
        if let Some(state) = self.morphology.as_ref()
            && (state.len() != self.ids.len() || state.derived.len() != self.ids.len())
        {
            return Err(InvariantViolation::MorphologyDesync {
                organisms: self.ids.len(),
                morphology: state.len(),
            });
        }
        // Phase 8 structural invariant, checked before the hazard arrays
        // are ever indexed by organism position.
        if let Some(physiology) = self.physiology.as_ref()
            && physiology.len() != self.ids.len()
        {
            return Err(InvariantViolation::PhysiologyDesync {
                organisms: self.ids.len(),
                physiology: physiology.len(),
            });
        }
        // Phase 7 structural and bounds invariants.
        if let Some(contest) = self.contest.as_ref() {
            if contest.len() != self.ids.len() {
                return Err(InvariantViolation::ContestDesync {
                    organisms: self.ids.len(),
                    contest: contest.len(),
                });
            }
            for index in 0..contest.len() {
                if contest.recent_damage_milli[index] < 0 {
                    return Err(InvariantViolation::ContestStateInvalid {
                        id: self.ids[index],
                    });
                }
            }
            // Carcasses are sorted by ID and hold non-negative energy.
            if contest
                .carcasses
                .windows(2)
                .any(|pair| pair[0].id >= pair[1].id)
            {
                return Err(InvariantViolation::CarcassOrder);
            }
            if contest.carcasses.iter().any(|c| c.energy_milli <= 0) {
                return Err(InvariantViolation::CarcassOrder);
            }
            // The carcass pool is its own exact ledger.
            let expected = contest.carcass_created_milli
                - contest.carcass_consumed_milli
                - contest.carcass_decayed_milli;
            if expected != contest.total_carcass_energy_milli() {
                return Err(InvariantViolation::CarcassLedgerMismatch {
                    expected,
                    actual: contest.total_carcass_energy_milli(),
                });
            }
        }

        // Phase 2 structural and validity invariants.
        if let Some(p2) = self.phase2.as_ref() {
            if p2.len() != self.ids.len() {
                return Err(InvariantViolation::Phase2Desync {
                    organisms: self.ids.len(),
                    phase2: p2.len(),
                });
            }
            for index in 0..p2.len() {
                let id = self.ids[index];
                if p2.genomes.is_empty() {
                    // Schema 2: the flat genome arrays are empty and the
                    // schema-2 invariant above covers genome validity.
                    break;
                }
                let genome = &p2.genomes[index];
                if Genome::validated(*genome.traits(), genome.neural().to_vec()).is_err() {
                    return Err(InvariantViolation::InvalidGenome { id });
                }
                if p2.genome_hashes[index] != genome.stable_hash()
                    || p2.phenotypes[index] != Phenotype::derive(genome)
                {
                    return Err(InvariantViolation::InvalidGenome { id });
                }
                let [parent_a, parent_b] = p2.parents[index];
                let founders = parent_a == 0 && parent_b == 0;
                if !founders && (parent_a >= id || parent_b >= id) {
                    return Err(InvariantViolation::AncestryInvalid { id });
                }
                for &value in &p2.memory[index] {
                    if !value.is_finite() || !(-1.0..=1.0).contains(&value) {
                        return Err(InvariantViolation::ControllerStateInvalid { id });
                    }
                }
                if p2.speed_milli[index] < 0
                    || p2.speed_milli[index] > p2.phenotypes[index].max_speed_milli
                {
                    return Err(InvariantViolation::ControllerStateInvalid { id });
                }
            }
        }
        Ok(())
    }
}

pub(crate) fn retain_by_flags<T: Copy>(values: &mut Vec<T>, remove: &[bool]) {
    let mut write = 0_usize;
    for read in 0..values.len() {
        if !remove[read] {
            values[write] = values[read];
            write += 1;
        }
    }
    values.truncate(write);
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SEED: u64 = 0x5eed_cafe_f00d_beef;

    fn small_config() -> SimConfig {
        let mut config = SimConfig::phase1_default(TEST_SEED);
        config.cells_x = 64;
        config.cells_y = 64;
        config.initial_organisms = 40;
        config.max_entities = 200;
        config
    }

    /// A schema-2 world with the plasticity section live. Founder genomes
    /// carry no plastic edge, which is what makes the rows below length
    /// zero - fine for the lockstep and bounds assertions, which are about
    /// the arrays and not about learning.
    fn plasticity_world() -> World {
        let mut config = SimConfig::phase11_default(TEST_SEED);
        config.cells_x = 64;
        config.cells_y = 64;
        config.initial_organisms = 40;
        config.max_entities = 200;
        World::new(config).unwrap()
    }

    #[test]
    fn the_learn_state_invariants_fire_rather_than_waiting_for_an_index_panic() {
        // Both halves of `LearnDesync` and `LearnBounds`, injected directly.
        // D2 accepts hand-maintained lockstep as the price of keeping learned
        // state out of `Schema2State`; these invariants are what that price
        // buys, and an invariant nobody has watched fail is an invariant
        // nobody knows works.
        let world = plasticity_world();
        world
            .check_invariants()
            .expect("a fresh world is consistent");
        assert_eq!(world.learn.as_ref().map(|learn| learn.len()), Some(40));

        // An extra row: the missed-push failure, in the direction a birth
        // path gets wrong.
        let mut desynced = world.clone();
        desynced.learn.as_mut().unwrap().push_organism(0);
        assert_eq!(
            desynced.check_invariants(),
            Err(InvariantViolation::LearnDesync {
                organisms: 40,
                learn: 41,
            })
        );

        // A row of the wrong width: the failure an outer length check cannot
        // see, and the one that would let an organism read a neighbour's
        // learned delta.
        let mut ragged = world.clone();
        ragged.learn.as_mut().unwrap().learned_q16[3].push(0);
        ragged.learn.as_mut().unwrap().trace_q16[3].push(0);
        assert!(matches!(
            ragged.check_invariants(),
            Err(InvariantViolation::LearnDesync { .. })
        ));

        // A value outside the clamp. Unreachable through `step`, which is
        // exactly why the restore path needs the check.
        let mut out_of_bounds = world.clone();
        {
            let learn = out_of_bounds.learn.as_mut().unwrap();
            learn.learned_q16[5].push(crate::plasticity::LEARN_LIMIT_Q16 + 1);
            learn.trace_q16[5].push(0);
        }
        // The row is now ragged too, so the width check would fire first;
        // widen the plan-side expectation out of the way by removing schema2
        // from the comparison, which is what a restored world with no plans
        // looks like.
        out_of_bounds.schema2 = None;
        assert!(matches!(
            out_of_bounds.check_invariants(),
            Err(InvariantViolation::LearnBounds { .. })
        ));
    }

    #[test]
    fn the_worldmod_invariants_fire_on_a_payload_the_writers_cannot_produce() {
        // `set` and `clear` cannot produce an unsorted, duplicated, or
        // out-of-domain modification set, so the *only* way one enters a
        // world is a restore decoding an untrusted section - which is the
        // path that matters, because the modification set is the one part of
        // a Phase 12 save that cannot be regenerated and compared. Injected
        // directly here, for the reason the learn-state test above exists:
        // an invariant nobody has watched fail is an invariant nobody knows
        // works, and the corruption is unreachable from the public API.
        let mut config = small_config();
        config.worldmod.enabled = true;
        let mut world = World::new(config).unwrap();
        for cell in [10_u32, 40, 900] {
            assert_eq!(
                world.apply_terrain_modification(LAYER_CAPACITY_SCALE, cell as usize, Some(Q16)),
                ModOutcome::Inserted
            );
        }
        world
            .check_invariants()
            .expect("a written set is consistent");

        let mut unsorted = world.clone();
        unsorted.worldmod.as_mut().unwrap().cells.swap(0, 1);
        assert_eq!(
            unsorted.check_invariants(),
            Err(InvariantViolation::TerrainModOrder { index: 1 })
        );

        let mut duplicated = world.clone();
        duplicated.worldmod.as_mut().unwrap().cells[1] = 10;
        assert_eq!(
            duplicated.check_invariants(),
            Err(InvariantViolation::TerrainModOrder { index: 1 })
        );

        // Out of domain in each of the three ways: an unknown layer id, a
        // cell past the end of the map, and a negative capacity scale, which
        // would make the biomass bounds check unsatisfiable rather than
        // merely wrong.
        let mut bad_layer = world.clone();
        bad_layer.worldmod.as_mut().unwrap().layers[2] = LAYER_COUNT;
        assert_eq!(
            bad_layer.check_invariants(),
            Err(InvariantViolation::TerrainModBounds { index: 2 })
        );

        let mut past_the_end = world.clone();
        let cells = past_the_end.terrain.cell_count() as u32;
        past_the_end.worldmod.as_mut().unwrap().cells[2] = cells;
        assert_eq!(
            past_the_end.check_invariants(),
            Err(InvariantViolation::TerrainModBounds { index: 2 })
        );

        let mut negative = world.clone();
        negative.worldmod.as_mut().unwrap().values[0] = -1;
        assert_eq!(
            negative.check_invariants(),
            Err(InvariantViolation::TerrainModBounds { index: 0 })
        );
    }

    #[test]
    fn new_world_satisfies_invariants() {
        let world = World::new(small_config()).unwrap();
        world.check_invariants().unwrap();
        assert_eq!(world.population(), 40);
        assert_eq!(world.tick_number(), 0);
        assert!(!world.is_extinct());
    }

    #[test]
    fn paused_world_advances_zero_ticks() {
        let mut world = World::new(small_config()).unwrap();
        world.set_paused(true);
        let before = world.state_checksum();
        for _ in 0..10 {
            world.step();
        }
        assert_eq!(world.tick_number(), 0);
        // Pause flag participates in the checksum, so compare while paused.
        assert_eq!(world.state_checksum(), before);
        world.set_paused(false);
        world.step();
        assert_eq!(world.tick_number(), 1);
    }

    #[test]
    fn invariants_hold_over_short_run() {
        let mut world = World::new(small_config()).unwrap();
        for _ in 0..500 {
            world.step();
            world.check_invariants().unwrap();
        }
    }

    #[test]
    fn same_config_same_checksum_diverging_seed_diverges() {
        let mut first = World::new(small_config()).unwrap();
        let mut second = World::new(small_config()).unwrap();
        for _ in 0..200 {
            first.step();
            second.step();
        }
        assert_eq!(first.state_checksum(), second.state_checksum());

        let mut other_seed_config = small_config();
        other_seed_config.world_seed = TEST_SEED + 1;
        let mut third = World::new(other_seed_config).unwrap();
        for _ in 0..200 {
            third.step();
        }
        assert_ne!(first.state_checksum(), third.state_checksum());
    }

    #[test]
    fn starvation_kills_and_records_cause() {
        let mut config = small_config();
        // No food value: nothing assimilates, so everyone starves.
        config.intake_rate_milli_per_s = 0;
        config.initial_energy_milli = 30;
        config.basal_cost_milli_per_s = 100;
        config.reproduction_enabled = false;
        let mut world = World::new(config).unwrap();
        let initial = world.population();
        for _ in 0..20 {
            world.step();
            world.check_invariants().unwrap();
        }
        assert_eq!(world.population(), 0);
        assert!(world.is_extinct());
        assert_eq!(world.counters().deaths_starvation_total, initial as u64);
        // Extinct world remains steppable and valid.
        world.step();
        world.check_invariants().unwrap();
    }

    #[test]
    fn old_age_kills_at_max_age() {
        let mut config = small_config();
        config.max_age_ticks = 25;
        config.maturity_age_ticks = 10;
        config.reproduction_enabled = false;
        let mut world = World::new(config).unwrap();
        for _ in 0..24 {
            world.step();
        }
        assert!(world.population() > 0);
        world.step();
        assert_eq!(world.population(), 0);
        assert_eq!(
            world.counters().deaths_old_age_total,
            u64::from(world.config().initial_organisms)
        );
    }

    #[test]
    fn reproduction_produces_births_with_fresh_ids() {
        let mut config = small_config();
        config.maturity_age_ticks = 5;
        config.repro_cooldown_ticks = 5;
        config.initial_energy_milli = 12_000;
        config.energy_max_milli = 12_000;
        config.repro_threshold_milli = 9_000;
        let mut world = World::new(config).unwrap();
        let mut births = Vec::new();
        for _ in 0..50 {
            world.step();
            world.check_invariants().unwrap();
            for event in world.events() {
                if let EventKind::Birth { id, parent_id } = event.kind {
                    births.push((id, parent_id));
                }
            }
        }
        assert!(!births.is_empty(), "expected at least one birth");
        for (id, parent_id) in &births {
            assert!(*id > u64::from(world.config().initial_organisms));
            assert!(*parent_id < *id);
        }
        assert_eq!(world.counters().births_total, births.len() as u64);
    }

    #[test]
    fn capacity_ceiling_rejects_births_deterministically() {
        let mut config = small_config();
        config.max_entities = config.initial_organisms; // already at ceiling
        config.maturity_age_ticks = 1;
        config.repro_cooldown_ticks = 1;
        config.initial_energy_milli = 12_000;
        config.repro_threshold_milli = 9_000;
        let mut world = World::new(config).unwrap();
        for _ in 0..10 {
            world.step();
            world.check_invariants().unwrap();
        }
        assert_eq!(world.counters().births_total, 0);
        assert!(world.counters().capacity_rejections_total > 0);
        assert_eq!(world.population() as u32, world.config().initial_organisms);
    }

    #[test]
    fn organisms_never_enter_water() {
        let mut world = World::new(small_config()).unwrap();
        for _ in 0..300 {
            world.step();
        }
        world.check_invariants().unwrap();
    }

    #[test]
    fn events_are_bounded_and_host_reads_cannot_change_state() {
        let mut first = World::new(small_config()).unwrap();
        let mut second = World::new(small_config()).unwrap();
        for _ in 0..100 {
            first.step();
            let _ = first.events(); // reader
            second.step(); // non-reader
        }
        assert_eq!(first.state_checksum(), second.state_checksum());
    }

    #[test]
    fn retain_by_flags_preserves_order() {
        let mut values = vec![1, 2, 3, 4, 5];
        retain_by_flags(&mut values, &[false, true, false, true, false]);
        assert_eq!(values, vec![1, 3, 5]);
    }
}
