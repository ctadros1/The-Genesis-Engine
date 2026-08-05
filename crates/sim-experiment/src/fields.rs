//! Config fields addressable by name.
//!
//! A condition is a named config delta, so something has to name config
//! fields, and the same registry is what lets the comparison report answer
//! "do these two runs differ anywhere other than the field the report
//! declares it varied?" That question is the whole of acceptance criterion
//! A5.6, and it cannot be answered by comparing config hashes: a hash says
//! *that* two configs differ, never *where*.
//!
//! `world_seed` is deliberately absent. Seeds are the replicate axis of
//! every campaign, not a condition delta, and allowing a condition to set
//! one would let a treatment and its control silently run different worlds.

use sim_core::SimConfig;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FieldValue {
    /// A named enumerated choice, e.g. `origin.mode`.
    Choice(&'static str),
    U32(u32),
    U64(u64),
    I32(i32),
    I64(i64),
    Bool(bool),
}

impl fmt::Display for FieldValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Choice(value) => write!(formatter, "{value}"),
            Self::U32(value) => write!(formatter, "{value}"),
            Self::U64(value) => write!(formatter, "{value}"),
            Self::I32(value) => write!(formatter, "{value}"),
            Self::I64(value) => write!(formatter, "{value}"),
            Self::Bool(value) => write!(formatter, "{value}"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FieldError {
    Unknown(String),
    /// The value did not parse as the field's type.
    BadValue {
        field: String,
        value: String,
    },
    /// `world_seed` is the campaign's replicate axis, never a delta.
    SeedIsNotAField,
}

impl fmt::Display for FieldError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown(name) => write!(
                formatter,
                "unknown config field '{name}'; run `lifesim fields` for the list"
            ),
            Self::BadValue { field, value } => {
                write!(
                    formatter,
                    "invalid value '{value}' for config field '{field}'"
                )
            }
            Self::SeedIsNotAField => write!(
                formatter,
                "world_seed is the campaign seed axis and cannot be set as a config field; \
                 use the `seeds` directive"
            ),
        }
    }
}

impl std::error::Error for FieldError {}

fn parse_bool(value: &str) -> Option<bool> {
    match value {
        "true" | "on" | "yes" | "1" => Some(true),
        "false" | "off" | "no" | "0" => Some(false),
        _ => None,
    }
}

macro_rules! config_fields {
    ($( $name:literal => $($path:ident).+ : $kind:ident ),* $(,)?) => {
        /// Every settable field, in a fixed order so reports and manifests
        /// are byte-stable. The two coordinated fields lead.
        pub const FIELD_NAMES: &[&str] = &[
            "climate.enabled",
            "origin.mode",
            $($name),*
        ];

        pub fn read_field(config: &SimConfig, name: &str) -> Option<FieldValue> {
            // Two fields are not plain struct assignments and are handled
            // here rather than in the table below.
            match name {
                "climate.enabled" => return Some(FieldValue::Bool(config.climate.enabled)),
                "origin.mode" => {
                    return Some(FieldValue::Choice(config.origin.mode.name()));
                }
                _ => {}
            }
            match name {
                $( $name => Some(config_fields!(@read $kind, config.$($path).+)), )*
                _ => None,
            }
        }

        pub fn set_field(
            config: &mut SimConfig,
            name: &str,
            value: &str,
        ) -> Result<(), FieldError> {
            if name == "world_seed" {
                return Err(FieldError::SeedIsNotAField);
            }
            // `climate.enabled` and the generator version move together, and
            // config validation enforces that, so setting one field has to
            // set both or every campaign that touches climate would be
            // rejected for a reason the author did not write.
            match name {
                "climate.enabled" => {
                    let enabled = parse_bool(value).ok_or_else(|| FieldError::BadValue {
                        field: name.to_owned(),
                        value: value.to_owned(),
                    })?;
                    config.climate.enabled = enabled;
                    config.climate.worldgen_version = if enabled {
                        sim_core::WorldgenVersion::V2
                    } else {
                        sim_core::WorldgenVersion::V1
                    };
                    return Ok(());
                }
                "origin.mode" => {
                    config.origin.mode = match value {
                        "random" => sim_core::OriginMode::Random,
                        "seeded" => sim_core::OriginMode::Seeded,
                        _ => {
                            return Err(FieldError::BadValue {
                                field: name.to_owned(),
                                value: value.to_owned(),
                            });
                        }
                    };
                    return Ok(());
                }
                _ => {}
            }
            match name {
                $(
                    $name => {
                        config.$($path).+ = config_fields!(@parse $kind, name, value)?;
                        Ok(())
                    }
                )*
                _ => Err(FieldError::Unknown(name.to_owned())),
            }
        }
    };
    (@read bool, $expr:expr) => { FieldValue::Bool($expr) };
    (@read u32, $expr:expr) => { FieldValue::U32($expr) };
    (@read u64, $expr:expr) => { FieldValue::U64($expr) };
    (@read i32, $expr:expr) => { FieldValue::I32($expr) };
    (@read i64, $expr:expr) => { FieldValue::I64($expr) };
    (@parse bool, $name:expr, $value:expr) => {
        parse_bool($value).ok_or_else(|| FieldError::BadValue {
            field: $name.to_owned(),
            value: $value.to_owned(),
        })
    };
    (@parse $kind:ident, $name:expr, $value:expr) => {
        $value.parse::<$kind>().map_err(|_| FieldError::BadValue {
            field: $name.to_owned(),
            value: $value.to_owned(),
        })
    };
}

config_fields! {
    "cells_x" => cells_x: u32,
    "cells_y" => cells_y: u32,
    "cell_size_m" => cell_size_m: u32,
    "initial_organisms" => initial_organisms: u32,
    "max_entities" => max_entities: u32,
    "dt_ms" => dt_ms: u32,
    "growth_rate_q16_per_s" => growth_rate_q16_per_s: u32,
    "cell_capacity_milli" => cell_capacity_milli: i64,
    "initial_biomass_q16" => initial_biomass_q16: u32,
    "energy_max_milli" => energy_max_milli: i64,
    "initial_energy_milli" => initial_energy_milli: i64,
    "basal_cost_milli_per_s" => basal_cost_milli_per_s: i64,
    "move_cost_milli_per_s" => move_cost_milli_per_s: i64,
    "intake_rate_milli_per_s" => intake_rate_milli_per_s: i64,
    "assimilation_q16" => assimilation_q16: u32,
    "speed_mps_q16" => speed_mps_q16: u32,
    "crowding_radius_m" => crowding_radius_m: u32,
    "crowding_threshold" => crowding_threshold: u32,
    "crowding_cost_milli_per_s" => crowding_cost_milli_per_s: i64,
    "maturity_age_ticks" => maturity_age_ticks: u64,
    "max_age_ticks" => max_age_ticks: u64,
    "reproduction_enabled" => reproduction_enabled: bool,
    "repro_threshold_milli" => repro_threshold_milli: i64,
    "offspring_energy_milli" => offspring_energy_milli: i64,
    "repro_overhead_milli" => repro_overhead_milli: i64,
    "repro_cooldown_ticks" => repro_cooldown_ticks: u64,
    "land_threshold_q16" => land_threshold_q16: u32,
    "min_land_fraction_q16" => min_land_fraction_q16: u32,
    "max_land_fraction_q16" => max_land_fraction_q16: u32,
    "phase2.enabled" => phase2.enabled: bool,
    "phase2.variation_probability_q16" => phase2.variation_probability_q16: u32,
    "phase2.variation_trait_sigma_q16" => phase2.variation_trait_sigma_q16: u32,
    "phase2.variation_neural_sigma_q16" => phase2.variation_neural_sigma_q16: u32,
    "phase2.pairing_range_m" => phase2.pairing_range_m: u32,
    "phase2.compatibility_threshold_q16" => phase2.compatibility_threshold_q16: u32,
    "phase2.pairing_energy_threshold_milli" => phase2.pairing_energy_threshold_milli: i64,
    "phase2.pairing_overhead_milli" => phase2.pairing_overhead_milli: i64,
    "phase2.eat_threshold_q16" => phase2.eat_threshold_q16: i32,
    "phase2.mate_threshold_q16" => phase2.mate_threshold_q16: i32,
    "phase2.rest_threshold_q16" => phase2.rest_threshold_q16: i32,
    "phase2.max_turn_per_tick_bam" => phase2.max_turn_per_tick_bam: u32,
    "phase2.cluster_threshold_q16" => phase2.cluster_threshold_q16: u32,
    "phase2.cluster_sample_max" => phase2.cluster_sample_max: u32,
    "phase2.cluster_neural_weight_q16" => phase2.cluster_neural_weight_q16: u32,
    "climate.season_amplitude_milli" => climate.season_amplitude_milli: i32,
    "climate.base_temperature_milli" => climate.base_temperature_milli: i32,
    "climate.latitude_amplitude_milli" => climate.latitude_amplitude_milli: i32,
    "climate.initial_moisture_milli" => climate.initial_moisture_milli: i64,
    "climate.sea_proximity_weight_q16" => climate.sea_proximity_weight_q16: u32,
    "climate.moisture_diffusion_q16" => climate.moisture_diffusion_q16: u32,
    "climate.highland_elevation_q16" => climate.highland_elevation_q16: u32,
    "climate.wetland_moisture_milli" => climate.wetland_moisture_milli: i64,
    "climate.arid_moisture_milli" => climate.arid_moisture_milli: i64,
    "climate.forest_moisture_milli" => climate.forest_moisture_milli: i64,
    "climate.reclassify_interval_ticks" => climate.reclassify_interval_ticks: u64,
    "origin.deme_count" => origin.deme_count: u32,
    "origin.deme_radius_m" => origin.deme_radius_m: u32,
    "origin.deme_min_separation_m" => origin.deme_min_separation_m: u32,
    "origin.deme_trait_spread_q16" => origin.deme_trait_spread_q16: u32,
    "origin.archetype_count" => origin.archetype_count: u32,
    "contest.enabled" => contest.enabled: bool,
    "contest.base_health_milli" => contest.base_health_milli: i64,
    "contest.damage_base_milli" => contest.damage_base_milli: i64,
    "contest.damage_variance_q16" => contest.damage_variance_q16: u32,
    "contest.attack_cost_milli" => contest.attack_cost_milli: i64,
    "contest.attack_range_m" => contest.attack_range_m: u32,
    "contest.attack_threshold_q16" => contest.attack_threshold_q16: i32,
    "contest.attack_cooldown_ticks" => contest.attack_cooldown_ticks: u64,
    "contest.heal_milli_per_s" => contest.heal_milli_per_s: i64,
    "contest.carcass_energy_q16" => contest.carcass_energy_q16: u32,
    "contest.carcass_decay_q16_per_s" => contest.carcass_decay_q16_per_s: u32,
    "contest.carcass_reach_m" => contest.carcass_reach_m: u32,
    "contest.local_depletion_milli" => contest.local_depletion_milli: i64,
    "physiology.enabled" => physiology.enabled: bool,
    "physiology.allometry_enabled" => physiology.allometry_enabled: bool,
    "physiology.basal_exponent_quarters" => physiology.basal_exponent_quarters: u32,
    "physiology.thermoregulation_enabled" => physiology.thermoregulation_enabled: bool,
    "physiology.thermal_pref_low_milli" => physiology.thermal_pref_low_milli: i32,
    "physiology.thermal_pref_high_milli" => physiology.thermal_pref_high_milli: i32,
    "physiology.thermal_neutral_band_milli" => physiology.thermal_neutral_band_milli: i32,
    "physiology.thermal_cost_milli_per_s_per_degree" => physiology.thermal_cost_milli_per_s_per_degree: i64,
    "physiology.senescence_enabled" => physiology.senescence_enabled: bool,
    "physiology.senescence_onset_ticks" => physiology.senescence_onset_ticks: u64,
    "physiology.senescence_scale_ticks" => physiology.senescence_scale_ticks: u64,
    "physiology.senescence_power" => physiology.senescence_power: u32,
    "physiology.senescence_hazard_q16_per_s" => physiology.senescence_hazard_q16_per_s: u32,
    "physiology.extrinsic_hazard_q16_per_s" => physiology.extrinsic_hazard_q16_per_s: u32,
    "physiology.juvenile_hazard_multiplier_q16" => physiology.juvenile_hazard_multiplier_q16: u32,
}

/// Every field on which two configs disagree, in `FIELD_NAMES` order.
/// `world_seed` is never reported: it is the replicate axis and differs by
/// construction between runs of the same condition.
pub fn differing_fields(left: &SimConfig, right: &SimConfig) -> Vec<&'static str> {
    FIELD_NAMES
        .iter()
        .copied()
        .filter(|name| read_field(left, name) != read_field(right, name))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_registered_field_reads_and_round_trips() {
        let config = SimConfig::phase2_default(1);
        for name in FIELD_NAMES {
            let value = read_field(&config, name).unwrap_or_else(|| panic!("read {name}"));
            let mut copy = config;
            set_field(&mut copy, name, &value.to_string())
                .unwrap_or_else(|error| panic!("set {name}: {error}"));
            assert_eq!(read_field(&copy, name), Some(value), "round trip {name}");
        }
        assert!(differing_fields(&config, &config).is_empty());
    }

    #[test]
    fn setting_a_field_is_visible_and_changes_the_config_hash() {
        let base = SimConfig::phase2_default(1);
        let mut changed = base;
        set_field(&mut changed, "crowding_threshold", "9").unwrap();
        assert_eq!(
            read_field(&changed, "crowding_threshold"),
            Some(FieldValue::U32(9))
        );
        assert_ne!(base.stable_hash(), changed.stable_hash());
        assert_eq!(
            differing_fields(&base, &changed),
            vec!["crowding_threshold"]
        );
    }

    #[test]
    fn unknown_fields_bad_values_and_the_seed_are_all_rejected() {
        let mut config = SimConfig::phase1_default(1);
        assert!(matches!(
            set_field(&mut config, "not_a_field", "1"),
            Err(FieldError::Unknown(_))
        ));
        assert!(matches!(
            set_field(&mut config, "cells_x", "twelve"),
            Err(FieldError::BadValue { .. })
        ));
        assert!(matches!(
            set_field(&mut config, "cells_x", "-4"),
            Err(FieldError::BadValue { .. })
        ));
        assert_eq!(
            set_field(&mut config, "world_seed", "7"),
            Err(FieldError::SeedIsNotAField)
        );
    }

    #[test]
    fn booleans_accept_the_documented_spellings_only() {
        let mut config = SimConfig::phase1_default(1);
        for (text, expected) in [("true", true), ("on", true), ("false", false), ("0", false)] {
            set_field(&mut config, "reproduction_enabled", text).unwrap();
            assert_eq!(config.reproduction_enabled, expected);
        }
        assert!(set_field(&mut config, "reproduction_enabled", "maybe").is_err());
    }
}
