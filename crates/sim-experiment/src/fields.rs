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
            "physiology.intake_order",
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
                "physiology.intake_order" => {
                    return Some(FieldValue::Choice(config.physiology.intake_order.name()));
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
                "physiology.intake_order" => {
                    config.physiology.intake_order = match value {
                        "ascending" => sim_core::IntakeOrder::Ascending,
                        "descending" => sim_core::IntakeOrder::Descending,
                        _ => {
                            return Err(FieldError::BadValue {
                                field: name.to_owned(),
                                value: value.to_owned(),
                            });
                        }
                    };
                    return Ok(());
                }
                "origin.mode" => {
                    config.origin.mode = match value {
                        "random" => sim_core::OriginMode::Random,
                        "seeded" => sim_core::OriginMode::Seeded,
                        "scratch" => sim_core::OriginMode::Scratch,
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
    // 16-bit fields report as their widened form. Widening on read and
    // narrowing on parse keeps `FieldValue` from growing a variant per
    // integer width, and the narrowing is checked rather than truncating:
    // `max_modules 70000` is refused, not silently wrapped to 4464.
    (@read u16, $expr:expr) => { FieldValue::U32(u32::from($expr)) };
    (@read i16, $expr:expr) => { FieldValue::I32(i32::from($expr)) };
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
    "physiology.ontogeny_enabled" => physiology.ontogeny_enabled: bool,
    "physiology.birth_modules_min" => physiology.birth_modules_min: u32,
    "physiology.growth_cost_milli_per_mass_milli" => physiology.growth_cost_milli_per_mass_milli: i64,
    "physiology.growth_rate_milli_per_s" => physiology.growth_rate_milli_per_s: i64,
    "physiology.mate_choice_enabled" => physiology.mate_choice_enabled: bool,
    "physiology.mate_choice_scramble" => physiology.mate_choice_scramble: bool,
    "chemistry.enabled" => chemistry.enabled: bool,
    "chemistry.field_steps_per_tick" => chemistry.field_steps_per_tick: u32,
    "chemistry.diffusion_q16" => chemistry.diffusion_q16: u32,
    "chemistry.reaction_monomer_q16" => chemistry.reaction_monomer_q16: u32,
    "chemistry.reaction_recycle_q16" => chemistry.reaction_recycle_q16: u32,
    "chemistry.production_milli_per_step" => chemistry.production_milli_per_step: i64,
    "chemistry.scaffold_patch_radius_cells" => chemistry.scaffold_patch_radius_cells: u32,
    "chemistry.scaffold_patch_contrast_q16" => chemistry.scaffold_patch_contrast_q16: u32,
    "chemistry.abiogenesis_enabled" => chemistry.abiogenesis_enabled: bool,
    "chemistry.abiogenesis_weight_primordial_q16" => chemistry.abiogenesis_weight_primordial_q16: u32,
    "chemistry.abiogenesis_weight_monomer_q16" => chemistry.abiogenesis_weight_monomer_q16: u32,
    "chemistry.abiogenesis_weight_polymer_q16" => chemistry.abiogenesis_weight_polymer_q16: u32,
    "chemistry.abiogenesis_cap_q16" => chemistry.abiogenesis_cap_q16: u32,
    "chemistry.abiogenesis_seed_milli" => chemistry.abiogenesis_seed_milli: i64,
    "chemistry.microbial_enabled" => chemistry.microbial_enabled: bool,
    "chemistry.replication_axis" => chemistry.replication_axis: u32,
    "chemistry.aggregation_axis" => chemistry.aggregation_axis: u32,
    "chemistry.growth_rate_low_q16" => chemistry.growth_rate_low_q16: u32,
    "chemistry.growth_rate_high_q16" => chemistry.growth_rate_high_q16: u32,
    "chemistry.growth_yield_q16" => chemistry.growth_yield_q16: u32,
    "chemistry.death_q16" => chemistry.death_q16: u32,
    "chemistry.death_waste_fraction_q16" => chemistry.death_waste_fraction_q16: u32,
    "chemistry.mutation_q16" => chemistry.mutation_q16: u32,
    "chemistry.excretion_fraction_q16" => chemistry.excretion_fraction_q16: u32,
    "chemistry.remains_fraction_q16" => chemistry.remains_fraction_q16: u32,
    "chemistry.consumption_fraction_q16" => chemistry.consumption_fraction_q16: u32,
    "chemistry.consumption_yield_q16" => chemistry.consumption_yield_q16: u32,
    "transition.enabled" => transition.enabled: bool,
    "transition.check_interval_ticks" => transition.check_interval_ticks: u64,
    "transition.density_floor_milli" => transition.density_floor_milli: i64,
    "transition.persistence_checks" => transition.persistence_checks: u32,
    "transition.aggregation_step_min" => transition.aggregation_step_min: u32,
    "transition.organism_energy_milli" => transition.organism_energy_milli: i64,
    "transition.max_organisms_per_event" => transition.max_organisms_per_event: u32,
    "transition.max_materializations_per_tick" => transition.max_materializations_per_tick: u32,
    "genome2.enabled" => genome2.enabled: bool,
    "genome2.caps.max_loci_per_chromosome" => genome2.caps.max_loci_per_chromosome: u32,
    "genome2.caps.max_nodes" => genome2.caps.max_nodes: u32,
    "genome2.caps.max_edges" => genome2.caps.max_edges: u32,
    "genome2.caps.max_edges_per_node" => genome2.caps.max_edges_per_node: u32,
    "genome2.caps.max_genome_bytes" => genome2.caps.max_genome_bytes: u32,
    "genome2.meiosis.max_extra_crossovers" => genome2.meiosis.max_extra_crossovers: u32,
    "genome2.mutation.point_q16" => genome2.mutation.point_q16: u32,
    "genome2.mutation.duplication_q16" => genome2.mutation.duplication_q16: u32,
    "genome2.mutation.deletion_q16" => genome2.mutation.deletion_q16: u32,
    "genome2.mutation.insertion_q16" => genome2.mutation.insertion_q16: u32,
    "genome2.mutation.transposition_q16" => genome2.mutation.transposition_q16: u32,
    "genome2.mutation.max_run" => genome2.mutation.max_run: u32,
    // Settable because Phase 11's C11.2 bar is *anchored* to it: the smallest
    // excess over the neutral marker that counts is one expected mutational
    // step, which is half of `point_delta_q16 / 65536` of the value's range.
    // A campaign that states that bar and cannot pin the constant it is
    // computed from would have its threshold moved by a later revision of a
    // default - the coupling D-078 removed for Phase 9's caps. The codec has
    // always carried the field; only the registry entry was missing, which is
    // exactly the "visible gap rather than a silent one" standing rule 3
    // describes.
    "genome2.mutation.point_delta_q16" => genome2.mutation.point_delta_q16: u32,
    // Phase 12's `bind` operator (D-114). Zero by default and hashed only
    // when nonzero; every Phase 12 condition arm sets the same value, so it
    // is common-mode for C12.1-C12.3 and has to be expressible as a `base`
    // line.
    "genome2.mutation.binding_q16" => genome2.mutation.binding_q16: u32,
    "genome2.mutation.regulatory_enabled" => genome2.mutation.regulatory_enabled: bool,
    // Phase 11's A/B ablation lives on this one flag, so without the entry
    // the phase's two conditions are not expressible as a campaign at all.
    "genome2.mutation.plasticity_enabled" => genome2.mutation.plasticity_enabled: bool,
    // Phase 11. `plasticity.enabled` and
    // `genome2.mutation.plasticity_enabled` are separate fields on purpose:
    // condition A sets both, condition B sets neither, and a third condition
    // that ran the learn phase over frozen plasticity genes is a distinct
    // experiment that has to be expressible to be ruled out.
    //
    // `lamarckian_fraction_q16` is here so a campaign that adopts the
    // declared experimental condition records it as a named delta in the
    // manifest, which is what makes the reporting obligation enforceable
    // rather than a convention. Validation refuses a nonzero value today.
    "plasticity.enabled" => plasticity.enabled: bool,
    "plasticity.plastic_edge_cost_milli_per_s" => plasticity.plastic_edge_cost_milli_per_s: i64,
    "plasticity.max_plastic_edges" => plasticity.max_plastic_edges: u32,
    "plasticity.lamarckian_fraction_q16" => plasticity.lamarckian_fraction_q16: u32,
    // `live_rule_zero` is registered in the same change that encodes it, not
    // in the later one that gives it behaviour, and the order is what makes
    // `config_field_coverage.rs` defend it from the start: a field that is
    // settable but unregistered is a field the sweep never perturbs, which is
    // how the whole genome2 section went two phases undefended. Validation
    // refuses `true` until the kernel half lands, exactly as it refuses a
    // nonzero `lamarckian_fraction_q16`, so the sweep's `false -> true`
    // perturbation exercises the codec without ever building such a world.
    "plasticity.live_rule_zero" => plasticity.live_rule_zero: bool,
    // The moat. Registered beside the chain because D-107's 2x2 crosses them
    // and both arms have to be expressible as a campaign delta; a factor that
    // cannot be named in a campaign file cannot be an arm.
    "plasticity.price_moved_edges_only" => plasticity.price_moved_edges_only: bool,
    // Phase 12. `worldmod.enabled` and `worldmod.patch_enabled` are separate
    // fields on purpose, and so is the scale: the phase's three arms are
    // "section off", "section on, schedule on, scale 1.0" and "section on,
    // schedule on, scale 2.0", and only the last two are matched. A control
    // that turned the schedule *off* would differ from its treatment by the
    // whole capacity-loss sink as well as by the move, so the arm that
    // matters is expressible only because the scale is its own field.
    "worldmod.enabled" => worldmod.enabled: bool,
    "worldmod.dense_threshold_q16" => worldmod.dense_threshold_q16: u32,
    "worldmod.max_traversable_overrides" => worldmod.max_traversable_overrides: u32,
    "worldmod.max_capacity_overrides" => worldmod.max_capacity_overrides: u32,
    "worldmod.max_material_overrides" => worldmod.max_material_overrides: u32,
    "worldmod.patch_enabled" => worldmod.patch_enabled: bool,
    "worldmod.relocate_interval_ticks" => worldmod.relocate_interval_ticks: u64,
    "worldmod.patch_radius_cells" => worldmod.patch_radius_cells: u32,
    "worldmod.patch_capacity_scale_q16" => worldmod.patch_capacity_scale_q16: u32,
    // Phase 11 measurement section. All three are here because standing rule
    // 3 makes the coverage sweep drive itself from this list: a settable
    // field absent from it is protected by nothing, and a config section has
    // now been lost that way three times (D-065, D-086, and the plasticity
    // gate). The sub-gates are separate fields for the same reason
    // `worldmod.patch_enabled` is - a campaign that wants the drift control
    // without a per-organism histogram in every snapshot has to be able to
    // say so, and so does one that wants the histogram in a schema-1 world
    // where there is no genome to put a marker in.
    "probe.enabled" => probe.enabled: bool,
    "probe.action_census_enabled" => probe.action_census_enabled: bool,
    "probe.marker_locus_enabled" => probe.marker_locus_enabled: bool,
    "morphology.enabled" => morphology.enabled: bool,
    "morphology.base_node_budget" => morphology.base_node_budget: u32,
    "morphology.caps.max_modules" => morphology.caps.max_modules: u16,
    "morphology.caps.max_growth_steps" => morphology.caps.max_growth_steps: u16,
    "morphology.caps.lattice_radius" => morphology.caps.lattice_radius: i16,
    // Phase 12 artifact section (ADR-0028). Every field, because the sweep
    // in `config_field_coverage.rs` drives itself from this list and a field
    // absent from it is defended by nothing; and because the four campaign
    // conditions are `set` lines on `inert`, `ephemeral` and
    // `max_composition_depth`, which therefore have to be nameable.
    "artifact.enabled" => artifact.enabled: bool,
    "artifact.inert" => artifact.inert: bool,
    "artifact.ephemeral" => artifact.ephemeral: bool,
    "artifact.max_objects" => artifact.max_objects: u32,
    "artifact.max_objects_per_cell" => artifact.max_objects_per_cell: u32,
    "artifact.max_composition_depth" => artifact.max_composition_depth: u32,
    "artifact.max_composition_breadth" => artifact.max_composition_breadth: u32,
    "artifact.max_held_objects" => artifact.max_held_objects: u32,
    "artifact.max_candidates" => artifact.max_candidates: u32,
    "artifact.carry_capacity_milli" => artifact.carry_capacity_milli: i64,
    "artifact.carry_move_cost_q16" => artifact.carry_move_cost_q16: u32,
    "artifact.hold_cost_milli_per_s" => artifact.hold_cost_milli_per_s: i64,
    "artifact.action_cost_milli" => artifact.action_cost_milli: i64,
    "artifact.strike_cost_milli" => artifact.strike_cost_milli: i64,
    "artifact.action_threshold_q16" => artifact.action_threshold_q16: i32,
    "artifact.reach_m" => artifact.reach_m: u32,
    "artifact.consume_reach_m" => artifact.consume_reach_m: u32,
    "artifact.perception_range_m" => artifact.perception_range_m: u32,
    "artifact.strike_force_q16" => artifact.strike_force_q16: u32,
    "artifact.strike_mass_reference_milli" => artifact.strike_mass_reference_milli: i64,
    "artifact.fracture_margin_q16" => artifact.fracture_margin_q16: u32,
    "artifact.max_fragments" => artifact.max_fragments: u32,
    "artifact.min_fragment_mass_milli" => artifact.min_fragment_mass_milli: i64,
    "artifact.joint_floor_q16" => artifact.joint_floor_q16: u32,
    "artifact.blocking_mass_milli" => artifact.blocking_mass_milli: i64,
    "artifact.terrain_yield_milli" => artifact.terrain_yield_milli: i64,
    "artifact.extraction_milli" => artifact.extraction_milli: i64,
    "artifact.yield_regen_milli" => artifact.yield_regen_milli: i64,
    "artifact.yield_regen_interval_ticks" => artifact.yield_regen_interval_ticks: u64,
    "artifact.stone_relative_q16" => artifact.stone_relative_q16: u32,
    "artifact.wood_relative_q16" => artifact.wood_relative_q16: u32,
    "social.enabled" => social.enabled: bool,
    "social.perception_enabled" => social.perception_enabled: bool,
    "social.signal_enabled" => social.signal_enabled: bool,
    "social.scramble_delivery" => social.scramble_delivery: bool,
    "social.observational_enabled" => social.observational_enabled: bool,
    "social.perception_k" => social.perception_k: u32,
    "social.perception_radius_m" => social.perception_radius_m: u32,
    "social.signal_channels" => social.signal_channels: u32,
    "social.signal_base_range_m" => social.signal_base_range_m: u32,
    "social.signal_cost_milli" => social.signal_cost_milli: i64,
    "social.signal_retain_q16" => social.signal_retain_q16: u32,
    "social.signal_corruption_q16" => social.signal_corruption_q16: u32,
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
