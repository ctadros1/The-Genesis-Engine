//! Versioned experiment configuration for the Phase 1 minimum simulation.
//!
//! Every field is part of the canonical config hash. Changing a default or a
//! formula constant here creates a new experiment lineage; never silently
//! reinterpret an old run.

use crate::checksum::Fnv1a64;
use std::fmt;

/// Bumped whenever the meaning or set of config fields changes.
pub const CONFIG_SCHEMA_VERSION: u32 = 1;

/// Version tag for the Phase 1 behavioral rule set (movement, feeding,
/// crowding, reproduction, death). Included in the config hash.
pub const BEHAVIOR_POLICY_VERSION: &str = "phase1-behavior-v1";

/// Version tag for the Phase 2 behavioral rule set (inherited controllers,
/// heading/throttle movement, paired-parent reproduction, ancestry).
/// Included in the config hash only when `phase2.enabled` is true; enabling
/// Phase 2 always starts a new replay lineage.
pub const PHASE2_BEHAVIOR_POLICY_VERSION: &str = "phase2-behavior-v1";

/// Q16 fixed-point one (65536 == 1.0).
pub const Q16_ONE: u32 = 65536;

/// Hard process-safety ceiling independent of configuration.
pub const ABSOLUTE_MAX_ENTITIES: u32 = 200_000;

const MAX_CELLS_PER_AXIS: u32 = 4_096;

/// Complete Phase 1 configuration. All values are experimental policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SimConfig {
    pub world_seed: u64,
    /// Raster cells along x/y. World meters = cells * cell_size_m.
    pub cells_x: u32,
    pub cells_y: u32,
    pub cell_size_m: u32,
    pub initial_organisms: u32,
    /// Process safety ceiling; births beyond it are rejected, never culled.
    pub max_entities: u32,
    /// Fixed tick length in simulated milliseconds.
    pub dt_ms: u32,

    // Resource policy (logistic regrowth).
    /// Logistic growth rate r per simulated second, Q16.
    pub growth_rate_q16_per_s: u32,
    /// Carrying capacity K of the most suitable cell, milli-biomass.
    pub cell_capacity_milli: i64,
    /// Initial biomass as a Q16 fraction of each cell's capacity.
    pub initial_biomass_q16: u32,

    // Organism energetics.
    pub energy_max_milli: i64,
    pub initial_energy_milli: i64,
    pub basal_cost_milli_per_s: i64,
    /// Movement cost per simulated second while moving at cruise speed.
    pub move_cost_milli_per_s: i64,
    pub intake_rate_milli_per_s: i64,
    /// Assimilation efficiency, Q16 fraction in (0, 1].
    pub assimilation_q16: u32,
    /// Cruise speed in meters per simulated second, Q16.
    pub speed_mps_q16: u32,

    // Crowding (competition proxy; exercises the spatial index).
    pub crowding_radius_m: u32,
    pub crowding_threshold: u32,
    pub crowding_cost_milli_per_s: i64,

    // Lifecycle.
    pub maturity_age_ticks: u64,
    pub max_age_ticks: u64,
    /// Asexual reproduction gate; disabled when `reproduction_enabled` is 0.
    pub reproduction_enabled: bool,
    pub repro_threshold_milli: i64,
    pub offspring_energy_milli: i64,
    pub repro_overhead_milli: i64,
    pub repro_cooldown_ticks: u64,

    // World generation.
    /// Elevation at or above this Q16 threshold is land before masking.
    pub land_threshold_q16: u32,
    pub min_land_fraction_q16: u32,
    pub max_land_fraction_q16: u32,

    /// Phase 2 policy section. When `phase2.enabled` is false, the world
    /// behaves bit-identically to Phase 1 and this section is excluded from
    /// the config hash, preserving Phase 1 fixtures and replay lineages.
    pub phase2: Phase2Config,

    /// Phase 6 climate and biome section, disabled by default. Follows the
    /// same D-014 rule: excluded from the config hash and behaviorally inert
    /// when disabled, so a disabled world takes the exact Phase 1/2 code
    /// paths and reproduces both fixtures.
    pub climate: ClimateConfig,

    /// Phase 6 origin section. Excluded from the config hash while it holds
    /// its documented defaults, which are exactly the Phase 1/2 founder
    /// behavior, so both fixtures are preserved.
    pub origin: OriginConfig,

    /// Phase 7 contest section, disabled by default. Same D-014 rule: a
    /// disabled section is behaviorally inert, excluded from the config
    /// hash, and appends nothing to the checksum, so both fixtures survive.
    pub contest: ContestConfig,
    /// Phase 8 demography section, disabled by default. Same D-014 rule: a
    /// disabled section is behaviorally inert, excluded from the config
    /// hash, and appends nothing to the checksum, so every earlier fixture
    /// survives.
    pub physiology: PhysiologyConfig,
    /// Phase 9 genome schema 2 section, disabled by default. A world is
    /// schema 1 or schema 2 by config; there is no mixed-schema world and no
    /// migration between them.
    pub genome2: Genome2Config,
    pub morphology: MorphologyConfig,
    /// Phase 11 plasticity section, disabled by default. Same D-014 rule.
    ///
    /// This section is what `specifications/determinism-extensions.md` Rule 0
    /// demands of Phase 11 by name: `PlasticityGenes` and
    /// `EDGE_FLAG_PLASTIC` are already on every schema-2 edge, so an
    /// implementation that acted on them **without its own gate** would move
    /// the Phase 9 fixture while every section that was disabled stayed
    /// disabled. Disabled, no edge compiles plastic, the learn phase writes
    /// nothing, no learned state exists, and the checksum appends nothing.
    pub plasticity: PlasticityConfig,
}

/// Versioned Phase 9 genome schema 2 policy.
///
/// Enabling this replaces the genome and the controller and nothing else.
/// Every other subsystem - movement, feeding, pairing, contest, physiology -
/// runs the same code, which is what makes a schema-1 world a usable
/// baseline rather than a different simulation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Genome2Config {
    pub enabled: bool,
    pub caps: crate::genome2::GenomeCaps,
    pub meiosis: crate::meiosis::MeiosisConfig,
    pub mutation: crate::structmut::MutationConfig,
}

impl Genome2Config {
    pub fn genome2_default() -> Self {
        Self {
            enabled: false,
            caps: crate::genome2::GenomeCaps::provisional(),
            meiosis: crate::meiosis::MeiosisConfig::default(),
            mutation: crate::structmut::MutationConfig::default(),
        }
    }
}

/// Versioned Phase 10 morphology policy (`lifesim-morphology-v1`,
/// `lifesim-develop-v1`).
///
/// The seam is as narrow as Phase 9's and for the same reason (D-072):
/// enabling morphology replaces exactly **how the phenotype is computed** -
/// from a grown body rather than from trait genes - and nothing else.
/// Movement, feeding, pairing, contest, and physiology all read `Phenotype`
/// and cannot tell which produced it. If morphology also rewrote the tick,
/// C10.3's "morphological change has consequence" and C10.6's "the ecology
/// is still stable" would be comparisons between two different simulations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MorphologyConfig {
    pub enabled: bool,
    pub lattice: crate::morphology::LatticeKind,
    pub caps: crate::morphology::MorphologyCaps,
    /// Controller nodes available before any neural module is grown.
    ///
    /// Non-zero on purpose. A unicell with no neural tissue still responds
    /// to its surroundings - chemotaxis needs no neurons - and a floor of
    /// zero would make every founder non-viable, since the minimal founder
    /// network has three nodes and no body starts with a brain. Neural
    /// modules buy budget *above* this floor, which is where C10.7's
    /// coupling lives.
    pub base_node_budget: u32,
}

impl MorphologyConfig {
    pub fn morphology_default() -> Self {
        Self {
            enabled: false,
            lattice: crate::morphology::LatticeKind::Square,
            caps: crate::morphology::MorphologyCaps::provisional(),
            base_node_budget: 4,
        }
    }
}

/// Versioned Phase 11 plasticity policy (`lifesim-plasticity-v2`).
///
/// The seam is as narrow as Phase 9's and Phase 10's, and for the same reason
/// (D-072): enabling plasticity changes exactly **what an edge's weight is
/// during evaluation** - the genome weight plus a per-organism learned delta
/// instead of the genome weight alone - and adds one tick phase and one
/// energy cost. Sensing, movement, feeding, pairing, contest, and physiology
/// all run the same code and cannot tell which weight produced the intent.
///
/// # There is no reward here either
///
/// Nothing in this struct describes what an organism should learn, and
/// nothing in it can. `plastic_edge_cost_milli_per_s` is a price, not a
/// signal: it is charged for every plastic edge whatever that edge does, so
/// it cannot reward an outcome. What gates learning is the organism's own
/// modulatory node (`plasticity.rs`), and a config field that measured how
/// well an organism was doing would be the prohibited thing rather than a
/// refinement of it (`docs/02-scope-and-non-goals.md`, ADR-0014).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlasticityConfig {
    pub enabled: bool,
    /// Energy charged per plastic edge per simulated second.
    ///
    /// Nonzero on purpose. Without a cost every edge becomes plastic by
    /// drift, the trait stops being informative, and C11.2's "is plasticity
    /// under selection" has no direction it could answer in. With a cost the
    /// plastic-edge count is itself under selection and "how much plasticity
    /// does this environment pay for" becomes a measurable result.
    pub plastic_edge_cost_milli_per_s: i64,
    /// Hard ceiling on how many of one organism's expressed edges may be
    /// plastic. Edges beyond it, in ascending `homology_id` order, compile
    /// as ordinary fixed edges and are counted (`plastic_over_cap`).
    ///
    /// **Provisional.** C11.7 sets this from measurement: learned state is
    /// the per-organism snapshot term this cap bounds, and the Phase 4 record
    /// already has snapshots dominated by per-organism genome arrays with a
    /// synchronous checkpoint on the tick thread. 32 is a placeholder chosen
    /// to sit at a fifth of `GenomeCaps::provisional().max_edges` (160), not
    /// a measured budget, and it must be restated once C11.7 has numbers -
    /// exactly as the genome caps were restated once by C9.8.
    pub max_plastic_edges: u32,
    /// Q16 fraction of a parent's learned delta a child inherits.
    ///
    /// **Zero, and zero is not a tuning default.** Reset at birth is the
    /// invariant that keeps Phase 13's question meaningful: if learned state
    /// were inherited, a discovery would become a heritable trait and
    /// transmission would be indistinguishable from inheritance. A nonzero
    /// value is an explicit experimental condition that **must be reported in
    /// every result derived from such a run**, never a default and never
    /// silently enabled. The `PlasticityInit` RNG stream is reserved for the
    /// policy that would implement it, so adopting one later cannot renumber
    /// a stream.
    pub lamarckian_fraction_q16: u32,
}

impl PlasticityConfig {
    /// Documented conservative Phase 11 defaults (disabled by default).
    pub fn plasticity_default() -> Self {
        Self {
            enabled: false,
            // 2 milli-EU per plastic edge per second against a basal cost of
            // 100: a fully capped 32-edge organism pays 64, about two thirds
            // of basal. Provisional in the same sense the cap is - the pair
            // sets the selective price of plasticity and C11.2 reads the
            // answer off it, so it is a policy value to be reported, not a
            // constant to be trusted.
            plastic_edge_cost_milli_per_s: 2,
            max_plastic_edges: 32,
            lamarckian_fraction_q16: 0,
        }
    }
}

/// Versioned Phase 8 demography policy (`lifesim-demography-v1`).
///
/// Six independently gated mechanisms rather than one switch, because the
/// phase's acceptance criteria compare them against each other and a
/// campaign has to be able to turn exactly one on. All disabled is
/// behaviorally identical to Phase 7.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PhysiologyConfig {
    pub enabled: bool,

    /// Basal cost scales as body mass to `basal_exponent_quarters / 4`.
    /// The default 3 is Kleiber's 0.75. Quarters rather than a Q16
    /// fraction because a quarter-power is exactly two integer square
    /// roots and needs no transcendental (see `physiology.rs`).
    pub allometry_enabled: bool,
    pub basal_exponent_quarters: u32,

    /// Thermal preference becomes live against the Phase 6 temperature
    /// field. Inert without climate, which is a documented precondition
    /// rather than an error: a world with no temperature field has nothing
    /// for the gene to be preferred against.
    pub thermoregulation_enabled: bool,
    /// Temperatures that thermal preference 0 and 1000 map to,
    /// milli-degrees.
    pub thermal_pref_low_milli: i32,
    pub thermal_pref_high_milli: i32,
    /// Deviation tolerated for free, milli-degrees.
    pub thermal_neutral_band_milli: i32,
    /// Milli-EU per second per milli-degree of excess deviation.
    pub thermal_cost_milli_per_s_per_degree: i64,

    /// Age-dependent hazard replacing the hard `max_age_ticks` cutoff.
    pub senescence_enabled: bool,
    pub senescence_onset_ticks: u64,
    /// Age scale over which the hazard reaches its base rate.
    pub senescence_scale_ticks: u64,
    /// Weibull shape; 1..=4.
    pub senescence_power: u32,
    pub senescence_hazard_q16_per_s: u32,

    /// Non-food hazard, the mechanism that lets a population sit below its
    /// food ceiling. Zero is off.
    pub extrinsic_hazard_q16_per_s: u32,

    /// Hazard multiplier applied before maturity. `Q16_ONE` is no penalty.
    pub juvenile_hazard_multiplier_q16: u32,
}

impl PhysiologyConfig {
    /// Documented conservative Phase 8 defaults (disabled by default).
    pub fn physiology_default() -> Self {
        Self {
            enabled: false,
            allometry_enabled: true,
            basal_exponent_quarters: 3, // 0.75, Kleiber
            thermoregulation_enabled: true,
            thermal_pref_low_milli: 0,
            thermal_pref_high_milli: 40_000,
            thermal_neutral_band_milli: 6_000,
            thermal_cost_milli_per_s_per_degree: 4,
            senescence_enabled: true,
            senescence_onset_ticks: 6_000,
            senescence_scale_ticks: 12_000,
            senescence_power: 2,
            senescence_hazard_q16_per_s: 655, // 0.01 per second at scale
            extrinsic_hazard_q16_per_s: 13,   // ~0.0002 per second
            juvenile_hazard_multiplier_q16: 2 * Q16_ONE,
        }
    }
}

/// Versioned Phase 7 contest policy (`contest-behavior-v1`). Every value is
/// experimental policy, hashed only when `enabled` is true.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContestConfig {
    pub enabled: bool,
    /// Health of a body-scale-1.0 organism, milli-units.
    pub base_health_milli: i64,
    /// Damage one landed attack does before body-scale scaling and variance.
    /// Zero is condition C of the phase design: the action fires and costs
    /// energy without consequence.
    pub damage_base_milli: i64,
    /// Half-width of the damage draw, Q16.
    pub damage_variance_q16: u32,
    /// Energy an attack costs its attacker, whether or not it lands.
    pub attack_cost_milli: i64,
    /// Maximum distance at which an attack can land, meters.
    pub attack_range_m: u32,
    /// Controller output above which the attack intent fires, Q16 signed.
    pub attack_threshold_q16: i32,
    /// Ticks an organism must wait between attacks.
    pub attack_cooldown_ticks: u64,

    /// Health restored per second while healing.
    pub heal_milli_per_s: i64,
    /// Energy spent per milli-unit of health restored, Q16.
    pub heal_energy_cost_q16: u32,
    /// Healing only happens above this energy fraction, Q16.
    pub heal_energy_floor_q16: u32,
    /// Q16 fraction of accumulated recent damage that decays each second.
    pub damage_decay_q16_per_s: u32,

    /// Q16 fraction of a dead organism's energy that becomes a carcass.
    pub carcass_energy_q16: u32,
    /// Q16 fraction of a carcass's energy that decays each second.
    pub carcass_decay_q16_per_s: u32,
    /// Maximum distance at which a carcass can be eaten, meters.
    pub carcass_reach_m: u32,
    /// Maximum carcasses retained; the oldest are dropped beyond it, with
    /// the loss ledgered as decay rather than silently discarded.
    pub max_carcasses: u32,
    /// Extra biomass a feeding organism removes from its own cell, sharpening
    /// local resource contention so a patch is worth defending.
    pub local_depletion_milli: i64,
}

impl ContestConfig {
    /// Documented conservative Phase 7 defaults (disabled by default).
    pub fn contest_default() -> Self {
        Self {
            enabled: false,
            base_health_milli: 10_000,
            damage_base_milli: 1_200,
            damage_variance_q16: 16_384, // +/- 0.25
            attack_cost_milli: 120,
            attack_range_m: 3,
            attack_threshold_q16: 32_768, // 0.5
            attack_cooldown_ticks: 10,
            heal_milli_per_s: 60,
            heal_energy_cost_q16: 131_072,  // 2.0 energy per health
            heal_energy_floor_q16: 32_768,  // heal only above half energy
            damage_decay_q16_per_s: 6_554,  // 0.10 per second
            carcass_energy_q16: 45_875,     // 0.70 of remaining energy
            carcass_decay_q16_per_s: 3_277, // 0.05 per second
            carcass_reach_m: 2,
            max_carcasses: 4_096,
            local_depletion_milli: 0,
        }
    }
}

/// Versioned Phase 6 origin policy (`lifesim-origin-v1`).
///
/// The `random` defaults below are the constants Phase 1 and Phase 2 had
/// hard-coded, lifted into config without changing them: founder traits land
/// in `0.25 + u * 0.5` and founder neural weights in `+/- 0.5`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OriginConfig {
    pub mode: crate::origin::OriginMode,
    pub trait_low_q16: u32,
    pub trait_span_q16: u32,
    pub neural_span_q16: u32,
    /// Number of separated founder groups. `1` is the Phase 1/2 behavior.
    pub deme_count: u32,
    pub deme_radius_m: u32,
    pub deme_min_separation_m: u32,
    /// Half-width of a founder's draw around its deme's trait centre. Small
    /// relative to `trait_span_q16`, which is what makes within-deme genetic
    /// distance smaller than between-deme distance.
    pub deme_trait_spread_q16: u32,
    pub archetype_count: u32,
    pub archetypes: [crate::origin::Archetype; crate::origin::MAX_ARCHETYPES],
}

impl OriginConfig {
    /// Defaults reproducing the Phase 1/2 founder behavior exactly.
    pub fn origin_default() -> Self {
        Self {
            mode: crate::origin::OriginMode::Random,
            trait_low_q16: 16_384,  // 0.25
            trait_span_q16: 32_768, // 0.5
            neural_span_q16: Q16_ONE,
            deme_count: 1,
            deme_radius_m: 128,
            deme_min_separation_m: 192,
            deme_trait_spread_q16: 6_554, // 0.10
            archetype_count: 0,
            archetypes: [crate::origin::Archetype::neutral(0); crate::origin::MAX_ARCHETYPES],
        }
    }
}

/// Which world generator produced (and will regenerate) a world's terrain.
///
/// The selected version is folded into the config hash in place of a
/// constant, so a v1 world keeps its terrain checksum and both fixtures
/// **forever**: adding v2 cannot change what v1 hashes to. v1 stays in the
/// build permanently and v1 worlds never see v2.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum WorldgenVersion {
    #[default]
    V1,
    /// Adds moisture, temperature, and biome fields (Phase 6).
    V2,
}

impl WorldgenVersion {
    pub fn tag(self) -> &'static str {
        match self {
            WorldgenVersion::V1 => "lifesim-worldgen-v1",
            WorldgenVersion::V2 => "lifesim-worldgen-v2",
        }
    }
}

/// Versioned Phase 6 climate, biome, and generator policy. Every value is
/// experimental policy, hashed only when `enabled` is true.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClimateConfig {
    pub enabled: bool,
    /// Generator selection. `enabled` implies v2; a disabled section leaves
    /// this at v1 and validation enforces the pairing.
    pub worldgen_version: WorldgenVersion,

    // Static base temperature, milli-degrees.
    pub base_temperature_milli: i32,
    /// Temperature drop across the full Q16 elevation range.
    pub lapse_milli_per_full_elevation: i32,
    /// Pole-to-equator temperature difference.
    pub latitude_amplitude_milli: i32,

    // Stateless time terms.
    pub season_period_ticks: u64,
    pub season_amplitude_milli: i32,
    /// Three incommensurate drift periods, all far longer than the season.
    pub drift_period_ticks: [u64; 3],
    pub drift_amplitude_milli: [i32; 3],

    pub temperature_min_milli: i32,
    pub temperature_max_milli: i32,

    // Moisture, milli-units. The field is conserved: these values set the
    // initial distribution and how it redistributes, never a source or sink.
    pub initial_moisture_milli: i64,
    pub coastal_moisture_bonus_milli: i64,
    pub moisture_max_milli: i64,
    /// Absolute ceiling any cell may reach; validation, not a clamp.
    pub moisture_ceiling_milli: i64,
    /// How much of the initial moisture blend comes from sea proximity
    /// rather than low ground, Q16. Zero makes moisture a pure function of
    /// elevation, which collapses the biome map onto elevation bands.
    pub sea_proximity_weight_q16: u32,
    /// Fraction of a cell's moisture that leaves it each step, Q16.
    pub moisture_diffusion_q16: u32,
    /// Extra share a downhill neighbour receives (drainage).
    pub moisture_drain_weight: u32,

    // Biome thresholds (`lifesim-biome-v1`).
    pub highland_elevation_q16: u32,
    pub wetland_moisture_milli: i64,
    pub arid_moisture_milli: i64,
    pub forest_moisture_milli: i64,
    pub forest_min_temperature_milli: i32,
    /// Per-biome carrying-capacity multiplier, Q16, indexed by biome ID.
    pub biome_capacity_q16: [u32; crate::climate::BIOME_COUNT],
    /// Ticks between reclassifications. Climate moves on timescales far
    /// longer than a tick, so reclassifying every tick would be pure cost;
    /// the cadence is versioned policy like any other formula constant.
    pub reclassify_interval_ticks: u64,
}

impl ClimateConfig {
    /// Documented conservative Phase 6 defaults (disabled by default).
    pub fn climate_default() -> Self {
        Self {
            enabled: false,
            worldgen_version: WorldgenVersion::V1,
            base_temperature_milli: 22_000,
            lapse_milli_per_full_elevation: 30_000,
            latitude_amplitude_milli: 28_000,
            season_period_ticks: 36_000,
            season_amplitude_milli: 6_000,
            // Pairwise-coprime primes: the sum is quasi-periodic rather than
            // repeating on any short cycle.
            drift_period_ticks: [1_000_003, 410_009, 173_021],
            drift_amplitude_milli: [7_000, 3_000, 1_500],
            temperature_min_milli: -60_000,
            temperature_max_milli: 60_000,
            initial_moisture_milli: 120_000,
            coastal_moisture_bonus_milli: 40_000,
            moisture_max_milli: 200_000,
            moisture_ceiling_milli: 4_000_000,
            sea_proximity_weight_q16: 39_322, // 0.60 sea proximity, 0.40 relief
            moisture_diffusion_q16: 3_277,    // 0.05 of a cell per step
            moisture_drain_weight: 2,
            // Calibrated against measured field distributions rather than
            // guessed: across seven seeds at 96x96, land elevation spans
            // roughly p0 17,000 to p100 35,500-53,300, and inland moisture
            // spans p0 18,500-37,000 to p100 111,500-115,600. Each threshold
            // sits inside the range every seed reaches, so no biome is
            // unreachable by construction.
            highland_elevation_q16: 30_000,
            wetland_moisture_milli: 105_000,
            arid_moisture_milli: 55_000,
            forest_moisture_milli: 90_000,
            forest_min_temperature_milli: 4_000,
            // Water contributes nothing; wetland and forest are the most
            // productive; arid and highland the least.
            biome_capacity_q16: [
                0,      // water
                72_000, // coast
                26_214, // highland  (0.40)
                98_304, // wetland   (1.50)
                19_661, // arid      (0.30)
                85_197, // forest    (1.30)
                65_536, // grassland (1.00)
            ],
            reclassify_interval_ticks: 100,
        }
    }
}

/// Versioned Phase 2 policy: inherited controllers, paired-parent
/// reproduction, and offline similarity analysis. All values are
/// experimental simulation policy, hashed only when `enabled` is true.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Phase2Config {
    pub enabled: bool,
    /// Per-gene variation probability, Q16.
    pub variation_probability_q16: u32,
    /// Maximum trait-gene delta, Q16 gene units.
    pub variation_trait_sigma_q16: u32,
    /// Maximum neural-gene delta as a Q16 fraction of the weight limit.
    pub variation_neural_sigma_q16: u32,
    /// Maximum pairing distance in meters.
    pub pairing_range_m: u32,
    /// Maximum normalized trait distance for a compatible pair, Q16.
    pub compatibility_threshold_q16: u32,
    /// Energy readiness gate for pairing intent, milli-EU.
    pub pairing_energy_threshold_milli: i64,
    /// Non-transferred energy each parent spends per accepted pairing.
    pub pairing_overhead_milli: i64,
    /// Controller output thresholds, Q16 signed (output range [-1, 1]).
    pub eat_threshold_q16: i32,
    pub mate_threshold_q16: i32,
    pub rest_threshold_q16: i32,
    /// Maximum heading change per tick, BAM units (65536 per full turn).
    pub max_turn_per_tick_bam: u32,
    /// Offline similarity analysis: cluster threshold and sampling bound.
    pub cluster_threshold_q16: u32,
    pub cluster_sample_max: u32,
    /// Q16 weight of controller-parameter distance in similarity analysis
    /// (0 omits controller parameters, the documented starting policy).
    pub cluster_neural_weight_q16: u32,
}

impl Phase2Config {
    /// Documented conservative Phase 2 defaults (disabled by default).
    pub fn phase2_default() -> Self {
        Self {
            enabled: false,
            variation_probability_q16: 1_311,  // 0.02
            variation_trait_sigma_q16: 3_277,  // 0.05 gene units
            variation_neural_sigma_q16: 3_277, // 0.05 * 8.0 = 0.4 weight units
            pairing_range_m: 4,
            compatibility_threshold_q16: 32_768, // 0.5 normalized distance
            pairing_energy_threshold_milli: 7_000,
            pairing_overhead_milli: 500,
            // Permissive defaults keep founder populations viable while the
            // gates stay evolvable: an output below the threshold still
            // suppresses the action.
            eat_threshold_q16: -32_768,    // eat unless output < -0.5
            mate_threshold_q16: -16_384,   // mate intent unless output < -0.25
            rest_threshold_q16: 32_768,    // 0.5
            max_turn_per_tick_bam: 2_048,  // 1/32 turn per tick
            cluster_threshold_q16: 13_107, // 0.2
            cluster_sample_max: 2_048,
            cluster_neural_weight_q16: 0,
        }
    }
}

impl SimConfig {
    /// Documented Phase 1 defaults: a 1,024 m square continent world tuned
    /// for the 500-organism baseline. These are experimental values.
    pub fn phase1_default(world_seed: u64) -> Self {
        Self {
            world_seed,
            cells_x: 256,
            cells_y: 256,
            cell_size_m: 4,
            initial_organisms: 500,
            max_entities: 5_000,
            dt_ms: 100,
            growth_rate_q16_per_s: 3_277, // 0.05 per second
            cell_capacity_milli: 30_000,
            initial_biomass_q16: 32_768, // 0.5
            energy_max_milli: 12_000,
            initial_energy_milli: 8_000,
            basal_cost_milli_per_s: 100,
            move_cost_milli_per_s: 500,
            intake_rate_milli_per_s: 2_000,
            assimilation_q16: 45_875, // 0.7
            speed_mps_q16: 131_072,   // 2.0 m/s
            crowding_radius_m: 6,
            crowding_threshold: 4,
            crowding_cost_milli_per_s: 200,
            maturity_age_ticks: 600,
            max_age_ticks: 36_000,
            reproduction_enabled: true,
            repro_threshold_milli: 10_000,
            offspring_energy_milli: 4_000,
            repro_overhead_milli: 1_000,
            repro_cooldown_ticks: 300,
            land_threshold_q16: 17_000,
            min_land_fraction_q16: 6_554,  // 0.10
            max_land_fraction_q16: 58_982, // 0.90
            phase2: Phase2Config::phase2_default(),
            climate: ClimateConfig::climate_default(),
            origin: OriginConfig::origin_default(),
            contest: ContestConfig::contest_default(),
            physiology: PhysiologyConfig::physiology_default(),
            genome2: Genome2Config::genome2_default(),
            morphology: MorphologyConfig::morphology_default(),
            plasticity: PlasticityConfig::plasticity_default(),
        }
    }

    /// Phase 11 defaults: the schema-2 world with plasticity live.
    ///
    /// Both flags, because either alone is a condition rather than the
    /// treatment. `plasticity.enabled` without
    /// `genome2.mutation.plasticity_enabled` is a world where plasticity runs
    /// but no organism's plasticity genes can ever change - which is exactly
    /// condition B, not condition A - and the mutation flag without the
    /// section is genes that mutate and never act.
    pub fn phase11_default(world_seed: u64) -> Self {
        let mut config = Self::phase2_default(world_seed);
        config.genome2.enabled = true;
        config.genome2.mutation.plasticity_enabled = true;
        config.plasticity.enabled = true;
        config
    }

    /// Phase 7 defaults: the Phase 2 world with contest enabled.
    pub fn phase7_default(world_seed: u64) -> Self {
        let mut config = Self::phase2_default(world_seed);
        config.contest.enabled = true;
        config
    }

    /// Phase 6 defaults: the Phase 2 world with climate, biomes, and the
    /// `lifesim-worldgen-v2` generator enabled.
    pub fn phase6_default(world_seed: u64) -> Self {
        let mut config = Self::phase2_default(world_seed);
        config.climate.enabled = true;
        config.climate.worldgen_version = WorldgenVersion::V2;
        config
    }

    /// Phase 2 defaults: the Phase 1 world with inherited controllers and
    /// paired-parent reproduction enabled (`phase2-behavior-v1`).
    pub fn phase2_default(world_seed: u64) -> Self {
        let mut config = Self::phase1_default(world_seed);
        config.phase2.enabled = true;
        config
    }

    /// Validate ranges and cross-field consistency. All construction paths
    /// must call this before building a world.
    pub fn validate(&self) -> Result<(), ConfigError> {
        let fraction_fields = [
            ("initial_biomass_q16", self.initial_biomass_q16),
            ("assimilation_q16", self.assimilation_q16),
            ("land_threshold_q16", self.land_threshold_q16),
            ("min_land_fraction_q16", self.min_land_fraction_q16),
            ("max_land_fraction_q16", self.max_land_fraction_q16),
        ];
        for (name, value) in fraction_fields {
            if value > Q16_ONE {
                return Err(ConfigError::FractionOutOfRange(name, value));
            }
        }
        if self.cells_x < 8
            || self.cells_y < 8
            || self.cells_x > MAX_CELLS_PER_AXIS
            || self.cells_y > MAX_CELLS_PER_AXIS
        {
            return Err(ConfigError::WorldDimensions(self.cells_x, self.cells_y));
        }
        if self.cell_size_m == 0 || self.cell_size_m > 64 {
            return Err(ConfigError::CellSize(self.cell_size_m));
        }
        // World extent must fit i32 fixed-point.
        let extent_fp = i64::from(self.cells_x.max(self.cells_y))
            * i64::from(self.cell_size_m)
            * i64::from(crate::FP_PER_METER);
        if extent_fp > i64::from(i32::MAX) {
            return Err(ConfigError::WorldDimensions(self.cells_x, self.cells_y));
        }
        if self.max_entities == 0 || self.max_entities > ABSOLUTE_MAX_ENTITIES {
            return Err(ConfigError::MaxEntities(self.max_entities));
        }
        if self.initial_organisms == 0 || self.initial_organisms > self.max_entities {
            return Err(ConfigError::InitialOrganisms(self.initial_organisms));
        }
        if self.dt_ms == 0 || self.dt_ms > 10_000 {
            return Err(ConfigError::TickLength(self.dt_ms));
        }
        if self.assimilation_q16 == 0 {
            return Err(ConfigError::FractionOutOfRange(
                "assimilation_q16",
                self.assimilation_q16,
            ));
        }
        if self.cell_capacity_milli <= 0 {
            return Err(ConfigError::NonPositive("cell_capacity_milli"));
        }
        if self.energy_max_milli <= 0 {
            return Err(ConfigError::NonPositive("energy_max_milli"));
        }
        if self.initial_energy_milli <= 0 || self.initial_energy_milli > self.energy_max_milli {
            return Err(ConfigError::InitialEnergy(self.initial_energy_milli));
        }
        let non_negative = [
            ("basal_cost_milli_per_s", self.basal_cost_milli_per_s),
            ("move_cost_milli_per_s", self.move_cost_milli_per_s),
            ("intake_rate_milli_per_s", self.intake_rate_milli_per_s),
            ("crowding_cost_milli_per_s", self.crowding_cost_milli_per_s),
        ];
        for (name, value) in non_negative {
            if value < 0 {
                return Err(ConfigError::Negative(name));
            }
        }
        if self.speed_mps_q16 == 0 || self.speed_mps_q16 > 20 * Q16_ONE {
            return Err(ConfigError::Speed(self.speed_mps_q16));
        }
        if self.crowding_radius_m == 0 || self.crowding_radius_m > 64 {
            return Err(ConfigError::CrowdingRadius(self.crowding_radius_m));
        }
        if self.max_age_ticks == 0 || self.max_age_ticks <= self.maturity_age_ticks {
            return Err(ConfigError::AgePolicy {
                maturity: self.maturity_age_ticks,
                max_age: self.max_age_ticks,
            });
        }
        if self.reproduction_enabled {
            if self.offspring_energy_milli <= 0 || self.repro_overhead_milli < 0 {
                return Err(ConfigError::NonPositive("offspring_energy_milli"));
            }
            let total_cost = self
                .offspring_energy_milli
                .saturating_add(self.repro_overhead_milli);
            if self.repro_threshold_milli < total_cost {
                return Err(ConfigError::ReproductionEnergy {
                    threshold: self.repro_threshold_milli,
                    total_cost,
                });
            }
            if self.repro_threshold_milli > self.energy_max_milli {
                return Err(ConfigError::ReproductionEnergy {
                    threshold: self.repro_threshold_milli,
                    total_cost,
                });
            }
        }
        if self.min_land_fraction_q16 >= self.max_land_fraction_q16 {
            return Err(ConfigError::LandFractionBounds {
                min: self.min_land_fraction_q16,
                max: self.max_land_fraction_q16,
            });
        }
        if self.phase2.enabled {
            self.validate_phase2()?;
        }
        self.validate_climate()?;
        self.validate_origin()?;
        self.validate_contest()?;
        self.validate_subsystems()?;
        Ok(())
    }

    /// Validation for the subsystems that are not contest.
    ///
    /// **These checks used to live inside `validate_contest`**, which
    /// early-returns when the contest section is disabled - so every
    /// genome2 and physiology cap check was silently skipped in any world
    /// without contest, which is most of them. The blocks were appended to
    /// the wrong function over successive phases and nothing failed,
    /// because a skipped validation looks exactly like a passing one.
    fn validate_subsystems(&self) -> Result<(), ConfigError> {
        if self.genome2.enabled {
            if !self.phase2.enabled {
                return Err(ConfigError::PhysiologyRange("genome2 requires phase2", 0));
            }
            let caps = &self.genome2.caps;
            if caps.max_chromosomes == 0
                || caps.max_loci_per_chromosome == 0
                || caps.max_nodes == 0
                || caps.max_edges_per_node == 0
                || caps.max_genome_bytes == 0
                || caps.min_nodes == 0
            {
                return Err(ConfigError::PhysiologyRange("genome2 cap is zero", 0));
            }
        }
        if self.morphology.enabled {
            // Morphology is expressed from schema-2 loci, so it cannot run
            // without the genome that carries them. Refused rather than
            // silently ignored: a config that asks for bodies and gets none
            // would report morphology metrics of zero and read as a null.
            if !self.genome2.enabled {
                return Err(ConfigError::PhysiologyRange(
                    "morphology requires genome2",
                    0,
                ));
            }
            let caps = &self.morphology.caps;
            if caps.max_modules == 0 || caps.lattice_radius <= 0 || caps.max_growth_steps == 0 {
                return Err(ConfigError::PhysiologyRange("morphology cap is zero", 0));
            }
        }
        // Phase 11. **Inside `validate_subsystems`, never appended to
        // `validate_contest`** - that function early-returns on a disabled
        // contest section, and D-084 records what appending checks there
        // cost: three phases of cap validation that never ran in any world
        // without contest, which is most of them.
        let plasticity = &self.plasticity;
        if plasticity.enabled {
            // Plastic edges live on schema-2 edge loci and are gated by
            // schema-2 modulatory nodes, so there is nothing for the section
            // to act on without genome2. Refused rather than silently
            // ignored, for the reason morphology is: a config that asks for
            // learning and gets none would report plasticity metrics of zero
            // and read as C11.1's null.
            if !self.genome2.enabled {
                return Err(ConfigError::PhysiologyRange(
                    "plasticity requires genome2",
                    0,
                ));
            }
            if plasticity.plastic_edge_cost_milli_per_s < 0 {
                return Err(ConfigError::Negative("plastic_edge_cost_milli_per_s"));
            }
            if plasticity.max_plastic_edges == 0 {
                return Err(ConfigError::PhysiologyRange("max_plastic_edges is zero", 0));
            }
            // A plastic-edge cap above the structural edge cap could never
            // bind, which would make the field look enforced while being
            // decorative - and C11.7 is going to set this number from a
            // measurement of what it actually bounds.
            if plasticity.max_plastic_edges > self.genome2.caps.max_edges {
                return Err(ConfigError::PhysiologyRange(
                    "max_plastic_edges exceeds genome2.caps.max_edges",
                    i64::from(plasticity.max_plastic_edges),
                ));
            }
            if plasticity.lamarckian_fraction_q16 > Q16_ONE {
                return Err(ConfigError::FractionOutOfRange(
                    "lamarckian_fraction_q16",
                    plasticity.lamarckian_fraction_q16,
                ));
            }
            // Nonzero Lamarckian inheritance is a declared experimental
            // condition with a reporting obligation attached, and **no
            // policy implements it yet**. Accepting the value silently would
            // produce runs that look like the condition and are not it,
            // which is worse than refusing them.
            if plasticity.lamarckian_fraction_q16 != 0 {
                return Err(ConfigError::PhysiologyRange(
                    "lamarckian_fraction_q16 is nonzero but no inheritance policy is implemented; \
                     see specifications/plasticity-and-learning.md",
                    i64::from(plasticity.lamarckian_fraction_q16),
                ));
            }
        }
        let physiology = &self.physiology;
        if physiology.enabled {
            if !(1..=6).contains(&physiology.basal_exponent_quarters) {
                return Err(ConfigError::PhysiologyRange(
                    "basal_exponent_quarters",
                    i64::from(physiology.basal_exponent_quarters),
                ));
            }
            if !(1..=4).contains(&physiology.senescence_power) {
                return Err(ConfigError::PhysiologyRange(
                    "senescence_power",
                    i64::from(physiology.senescence_power),
                ));
            }
            if physiology.senescence_scale_ticks == 0 {
                return Err(ConfigError::PhysiologyRange("senescence_scale_ticks", 0));
            }
            if physiology.thermal_pref_high_milli <= physiology.thermal_pref_low_milli {
                return Err(ConfigError::PhysiologyRange(
                    "thermal_pref_high_milli",
                    i64::from(physiology.thermal_pref_high_milli),
                ));
            }
            if physiology.thermal_neutral_band_milli < 0
                || physiology.thermal_cost_milli_per_s_per_degree < 0
            {
                return Err(ConfigError::PhysiologyRange(
                    "thermal_neutral_band_milli",
                    i64::from(physiology.thermal_neutral_band_milli),
                ));
            }
        }
        Ok(())
    }

    fn validate_contest(&self) -> Result<(), ConfigError> {
        let contest = &self.contest;
        if !contest.enabled {
            return Ok(());
        }
        // Contest is a Phase 2 mechanism: it wires reserved controller
        // channels, which only exist when a controller does.
        if !self.phase2.enabled {
            return Err(ConfigError::ContestRequiresPhase2);
        }
        if contest.base_health_milli <= 0 {
            return Err(ConfigError::NonPositive("base_health_milli"));
        }
        if contest.damage_base_milli < 0 {
            return Err(ConfigError::Negative("damage_base_milli"));
        }
        if contest.attack_cost_milli < 0 {
            return Err(ConfigError::Negative("attack_cost_milli"));
        }
        if contest.local_depletion_milli < 0 {
            return Err(ConfigError::Negative("local_depletion_milli"));
        }
        if contest.attack_range_m == 0 || contest.attack_range_m > 32 {
            return Err(ConfigError::AttackRange(contest.attack_range_m));
        }
        if contest.carcass_reach_m == 0 || contest.carcass_reach_m > 32 {
            return Err(ConfigError::AttackRange(contest.carcass_reach_m));
        }
        if contest.max_carcasses == 0 || contest.max_carcasses > 200_000 {
            return Err(ConfigError::MaxCarcasses(contest.max_carcasses));
        }
        for (name, value) in [
            ("damage_variance_q16", contest.damage_variance_q16),
            ("heal_energy_floor_q16", contest.heal_energy_floor_q16),
            ("damage_decay_q16_per_s", contest.damage_decay_q16_per_s),
            ("carcass_energy_q16", contest.carcass_energy_q16),
            ("carcass_decay_q16_per_s", contest.carcass_decay_q16_per_s),
        ] {
            if value > Q16_ONE {
                return Err(ConfigError::FractionOutOfRange(name, value));
            }
        }
        if !(-(Q16_ONE as i32)..=Q16_ONE as i32).contains(&contest.attack_threshold_q16) {
            return Err(ConfigError::ControllerThreshold(
                "attack_threshold_q16",
                contest.attack_threshold_q16,
            ));
        }
        Ok(())
    }

    fn validate_origin(&self) -> Result<(), ConfigError> {
        let origin = &self.origin;
        if origin.deme_count == 0 || origin.deme_count > crate::origin::MAX_DEMES {
            return Err(ConfigError::DemeCount(origin.deme_count));
        }
        if origin.archetype_count as usize > crate::origin::MAX_ARCHETYPES {
            return Err(ConfigError::ArchetypeCount(origin.archetype_count));
        }
        for field in [
            ("trait_low_q16", origin.trait_low_q16),
            ("trait_span_q16", origin.trait_span_q16),
            ("neural_span_q16", origin.neural_span_q16),
            ("deme_trait_spread_q16", origin.deme_trait_spread_q16),
        ] {
            if field.1 > Q16_ONE {
                return Err(ConfigError::FractionOutOfRange(field.0, field.1));
            }
        }
        if origin.mode == crate::origin::OriginMode::Seeded {
            if origin.archetype_count == 0 {
                return Err(ConfigError::ArchetypeCount(0));
            }
            // Seeded placement matches against biomes, which the climate
            // section owns.
            if !self.climate.enabled {
                return Err(ConfigError::SeededRequiresClimate);
            }
            // Archetypes are sorted by ascending ID so that "allocate
            // founder IDs in ascending (archetype_id, draw_index) order" and
            // "key draws on array position" are the same ordering.
            for index in 1..origin.archetype_count as usize {
                if origin.archetypes[index].id <= origin.archetypes[index - 1].id {
                    return Err(ConfigError::ArchetypeOrder {
                        index,
                        id: origin.archetypes[index].id,
                    });
                }
            }
            for index in 0..origin.archetype_count as usize {
                if origin.archetypes[index].biome_affinity == 0 {
                    return Err(ConfigError::EmptyArchetypeAffinity {
                        id: origin.archetypes[index].id,
                    });
                }
            }
        }
        Ok(())
    }

    fn validate_climate(&self) -> Result<(), ConfigError> {
        let climate = &self.climate;
        // The generator and the climate section move together. A v2 world
        // without climate fields, or climate fields on a v1 world, would
        // both be worlds whose terrain and whose rules disagree.
        if climate.enabled != (climate.worldgen_version == WorldgenVersion::V2) {
            return Err(ConfigError::ClimateGeneratorMismatch {
                enabled: climate.enabled,
                generator: climate.worldgen_version.tag(),
            });
        }
        if !climate.enabled {
            return Ok(());
        }
        if climate.temperature_min_milli >= climate.temperature_max_milli {
            return Err(ConfigError::TemperatureBounds {
                min: climate.temperature_min_milli,
                max: climate.temperature_max_milli,
            });
        }
        for period in climate.drift_period_ticks {
            if period == 0 {
                return Err(ConfigError::NonPositive("drift_period_ticks"));
            }
        }
        if climate.season_period_ticks == 0 {
            return Err(ConfigError::NonPositive("season_period_ticks"));
        }
        // Drift must be slow relative to the season, or it is a second
        // season rather than a long-timescale term.
        for period in climate.drift_period_ticks {
            if period <= climate.season_period_ticks {
                return Err(ConfigError::DriftPeriodTooShort {
                    period,
                    season: climate.season_period_ticks,
                });
            }
        }
        if climate.sea_proximity_weight_q16 > Q16_ONE {
            return Err(ConfigError::FractionOutOfRange(
                "sea_proximity_weight_q16",
                climate.sea_proximity_weight_q16,
            ));
        }
        if climate.moisture_diffusion_q16 == 0 || climate.moisture_diffusion_q16 > Q16_ONE / 2 {
            return Err(ConfigError::FractionOutOfRange(
                "moisture_diffusion_q16",
                climate.moisture_diffusion_q16,
            ));
        }
        if climate.initial_moisture_milli <= 0 || climate.moisture_max_milli <= 0 {
            return Err(ConfigError::NonPositive("initial_moisture_milli"));
        }
        if climate.moisture_ceiling_milli < climate.moisture_max_milli {
            return Err(ConfigError::NonPositive("moisture_ceiling_milli"));
        }
        if climate.coastal_moisture_bonus_milli < 0 {
            return Err(ConfigError::Negative("coastal_moisture_bonus_milli"));
        }
        // Biome thresholds must be ordered, or a biome is unreachable by
        // construction and C6.7 would fail for a reason config caused.
        if !(climate.arid_moisture_milli < climate.forest_moisture_milli
            && climate.forest_moisture_milli < climate.wetland_moisture_milli)
        {
            return Err(ConfigError::BiomeThresholdOrder {
                arid: climate.arid_moisture_milli,
                forest: climate.forest_moisture_milli,
                wetland: climate.wetland_moisture_milli,
            });
        }
        if climate.highland_elevation_q16 > Q16_ONE {
            return Err(ConfigError::FractionOutOfRange(
                "highland_elevation_q16",
                climate.highland_elevation_q16,
            ));
        }
        if climate.reclassify_interval_ticks == 0 {
            return Err(ConfigError::NonPositive("reclassify_interval_ticks"));
        }
        if climate.biome_capacity_q16[crate::climate::Biome::Water as usize] != 0 {
            return Err(ConfigError::NonPositive("biome_capacity_q16[water]"));
        }
        Ok(())
    }

    fn validate_phase2(&self) -> Result<(), ConfigError> {
        let phase2 = &self.phase2;
        let q16_fractions = [
            (
                "variation_probability_q16",
                phase2.variation_probability_q16,
            ),
            (
                "variation_trait_sigma_q16",
                phase2.variation_trait_sigma_q16,
            ),
            (
                "variation_neural_sigma_q16",
                phase2.variation_neural_sigma_q16,
            ),
            (
                "compatibility_threshold_q16",
                phase2.compatibility_threshold_q16,
            ),
            ("cluster_threshold_q16", phase2.cluster_threshold_q16),
            (
                "cluster_neural_weight_q16",
                phase2.cluster_neural_weight_q16,
            ),
        ];
        for (name, value) in q16_fractions {
            if value > Q16_ONE {
                return Err(ConfigError::FractionOutOfRange(name, value));
            }
        }
        if phase2.pairing_range_m == 0 || phase2.pairing_range_m > 32 {
            return Err(ConfigError::PairingRange(phase2.pairing_range_m));
        }
        if phase2.pairing_energy_threshold_milli <= 0
            || phase2.pairing_energy_threshold_milli > self.energy_max_milli
        {
            return Err(ConfigError::PairingEnergy(
                phase2.pairing_energy_threshold_milli,
            ));
        }
        if phase2.pairing_overhead_milli < 0 {
            return Err(ConfigError::Negative("pairing_overhead_milli"));
        }
        let thresholds = [
            ("eat_threshold_q16", phase2.eat_threshold_q16),
            ("mate_threshold_q16", phase2.mate_threshold_q16),
            ("rest_threshold_q16", phase2.rest_threshold_q16),
        ];
        for (name, value) in thresholds {
            if !(-(Q16_ONE as i32)..=Q16_ONE as i32).contains(&value) {
                return Err(ConfigError::ControllerThreshold(name, value));
            }
        }
        if phase2.max_turn_per_tick_bam == 0 || phase2.max_turn_per_tick_bam > 16_384 {
            return Err(ConfigError::TurnRate(phase2.max_turn_per_tick_bam));
        }
        if phase2.cluster_sample_max == 0 || phase2.cluster_sample_max > 65_536 {
            return Err(ConfigError::ClusterSample(phase2.cluster_sample_max));
        }
        Ok(())
    }

    /// Canonical config hash over the schema version, policy versions, and
    /// every field in declaration order.
    pub fn stable_hash(&self) -> u64 {
        let mut hasher = Fnv1a64::new();
        hasher.update(b"lifesim-config");
        hasher.update_u32(CONFIG_SCHEMA_VERSION);
        hasher.update(BEHAVIOR_POLICY_VERSION.as_bytes());
        hasher.update(crate::rng::RNG_ALGORITHM_VERSION.as_bytes());
        // The *selected* generator, not a constant. With the default V1 this
        // hashes exactly the byte string it always did, so adding V2 cannot
        // move any existing world's config hash.
        hasher.update(self.climate.worldgen_version.tag().as_bytes());
        hasher.update_u64(self.world_seed);
        hasher.update_u32(self.cells_x);
        hasher.update_u32(self.cells_y);
        hasher.update_u32(self.cell_size_m);
        hasher.update_u32(self.initial_organisms);
        hasher.update_u32(self.max_entities);
        hasher.update_u32(self.dt_ms);
        hasher.update_u32(self.growth_rate_q16_per_s);
        hasher.update_i64(self.cell_capacity_milli);
        hasher.update_u32(self.initial_biomass_q16);
        hasher.update_i64(self.energy_max_milli);
        hasher.update_i64(self.initial_energy_milli);
        hasher.update_i64(self.basal_cost_milli_per_s);
        hasher.update_i64(self.move_cost_milli_per_s);
        hasher.update_i64(self.intake_rate_milli_per_s);
        hasher.update_u32(self.assimilation_q16);
        hasher.update_u32(self.speed_mps_q16);
        hasher.update_u32(self.crowding_radius_m);
        hasher.update_u32(self.crowding_threshold);
        hasher.update_i64(self.crowding_cost_milli_per_s);
        hasher.update_u64(self.maturity_age_ticks);
        hasher.update_u64(self.max_age_ticks);
        hasher.update_u32(u32::from(self.reproduction_enabled));
        hasher.update_i64(self.repro_threshold_milli);
        hasher.update_i64(self.offspring_energy_milli);
        hasher.update_i64(self.repro_overhead_milli);
        hasher.update_u64(self.repro_cooldown_ticks);
        hasher.update_u32(self.land_threshold_q16);
        hasher.update_u32(self.min_land_fraction_q16);
        hasher.update_u32(self.max_land_fraction_q16);
        // The Phase 2 section participates in the hash only when enabled so
        // that phase2-disabled configs hash identically to Phase 1 configs.
        // A disabled section is behaviorally inert by construction (the
        // world takes the exact Phase 1 code paths), so this equality is
        // semantic, not cosmetic. Enabling Phase 2 changes the hash and
        // starts a new replay lineage.
        if self.phase2.enabled {
            hasher.update(b"lifesim-phase2-config");
            hasher.update(PHASE2_BEHAVIOR_POLICY_VERSION.as_bytes());
            hasher.update(crate::genome::GENOME_POLICY_VERSION.as_bytes());
            hasher.update(crate::controller::CONTROLLER_POLICY_VERSION.as_bytes());
            hasher.update_u32(u32::from(crate::genome::GENOME_SCHEMA_VERSION));
            hasher.update_u32(u32::from(crate::genome::TOPOLOGY_ID));
            hasher.update_u32(self.phase2.variation_probability_q16);
            hasher.update_u32(self.phase2.variation_trait_sigma_q16);
            hasher.update_u32(self.phase2.variation_neural_sigma_q16);
            hasher.update_u32(self.phase2.pairing_range_m);
            hasher.update_u32(self.phase2.compatibility_threshold_q16);
            hasher.update_i64(self.phase2.pairing_energy_threshold_milli);
            hasher.update_i64(self.phase2.pairing_overhead_milli);
            hasher.update_i32(self.phase2.eat_threshold_q16);
            hasher.update_i32(self.phase2.mate_threshold_q16);
            hasher.update_i32(self.phase2.rest_threshold_q16);
            hasher.update_u32(self.phase2.max_turn_per_tick_bam);
            hasher.update_u32(self.phase2.cluster_threshold_q16);
            hasher.update_u32(self.phase2.cluster_sample_max);
            hasher.update_u32(self.phase2.cluster_neural_weight_q16);
        }
        // The Phase 6 section participates only when enabled, so a
        // climate-disabled config hashes exactly as it did before Phase 6
        // existed and both fixtures are preserved.
        if self.climate.enabled {
            hasher.update(b"lifesim-climate-config");
            hasher.update(crate::climate::BIOME_POLICY_VERSION.as_bytes());
            hasher.update(crate::climate::CLIMATE_POLICY_VERSION.as_bytes());
            hasher.update_i32(self.climate.base_temperature_milli);
            hasher.update_i32(self.climate.lapse_milli_per_full_elevation);
            hasher.update_i32(self.climate.latitude_amplitude_milli);
            hasher.update_u64(self.climate.season_period_ticks);
            hasher.update_i32(self.climate.season_amplitude_milli);
            for period in self.climate.drift_period_ticks {
                hasher.update_u64(period);
            }
            for amplitude in self.climate.drift_amplitude_milli {
                hasher.update_i32(amplitude);
            }
            hasher.update_i32(self.climate.temperature_min_milli);
            hasher.update_i32(self.climate.temperature_max_milli);
            hasher.update_i64(self.climate.initial_moisture_milli);
            hasher.update_i64(self.climate.coastal_moisture_bonus_milli);
            hasher.update_i64(self.climate.moisture_max_milli);
            hasher.update_i64(self.climate.moisture_ceiling_milli);
            hasher.update_u32(self.climate.sea_proximity_weight_q16);
            hasher.update_u32(self.climate.moisture_diffusion_q16);
            hasher.update_u32(self.climate.moisture_drain_weight);
            hasher.update_u32(self.climate.highland_elevation_q16);
            hasher.update_i64(self.climate.wetland_moisture_milli);
            hasher.update_i64(self.climate.arid_moisture_milli);
            hasher.update_i64(self.climate.forest_moisture_milli);
            hasher.update_i32(self.climate.forest_min_temperature_milli);
            for multiplier in self.climate.biome_capacity_q16 {
                hasher.update_u32(multiplier);
            }
            hasher.update_u64(self.climate.reclassify_interval_ticks);
        }
        // The origin section participates only when it differs from the
        // Phase 1/2 founder behavior, so a default-origin config hashes
        // exactly as it did before Phase 6 existed.
        if !crate::origin::is_default_origin(&self.origin) {
            crate::origin::hash_origin_into(&mut hasher, &self.origin);
        }
        // Phase 7 section: hashed only when enabled, so a contest-disabled
        // config hashes exactly as it did before Phase 7 existed.
        if self.contest.enabled {
            hasher.update(b"lifesim-contest-config");
            hasher.update(crate::contest::CONTEST_POLICY_VERSION.as_bytes());
            hasher.update(crate::contest::PAIR_KEY_POLICY_VERSION.as_bytes());
            hasher.update_i64(self.contest.base_health_milli);
            hasher.update_i64(self.contest.damage_base_milli);
            hasher.update_u32(self.contest.damage_variance_q16);
            hasher.update_i64(self.contest.attack_cost_milli);
            hasher.update_u32(self.contest.attack_range_m);
            hasher.update_i32(self.contest.attack_threshold_q16);
            hasher.update_u64(self.contest.attack_cooldown_ticks);
            hasher.update_i64(self.contest.heal_milli_per_s);
            hasher.update_u32(self.contest.heal_energy_cost_q16);
            hasher.update_u32(self.contest.heal_energy_floor_q16);
            hasher.update_u32(self.contest.damage_decay_q16_per_s);
            hasher.update_u32(self.contest.carcass_energy_q16);
            hasher.update_u32(self.contest.carcass_decay_q16_per_s);
            hasher.update_u32(self.contest.carcass_reach_m);
            hasher.update_u32(self.contest.max_carcasses);
            hasher.update_i64(self.contest.local_depletion_milli);
        }
        // Phase 8 section: hashed only when enabled, so a
        // demography-disabled config hashes exactly as it did before Phase
        // 8 existed and every earlier fixture is preserved.
        if self.physiology.enabled {
            hasher.update(b"lifesim-physiology-config");
            hasher.update(crate::physiology::PHYSIOLOGY_POLICY_VERSION.as_bytes());
            hasher.update_u32(u32::from(self.physiology.allometry_enabled));
            hasher.update_u32(self.physiology.basal_exponent_quarters);
            hasher.update_u32(u32::from(self.physiology.thermoregulation_enabled));
            hasher.update_i32(self.physiology.thermal_pref_low_milli);
            hasher.update_i32(self.physiology.thermal_pref_high_milli);
            hasher.update_i32(self.physiology.thermal_neutral_band_milli);
            hasher.update_i64(self.physiology.thermal_cost_milli_per_s_per_degree);
            hasher.update_u32(u32::from(self.physiology.senescence_enabled));
            hasher.update_u64(self.physiology.senescence_onset_ticks);
            hasher.update_u64(self.physiology.senescence_scale_ticks);
            hasher.update_u32(self.physiology.senescence_power);
            hasher.update_u32(self.physiology.senescence_hazard_q16_per_s);
            hasher.update_u32(self.physiology.extrinsic_hazard_q16_per_s);
            hasher.update_u32(self.physiology.juvenile_hazard_multiplier_q16);
        }
        // Phase 9 section: hashed only when enabled, so a schema-1 config
        // hashes exactly as it did before schema 2 existed and every earlier
        // fixture is preserved.
        if self.genome2.enabled {
            hasher.update(b"lifesim-genome2-config");
            hasher.update(crate::genome2::GENOME2_POLICY_VERSION.as_bytes());
            hasher.update(crate::meiosis::MEIOSIS_POLICY_VERSION.as_bytes());
            hasher.update(crate::structmut::STRUCTMUT_POLICY_VERSION.as_bytes());
            hasher.update(crate::controller2::CONTROLLER2_POLICY_VERSION.as_bytes());
            // The registries are part of what a genome means, so their
            // versions enter the hash: the same loci under a different
            // channel registry describe a different organism.
            let (channels, activations) = crate::genome2::registry_versions();
            hasher.update_u32(u32::from(channels));
            hasher.update_u32(u32::from(activations));
            let caps = &self.genome2.caps;
            hasher.update_u32(u32::from(caps.max_chromosomes));
            hasher.update_u32(caps.max_loci_per_chromosome);
            hasher.update_u32(caps.max_nodes);
            hasher.update_u32(caps.max_edges);
            hasher.update_u32(caps.max_edges_per_node);
            hasher.update_u32(caps.max_genome_bytes);
            hasher.update_u32(caps.min_nodes);
            hasher.update_u32(u32::from(self.genome2.meiosis.mode.id()));
            hasher.update_u32(self.genome2.meiosis.max_extra_crossovers);
            let mutation = &self.genome2.mutation;
            hasher.update_u32(mutation.point_q16);
            hasher.update_u32(mutation.duplication_q16);
            hasher.update_u32(mutation.deletion_q16);
            hasher.update_u32(mutation.insertion_q16);
            hasher.update_u32(mutation.transposition_q16);
            hasher.update_u32(mutation.max_run);
            hasher.update_u32(mutation.point_delta_q16);
            hasher.update_u32(u32::from(mutation.regulatory_enabled));
            // Phase 11's plasticity-mutation gate: **hashed only when it is
            // true**, and appended after every field that existed before it.
            //
            // Both obvious choices are wrong. Omitting it would let two
            // behaviorally different worlds - one whose plasticity genes
            // evolve, one whose do not - share a config hash, which is a
            // real defect and the thing this hand-maintained list exists to
            // prevent. Hashing it unconditionally would fold a Phase 11
            // field into every schema-2 world that already exists and move
            // the Phase 9 fixture (config `0x9abc0cd47914127f`), which was
            // pinned before the field existed.
            //
            // This is D-014's "a section is folded in only when enabled"
            // applied at **field** granularity rather than section
            // granularity, which is what the situation actually needs: the
            // genome2 section *is* enabled in those worlds, and the field
            // is not. Its own tag keeps it self-describing, so a later
            // Phase 11 config block appending here cannot alias it.
            if mutation.plasticity_enabled {
                hasher.update(b"lifesim-plasticity-mutation-v1");
                hasher.update_u32(1);
            }
        }
        // Phase 10 section, on the same terms: hashed only when enabled, so
        // every earlier fixture survives untouched (D-014).
        if self.morphology.enabled {
            hasher.update(b"lifesim-morphology-config");
            hasher.update(crate::morphology::MORPHOLOGY_POLICY_VERSION.as_bytes());
            hasher.update(crate::develop::DEVELOP_POLICY_VERSION.as_bytes());
            // The module registry is part of what a body means: the same
            // modules under different coefficients describe a different
            // organism.
            hasher.update_u32(u32::from(crate::morphology::MODULE_REGISTRY_VERSION));
            hasher.update_u32(u32::from(self.morphology.lattice.id()));
            let caps = &self.morphology.caps;
            hasher.update_u32(u32::from(caps.max_modules));
            hasher.update_u32(caps.lattice_radius as u32);
            hasher.update_u32(u32::from(caps.max_growth_steps));
            hasher.update_u32(u32::from(caps.required_types_mask));
            hasher.update_u32(self.morphology.base_node_budget);
        }
        // Phase 11 section, **appended last and hashed only when enabled**.
        // Appended rather than slotted next to the genome2 block it depends
        // on: the order of this function is the definition of every existing
        // config hash, and inserting a section anywhere but the end would
        // move worlds that do not have it. Enabling plasticity changes the
        // hash and starts a new replay lineage, which is correct - a world
        // whose weights change within a lifetime is not the same experiment.
        if self.plasticity.enabled {
            hasher.update(b"lifesim-plasticity-config");
            hasher.update(crate::plasticity::PLASTICITY_POLICY_VERSION.as_bytes());
            // The rule registry is part of what a plasticity gene means: the
            // same `rule_id` under a different registry is a different rule,
            // exactly as the same locus under a different channel registry
            // describes a different organism.
            hasher.update_u32(u32::from(crate::plasticity::RULE_REGISTRY_VERSION));
            hasher.update_i64(self.plasticity.plastic_edge_cost_milli_per_s);
            hasher.update_u32(self.plasticity.max_plastic_edges);
            hasher.update_u32(self.plasticity.lamarckian_fraction_q16);
        }
        hasher.finish()
    }

    /// How many plastic edges a network compiled for this world may carry.
    ///
    /// `None` when the plasticity section is disabled, which is **not** the
    /// same as a budget of zero: with `None` no edge is compiled plastic at
    /// all and nothing is counted as refused, so the compiled plan is
    /// byte-identical to the one this world produced before Phase 11.
    pub fn plasticity_budget(&self) -> crate::controller2::PlasticityBudget {
        self.plasticity
            .enabled
            .then_some(self.plasticity.max_plastic_edges)
    }

    pub fn world_extent_x_fp(&self) -> i32 {
        (self.cells_x * self.cell_size_m) as i32 * crate::FP_PER_METER
    }

    pub fn world_extent_y_fp(&self) -> i32 {
        (self.cells_y * self.cell_size_m) as i32 * crate::FP_PER_METER
    }

    pub fn cell_size_fp(&self) -> i32 {
        self.cell_size_m as i32 * crate::FP_PER_METER
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfigError {
    /// A Phase 8 physiology parameter outside its documented range.
    PhysiologyRange(&'static str, i64),
    FractionOutOfRange(&'static str, u32),
    WorldDimensions(u32, u32),
    CellSize(u32),
    MaxEntities(u32),
    InitialOrganisms(u32),
    TickLength(u32),
    NonPositive(&'static str),
    Negative(&'static str),
    InitialEnergy(i64),
    Speed(u32),
    CrowdingRadius(u32),
    AgePolicy {
        maturity: u64,
        max_age: u64,
    },
    ReproductionEnergy {
        threshold: i64,
        total_cost: i64,
    },
    LandFractionBounds {
        min: u32,
        max: u32,
    },
    PairingRange(u32),
    PairingEnergy(i64),
    ControllerThreshold(&'static str, i32),
    TurnRate(u32),
    ClusterSample(u32),
    ClimateGeneratorMismatch {
        enabled: bool,
        generator: &'static str,
    },
    TemperatureBounds {
        min: i32,
        max: i32,
    },
    DriftPeriodTooShort {
        period: u64,
        season: u64,
    },
    BiomeThresholdOrder {
        arid: i64,
        forest: i64,
        wetland: i64,
    },
    DemeCount(u32),
    ArchetypeCount(u32),
    ArchetypeOrder {
        index: usize,
        id: u16,
    },
    EmptyArchetypeAffinity {
        id: u16,
    },
    SeededRequiresClimate,
    ContestRequiresPhase2,
    AttackRange(u32),
    MaxCarcasses(u32),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PhysiologyRange(name, value) => {
                write!(formatter, "{name} is outside its supported range: {value}")
            }
            Self::FractionOutOfRange(name, value) => {
                write!(formatter, "{name} must be a Q16 fraction, got {value}")
            }
            Self::WorldDimensions(x, y) => {
                write!(formatter, "unsupported world dimensions {x}x{y} cells")
            }
            Self::CellSize(size) => write!(formatter, "unsupported cell size {size} m"),
            Self::MaxEntities(value) => write!(formatter, "invalid max_entities {value}"),
            Self::InitialOrganisms(value) => {
                write!(formatter, "invalid initial organism count {value}")
            }
            Self::TickLength(value) => write!(formatter, "invalid dt_ms {value}"),
            Self::NonPositive(name) => write!(formatter, "{name} must be positive"),
            Self::Negative(name) => write!(formatter, "{name} must be non-negative"),
            Self::InitialEnergy(value) => write!(formatter, "invalid initial energy {value}"),
            Self::Speed(value) => write!(formatter, "invalid speed_mps_q16 {value}"),
            Self::CrowdingRadius(value) => write!(formatter, "invalid crowding radius {value} m"),
            Self::AgePolicy { maturity, max_age } => write!(
                formatter,
                "max_age_ticks {max_age} must exceed maturity_age_ticks {maturity}"
            ),
            Self::ReproductionEnergy {
                threshold,
                total_cost,
            } => write!(
                formatter,
                "repro_threshold_milli {threshold} must cover offspring cost {total_cost} and fit energy_max"
            ),
            Self::LandFractionBounds { min, max } => write!(
                formatter,
                "min_land_fraction {min} must be below max_land_fraction {max}"
            ),
            Self::PairingRange(value) => write!(formatter, "invalid pairing_range_m {value}"),
            Self::PairingEnergy(value) => {
                write!(formatter, "invalid pairing_energy_threshold_milli {value}")
            }
            Self::ControllerThreshold(name, value) => {
                write!(
                    formatter,
                    "{name} must be a signed Q16 fraction, got {value}"
                )
            }
            Self::TurnRate(value) => write!(formatter, "invalid max_turn_per_tick_bam {value}"),
            Self::ClusterSample(value) => write!(formatter, "invalid cluster_sample_max {value}"),
            Self::ClimateGeneratorMismatch { enabled, generator } => write!(
                formatter,
                "climate.enabled is {enabled} but the generator is {generator}; the climate \
                 section and lifesim-worldgen-v2 must be enabled together"
            ),
            Self::TemperatureBounds { min, max } => write!(
                formatter,
                "temperature_min_milli {min} must be below temperature_max_milli {max}"
            ),
            Self::DriftPeriodTooShort { period, season } => write!(
                formatter,
                "drift period {period} must exceed the season period {season}; a drift term \
                 on a seasonal timescale is a second season, not a long-timescale term"
            ),
            Self::BiomeThresholdOrder {
                arid,
                forest,
                wetland,
            } => write!(
                formatter,
                "biome moisture thresholds must ascend: arid {arid} < forest {forest} < \
                 wetland {wetland}, or a biome is unreachable by construction"
            ),
            Self::DemeCount(value) => write!(formatter, "invalid deme_count {value}"),
            Self::ArchetypeCount(value) => write!(
                formatter,
                "invalid archetype_count {value}; seeded origin needs at least one"
            ),
            Self::ArchetypeOrder { index, id } => write!(
                formatter,
                "archetype {index} has id {id}, which does not ascend; archetypes are sorted \
                 by id so founder allocation order is a function of the set, not the array"
            ),
            Self::EmptyArchetypeAffinity { id } => write!(
                formatter,
                "archetype {id} has an empty biome affinity, so no cell could ever match it"
            ),
            Self::SeededRequiresClimate => formatter.write_str(
                "origin.mode = seeded needs biomes to match against, so the climate section \
                 must be enabled",
            ),
            Self::ContestRequiresPhase2 => formatter.write_str(
                "the contest section wires reserved controller channels, so phase2 must be \
                 enabled",
            ),
            Self::AttackRange(value) => write!(formatter, "invalid reach {value} m"),
            Self::MaxCarcasses(value) => write!(formatter, "invalid max_carcasses {value}"),
        }
    }
}

impl std::error::Error for ConfigError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid_and_hash_is_stable_within_build() {
        let config = SimConfig::phase1_default(42);
        config.validate().unwrap();
        assert_eq!(config.stable_hash(), config.stable_hash());
        let other_seed = SimConfig::phase1_default(43);
        assert_ne!(config.stable_hash(), other_seed.stable_hash());
    }

    #[test]
    fn every_field_affects_the_hash() {
        let base = SimConfig::phase1_default(42);
        let mut changed = base;
        changed.crowding_threshold += 1;
        assert_ne!(base.stable_hash(), changed.stable_hash());
        let mut changed = base;
        changed.reproduction_enabled = false;
        assert_ne!(base.stable_hash(), changed.stable_hash());
    }

    #[test]
    fn the_plasticity_section_is_inert_when_disabled_and_hashed_when_enabled() {
        // D-014's rule at the config layer, and the two halves are separate
        // claims. Disabled, the section must not touch the hash at all - the
        // Phase 9 fixture depends on it. Enabled, every field must reach the
        // hash, or two behaviorally different worlds would share a lineage.
        let base = SimConfig::phase2_default(42);
        let mut with_defaults = base;
        with_defaults.plasticity = PlasticityConfig::plasticity_default();
        assert_eq!(
            base.stable_hash(),
            with_defaults.stable_hash(),
            "a disabled plasticity section reached the config hash"
        );
        // ...and it stays out even when its fields are moved, which is the
        // assertion a `plasticity.enabled` check alone would not make.
        let mut moved = base;
        moved.plasticity.plastic_edge_cost_milli_per_s = 999;
        moved.plasticity.max_plastic_edges = 7;
        assert_eq!(base.stable_hash(), moved.stable_hash());

        let enabled = SimConfig::phase11_default(42);
        enabled.validate().expect("phase11 defaults are valid");
        let reference = enabled.stable_hash();
        assert_ne!(reference, base.stable_hash());
        let mutators: [fn(&mut SimConfig); 2] = [
            |config| config.plasticity.plastic_edge_cost_milli_per_s += 1,
            |config| config.plasticity.max_plastic_edges += 1,
        ];
        for (index, mutate) in mutators.into_iter().enumerate() {
            let mut changed = enabled;
            mutate(&mut changed);
            assert_ne!(changed.stable_hash(), reference, "field {index}");
        }
    }

    #[test]
    fn the_plasticity_section_is_validated_where_validation_actually_runs() {
        // D-084: these checks live in `validate_subsystems`, not in
        // `validate_contest`, which early-returns on a disabled contest
        // section. **The contest section is disabled in every config below**,
        // so a check appended to the wrong function would make every
        // assertion here pass vacuously.
        let mut config = SimConfig::phase2_default(1);
        assert!(!config.contest.enabled);
        config.plasticity.enabled = true;
        assert_eq!(
            config.validate(),
            Err(ConfigError::PhysiologyRange(
                "plasticity requires genome2",
                0
            ))
        );

        let mut config = SimConfig::phase11_default(1);
        config.plasticity.max_plastic_edges = 0;
        assert!(matches!(
            config.validate(),
            Err(ConfigError::PhysiologyRange("max_plastic_edges is zero", 0))
        ));

        let mut config = SimConfig::phase11_default(1);
        config.plasticity.max_plastic_edges = config.genome2.caps.max_edges + 1;
        assert!(matches!(
            config.validate(),
            Err(ConfigError::PhysiologyRange(
                "max_plastic_edges exceeds genome2.caps.max_edges",
                _
            ))
        ));

        let mut config = SimConfig::phase11_default(1);
        config.plasticity.plastic_edge_cost_milli_per_s = -1;
        assert_eq!(
            config.validate(),
            Err(ConfigError::Negative("plastic_edge_cost_milli_per_s"))
        );

        // Nonzero Lamarckian inheritance is refused rather than silently
        // accepted, because no policy implements it: a run that looked like
        // the declared experimental condition and was not it is worse than a
        // refused config.
        let mut config = SimConfig::phase11_default(1);
        config.plasticity.lamarckian_fraction_q16 = 1;
        assert!(matches!(
            config.validate(),
            Err(ConfigError::PhysiologyRange(_, 1))
        ));
        let mut config = SimConfig::phase11_default(1);
        config.plasticity.lamarckian_fraction_q16 = Q16_ONE + 1;
        assert!(matches!(
            config.validate(),
            Err(ConfigError::FractionOutOfRange(
                "lamarckian_fraction_q16",
                _
            ))
        ));

        // The budget is `None` when the section is off, which is not the same
        // as a budget of zero: `None` compiles no plastic edge at all.
        assert_eq!(SimConfig::phase2_default(1).plasticity_budget(), None);
        assert_eq!(SimConfig::phase11_default(1).plasticity_budget(), Some(32));
    }

    #[test]
    fn invalid_configs_are_rejected() {
        let mut config = SimConfig::phase1_default(1);
        config.cells_x = 4;
        assert_eq!(config.validate(), Err(ConfigError::WorldDimensions(4, 256)));

        let mut config = SimConfig::phase1_default(1);
        config.initial_organisms = config.max_entities + 1;
        assert!(matches!(
            config.validate(),
            Err(ConfigError::InitialOrganisms(_))
        ));

        let mut config = SimConfig::phase1_default(1);
        config.assimilation_q16 = 0;
        assert!(matches!(
            config.validate(),
            Err(ConfigError::FractionOutOfRange("assimilation_q16", 0))
        ));

        let mut config = SimConfig::phase1_default(1);
        config.repro_threshold_milli = 100;
        assert!(matches!(
            config.validate(),
            Err(ConfigError::ReproductionEnergy { .. })
        ));

        let mut config = SimConfig::phase1_default(1);
        config.max_age_ticks = config.maturity_age_ticks;
        assert!(matches!(
            config.validate(),
            Err(ConfigError::AgePolicy { .. })
        ));

        let mut config = SimConfig::phase1_default(1);
        config.dt_ms = 0;
        assert_eq!(config.validate(), Err(ConfigError::TickLength(0)));
    }
}
