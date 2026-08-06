//! Phase 10 development: C10.1, C10.2, and **C10.4's gate**.
//!
//! C10.4 is not a metric. ADR-0022 D1 promoted it to a **gate**: two
//! commissioned reviews recommend against developmental encodings, ADR-0019
//! partially declines that for morphology, and this measurement is the
//! concession. If a typical single-locus mutation produces an unrelated body,
//! the genotype-phenotype map is noise, selection cannot act on morphology,
//! and the specified direct parameterized body-plan fallback is taken.
//!
//! ## The threshold, stated before the measurement
//!
//! **A single-locus mutation must leave more than half the body intact:
//! median phenotypic distance below 500 milli.** Distance is lattice
//! occupancy difference over the union, with a shared cell holding a
//! different module type counting as half a difference; 0 is identical and
//! 1000 shares no cell at all. Half is the point at which "a modified body"
//! becomes "a different body", and it is the reading of "beyond the stated
//! dissimilarity threshold" this phase commits to.
//!
//! ## The trap this measurement has to avoid
//!
//! Most mutations of a growth rule change nothing observable - they perturb a
//! threshold a rule never reaches, or re-point a direction that was already
//! blocked. Those score 0. A median taken over all mutations would therefore
//! sit at 0 and the gate would pass **while saying nothing**, which is
//! exactly the "criterion satisfied by an empty run" failure this project has
//! hit before. So both distributions are reported: the median over every
//! mutation that reached a growth rule, which is the criterion's literal
//! statistic and the one the gate uses, and the median over only those that
//! changed the body, which is what says whether the first number means
//! anything.

use sim_core::{
    Body, DevelopCounters, Genome2, LatticeKind, Locus, LocusKind, MorphologyCaps, MutationConfig,
    Regulatory, STRUCTURAL_HOMOLOGY_BASE, ViabilityFailure, grow, minimal_founder, mutate,
    named_random, phenotypic_distance_milli, rules_of,
};

const GATE_MEDIAN_MAX_MILLI: i64 = 500;
const TRIALS: u32 = 60_000;
const RULES_PER_GENOME: u32 = 6;

fn caps() -> MorphologyCaps {
    MorphologyCaps::provisional()
}

fn draw(seed: u64, index: u32) -> u64 {
    named_random(seed, 0, sim_core::RngSystem::StructuralMutation, 77, index)
}

/// A genome with a randomly drawn growth program bolted onto the minimal
/// founder scaffolding.
fn genome_with_rules(seed: u64, count: u32) -> Genome2 {
    let mut genome = minimal_founder(&[0.5; sim_core::TRAIT_COUNT]);
    for slot in 0..count {
        let base = draw(seed, slot * 8);
        let rule = Regulatory {
            condition_kind: (base & 0xff) as u8,
            condition_op: ((base >> 8) & 0xff) as u8,
            condition_param: ((base >> 16) & 0xff) as u8,
            // Small thresholds: a rule keyed on "module count >= 40000" can
            // never fire on a body capped at 64, and a program made of those
            // would measure nothing.
            threshold: ((base >> 24) & 0x7) as u16,
            action_kind: ((base >> 32) & 0xff) as u8,
            action_type: ((base >> 40) & 0xff) as u8,
            direction: ((base >> 48) & 0xff) as u8,
            scale_milli: 1_000,
        }
        .normalized();
        let homology_id = STRUCTURAL_HOMOLOGY_BASE + 10_000 + slot * 100;
        for haplotype in 0..2 {
            genome.haplotypes[haplotype].chromosomes[0].push(Locus {
                homology_id,
                gene_lineage_id: u64::from(homology_id),
                mutation_event_id: 0,
                kind: LocusKind::Regulatory { rule },
            });
        }
    }
    for haplotype in 0..2 {
        genome.haplotypes[haplotype].chromosomes[0].sort_by_key(|locus| locus.homology_id);
    }
    genome
}

fn body_of(genome: &Genome2, counters: &mut DevelopCounters) -> Body {
    grow(&rules_of(genome), LatticeKind::Square, &caps(), counters)
}

fn point_only() -> MutationConfig {
    MutationConfig {
        point_q16: 65_535,
        duplication_q16: 0,
        deletion_q16: 0,
        insertion_q16: 0,
        transposition_q16: 0,
        max_run: 1,
        point_delta_q16: 3_277,
    }
}

fn median(values: &mut [i64]) -> i64 {
    if values.is_empty() {
        return 0;
    }
    values.sort_unstable();
    values[(values.len() - 1) / 2]
}

#[test]
fn c10_4_single_locus_mutations_do_not_produce_unrelated_bodies() {
    let caps = caps();
    let mut distances_all: Vec<i64> = Vec::new();
    let mut distances_effective: Vec<i64> = Vec::new();
    // Stratified by parent size. On a one-module body the metric can only
    // take the values 0, 500, 666 and 1000, so a median over a population of
    // unicells reports the metric's own quantization rather than the
    // encoding's smoothness. Reported alongside, never instead of, the
    // pre-registered statistic.
    let mut distances_large: Vec<i64> = Vec::new();
    let mut distances_large_effective: Vec<i64> = Vec::new();
    let mut reached_a_rule = 0_u32;
    let mut lethal = 0_u32;
    let mut counters = DevelopCounters::default();

    for trial in 0..TRIALS {
        let seed = 0x9e37_79b9_u64
            .wrapping_mul(u64::from(trial) + 1)
            .wrapping_add(0x5eed);
        let parent = genome_with_rules(seed, RULES_PER_GENOME);
        let before_rules = rules_of(&parent);
        let parent_body = body_of(&parent, &mut counters);

        let mut child = parent.clone();
        let mut child_counters = sim_core::MutationCounters::default();
        mutate(
            &mut child,
            &point_only(),
            &sim_core::GenomeCaps::provisional(),
            &mut child_counters,
            seed,
            1,
            u64::from(trial),
        );
        let after_rules = rules_of(&child);
        if before_rules == after_rules {
            // The mutation landed on a trait, node, edge, or binding. Not a
            // morphological mutation at all, so it is not evidence about the
            // morphological map in either direction.
            continue;
        }
        reached_a_rule += 1;

        let child_body = body_of(&child, &mut counters);
        if child_body.validate(LatticeKind::Square, &caps).is_err()
            && parent_body.validate(LatticeKind::Square, &caps).is_ok()
        {
            // A mutation that kills the body is maximal phenotypic distance
            // as far as selection is concerned, and counting it as anything
            // less would flatter the encoding.
            lethal += 1;
            distances_all.push(1_000);
            distances_effective.push(1_000);
            continue;
        }
        let distance = phenotypic_distance_milli(&parent_body, &child_body);
        distances_all.push(distance);
        if distance > 0 {
            distances_effective.push(distance);
        }
        if parent_body.len() >= 4 {
            distances_large.push(distance);
            if distance > 0 {
                distances_large_effective.push(distance);
            }
        }
    }

    assert!(
        reached_a_rule > 500,
        "only {reached_a_rule} of {TRIALS} mutations reached a growth rule, which is too few \
         to say anything about the morphological map"
    );

    let silent = distances_all.iter().filter(|d| **d == 0).count();
    let silent_share_milli = silent as i64 * 1_000 / distances_all.len().max(1) as i64;
    let median_all = median(&mut distances_all.clone());
    let median_effective = median(&mut distances_effective.clone());
    let lethal_share_milli = i64::from(lethal) * 1_000 / reached_a_rule.max(1) as i64;

    println!(
        "PHASE10-GATE c10_4 trials={TRIALS} reached_rule={reached_a_rule} \
         silent={silent} silent_share_milli={silent_share_milli} \
         lethal={lethal} lethal_share_milli={lethal_share_milli} \
         median_all_milli={median_all} median_effective_milli={median_effective} \
         threshold_milli={GATE_MEDIAN_MAX_MILLI}"
    );

    println!(
        "PHASE10-GATE c10_4-stratified large_parent_n={} large_median_milli={} \
         large_effective_n={} large_median_effective_milli={}",
        distances_large.len(),
        median(&mut distances_large.clone()),
        distances_large_effective.len(),
        median(&mut distances_large_effective.clone()),
    );

    assert!(
        median_all < GATE_MEDIAN_MAX_MILLI,
        "C10.4 GATE FAILED: the median single-locus mutation moves the body {median_all} milli, \
         at or beyond the {GATE_MEDIAN_MAX_MILLI} threshold. The developmental encoding has \
         failed its own premise and ADR-0019's parameterized body-plan fallback is indicated."
    );
    // ...and the pass must not be an artifact of silence. If almost every
    // mutation were silent the median would be 0 while telling us nothing
    // about the mutations that do something.
    assert!(
        silent_share_milli < 900,
        "{silent_share_milli} per mille of rule-reaching mutations changed nothing, so the \
         median above is a statement about silence rather than about the map"
    );
}

#[test]
fn c10_1_development_is_pure_across_genomes_and_lattices() {
    // The same genome, developed twice, on both lattices. Purity is what
    // lets bodies be excluded from the save entirely.
    for trial in 0..200_u32 {
        let seed = u64::from(trial) * 0x1234_5677 + 11;
        let genome = genome_with_rules(seed, RULES_PER_GENOME);
        for lattice in [LatticeKind::Square, LatticeKind::Hex] {
            let mut first = DevelopCounters::default();
            let mut second = DevelopCounters::default();
            let left = grow(&rules_of(&genome), lattice, &caps(), &mut first);
            let right = grow(&rules_of(&genome), lattice, &caps(), &mut second);
            assert_eq!(left, right, "development is not pure at trial {trial}");
            assert_eq!(first, second, "counters diverged at trial {trial}");
        }
    }
}

#[test]
fn c10_2_and_c10_8_every_reachable_body_is_bounded_and_in_range() {
    // C10.8: no derived attribute leaves its clamp for any body reachable
    // within the caps, and no body exceeds a cap. Property-style over
    // randomly drawn growth programs rather than over hand-built bodies,
    // because the question is about what *development* can reach.
    let caps = caps();
    let mut counters = DevelopCounters::default();
    let mut viable = 0_u32;
    let mut unicellular = 0_u32;
    for trial in 0..3_000_u32 {
        let seed = u64::from(trial) * 0x9E37_79B1 + 7;
        let genome = genome_with_rules(seed, RULES_PER_GENOME);
        for lattice in [LatticeKind::Square, LatticeKind::Hex] {
            let body = grow(&rules_of(&genome), lattice, &caps, &mut counters);
            assert!(
                body.len() <= usize::from(caps.max_modules),
                "trial {trial} exceeded max_modules"
            );
            for module in body.modules() {
                assert!(
                    module.position.index(caps.lattice_radius).is_some(),
                    "trial {trial} placed a module outside the lattice"
                );
                assert!(
                    (sim_core::MIN_SCALE_MILLI..=sim_core::MAX_SCALE_MILLI)
                        .contains(&module.scale_milli)
                );
            }
            let derived = body.derive();
            assert!(derived.mass_milli >= 0 && derived.basal_cost_milli >= 0);
            let speed = derived.max_speed_milli(500, 3_000);
            assert!(
                (500..=3_000).contains(&speed),
                "trial {trial} produced speed {speed} outside its clamp"
            );
            if body.validate(lattice, &caps).is_ok() {
                viable += 1;
                if body.len() == 1 {
                    unicellular += 1;
                }
            }
        }
    }
    println!(
        "PHASE10-GATE c10_2 viable={viable} unicellular={unicellular} \
         nonviable_total={} refused_occupied={} refused_bounds={}",
        counters.total_nonviable(),
        counters.refused_occupied,
        counters.refused_out_of_bounds,
    );
    assert!(
        viable > 0,
        "no randomly drawn growth program produced a viable body, so the grammar is \
         unusable and C10.5's ceiling cannot be met at any rate"
    );
}

#[test]
fn c10_5_non_viability_is_typed_counted_and_reported() {
    // The non-viability rate is a first-class metric because a high one drops
    // effective fecundity and shifts the ecology, so a campaign that did not
    // report it could mistake a fecundity collapse for an ecological effect.
    let caps = caps();
    let mut counters = DevelopCounters::default();
    let mut attempts = 0_u32;
    let mut nonviable = 0_u32;
    let mut by_reason = std::collections::BTreeMap::<&'static str, u32>::new();
    for trial in 0..3_000_u32 {
        let seed = u64::from(trial) * 0x2545_F491 + 3;
        let genome = genome_with_rules(seed, RULES_PER_GENOME);
        let body = grow(
            &rules_of(&genome),
            LatticeKind::Square,
            &caps,
            &mut counters,
        );
        attempts += 1;
        if let Err(failure) = body.validate(LatticeKind::Square, &caps) {
            nonviable += 1;
            *by_reason.entry(failure.name()).or_default() += 1;
            // No repair path exists, which is the point: the reason is typed
            // and the birth is refused.
            assert!(matches!(
                failure,
                ViabilityFailure::Empty
                    | ViabilityFailure::Disconnected
                    | ViabilityFailure::MissingRequiredType(_)
                    | ViabilityFailure::TooManyModules
                    | ViabilityFailure::OutOfBounds
                    | ViabilityFailure::Overlap
                    | ViabilityFailure::ScaleOutOfRange
            ));
        }
    }
    let share_milli = i64::from(nonviable) * 1_000 / i64::from(attempts);
    println!(
        "PHASE10-GATE c10_5 attempts={attempts} nonviable={nonviable} \
         share_milli={share_milli} by_reason={by_reason:?}"
    );
    // Reported here rather than gated: the ceiling is a campaign-scale
    // property of a *founder* population, and these are uniformly drawn
    // programs, which are far harsher than anything evolution would carry.
    assert!(nonviable < attempts, "every drawn program was non-viable");
}
