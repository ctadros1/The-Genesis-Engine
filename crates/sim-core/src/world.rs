//! World state and the deterministic tick.
//!
//! Organism storage is struct-of-arrays kept sorted by stable entity ID.
//! Births append strictly increasing IDs and removal preserves order, so
//! index order always equals entity-ID order; `check_invariants` verifies it.

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
use crate::origin::{self, OriginError};
use crate::phase2::{
    PairRejectReason, PendingChild, Phase2Counters, Phase2State, SENSOR_RANGE_MAX_M,
};
use crate::physiology::{HazardOutcome, PhysiologyState};
use crate::rng::{RngSystem, named_random};
use crate::schema2::Schema2State;
use crate::structmut::MutationCounters;
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
    Lifecycle,
    Finalize,
}

impl TickPhase {
    pub const ALL: [TickPhase; 8] = [
        TickPhase::Commands,
        TickPhase::Environment,
        TickPhase::SpatialIndex,
        TickPhase::Sense,
        TickPhase::Controllers,
        TickPhase::Apply,
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
/// Phase 7 contest variants. Version 3 is additive: every version 2 payload
/// is unchanged. Reading events never alters simulation state.
pub const EVENT_SCHEMA_VERSION: u32 = 3;

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
    pub carcasses: u64,
    pub total_carcass_energy_milli: i64,
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
    ContestStateInvalid {
        id: u64,
    },
    CarcassOrder,
    CarcassLedgerMismatch {
        expected: i128,
        actual: i128,
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
            crowding_radius_fp: i64::from(config.crowding_radius_m)
                * i64::from(crate::FP_PER_METER),
            phase2: None,
            climate: None,
            founder_genomes: Vec::new(),
            contest: None,
            physiology: None,
            schema2: None,
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
            if let Some(p2) = world.phase2.as_mut() {
                for index in 0..world.ids.len() {
                    let genome = crate::schema2::founder_from_traits(p2.genomes[index].traits());
                    let traits = genome.express_traits();
                    p2.phenotypes[index] =
                        crate::genome::Phenotype::from_traits(&resolve_traits(&traits));
                    if !schema2.push_organism(genome) {
                        return Err(NewWorldError::Config(
                            crate::config::ConfigError::PhysiologyRange(
                                "founder genome does not compile",
                                index as i64,
                            ),
                        ));
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
        }
        if world.config.physiology.enabled {
            let mut physiology = PhysiologyState::with_capacity(world.ids.len());
            for _ in 0..world.ids.len() {
                physiology.push_organism();
            }
            world.physiology = Some(physiology);
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

    pub fn genome2_enabled(&self) -> bool {
        self.schema2.is_some()
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

    /// Structural-mutation outcomes broken out by operator and by rejection
    /// reason. `None` when schema 2 is disabled.
    ///
    /// The aggregate applied/rejected pair in [`MetricsSnapshot`] cannot
    /// distinguish "duplication never fired" from "duplication fired and was
    /// rejected every time", and a null result about structural evolution
    /// means opposite things in those two worlds.
    pub fn mutation_counters(&self) -> Option<MutationCounters> {
        self.schema2.as_ref().map(|state| state.counters)
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
            let energy_frac_q8 = ((self.energy_milli[index].clamp(0, self.config.energy_max_milli)
                * 255)
                / self.config.energy_max_milli.max(1)) as u8;
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

        // Canonical phase 1: apply validated queued commands. Project
        // Phase 1 defines no in-tick commands (pause/resume act between
        // ticks); the phase stays explicit so the canonical order is stable.
        observer.phase_started(TickPhase::Commands);
        observer.phase_finished(TickPhase::Commands);

        observer.phase_started(TickPhase::Environment);
        self.step_climate(next_tick);
        self.grow_food();
        observer.phase_finished(TickPhase::Environment);

        observer.phase_started(TickPhase::SpatialIndex);
        self.build_spatial_index();
        observer.phase_finished(TickPhase::SpatialIndex);

        observer.phase_started(TickPhase::Sense);
        if self.phase2.is_some() {
            self.sense_phase2();
        } else {
            self.sense(next_tick);
        }
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
        observer.phase_finished(TickPhase::Apply);

        observer.phase_started(TickPhase::Lifecycle);
        self.lifecycle(next_tick);
        observer.phase_finished(TickPhase::Lifecycle);

        observer.phase_started(TickPhase::Finalize);
        self.tick = next_tick;
        observer.phase_finished(TickPhase::Finalize);
    }

    /// Carrying capacity for one cell after biome scaling.
    ///
    /// Without the climate section this is the terrain's own
    /// elevation-derived capacity, so the Phase 1/2 arithmetic is unchanged
    /// byte for byte.
    pub fn effective_capacity_milli(&self, cell: usize) -> i64 {
        match self.climate.as_ref() {
            Some(climate) => climate.capacity_milli(&self.terrain, &self.config.climate, cell),
            None => self.terrain.capacity_milli[cell],
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
                if !self.terrain.land[neighbor] {
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
            // movement cost (policy v1).
            if self.terrain.land[self.cell_of(new_x, new_y)] {
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
            let remaining_capacity = self.config.energy_max_milli - self.energy_milli[index];
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
            inputs[0] = self.energy_milli[index] as f32 / self.config.energy_max_milli as f32;
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
                if !self.terrain.land[neighbor] {
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
        for index in 0..population {
            let output = match schema2.as_mut() {
                Some(state) => {
                    let inputs = p2.inputs[index];
                    let before = state.activations[index].faults;
                    let mut requests = std::mem::take(&mut state.requests);
                    crate::controller2::evaluate(
                        &state.plans[index],
                        &mut state.activations[index],
                        &|channel_id| {
                            // Channel IDs 1..=16 are the sixteen sensory
                            // inputs in `inputs[0..16]`; 17..20 are topology
                            // 1's memory registers, which schema 2 does not
                            // expose.
                            crate::schema2::SENSE_CHANNELS
                                .iter()
                                .position(|candidate| *candidate == channel_id)
                                .map(|slot| inputs[slot])
                                .unwrap_or(0.0)
                        },
                        &mut requests,
                    );
                    let outputs = crate::schema2::outputs_from_requests(&requests);
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
            if self.terrain.land[self.cell_of(new_x, new_y)] {
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
                cost += self.move_cost_tick * phenotype.body_scale_milli * speed_squared_q16
                    / (1000 * 65536);
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
            let remaining_capacity = self.config.energy_max_milli - self.energy_milli[index];
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
                            crate::structmut::mutate(
                                &mut child,
                                &self.config.genome2.mutation,
                                &self.config.genome2.caps,
                                &mut state.counters,
                                self.config.world_seed,
                                next_tick,
                                child_id,
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
            if self.terrain.land[self.cell_of(x, y)] {
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
        let energy_floor =
            (self.config.energy_max_milli * i64::from(contest_config.heal_energy_floor_q16)) >> 16;
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
                if self.energy_milli[index] >= self.config.energy_max_milli {
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
                    let room = self.config.energy_max_milli - self.energy_milli[index];
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
                if let (Some(state), Some(genome2)) = (self.schema2.as_mut(), child.genome2.clone())
                    && !state.push_organism(genome2)
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
        if let Some(physiology) = self.physiology.as_ref() {
            physiology.hash_into(&mut hasher);
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
                || !self.terrain.land[self.cell_of(x, y)]
            {
                return Err(InvariantViolation::PositionInvalid {
                    id: self.ids[index],
                });
            }
            let energy = self.energy_milli[index];
            if energy < 0 || energy > self.config.energy_max_milli {
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
        let expected_biomass = self.ledger.initial_biomass_milli + self.ledger.grown_milli
            - self.ledger.consumed_biomass_milli
            - capacity_loss;
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
        let expected_next =
            u64::from(self.config.initial_organisms) + self.counters.births_total + 1;
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

fn retain_by_flags<T: Copy>(values: &mut Vec<T>, remove: &[bool]) {
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
