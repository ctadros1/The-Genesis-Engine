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
        }
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
        hasher.update(crate::worldgen::WORLDGEN_VERSION.as_bytes());
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
        hasher.finish()
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
    AgePolicy { maturity: u64, max_age: u64 },
    ReproductionEnergy { threshold: i64, total_cost: i64 },
    LandFractionBounds { min: u32, max: u32 },
    PairingRange(u32),
    PairingEnergy(i64),
    ControllerThreshold(&'static str, i32),
    TurnRate(u32),
    ClusterSample(u32),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
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
