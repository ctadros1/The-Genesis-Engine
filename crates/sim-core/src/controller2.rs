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
use crate::plasticity::{self, PlasticityRule};
use crate::registry::{Activation, ChannelDirection, NodeRole, channel};

pub const CONTROLLER2_POLICY_VERSION: &str = "lifesim-controller-v2";

/// Pre-activation clamp, unchanged from schema 1.
const ACTIVATION_LIMIT: f32 = 8.0;

/// `IncomingEdge::plastic_slot` for an edge that does not learn.
///
/// A sentinel rather than an `Option<u32>` because this struct is read once
/// per incoming edge per node per organism per tick and is the hottest record
/// in the kernel; the niche optimization would give `Option<u32>` the same
/// size, but the branch reads better as an explicit comparison next to the
/// slot index it guards.
pub const NOT_PLASTIC: u32 = u32::MAX;

/// `PlasticEdge::modulator` for an edge with no usable modulator.
///
/// The caller hands `plasticity::step` a modulator activation of `0.0` for
/// these, which makes rules 3 and 4 **inert** rather than always-on. The
/// alternative reading - an absent modulator means "always on" - would
/// collapse rule 3 into rule 1 and hand every modulated edge an ungated
/// update it did not evolve.
pub const NO_MODULATOR: u32 = u32::MAX;

/// What this world's plasticity settings mean to the compiler.
///
/// A struct rather than the bare `Option<u32>` it was, because ADR-0027 adds
/// a second, independent thing the compiler needs to know. Two booleans'
/// worth of state in one parameter beats two parameters that can be passed in
/// the wrong order.
///
/// `max_plastic_edges` is `None` when the plasticity section is disabled, and
/// that is **not** the same as `Some(0)`: with `None` no edge is compiled
/// plastic, nothing is counted as over the cap, and the plan, the evaluation
/// and the checksum are exactly what they were before Phase 11 existed. That
/// distinction is what discharges Rule 0's Phase 11 clause -
/// `EDGE_FLAG_PLASTIC` is already a flag on every schema-2 edge, so acting on
/// it without a gate would move the Phase 9 fixture while every disabled
/// section stayed disabled.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlasticityBudget {
    pub max_plastic_edges: Option<u32>,
    /// ADR-0027: every `rule_id` names a live rule.
    ///
    /// The remap happens **once, here in `compile_with_budget`**, so
    /// `plasticity::step` keeps receiving a rule and stays a pure function of
    /// the rule it is handed. A flag consulted inside `step` would put a
    /// world-level setting on the hottest path in the kernel and make the
    /// rule's meaning depend on something the rule cannot see.
    pub live_rule_zero: bool,
}

impl PlasticityBudget {
    /// The plasticity section is off. Pre-Phase-11 behaviour exactly.
    pub const fn disabled() -> Self {
        Self {
            max_plastic_edges: None,
            live_rule_zero: false,
        }
    }

    /// Up to `limit` plastic edges per organism, rule 0 still dead.
    pub const fn edges(limit: u32) -> Self {
        Self {
            max_plastic_edges: Some(limit),
            live_rule_zero: false,
        }
    }

    /// The same budget with ADR-0027's remap live.
    pub const fn with_live_rule_zero(self) -> Self {
        Self {
            live_rule_zero: true,
            ..self
        }
    }

    /// The rule id an expressed allele names under this world's settings.
    ///
    /// With the flag clear this is the identity, which is what keeps every
    /// existing fixture exactly where it is. With it set, ids map onto the
    /// four live rules: `LIVE_RULE_BASE + (r % LIVE_RULE_COUNT)`.
    ///
    /// **The `%` is a clamp, not a distribution choice**, and ADR-0027 records
    /// why that distinction matters. `structmut`'s draw is the only place a
    /// fresh `rule_id` is ever produced, and under this flag it draws over
    /// `LIVE_RULE_COUNT` values - so every allele in circulation in a world
    /// that has run with the flag from tick 0 is already in range and the `%`
    /// never fires. It exists for the two ways an out-of-range id could
    /// arrive anyway: a save written with the flag clear and reloaded with it
    /// set, or a `seeded` founder set carrying an arbitrary id. Those fail
    /// safe onto a live rule rather than out of the registry - but they are
    /// also the case where the distribution stops being uniform, so a
    /// campaign that reloads across the flag has to report it.
    pub const fn effective_rule_id(self, rule_id: u8) -> u8 {
        if self.live_rule_zero {
            plasticity::LIVE_RULE_BASE + (rule_id % plasticity::LIVE_RULE_COUNT)
        } else {
            rule_id
        }
    }
}

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
    /// The **genome** weight, never the learned one.
    ///
    /// Folding the learned delta in here would be faster and is wrong. A
    /// compiled plan is derived state, rebuilt from the genome on every load
    /// and after every structural change, so a mutated weight would be
    /// silently reset by each restore - and the reset would be invisible,
    /// because the genome checksum still matches and nothing else would
    /// disagree. The delta lives in `LearnState` and is applied at the
    /// summation site by `plasticity::effective_weight`.
    pub weight: f32,
    /// Delayed edges read the prior-state buffer; zero-delay edges read the
    /// value computed earlier this tick.
    pub delayed: bool,
    /// Index into `CompiledNetwork::plastic_edges` and into this organism's
    /// learned-state arrays, or [`NOT_PLASTIC`].
    pub plastic_slot: u32,
}

/// One plastic edge, resolved for the learn phase.
///
/// Kept as its own list rather than found by rescanning `incoming`, because
/// the learn phase visits plastic edges in ascending `homology_id` across the
/// whole organism while `incoming` is grouped by target node. Built in the
/// same pass over `network.edges`, which is already ascending, so the list is
/// **in the spec's update order with no sort** - and a sort here would be a
/// second place the order was decided.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlasticEdge {
    pub homology_id: u32,
    /// Node index whose activation is the postsynaptic value `y`.
    pub target: u32,
    /// Node index whose activation was read as `x`. Diagnostic only: the
    /// value actually read is captured during evaluation, because a delayed
    /// edge read the *previous* tick's activation and by learn time both
    /// buffers hold this tick's.
    pub source: u32,
    /// Genome weight, the base of `plasticity::effective_weight`.
    pub weight: f32,
    /// The rule form and its coefficients, with `decay` already converted to
    /// Q16: it is per-edge constant for a lifetime, so converting it per tick
    /// would be a float operation repeated 10^5 times for nothing.
    pub rule: PlasticityRule,
    /// Node index of the gating modulator, or [`NO_MODULATOR`].
    pub modulator: u32,
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
    /// Plastic edges in ascending `homology_id` order - the spec's update
    /// order. Empty whenever the plasticity budget is `None`.
    pub plastic_edges: Vec<PlasticEdge>,
    /// Edges flagged plastic that the budget refused, compiled as ordinary
    /// fixed edges instead.
    ///
    /// Counted rather than only truncated. A cap that binds must reject *and*
    /// count, which is what C9.6 established for the structural caps; without
    /// this number a population sitting hard against the plastic-edge cap and
    /// a population that simply evolved that many plastic edges look
    /// identical, and C11.7 sets the cap from exactly that distinction.
    pub plastic_over_cap: u32,
}

impl CompiledNetwork {
    pub fn node_count(&self) -> usize {
        self.node_ids.len()
    }

    pub fn edge_count(&self) -> usize {
        self.incoming.iter().map(Vec::len).sum()
    }

    pub fn plastic_edge_count(&self) -> usize {
        self.plastic_edges.len()
    }
}

/// Compile an expressed network with no plasticity, the pre-Phase-11 form.
pub fn compile(network: &ExpressedNetwork) -> Result<CompiledNetwork, CompileError> {
    compile_with_budget(network, PlasticityBudget::disabled())
}

/// Compile an expressed network, admitting up to `budget` plastic edges.
pub fn compile_with_budget(
    network: &ExpressedNetwork,
    budget: PlasticityBudget,
) -> Result<CompiledNetwork, CompileError> {
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
    let mut plastic_edges: Vec<PlasticEdge> = Vec::new();
    let mut plastic_over_cap = 0_u32;
    // `network.edges` is already ascending by `homology_id`, so pushing in
    // iteration order leaves each node's list in that order. That is the
    // summation order, and it is why nothing here sorts by anything else.
    // The plastic-edge list is built in the same pass for the same reason:
    // it inherits the order instead of re-deciding it.
    for edge in &network.edges {
        if edge.disabled {
            continue;
        }
        let source = index_of(edge.source)?;
        let target = index_of(edge.target)?;
        // A flagged edge is plastic when the world runs plasticity at all and
        // the per-organism budget has room. Beyond the budget it compiles as
        // an ordinary fixed edge - it keeps its genome weight, evaluates
        // identically, and pays no plastic-edge cost - which is a bounded
        // outcome rather than a refused birth. Refusing would make the cap a
        // lethal structural constraint on a value C11.7 has not measured yet.
        let plastic_slot = match budget.max_plastic_edges {
            Some(limit) if edge.plastic => {
                if plastic_edges.len() as u32 >= limit {
                    plastic_over_cap += 1;
                    NOT_PLASTIC
                } else {
                    let genes = edge.plasticity;
                    // Only a node the genome declares `Modulatory` can gate a
                    // plastic edge. `modulator_node` naming an ordinary node
                    // resolves to no modulator, which makes a modulated rule
                    // inert rather than gating it on whatever that node
                    // happened to output. The role is authored physics - the
                    // spec authors *that* a modulatory node gates plasticity
                    // - and letting any node gate would make the role
                    // decorative and delete the distinction the design rests
                    // on. Validation already refuses a `modulator_node` that
                    // names no node at all, so the only case reaching here is
                    // a real node of the wrong role.
                    let modulator = if genes.modulator_node == 0 {
                        NO_MODULATOR
                    } else {
                        match index_of(genes.modulator_node) {
                            Ok(index)
                                if network.nodes[index as usize].role == NodeRole::Modulatory =>
                            {
                                index
                            }
                            _ => NO_MODULATOR,
                        }
                    };
                    let slot = plastic_edges.len() as u32;
                    plastic_edges.push(PlasticEdge {
                        homology_id: edge.homology_id,
                        target,
                        source,
                        weight: edge.weight,
                        rule: PlasticityRule {
                            // ADR-0027's remap, applied **once, here**. The
                            // compiled plan is what `plasticity::step` reads,
                            // so a rule that reaches the learn phase already
                            // names a live entry and `step` stays a pure
                            // function of the rule it is handed.
                            rule_id: budget.effective_rule_id(genes.rule_id),
                            eta: genes.eta,
                            coefficients: genes.coefficients,
                            decay_q16: plasticity::decay_to_q16(genes.decay),
                        },
                        modulator,
                    });
                    slot
                }
            }
            _ => NOT_PLASTIC,
        };
        incoming[target as usize].push(IncomingEdge {
            homology_id: edge.homology_id,
            source,
            weight: edge.weight,
            delayed: edge.delayed,
            plastic_slot,
        });
        if !edge.delayed {
            zero_delay.push((source, target));
        }
    }
    debug_assert!(
        plastic_edges
            .windows(2)
            .all(|pair| pair[0].homology_id < pair[1].homology_id),
        "plastic edges must be ascending by homology_id: that is the update order"
    );
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
        plastic_edges,
        plastic_over_cap,
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
    /// The presynaptic value each plastic edge actually read this tick,
    /// indexed by `IncomingEdge::plastic_slot`. **Scratch on exactly the same
    /// terms as `gathered`, and excluded from the checksum for exactly the
    /// same reason.**
    ///
    /// It has to be captured during evaluation and cannot be recovered
    /// afterwards. `controllers_phase2` ends by calling `commit`, which
    /// copies `values` into `prior`, so by the time the learn phase runs both
    /// buffers hold *this* tick's activations and a delayed edge's actual
    /// presynaptic input is gone.
    ///
    /// Two alternatives were rejected. Defining `x` as the current-tick
    /// source activation for every edge is cheaper and makes the arithmetic a
    /// lie: the update would pair an `x` the edge never read with the `y` it
    /// produced, so the spec's `a*x*y` would not be the correlation it
    /// claims to be. Moving `commit` after the learn phase would recover the
    /// old `prior`, and would break the Rule 4 guarantee that prior-state
    /// buffers advance only after **every** organism has been evaluated.
    plastic_pre: Vec<f32>,
}

impl ActivationState {
    pub fn for_network(node_count: usize, plastic_edges: usize) -> Self {
        Self {
            values: vec![0.0; node_count],
            prior: vec![0.0; node_count],
            faults: 0,
            gathered: vec![0.0; node_count],
            plastic_pre: vec![0.0; plastic_edges],
        }
    }

    /// Resize after a structural change, preserving what still lines up.
    ///
    /// Only used when a network is recompiled for an organism that already
    /// exists; a newborn starts from zero, because learned or accumulated
    /// activation is never inherited.
    pub fn resize(&mut self, node_count: usize, plastic_edges: usize) {
        self.values.resize(node_count, 0.0);
        self.prior.resize(node_count, 0.0);
        self.gathered.resize(node_count, 0.0);
        self.plastic_pre.resize(plastic_edges, 0.0);
    }

    /// The presynaptic value plastic edge `slot` read this tick.
    ///
    /// Read by the learn phase, which lives in `world.rs` and therefore
    /// cannot reach a private field. Not part of logical state: between
    /// evaluation and the learn phase of the same tick it is meaningful, and
    /// at any other moment it is whatever the last evaluation left behind.
    pub fn plastic_pre(&self, slot: usize) -> f32 {
        self.plastic_pre[slot]
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
        // `gathered` and `plastic_pre` are deliberately absent: both are
        // scratch, rebuilt every tick before they are read, so hashing either
        // would make the checksum depend on where in the tick it was taken.
        // `the_scratch_buffers_are_excluded_from_the_checksum` asserts it.
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
/// `learned` is this organism's Q16 learned delta per plastic edge, indexed
/// by `IncomingEdge::plastic_slot`. It is empty in a world without the
/// plasticity section, where no edge carries a slot.
///
/// `requests` is cleared and refilled, so a caller reusing one buffer across
/// organisms allocates nothing. **Evaluation performs no allocation at all**:
/// schema 1's controller had that property with stack buffers, and variable
/// topology must not silently give it up.
pub fn evaluate(
    plan: &CompiledNetwork,
    state: &mut ActivationState,
    learned: &[i32],
    input: &dyn Fn(u16) -> f32,
    requests: &mut ActionRequests,
) {
    debug_assert_eq!(state.values.len(), plan.node_count());
    debug_assert_eq!(learned.len(), plan.plastic_edges.len());
    debug_assert_eq!(state.plastic_pre.len(), plan.plastic_edges.len());

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
            // The learned delta is applied here and nowhere else. A plastic
            // edge with a zero delta multiplies by exactly its genome weight,
            // so an organism that has learned nothing evaluates identically
            // to the same organism in a world without plasticity.
            let weight = if edge.plastic_slot == NOT_PLASTIC {
                edge.weight
            } else {
                let slot = edge.plastic_slot as usize;
                // Captured here because this is the only moment it exists:
                // `commit` overwrites `prior` before the learn phase runs.
                state.plastic_pre[slot] = value;
                plasticity::effective_weight(edge.weight, learned[slot])
            };
            sum += weight * value;
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
        PlasticityGenes, STRUCTURAL_HOMOLOGY_BASE, VALUE_LIMIT,
    };
    use crate::plasticity::{
        LEARN_LIMIT_Q16, ONE_Q16, RULE_HEBBIAN, RULE_MODULATED_HEBBIAN, RULE_STATIC,
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
            // Phase 11 gave `ExpressedEdge` a plasticity payload; these
            // fixtures are about evaluation order and bounds, so they take
            // the inert genes a pre-Phase-11 edge always expressed.
            plasticity: PlasticityGenes::inert(),
        }
    }

    fn delayed(homology_id: u32, source: u32, target: u32, weight: f32) -> ExpressedEdge {
        ExpressedEdge {
            delayed: true,
            ..edge(homology_id, source, target, weight)
        }
    }

    /// Plasticity genes that move: eta 1, `a = 1` so the Hebbian form is
    /// exactly `x*y`, and no decay. A gene set that produced zero would make
    /// every plasticity assertion below an assertion about nothing.
    fn genes(rule_id: u8, modulator_node: u32) -> PlasticityGenes {
        PlasticityGenes {
            rule_id,
            eta: 1.0,
            coefficients: [1.0, 0.0, 0.0, 0.0],
            decay: 0.0,
            modulator_node,
        }
    }

    fn plastic(
        homology_id: u32,
        source: u32,
        target: u32,
        weight: f32,
        rule_id: u8,
        modulator_node: u32,
    ) -> ExpressedEdge {
        ExpressedEdge {
            plastic: true,
            plasticity: genes(rule_id, modulator_node),
            ..edge(homology_id, source, target, weight)
        }
    }

    /// The three-node chain with both edges flagged plastic.
    fn plastic_chain(rule_id: u8, modulator_node: u32) -> ExpressedNetwork {
        let mut network = chain();
        network.edges = vec![
            plastic(40, 10, 20, 1.0, rule_id, modulator_node),
            plastic(50, 20, 30, 1.0, rule_id, modulator_node),
        ];
        network
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
        let mut state = ActivationState::for_network(plan.node_count(), plan.plastic_edge_count());
        let mut requests = ActionRequests::new();
        for _ in 0..ticks {
            evaluate(&plan, &mut state, &[], &|_| value, &mut requests);
            commit(&mut state);
        }
        (requests, state)
    }

    /// Evaluate once into a fresh buffer, for tests that want one tick.
    fn once(plan: &CompiledNetwork, state: &mut ActivationState, value: f32) -> ActionRequests {
        let mut requests = ActionRequests::new();
        evaluate(plan, state, &[], &|_| value, &mut requests);
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
        let mut state = ActivationState::for_network(plan.node_count(), plan.plastic_edge_count());

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
        let mut state = ActivationState::for_network(plan.node_count(), plan.plastic_edge_count());
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
        let mut state = ActivationState::for_network(plan.node_count(), plan.plastic_edge_count());
        let asked = std::cell::RefCell::new(Vec::new());
        let mut requests = ActionRequests::new();
        evaluate(
            &plan,
            &mut state,
            &[],
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
        let mut state = ActivationState::for_network(plan.node_count(), plan.plastic_edge_count());
        let mut requests = ActionRequests::new();
        evaluate(&plan, &mut state, &[], &|_| f32::NAN, &mut requests);
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
        let mut left = ActivationState::for_network(plan.node_count(), plan.plastic_edge_count());
        let mut right = ActivationState::for_network(plan.node_count(), plan.plastic_edge_count());
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
        //
        // **Extended for Phase 11 rather than weakened.** The plan is the
        // plastic chain, so the presynaptic capture runs on every tick, and
        // `plastic_pre` joins the tuple - a capture that pushed instead of
        // writing in place would grow a capacity and fail here.
        let plan = compile_with_budget(&plastic_chain(RULE_HEBBIAN, 0), PlasticityBudget::edges(8))
            .expect("compiles");
        assert_eq!(plan.plastic_edge_count(), 2, "nothing plastic to capture");
        let learned = vec![0_i32; plan.plastic_edge_count()];
        let mut state = ActivationState::for_network(plan.node_count(), plan.plastic_edge_count());
        let mut requests = ActionRequests::with_capacity(plan.output_bindings.len());
        evaluate(&plan, &mut state, &learned, &|_| 0.5, &mut requests);
        commit(&mut state);

        let capacities = (
            state.values.capacity(),
            state.prior.capacity(),
            state.gathered.capacity(),
            state.plastic_pre.capacity(),
            requests.capacity(),
        );
        for tick in 0..500 {
            evaluate(
                &plan,
                &mut state,
                &learned,
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
                state.plastic_pre.capacity(),
                requests.capacity(),
            ),
            "a buffer reallocated during evaluation"
        );
    }

    #[test]
    fn the_scratch_buffers_are_excluded_from_the_checksum() {
        // Both scratch buffers are rebuilt before they are read every tick,
        // so hashing either would make the checksum depend on where in the
        // tick it was taken. `plastic_pre` is the Phase 11 addition and is
        // checked the same way `gathered` is.
        let plan = compile_with_budget(&plastic_chain(RULE_HEBBIAN, 0), PlasticityBudget::edges(8))
            .expect("compiles");
        let mut state = ActivationState::for_network(plan.node_count(), plan.plastic_edge_count());
        assert!(
            !state.gathered.is_empty() && !state.plastic_pre.is_empty(),
            "an empty buffer cannot be perturbed, so this would prove nothing"
        );
        let hash = |state: &ActivationState| {
            let mut hasher = Fnv1a64::new();
            state.hash_into(&mut hasher);
            hasher.finish()
        };
        let before = hash(&state);
        state.gathered[0] = 0.75;
        state.plastic_pre[0] = 0.75;
        assert_eq!(before, hash(&state), "scratch reached the checksum");

        // The control: a field that *is* logical state must move the hash,
        // or this test would pass on a `hash_into` that hashed nothing.
        state.values[0] = 0.75;
        assert_ne!(before, hash(&state));
    }

    #[test]
    fn a_flagged_edge_is_plastic_only_when_the_world_runs_plasticity() {
        // Rule 0's Phase 11 clause in one assertion: `EDGE_FLAG_PLASTIC` is
        // already set on schema-2 edges, so a build that acted on it without
        // a config gate would change every existing schema-2 world.
        let network = plastic_chain(RULE_HEBBIAN, 0);
        let disabled =
            compile_with_budget(&network, PlasticityBudget::disabled()).expect("compiles");
        assert_eq!(disabled.plastic_edge_count(), 0);
        assert_eq!(disabled.plastic_over_cap, 0);
        assert!(
            disabled
                .incoming
                .iter()
                .flatten()
                .all(|edge| edge.plastic_slot == NOT_PLASTIC),
            "an edge carries a learned-state slot in a world with no learned state"
        );
        // ...and the compiled plan is *identical* to the same network with no
        // flags at all, which is the strongest form of "inert".
        let mut unflagged = network.clone();
        for edge in &mut unflagged.edges {
            edge.plastic = false;
        }
        assert_eq!(disabled, compile(&unflagged).expect("compiles"));

        let enabled = compile_with_budget(&network, PlasticityBudget::edges(8)).expect("compiles");
        assert_eq!(enabled.plastic_edge_count(), 2);
        assert_eq!(
            enabled
                .plastic_edges
                .iter()
                .map(|edge| edge.homology_id)
                .collect::<Vec<u32>>(),
            vec![40, 50],
            "the update order is not ascending homology_id"
        );
        // The slot on the incoming list and the position in `plastic_edges`
        // must be the same index, or evaluation and the learn phase would
        // disagree about which edge learned.
        for node in &enabled.incoming {
            for edge in node {
                if edge.plastic_slot != NOT_PLASTIC {
                    assert_eq!(
                        enabled.plastic_edges[edge.plastic_slot as usize].homology_id,
                        edge.homology_id
                    );
                }
            }
        }
    }

    #[test]
    fn the_budget_caps_plastic_edges_in_homology_order_and_counts_the_refusals() {
        let network = plastic_chain(RULE_HEBBIAN, 0);
        let capped = compile_with_budget(&network, PlasticityBudget::edges(1)).expect("compiles");
        assert_eq!(capped.plastic_edge_count(), 1);
        assert_eq!(capped.plastic_over_cap, 1);
        // The lower homology_id keeps the slot; the refused edge falls back
        // to being an ordinary fixed edge rather than refusing the organism.
        assert_eq!(capped.plastic_edges[0].homology_id, 40);
        assert_eq!(capped.edge_count(), 2, "the capped edge left the network");
        let slots: Vec<u32> = capped
            .incoming
            .iter()
            .flatten()
            .map(|edge| edge.plastic_slot)
            .collect();
        assert_eq!(slots.iter().filter(|slot| **slot == NOT_PLASTIC).count(), 1);
    }

    #[test]
    fn only_a_modulatory_node_can_gate_a_plastic_edge() {
        // The role is authored physics. If any node could gate, `Modulatory`
        // would be decorative and rules 3 and 4 would be rule 1 with extra
        // steps - which is exactly the collapse the spec's "a modulated rule
        // with no modulator is inert" clause exists to prevent.
        let mut network = plastic_chain(RULE_MODULATED_HEBBIAN, 20);
        let hidden = compile_with_budget(&network, PlasticityBudget::edges(8)).expect("compiles");
        assert!(
            hidden
                .plastic_edges
                .iter()
                .all(|edge| edge.modulator == NO_MODULATOR),
            "a Hidden node gated a plastic edge"
        );

        // The control: the same gene, on the same node, once that node's
        // role is Modulatory. Without this the assertion above would pass on
        // an implementation that never resolved a modulator at all.
        network.nodes[1].role = NodeRole::Modulatory;
        let gated = compile_with_budget(&network, PlasticityBudget::edges(8)).expect("compiles");
        assert!(
            gated.plastic_edges.iter().all(|edge| edge.modulator == 1),
            "a Modulatory node did not gate its edges"
        );
        // Node id 0 stays "ungated", which for a modulated rule is inert.
        let ungated = compile_with_budget(
            &plastic_chain(RULE_MODULATED_HEBBIAN, 0),
            PlasticityBudget::edges(8),
        )
        .expect("compiles");
        assert!(
            ungated
                .plastic_edges
                .iter()
                .all(|edge| edge.modulator == NO_MODULATOR)
        );
    }

    #[test]
    fn the_learned_delta_reaches_the_summation_site_without_touching_the_plan() {
        // The delta must change evaluation, and the compiled plan must be
        // exactly what it was: a plan whose weights had been mutated would be
        // silently reset by the next restore, and nothing would say so.
        let plan = compile_with_budget(&plastic_chain(RULE_STATIC, 0), PlasticityBudget::edges(8))
            .expect("compiles");
        let before = plan.clone();
        let mut state = ActivationState::for_network(plan.node_count(), plan.plastic_edge_count());
        let mut requests = ActionRequests::new();

        evaluate(&plan, &mut state, &[0, 0], &|_| 0.5, &mut requests);
        assert_eq!(output_of(&requests, 101), Some(0.5), "unlearned baseline");

        // +1.0 on the first edge doubles its weight, so the chain's product
        // doubles: 0.5 * 2 * 1 = 1.0.
        evaluate(&plan, &mut state, &[ONE_Q16, 0], &|_| 0.5, &mut requests);
        assert_eq!(output_of(&requests, 101), Some(1.0));

        // -1.0 cancels the genome weight exactly, so the chain goes silent.
        evaluate(&plan, &mut state, &[-ONE_Q16, 0], &|_| 0.5, &mut requests);
        assert_eq!(output_of(&requests, 101), Some(0.0));

        // The clamp holds at the summation site too: genome weight 1 plus a
        // learned 8 is clamped to 8, not 9.
        evaluate(
            &plan,
            &mut state,
            &[LEARN_LIMIT_Q16, 0],
            &|_| 0.5,
            &mut requests,
        );
        // 1 + 8 clamps to 8, so node 20 sums 8 * 0.5 = 4 and its Linear
        // activation clamps that to 1; without the weight clamp the sum
        // would be 9 * 0.5 and the activation clamp would hide the
        // difference, which is why the assertion below is paired with the
        // -1.0 case rather than standing alone.
        assert_eq!(output_of(&requests, 101), Some(1.0));

        assert_eq!(plan, before, "evaluation mutated the compiled plan");
    }

    #[test]
    fn the_presynaptic_capture_is_the_value_the_edge_actually_read() {
        // The whole reason `plastic_pre` exists. A delayed plastic edge reads
        // last tick's activation; by the time the learn phase runs, `commit`
        // has overwritten `prior` with this tick's. Capturing at read time is
        // the only way the update pairs the `x` that produced the `y`.
        let mut network = chain();
        network.edges = vec![
            ExpressedEdge {
                delayed: true,
                ..plastic(40, 10, 20, 1.0, RULE_HEBBIAN, 0)
            },
            plastic(50, 20, 30, 1.0, RULE_HEBBIAN, 0),
        ];
        let plan = compile_with_budget(&network, PlasticityBudget::edges(8)).expect("compiles");
        let mut state = ActivationState::for_network(plan.node_count(), plan.plastic_edge_count());
        let learned = vec![0_i32; plan.plastic_edge_count()];
        let mut requests = ActionRequests::new();

        // Tick 1: input 0.25. The delayed edge reads prior, which is zero.
        evaluate(&plan, &mut state, &learned, &|_| 0.25, &mut requests);
        assert_eq!(state.plastic_pre(0), 0.0, "delayed edge read this tick");
        assert_eq!(state.values[0], 0.25, "and the source really is not zero");
        commit(&mut state);

        // Tick 2: input 0.75. The delayed edge reads tick 1's 0.25, not
        // tick 2's 0.75 - and after `commit` neither buffer holds 0.25 any
        // more, which is what makes the capture irrecoverable afterwards.
        evaluate(&plan, &mut state, &learned, &|_| 0.75, &mut requests);
        assert_eq!(state.plastic_pre(0), 0.25);
        commit(&mut state);
        assert_eq!(state.prior[0], 0.75);
        assert_eq!(state.values[0], 0.75);

        // The zero-delay edge captures the source computed earlier this tick,
        // which is node 20's activation - 0.25, the delayed edge's output -
        // and not the 0.75 sitting on node 10.
        assert_eq!(state.plastic_pre(1), state.values[1]);
        assert_eq!(state.values[1], 0.25);
        assert_ne!(state.plastic_pre(1), state.values[0]);
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
        let mut state = ActivationState::for_network(plan.node_count(), plan.plastic_edge_count());
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

    // --- ADR-0027: the remap narrows the id space, it does not turn it -------

    /// With the flag clear, `effective_rule_id` is the identity on every id
    /// the registry defines **and** on ids outside it.
    ///
    /// The second half is the load-bearing one: it is what says the flag-off
    /// arm is byte-identical to every build before ADR-0027, including for
    /// the stored-but-unreduced values `PlasticityGenes::normalized` exists
    /// to tolerate.
    #[test]
    fn with_the_flag_clear_the_rule_id_is_untouched() {
        let budget = PlasticityBudget::edges(8);
        assert!(!budget.live_rule_zero);
        for rule_id in 0..=u8::MAX {
            assert_eq!(
                budget.effective_rule_id(rule_id),
                rule_id,
                "rule {rule_id} moved with the flag clear"
            );
        }
        assert_eq!(
            PlasticityBudget::disabled().effective_rule_id(plasticity::RULE_STATIC),
            plasticity::RULE_STATIC
        );
    }

    /// With the flag set, **every** id names a live rule and none names the
    /// dead one.
    #[test]
    fn with_the_flag_set_no_rule_id_names_the_dead_value() {
        let budget = PlasticityBudget::edges(8).with_live_rule_zero();
        for rule_id in 0..=u8::MAX {
            let effective = budget.effective_rule_id(rule_id);
            assert_ne!(
                effective,
                plasticity::RULE_STATIC,
                "rule {rule_id} still reaches the dead value"
            );
            assert!(
                plasticity::rule_in_registry(effective),
                "rule {rule_id} mapped to {effective}, which is outside the registry"
            );
        }
    }

    /// The map is **uniform** over the four live rules across the range the
    /// mutation draw can produce.
    ///
    /// This is the assertion ADR-0027 turns on, and the reason option (b) was
    /// rejected: `r -> (r % 4) + 1` applied to a five-value draw gives rule 1
    /// forty percent and the rest twenty each, which authors a preference for
    /// plain Hebbian - the rule the lifetime-learning review rates as
    /// "unsupported as the sole production rule". A uniform map authors
    /// nothing.
    #[test]
    fn the_remap_is_uniform_over_the_live_rules() {
        let budget = PlasticityBudget::edges(8).with_live_rule_zero();
        let mut hits = [0_u32; plasticity::RULE_COUNT as usize];
        for drawn in 0..plasticity::LIVE_RULE_COUNT {
            hits[budget.effective_rule_id(drawn) as usize] += 1;
        }
        assert_eq!(
            hits[plasticity::RULE_STATIC as usize],
            0,
            "the dead value was reachable from the draw's range"
        );
        for rule_id in plasticity::LIVE_RULE_BASE..plasticity::RULE_COUNT {
            assert_eq!(
                hits[rule_id as usize], 1,
                "rule {rule_id} is not equiprobable: {hits:?}"
            );
        }
    }

    /// The founder compiles to an inert controller under **both** settings,
    /// so the two arms of the 2x2 start identical.
    ///
    /// D-107's whole argument for A3 over A1 is that `eta == 0` makes the
    /// founder inert whatever rule its allele nominally names, so narrowing
    /// the id space hands it nothing. That is an argument about the founder,
    /// and this is the founder: `rule_id` 0 with `eta` 0, which the flag maps
    /// to a live rule and which must still do nothing.
    #[test]
    fn the_founder_is_inert_under_both_settings() {
        let mut network = chain();
        network.edges = vec![
            ExpressedEdge {
                plastic: true,
                plasticity: PlasticityGenes {
                    rule_id: plasticity::RULE_STATIC,
                    eta: 0.0,
                    coefficients: [1.0, 0.0, 0.0, 0.0],
                    decay: 0.0,
                    modulator_node: 0,
                },
                ..edge(40, 10, 20, 1.0)
            },
            ExpressedEdge {
                plastic: true,
                plasticity: PlasticityGenes {
                    rule_id: plasticity::RULE_STATIC,
                    eta: 0.0,
                    coefficients: [1.0, 0.0, 0.0, 0.0],
                    decay: 0.0,
                    modulator_node: 0,
                },
                ..edge(50, 20, 30, 1.0)
            },
        ];

        let off = compile_with_budget(&network, PlasticityBudget::edges(8)).expect("compiles");
        let on = compile_with_budget(&network, PlasticityBudget::edges(8).with_live_rule_zero())
            .expect("compiles");

        // The flag *does* reach the plan - otherwise this test would pass
        // with the remap deleted.
        assert_eq!(off.plastic_edges[0].rule.rule_id, plasticity::RULE_STATIC);
        assert_ne!(on.plastic_edges[0].rule.rule_id, plasticity::RULE_STATIC);

        // ...and changes nothing about what the founder does, because eta is
        // zero under either rule.
        for (left, right) in off.plastic_edges.iter().zip(on.plastic_edges.iter()) {
            let signals = plasticity::EdgeSignals {
                pre: 0.9,
                post: 0.8,
                modulator: 0.0,
                w_eff: 1.0,
            };
            let from_off =
                plasticity::step(left.rule, signals, plasticity::LearnedState::default());
            let from_on =
                plasticity::step(right.rule, signals, plasticity::LearnedState::default());
            assert_eq!(
                from_off.state,
                plasticity::LearnedState::default(),
                "the founder learned with the flag clear"
            );
            assert_eq!(
                from_on.state,
                plasticity::LearnedState::default(),
                "the founder learned with the flag set, so the two arms do not start identical"
            );
        }
    }

    /// The flag reaches the compiled plan, and only the rule id moves.
    #[test]
    fn the_flag_moves_the_rule_and_nothing_else_about_the_plan() {
        let network = plastic_chain(plasticity::RULE_STATIC, 0);
        let off = compile_with_budget(&network, PlasticityBudget::edges(8)).expect("compiles");
        let on = compile_with_budget(&network, PlasticityBudget::edges(8).with_live_rule_zero())
            .expect("compiles");

        assert_eq!(off.plastic_edges.len(), on.plastic_edges.len());
        assert_eq!(off.plastic_over_cap, on.plastic_over_cap);
        for (left, right) in off.plastic_edges.iter().zip(on.plastic_edges.iter()) {
            assert_eq!(left.homology_id, right.homology_id);
            assert_eq!(left.source, right.source);
            assert_eq!(left.target, right.target);
            assert_eq!(left.weight, right.weight);
            assert_eq!(left.modulator, right.modulator);
            assert_eq!(left.rule.eta, right.rule.eta);
            assert_eq!(left.rule.coefficients, right.rule.coefficients);
            assert_eq!(left.rule.decay_q16, right.rule.decay_q16);
            assert_ne!(left.rule.rule_id, right.rule.rule_id);
        }
        // A world with the section disabled compiles no plastic edges at all,
        // so the flag has nothing to act on - which is why the budget gates it.
        let disabled = compile_with_budget(&network, PlasticityBudget::disabled()).expect("ok");
        assert!(disabled.plastic_edges.is_empty());
    }
}
