//! Controller v2: hybrid evaluation over an arbitrary graph (Phase 9,
//! `lifesim-controller-v2`).
//!
//! The contract is ADR-0022 A9 as reaffirmed by D-066, and it is a hybrid
//! rather than the fully synchronous update ADR-0013 originally described:
//!
//! - **Zero-delay edges evaluate in a canonical topological order** over the
//!   acyclic subgraph, so information crosses several edges within one tick.
//!   A fully synchronous update costs one edge of propagation per tick,
//!   which makes a deep network unable to respond within a tick at all.
//! - **Delayed and recurrent edges read prior-state buffers.** That breaks
//!   every cycle by construction and needs no cycle special case, no
//!   relaxation iteration, and no convergence assumption.
//! - **A cycle among zero-delay edges is a decode-time error**, refused by
//!   `Genome2::validate_structure`, because under this scheme it has no
//!   fixed point. Inventing an iteration order that produces *a* number
//!   would be worse than refusing the genome.
//!
//! Three determinism obligations, and the third is the one this project's
//! own notes call the easiest to overlook:
//!
//! - **The topological order is canonical.** Kahn's algorithm with the ready
//!   set always drained in ascending `homology_id`, so the order is a pure
//!   function of the graph and never of storage layout.
//! - **Activations and prior-state buffers are world state.** They are
//!   saved and checksummed; they are not derived and must not be recomputed
//!   on load, because a recurrent network's behaviour depends on them.
//! - **Per-node incoming edges are summed in ascending edge `homology_id`
//!   order, never storage order.** Float addition is not associative, so a
//!   storage-order sum is a replay bug that stays invisible until a
//!   compaction changes layout. The merged list interleaves zero-delay and
//!   delayed edges by ID rather than grouping them by kind, precisely so
//!   this order is the *only* thing that decides the sum.

use crate::checksum::Fnv1a64;
use crate::genome2::ExpressedNetwork;
use crate::registry::{Activation, ChannelDirection, NodeRole, channel};

pub const CONTROLLER2_POLICY_VERSION: &str = "lifesim-controller-v2";

/// Pre-activation clamp, unchanged from schema 1.
const ACTIVATION_LIMIT: f32 = 8.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompileError {
    /// A cycle among zero-delay edges. `validate_structure` refuses these at
    /// decode, so reaching this is a bug rather than a runtime condition -
    /// but it is returned rather than panicked, because a compile path that
    /// can panic is a compile path that can take down a tick.
    ZeroDelayCycle,
    /// An edge or binding referring to a node not in the expressed network.
    DanglingReference(u32),
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

/// One incoming edge, resolved to a node index.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IncomingEdge {
    /// Kept so the sort key is inspectable and testable.
    pub homology_id: u32,
    pub source: u32,
    pub weight: f32,
    /// Delayed edges read the prior-state buffer; zero-delay edges read the
    /// value computed earlier this tick.
    pub delayed: bool,
}

/// A network compiled for evaluation.
///
/// Compilation is a pure function of the expressed network, so it can be
/// cached against structural identity and rebuilt whenever the genome
/// changes. It is deliberately separate from evaluation: the topological
/// sort is the expensive part and it does not depend on the tick.
#[derive(Clone, Debug, PartialEq)]
pub struct CompiledNetwork {
    pub node_ids: Vec<u32>,
    pub roles: Vec<NodeRole>,
    pub activations: Vec<Activation>,
    pub biases: Vec<f32>,
    /// Node indices in canonical topological order over the zero-delay
    /// subgraph.
    pub order: Vec<u32>,
    /// Per node, incoming edges in ascending edge `homology_id` order.
    pub incoming: Vec<Vec<IncomingEdge>>,
    /// `(channel_id, node index, gain)`, ascending by binding
    /// `homology_id`, so repeated bindings to one channel accumulate in a
    /// pinned order.
    pub input_bindings: Vec<(u16, u32, f32)>,
    pub output_bindings: Vec<(u16, u32, f32)>,
}

impl CompiledNetwork {
    pub fn node_count(&self) -> usize {
        self.node_ids.len()
    }

    pub fn edge_count(&self) -> usize {
        self.incoming.iter().map(Vec::len).sum()
    }
}

/// Compile an expressed network.
pub fn compile(network: &ExpressedNetwork) -> Result<CompiledNetwork, CompileError> {
    let node_ids: Vec<u32> = network.nodes.iter().map(|node| node.homology_id).collect();
    // The expressed network is sorted by homology, so a node's index *is*
    // its homology rank and a binary search is the canonical lookup.
    let index_of = |id: u32| -> Result<u32, CompileError> {
        node_ids
            .binary_search(&id)
            .map(|index| index as u32)
            .map_err(|_| CompileError::DanglingReference(id))
    };

    let mut incoming: Vec<Vec<IncomingEdge>> = vec![Vec::new(); node_ids.len()];
    let mut zero_delay: Vec<(u32, u32)> = Vec::new();
    // `network.edges` is already ascending by `homology_id`, so pushing in
    // iteration order leaves each node's list in that order. That is the
    // summation order, and it is why nothing here sorts by anything else.
    for edge in &network.edges {
        if edge.disabled {
            continue;
        }
        let source = index_of(edge.source)?;
        let target = index_of(edge.target)?;
        incoming[target as usize].push(IncomingEdge {
            homology_id: edge.homology_id,
            source,
            weight: edge.weight,
            delayed: edge.delayed,
        });
        if !edge.delayed {
            zero_delay.push((source, target));
        }
    }
    debug_assert!(
        incoming
            .iter()
            .all(|list| list.windows(2).all(|w| w[0].homology_id < w[1].homology_id)),
        "incoming edge lists must be ascending by homology_id"
    );

    let order = canonical_topological_order(node_ids.len(), &zero_delay)?;

    let mut input_bindings = Vec::new();
    let mut output_bindings = Vec::new();
    for binding in &network.bindings {
        let node = index_of(binding.node)?;
        // A binding to a channel this build does not know cannot reach here:
        // decode refuses it. Direction decides which list it joins, so an
        // organism can never read an action or write a sensor.
        match channel(binding.channel_id).map(|entry| entry.direction) {
            Some(ChannelDirection::Input) => {
                input_bindings.push((binding.channel_id, node, binding.gain))
            }
            Some(ChannelDirection::Output) => {
                output_bindings.push((binding.channel_id, node, binding.gain))
            }
            None => {
                return Err(CompileError::DanglingReference(u32::from(
                    binding.channel_id,
                )));
            }
        }
    }

    Ok(CompiledNetwork {
        node_ids,
        roles: network.nodes.iter().map(|node| node.role).collect(),
        activations: network.nodes.iter().map(|node| node.activation).collect(),
        biases: network.nodes.iter().map(|node| node.bias).collect(),
        order,
        incoming,
        input_bindings,
        output_bindings,
    })
}

/// Kahn's algorithm with the ready set drained in ascending node index.
///
/// Node index is homology rank, so "ascending index" is "ascending
/// `homology_id`" and the order is a pure function of the graph. A plain
/// stack or a hash set would produce a valid topological order too, and a
/// *different* one on a different layout - which is exactly the class of
/// replay bug that stays invisible until something reorders storage.
fn canonical_topological_order(
    node_count: usize,
    edges: &[(u32, u32)],
) -> Result<Vec<u32>, CompileError> {
    let mut in_degree = vec![0_u32; node_count];
    let mut adjacency: Vec<Vec<u32>> = vec![Vec::new(); node_count];
    for &(source, target) in edges {
        adjacency[source as usize].push(target);
        in_degree[target as usize] += 1;
    }
    // A min-heap over indices gives the canonical drain order without
    // re-scanning the whole in-degree array each step.
    let mut ready: std::collections::BinaryHeap<std::cmp::Reverse<u32>> = (0..node_count as u32)
        .filter(|index| in_degree[*index as usize] == 0)
        .map(std::cmp::Reverse)
        .collect();
    let mut order = Vec::with_capacity(node_count);
    while let Some(std::cmp::Reverse(node)) = ready.pop() {
        order.push(node);
        for &next in &adjacency[node as usize] {
            in_degree[next as usize] -= 1;
            if in_degree[next as usize] == 0 {
                ready.push(std::cmp::Reverse(next));
            }
        }
    }
    if order.len() != node_count {
        return Err(CompileError::ZeroDelayCycle);
    }
    Ok(order)
}

/// Per-organism activation state.
///
/// **World state, not derived.** A recurrent network's behaviour depends on
/// what its delayed edges read, so recomputing this on load would silently
/// restart every memory the organism had. Saved and checksummed under
/// `lifesim-activation-state-v1`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ActivationState {
    /// This tick's activation per node, in ascending `homology_id` order.
    pub values: Vec<f32>,
    /// Last tick's activation, which delayed edges read.
    pub prior: Vec<f32>,
    /// Non-finite values neutralized over this organism's lifetime.
    pub faults: u32,
    /// Per-tick gathered sensory contribution. **Scratch, not logical
    /// state**: rebuilt from nothing every tick, excluded from the
    /// checksum, and carried here only so evaluation allocates nothing --
    /// the same treatment the contest intent buffers get.
    gathered: Vec<f32>,
}

impl ActivationState {
    pub fn for_network(node_count: usize) -> Self {
        Self {
            values: vec![0.0; node_count],
            prior: vec![0.0; node_count],
            faults: 0,
            gathered: vec![0.0; node_count],
        }
    }

    /// Resize after a structural change, preserving what still lines up.
    ///
    /// Only used when a network is recompiled for an organism that already
    /// exists; a newborn starts from zero, because learned or accumulated
    /// activation is never inherited.
    pub fn resize(&mut self, node_count: usize) {
        self.values.resize(node_count, 0.0);
        self.prior.resize(node_count, 0.0);
        self.gathered.resize(node_count, 0.0);
    }

    pub fn hash_into(&self, hasher: &mut Fnv1a64) {
        hasher.update(b"lifesim-activation-state-v1");
        // Bit patterns rather than values: two f32s that compare equal have
        // the same bits here, and a checksum that ignored the difference
        // between 0.0 and -0.0 would hide a real divergence.
        for value in &self.values {
            hasher.update_u32(value.to_bits());
        }
        for value in &self.prior {
            hasher.update_u32(value.to_bits());
        }
        hasher.update_u32(self.faults);
        // `gathered` is deliberately absent: it is scratch, rebuilt every
        // tick before it is read, so hashing it would make the checksum
        // depend on where in the tick it was taken.
    }
}

/// One tick's action requests: `(channel_id, value)`, ascending by channel.
pub type ActionRequests = Vec<(u16, f32)>;

/// Evaluate one organism for one tick.
///
/// `input` supplies a sensory channel's current value; it is only called for
/// channels the organism actually binds, so an unbound channel costs
/// nothing - there is no "documented neutral zero" placeholder any more.
///
/// `requests` is cleared and refilled, so a caller reusing one buffer across
/// organisms allocates nothing. **Evaluation performs no allocation at all**:
/// schema 1's controller had that property with stack buffers, and variable
/// topology must not silently give it up.
pub fn evaluate(
    plan: &CompiledNetwork,
    state: &mut ActivationState,
    input: &dyn Fn(u16) -> f32,
    requests: &mut ActionRequests,
) {
    debug_assert_eq!(state.values.len(), plan.node_count());

    // Gathered sensory contribution per node. Bindings are in ascending
    // homology order, so two bindings onto one node accumulate in a pinned
    // order.
    let gathered = &mut state.gathered;
    gathered.fill(0.0);
    let mut faults = 0_u32;
    for &(channel_id, node, gain) in &plan.input_bindings {
        let raw = input(channel_id);
        let sanitized = if raw.is_finite() {
            raw.clamp(-1.0, 1.0)
        } else {
            faults += 1;
            0.0
        };
        gathered[node as usize] += sanitized * gain;
    }

    for &node in &plan.order {
        let index = node as usize;
        let mut sum = plan.biases[index] + state.gathered[index];
        // Ascending edge `homology_id`, zero-delay and delayed interleaved.
        // Grouping by kind would be faster and would change the sum.
        for edge in &plan.incoming[index] {
            let source = edge.source as usize;
            let value = if edge.delayed {
                state.prior[source]
            } else {
                // The topological order guarantees a zero-delay source was
                // computed earlier this tick.
                state.values[source]
            };
            sum += edge.weight * value;
        }
        let activated =
            plan.activations[index].apply(sum.clamp(-ACTIVATION_LIMIT, ACTIVATION_LIMIT));
        state.values[index] = if activated.is_finite() {
            activated
        } else {
            faults += 1;
            0.0
        };
    }
    state.faults = state.faults.saturating_add(faults);

    requests.clear();
    for &(channel_id, node, gain) in &plan.output_bindings {
        let value = (state.values[node as usize] * gain).clamp(-1.0, 1.0);
        match requests.binary_search_by_key(&channel_id, |(existing, _)| *existing) {
            Ok(index) => requests[index].1 = (requests[index].1 + value).clamp(-1.0, 1.0),
            Err(index) => requests.insert(index, (channel_id, value)),
        }
    }
}

/// Roll this tick's activations into the prior-state buffer.
///
/// Called once, after **every** organism has been evaluated, exactly as
/// schema 1's memory values become next-tick memory only after all
/// controller evaluation completes. Doing it inline would let an organism
/// evaluated later in the tick read a neighbour's *current* activation
/// through a delayed edge, which is not what delayed means.
pub fn commit(state: &mut ActivationState) {
    state.prior.copy_from_slice(&state.values);
}

/// Diagnostic: the value bound to each output channel, for tests and the
/// observer. Never used by the tick.
pub fn output_of(requests: &ActionRequests, channel_id: u16) -> Option<f32> {
    requests
        .binary_search_by_key(&channel_id, |(existing, _)| *existing)
        .ok()
        .map(|index| requests[index].1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genome2::{
        ExpressedBinding, ExpressedEdge, ExpressedNode, Genome2, Haplotype, Locus, LocusKind,
        STRUCTURAL_HOMOLOGY_BASE, VALUE_LIMIT,
    };

    fn node(homology_id: u32, role: NodeRole, activation: Activation, bias: f32) -> ExpressedNode {
        ExpressedNode {
            homology_id,
            role,
            activation,
            bias,
            time_constant: 0,
        }
    }

    fn edge(homology_id: u32, source: u32, target: u32, weight: f32) -> ExpressedEdge {
        ExpressedEdge {
            homology_id,
            source,
            target,
            weight,
            disabled: false,
            plastic: false,
            delayed: false,
        }
    }

    fn delayed(homology_id: u32, source: u32, target: u32, weight: f32) -> ExpressedEdge {
        ExpressedEdge {
            delayed: true,
            ..edge(homology_id, source, target, weight)
        }
    }

    fn binding(homology_id: u32, node: u32, channel_id: u16, gain: f32) -> ExpressedBinding {
        ExpressedBinding {
            homology_id,
            node,
            channel_id,
            gain,
        }
    }

    /// A three-node chain: input 1 -> hidden 2 -> output 3, all linear so
    /// the arithmetic is exact and checkable by hand.
    fn chain() -> ExpressedNetwork {
        ExpressedNetwork {
            nodes: vec![
                node(10, NodeRole::Input, Activation::Linear, 0.0),
                node(20, NodeRole::Hidden, Activation::Linear, 0.0),
                node(30, NodeRole::Output, Activation::Linear, 0.0),
            ],
            edges: vec![edge(40, 10, 20, 1.0), edge(50, 20, 30, 1.0)],
            bindings: vec![binding(60, 10, 1, 1.0), binding(70, 30, 101, 1.0)],
        }
    }

    fn run(
        network: &ExpressedNetwork,
        value: f32,
        ticks: usize,
    ) -> (ActionRequests, ActivationState) {
        let plan = compile(network).expect("compiles");
        let mut state = ActivationState::for_network(plan.node_count());
        let mut requests = ActionRequests::new();
        for _ in 0..ticks {
            evaluate(&plan, &mut state, &|_| value, &mut requests);
            commit(&mut state);
        }
        (requests, state)
    }

    /// Evaluate once into a fresh buffer, for tests that want one tick.
    fn once(plan: &CompiledNetwork, state: &mut ActivationState, value: f32) -> ActionRequests {
        let mut requests = ActionRequests::new();
        evaluate(plan, state, &|_| value, &mut requests);
        requests
    }

    #[test]
    fn zero_delay_propagation_crosses_the_whole_chain_in_one_tick() {
        // The property the hybrid exists for, and the one a fully
        // synchronous update cannot have: a signal entering at the input
        // reaches the output within the *same* tick, not one edge per tick.
        let (requests, _) = run(&chain(), 0.5, 1);
        assert_eq!(
            output_of(&requests, 101),
            Some(0.5),
            "the signal did not cross two edges in one tick"
        );
    }

    #[test]
    fn a_delayed_edge_costs_exactly_one_tick_of_propagation() {
        // The contrast that makes the previous test meaningful. The same
        // chain with delayed edges takes one tick per edge, so after one
        // tick the output is still zero and after two it is not.
        let mut network = chain();
        network.edges = vec![delayed(40, 10, 20, 1.0), delayed(50, 20, 30, 1.0)];
        let plan = compile(&network).expect("compiles");
        let mut state = ActivationState::for_network(plan.node_count());

        let first = once(&plan, &mut state, 1.0);
        commit(&mut state);
        assert_eq!(output_of(&first, 101), Some(0.0), "tick 1");
        let second = once(&plan, &mut state, 1.0);
        commit(&mut state);
        assert_eq!(output_of(&second, 101), Some(0.0), "tick 2");
        let third = once(&plan, &mut state, 1.0);
        commit(&mut state);
        assert_eq!(output_of(&third, 101), Some(1.0), "tick 3");
    }

    #[test]
    fn a_recurrent_cycle_evaluates_without_special_handling() {
        // A cycle closed by a delayed edge is legal and useful: it is how an
        // organism evolves memory for itself, replacing schema 1's fixed
        // register file. It must simply evaluate, with no relaxation and no
        // convergence assumption.
        let mut network = chain();
        network.edges.push(delayed(80, 30, 20, 0.5));
        let plan = compile(&network).expect("a delayed cycle compiles");
        let mut state = ActivationState::for_network(plan.node_count());
        // Feed a pulse, then silence, and watch the loop carry it.
        let mut history = Vec::new();
        for tick in 0..6 {
            let value = if tick == 0 { 1.0 } else { 0.0 };
            let requests = once(&plan, &mut state, value);
            commit(&mut state);
            history.push(output_of(&requests, 101).expect("bound"));
        }
        assert_eq!(history[0], 1.0);
        // The echo decays by the loop gain each tick rather than vanishing.
        assert!(
            (history[1] - 0.5).abs() < 1e-6 && (history[2] - 0.25).abs() < 1e-6,
            "the recurrent loop did not carry state: {history:?}"
        );
    }

    #[test]
    fn a_zero_delay_cycle_is_refused_by_the_compiler() {
        // Decode refuses these, so this is the belt to that braces: the
        // compiler must not silently invent an order for a graph with no
        // fixed point.
        let mut network = chain();
        network.edges.push(edge(80, 30, 10, 0.5));
        assert_eq!(compile(&network), Err(CompileError::ZeroDelayCycle));
    }

    /// One structural locus, in the reserved structural homology block.
    fn structural(offset: u32, kind: LocusKind) -> Locus {
        Locus {
            homology_id: STRUCTURAL_HOMOLOGY_BASE + offset,
            gene_lineage_id: u64::from(offset) + 1,
            mutation_event_id: 0,
            kind,
        }
    }

    /// A homozygous diploid genome carrying `layout` on both haplotypes,
    /// where `layout` is a list of chromosomes.
    fn genome_with_layout(layout: &[&[Locus]]) -> Genome2 {
        let haplotype = || Haplotype {
            chromosomes: layout
                .iter()
                .map(|chromosome| chromosome.to_vec())
                .collect(),
        };
        Genome2 {
            haplotypes: [haplotype(), haplotype()],
        }
    }

    #[test]
    fn expression_is_independent_of_how_loci_are_split_across_chromosomes() {
        // The determinism obligation, tested through a genuinely different
        // storage layout.
        //
        // **The test this replaces was vacuous.** It reversed an already
        // sorted `Vec` and then `sort_by_key`ed it back; the keys are
        // distinct and ascending, so the stable sort restored the original
        // vector exactly and the assertion compared a value with a copy of
        // itself. The permutation was undone before the code under test ever
        // saw it, which is precisely the trap this project's own notes name.
        //
        // `validate_structure` requires loci to ascend only *within* a
        // chromosome, so the same locus set partitioned differently is a
        // legal and genuinely distinct layout: the split below reaches
        // `express_network`'s traversal in the order 10, 30, 50, 70, 81, 90,
        // 20, 40, 60, 80, 82, which is not ascending anywhere. `upsert`
        // merges by binary search, so the expressed network should be
        // order-free by construction - and if it were ever changed to append,
        // node 40's incoming list would arrive as 50, 70, 60 and the sum
        // would move, which the weights below are chosen to make visible.
        //
        // **This must not be lifted to world-level checksum equality.** The
        // chromosome count is an input to meiosis: crossover draws are per
        // chromosome pair, so two worlds whose founders are partitioned
        // differently consume the meiosis stream differently and legitimately
        // diverge at the first birth. The claim here is about expression and
        // evaluation, and that is the whole claim.
        let big = 1.0_f32;
        // Straddling f32 epsilon (about 1.19e-7) for the same reason
        // `incoming_edges_are_summed_in_homology_order_not_storage_order`
        // does: adding each small weight to 1.0 separately loses both, while
        // accumulating them first does not. Weights on which both orders
        // agree would make this test pass without demonstrating anything.
        let small = 6.0e-8_f32;
        let hidden = |offset: u32, role: NodeRole| {
            structural(
                offset,
                LocusKind::Node {
                    role,
                    activation_id: Activation::Linear.id(),
                    bias: 0.0,
                    time_constant: 0,
                },
            )
        };
        let wire = |offset: u32, source: u32, target: u32, weight: f32| {
            structural(
                offset,
                LocusKind::Edge {
                    source: STRUCTURAL_HOMOLOGY_BASE + source,
                    target: STRUCTURAL_HOMOLOGY_BASE + target,
                    weight,
                    flags: 0,
                    plasticity: crate::genome2::PlasticityGenes::inert(),
                },
            )
        };
        let bind = |offset: u32, node: u32, channel_id: u16| {
            structural(
                offset,
                LocusKind::IoBinding {
                    node: STRUCTURAL_HOMOLOGY_BASE + node,
                    channel_id,
                    gain: 1.0,
                },
            )
        };

        let loci = [
            hidden(10, NodeRole::Input),
            hidden(20, NodeRole::Input),
            hidden(30, NodeRole::Input),
            hidden(40, NodeRole::Output),
            wire(50, 10, 40, big),
            wire(60, 20, 40, small),
            wire(70, 30, 40, small),
            bind(80, 10, 1),
            bind(81, 20, 2),
            bind(82, 30, 3),
            bind(90, 40, 101),
        ];
        let single: Vec<Locus> = loci.to_vec();
        // Interleaved partition: every other locus by position, so neither
        // chromosome is a prefix of the other and the flattened traversal is
        // not ascending.
        let even: Vec<Locus> = loci.iter().step_by(2).copied().collect();
        let odd: Vec<Locus> = loci.iter().skip(1).step_by(2).copied().collect();

        let one_chromosome = genome_with_layout(&[&single]);
        let two_chromosomes = genome_with_layout(&[&even, &odd]);
        let caps = crate::genome2::GenomeCaps::provisional();
        one_chromosome
            .validate_structure(&caps)
            .expect("a single-chromosome layout is valid");
        two_chromosomes
            .validate_structure(&caps)
            .expect("a split layout is valid: loci ascend within each chromosome");
        assert_ne!(
            one_chromosome.chromosome_count(),
            two_chromosomes.chromosome_count(),
            "the two layouts are the same, so this test compares a value with itself"
        );
        assert_ne!(
            one_chromosome, two_chromosomes,
            "the two genomes are identical records, so nothing was permuted"
        );

        let flat = one_chromosome.express_network();
        let split = two_chromosomes.express_network();
        assert_eq!(flat, split, "expression depended on the chromosome split");

        let flat_plan = compile(&flat).expect("compiles");
        let split_plan = compile(&split).expect("compiles");
        assert_eq!(flat_plan, split_plan, "compilation depended on the layout");
        let ids: Vec<u32> = split_plan.incoming[3]
            .iter()
            .map(|edge| edge.homology_id - STRUCTURAL_HOMOLOGY_BASE)
            .collect();
        assert_eq!(
            ids,
            vec![50, 60, 70],
            "the split layout produced a different summation order"
        );

        // ...and the two orders really are numerically different at these
        // magnitudes, so pinning the order above is not about nothing.
        // Homology order adds the large weight first and loses both small
        // ones; any order that accumulates the small ones first does not.
        assert_ne!(
            ((0.0_f32 + big) + small) + small,
            ((0.0_f32 + small) + small) + big,
            "the chosen weights are not order-sensitive, so the order assertion proves nothing"
        );

        // Evaluated activations must agree bit for bit. `state.values` and
        // not the requests: the output binding clamps to [-1, 1], which would
        // hide exactly the difference the weights were chosen to expose.
        let (_, flat_state) = run(&flat, 1.0, 3);
        let (_, split_state) = run(&split, 1.0, 3);
        assert_eq!(
            flat_state
                .values
                .iter()
                .map(|value| value.to_bits())
                .collect::<Vec<u32>>(),
            split_state
                .values
                .iter()
                .map(|value| value.to_bits())
                .collect::<Vec<u32>>()
        );
    }

    #[test]
    fn incoming_edges_are_summed_in_homology_order_not_storage_order() {
        // Float addition is not associative, so this is a real difference
        // and not a formality. Three incoming weights chosen so the two
        // orders give different f32 results are summed in homology order;
        // the compiled list must be in that order regardless of how the
        // edges arrived.
        // Values straddling f32 epsilon (about 1.19e-7). Adding each small
        // weight to 1.0 separately loses both; adding them to each other
        // first does not. An earlier draft used 1e8 and 1.0, which round to
        // the same answer in *both* orders and would have made this test
        // pass without demonstrating anything.
        let big = 1.0_f32;
        let small = 6.0e-8_f32;
        let mut network = ExpressedNetwork {
            nodes: vec![
                node(10, NodeRole::Input, Activation::Linear, 0.0),
                node(20, NodeRole::Input, Activation::Linear, 0.0),
                node(30, NodeRole::Input, Activation::Linear, 0.0),
                node(40, NodeRole::Output, Activation::Linear, 0.0),
            ],
            edges: vec![
                edge(50, 10, 40, big),
                edge(60, 20, 40, small),
                edge(70, 30, 40, small),
            ],
            bindings: vec![
                binding(80, 10, 1, 1.0),
                binding(81, 20, 2, 1.0),
                binding(82, 30, 3, 1.0),
                binding(90, 40, 101, 1.0),
            ],
        };
        let plan = compile(&network).expect("compiles");
        let ids: Vec<u32> = plan.incoming[3].iter().map(|e| e.homology_id).collect();
        assert_eq!(ids, vec![50, 60, 70], "not in homology order");

        // Arriving in a different order must not change the compiled order.
        network.edges.swap(0, 2);
        network.edges.sort_by_key(|edge| edge.homology_id);
        let again = compile(&network).expect("compiles");
        assert_eq!(plan.incoming[3], again.incoming[3]);

        // ...and the sum really is order-sensitive at these magnitudes, so
        // the assertion above is not about nothing. Homology order sums the
        // large weight first and loses both small ones; the reverse order
        // accumulates the small ones into a representable difference.
        let forward = ((0.0_f32 + big) + small) + small;
        let reverse = ((0.0_f32 + small) + small) + big;
        assert_ne!(
            forward, reverse,
            "the chosen weights are not order-sensitive, so pinning the order proves nothing"
        );
    }

    #[test]
    fn a_disabled_edge_contributes_nothing_and_leaves_no_dependency() {
        let mut network = chain();
        network.edges[0].disabled = true;
        let plan = compile(&network).expect("compiles");
        assert_eq!(plan.edge_count(), 1, "a disabled edge was compiled in");
        let (requests, _) = run(&network, 1.0, 3);
        assert_eq!(output_of(&requests, 101), Some(0.0));
    }

    #[test]
    fn unbound_channels_are_never_read() {
        // The registry's promise: an organism binding a subset pays nothing
        // for the rest, and no placeholder value is invented for them.
        let plan = compile(&chain()).expect("compiles");
        let mut state = ActivationState::for_network(plan.node_count());
        let asked = std::cell::RefCell::new(Vec::new());
        let mut requests = ActionRequests::new();
        evaluate(
            &plan,
            &mut state,
            &|channel_id| {
                asked.borrow_mut().push(channel_id);
                0.25
            },
            &mut requests,
        );
        assert_eq!(
            *asked.borrow(),
            vec![1],
            "the evaluator read a channel the organism does not bind"
        );
    }

    #[test]
    fn a_non_finite_input_is_neutralized_and_counted() {
        let plan = compile(&chain()).expect("compiles");
        let mut state = ActivationState::for_network(plan.node_count());
        let mut requests = ActionRequests::new();
        evaluate(&plan, &mut state, &|_| f32::NAN, &mut requests);
        assert_eq!(output_of(&requests, 101), Some(0.0));
        assert_eq!(state.faults, 1, "the fault was not counted");
        assert!(state.values.iter().all(|value| value.is_finite()));
    }

    #[test]
    fn every_activation_stays_bounded_under_extreme_weights() {
        let mut network = chain();
        network.edges = vec![edge(40, 10, 20, VALUE_LIMIT), edge(50, 20, 30, VALUE_LIMIT)];
        network.nodes[1].bias = VALUE_LIMIT;
        network.nodes[2].bias = VALUE_LIMIT;
        let (requests, state) = run(&network, 1.0, 10);
        assert!(
            state
                .values
                .iter()
                .all(|value| (-1.0..=1.0).contains(value))
        );
        for (_, value) in &requests {
            assert!((-1.0..=1.0).contains(value));
        }
    }

    #[test]
    fn two_bindings_to_one_output_channel_accumulate_in_a_pinned_order() {
        let mut network = chain();
        network.bindings.push(binding(71, 20, 101, 0.25));
        let plan = compile(&network).expect("compiles");
        let ids: Vec<u16> = plan
            .output_bindings
            .iter()
            .map(|(channel, _, _)| *channel)
            .collect();
        assert_eq!(ids, vec![101, 101]);
        let (requests, _) = run(&network, 1.0, 1);
        assert_eq!(requests.len(), 1, "the channel was not merged");
        assert_eq!(output_of(&requests, 101), Some(1.0));
    }

    #[test]
    fn the_topological_order_is_canonical_not_merely_valid() {
        // Two nodes with no dependency between them could be ordered either
        // way and both would be valid topological orders. The canonical one
        // is ascending `homology_id`, so the result cannot depend on which
        // the algorithm happened to reach first.
        let network = ExpressedNetwork {
            nodes: vec![
                node(10, NodeRole::Input, Activation::Linear, 0.0),
                node(20, NodeRole::Input, Activation::Linear, 0.0),
                node(30, NodeRole::Output, Activation::Linear, 0.0),
            ],
            edges: vec![edge(40, 10, 30, 1.0), edge(50, 20, 30, 1.0)],
            bindings: vec![binding(60, 30, 101, 1.0)],
        };
        let plan = compile(&network).expect("compiles");
        assert_eq!(plan.order, vec![0, 1, 2]);
    }

    #[test]
    fn activation_state_is_saved_state_and_a_checksum_notices_it() {
        // Prior-state buffers carry a recurrent organism's memory, so a
        // checksum that ignored them would call two genuinely different
        // worlds identical.
        let plan = compile(&chain()).expect("compiles");
        let mut left = ActivationState::for_network(plan.node_count());
        let mut right = ActivationState::for_network(plan.node_count());
        let hash = |state: &ActivationState| {
            let mut hasher = Fnv1a64::new();
            state.hash_into(&mut hasher);
            hasher.finish()
        };
        assert_eq!(hash(&left), hash(&right));
        let _ = once(&plan, &mut left, 0.5);
        commit(&mut left);
        assert_ne!(hash(&left), hash(&right));
        let _ = once(&plan, &mut right, 0.5);
        commit(&mut right);
        assert_eq!(hash(&left), hash(&right));
    }

    #[test]
    fn evaluation_allocates_nothing_after_the_first_tick() {
        // Schema 1's controller evaluated with stack buffers and no per-tick
        // heap allocation, and the Phase 9 plan requires that property to be
        // preserved or its loss explicitly recorded. Preserving it is the
        // better answer, and the check is that no buffer's capacity ever
        // grows once it has been sized: a `Vec` that reallocates is a `Vec`
        // whose capacity changed.
        let plan = compile(&chain()).expect("compiles");
        let mut state = ActivationState::for_network(plan.node_count());
        let mut requests = ActionRequests::with_capacity(plan.output_bindings.len());
        evaluate(&plan, &mut state, &|_| 0.5, &mut requests);
        commit(&mut state);

        let capacities = (
            state.values.capacity(),
            state.prior.capacity(),
            state.gathered.capacity(),
            requests.capacity(),
        );
        for tick in 0..500 {
            evaluate(
                &plan,
                &mut state,
                &|_| (tick % 7) as f32 / 7.0,
                &mut requests,
            );
            commit(&mut state);
        }
        assert_eq!(
            capacities,
            (
                state.values.capacity(),
                state.prior.capacity(),
                state.gathered.capacity(),
                requests.capacity(),
            ),
            "a buffer reallocated during evaluation"
        );
    }

    #[test]
    fn the_scratch_buffer_is_excluded_from_the_checksum() {
        // `gathered` is rebuilt before it is read every tick, so hashing it
        // would make the checksum depend on where in the tick it was taken.
        let plan = compile(&chain()).expect("compiles");
        let mut state = ActivationState::for_network(plan.node_count());
        let hash = |state: &ActivationState| {
            let mut hasher = Fnv1a64::new();
            state.hash_into(&mut hasher);
            hasher.finish()
        };
        let before = hash(&state);
        state.gathered[0] = 0.75;
        assert_eq!(before, hash(&state), "scratch reached the checksum");
    }

    #[test]
    fn commit_is_what_separates_this_tick_from_the_last() {
        // Delayed edges must read the *previous* tick. Evaluating twice
        // without committing must therefore give the same answer, and the
        // difference between the two is the whole content of `commit`.
        // A single delayed edge, so one commit is enough to move the
        // output. A longer all-delayed chain would still read zero at the
        // output after one commit, and the test would compare 0.0 with 0.0.
        let network = ExpressedNetwork {
            nodes: vec![
                node(10, NodeRole::Input, Activation::Linear, 0.0),
                node(20, NodeRole::Output, Activation::Linear, 0.0),
            ],
            edges: vec![delayed(30, 10, 20, 1.0)],
            bindings: vec![binding(40, 10, 1, 1.0), binding(50, 20, 101, 1.0)],
        };
        let plan = compile(&network).expect("compiles");
        let mut state = ActivationState::for_network(plan.node_count());
        let first = once(&plan, &mut state, 1.0);
        let repeat = once(&plan, &mut state, 1.0);
        assert_eq!(first, repeat, "evaluation without commit is not idempotent");
        assert_eq!(output_of(&first, 101), Some(0.0));
        commit(&mut state);
        let after = once(&plan, &mut state, 1.0);
        assert_ne!(
            first, after,
            "commit did not advance the prior-state buffer"
        );
        assert_eq!(output_of(&after, 101), Some(1.0));
    }
}
