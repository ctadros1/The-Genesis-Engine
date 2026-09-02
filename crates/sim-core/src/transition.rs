//! The field-to-individual transition (Phase 16, ADR-0032,
//! `lifesim-transition-v1`).
//!
//! One representation change and nothing else: microbial density that has
//! sat above a floor for a stated number of checks, in a class at or above
//! a stated aggregation position, in a cell an organism can stand on,
//! becomes one-module organisms in Phase 10's morphospace. There is no
//! multicellularity mechanic here - nothing reads a module count, nothing
//! grants anything for crossing anything (plan C16.7). What happens after
//! admission is ordinary structural mutation under ordinary selection.
//!
//! Everything the transition adds to the world's identities is one number
//! counted on both sides: `materialized_milli` is debited from the field
//! and credited to organisms in one operation, so the joint field identity
//! and the organism energy identity both gain the same term and
//! `World::check_invariants` closes the conversion exactly (C16.1's
//! in-run half).
//!
//! The class-to-genome map is **constant across classes** in v1, on
//! purpose (ADR-0032): no genome locus corresponds to any class axis, and
//! a map that read the axes into traits would be exactly the "quietly
//! encodes a good starting organism" risk the plan names. The registry
//! stays consequential where it should - which slots trigger.

use crate::checksum::Fnv1a64;
use crate::config::{ChemistryConfig, MorphologyConfig, SimConfig, TransitionConfig};
use crate::develop::{
    ACT_DIFFERENTIATE, COND_SELF_TYPE, DevelopCounters, OP_GE, Regulatory, grow,
};
use crate::genome2::Genome2;
use crate::microbial::{class_count, class_parameters};
use crate::morphology::{Body, DerivedBody, TYPE_DIGESTIVE, TYPE_STRUCTURAL};

pub const TRANSITION_POLICY_VERSION: &str = "lifesim-transition-v1";
/// Bumped whenever the class-to-genome map or the unicell program changes
/// meaning; enters the config hash (C16.4's "versioned").
pub const GENOME_MAP_VERSION: u16 = 1;

/// The unicell growth program: the founder program's first rule alone -
/// the origin module differentiates into a digestive module and nothing
/// else grows. Same homology slot as the founder's first rule, so a
/// materialized genome and a founder genome recombine over a shared
/// locus rather than two unrelated ones.
pub fn unicell_program() -> Vec<(u32, Regulatory)> {
    let base = crate::genome2::STRUCTURAL_HOMOLOGY_BASE + 20_000;
    vec![(
        base,
        Regulatory {
            condition_kind: COND_SELF_TYPE,
            condition_op: OP_GE,
            condition_param: TYPE_STRUCTURAL,
            threshold: 0,
            action_kind: ACT_DIFFERENTIATE,
            action_type: TYPE_DIGESTIVE,
            direction: 0,
            scale_milli: 1_000,
        },
    )]
}

/// The unicell body under a morphology configuration: one digestive
/// module at the origin. Pure - the program and the registry are build
/// constants - so it is the same in every world and needs no saving.
pub fn unicell_body(config: &MorphologyConfig) -> Body {
    let mut counters = DevelopCounters::default();
    grow(&unicell_program(), config.lattice, &config.caps, &mut counters)
}

/// The unicell's derived attributes, for the energy-capacity bound the
/// configuration must respect and for the neutrality test.
pub fn unicell_derived(config: &MorphologyConfig) -> DerivedBody {
    unicell_body(config).derive()
}

/// The class-to-genome map (`GENOME_MAP_VERSION` 1): the minimal
/// schema-2 founder at every trait 0.5 plus the unicell program, then
/// whatever loci this world's other gates layer onto founders, in the
/// order the founder path layers them (marker, then preference band), so
/// a materialized genome is structurally identical to a founder's in the
/// same world.
///
/// `class` is accepted and deliberately unread: the map is constant
/// across classes in this version (ADR-0032), and taking the argument
/// keeps the signature the specification names so a later version that
/// does read it is a map change, not an interface change.
pub fn synthesize_genome(config: &SimConfig, class: usize) -> Genome2 {
    let _ = class;
    let mut genome = crate::structmut::minimal_founder(&[0.5; crate::genome::TRAIT_COUNT]);
    for (homology_id, rule) in unicell_program() {
        let locus = crate::genome2::Locus {
            homology_id,
            gene_lineage_id: u64::from(homology_id),
            mutation_event_id: 0,
            kind: crate::genome2::LocusKind::Regulatory { rule },
        };
        for haplotype in &mut genome.haplotypes {
            for chromosome in &mut haplotype.chromosomes {
                chromosome.push(locus);
            }
        }
    }
    for haplotype in &mut genome.haplotypes {
        for chromosome in &mut haplotype.chromosomes {
            chromosome.sort_unstable_by_key(|locus| locus.homology_id);
        }
    }
    let genome = if config.probe.enabled && config.probe.marker_locus_enabled {
        crate::schema2::with_marker_locus(genome)
    } else {
        genome
    };
    if config.physiology.enabled && config.physiology.mate_choice_enabled {
        crate::schema2::with_preference_loci(genome)
    } else {
        genome
    }
}

/// One slot that met the trigger at this check, in ascending `(cell,
/// class)` order. The biomass to convert is read from the density at
/// materialization time, not here, so a deferred slot converts what it
/// holds when it is finally admitted.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Trigger {
    pub cell: usize,
    pub class: usize,
}

/// Per-world transition state. `None` exactly when the gate is off.
#[derive(Clone, Debug, PartialEq)]
pub struct TransitionState {
    /// Consecutive checks at or above the density floor, per slot
    /// (`cell * class_count + class`). Real state: the plan's persistence
    /// window cannot be recomputed from densities, so it is saved and
    /// hashed (ADR-0032's recorded deviation from "no new section").
    pub persistence: Vec<u32>,
    /// Organisms admitted by materialization, whole run. Enters the
    /// population and entity-ID identities.
    pub materialized_total: u64,
    /// `(cell, class)` triggers that materialized, whole run.
    pub events_total: u64,
    /// Energy credited to materialized organisms == density debited from
    /// the field, whole run. The one number counted on both sides of the
    /// conversion.
    pub materialized_milli: i128,
    /// Triggers deferred because the per-tick cap was reached (kept their
    /// persistence, re-evaluated at the next check).
    pub deferred_cap_total: u64,
    /// Triggers deferred because `max_entities` would be exceeded. A
    /// separate counter from the cap: "the world was full" and "the tick
    /// was full" are different findings (D-074).
    pub deferred_capacity_total: u64,
    /// Admissions the schema-2 push refused. Expected to stay at zero -
    /// the synthesized genome is validated at world construction - so a
    /// nonzero value is a bug report, and a test asserts it is zero.
    pub refused_total: u64,
    /// Per-class eligibility under `aggregation_step_min`, derived from
    /// config once; never saved or hashed.
    class_eligible: Vec<bool>,
}

impl TransitionState {
    pub fn new(cells: usize, chemistry: &ChemistryConfig, config: &TransitionConfig) -> Self {
        let classes = class_count(chemistry);
        Self {
            persistence: vec![0; cells * classes],
            materialized_total: 0,
            events_total: 0,
            materialized_milli: 0,
            deferred_cap_total: 0,
            deferred_capacity_total: 0,
            refused_total: 0,
            class_eligible: (0..classes)
                .map(|class| {
                    class_parameters(chemistry, class).aggregation_step
                        >= config.aggregation_step_min
                })
                .collect(),
        }
    }

    /// Rebuild the derived members after a restore installed the saved
    /// arrays.
    pub fn rebuild_derived(&mut self, chemistry: &ChemistryConfig, config: &TransitionConfig) {
        let classes = class_count(chemistry);
        self.class_eligible = (0..classes)
            .map(|class| {
                class_parameters(chemistry, class).aggregation_step >= config.aggregation_step_min
            })
            .collect();
    }

    /// One check: advance every slot's persistence counter against the
    /// floor, and return the slots that meet the whole trigger, in
    /// ascending `(cell, class)`. `traversable` says whether an organism
    /// may stand in a cell; density in a cell that fails it keeps
    /// counting but never triggers - it stays density, recorded rather
    /// than smoothed over.
    pub fn check(
        &mut self,
        densities: &[i64],
        classes: usize,
        config: &TransitionConfig,
        traversable: impl Fn(usize) -> bool,
    ) -> Vec<Trigger> {
        debug_assert_eq!(densities.len(), self.persistence.len());
        let mut triggers = Vec::new();
        for (slot, &density) in densities.iter().enumerate() {
            let counter = &mut self.persistence[slot];
            if density >= config.density_floor_milli {
                *counter = counter.saturating_add(1);
            } else {
                *counter = 0;
                continue;
            }
            if *counter < config.persistence_checks {
                continue;
            }
            let cell = slot / classes;
            let class = slot % classes;
            if !self.class_eligible[class] || !traversable(cell) {
                continue;
            }
            triggers.push(Trigger { cell, class });
        }
        triggers
    }

    pub fn hash_into(&self, hasher: &mut Fnv1a64) {
        hasher.update(b"lifesim-transition-state-v1");
        for &value in &self.persistence {
            hasher.update_u32(value);
        }
        hasher.update_u64(self.materialized_total);
        hasher.update_u64(self.events_total);
        hasher.update_i128(self.materialized_milli);
        hasher.update_u64(self.deferred_cap_total);
        hasher.update_u64(self.deferred_capacity_total);
        hasher.update_u64(self.refused_total);
    }

    pub fn to_save(&self) -> TransitionSave {
        TransitionSave {
            persistence: self.persistence.clone(),
            materialized_total: self.materialized_total,
            events_total: self.events_total,
            materialized_milli: self.materialized_milli,
            deferred_cap_total: self.deferred_cap_total,
            deferred_capacity_total: self.deferred_capacity_total,
            refused_total: self.refused_total,
        }
    }
}

/// The transition's saved half: the persistence counters plus every
/// counter that enters an identity or a report. Exhaustive: a field
/// added to the state and not here would restore silently wrong
/// (D-077's lesson), so `to_save` and the restore path both spell every
/// field.
#[derive(Clone, Debug, PartialEq)]
pub struct TransitionSave {
    pub persistence: Vec<u32>,
    pub materialized_total: u64,
    pub events_total: u64,
    pub materialized_milli: i128,
    pub deferred_cap_total: u64,
    pub deferred_capacity_total: u64,
    pub refused_total: u64,
}

/// The joint field identity with the transition's term: exactly zero in a
/// correct world. `produced + deposited - materialized_out == chemistry +
/// microbial`.
pub fn field_conservation_defect_milli(
    chemistry: &crate::chemistry::ChemistryState,
    microbial: &crate::microbial::MicrobialState,
    transition: &TransitionState,
) -> i128 {
    crate::microbial::field_conservation_defect_milli(chemistry, microbial)
        - transition.materialized_milli
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::morphology::ModuleType;

    #[test]
    fn the_unicell_is_one_digestive_module_with_intake_and_no_thrust() {
        let config = MorphologyConfig::morphology_default();
        let body = unicell_body(&config);
        assert_eq!(body.len(), 1);
        assert_eq!(body.count_of(ModuleType::Digestive), 1);
        assert!(body.validate(config.lattice, &config.caps).is_ok());
        let derived = body.derive();
        assert!(derived.intake_milli > 0, "a gut must eat");
        assert_eq!(derived.thrust_milli, 0, "no motor, no thrust");
        assert_eq!(derived.sensory_modules, 0, "no sensor");
        assert!(derived.energy_capacity_milli > 0);
    }

    #[test]
    fn the_map_is_deterministic_and_constant_across_classes() {
        let config = SimConfig::phase2_default(0x5eed);
        let first = synthesize_genome(&config, 0).encode();
        for class in 0..8 {
            assert_eq!(synthesize_genome(&config, class).encode(), first);
        }
        // And it is the unicell program's genome: it grows to one module.
        let morphology = MorphologyConfig::morphology_default();
        let mut counters = DevelopCounters::default();
        let body = crate::develop::develop(
            &synthesize_genome(&config, 3),
            morphology.lattice,
            &morphology.caps,
            &mut counters,
        )
        .expect("the synthesized genome develops");
        assert_eq!(body, unicell_body(&morphology));
    }

    fn chemistry() -> ChemistryConfig {
        let mut chemistry = ChemistryConfig::chemistry_default();
        chemistry.enabled = true;
        chemistry.microbial_enabled = true;
        chemistry
    }

    fn transition() -> TransitionConfig {
        let mut config = TransitionConfig::transition_default();
        config.enabled = true;
        config.density_floor_milli = 100;
        config.persistence_checks = 3;
        config.organism_energy_milli = 50;
        config
    }

    #[test]
    fn a_slot_triggers_only_after_the_persistence_window_and_resets_on_a_dip() {
        let chemistry = chemistry();
        let config = transition();
        let classes = class_count(&chemistry);
        let mut state = TransitionState::new(2, &chemistry, &config);
        // Slot (cell 1, class 1): aggregation step 1 under the default
        // 2x2x2 registry, eligible under aggregation_step_min 1.
        let slot = classes + 1;
        let mut densities = vec![0_i64; 2 * classes];
        densities[slot] = 100;
        assert!(state.check(&densities, classes, &config, |_| true).is_empty());
        assert!(state.check(&densities, classes, &config, |_| true).is_empty());
        let third = state.check(&densities, classes, &config, |_| true);
        assert_eq!(third, vec![Trigger { cell: 1, class: 1 }]);
        // A dip below the floor resets the window.
        densities[slot] = 99;
        assert!(state.check(&densities, classes, &config, |_| true).is_empty());
        assert_eq!(state.persistence[slot], 0);
    }

    #[test]
    fn ineligible_classes_and_untraversable_cells_never_trigger() {
        let chemistry = chemistry();
        let config = transition();
        let classes = class_count(&chemistry);
        let mut state = TransitionState::new(2, &chemistry, &config);
        let mut densities = vec![0_i64; 2 * classes];
        // Class 0 sits at aggregation step 0: below the minimum.
        densities[0] = 1_000;
        // Class 1 in cell 1: eligible class, but the cell is water.
        densities[classes + 1] = 1_000;
        for _ in 0..5 {
            let triggers = state.check(&densities, classes, &config, |cell| cell == 0);
            assert!(triggers.is_empty(), "{triggers:?}");
        }
        // The counters still advanced: the condition that failed was not
        // persistence.
        assert_eq!(state.persistence[0], 5);
        assert_eq!(state.persistence[classes + 1], 5);
    }

    #[test]
    fn triggers_come_out_in_ascending_slot_order() {
        let chemistry = chemistry();
        let mut config = transition();
        config.persistence_checks = 1;
        config.aggregation_step_min = 0;
        let classes = class_count(&chemistry);
        let mut state = TransitionState::new(4, &chemistry, &config);
        let mut densities = vec![0_i64; 4 * classes];
        for slot in [3 * classes + 2, 5, classes + 7, 0] {
            densities[slot] = 100;
        }
        let triggers = state.check(&densities, classes, &config, |_| true);
        let slots: Vec<usize> = triggers
            .iter()
            .map(|trigger| trigger.cell * classes + trigger.class)
            .collect();
        assert_eq!(slots, vec![0, 5, classes + 7, 3 * classes + 2]);
    }

    #[test]
    fn the_save_twin_carries_every_field() {
        let chemistry = chemistry();
        let config = transition();
        let mut state = TransitionState::new(1, &chemistry, &config);
        state.persistence[2] = 7;
        state.materialized_total = 1;
        state.events_total = 2;
        state.materialized_milli = 3;
        state.deferred_cap_total = 4;
        state.deferred_capacity_total = 5;
        state.refused_total = 6;
        let save = state.to_save();
        let TransitionSave {
            persistence,
            materialized_total,
            events_total,
            materialized_milli,
            deferred_cap_total,
            deferred_capacity_total,
            refused_total,
        } = save;
        assert_eq!(persistence, state.persistence);
        assert_eq!(
            (
                materialized_total,
                events_total,
                materialized_milli,
                deferred_cap_total,
                deferred_capacity_total,
                refused_total
            ),
            (1, 2, 3, 4, 5, 6)
        );
    }
}
