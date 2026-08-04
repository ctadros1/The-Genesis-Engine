//! World state and the deterministic tick.
//!
//! Organism storage is struct-of-arrays kept sorted by stable entity ID.
//! Births append strictly increasing IDs and removal preserves order, so
//! index order always equals entity-ID order; `check_invariants` verifies it.

use crate::checksum::Fnv1a64;
use crate::config::{ConfigError, SimConfig};
use crate::controller::{
    self, OUT_AVOID, OUT_EAT, OUT_FOLLOW, OUT_MATE, OUT_REST, OUT_THROTTLE, OUT_TURN, cos_bam_q15,
    sin_bam_q15,
};
use crate::genome::{
    CONTROLLER_INPUTS, Genome, Phenotype, VariationPolicy, VariationSummary, recombine,
};
use crate::phase2::{
    PairRejectReason, PendingChild, Phase2Counters, Phase2State, SENSOR_RANGE_MAX_M,
};
use crate::rng::{RngSystem, named_random};
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
const MAX_EVENTS_PER_TICK: usize = 4_096;

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
}

impl DeathCause {
    pub fn name(self) -> &'static str {
        match self {
            DeathCause::Starvation => "starvation",
            DeathCause::OldAge => "old_age",
        }
    }
}

/// Event payloads. `EVENT_SCHEMA_VERSION` 1 covered Birth/Death/
/// CapacityRejected/Extinction; version 2 adds the Phase 2 variants with
/// bounded audit payloads. Reading events never alters simulation state.
pub const EVENT_SCHEMA_VERSION: u32 = 2;

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
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InvariantViolation {
    EntityOrder,
    PositionInvalid { id: u64 },
    EnergyOutOfBounds { id: u64, energy_milli: i64 },
    AgeOutOfBounds { id: u64, age_ticks: u64 },
    BiomassOutOfBounds { cell: usize, biomass_milli: i64 },
    EnergyLedgerMismatch { expected: i128, actual: i128 },
    BiomassLedgerMismatch { expected: i128, actual: i128 },
    PopulationAccounting { expected: i128, actual: i128 },
    EntityIdAllocation { expected: u64, actual: u64 },
    Phase2Desync { organisms: usize, phase2: usize },
    InvalidGenome { id: u64 },
    AncestryInvalid { id: u64 },
    ControllerStateInvalid { id: u64 },
}

impl fmt::Display for InvariantViolation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for InvariantViolation {}

#[derive(Clone, Debug)]
pub enum NewWorldError {
    Config(ConfigError),
    WorldGen(WorldGenError),
}

impl fmt::Display for NewWorldError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(error) => write!(formatter, "invalid config: {error}"),
            Self::WorldGen(error) => write!(formatter, "world generation failed: {error}"),
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
            config,
        };

        world.spawn_initial_population();
        if world.config.phase2.enabled {
            let mut state = Phase2State::with_capacity(world.ids.len());
            for index in 0..world.ids.len() {
                let id = world.ids[index];
                let genome = Genome::founder(world.config.world_seed, id);
                let genome_hash = genome.stable_hash();
                let phenotype = Phenotype::derive(&genome);
                let heading = (named_random(world.config.world_seed, 0, RngSystem::Spawn, id, 3)
                    & 0xffff) as u16;
                state.push_organism(genome, genome_hash, phenotype, heading, [0, 0], 0, 0);
            }
            world.phase2 = Some(state);
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

    pub(crate) fn organism_ids(&self) -> &[u64] {
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
                    let traits = p2.genomes[index].traits();
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
            trait_genes: *p2.genomes[index].traits(),
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
        observer.phase_finished(TickPhase::Apply);

        observer.phase_started(TickPhase::Lifecycle);
        self.lifecycle(next_tick);
        observer.phase_finished(TickPhase::Lifecycle);

        observer.phase_started(TickPhase::Finalize);
        self.tick = next_tick;
        observer.phase_finished(TickPhase::Finalize);
    }

    /// Logistic regrowth with a one-milli seeding floor so grazed cells
    /// recover (policy v1). Exact integer arithmetic; ledger-recorded.
    fn grow_food(&mut self) {
        let rate = i128::from(self.config.growth_rate_q16_per_s);
        let dt = i128::from(self.config.dt_ms);
        for index in 0..self.biomass_milli.len() {
            let capacity = self.terrain.capacity_milli[index];
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

        for index in 0..population {
            let inputs = &mut p2.inputs[index];
            let phenotype = &p2.phenotypes[index];

            // 1: energy fraction; 2: health neutral; 3: age fraction.
            inputs[0] = self.energy_milli[index] as f32 / self.config.energy_max_milli as f32;
            inputs[1] = 1.0;
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

        for index in 0..population {
            let output = controller::evaluate(&p2.genomes[index], &p2.inputs[index]);
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
            // OUT_ATTACK is a documented no-op in Phase 2.
            p2.next_memory[index] = controller::next_memory(&output);
        }
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

        // Cost pass: metabolism scales with the genome-derived multipliers.
        for (index, &did_move) in moved.iter().enumerate() {
            let phenotype = &p2.phenotypes[index];
            let mut cost = self.basal_cost_tick * phenotype.basal_mult_milli / 1000;
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
                        if p2.genomes[index].normalized_distance(&p2.genomes[candidate], 0)
                            > compatibility
                        {
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
            let (genome, variation): (Genome, VariationSummary) = recombine(
                &p2.genomes[index],
                &p2.genomes[partner],
                policy,
                self.config.world_seed,
                next_tick,
                child_id,
            );
            let genome_hash = genome.stable_hash();
            let phenotype = Phenotype::derive(&genome);
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

    fn lifecycle(&mut self, next_tick: u64) {
        // Death marks in stable order. Starvation is checked before age
        // (documented tie policy).
        let population = self.ids.len();
        let mut dead = vec![false; population];
        for (index, dead_flag) in dead.iter_mut().enumerate() {
            let cause = if self.energy_milli[index] <= 0 {
                Some(DeathCause::Starvation)
            } else if self.age_ticks[index] >= self.config.max_age_ticks {
                Some(DeathCause::OldAge)
            } else {
                None
            };
            let Some(cause) = cause else { continue };
            *dead_flag = true;
            match cause {
                DeathCause::Starvation => self.counters.deaths_starvation_total += 1,
                DeathCause::OldAge => self.counters.deaths_old_age_total += 1,
            }
            self.ledger.removed_at_death_milli += i128::from(self.energy_milli[index]);
            let id = self.ids[index];
            self.push_event(next_tick, EventKind::Death { id, cause });
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
            self.counters.births_total += 1;
            self.push_event(next_tick, EventKind::Birth { id, parent_id });
        }

        // Phase 2 paired births. Children act starting next tick.
        if let Some(mut p2) = self.phase2.take() {
            let pending: Vec<PendingChild> = std::mem::take(&mut p2.pending);
            for child in pending {
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
            if self.age_ticks[index] >= self.config.max_age_ticks {
                return Err(InvariantViolation::AgeOutOfBounds {
                    id: self.ids[index],
                    age_ticks: self.age_ticks[index],
                });
            }
        }
        for (cell, &biomass) in self.biomass_milli.iter().enumerate() {
            if biomass < 0 || biomass > self.terrain.capacity_milli[cell] {
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
        let expected_biomass = self.ledger.initial_biomass_milli + self.ledger.grown_milli
            - self.ledger.consumed_biomass_milli;
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
        let expected_population = i128::from(self.config.initial_organisms)
            + i128::from(self.counters.births_total)
            - i128::from(self.counters.deaths_starvation_total)
            - i128::from(self.counters.deaths_old_age_total);
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
