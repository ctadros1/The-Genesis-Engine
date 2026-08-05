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
use crate::controller2::{ActionRequests, ActivationState, CompiledNetwork, compile};
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
    pub fn push_organism(&mut self, genome: Genome2) -> bool {
        let network = genome.express_network();
        let Ok(plan) = compile(&network) else {
            return false;
        };
        self.activations
            .push(ActivationState::for_network(plan.node_count()));
        self.plans.push(plan);
        self.genomes.push(genome);
        true
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
            assert!(state.push_organism(founder()));
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

    #[test]
    fn structure_statistics_report_what_c9_1_measures() {
        let mut state = Schema2State::with_capacity(2);
        state.push_organism(founder());
        state.push_organism(founder());
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
        left.push_organism(founder());
        let mut right = Schema2State::with_capacity(1);
        right.push_organism(founder());
        assert_eq!(hash(&left), hash(&right));

        // A differing activation must move it, because that is a recurrent
        // organism's memory.
        left.activations[0].values[0] = 0.5;
        assert_ne!(hash(&left), hash(&right));
        left.activations[0].values[0] = 0.0;
        assert_eq!(hash(&left), hash(&right));

        // ...and so must a differing genome.
        let mut other = Schema2State::with_capacity(1);
        other.push_organism(founder_from_traits(&[0.25; crate::genome::TRAIT_COUNT]));
        assert_ne!(hash(&left), hash(&other));
    }
}
