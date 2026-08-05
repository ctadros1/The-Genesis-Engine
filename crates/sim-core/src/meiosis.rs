//! Meiosis and inheritance modes (Phase 9, `lifesim-meiosis-v1`).
//!
//! Schema 1 inherits per gene by independent parent choice, which is **free
//! recombination**: linkage disequilibrium decays instantly and a co-adapted
//! set of loci cannot be held together. That removes one of the main forces
//! the model is supposed to be able to exhibit. Meiosis with a small number
//! of crossovers per chromosome preserves linkage, and linkage decay with
//! map distance becomes a testable prediction rather than an assumption
//! (C9.4).
//!
//! Two details make the crossover meaningful, and both are easy to get
//! subtly wrong.
//!
//! **Positions are in merged homology space, not array indices.** Two
//! homologues generally have different lengths and different content - that
//! is the whole point of structural mutation - so "position 5" as an array
//! index would mean different things on each side and the walk would
//! desynchronize. Walking the *merged* ordering of both homologues'
//! `homology_id` values makes a crossover point a position in innovation
//! space, which is what lets disjoint and excess structural material
//! inherit sensibly without a special case. That is NEAT's alignment idea
//! obtained as an ordinary consequence of chromosomal inheritance.
//!
//! **Two corrections the specification's crossover model needs, both found
//! by C9.3 and C9.4 failing.** As written - start reading from haplotype 0,
//! flip at every crossover - the model is not Mendelian, in two separable
//! ways:
//!
//! - *Position zero was almost never inherited from haplotype 1.* The walk
//!   always began on side 0, so the first locus of every chromosome came
//!   from haplotype 0 unless a crossover happened to land exactly on it.
//!   Measured transmission of the second allele at a heterozygous marker was
//!   **0.14, not 0.5**. The fix is to draw the starting side, which is what
//!   real meiosis does: which chromatid a gamete begins reading is not
//!   fixed.
//! - *Recombination fractions exceeded one half.* With an obligate crossover
//!   and a small crossover count, the parity between two distant loci is
//!   dominated by the parity of the count, and the measured fraction across
//!   a chromosome reached **0.64**. No crossover model can produce that:
//!   `r <= 0.5` is a hard constraint of Mendelian genetics. The cause is
//!   that a real crossover happens in a four-strand bundle and involves only
//!   two of the four chromatids, so it makes a gamete recombinant with
//!   probability one half rather than certainty. Flipping the side with
//!   probability one half at each crossover reproduces that, and `r` then
//!   rises toward 0.5 without ever passing it.
//!
//! Both are recorded in D-068. A model that got either wrong would have
//! biased every allele frequency and every linkage measurement downstream,
//! silently.
//!
//! **The draw is keyed on the parent's slot as well as the child.** The
//! specification's key is `(seed, tick, Meiosis, child)`, which is the same
//! for both parents and would therefore give both gametes identical
//! crossover positions. Slot is assigned by ID comparison (Rule 3), so
//! adding it keeps the result independent of traversal order while removing
//! a correlation that has no business being there. Recorded as a deviation.
//!
//! `uniform-bounded-v1` remains valid and unchanged for schema-1 worlds.
//! Nothing here touches them.

use crate::genome2::{Genome2, Haplotype, Locus};
use crate::rng::{RngSystem, named_random};

pub const MEIOSIS_POLICY_VERSION: &str = "lifesim-meiosis-v1";

/// Draw indices reserved per chromosome.
///
/// The specification says `draw_base + i*4`, which leaves four draws per
/// chromosome: one for the crossover count and three for positions. That is
/// fewer than `max_extra_crossovers` can require, and overflowing into the
/// next chromosome's block would couple them. Sixteen is generous and
/// bounded; `MAX_EXTRA_CROSSOVERS` is capped against it.
const DRAWS_PER_CHROMOSOME: u32 = 16;
/// One draw for the count leaves fifteen for positions.
pub const MAX_EXTRA_CROSSOVERS: u32 = 14;

/// How offspring inherit.
///
/// `genetics` section 1.2 records that crossover is **not** universally
/// beneficial and can destroy coadapted structure under strong epistasis.
/// So paired reproduction does not imply mandatory crossover, and the
/// alternatives are first-class controls rather than a future idea
/// (ADR-0022 A10). Any claim about the benefit of recombination is reported
/// against at least `Clonal` and `PairedWholeGenome`.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum InheritanceMode {
    /// Single parent, no recombination. The baseline control.
    Clonal,
    /// Two parents; the offspring takes one parent's genome intact.
    /// Isolates mate choice from recombination.
    PairedWholeGenome,
    /// Independent assortment of whole chromosomes, no within-chromosome
    /// crossover.
    BiparentalAssort,
    /// Full meiosis with crossover. The default.
    #[default]
    Meiotic,
}

impl InheritanceMode {
    pub fn name(self) -> &'static str {
        match self {
            InheritanceMode::Clonal => "clonal",
            InheritanceMode::PairedWholeGenome => "paired_whole_genome",
            InheritanceMode::BiparentalAssort => "biparental_assort",
            InheritanceMode::Meiotic => "meiotic",
        }
    }

    pub fn from_id(id: u8) -> Option<Self> {
        match id {
            1 => Some(InheritanceMode::Clonal),
            2 => Some(InheritanceMode::PairedWholeGenome),
            3 => Some(InheritanceMode::BiparentalAssort),
            4 => Some(InheritanceMode::Meiotic),
            _ => None,
        }
    }

    pub fn id(self) -> u8 {
        match self {
            InheritanceMode::Clonal => 1,
            InheritanceMode::PairedWholeGenome => 2,
            InheritanceMode::BiparentalAssort => 3,
            InheritanceMode::Meiotic => 4,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MeiosisConfig {
    pub mode: InheritanceMode,
    /// Crossovers per chromosome are `1 + (draw mod (max_extra + 1))`, so
    /// there is **at least one per chromosome per meiosis**, which is the
    /// biological norm, and the distribution is biased toward small counts.
    pub max_extra_crossovers: u32,
}

impl Default for MeiosisConfig {
    fn default() -> Self {
        Self {
            mode: InheritanceMode::Meiotic,
            max_extra_crossovers: 2,
        }
    }
}

/// One parent's gamete: a single haplotype's worth of chromosomes.
pub type Gamete = Haplotype;

/// Produce one gamete from one diploid parent.
///
/// `parent_slot` is 0 for the lower-ID parent and 1 for the higher-ID
/// parent, assigned by ID comparison so no traversal order enters the
/// result (Rule 3).
pub fn gamete(
    parent: &Genome2,
    config: &MeiosisConfig,
    world_seed: u64,
    tick: u64,
    child_id: u64,
    parent_slot: u32,
) -> Gamete {
    let chromosome_count = parent.chromosome_count();
    let mut chromosomes = Vec::with_capacity(chromosome_count);
    for index in 0..chromosome_count {
        let left = &parent.haplotypes[0].chromosomes[index];
        let right = &parent.haplotypes[1].chromosomes[index];
        let base = parent_slot * (chromosome_count as u32 + 1) * DRAWS_PER_CHROMOSOME
            + index as u32 * DRAWS_PER_CHROMOSOME;
        let draw = |offset: u32| {
            named_random(
                world_seed,
                tick,
                RngSystem::Meiosis,
                child_id,
                base + offset,
            )
        };
        chromosomes.push(match config.mode {
            // No recombination at all: the gamete is one whole haplotype,
            // chosen per organism rather than per chromosome, so the two
            // haplotypes are never mixed.
            InheritanceMode::Clonal | InheritanceMode::PairedWholeGenome => {
                let pick = (named_random(
                    world_seed,
                    tick,
                    RngSystem::Meiosis,
                    child_id,
                    parent_slot * DRAWS_PER_CHROMOSOME,
                ) & 1) as usize;
                parent.haplotypes[pick].chromosomes[index].clone()
            }
            // Whole chromosomes assort independently, but nothing crosses
            // over within one.
            InheritanceMode::BiparentalAssort => {
                let pick = (draw(0) & 1) as usize;
                parent.haplotypes[pick].chromosomes[index].clone()
            }
            InheritanceMode::Meiotic => {
                let extra = config.max_extra_crossovers.min(MAX_EXTRA_CROSSOVERS);
                let count = 1 + (draw(0) % u64::from(extra + 1)) as usize;
                cross_over(left, right, count, &draw)
            }
        });
    }
    Haplotype { chromosomes }
}

/// Walk the merged homology ordering of two homologues, flipping which side
/// is being read at each crossover position.
///
/// A locus present only in the *selected* haplotype is emitted; one present
/// only in the non-selected haplotype is not. That is ordinary segregation,
/// and it is what makes disjoint and excess structural material inherit
/// without a special case.
fn cross_over(
    left: &[Locus],
    right: &[Locus],
    crossovers: usize,
    draw: &dyn Fn(u32) -> u64,
) -> Vec<Locus> {
    // Merged homology ordering. Both inputs are already sorted and strictly
    // ascending, so this is a linear merge and the result is the union of
    // the two ID sets in ascending order.
    let mut merged: Vec<u32> = Vec::with_capacity(left.len() + right.len());
    let (mut i, mut j) = (0_usize, 0_usize);
    while i < left.len() || j < right.len() {
        let next = match (left.get(i), right.get(j)) {
            (Some(a), Some(b)) => {
                if a.homology_id < b.homology_id {
                    i += 1;
                    a.homology_id
                } else if b.homology_id < a.homology_id {
                    j += 1;
                    b.homology_id
                } else {
                    i += 1;
                    j += 1;
                    a.homology_id
                }
            }
            (Some(a), None) => {
                i += 1;
                a.homology_id
            }
            (None, Some(b)) => {
                j += 1;
                b.homology_id
            }
            (None, None) => break,
        };
        merged.push(next);
    }
    if merged.is_empty() {
        return Vec::new();
    }

    // Positions in merged space, each carrying its own effective-crossover
    // decision in the high bits of the same draw, then deduplicated and
    // sorted ascending.
    let mut positions: Vec<(usize, bool)> = (0..crossovers)
        .map(|k| {
            let value = draw(1 + k as u32);
            (
                (value % merged.len() as u64) as usize,
                // The four-strand correction: a crossover involves two of
                // four chromatids, so it makes this gamete recombinant with
                // probability one half. Without it a recombination fraction
                // can exceed 0.5, which no crossover model may do.
                (value >> 32) & 1 == 1,
            )
        })
        .collect();
    positions.sort_unstable();
    positions.dedup_by_key(|(position, _)| *position);

    let mut out = Vec::with_capacity(merged.len());
    // Which homologue the gamete starts reading is itself a draw. Starting
    // always on side 0 made the first locus of every chromosome inherit from
    // haplotype 0 unless a crossover landed exactly on it.
    let mut side = (draw(0) >> 48) as usize & 1;
    let mut cut = 0_usize;
    for (rank, homology_id) in merged.iter().copied().enumerate() {
        while cut < positions.len() && positions[cut].0 == rank {
            if positions[cut].1 {
                side ^= 1;
            }
            cut += 1;
        }
        let source = if side == 0 { left } else { right };
        if let Ok(index) = source.binary_search_by_key(&homology_id, |locus| locus.homology_id) {
            out.push(source[index]);
        }
    }
    out
}

/// Build a child genome from two parents.
///
/// Haplotype 0 is the gamete from the **lower-ID** parent and haplotype 1
/// from the higher-ID parent, so the child is a function of the pair and not
/// of which parent the tick visited first.
pub fn recombine(
    parent_a: (&Genome2, u64),
    parent_b: (&Genome2, u64),
    config: &MeiosisConfig,
    world_seed: u64,
    tick: u64,
    child_id: u64,
) -> Genome2 {
    let (low, high) = if parent_a.1 <= parent_b.1 {
        (parent_a, parent_b)
    } else {
        (parent_b, parent_a)
    };
    match config.mode {
        // One parent contributes the whole child. The lower-ID parent is
        // the source, so the outcome is a function of the pair rather than
        // of visit order; which of the two haplotypes is copied is the
        // ordinary gamete draw.
        InheritanceMode::Clonal => {
            let single = gamete(low.0, config, world_seed, tick, child_id, 0);
            Genome2 {
                haplotypes: [single.clone(), single],
            }
        }
        InheritanceMode::PairedWholeGenome => Genome2 {
            haplotypes: [low.0.haplotypes[0].clone(), low.0.haplotypes[1].clone()],
        },
        _ => Genome2 {
            haplotypes: [
                gamete(low.0, config, world_seed, tick, child_id, 0),
                gamete(high.0, config, world_seed, tick, child_id, 1),
            ],
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genome2::{GenomeCaps, LocusKind, PlasticityGenes, STRUCTURAL_HOMOLOGY_BASE};
    use crate::registry::{Activation, NodeRole};

    fn node(homology_id: u32, bias: f32) -> Locus {
        Locus {
            homology_id,
            gene_lineage_id: u64::from(homology_id),
            mutation_event_id: 0,
            kind: LocusKind::Node {
                role: NodeRole::Hidden,
                activation_id: Activation::TanhApprox.id(),
                bias,
                time_constant: 0,
            },
        }
    }

    fn edge_locus(homology_id: u32, source: u32, target: u32) -> Locus {
        Locus {
            homology_id,
            gene_lineage_id: u64::from(homology_id),
            mutation_event_id: 0,
            kind: LocusKind::Edge {
                source,
                target,
                weight: 1.0,
                flags: 0,
                plasticity: PlasticityGenes::inert(),
            },
        }
    }

    /// A haplotype of `count` nodes at evenly spaced homology IDs, with the
    /// bias carrying which side it came from so segregation is visible.
    fn side(count: u32, bias: f32, stride: u32) -> Haplotype {
        Haplotype {
            chromosomes: vec![
                (0..count)
                    .map(|k| node(STRUCTURAL_HOMOLOGY_BASE + k * stride, bias))
                    .collect(),
            ],
        }
    }

    fn parent(count: u32, stride: u32) -> Genome2 {
        Genome2 {
            haplotypes: [side(count, -1.0, stride), side(count, 1.0, stride)],
        }
    }

    fn biases(haplotype: &Haplotype) -> Vec<f32> {
        haplotype.chromosomes[0]
            .iter()
            .map(|locus| match locus.kind {
                LocusKind::Node { bias, .. } => bias,
                _ => 0.0,
            })
            .collect()
    }

    #[test]
    fn a_gamete_is_a_pure_function_of_its_inputs() {
        let subject = parent(20, 1);
        let config = MeiosisConfig::default();
        let first = gamete(&subject, &config, 7, 11, 42, 0);
        let second = gamete(&subject, &config, 7, 11, 42, 0);
        assert_eq!(first, second);
    }

    #[test]
    fn a_child_does_not_depend_on_which_parent_was_visited_first() {
        // The order-independence obligation. Swapping the argument order
        // must produce an identical child, because slots are assigned by ID
        // comparison rather than by position.
        let left = parent(16, 1);
        let mut right = parent(16, 1);
        for locus in &mut right.haplotypes[0].chromosomes[0] {
            if let LocusKind::Node { bias, .. } = &mut locus.kind {
                *bias = 0.5;
            }
        }
        let config = MeiosisConfig::default();
        let forward = recombine((&left, 100), (&right, 200), &config, 3, 5, 9);
        let backward = recombine((&right, 200), (&left, 100), &config, 3, 5, 9);
        assert_eq!(forward, backward);
    }

    #[test]
    fn crossover_actually_mixes_the_two_homologues() {
        // A gamete that never switched sides would be one parent haplotype
        // copied, and every linkage claim downstream would be vacuous.
        let subject = parent(40, 1);
        let config = MeiosisConfig::default();
        let mut mixed = 0;
        for child in 0..200_u64 {
            let produced = gamete(&subject, &config, 1, 1, child, 0);
            let values = biases(&produced);
            let from_left = values.iter().filter(|bias| **bias < 0.0).count();
            if from_left > 0 && from_left < values.len() {
                mixed += 1;
            }
        }
        // Roughly two thirds, not nearly all. The four-strand correction
        // means each crossover involves this chromatid only half the time,
        // so with one to three crossovers about a third of gametes come
        // through as a clean copy of one parental haplotype -- which is what
        // real meiosis does. A figure near 200 would mean the correction is
        // not being applied and recombination fractions can exceed one half.
        assert!(
            (110..190).contains(&mixed),
            "{mixed} of 200 gametes mixed the two homologues; expected roughly two thirds"
        );
    }

    #[test]
    fn linkage_survives_meiosis() {
        // The property that separates meiosis from free recombination:
        // adjacent loci co-segregate far more often than distant ones. With
        // per-gene independent choice both would be at one half.
        let subject = parent(64, 1);
        let config = MeiosisConfig::default();
        let mut adjacent_same = 0;
        let mut distant_same = 0;
        let trials = 400;
        for child in 0..trials as u64 {
            let values = biases(&gamete(&subject, &config, 2, 2, child, 0));
            if values[0] == values[1] {
                adjacent_same += 1;
            }
            if values[0] == values[63] {
                distant_same += 1;
            }
        }
        let adjacent_rate = adjacent_same as f64 / trials as f64;
        let distant_rate = distant_same as f64 / trials as f64;
        assert!(
            adjacent_rate > 0.9,
            "adjacent loci co-segregated only {adjacent_rate:.2} of the time"
        );
        assert!(
            distant_rate < adjacent_rate - 0.2,
            "distant co-segregation {distant_rate:.2} is not below adjacent {adjacent_rate:.2}"
        );
    }

    #[test]
    fn homologues_of_different_lengths_segregate_without_a_special_case() {
        // The reason crossover positions live in homology space. One side
        // carries loci the other does not; every emitted locus must come
        // from whichever side was selected at that homology position, and
        // the result must stay sorted and free of duplicates.
        let mut subject = parent(10, 4);
        // Give haplotype 1 five extra loci interleaved between the shared
        // ones, so the two homologues differ in length and in content.
        for k in 0..5_u32 {
            subject.haplotypes[1].chromosomes[0]
                .push(node(STRUCTURAL_HOMOLOGY_BASE + k * 4 + 2, 1.0));
        }
        subject.haplotypes[1].chromosomes[0].sort_by_key(|locus| locus.homology_id);

        let config = MeiosisConfig::default();
        for child in 0..100_u64 {
            let produced = gamete(&subject, &config, 4, 4, child, 0);
            let ids: Vec<u32> = produced.chromosomes[0]
                .iter()
                .map(|locus| locus.homology_id)
                .collect();
            assert!(
                ids.windows(2).all(|pair| pair[0] < pair[1]),
                "gamete {child} is not strictly ascending: {ids:?}"
            );
            // Every emitted locus must exist on the side it claims to come
            // from -- a locus invented by the merge would be a bug.
            for locus in &produced.chromosomes[0] {
                let in_left = subject.haplotypes[0].chromosomes[0].contains(locus);
                let in_right = subject.haplotypes[1].chromosomes[0].contains(locus);
                assert!(in_left || in_right, "gamete contains an invented locus");
            }
        }
    }

    #[test]
    fn the_two_parents_gametes_are_not_drawn_from_the_same_sequence() {
        // The deviation this module records: keying only on the child would
        // give both parents identical crossover positions.
        let subject = parent(32, 1);
        let config = MeiosisConfig::default();
        let from_slot_0 = gamete(&subject, &config, 5, 5, 77, 0);
        let from_slot_1 = gamete(&subject, &config, 5, 5, 77, 1);
        assert_ne!(biases(&from_slot_0), biases(&from_slot_1));
    }

    #[test]
    fn clonal_and_whole_genome_modes_do_not_recombine() {
        let left = parent(24, 1);
        let right = parent(24, 1);
        for mode in [
            InheritanceMode::Clonal,
            InheritanceMode::PairedWholeGenome,
            InheritanceMode::BiparentalAssort,
        ] {
            let config = MeiosisConfig {
                mode,
                ..MeiosisConfig::default()
            };
            for child in 0..50_u64 {
                let produced = recombine((&left, 1), (&right, 2), &config, 6, 6, child);
                for haplotype in &produced.haplotypes {
                    let values = biases(haplotype);
                    // Every locus in a chromosome came from one side, so all
                    // biases within it are equal.
                    assert!(
                        values.windows(2).all(|pair| pair[0] == pair[1]),
                        "{} recombined within a chromosome",
                        mode.name()
                    );
                }
            }
        }
    }

    #[test]
    fn a_recombined_child_is_still_a_valid_genome() {
        // Meiosis emits whole loci from one parent or the other, so it
        // cannot invent a dangling reference -- but that has to be checked
        // rather than assumed, because a merge bug would show up here first.
        let subject = Genome2 {
            haplotypes: [
                Haplotype {
                    chromosomes: vec![vec![
                        node(STRUCTURAL_HOMOLOGY_BASE + 1, 0.0),
                        node(STRUCTURAL_HOMOLOGY_BASE + 2, 0.0),
                        edge_locus(
                            STRUCTURAL_HOMOLOGY_BASE + 3,
                            STRUCTURAL_HOMOLOGY_BASE + 1,
                            STRUCTURAL_HOMOLOGY_BASE + 2,
                        ),
                    ]],
                },
                Haplotype {
                    chromosomes: vec![vec![
                        node(STRUCTURAL_HOMOLOGY_BASE + 1, 0.5),
                        node(STRUCTURAL_HOMOLOGY_BASE + 2, 0.5),
                        edge_locus(
                            STRUCTURAL_HOMOLOGY_BASE + 3,
                            STRUCTURAL_HOMOLOGY_BASE + 1,
                            STRUCTURAL_HOMOLOGY_BASE + 2,
                        ),
                    ]],
                },
            ],
        };
        subject
            .validate_structure(&GenomeCaps::provisional())
            .expect("the parent is valid");
        let config = MeiosisConfig::default();
        for child in 0..200_u64 {
            let produced = recombine((&subject, 1), (&subject, 2), &config, 8, 8, child);
            produced
                .validate_structure(&GenomeCaps::provisional())
                .unwrap_or_else(|error| panic!("child {child} is invalid: {error}"));
        }
    }
}
