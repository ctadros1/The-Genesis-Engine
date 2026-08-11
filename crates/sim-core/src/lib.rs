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

mod actioncensus;
mod checksum;
mod climate;
mod config;
mod contest;
mod controller;
mod controller2;
mod develop;
mod genome;
mod genome2;
mod learnstate;
mod meiosis;
mod morphology;
mod morphstate;
mod origin;
mod phase2;
mod physiology;
mod plasticity;
mod registry;
mod rng;
mod save;
mod schema2;
mod similarity;
mod structmut;
mod terrainmod;
mod world;
mod worldgen;

pub use actioncensus::{
    ACTION_CENSUS_POLICY_VERSION, ACTION_CLASS_COUNT, ActionCensusCounters, ActionClass,
    LOCOMOTION_CLASS_COUNT, TURN_BAND_MILLI, locomotion as locomotion_class,
};
pub use checksum::{Fnv1a64, fnv1a64};
pub use climate::{
    BIOME_COUNT, BIOME_POLICY_VERSION, Biome, CLIMATE_POLICY_VERSION, ClimateBase, ClimateError,
    ClimateState, classify as classify_biome, drift_milli, season_milli,
};
pub use config::{
    BEHAVIOR_POLICY_VERSION, CONFIG_SCHEMA_VERSION, ClimateConfig, ConfigError, ContestConfig,
    Genome2Config, MAX_CAPACITY_SCALE_Q16, MAX_PATCH_RADIUS_CELLS, MorphologyConfig, OriginConfig,
    PHASE2_BEHAVIOR_POLICY_VERSION, Phase2Config, PhysiologyConfig, PlasticityConfig, ProbeConfig,
    SimConfig, WorldModConfig, WorldgenVersion,
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
pub use controller2::{
    ActionRequests, ActivationState, CONTROLLER2_POLICY_VERSION, CompileError, CompiledNetwork,
    IncomingEdge, NO_MODULATOR, NOT_PLASTIC, PlasticEdge, PlasticityBudget,
    commit as commit_activations, compile as compile_network,
    compile_with_budget as compile_network_with_budget, evaluate as evaluate_network, output_of,
};
pub use develop::{
    ACT_DIFFERENTIATE, ACT_PLACE, ACT_SET_SCALE, ACT_TERMINATE, ACTION_KIND_COUNT, COND_ALWAYS,
    COND_DISTANCE, COND_MODULE_COUNT, COND_NEIGHBOURS, COND_SELF_TYPE, COND_STEP, COND_TYPE_COUNT,
    CONDITION_KIND_COUNT, DEVELOP_POLICY_VERSION, DevelopCounters, OP_EQ, OP_GE, OP_LT,
    OPERATOR_COUNT, Regulatory, develop, grow, phenotypic_distance_milli, rules_of,
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
    Genome2, Genome2Error, GenomeCaps, Haplotype, Locus, LocusKind, MARKER_FLAG_NEUTRAL, PLOIDY,
    PlasticityGenes, STRUCTURAL_HOMOLOGY_BASE, TRAIT_HOMOLOGY_BASE, TRAIT_HOMOLOGY_LIMIT,
    VALUE_LIMIT, blend_by_dominance, derive_gene_lineage_id, derive_homology_id,
    derive_mutation_event_id, registry_versions,
};
pub use meiosis::{
    Gamete, InheritanceMode, MAX_EXTRA_CROSSOVERS, MEIOSIS_POLICY_VERSION, MeiosisConfig, gamete,
    recombine as recombine2,
};
pub use morphology::{
    Body, DerivedBody, LatticeKind, LatticePos, MAX_SCALE_MILLI, MIN_SCALE_MILLI,
    MODULE_REGISTRY_VERSION, MODULE_TYPE_COUNT, MORPHOLOGY_POLICY_VERSION, Module, ModuleType,
    MorphologyCaps, TypeEntry, ViabilityFailure, registry_entry,
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
pub use plasticity::{
    EdgeSignals, LEARN_LIMIT_Q16, LearnedState, ONE_Q16, PLASTICITY_POLICY_VERSION,
    PlasticityCounters, PlasticityRule, RULE_COUNT, RULE_ELIGIBILITY_TRACE, RULE_HEBBIAN,
    RULE_MODULATED_HEBBIAN, RULE_OJA, RULE_REGISTRY_VERSION, RULE_STATIC, StepKind, StepOutcome,
    accumulate_clamped, decay_to_q16, decay_toward_zero, effective_weight, q16_to_f32,
    rule_in_registry, rule_is_modulated, step as plasticity_step, to_q16, to_q16_checked,
};
pub use registry::{
    ACTIVATION_LINEAR, ACTIVATION_REGISTRY_VERSION, ACTIVATION_TANH, Activation,
    CHANNEL_REGISTRY_VERSION, CHANNELS, ChannelDirection, ChannelEntry, NodeRole, channel,
    channel_exists, input_channels, output_channels,
};
pub use rng::{RNG_ALGORITHM_VERSION, RngSystem, named_random};
pub use save::{
    ActionCensusSaveState, ClimateSaveState, ContestSaveState, LearnSaveState, LearnedEdgeSave,
    MorphologySaveState, Phase2SaveState, PhysiologySaveState, RestoreError, SAVE_STATE_VERSION,
    SaveState, Schema2SaveState,
};
pub use schema2::{
    ACTION_CHANNELS, MARKER_HOMOLOGY_ID, SENSE_CHANNELS, compatibility_distance,
    founder_from_traits, founder_with_morphology, outputs_from_requests, with_marker_locus,
};
pub use similarity::{SIMILARITY_ALGORITHM_VERSION, SimilarityReport, analyze};
pub use structmut::{
    DUPLICATE_SPAN, MutationConfig, MutationCounters, MutationReport, OP_DELETION, OP_DUPLICATION,
    OP_INSERTION, OP_POINT, OP_TRANSPOSITION, RejectReason, STRUCTMUT_POLICY_VERSION,
    minimal_founder, mutate,
};
pub use terrainmod::{
    LAYER_CAPACITY_SCALE, LAYER_COUNT, LAYER_MATERIAL_YIELD, LAYER_TRAVERSABLE, ModOutcome,
    TerrainModCounters, TerrainModState, WORLDMOD_POLICY_VERSION, scale_capacity, value_in_domain,
};
pub use world::{
    ActionSample, Counters, DeathCause, EVENT_SCHEMA_VERSION, Event, EventKind, InvariantViolation,
    LearnedSample, Ledger, MAX_EVENTS_PER_TICK, MarkerSample, MetricsSnapshot, MorphologySample,
    NewWorldError, NoopObserver, OrganismDetail, Phase2Detail, RenderEntity, StructureSample,
    TickObserver, TickPhase, World,
};
pub use worldgen::{Terrain, WORLDGEN_VERSION, WorldGenError, generate as generate_terrain};

/// Fixed-point sub-units per simulation meter for continuous positions.
pub const FP_PER_METER: i32 = 1024;

/// Milli-units per Energy Unit / biomass unit.
pub const MILLI: i64 = 1000;
