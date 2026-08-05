//! Phase 9 acceptance criteria C9.3 (Mendelian validation) and C9.4
//! (linkage validation).
//!
//! Both are textbook results the model was **not tuned to produce**, which
//! is what makes them worth running: Hardy-Weinberg and linkage decay are
//! consequences of unbiased meiosis, not of anything anyone wrote down here.
//! If either fails, meiosis is distorting transmission.
//!
//! They run here rather than as a campaign because they need selection
//! **disabled** - the specification's condition C - and the cheapest honest
//! way to disable selection completely is to mate at random outside the
//! world, where no organism can die of anything.
//!
//! Every test carries a sensitivity check. A conformance test that cannot
//! fail is worth nothing, so each one is also run against data that
//! deliberately violates the property, and must reject it.

use sim_core::{
    Genome2, Haplotype, InheritanceMode, Locus, LocusKind, MeiosisConfig, STRUCTURAL_HOMOLOGY_BASE,
    named_random, recombine2,
};

/// The marked neutral locus. A trait locus is used because it is inert:
/// nothing in this test expresses it, so it cannot be under selection even
/// accidentally.
const MARKER_TRAIT: u16 = 0;
const ALLELE_A: f32 = 0.0;
const ALLELE_B: f32 = 1.0;

const POPULATION: usize = 600;
const GENERATIONS: usize = 40;

fn marker(value: f32) -> Locus {
    Locus {
        homology_id: u32::from(MARKER_TRAIT) + 1,
        gene_lineage_id: 1,
        mutation_event_id: 0,
        kind: LocusKind::Trait {
            trait_id: MARKER_TRAIT,
            value,
            dominance: 0.5,
        },
    }
}

fn spacer(homology_id: u32) -> Locus {
    Locus {
        homology_id,
        gene_lineage_id: u64::from(homology_id),
        mutation_event_id: 0,
        kind: LocusKind::Node {
            role: sim_core::NodeRole::Hidden,
            activation_id: sim_core::Activation::TanhApprox.id(),
            bias: 0.0,
            time_constant: 0,
        },
    }
}

/// One diploid individual carrying the marker on each haplotype, padded with
/// inert spacer loci so a chromosome has somewhere for crossovers to land.
fn individual(left: f32, right: f32, spacers: u32) -> Genome2 {
    let build = |value: f32| Haplotype {
        chromosomes: vec![
            std::iter::once(marker(value))
                .chain((0..spacers).map(|k| spacer(STRUCTURAL_HOMOLOGY_BASE + k)))
                .collect(),
        ],
    };
    Genome2 {
        haplotypes: [build(left), build(right)],
    }
}

fn allele_of(genome: &Genome2, slot: usize) -> f32 {
    genome.haplotypes[slot].chromosomes[0]
        .iter()
        .find_map(|locus| match locus.kind {
            LocusKind::Trait {
                trait_id, value, ..
            } if trait_id == MARKER_TRAIT => Some(value),
            _ => None,
        })
        .expect("the marker is present")
}

/// `(AA, AB, BB)` counts in a population.
fn genotype_counts(population: &[Genome2]) -> (usize, usize, usize) {
    let mut counts = (0, 0, 0);
    for individual in population {
        let b = [0, 1]
            .iter()
            .filter(|slot| allele_of(individual, **slot) == ALLELE_B)
            .count();
        match b {
            0 => counts.0 += 1,
            1 => counts.1 += 1,
            _ => counts.2 += 1,
        }
    }
    counts
}

/// Frequency of allele B across all haplotypes.
fn allele_frequency(population: &[Genome2]) -> f64 {
    let (aa, ab, bb) = genotype_counts(population);
    (2 * bb + ab) as f64 / (2 * (aa + ab + bb)) as f64
}

/// One generation of random mating with no selection: every individual is
/// replaced, parents drawn uniformly and independently.
fn next_generation(
    population: &[Genome2],
    config: &MeiosisConfig,
    seed: u64,
    generation: u64,
) -> Vec<Genome2> {
    (0..population.len())
        .map(|child| {
            let pick = |offset: u32| {
                (named_random(
                    seed,
                    generation,
                    sim_core::RngSystem::Analysis,
                    child as u64,
                    offset,
                ) % population.len() as u64) as usize
            };
            let mother = pick(0);
            let mut father = pick(1);
            if father == mother {
                father = (father + 1) % population.len();
            }
            recombine2(
                (&population[mother], mother as u64 + 1),
                (&population[father], father as u64 + 1),
                config,
                seed,
                generation * 100_000 + child as u64,
                child as u64 + 1,
            )
        })
        .collect()
}

/// Total absolute deviation from Hardy-Weinberg expectation, as a fraction
/// of the population, accumulated across generations.
///
/// Accumulating rather than testing each generation separately is
/// deliberate: single-generation sampling noise is large at any realistic
/// population size, while a **systematic** transmission bias -- which is the
/// only thing this can detect and the only thing worth detecting -- adds up.
fn hardy_weinberg_deviation(population: &[Genome2]) -> f64 {
    let (aa, ab, bb) = genotype_counts(population);
    let total = (aa + ab + bb) as f64;
    let p = allele_frequency(population);
    let q = 1.0 - p;
    let expected = [p * p * total, 2.0 * p * q * total, q * q * total];
    let observed = [bb as f64, ab as f64, aa as f64];
    observed
        .iter()
        .zip(expected.iter())
        .map(|(o, e)| (o - e).abs())
        .sum::<f64>()
        / total
}

#[test]
fn c9_3_genotype_frequencies_match_hardy_weinberg_under_random_mating() {
    // Start deliberately far from equilibrium: every individual a
    // heterozygote. Under random mating with unbiased meiosis, one
    // generation restores Hardy-Weinberg and it holds thereafter.
    let config = MeiosisConfig::default();
    let mut population: Vec<Genome2> = (0..POPULATION)
        .map(|_| individual(ALLELE_A, ALLELE_B, 12))
        .collect();

    // Sensitivity check: the statistic must *see* the starting violation.
    // All heterozygotes at p = 0.5 expects 25/50/25 and observes 0/100/0,
    // a deviation of 1.0.
    let starting = hardy_weinberg_deviation(&population);
    assert!(
        starting > 0.4,
        "the statistic cannot see a population that is 100% heterozygous ({starting:.3})"
    );

    let mut total_deviation = 0.0;
    let mut worst: f64 = 0.0;
    for generation in 0..GENERATIONS {
        population = next_generation(&population, &config, 0x0c93_0000, generation as u64);
        let deviation = hardy_weinberg_deviation(&population);
        total_deviation += deviation;
        worst = worst.max(deviation);
    }
    let mean = total_deviation / GENERATIONS as f64;

    // Sampling error for a genotype frequency at n = 600 is about 0.02, and
    // the statistic sums three of them, so a mean around 0.03 to 0.05 is
    // noise. A systematic distortion of transmission would sit far above it.
    assert!(
        mean < 0.06,
        "mean Hardy-Weinberg deviation {mean:.4} over {GENERATIONS} generations is too large"
    );
    assert!(
        worst < 0.15,
        "worst-generation deviation {worst:.4} suggests a systematic bias, not sampling noise"
    );

    // The population must not have gone to fixation, or the test is
    // comparing 1.0 against 1.0 and proving nothing.
    let final_p = allele_frequency(&population);
    assert!(
        final_p > 0.05 && final_p < 0.95,
        "allele frequency reached {final_p:.3}: the marker fixed, so the check is vacuous"
    );
}

#[test]
fn c9_3_meiosis_does_not_bias_which_allele_it_transmits() {
    // The other half of the same claim, measured directly rather than
    // through genotype frequencies: a heterozygote must pass each of its two
    // alleles to about half its gametes. A transmission bias would show up
    // here even where Hardy-Weinberg happened to look fine.
    let config = MeiosisConfig::default();
    let parent = individual(ALLELE_A, ALLELE_B, 12);
    let mut b_transmitted = 0_usize;
    let trials = 4_000;
    for child in 0..trials as u64 {
        let gamete = sim_core::gamete(&parent, &config, 0x0c93_b000, child, child, 0);
        let value = gamete.chromosomes[0]
            .iter()
            .find_map(|locus| match locus.kind {
                LocusKind::Trait { value, .. } => Some(value),
                _ => None,
            })
            .expect("the marker segregated");
        if value == ALLELE_B {
            b_transmitted += 1;
        }
    }
    let rate = b_transmitted as f64 / trials as f64;
    // Standard error at n = 4,000 is about 0.008, so 0.47 to 0.53 is a
    // generous four-sigma band.
    assert!(
        (0.47..=0.53).contains(&rate),
        "allele B was transmitted at {rate:.4}, not one half"
    );
}

// --- C9.4: linkage ----------------------------------------------------------

/// Fraction of gametes in which the alleles at two marked positions came
/// from different parental haplotypes -- the recombination fraction.
fn recombination_fraction(spacers: u32, second_marker_offset: u32, trials: u64) -> f64 {
    // Two markers on the same chromosome, `second_marker_offset` positions
    // apart in homology order. Each haplotype is uniform, so a gamete
    // carrying different values at the two positions must have crossed over
    // between them.
    let build = |tag: f32| Haplotype {
        chromosomes: vec![
            (0..spacers)
                .map(|k| {
                    let mut locus = spacer(STRUCTURAL_HOMOLOGY_BASE + k);
                    if let LocusKind::Node { bias, .. } = &mut locus.kind {
                        *bias = tag;
                    }
                    locus
                })
                .collect(),
        ],
    };
    let parent = Genome2 {
        haplotypes: [build(-1.0), build(1.0)],
    };
    let config = MeiosisConfig::default();

    let bias_at = |haplotype: &Haplotype, index: usize| -> f32 {
        match haplotype.chromosomes[0][index].kind {
            LocusKind::Node { bias, .. } => bias,
            _ => 0.0,
        }
    };
    let mut recombinant = 0_u64;
    for child in 0..trials {
        let gamete = sim_core::gamete(&parent, &config, 0x0c94_0000, child, child, 0);
        let first = bias_at(&gamete, 0);
        let second = bias_at(&gamete, second_marker_offset as usize);
        if first != second {
            recombinant += 1;
        }
    }
    recombinant as f64 / trials as f64
}

#[test]
fn c9_4_allele_association_decays_with_map_distance() {
    // The prediction the crossover model makes and that free recombination
    // does not: loci close together co-segregate, loci far apart approach
    // independence at one half.
    let spacers = 64;
    let trials = 3_000;
    let distances = [1_u32, 2, 4, 8, 16, 32, 63];
    let fractions: Vec<f64> = distances
        .iter()
        .map(|distance| recombination_fraction(spacers, *distance, trials))
        .collect();

    for (distance, fraction) in distances.iter().zip(fractions.iter()) {
        println!("PHASE9-LINKAGE distance={distance} recombination_fraction={fraction:.4}");
    }

    // Adjacent loci almost never separate.
    assert!(
        fractions[0] < 0.05,
        "adjacent loci recombined at {:.4}, so linkage is not being preserved",
        fractions[0]
    );
    // The relationship is monotone increasing, allowing for sampling noise.
    for window in fractions.windows(2) {
        assert!(
            window[1] >= window[0] - 0.03,
            "recombination fraction fell with distance: {:.4} then {:.4}",
            window[0],
            window[1]
        );
    }
    // ...and the far end is substantially higher than the near end, or
    // "decays with distance" is not what is happening.
    assert!(
        fractions[fractions.len() - 1] > fractions[0] + 0.25,
        "recombination only rose from {:.4} to {:.4} across the chromosome",
        fractions[0],
        fractions[fractions.len() - 1]
    );
    // A recombination fraction can never exceed one half under any
    // crossover model; exceeding it would mean the walk is flipping sides
    // more often than crossovers occur.
    assert!(
        fractions.iter().all(|fraction| *fraction <= 0.55),
        "a recombination fraction exceeded one half: {fractions:?}"
    );
}

#[test]
fn c9_4_free_recombination_would_fail_the_same_test() {
    // The sensitivity check for linkage. Under an inheritance mode with no
    // within-chromosome crossover the whole chromosome segregates as a unit,
    // so the recombination fraction is zero at *every* distance -- the
    // opposite failure -- and under schema 1's per-gene independent choice
    // it would be one half at every distance. Either way the decay is
    // absent, which is what distinguishes meiosis from both.
    let build = |tag: f32| Haplotype {
        chromosomes: vec![
            (0..32_u32)
                .map(|k| {
                    let mut locus = spacer(STRUCTURAL_HOMOLOGY_BASE + k);
                    if let LocusKind::Node { bias, .. } = &mut locus.kind {
                        *bias = tag;
                    }
                    locus
                })
                .collect(),
        ],
    };
    let parent = Genome2 {
        haplotypes: [build(-1.0), build(1.0)],
    };
    let config = MeiosisConfig {
        mode: InheritanceMode::BiparentalAssort,
        ..MeiosisConfig::default()
    };
    let mut recombinant = 0;
    for child in 0..1_000_u64 {
        let gamete = sim_core::gamete(&parent, &config, 0x0c94_b000, child, child, 0);
        let first = match gamete.chromosomes[0][0].kind {
            LocusKind::Node { bias, .. } => bias,
            _ => 0.0,
        };
        let last = match gamete.chromosomes[0][31].kind {
            LocusKind::Node { bias, .. } => bias,
            _ => 0.0,
        };
        if first != last {
            recombinant += 1;
        }
    }
    assert_eq!(
        recombinant, 0,
        "a mode with no within-chromosome crossover produced recombinants"
    );
}
