//! Phase 13 reachability census (the D-117 `BindingCensus` analog).
//!
//! Counts, from a final population's genomes, what the social channel's
//! campaign must read its nulls against: which organisms can *hear*
//! (`signal_in` bindings), which can *speak* (`signal_emit` bindings),
//! which perceive conspecifics at all, and whether the observational rule
//! exists anywhere as an allele or as a compiled plastic edge. A C13.1
//! null over a population in which nothing binds a receiver is a
//! reachability null, not a transmission null - D-117 against D-099 is the
//! precedent for why the distinction must be measured rather than assumed.
//!
//! A census, not a criterion: no threshold, no verdict (ADR-0016).

use sim_core::{
    CHANNEL_NEIGHBOUR_BASE, CHANNEL_SIGNAL_EMIT_BASE, CHANNEL_SIGNAL_IN_BASE, Genome2, LocusKind,
    NEIGHBOUR_CUE_COUNT, PERCEPTION_K_MAX, PlasticityBudget, RULE_OBSERVATIONAL, RULE_SPACE,
    SIGNAL_CHANNELS_MAX, compile_network_with_budget,
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SocialBindingCensus {
    pub population: usize,
    /// Organisms with at least one binding to any neighbour cue (23..=58).
    pub binds_neighbour_cues: usize,
    /// Organisms with at least one `signal_in` binding (59..=62): hearers.
    pub binds_signal_in: usize,
    /// Organisms with at least one `signal_emit` binding (118..=121):
    /// speakers.
    pub binds_signal_emit: usize,
    /// Per-channel hearer counts, `signal_in_0..=3`.
    pub signal_in_by_channel: [usize; SIGNAL_CHANNELS_MAX as usize],
    /// Per-channel speaker counts, `signal_emit_0..=3`.
    pub signal_emit_by_channel: [usize; SIGNAL_CHANNELS_MAX as usize],
    /// Organisms that both speak and hear - the conjunction a transmission
    /// chain needs at both ends.
    pub binds_emit_and_in: usize,
    /// Organisms that hear and perceive conspecifics - the receiver-side
    /// conjunction the imitation question needs.
    pub binds_in_and_cues: usize,
    /// Rule-5 alleles across both haplotypes (stored ids that normalize to
    /// the observational rule).
    pub rule5_alleles: usize,
    /// Organisms carrying at least one rule-5 allele.
    pub organisms_with_rule5_allele: usize,
    /// Compiled plastic edges whose effective rule is the observational
    /// rule under the world's own budget - what the learn phase would
    /// actually run.
    pub rule5_expressed_edges: usize,
    /// Organisms with at least one such edge.
    pub organisms_with_rule5_expressed: usize,
}

/// Census a population's social bindings. `budget` is the world's own
/// plasticity budget (`SimConfig::plasticity_budget`), so the expressed
/// half counts what that world's learn phase would run, gate included.
/// A compile failure is an error rather than a skip: a genome that will
/// not compile belongs to an organism that could not have been alive.
pub fn social_binding_census(
    genomes: &[Genome2],
    budget: PlasticityBudget,
) -> Result<SocialBindingCensus, String> {
    let mut census = SocialBindingCensus {
        population: genomes.len(),
        ..Default::default()
    };
    let cue_end = CHANNEL_NEIGHBOUR_BASE + PERCEPTION_K_MAX as u16 * NEIGHBOUR_CUE_COUNT;
    for (index, genome) in genomes.iter().enumerate() {
        let mut cues = false;
        let mut hears = [false; SIGNAL_CHANNELS_MAX as usize];
        let mut speaks = [false; SIGNAL_CHANNELS_MAX as usize];
        let mut rule5_alleles_here = 0_usize;
        for haplotype in &genome.haplotypes {
            for chromosome in &haplotype.chromosomes {
                for locus in chromosome {
                    match locus.kind {
                        LocusKind::IoBinding { channel_id, .. } => {
                            if (CHANNEL_NEIGHBOUR_BASE..cue_end).contains(&channel_id) {
                                cues = true;
                            } else if (CHANNEL_SIGNAL_IN_BASE
                                ..CHANNEL_SIGNAL_IN_BASE + SIGNAL_CHANNELS_MAX as u16)
                                .contains(&channel_id)
                            {
                                hears[usize::from(channel_id - CHANNEL_SIGNAL_IN_BASE)] = true;
                            } else if (CHANNEL_SIGNAL_EMIT_BASE
                                ..CHANNEL_SIGNAL_EMIT_BASE + SIGNAL_CHANNELS_MAX as u16)
                                .contains(&channel_id)
                            {
                                speaks[usize::from(channel_id - CHANNEL_SIGNAL_EMIT_BASE)] = true;
                            }
                        }
                        LocusKind::Edge { plasticity, .. } => {
                            if plasticity.rule_id % RULE_SPACE == RULE_OBSERVATIONAL {
                                rule5_alleles_here += 1;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        let hears_any = hears.iter().any(|&flag| flag);
        let speaks_any = speaks.iter().any(|&flag| flag);
        census.binds_neighbour_cues += usize::from(cues);
        census.binds_signal_in += usize::from(hears_any);
        census.binds_signal_emit += usize::from(speaks_any);
        for channel in 0..SIGNAL_CHANNELS_MAX as usize {
            census.signal_in_by_channel[channel] += usize::from(hears[channel]);
            census.signal_emit_by_channel[channel] += usize::from(speaks[channel]);
        }
        census.binds_emit_and_in += usize::from(hears_any && speaks_any);
        census.binds_in_and_cues += usize::from(hears_any && cues);
        census.rule5_alleles += rule5_alleles_here;
        census.organisms_with_rule5_allele += usize::from(rule5_alleles_here > 0);

        // The expressed half, through the same public compile the world
        // uses (D-076: the logical path and the encoded path are different
        // paths; here the allele level and the compiled level are).
        let compiled = compile_network_with_budget(&genome.express_network(), budget)
            .map_err(|error| format!("organism {index}: compile: {error:?}"))?;
        let expressed_here = compiled
            .plastic_edges
            .iter()
            .filter(|edge| edge.rule.rule_id == RULE_OBSERVATIONAL)
            .count();
        census.rule5_expressed_edges += expressed_here;
        census.organisms_with_rule5_expressed += usize::from(expressed_here > 0);
    }
    Ok(census)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim_core::{Activation, GenomeCaps, Locus, NodeRole, STRUCTURAL_HOMOLOGY_BASE};

    fn caps() -> GenomeCaps {
        let mut caps = sim_core::SimConfig::phase1_default(1).genome2.caps;
        caps.max_loci_per_chromosome = 160;
        caps.max_nodes = 160;
        caps.max_edges = 160;
        caps
    }

    fn founder() -> Genome2 {
        sim_core::founder_from_traits(&[0.5; 14])
    }

    fn bind(genome: &mut Genome2, channel: u16, salt: u32) {
        let node_id = STRUCTURAL_HOMOLOGY_BASE + 40_000 + salt * 10;
        for haplotype in &mut genome.haplotypes {
            let chromosome = &mut haplotype.chromosomes[0];
            chromosome.push(Locus {
                homology_id: node_id,
                gene_lineage_id: u64::from(node_id),
                mutation_event_id: 0,
                kind: LocusKind::Node {
                    role: NodeRole::Output,
                    activation_id: Activation::TanhApprox.id(),
                    bias: 0.0,
                    time_constant: 0,
                },
            });
            chromosome.push(Locus {
                homology_id: node_id + 1,
                gene_lineage_id: u64::from(node_id + 1),
                mutation_event_id: 0,
                kind: LocusKind::IoBinding {
                    node: node_id,
                    channel_id: channel,
                    gain: 1.0,
                },
            });
            chromosome.sort_unstable_by_key(|locus| locus.homology_id);
        }
        genome.validate_structure(&caps()).expect("validates");
    }

    #[test]
    fn the_census_counts_families_channels_and_conjunctions_exactly() {
        let mut speaker = founder();
        bind(&mut speaker, CHANNEL_SIGNAL_EMIT_BASE + 2, 0);
        let mut hearer = founder();
        bind(&mut hearer, CHANNEL_SIGNAL_IN_BASE, 0);
        bind(&mut hearer, CHANNEL_NEIGHBOUR_BASE + 12, 1);
        let mut both = founder();
        bind(&mut both, CHANNEL_SIGNAL_EMIT_BASE, 0);
        bind(&mut both, CHANNEL_SIGNAL_IN_BASE + 3, 1);
        let silent = founder();

        let census = social_binding_census(
            &[speaker, hearer, both, silent],
            PlasticityBudget::disabled(),
        )
        .expect("census");
        assert_eq!(census.population, 4);
        assert_eq!(census.binds_neighbour_cues, 1);
        assert_eq!(census.binds_signal_in, 2);
        assert_eq!(census.binds_signal_emit, 2);
        assert_eq!(census.signal_in_by_channel, [1, 0, 0, 1]);
        assert_eq!(census.signal_emit_by_channel, [1, 0, 1, 0]);
        assert_eq!(census.binds_emit_and_in, 1);
        assert_eq!(census.binds_in_and_cues, 1);
        assert_eq!(census.rule5_alleles, 0);
        assert_eq!(census.rule5_expressed_edges, 0);
    }

    #[test]
    fn rule5_is_counted_at_the_allele_level_and_at_the_expressed_level_with_the_gate() {
        use sim_core::{EDGE_FLAG_PLASTIC, PlasticityGenes};
        let mut genome = founder();
        // A plastic rule-5 edge feeding a hidden node.
        const NODE: u32 = STRUCTURAL_HOMOLOGY_BASE + 8_000;
        const EDGE: u32 = STRUCTURAL_HOMOLOGY_BASE + 9_000;
        const INPUT: u32 = STRUCTURAL_HOMOLOGY_BASE + 1_000;
        for haplotype in &mut genome.haplotypes {
            let chromosome = &mut haplotype.chromosomes[0];
            chromosome.push(Locus {
                homology_id: NODE,
                gene_lineage_id: u64::from(NODE),
                mutation_event_id: 0,
                kind: LocusKind::Node {
                    role: NodeRole::Hidden,
                    activation_id: Activation::TanhApprox.id(),
                    bias: 0.0,
                    time_constant: 0,
                },
            });
            chromosome.push(Locus {
                homology_id: EDGE,
                gene_lineage_id: u64::from(EDGE),
                mutation_event_id: 0,
                kind: LocusKind::Edge {
                    source: INPUT,
                    target: NODE,
                    weight: 1.0,
                    flags: EDGE_FLAG_PLASTIC,
                    plasticity: PlasticityGenes {
                        rule_id: RULE_OBSERVATIONAL,
                        eta: 0.5,
                        coefficients: [1.0, 0.0, 0.0, 0.0],
                        decay: 0.0,
                        modulator_node: 0,
                    },
                },
            });
            chromosome.sort_unstable_by_key(|locus| locus.homology_id);
        }
        genome.validate_structure(&caps()).expect("validates");

        // Two alleles (both haplotypes) whatever the gate; the expressed
        // half follows the budget: gated in, one edge; gated out, zero -
        // the census must count what the world's learn phase would run.
        let gated_in = social_binding_census(
            std::slice::from_ref(&genome),
            PlasticityBudget::edges(8).with_observational(),
        )
        .expect("census");
        assert_eq!(gated_in.rule5_alleles, 2);
        assert_eq!(gated_in.organisms_with_rule5_allele, 1);
        assert_eq!(gated_in.rule5_expressed_edges, 1);
        assert_eq!(gated_in.organisms_with_rule5_expressed, 1);

        let gated_out =
            social_binding_census(std::slice::from_ref(&genome), PlasticityBudget::edges(8))
                .expect("census");
        assert_eq!(gated_out.rule5_alleles, 2, "alleles are gate-independent");
        assert_eq!(
            gated_out.rule5_expressed_edges, 0,
            "an ungated allele compiles to the static rule, and the census \
             must say so rather than credit the world with a rule it cannot run"
        );
    }
}
