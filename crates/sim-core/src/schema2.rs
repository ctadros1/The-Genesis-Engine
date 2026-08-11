//! Per-organism schema-2 world state (Phase 9).
//!
//! Schema 2 replaces two things and nothing else: **the genome** and **the
//! controller**. Movement, feeding, pairing, contest, and physiology are
//! untouched, and a schema-2 world runs the same tick as a schema-1 world
//! through the same code.
//!
//! That is deliberate. If schema 2 also rewrote the tick, C9.1's "structure
//! evolved" and C9.2's "the ecology is still stable" would be comparisons
//! between two different simulations, and neither could be attributed to
//! structural freedom. Keeping the seam narrow is what makes the comparison
//! mean anything.
//!
//! The seam is exactly:
//!
//! - **Traits** come from diploid expression instead of a flat vector, and
//!   feed the same `Phenotype::from_traits`.
//! - **Intents** come from `controller2::evaluate` over the organism's own
//!   evolved graph instead of `controller::evaluate` over topology 1. The
//!   action channels map onto the same output slots the intent mapping
//!   already reads, so nothing downstream knows which schema produced them.
//! - **Reproduction** is meiosis plus structural mutation instead of
//!   per-gene independent choice plus point mutation.
//!
//! Schema 1's arrays are simply empty in a schema-2 world, and vice versa.
//! There is no mixed-schema world and no migration between them.

use crate::checksum::Fnv1a64;
use crate::controller2::{
    ActionRequests, ActivationState, CompiledNetwork, PlasticityBudget, compile_with_budget,
};
use crate::genome2::{Genome2, GenomeCaps};
use crate::structmut::MutationCounters;

/// Action channel IDs, in the order the intent mapping expects them.
///
/// The four memory outputs topology 1 carried have no entry: under schema 2
/// memory is a recurrent edge the organism evolves, so a memory *channel*
/// would offer the same capability twice and make it impossible to say
/// which one an organism used.
pub const ACTION_CHANNELS: [u16; 8] = [101, 102, 103, 104, 105, 106, 107, 108];

/// Sensory channel IDs, in the order `Phase2State::inputs` stores them.
///
/// Indices 0..16 of that array are the sixteen documented sensory inputs;
/// indices 16..20 are topology 1's memory registers, which schema 2 does
/// not expose.
pub const SENSE_CHANNELS: [u16; 16] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];

/// Parallel per-organism schema-2 arrays, kept in lockstep with the world's
/// primary arrays exactly as the Phase 2 and contest arrays are.
#[derive(Clone, Debug)]
pub(crate) struct Schema2State {
    pub genomes: Vec<Genome2>,
    /// Compiled evaluation plan, derived from the genome and rebuilt
    /// whenever it changes. Not logical state: recomputed on load.
    pub plans: Vec<CompiledNetwork>,
    /// Activations and prior-state buffers. **Logical state**: a recurrent
    /// organism's memory lives here.
    pub activations: Vec<ActivationState>,
    pub counters: MutationCounters,

    /// Per-tick scratch, rebuilt every tick and never logical state.
    pub requests: ActionRequests,
}

impl Schema2State {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            genomes: Vec::with_capacity(capacity),
            plans: Vec::with_capacity(capacity),
            activations: Vec::with_capacity(capacity),
            counters: MutationCounters::default(),
            requests: ActionRequests::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.genomes.len()
    }

    /// Append an organism, compiling its network.
    ///
    /// A genome whose network will not compile cannot reach here: decode and
    /// the mutation operators both reject a zero-delay cycle, so the fallback
    /// is an assertion rather than a runtime path.
    ///
    /// `budget` is the world's plasticity section, threaded through rather
    /// than read from a global: the plan records which edges are plastic and
    /// in which slot, and a plan compiled under a different budget would
    /// index a different organism's learned state. Passing `None` is a world
    /// with no plasticity section and produces exactly the pre-Phase-11 plan.
    pub fn push_organism(&mut self, genome: Genome2, budget: PlasticityBudget) -> bool {
        let network = genome.express_network();
        let Ok(plan) = compile_with_budget(&network, budget) else {
            return false;
        };
        self.activations.push(ActivationState::for_network(
            plan.node_count(),
            plan.plastic_edge_count(),
        ));
        self.plans.push(plan);
        self.genomes.push(genome);
        true
    }

    /// Plastic edges the organism at `index` carries, i.e. the length its
    /// learned-state row must have.
    pub fn plastic_edges(&self, index: usize) -> usize {
        self.plans[index].plastic_edge_count()
    }

    /// Expressed edge count per organism, the denominator C11.2's
    /// plastic-edge fraction divides by.
    pub fn edges_per_organism(&self) -> Vec<usize> {
        self.plans.iter().map(|plan| plan.edge_count()).collect()
    }

    /// Edges refused by the plastic-edge cap across the living population.
    pub fn plastic_over_cap(&self) -> u64 {
        self.plans
            .iter()
            .map(|plan| u64::from(plan.plastic_over_cap))
            .sum()
    }

    pub fn retain(&mut self, remove: &[bool]) {
        let mut write = 0_usize;
        for (read, removed) in remove.iter().enumerate() {
            if !removed {
                if write != read {
                    self.genomes.swap(write, read);
                    self.plans.swap(write, read);
                    self.activations.swap(write, read);
                }
                write += 1;
            }
        }
        self.genomes.truncate(write);
        self.plans.truncate(write);
        self.activations.truncate(write);
    }

    /// Mean expressed node and edge count across living organisms, in
    /// milli-units so the figure is exact.
    ///
    /// Reported alongside [`Self::median_structure`], not instead of it. The
    /// mean is exactly reconstructible from counts and moves continuously,
    /// so it is the sensitive detector of *any* structural change; the
    /// median is C9.1's stated quantity and answers the different question
    /// of whether change reached most of the population. A single duplicate
    /// carried by three organisms in a thousand moves the mean and not the
    /// median, and the two disagreeing that way is a finding rather than a
    /// discrepancy.
    pub fn mean_structure_milli(&self) -> (u64, u64) {
        if self.plans.is_empty() {
            return (0, 0);
        }
        let nodes: u64 = self.plans.iter().map(|plan| plan.node_count() as u64).sum();
        let edges: u64 = self.plans.iter().map(|plan| plan.edge_count() as u64).sum();
        let count = self.plans.len() as u64;
        (nodes * 1_000 / count, edges * 1_000 / count)
    }

    /// Median expressed node and edge count across living organisms.
    ///
    /// C9.1's stated quantity, and stated for a reason: evolved topology
    /// sizes are expected to be right-skewed, so one runaway lineage can
    /// carry a mean that describes no organism actually alive. These are
    /// whole counts, not milli-units, because a median of integers is an
    /// integer - the lower of the two middle values at even population,
    /// which is the same convention `world_demography` uses for lifespan.
    ///
    /// Note what a median can and cannot show. Founders are three nodes and
    /// two edges, so the median moves only once *half* the population has
    /// diverged from the founding topology. That is a deliberately hard bar
    /// and it is the bar C9.1 sets.
    pub fn median_structure(&self) -> (u64, u64) {
        if self.plans.is_empty() {
            return (0, 0);
        }
        let mut nodes: Vec<usize> = self.plans.iter().map(|plan| plan.node_count()).collect();
        let mut edges: Vec<usize> = self.plans.iter().map(|plan| plan.edge_count()).collect();
        nodes.sort_unstable();
        edges.sort_unstable();
        let middle = (nodes.len() - 1) / 2;
        (nodes[middle] as u64, edges[middle] as u64)
    }

    /// Distinct `(node count, edge count)` pairs among living organisms.
    ///
    /// C9.1's second clause: a population that evolved structure but whose
    /// members are all identical has not diversified, and the mean alone
    /// cannot tell the two apart.
    pub fn distinct_structures(&self) -> usize {
        let mut seen: Vec<(usize, usize)> = self
            .plans
            .iter()
            .map(|plan| (plan.node_count(), plan.edge_count()))
            .collect();
        seen.sort_unstable();
        seen.dedup();
        seen.len()
    }

    pub fn hash_into(&self, hasher: &mut Fnv1a64) {
        hasher.update(b"lifesim-genome2-state-v1");
        for genome in &self.genomes {
            // The encoded form is the canonical one, and it already carries
            // its own checksum, so hashing that is both cheaper and exactly
            // as discriminating as walking every locus.
            hasher.update_u64(crate::checksum::fnv1a64(&genome.encode()));
        }
        for activation in &self.activations {
            activation.hash_into(hasher);
        }
        self.counters.hash_into(hasher);
    }
}

/// Build the twelve-slot output array the Phase 2 intent mapping reads, from
/// a schema-2 organism's action requests.
///
/// Slots 8 to 11 are topology 1's memory writes and stay zero: schema 2 has
/// no memory registers. An unbound action channel is zero, which is the same
/// neutral value the mapping already assumes for an inert channel, so an
/// organism that binds nothing simply does nothing.
pub fn outputs_from_requests(
    requests: &ActionRequests,
) -> [f32; crate::genome::CONTROLLER_OUTPUTS] {
    let mut outputs = [0.0_f32; crate::genome::CONTROLLER_OUTPUTS];
    for (slot, channel_id) in ACTION_CHANNELS.iter().enumerate() {
        if let Some(value) = crate::controller2::output_of(requests, *channel_id) {
            outputs[slot] = value;
        }
    }
    outputs
}

/// Founder genome for a schema-2 world, built from the traits the origin
/// process drew.
///
/// The origin modes - deme centres, archetypes, biome-matched placement -
/// are unchanged and still draw a schema-1 founder; schema 2 re-expresses
/// that draw as a minimal diploid genome. Founders therefore differ between
/// the two schemas in *structure* and not in trait distribution, which is
/// what makes a schema-1 world a usable baseline for C9.2.
pub fn founder_from_traits(traits: &[f32; crate::genome::TRAIT_COUNT]) -> Genome2 {
    crate::structmut::minimal_founder(traits)
}

/// Founder genome for a world that also runs morphology: the minimal
/// schema-2 founder plus the one-rule growth program that makes the origin
/// module a gut.
///
/// Kept separate from [`founder_from_traits`] rather than made conditional
/// inside it, so a schema-2 world's founder is byte-identical to what it was
/// before Phase 10 existed and its fixture cannot move.
pub fn founder_with_morphology(traits: &[f32; crate::genome::TRAIT_COUNT]) -> Genome2 {
    let mut genome = founder_from_traits(traits);
    for (homology_id, rule) in crate::develop::founder_program() {
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
    genome
}

/// Homology slot of the founder's neutral marker locus.
///
/// **Between the two founder edge loci** (`BASE + 4_000` and `BASE + 5_000`),
/// which is not cosmetic. Crossover positions are drawn over *merged rank*,
/// so a marker one rank from each edge is as tightly linked to each plastic-
/// capable edge as those two edges are to each other. A marker parked at the
/// end of the chromosome would recombine away from the genes it controls for
/// faster than they recombine from each other, and it would drift under a
/// different linkage regime than the thing it is supposed to be matched to.
///
/// This is the best available matching and not a perfect one: no single
/// position is equidistant from two loci that are not adjacent, so the marker
/// is exactly one rank from each edge while the edges are two ranks apart.
/// Recorded rather than smoothed over.
pub const MARKER_HOMOLOGY_ID: u32 = crate::genome2::STRUCTURAL_HOMOLOGY_BASE + 4_500;

/// Add the neutral marker locus to every haplotype of a founder genome.
///
/// Kept out of [`founder_from_traits`] and applied on top, exactly as
/// [`founder_with_morphology`] is, so a founder in a world with the probe
/// section disabled is byte-identical to what it was before this existed and
/// no fixture can move.
///
/// Both alleles start at the **same** place their targets start at: `value`
/// at 0.0, matching `PlasticityGenes::inert`'s `eta`, and the neutral flag
/// clear, matching `minimal_founder`'s `flags: 0`. A founder population is
/// therefore monomorphic at the marker and at `eta` alike, so both
/// distributions have the same amount of variance to build - none - and the
/// comparison starts matched instead of starting with the control already
/// spread out.
pub fn with_marker_locus(mut genome: Genome2) -> Genome2 {
    let locus = crate::genome2::Locus {
        homology_id: MARKER_HOMOLOGY_ID,
        // Derived from the slot exactly as every other founder locus's is
        // (`minimal_founder` uses `u64::from(homology_id)`), so two founders
        // agree on marker identity.
        //
        // **Alignment during meiosis is `homology_id`'s job, not this
        // field's.** `gene_lineage_id` is provenance only: nothing in the
        // engine reads it, so a marker whose lineage id were derived some
        // other way would inherit, recombine, mutate and express exactly the
        // same. Stated because the alternative reading - that this line is
        // what keeps the control matched - is wrong, and a mutation of it is
        // unobservable rather than defended.
        gene_lineage_id: u64::from(MARKER_HOMOLOGY_ID),
        mutation_event_id: 0,
        kind: crate::genome2::LocusKind::Marker {
            value: 0.0,
            flags: 0,
        },
    };
    for haplotype in &mut genome.haplotypes {
        for chromosome in &mut haplotype.chromosomes {
            chromosome.push(locus);
            chromosome.sort_unstable_by_key(|locus| locus.homology_id);
        }
    }
    genome
}

/// Validate every genome against the caps, for the world invariant check.
pub(crate) fn validate_all(state: &Schema2State, caps: &GenomeCaps) -> Result<(), usize> {
    for (index, genome) in state.genomes.iter().enumerate() {
        if genome.validate_structure(caps).is_err() {
            return Err(index);
        }
    }
    Ok(())
}

/// Compatibility distance between two schema-2 genomes, in `[0, 1]`.
///
/// `distance = w_t * trait_distance + w_s * structural_distance`, where
/// structural distance is the fraction of `homology_id` values not shared
/// between the two expressed networks.
///
/// **This is physics, not an analysis label.** It is computed from the
/// records themselves and would still function if every analysis module were
/// deleted, which is the line `docs/25-emergence-and-epistemic-position.md`
/// draws. The weights are equal for now and are a config question the
/// compatibility sweep in the phase plan exists to settle; the risk being
/// managed is that structural distance creates instant reproductive
/// isolation and fragments the population.
pub fn compatibility_distance(left: &Genome2, right: &Genome2) -> f32 {
    let left_traits = left.express_traits();
    let right_traits = right.express_traits();
    let mut trait_sum = 0.0_f32;
    let mut trait_count = 0_u32;
    for slot in 0..crate::genome::TRAIT_COUNT {
        if let (Some(a), Some(b)) = (left_traits[slot], right_traits[slot]) {
            trait_sum += (a - b).abs();
            trait_count += 1;
        }
    }
    let trait_distance = if trait_count == 0 {
        0.0
    } else {
        trait_sum / trait_count as f32
    };

    let ids_of = |genome: &Genome2| -> Vec<u32> {
        let network = genome.express_network();
        let mut ids: Vec<u32> = network
            .nodes
            .iter()
            .map(|node| node.homology_id)
            .chain(network.edges.iter().map(|edge| edge.homology_id))
            .collect();
        ids.sort_unstable();
        ids.dedup();
        ids
    };
    let left_ids = ids_of(left);
    let right_ids = ids_of(right);
    let mut shared = 0_usize;
    let (mut i, mut j) = (0_usize, 0_usize);
    while i < left_ids.len() && j < right_ids.len() {
        match left_ids[i].cmp(&right_ids[j]) {
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
            std::cmp::Ordering::Equal => {
                shared += 1;
                i += 1;
                j += 1;
            }
        }
    }
    let union = left_ids.len() + right_ids.len() - shared;
    let structural_distance = if union == 0 {
        0.0
    } else {
        1.0 - (shared as f32 / union as f32)
    };

    (0.5 * trait_distance + 0.5 * structural_distance).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn founder() -> Genome2 {
        founder_from_traits(&[0.5; crate::genome::TRAIT_COUNT])
    }

    #[test]
    fn the_action_channel_map_covers_every_non_memory_output() {
        // Topology 1 has twelve outputs, eight of which are actions and four
        // memory writes. If this drifts, an evolved organism's action would
        // land in the wrong intent slot.
        assert_eq!(ACTION_CHANNELS.len(), 8);
        assert_eq!(crate::registry::output_channels().count(), 8);
        for channel_id in ACTION_CHANNELS {
            assert!(crate::registry::channel_exists(channel_id));
        }
        assert_eq!(SENSE_CHANNELS.len(), 16);
        assert_eq!(crate::registry::input_channels().count(), 16);
    }

    #[test]
    fn unbound_action_channels_produce_neutral_intents() {
        let requests: ActionRequests = vec![(103, 0.75)];
        let outputs = outputs_from_requests(&requests);
        assert_eq!(outputs[2], 0.75, "the eat channel");
        assert!(
            outputs
                .iter()
                .enumerate()
                .all(|(slot, value)| slot == 2 || *value == 0.0),
            "an unbound channel produced a non-neutral intent"
        );
        // Memory slots stay zero: schema 2 has no memory registers.
        assert_eq!(&outputs[8..12], &[0.0; 4]);
    }

    #[test]
    fn state_stays_in_lockstep_across_births_and_deaths() {
        let mut state = Schema2State::with_capacity(4);
        for _ in 0..4 {
            assert!(state.push_organism(founder(), None));
        }
        assert_eq!(state.len(), 4);
        assert_eq!(state.plans.len(), 4);
        assert_eq!(state.activations.len(), 4);
        state.retain(&[false, true, false, true]);
        assert_eq!(
            (state.len(), state.plans.len(), state.activations.len()),
            (2, 2, 2)
        );
    }

    /// A network of `inputs` input nodes all feeding one output node, with
    /// the first edge heavy and the rest at the f32-epsilon scale.
    ///
    /// The weights are the same magnitudes
    /// `controller2::incoming_edges_are_summed_in_homology_order_not_storage_order`
    /// uses, and for the same reason: adding the heavy weight first loses
    /// every small one, while accumulating the small ones first does not. A
    /// compaction test built on weights where both summation orders agree
    /// would pass whatever compaction did to the edge lists.
    fn fan_in(inputs: u32) -> Genome2 {
        use crate::genome2::{Haplotype, Locus, LocusKind, PlasticityGenes};
        use crate::registry::{Activation, NodeRole};

        const BASE: u32 = crate::genome2::STRUCTURAL_HOMOLOGY_BASE;
        let output_id = BASE + 1;
        let mut loci: Vec<Locus> = Vec::new();
        let mut push = |homology_id: u32, kind: LocusKind| {
            loci.push(Locus {
                homology_id,
                gene_lineage_id: u64::from(homology_id),
                mutation_event_id: 0,
                kind,
            });
        };
        push(
            output_id,
            LocusKind::Node {
                role: NodeRole::Output,
                activation_id: Activation::Linear.id(),
                bias: 0.0,
                time_constant: 0,
            },
        );
        for index in 0..inputs {
            let node_id = BASE + 100 + index;
            push(
                node_id,
                LocusKind::Node {
                    role: NodeRole::Input,
                    activation_id: Activation::Linear.id(),
                    bias: 0.0,
                    time_constant: 0,
                },
            );
            push(
                BASE + 10_000 + index,
                LocusKind::Edge {
                    source: node_id,
                    target: output_id,
                    weight: if index == 0 { 1.0 } else { 6.0e-8 },
                    flags: 0,
                    plasticity: PlasticityGenes::inert(),
                },
            );
            push(
                BASE + 20_000 + index,
                LocusKind::IoBinding {
                    node: node_id,
                    channel_id: SENSE_CHANNELS[index as usize % SENSE_CHANNELS.len()],
                    gain: 1.0,
                },
            );
        }
        push(
            BASE + 30_000,
            LocusKind::IoBinding {
                node: output_id,
                channel_id: ACTION_CHANNELS[0],
                gain: 1.0,
            },
        );
        loci.sort_unstable_by_key(|locus| locus.homology_id);
        let haplotype = || Haplotype {
            chromosomes: vec![loci.clone()],
        };
        Genome2 {
            haplotypes: [haplotype(), haplotype()],
        }
    }

    /// Evaluate one organism for one tick and return its activations and its
    /// action requests, so two states can be compared on behaviour and not
    /// only on bytes.
    fn evaluate_one(
        state: &mut Schema2State,
        index: usize,
        input: f32,
    ) -> (Vec<u32>, ActionRequests) {
        let mut requests = ActionRequests::new();
        crate::controller2::evaluate(
            &state.plans[index],
            &mut state.activations[index],
            &[],
            &|_| input,
            &mut requests,
        );
        (
            state.activations[index]
                .values
                .iter()
                .map(|value| value.to_bits())
                .collect(),
            requests,
        )
    }

    fn content_hash(state: &Schema2State) -> u64 {
        let mut hasher = Fnv1a64::new();
        state.hash_into(&mut hasher);
        hasher.finish()
    }

    #[test]
    fn compaction_leaves_the_survivors_identical_to_a_state_built_from_them() {
        // What `state_stays_in_lockstep_across_births_and_deaths` checks is
        // three lengths. Three arrays can be the right length and hold the
        // wrong organisms' contents, which is the whole failure mode
        // `InvariantViolation::Schema2Desync` exists to name, so this checks
        // content and behaviour instead.
        //
        // Two comparisons, because neither alone is sufficient:
        //
        // - `hash_into` covers genomes and activations, and would catch a
        //   `retain` that moved one and not the other. It does **not** cover
        //   `plans`, which are derived and deliberately excluded from the
        //   checksum.
        // - Evaluation covers `plans`. A `retain` that compacted genomes and
        //   activations correctly but left `plans` behind would produce a
        //   state that hashes identically and evaluates as a different
        //   organism, and only the second comparison sees it.
        //
        // The organisms are given genuinely different networks - different
        // node counts, different in-degrees - so a mis-paired plan cannot
        // coincidentally behave the same way.
        let sizes = [1_u32, 2, 3, 4, 5];
        let mut state = Schema2State::with_capacity(sizes.len());
        for inputs in sizes {
            assert!(state.push_organism(fan_in(inputs), None));
        }
        // Distinct prior-state buffers: a recurrent organism's memory lives
        // here, and it is the array a length-only test cannot see move.
        for (index, activation) in state.activations.iter_mut().enumerate() {
            for (slot, value) in activation.prior.iter_mut().enumerate() {
                *value = (index as f32 + 1.0) / 16.0 + (slot as f32) / 256.0;
            }
            activation.faults = index as u32;
        }
        assert!(
            state.plans.iter().map(|plan| plan.node_count()).max()
                != state.plans.iter().map(|plan| plan.node_count()).min(),
            "every organism has the same network, so a mis-paired plan would \
             be invisible"
        );

        // Keep 0, 2 and 4: a pattern that forces every survivor after the
        // first to move, so a `retain` that only truncated would be caught.
        let removed = [false, true, false, true, false];
        let survivors: Vec<usize> = (0..sizes.len()).filter(|index| !removed[*index]).collect();
        let mut fresh = Schema2State::with_capacity(survivors.len());
        for &index in &survivors {
            assert!(fresh.push_organism(fan_in(sizes[index]), None));
        }
        for (slot, &index) in survivors.iter().enumerate() {
            fresh.activations[slot]
                .prior
                .clone_from(&state.activations[index].prior);
            fresh.activations[slot].faults = state.activations[index].faults;
        }
        // The two states must differ *before* compaction, or the equality
        // afterwards is an equality between a value and its own copy.
        assert_ne!(content_hash(&state), content_hash(&fresh));

        state.retain(&removed);
        assert_eq!(state.len(), survivors.len());
        assert_eq!(
            content_hash(&state),
            content_hash(&fresh),
            "compaction did not leave the survivors' genomes and activations \
             matching a state built from those survivors alone"
        );

        for index in 0..survivors.len() {
            assert_eq!(
                state.plans[index], fresh.plans[index],
                "organism {index} carries a plan that is not its own after \
                 compaction"
            );
            for tick in 0..3 {
                let input = 0.25 + tick as f32 / 8.0;
                assert_eq!(
                    evaluate_one(&mut state, index, input),
                    evaluate_one(&mut fresh, index, input),
                    "organism {index} evaluated differently after compaction"
                );
                crate::controller2::commit(&mut state.activations[index]);
                crate::controller2::commit(&mut fresh.activations[index]);
            }
        }
    }

    #[test]
    fn structure_statistics_report_what_c9_1_measures() {
        let mut state = Schema2State::with_capacity(2);
        state.push_organism(founder(), None);
        state.push_organism(founder(), None);
        let (nodes, edges) = state.mean_structure_milli();
        assert_eq!(nodes, 3_000, "three nodes, in milli");
        assert_eq!(edges, 2_000);
        // Identical organisms are one distinct structure, however many there
        // are; that is the point of the second clause.
        assert_eq!(state.distinct_structures(), 1);
    }

    #[test]
    fn the_checksum_notices_a_genome_and_an_activation() {
        let hash = |state: &Schema2State| {
            let mut hasher = Fnv1a64::new();
            state.hash_into(&mut hasher);
            hasher.finish()
        };
        let mut left = Schema2State::with_capacity(1);
        left.push_organism(founder(), None);
        let mut right = Schema2State::with_capacity(1);
        right.push_organism(founder(), None);
        assert_eq!(hash(&left), hash(&right));

        // A differing activation must move it, because that is a recurrent
        // organism's memory.
        left.activations[0].values[0] = 0.5;
        assert_ne!(hash(&left), hash(&right));
        left.activations[0].values[0] = 0.0;
        assert_eq!(hash(&left), hash(&right));

        // ...and so must a differing genome.
        let mut other = Schema2State::with_capacity(1);
        other.push_organism(
            founder_from_traits(&[0.25; crate::genome::TRAIT_COUNT]),
            None,
        );
        assert_ne!(hash(&left), hash(&other));
    }
}
