//! Deterministic headless simulation kernel (Phases 1 and 2).
//!
//! Invariants owned by this crate:
//! - No wall-clock, filesystem, network, database, GPU, or UI dependency.
//! - World state uses fixed-point integer arithmetic. Phase 2 adds bounded
//!   f32 genome/controller values evaluated with add/multiply/divide and a
//!   rational activation approximation only (no libm transcendentals), so
//!   same-build replay remains exact on the recorded platform.
//! - All randomness derives from named streams keyed by
//!   `(world_seed, tick, system, subject, draw_index)`.
//! - Order-sensitive work iterates organisms in stable entity-ID order.
//! - Energy and biomass change only through the explicit ledger paths.
//! - Malformed parameter records are rejected with typed errors and never
//!   repaired or admitted into world state.
//!
//! Behavioral formulas are versioned experimental policy
//! (`phase1-behavior-v1`, `phase2-behavior-v1`), not permanent doctrine.
//! See `docs/04-simulation-model.md`, `docs/07-neural-network-design.md`,
//! and `docs/08-genetics-and-evolution.md`.

mod checksum;
mod climate;
mod config;
mod contest;
mod controller;
mod genome;
mod genome2;
mod origin;
mod phase2;
mod physiology;
mod registry;
mod rng;
mod save;
mod similarity;
mod world;
mod worldgen;

pub use checksum::{Fnv1a64, fnv1a64};
pub use climate::{
    BIOME_COUNT, BIOME_POLICY_VERSION, Biome, CLIMATE_POLICY_VERSION, ClimateBase, ClimateError,
    ClimateState, classify as classify_biome, drift_milli, season_milli,
};
pub use config::{
    BEHAVIOR_POLICY_VERSION, CONFIG_SCHEMA_VERSION, ClimateConfig, ConfigError, ContestConfig,
    OriginConfig, PHASE2_BEHAVIOR_POLICY_VERSION, Phase2Config, PhysiologyConfig, SimConfig,
    WorldgenVersion,
};
pub use contest::{
    CONTEST_POLICY_VERSION, Carcass, ContestState, PAIR_KEY_POLICY_VERSION, pair_key,
};
pub use controller::{
    CONTROLLER_POLICY_VERSION, ControllerOutput, OUT_ATTACK, OUT_AVOID, OUT_EAT, OUT_FOLLOW,
    OUT_MATE, OUT_MEMORY_BASE, OUT_REST, OUT_THROTTLE, OUT_TURN, cos_bam_q15,
    evaluate as evaluate_controller, next_memory as controller_next_memory, sin_bam_q15,
    tanh_approx,
};
pub use genome::{
    CONTROLLER_INPUTS, CONTROLLER_OUTPUTS, ENCODED_LEN as GENOME_ENCODED_LEN,
    GENE_APPROACH_TENDENCY, GENE_BODY_SCALE, GENE_DEFENSE_TENDENCY, GENE_DIET_AFFINITY,
    GENE_MATURITY, GENE_METABOLISM, GENE_PIGMENT_HUE, GENE_PIGMENT_PATTERN, GENE_REPRO_COOLDOWN,
    GENE_REPRO_INVESTMENT, GENE_SENSOR_RANGE, GENE_SENSOR_SENSITIVITY, GENE_SPEED_POTENTIAL,
    GENE_THERMAL_PREFERENCE, GENOME_POLICY_VERSION, GENOME_SCHEMA_VERSION, Genome, GenomeError,
    MEMORY_VALUES, NEURAL_COUNT, Phenotype, TOPOLOGY_ID, TRAIT_COUNT, VariationPolicy,
    VariationSummary, WEIGHT_LIMIT, recombine,
};
pub use genome2::{
    EDGE_FLAG_DELAYED, EDGE_FLAG_DISABLED, EDGE_FLAG_PLASTIC, ExpressedBinding, ExpressedEdge,
    ExpressedNetwork, ExpressedNode, GENOME2_MAGIC, GENOME2_POLICY_VERSION, GENOME2_SCHEMA_VERSION,
    Genome2, Genome2Error, GenomeCaps, Haplotype, Locus, LocusKind, PLOIDY, PlasticityGenes,
    STRUCTURAL_HOMOLOGY_BASE, TRAIT_HOMOLOGY_BASE, TRAIT_HOMOLOGY_LIMIT, VALUE_LIMIT,
    blend_by_dominance, derive_gene_lineage_id, derive_homology_id, derive_mutation_event_id,
    registry_versions,
};
pub use origin::{
    Archetype, Founder, MAX_ARCHETYPES, MAX_DEMES, ORIGIN_POLICY_VERSION, OriginError, OriginMode,
    affinity_biomes, all_biomes_mask, is_default_origin, mean_trait_distance,
};
pub use phase2::{PairRejectReason, Phase2Counters, SENSOR_RANGE_MAX_M};
pub use physiology::{
    HazardOutcome, PHYSIOLOGY_POLICY_VERSION, PhysiologyState, allometry_multiplier_milli,
    hazard_draw, pow_quarter_milli, preferred_temperature_milli, senescence_hazard_q16_per_s,
    thermal_cost_milli,
};
pub use registry::{
    ACTIVATION_LINEAR, ACTIVATION_REGISTRY_VERSION, ACTIVATION_TANH, Activation,
    CHANNEL_REGISTRY_VERSION, CHANNELS, ChannelDirection, ChannelEntry, NodeRole, channel,
    channel_exists, input_channels, output_channels,
};
pub use rng::{RNG_ALGORITHM_VERSION, RngSystem, named_random};
pub use save::{
    ClimateSaveState, ContestSaveState, Phase2SaveState, PhysiologySaveState, RestoreError,
    SAVE_STATE_VERSION, SaveState,
};
pub use similarity::{SIMILARITY_ALGORITHM_VERSION, SimilarityReport, analyze};
pub use world::{
    Counters, DeathCause, EVENT_SCHEMA_VERSION, Event, EventKind, InvariantViolation, Ledger,
    MAX_EVENTS_PER_TICK, MetricsSnapshot, NewWorldError, NoopObserver, OrganismDetail,
    Phase2Detail, RenderEntity, TickObserver, TickPhase, World,
};
pub use worldgen::{Terrain, WORLDGEN_VERSION, WorldGenError, generate as generate_terrain};

/// Fixed-point sub-units per simulation meter for continuous positions.
pub const FP_PER_METER: i32 = 1024;

/// Milli-units per Energy Unit / biomass unit.
pub const MILLI: i64 = 1000;
