//! The microbial half of the field regime (Phase 15, ADR-0031,
//! `lifesim-microbial-v1`).
//!
//! Per-cell densities over a bounded genotype-class registry - the
//! ADR-0020 discretization, a deliberate realism loss the record owns.
//! Classes are the cross product of three axes (substrate preference,
//! replication rate, aggregation tendency) generated in a fixed order, so
//! a class id is permanent for a given axis configuration and the axis
//! sizes are inside the config hash.
//!
//! Every flow names its source and sink, all of them inside the one field
//! ledger the chemistry half carries, so the JOINT identity
//!
//!   chemistry total + microbial total == produced + deposited
//!
//! holds to the milli-unit: growth moves substrate mass into density
//! (with the yield remainder going to waste in the same step), death
//! moves density back into waste and primordial, mutation moves density
//! between classes, and abiogenesis moves primordial into the founder
//! class. Nothing is created or destroyed anywhere in the field.

use crate::checksum::Fnv1a64;
use crate::chemistry::{ChemistryState, S_MONOMER, S_PRIMORDIAL, S_WASTE, SUBSTRATE_COUNT};
use crate::config::ChemistryConfig;
use crate::rng::{RngSystem, named_random};

pub const MICROBIAL_POLICY_VERSION: &str = "lifesim-microbial-v1";
/// Bumped whenever the axis semantics or generation order change; enters
/// the config hash.
pub const CLASS_REGISTRY_VERSION: u16 = 1;

/// One genotype class's expressed parameters, derived from its id.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClassParameters {
    /// Which substrate this class consumes: S_PRIMORDIAL or S_MONOMER.
    pub substrate: usize,
    /// Position on the replication axis, 0-based.
    pub replication_step: u32,
    /// Position on the aggregation axis, 0-based.
    pub aggregation_step: u32,
}

/// Classes generate in ascending id = ((pref * replication_axis) +
/// replication_step) * aggregation_axis + aggregation_step - the fixed
/// order that makes ids permanent for a given axis configuration.
pub fn class_parameters(config: &ChemistryConfig, class: usize) -> ClassParameters {
    let aggregation = config.aggregation_axis as usize;
    let replication = config.replication_axis as usize;
    let aggregation_step = (class % aggregation) as u32;
    let replication_step = ((class / aggregation) % replication) as u32;
    let preference = class / (aggregation * replication);
    ClassParameters {
        substrate: if preference == 0 {
            S_PRIMORDIAL
        } else {
            S_MONOMER
        },
        replication_step,
        aggregation_step,
    }
}

pub fn class_count(config: &ChemistryConfig) -> usize {
    2 * config.replication_axis as usize * config.aggregation_axis as usize
}

/// The class's growth rate: linear interpolation between the low and high
/// ends of the replication axis, exact in integers.
fn growth_rate_q16(config: &ChemistryConfig, replication_step: u32) -> i64 {
    let low = i64::from(config.growth_rate_low_q16);
    let high = i64::from(config.growth_rate_high_q16);
    let steps = i64::from(config.replication_axis - 1).max(1);
    low + (high - low) * i64::from(replication_step) / steps
}

/// Per-world microbial state. `None` exactly when the microbial gate is
/// off.
#[derive(Clone, Debug, PartialEq)]
pub struct MicrobialState {
    /// Densities, milli, flattened `cell * class_count + class`.
    pub densities: Vec<i64>,
    /// Scratch for the mutation pass (the only pass that moves density
    /// between slots of the same cell); never saved or hashed.
    scratch: Vec<i64>,
    /// Derived per-class caches, built once from the config: the field
    /// runs these lookups per occupied slot per step, and recomputing
    /// the id decode (and allocating neighbour lists) there dominated
    /// the saturated-field tick. Same arithmetic, hoisted - never saved
    /// or hashed.
    class_substrate: Vec<usize>,
    class_growth_rate_q16: Vec<i64>,
    class_neighbours: Vec<Vec<usize>>,

    /// Counters: internal transfers, so they do not enter the joint
    /// identity - they exist so a census can attribute what moved.
    pub grown_milli_total: i128,
    pub died_milli_total: i128,
    pub mutated_milli_total: i128,
}

impl MicrobialState {
    pub fn new(cells: usize, config: &ChemistryConfig) -> Self {
        let classes = class_count(config);
        let slots = cells * classes;
        Self {
            densities: vec![0; slots],
            scratch: vec![0; slots],
            class_substrate: (0..classes)
                .map(|class| class_parameters(config, class).substrate)
                .collect(),
            class_growth_rate_q16: (0..classes)
                .map(|class| {
                    growth_rate_q16(config, class_parameters(config, class).replication_step)
                })
                .collect(),
            class_neighbours: (0..classes)
                .map(|class| neighbour_classes(config, class))
                .collect(),
            grown_milli_total: 0,
            died_milli_total: 0,
            mutated_milli_total: 0,
        }
    }

    pub fn rebuild_derived(&mut self) {
        self.scratch = vec![0; self.densities.len()];
    }

    pub fn total_milli(&self) -> i128 {
        self.densities.iter().map(|&value| i128::from(value)).sum()
    }

    /// One microbial pass over every cell: growth, death, then mutation
    /// flow, all exact. Runs inside the chemistry field step, after the
    /// abiotic reactions and before production.
    pub fn step(&mut self, chemistry: &mut ChemistryState, config: &ChemistryConfig) {
        let classes = class_count(config);
        let cells = self.densities.len() / classes;
        let yield_q16 = i64::from(config.growth_yield_q16);
        let death_q16 = i64::from(config.death_q16);
        let waste_fraction = i64::from(config.death_waste_fraction_q16);
        for cell in 0..cells {
            for class in 0..classes {
                let slot = cell * classes + class;
                let density = self.densities[slot];
                if density > 0 {
                    // Growth: consume preferred substrate, bounded by what
                    // the cell holds; yield becomes density, the remainder
                    // is metabolic loss into waste - same cell, same step.
                    let rate = self.class_growth_rate_q16[class];
                    let substrate_slot = cell * SUBSTRATE_COUNT + self.class_substrate[class];
                    let appetite = (density * rate) >> 16;
                    let consumed = appetite.min(chemistry.concentrations[substrate_slot]);
                    if consumed > 0 {
                        let gained = (consumed * yield_q16) >> 16;
                        chemistry.concentrations[substrate_slot] -= consumed;
                        self.densities[slot] += gained;
                        chemistry.concentrations[cell * SUBSTRATE_COUNT + S_WASTE] +=
                            consumed - gained;
                        self.grown_milli_total += i128::from(gained);
                    }
                }
                // Death runs on the post-growth density.
                let density = self.densities[slot];
                if density > 0 {
                    let died = (density * death_q16) >> 16;
                    if died > 0 {
                        let to_waste = (died * waste_fraction) >> 16;
                        self.densities[slot] -= died;
                        chemistry.concentrations[cell * SUBSTRATE_COUNT + S_WASTE] += to_waste;
                        chemistry.concentrations[cell * SUBSTRATE_COUNT + S_PRIMORDIAL] +=
                            died - to_waste;
                        self.died_milli_total += i128::from(died);
                    }
                }
            }
        }
        self.mutate(config);
    }

    /// Mutation flow between single-axis-step neighbour classes, in
    /// ascending `(cell, source, target)`, double-buffered so the pass
    /// reads only committed densities.
    fn mutate(&mut self, config: &ChemistryConfig) {
        if config.mutation_q16 == 0 {
            return;
        }
        let classes = class_count(config);
        let cells = self.densities.len() / classes;
        let rate = i64::from(config.mutation_q16);
        self.scratch.copy_from_slice(&self.densities);
        for cell in 0..cells {
            for source in 0..classes {
                let density = self.densities[cell * classes + source];
                if density <= 0 {
                    continue;
                }
                let flow = (density * rate) >> 16;
                if flow == 0 {
                    continue;
                }
                for &target in &self.class_neighbours[source] {
                    self.scratch[cell * classes + source] -= flow;
                    self.scratch[cell * classes + target] += flow;
                    self.mutated_milli_total += i128::from(flow);
                }
            }
        }
        std::mem::swap(&mut self.densities, &mut self.scratch);
    }

    /// Abiogenesis for one field step: per cell, the capped weighted rate
    /// against a draw on the `Abiogenesis` stream; a firing transfers seed
    /// mass from S_PRIMORDIAL into the founder class (the lowest id, which
    /// by generation order prefers S_PRIMORDIAL), bounded by what the cell
    /// holds - genesis conserves.
    pub fn abiogenesis(
        &mut self,
        chemistry: &mut ChemistryState,
        config: &ChemistryConfig,
        world_seed: u64,
        tick: u64,
        field_step: u32,
    ) {
        if !config.abiogenesis_enabled {
            return;
        }
        let classes = class_count(config);
        let cells = self.densities.len() / classes;
        for cell in 0..cells {
            let base = cell * SUBSTRATE_COUNT;
            let rate = ((i64::from(config.abiogenesis_weight_primordial_q16)
                * chemistry.concentrations[base + S_PRIMORDIAL]
                + i64::from(config.abiogenesis_weight_monomer_q16)
                    * chemistry.concentrations[base + S_MONOMER]
                + i64::from(config.abiogenesis_weight_polymer_q16)
                    * chemistry.concentrations[base + crate::chemistry::S_POLYMER])
                / 1_000)
                .clamp(0, i64::from(config.abiogenesis_cap_q16));
            if rate == 0 {
                continue;
            }
            let draw = named_random(
                world_seed,
                tick,
                RngSystem::Abiogenesis,
                cell as u64,
                field_step,
            ) & 0xffff;
            if (draw as i64) < rate {
                let seed = config
                    .abiogenesis_seed_milli
                    .min(chemistry.concentrations[base + S_PRIMORDIAL]);
                if seed > 0 {
                    chemistry.concentrations[base + S_PRIMORDIAL] -= seed;
                    self.densities[cell * classes] += seed;
                    chemistry.seeded_out_milli += i128::from(seed);
                    chemistry.abiogenesis_fired_total += 1;
                }
            }
        }
    }

    pub fn hash_into(&self, hasher: &mut Fnv1a64) {
        hasher.update(b"lifesim-microbial-state-v1");
        for &value in &self.densities {
            hasher.update_i64(value);
        }
        hasher.update_i128(self.grown_milli_total);
        hasher.update_i128(self.died_milli_total);
        hasher.update_i128(self.mutated_milli_total);
    }
}

/// The single-axis-step neighbours of a class, ascending id order.
fn neighbour_classes(config: &ChemistryConfig, class: usize) -> Vec<usize> {
    let aggregation = config.aggregation_axis as usize;
    let replication = config.replication_axis as usize;
    let parameters = class_parameters(config, class);
    let mut out = Vec::new();
    let preference = class / (aggregation * replication);
    // Preference axis (two positions): the same class in the other half.
    let other_preference =
        (1 - preference) * aggregation * replication + class % (aggregation * replication);
    out.push(other_preference);
    if parameters.replication_step > 0 {
        out.push(class - aggregation);
    }
    if (parameters.replication_step as usize) + 1 < replication {
        out.push(class + aggregation);
    }
    if parameters.aggregation_step > 0 {
        out.push(class - 1);
    }
    if (parameters.aggregation_step as usize) + 1 < aggregation {
        out.push(class + 1);
    }
    out.sort_unstable();
    out
}

/// The joint field identity's defect: exactly zero in a correct world.
pub fn field_conservation_defect_milli(
    chemistry: &ChemistryState,
    microbial: &MicrobialState,
) -> i128 {
    chemistry.produced_milli + chemistry.deposited_milli
        - chemistry.total_milli()
        - microbial.total_milli()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> ChemistryConfig {
        let mut config = ChemistryConfig::chemistry_default();
        config.enabled = true;
        config.microbial_enabled = true;
        config.abiogenesis_enabled = true;
        // The default mutation rate (0.001) truncates to zero below 993
        // milli of density; the seeded densities here sit near 1000 with
        // death running first, so use a rate that actually moves mass at
        // test densities (still inside the Q16/8 validation cap).
        config.mutation_q16 = 4_096;
        config
    }

    #[test]
    fn class_ids_cover_the_cross_product_in_a_fixed_order() {
        let config = config();
        assert_eq!(class_count(&config), 8);
        let first = class_parameters(&config, 0);
        assert_eq!(first.substrate, S_PRIMORDIAL);
        assert_eq!(first.replication_step, 0);
        assert_eq!(first.aggregation_step, 0);
        let last = class_parameters(&config, 7);
        assert_eq!(last.substrate, S_MONOMER);
        assert_eq!(last.replication_step, 1);
        assert_eq!(last.aggregation_step, 1);
    }

    #[test]
    fn neighbours_differ_in_exactly_one_axis_step() {
        let config = config();
        for class in 0..class_count(&config) {
            let from = class_parameters(&config, class);
            for &target in &neighbour_classes(&config, class) {
                let to = class_parameters(&config, target);
                let differences = usize::from(from.substrate != to.substrate)
                    + usize::from(from.replication_step != to.replication_step)
                    + usize::from(from.aggregation_step != to.aggregation_step);
                assert_eq!(differences, 1, "class {class} -> {target}");
            }
        }
    }

    #[test]
    fn the_joint_identity_holds_through_growth_death_mutation_and_genesis() {
        let config = config();
        let mut chemistry = ChemistryState::new(8, 8, &config);
        let mut microbial = MicrobialState::new(64, &config);
        for tick in 1..=400 {
            chemistry.step(8, 8, &config);
            microbial.step(&mut chemistry, &config);
            microbial.abiogenesis(&mut chemistry, &config, 0x5eed, tick, 0);
            assert_eq!(
                field_conservation_defect_milli(&chemistry, &microbial),
                0,
                "tick {tick}"
            );
        }
        assert!(
            chemistry.abiogenesis_fired_total > 0,
            "abiogenesis never fired, so the test pinned nothing about it"
        );
        assert!(microbial.total_milli() > 0, "no density ever existed");
        assert!(microbial.grown_milli_total > 0, "growth never ran");
        assert!(microbial.died_milli_total > 0, "death never ran");
        assert!(microbial.mutated_milli_total > 0, "mutation never flowed");
    }

    #[test]
    fn abiogenesis_disabled_seeds_nothing_ever() {
        let mut config = config();
        config.abiogenesis_enabled = false;
        let mut chemistry = ChemistryState::new(4, 4, &config);
        let mut microbial = MicrobialState::new(16, &config);
        for tick in 1..=200 {
            chemistry.step(4, 4, &config);
            microbial.step(&mut chemistry, &config);
            microbial.abiogenesis(&mut chemistry, &config, 0x5eed, tick, 0);
        }
        assert_eq!(chemistry.abiogenesis_fired_total, 0);
        assert_eq!(microbial.total_milli(), 0);
    }
}
