# The Genesis Engine: Neuroevolution and Lifetime Learning

**Engineering-oriented scientific review**  
**Prepared:** 2026-08-04  
**Scope:** structurally evolvable neural controllers, lifetime plasticity, gene–learning interaction, deterministic mutation/evaluation, exact persistence, and staged validation for a long-running artificial-life system.

---

## Evidence labels used in this report

Every material recommendation is tagged with one of four labels:

- **Strongly supported** — supported by canonical theory, multiple empirical lines, or direct engineering necessity under the Genesis determinism contract.
- **Supported in narrower conditions** — demonstrated in particular task classes or dependent on identifiable assumptions; it should not be generalized without a Genesis-specific test.
- **Plausible but unproven** — technically coherent and grounded in prior work, but not established for thousands of persistent artificial organisms in an open-ended world.
- **Speculative** — a research direction whose expected benefit is uncertain or whose implementation cost is currently disproportionate to the evidence.

The labels describe the strength of the recommendation **for Genesis**, not the general scientific merit of the cited work.

---

# 1. Executive summary

## 1.1 Recommended controller strategy

The recommended first successor to the current fixed-topology controller is:

> **A typed, recurrent, directly encoded graph genotype with NEAT-like historical alignment, deterministic content-addressed structural identities, bounded add/disable/delete mutations, explicit one-tick or integer-delay recurrence, and a versioned family of local neuromodulated plasticity rules whose full learned state is checkpointed.**

This is a deliberately conservative hybrid. It adopts the strongest ideas from NEAT—historical alignment, protection of structural innovations, and incremental structural change—without treating every implementation detail of the 2002 algorithm as mandatory. It also adds capabilities that the original NEAT design did not prioritize: deletion, hard complexity budgets, stable parallel mutation identity, exact learned-state persistence, explicit numeric semantics, and fail-closed graph validation. [R1–R3]

**Recommendation strength: Strongly supported.** Direct graph encodings are the most mature route to variable topology and are substantially easier to serialize, inspect, validate, replay, and cross over than developmental encodings. The particular Genesis identity and deterministic-event design is an engineering synthesis rather than a published algorithm, but it follows directly from the engine’s determinism requirements.

## 1.2 What should be inherited versus learned

The initial production policy should be **Darwinian at the weight-state boundary**:

- The genome inherits topology, initial/base weights, neuron parameters, plasticity masks, learning-rule identifiers, plasticity coefficients, neuromodulator sensitivities, trace constants, and bounds.
- The organism’s acquired weight deltas, eligibility traces, neuron activations, recurrent delay buffers, and modulatory state persist through that organism’s save/restore cycle but are **not copied into offspring**.
- A later experimental policy may support partial or full Lamarckian inheritance, but it must be an explicit, versioned reproduction mode with separate lineage semantics and direct controls. [R51–R58]

**Recommendation strength: Strongly supported for the initial policy.** This cleanly separates evolutionary adaptation from lifetime adaptation, makes experiments interpretable, and avoids allowing transient experience to overwrite the inherited prior by default. Lamarckian algorithms can be useful in stable tasks, but their superiority is not general and can reverse in changing environments. [R57–R58]

## 1.3 Preferred lifetime-learning rule

The best initial learning family is a **bounded local three-factor rule**:

1. a local pre/post activity term, such as a Hebbian covariance or Oja-normalized term;
2. a decaying eligibility trace that records recent local correlation;
3. a modulatory gate derived from reward, internal homeostatic state, novelty, pain/damage, social outcome, or another explicitly represented signal.

A generic form is:

\[
 e_{ij}(t+1)=\lambda_{ij}e_{ij}(t)+F_{r_{ij}}(x_i(t),y_j(t),w_{ij}(t),\Delta x_i,\Delta y_j)
\]

\[
 \Delta w_{ij}(t)=\operatorname{clip}\left(\eta_{ij}\,m_{c(i,j)}(t)\,e_{ij}(t)-d_{ij}\,\delta w_{ij}(t),\; -\Delta_{\max},\Delta_{\max}\right)
\]

\[
 w_{ij}^{\text{effective}}(t)=\operatorname{clip}\left(w_{ij}^{\text{base}}+\delta w_{ij}(t),\;w_{ij}^{\min},w_{ij}^{\max}\right).
\]

Here, the genome selects a rule from a small versioned palette and encodes its coefficients; it does not initially evolve arbitrary executable code. This provides local online learning, temporal credit assignment, compact genetic control, deterministic execution, and exact checkpointability without requiring backpropagation. Hebbian, Oja, STDP, reward-modulated STDP, neuromodulated neuroevolution, and differentiable-plasticity work collectively support the ingredients, while no single paper establishes this exact Genesis combination. [R34–R45]

**Recommendation strength: Supported in narrower conditions.** Three-factor rules are scientifically well motivated and have worked in reward-based adaptation, but the right signal definitions and coefficient sharing pattern are task-dependent. Their benefit must be established in Genesis environments rather than assumed.

## 1.4 Why not begin with HyperNEAT or a developmental program

HyperNEAT and CPPN encodings are valuable when the task contains stable geometric regularities—symmetry, repetition, locality, or scale-independent patterns—and when substrate coordinates encode real structure. They have outperformed direct encodings on deliberately regular problems and can compactly generate large phenotypes. However, their inductive bias can be harmful when the useful connectivity is irregular; mutations can alter many expressed weights at once; crossover acts on the generator rather than the expressed circuit; and lifetime plasticity still requires storing the expressed synaptic state. [R7–R11]

Developmental programs, cellular encodings, graph grammars, neural cellular automata, and recent Neural Developmental Programs offer stronger reuse and potentially richer modularity, but they introduce difficult genotype–phenotype credit assignment, developmental validation, symmetry-collapse, and replay-policy problems. Recent NDP work is promising but still early relative to direct encodings and often uses computational machinery or training methods poorly matched to a many-organism deterministic simulator. [R12–R22]

**Recommendation strength: Strongly supported for sequencing, not rejection.** Implement direct graph evolution first. Treat HyperNEAT/CPPN and developmental controllers as controlled research branches after the direct representation, plastic-state format, and replay harness are proven.

## 1.5 Main conclusions about NEAT

NEAT is important, but the evidence does **not** justify the claim that variable-topology NEAT generally outperforms fixed networks.

- The original paper showed strong gains on a challenging double-pole balancing reinforcement-learning benchmark and used ablations to show that historical crossover alignment, speciation, and minimal-start complexification worked as an interdependent system. [R1]
- Later literature contains many successful descendants and applications, but methods, tasks, baselines, and reporting quality vary substantially. A systematic review identified dozens of successors rather than a single universally dominant formulation. [R2]
- Variable topology is most likely to help when appropriate network size and recurrence are unknown, structural stepping stones matter, and new structure receives enough protection to tune its weights.
- Fixed topology remains competitive or superior when a good architecture is already known, the relevant problem is predominantly weight optimization, vectorized throughput dominates, or the task requires a large regular substrate better handled by an indirect encoding.

**Recommendation strength: Strongly supported.** Use NEAT as a design source, not as an unquestioned reference implementation.

## 1.6 Structural identity is not one number

Genesis should not overload a single “innovation number” with four different meanings. Store separately:

- **gene lineage ID** — the persistent identity of a particular heritable gene lineage;
- **homology class ID** — the structural slot used to align genes during crossover;
- **structural signature** — canonical phenotype-relevant fields such as source, destination, delay, and type;
- **mutation event ID** — the deterministic identity of the reproduction/mutation event that created the gene.

Original NEAT assigns chronological historical markers and uses an innovation record so equivalent structural mutations arising within the tracked window can receive the same marker. A global sequential counter is unsafe for parallel Genesis worlds because assignment can depend on scheduling. Instead, IDs should be derived by a domain-separated hash or deterministic bijection over a canonical event key containing policy version, parent lineage information, operator, target homology, offspring event identity, and attempt ordinal. [R1]

**Recommendation strength: Strongly supported as an engineering requirement.** The exact field layout is new, but separating ancestry, alignment, phenotype, and event identity prevents false equivalence and removes order-dependent global allocation.

## 1.7 Removal, bloat, and bounds

Removal must exist from the first variable-topology version. A system that only adds nodes and connections accumulates historical debris, raises per-tick cost, and eventually makes the controller budget a function of lineage age rather than adaptive value.

Use all of the following:

- reversible enable/disable mutations;
- low-rate connection deletion and conservative node-pruning mutations;
- hard maximum node, edge, plastic-edge, delay, and module budgets;
- an explicit controller evaluation or metabolic cost;
- optional Pareto/lexicographic parsimony pressure;
- canonical pruning of unreachable executable phenotype state while preserving genotype history as policy allows.

Deletion-first strategies such as EPNet and research on connection-cost pressure support the value of simplification, while modularity studies show that cost pressure can alter evolved organization. [R6, R24, R26]

**Recommendation strength: Strongly supported.** The exact mutation rates are not known a priori and require Genesis-specific sweeps.

## 1.8 Determinism and persistence are part of controller semantics

A controller is not defined only by its genome. Its complete replay-relevant state includes the compiled graph, active neuron state, learned synaptic state, eligibility traces, recurrent buffers, modulators, update phase, and numeric policy. Save files must include all such state or be captured only at an explicitly atomic boundary where omitted transients are provably empty.

The deterministic implementation should:

- serialize genes and runtime records in canonical stable-ID order;
- use named keyed RNG streams for every stochastic operator;
- construct mutation candidate lists canonically before drawing;
- separate mutation-event identity from traversal order;
- evaluate zero-delay acyclic dependencies in a canonical topological order and delayed/recurrent edges from prior-state buffers;
- use deterministic accumulation order and an explicitly versioned fixed-point or strict numeric policy;
- compute organism actions from a world snapshot, then commit them in canonical order;
- checksum learned weights, traces, buffers, and controller phase—not only the birth genome.

**Recommendation strength: Strongly supported.** These are direct consequences of exact replay and cross-platform determinism. Rust’s default `HashMap` is randomly seeded and does not provide canonical iteration order, while several floating-point library operations have platform- or version-dependent accuracy or NaN behavior. [R69–R71]

## 1.9 Development sequence

The dependency-ordered path is:

1. canonical graph schema, validators, stable IDs, and graph compiler;
2. deterministic fixed-state recurrent evaluation without plasticity;
3. structural mutation, crossover, deletion, and innovation-survival tests;
4. complexity budgets and cost accounting;
5. checkpoint/replay of recurrent state;
6. one fixed local plasticity rule with exact save/restore;
7. evolved coefficients and neuromodulatory channels;
8. rule-palette evolution and module duplication;
9. novelty/QD experiments;
10. social-observation experiments;
11. CPPN/HyperNEAT and developmental branches;
12. optional Lamarckian and lifelong structural-plasticity policies.

This order avoids confounding structural evolution, lifetime learning, developmental growth, and persistence before each lower layer has a bit-exact test oracle.

---

# 2. Current scientific landscape

## 2.1 Mature components

The following areas have enough history and reproducible empirical work to justify production-oriented implementation:

| Area | Scientific state | Genesis interpretation |
|---|---|---|
| Direct graph neuroevolution | Mature family of methods; NEAT is the canonical historical-alignment design, with many successors and alternative topology-and-weight evolving systems. [R1–R6] | Suitable foundation for Controller V2. Do not assume one canonical mutation-rate or speciation formula. |
| Recurrent neural controllers | Mature in cognitive modeling and evolutionary robotics. Both discrete RNNs and CTRNNs can produce memory and dynamical behavior. [R5, R30–R33] | Begin with explicit discrete-time recurrence/delays for simpler deterministic semantics; retain a future CTRNN policy boundary. |
| Local synaptic plasticity | Strong biological and computational foundations for Hebbian, normalized Hebbian, timing-based, and three-factor rules. [R34–R41] | Implement a constrained, bounded, versioned local rule family; biological plausibility alone does not determine engineering fit. |
| Evolved plasticity parameters | Repeated demonstrations across evolutionary robotics and meta-learning, although usually on bounded tasks. [R41–R50] | Plausible for Genesis after fixed-rule validation. Use coefficient sharing before per-synapse parameter explosions. |
| Novelty and quality diversity | Mature enough as population-search tools, with clear successes and clear dependence on behavior characterization. [R42, R59–R62] | Optional experimental selection layer, not a controller architecture and not a guarantee of open-endedness. |

## 2.2 Promising but conditional components

Indirect geometric encodings are mature as algorithms but conditional in applicability. HyperNEAT’s evidence is strongest when useful connectivity has geometric regularity. ES-HyperNEAT addresses neuron placement and density, and hybrid indirect/direct methods address exceptions to regular patterns, but all require an externally chosen coordinate system or substrate interpretation. [R7–R11]

Module duplication, connection-cost pressure, modularly varying goals, and hierarchy-promoting pressure have empirical support, but they do not amount to a universal recipe for modularity. Modular networks can improve evolvability and reduce forgetting in constructed task sequences; however, some evolutionary paths become trapped in nonmodular attractors, and duplication may be more important than merely making modular designs computationally efficient. [R23–R29]

Neuromodulated plasticity has compelling evidence in dynamic reward-based tasks, but the modulatory signal is an architectural commitment. A badly designed reward or internal-state signal creates shortcut learning, oscillation, or indiscriminate reinforcement. [R39–R43]

## 2.3 Early and high-risk components

Developmental neural encodings have a long history, from graph-generation systems and cellular encodings to artificial embryogeny. They can compactly reuse developmental instructions and produce repeated or modular structures, but they generally make mutation effects less local and crossover less transparent. [R12–R17]

Recent Neural Developmental Programs and related NCA-based approaches show that local growth programs can assemble functional neural networks, and newer work combines developmental and activity-dependent plasticity. The evidence is scientifically relevant but still preliminary for Genesis: benchmarks are small relative to a persistent artificial-life world; development often uses stochastic or gradient-trained machinery; and the implementation must preserve every growth-policy detail for exact replay. [R18–R22]

Lifelong structural plasticity—adding and deleting connections during an organism’s life rather than only across reproduction—is especially high risk. It combines developmental mutation, learning, structural identity, runtime memory allocation, persistence, and credit assignment. It should be isolated behind a later policy version after synaptic plasticity and hereditary structural evolution are independently validated. [R20]

## 2.4 What the literature does not establish

The literature does not establish that any of the following is sufficient for open-ended behavioral complexity:

- variable topology;
- recurrence;
- plasticity;
- modularity;
- indirect encoding;
- novelty search;
- developmental growth;
- social observation.

Each can remove a bottleneck or alter the search landscape. None substitutes for appropriate embodied sensors and actions, ecological opportunities, energetic tradeoffs, persistent environmental consequences, population structure, and enough compute. The controller architecture should therefore be evaluated as one component of an open-ended system, not as the source of progress by itself.

---

# 3. Variable-topology approaches

## 3.1 Advantages over large fixed-topology networks

A sufficiently large fixed network can represent every smaller network obtainable by masking weights. That fact does not make the two evolutionary search problems equivalent.

Variable topology can offer:

1. **Dimension matching.** Evolution searches only the parameters currently expressed rather than optimizing a large number of irrelevant weights. This matters when the required controller size is unknown and fitness evaluations dominate cost.
2. **Structural stepping stones.** Adding a node or path can create a qualitatively new dynamical mechanism that can then be tuned locally.
3. **Implicit regularization and efficiency.** Smaller controllers can be cheaper to evaluate and less prone to relying on fragile redundant interactions.
4. **Evolvable recurrence and memory.** Recurrent loops, delays, and modulatory pathways can appear only where useful rather than being prewired everywhere.
5. **Lineage-level interpretability.** Structural additions, deletions, duplications, and reuse can be tracked as evolutionary events.
6. **Co-evolution with capabilities.** If future genomes vary sensors, effectors, or object-interaction channels, topology can adapt to changed interfaces instead of retaining a fixed dense scaffold.

These advantages are conditional. A large fixed sparse-mask architecture can provide deterministic memory bounds, efficient array execution, simple crossover, and a stable search domain. It may be better when the interface is stable, a reasonable maximum architecture is known, and hardware throughput matters more than genotype compactness.

**Finding: Supported in narrower conditions.** Variable topology is a search bias, not a dominance theorem.

## 3.2 When NEAT has actually outperformed fixed topology

The original NEAT study compared its complexifying topology-and-weight evolution against fixed-topology neuroevolution on pole-balancing benchmarks, including a difficult non-Markovian double-pole task. It reported faster learning and used ablations to show that removing speciation, historical crossover alignment, or minimal-start complexification harmed the integrated system. [R1]

The defensible conclusion is narrower than “NEAT beats fixed topology”:

- the task required temporal behavior and had an unknown useful topology;
- structural innovations were initially disruptive because their new weights were not yet optimized;
- speciation protected those innovations;
- historical markers made sexual crossover between differing graphs workable;
- starting minimally reduced early search dimension.

Later NEAT-family studies span games, control, feature selection, recurrent/continuous-time variants, modular systems, and hybrid optimization, but comparison protocols are heterogeneous and many application papers do not establish superiority over well-tuned modern fixed-topology baselines. [R2–R3]

NEAT-like evolution is most likely to outperform fixed topology when:

- controller size is unknown and varies meaningfully among solutions;
- useful behavior requires a sequence of topology changes rather than only fine weight adjustment;
- the fitness landscape is deceptive enough that preserving structural lineages matters;
- networks remain small or medium enough that direct graph search is tractable;
- evaluations are expensive enough that reduced parameter dimension offsets population-management overhead.

It is less likely to win when:

- a strong topology is known;
- the task is smooth and mostly weight optimization;
- high-dimensional regular connectivity is required;
- fixed arrays permit substantially better vectorization;
- recombination of heterogeneous graphs destroys more useful structure than it creates;
- population size is too small to support speciation and structural exploration simultaneously.

**Finding: Strongly supported as a conditional statement.**

## 3.3 Essential NEAT ideas versus historical choices

### Essential or broadly reusable ideas

| Idea | Why it matters | Genesis treatment |
|---|---|---|
| Historical structural alignment | Prevents arbitrary positional crossover between differently shaped graphs. | Keep, but represent homology separately from origin and phenotype. |
| Protection for new structural innovations | New structure often begins with poorly tuned weights and loses under immediate global competition. | Keep through species, age/probation, lineage niches, or another explicit protection mechanism. |
| Incremental, locality-preserving topology mutation | Makes structural search less destructive than resampling whole graphs. | Keep add-edge, split-edge/add-node, disable, enable, delete, and later duplicate-module operators. |
| A complexity bias | Prevents the search from starting in an unnecessarily large parameter space. | Use small initial graphs, explicit costs, deletion, and hard budgets. |

### Historical choices rather than universal requirements

- one global monotonically increasing innovation counter;
- a particular compatibility-distance formula and coefficients;
- a particular species threshold and fitness-sharing procedure;
- a generational innovation table with a specific lifetime;
- beginning with every input connected directly to every output;
- only complexifying and not structurally simplifying;
- a particular sigmoid activation and response range;
- a fixed disabled-gene inheritance heuristic;
- a particular excess/disjoint-gene rule based on parent fitness;
- fixed mutation probabilities reported for the benchmark;
- sexual crossover as the dominant reproduction mechanism.

Genesis should preserve the **invariants** the mechanisms are intended to provide and version the concrete policy used to provide them.

**Recommendation strength: Strongly supported.**

## 3.4 Representing structural homology during crossover

Homology is a statement about evolutionary correspondence, not merely about graph appearance. Two edges can have the same source and destination yet arise through different duplicated modules, and two historically homologous genes can diverge in weight, plasticity, enable state, delay, or activation context.

A robust Genesis gene should therefore contain or derive:

```text
GeneIdentity {
    gene_lineage_id       // identity inherited through descendants
    homology_class_id     // crossover alignment class
    origin_event_id       // creation event
    structural_signature  // canonical expressed role
}
```

For nodes, the structural signature includes node role/type and, where relevant, the ancestral connection split that created it. For connections, it includes source homology, destination homology, delay class, connection type, and possibly modulator channel. The signature is useful for validation and secondary alignment but should not erase ancestry.

Recommended crossover alignment order:

1. match exact `homology_class_id`;
2. within a class, prefer exact `gene_lineage_id` where duplicates or allelic alternatives exist;
3. treat remaining genes as nonmatching rather than matching by vector position;
4. select alleles using a named keyed RNG draw;
5. apply an explicit policy for disabled genes;
6. construct node closure for inherited connections;
7. run canonical validation and a deterministic, versioned repair policy—or reject the offspring if repair would be ambiguous.

**Recommendation strength: Plausible but unproven.** Historical alignment is strongly supported; this four-identity decomposition is a Genesis-specific design that needs property-based crossover tests.

## 3.5 Innovation identifiers and deterministic parallel assignment

### Original pattern

NEAT assigns historical innovation numbers when structural genes arise. The original implementation checks a record of recent structural innovations so an equivalent mutation arising again in the tracked period can reuse the historical marker. These markers then persist through inheritance and align genes during crossover. [R1]

### Why a shared sequential counter is unsuitable

In a parallel simulator, a process-wide `next_innovation += 1` makes identity depend on which reproduction event reaches the allocator first. Thread scheduling, work-stealing, shard order, or a different core count can change the resulting genomes even if every local random draw is identical.

### Recommended deterministic event key

Define an offspring event before mutation:

```text
reproduction_event_id = H(
    "genesis.reproduction.vN",
    world_lineage_id,
    generation_or_birth_epoch,
    ordered_parent_lineage_ids,
    reproductive_encounter_id,
    offspring_ordinal,
    reproduction_policy_version
)
```

Each mutation operator then derives its own stream and event:

```text
mutation_event_id = H(
    "genesis.mutation.add_edge.vN",
    reproduction_event_id,
    operator_invocation_ordinal,
    bounded_attempt_ordinal,
    canonical_target_key
)
```

The created gene lineage ID can be a domain-separated hash of the mutation event. Its homology class can be derived from the ancestral structural operation—for example, “split ancestral edge homology X” or “connect source homology A to destination homology B with delay D and type T.” A collision-resistant 128- or 256-bit internal ID avoids global coordination; serialized IDs should use a fixed byte order and hash/version identifier.

A hash is not randomness here. It is a deterministic identity function. Collision handling must nevertheless be specified: a collision between unequal canonical preimages is a fail-closed integrity error, not an invitation to allocate the next free number.

**Recommendation strength: Strongly supported as a deterministic design; plausible but unproven as a crossover-homology policy.**

## 3.6 Independent identical mutations

Two identical-looking mutations can mean different things:

- **Same ancestral operation:** two offspring independently split homologous copies of the same ancestral connection. Treating the resulting node/edge roles as homologous is useful for crossover.
- **Convergent connection:** two lineages independently add the same source-to-destination structural signature. They may occupy the same crossover slot, but their creation histories remain distinct.
- **Duplicate-module ambiguity:** two modules contain internally identical edge signatures. Matching solely by endpoints can align the wrong module copies.
- **Phenotypic convergence:** different developmental or duplication histories produce the same final graph. Phenotype equality does not prove gene homology.

If every independent event receives a completely unrelated innovation ID, crossover overcounts disjoint genes and can fail to combine genuinely corresponding structures. If every equal structural signature is declared identical, crossover can collapse duplicated or convergently evolved roles. The dual identity design allows a shared homology class with distinct origin lineages.

A Genesis experiment should explicitly create these cases and test whether crossover preserves function more often than either “event ID only” or “signature only” alignment.

## 3.7 Disabled genes

Disabled genes are valuable because they provide reversible structural variation and preserve historical alignment. They should:

- remain serialized in canonical order;
- be excluded from runtime evaluation;
- retain their IDs, weights, plasticity metadata, and ancestral role;
- be eligible for deterministic re-enable mutation;
- have an explicit inheritance rule when one or both parental alleles are disabled;
- count toward a genotype-history budget even when they do not count toward the hot phenotype budget.

Without a separate history budget, disabled genes become an unbounded archive. Genesis should therefore define one of three policies:

1. retain all disabled genes until a hard genome cap, then reject further growth;
2. garbage-collect disabled genes only after a versioned age and ancestry criterion;
3. move ancient disabled records to a lineage archive outside the reproductive genome while retaining enough tombstone information for audit.

Policy 2 or 3 is preferable for long runs, but both require tests showing that crossover behavior does not silently change after compaction.

**Recommendation strength: Strongly supported for reversible disable; plausible but unproven for archival compaction.**

## 3.8 Recurrent connections, cycles, isolated nodes, and invalid graphs

### Recurrent connections

Do not define recurrence as “whatever cycle happens to be encountered by the evaluator.” Encode a nonnegative integer delay on every connection.

- `delay = 0`: same-tick dependency; the enabled zero-delay subgraph must be acyclic.
- `delay >= 1`: reads a previous activation from an explicit ring buffer; delayed cycles are legal.

This gives an unambiguous compiler and evaluation order. A simpler first policy may make every non-sensor edge one-tick delayed; that is less expressive per tick but even easier to reason about. The choice must be a controller-policy version.

### Isolated nodes

An isolated hidden node is not inherently invalid. It can be:

- retained as neutral genotype material;
- excluded from the compiled executable phenotype;
- charged a small genotype-history cost;
- eligible for reconnection or deletion.

A node that is unreachable from any sensor or cannot reach any output does not affect current behavior. The graph compiler should identify active reachability deterministically and report dormant structure separately.

### Invalid graphs

Fail closed on:

- duplicate stable IDs with unequal records;
- missing endpoints;
- illegal node roles;
- zero-delay cycles;
- out-of-range delays or coefficients;
- unsupported activation or plasticity rule versions;
- multiple edges with a forbidden duplicate structural signature;
- output nodes missing required semantics;
- numeric encodings outside policy bounds;
- inconsistent module or homology references.

Mutation and crossover should generate valid graphs by construction where possible. A deterministic repair pass may disable offending edges in stable-ID order, but repair semantics become part of evolution and therefore must be versioned, logged, and tested. Silent repair is unacceptable.

**Recommendation strength: Strongly supported.**

## 3.9 Should removal mutations exist from the beginning?

Yes.

A direct graph system without removal has a one-way complexity ratchet. Selection can reject costly additions, but neutral or weakly deleterious structure can hitchhike, disabled history can accumulate, and long-lived lineages can become expensive even when behavior does not improve. EPNet explicitly prioritized deletion before addition and produced compact networks in its benchmark family; connection-cost research likewise shows that structural cost changes evolved organization. [R6, R24, R26]

Recommended first-version operators:

- **disable edge:** common, reversible;
- **enable edge:** uncommon, reversible;
- **delete edge:** low rate, irreversible within that genome except by convergent re-addition;
- **prune hidden node:** only when all incident edges are removed/disabled, or as one atomic event that removes the node and incident edges;
- **simplify pass:** not an optimizer outside evolution; only a deterministic representation cleanup that removes unreachable compiled state without altering the heritable genome.

Keep addition and deletion separately measurable. A useful tuning target is near-zero expected neutral edge growth rather than assuming add rates should exceed remove rates.

**Recommendation strength: Strongly supported.**

## 3.10 Topology mutation rates, bloat, convergence, and evolvability

There is no scientifically defensible universal add-node or add-edge rate. The correct rate depends on population size, reproductive cadence, controller evaluation length, structural cost, crossover frequency, and how disruptive new edges are.

Expected effects:

| Regime | Likely behavior |
|---|---|
| Structural rate too low | Topology freezes; evolution behaves like fixed-topology weight search; structural innovations rarely receive repeated tuning. |
| Structural rate moderately low | Innovations are sparse enough to be protected and tuned; lineages retain functional continuity. |
| Structural rate high | Continual disruption, species fragmentation, poor weight refinement, large invalid/rejected-mutation fraction. |
| Addition exceeds deletion under weak cost | Monotonic bloat and increasing simulation cost. |
| Deletion too aggressive | Fragile useful pathways are repeatedly erased; selection favors redundant or deletion-resistant structures. |
| Re-enable too high | Disabled-history cycling consumes search without meaningful novelty. |

Tune rates by measuring **realized** mutations, not only attempted probabilities. Bounded retries and invalid-target rates can make the realized distribution differ sharply from configuration values.

Recommended telemetry:

- attempted, valid, and inherited mutations by operator;
- structural survival at 1, 5, 20, and 100 descendant generations;
- fitness change immediately after mutation and after a tuning window;
- edge/node growth per birth and per unit fitness gain;
- active versus dormant versus disabled genes;
- species or lineage fragmentation;
- evaluation cost and checkpoint size by lineage age;
- deletion of genes later found useful in counterfactual re-enable tests.

**Recommendation strength: Strongly supported for measurement; rates themselves are empirical.**

## 3.11 Keeping networks bounded

Use layered controls rather than one mechanism:

1. **Hard caps:** maximum nodes, enabled edges, plastic edges, delayed edges, maximum delay, and serialized gene records.
2. **Execution budget:** controller evaluation receives a deterministic maximum operation count. A genome that cannot compile within budget is invalid or has low fitness under an explicit rule.
3. **Energetic cost:** each active node, edge, plastic update, and memory cell can consume simulated energy if that cost belongs in the world model.
4. **Selection parsimony:** use complexity as a secondary lexicographic objective, Pareto objective, or tie-breaker rather than an arbitrary large weighted penalty.
5. **Deletion and pruning:** permit lineages to simplify.
6. **Sparse compilation:** execute only active, output-relevant nodes and edges.
7. **Budget-aware mutation:** reject additions beyond caps without consuming order-dependent random draws.

Hard caps are safety mechanisms, not evolutionary explanations. Energetic cost is scientifically meaningful only if controller computation is intended to represent a biological/metabolic tradeoff. Keep engine-resource caps distinct from organism-visible metabolic costs.

## 3.12 Encouraging modularity, reuse, and hierarchy

No single mechanism guarantees functional modularity. Evidence supports several conditional pressures:

- **Connection cost/locality:** shorter or fewer connections can favor modules and hierarchy. [R24, R26–R27]
- **Modularly varying goals:** environments whose subproblems recombine can favor reusable modules. [R25]
- **Duplication and divergence:** copying a functional subgraph allows one copy to preserve function while the other specializes. [R23, R29]
- **Module-level operators:** crossover and mutation that preserve subgraphs reduce destructive variation.
- **Protected innovation and QD:** maintaining multiple structural strategies prevents premature convergence, though behavior characterization can misdirect search. [R59–R62]
- **Changing environments:** modular networks have shown advantages in learning new skills without forgetting old ones in constructed experiments. [R28]

For Genesis, the first modularity intervention should be **module duplication with persistent copy identity**, not an elaborate developmental grammar. Define a module as an evolvable subgraph record only after empirical graph-community measures correlate with functional lesion tests. Otherwise, the engine risks imposing human-declared modules that do not match evolved computation.

**Recommendation strength: Supported in narrower conditions.**

---

# 4. Direct versus indirect encodings

## 4.1 Encoding families

### Direct node-gene and connection-gene graph

Each expressed node and connection has an explicit gene. Mutation changes local graph records, crossover aligns historical homologs, and compilation is straightforward.

**Strengths:** transparent genotype–phenotype mapping, local mutations, mature crossover concepts, simple checkpoint integration, easy structural telemetry, natural support for arbitrary recurrence and plasticity metadata.

**Weaknesses:** genome size grows with phenotype; repeated motifs are separately encoded; crossover becomes difficult under duplication; large geometric networks are inefficient; direct encodings do not automatically create modules.

**Genesis fit:** highest for Controller V2.

### CPPN

A Compositional Pattern-Producing Network composes functions that naturally generate symmetry, repetition, gradients, and other spatial patterns. The CPPN is a generative function, not the organism controller itself. [R7]

**Strengths:** compact encoding of regular patterns; mutations can change coherent global structure; reusable geometric bias.

**Weaknesses:** function composition has global pleiotropic effects; useful coordinate system must be chosen; irregular exceptions are awkward; expressed learned state is not compact merely because the birth generator is compact.

**Genesis fit:** later branch for stable sensorimotor geometry, body plans, or repeated spatial neural motifs.

### HyperNEAT and ES-HyperNEAT

HyperNEAT uses a CPPN to query weights or expression across a geometric substrate. ES-HyperNEAT also discovers placement and density rather than requiring every hidden location to be fixed. [R8–R9]

**Strengths:** scales to larger regular networks; can exploit symmetry/locality; resolution can change while retaining a related pattern.

**Weaknesses:** substrate coordinates are an experimenter-defined prior; phenotype generation and thresholding add policy semantics; indirect mutations can alter many edges; runtime and checkpoint cost remain proportional to the expressed network; structural homology between generated phenotypes is less direct.

**Genesis fit:** supported only where coordinate meaning is stable and empirically useful.

### Developmental programs and grammars

A compact program iteratively grows a network through production rules, cell division, local signaling, or graph rewriting. Classical examples include Kitano’s graph-generation system and Gruau’s cellular encoding. [R12–R16]

**Strengths:** gene reuse, repeated motifs, scalable structure, developmental lineage, potentially natural duplication and repair.

**Weaknesses:** small genome changes can cause large developmental changes; development can fail or explode; crossover of programs is weakly aligned with phenotype; exact replay must version the entire interpreter and growth schedule; evolved programs can exploit interpreter edge cases.

**Genesis fit:** research branch after direct encoding.

### Gene-regulatory encodings

A gene-regulatory network maps concentrations, signals, thresholds, and regulatory interactions into growth or controller parameters.

**Strengths:** rich context-sensitive development, reuse of regulatory motifs, potential robustness.

**Weaknesses:** large semantic design space, numerical dynamics, indirect credit assignment, difficult validation, and high risk that human-authored regulatory semantics dominate outcomes.

**Genesis fit:** speculative.

### Module duplication

A direct graph remains primary, but a mutation copies a subgraph and its external interface, assigning new lineage IDs while preserving correspondence to the source module. [R23, R29]

**Strengths:** direct, interpretable reuse; preserves existing function while enabling divergence; easier than a full developmental program.

**Weaknesses:** choosing module boundaries is difficult; duplication can cause large immediate cost; crossover must distinguish copy identity from internal homology; duplicated outputs can interfere.

**Genesis fit:** best second-stage modular extension.

### Repeated motifs and coordinate annotations

A direct graph can add optional motif references, neuron coordinates, or module tags without making them the sole generator.

**Strengths:** hybrid path; preserves direct inspectability while allowing geometric mutation or duplication.

**Weaknesses:** redundant representations can disagree; tags may become nonfunctional metadata; coordinate mutation adds another search dimension.

**Genesis fit:** plausible after the direct baseline.

## 4.2 Evidence about regularity

Experiments comparing HyperNEAT with direct encodings found that indirect encoding gained advantage as problem regularity increased, while hybrid methods were better able to add irregular refinements to a regular pattern. [R10–R11] This is the right interpretation for Genesis:

- geometric encodings are not “more advanced” in the abstract;
- they are powerful when their bias matches the environment;
- they can be actively harmful when a coordinate system groups elements that should evolve independently;
- a hybrid direct-offset mechanism can recover exceptions but adds complexity and another representation to reconcile.

Before adopting HyperNEAT, construct a benchmark continuum in which Genesis sensor/actuator layouts range from deliberately regular to permuted or irregular. If the indirect representation does not retain a clear sample-efficiency or compactness advantage under the actual interface, it should not enter production.

## 4.3 Crossover comparison

| Encoding | Natural crossover unit | Main failure mode | Deterministic difficulty |
|---|---|---|---|
| Direct graph | Historical node/connection homologs | Competing conventions, duplication ambiguity, invalid offspring | Moderate; canonical alignment solves order dependence but not biological correspondence |
| CPPN/HyperNEAT | Generator genes | Small generator recombination causes global phenotype disruption | Moderate at genotype level; phenotype regeneration and thresholds must be versioned |
| Grammar | Production subtrees/rules | Syntactically valid but developmentally catastrophic offspring | High; interpreter, expansion order, and resource limits are semantics |
| Gene-regulatory | Regulatory genes/interactions | Nonlocal dynamical changes and unstable development | High; numeric integration and event ordering are critical |
| Module duplication | Whole module plus internal genes | Copy-number and interface mismatch | Moderate-to-high; copy IDs and boundary closure required |
| Fixed sparse mask | Fixed array position | Positional genes may not correspond functionally | Low; representation is stable, but homology is imposed rather than discovered |
| Cartesian GP | Fixed grid positions/function genes | Inactive-code accumulation and positional bias | Low-to-moderate; decode is deterministic and recurrence can be explicit |

## 4.4 Learned state changes the compactness calculation

An indirect genome may generate one million synapses from a small CPPN, but if 200,000 of those synapses are plastic, exact restoration still requires either:

- storing every learned effective weight or delta and every trace; or
- storing a lossless event log from birth and deterministically replaying the entire lifetime.

The second option is generally unacceptable for checkpoints because restore time grows with age and any missing environmental interaction breaks reconstruction. Therefore, indirect encodings reduce **hereditary genome size**, not necessarily runtime or checkpoint size.

This is a central constraint for Genesis and a reason not to select HyperNEAT solely for genome compactness.

**Finding: Strongly supported by persistence requirements.**

## 4.5 Recommendation

Use a direct graph as the normative representation. Design its IDs and serialization so a future gene may optionally reference:

- a duplicated module;
- a CPPN-generated initialization pattern;
- a developmental lineage;
- a coordinate or body-relative anchor.

Do not make those fields active until separate policy versions and experiments justify them.

**Recommendation strength: Strongly supported.**

# 5. Neural modularity and developmental encodings

## 5.1 Modularity is functional, structural, and representational

“Modular network” can refer to at least three different properties:

1. **Structural modularity:** the graph contains dense within-group and sparse between-group connectivity.
2. **Functional modularity:** different subcircuits make separable causal contributions to behavior.
3. **Genotypic modularity:** genetic operators can vary one subsystem without disrupting others.

These properties often correlate but are not equivalent. A graph can have high community modularity while computation is distributed across communities. Conversely, a recurrent dynamical system can implement separable modes without a visually obvious graph partition. A genotype may encode modules cleanly even if development overlaps them in the phenotype.

Genesis should not award “modularity” based solely on a graph-community score. Use at least:

- a structural community measure computed canonically;
- lesion or silencing tests measuring behavioral specificity;
- mutation-locality tests measuring whether changes remain within a subsystem;
- reuse evidence, such as one subgraph contributing to multiple tasks or contexts;
- crossover-survival evidence.

**Recommendation strength: Strongly supported as a measurement principle.**

## 5.2 Evidence for connection cost and modularly varying environments

Work on the evolutionary origins of modularity found that selecting for lower connection cost can produce more modular networks and increase evolvability in constructed tasks. Related experiments found hierarchy under connection-cost pressure and modularity under environments whose goals vary by recombining subproblems. [R24–R27]

The narrow, defensible claim is:

> When tasks contain decomposable structure, and the evolutionary objective or environment rewards economical wiring or repeatedly recombines subproblems, modular organizations can be favored and can improve future adaptation.

This does not imply that a generic edge penalty will create useful modules in Genesis. If perception, action, and reward do not contain reusable subproblems, cost pressure may merely shrink the network. If the cost dominates, it can eliminate necessary long-range integration. If coordinates are arbitrary, “short connection” has no physical meaning.

For Genesis:

- count active edges as an engine cost;
- consider a simulated wiring/metabolic cost only if node coordinates or communication distances have semantic meaning;
- use modularly varying benchmark tasks to test reuse before expecting ecological worlds to induce it;
- never use a post hoc modularity score as an organism-visible objective unless the research question explicitly concerns imposed modularity.

**Recommendation strength: Supported in narrower conditions.**

## 5.3 Gene duplication and divergence

Biological gene duplication provides a general evolutionary route in which one copy preserves an existing function while another diverges. Artificial-life work has shown that duplicating modules can facilitate specialization, and later analysis suggests that duplication can provide a route out of nonmodular attractors that gradual edge-level variation does not reliably escape. [R23, R29]

A Genesis module-duplication event should be atomic and explicit:

```text
DuplicateModuleEvent {
    source_module_instance_id
    selected_node_ids[]
    selected_internal_edge_ids[]
    copied_boundary_policy
    new_module_instance_id
    origin_event_id
}
```

Recommended semantics:

1. Select a candidate subgraph from a canonical list. Initial candidates should be small motifs or previously declared module instances, not arbitrary graph partitions chosen nondeterministically.
2. Copy every internal node and edge with new `gene_lineage_id` values.
3. Preserve an `ancestral_homology_id` linking each copy to its source role while assigning a distinct `module_copy_id`.
4. Apply a versioned boundary policy: duplicate incoming edges, outgoing edges, both, or neither. Do not choose ad hoc based on iteration.
5. Optionally perturb one boundary or one internal parameter after duplication using a separate named RNG stream.
6. Validate budgets and zero-delay acyclicity before committing.

Crossover then aligns genes by module-copy ancestry and internal role rather than confusing two copies that happen to contain identical endpoints.

A first implementation can avoid automatic module discovery by allowing duplication only of:

- a single hidden node plus incident edges;
- a bounded-radius motif rooted at a selected node;
- a subgraph previously created by a duplication event.

More ambitious community-based duplication should wait until lesion tests show that graph communities correspond to function.

**Recommendation strength: Supported in narrower conditions for duplication; plausible but unproven for the proposed identity scheme.**

## 5.4 Recurrent neural networks

Discrete recurrent neural networks provide internal state through feedback. They are a natural controller family for partially observable worlds because organisms rarely receive a complete Markov state. Classical recurrent-network work demonstrates sequence processing, while early neuroevolution constructed recurrent networks and used recurrent dynamics for adaptive behavior. [R5, R30, R65]

Advantages for Genesis:

- compact memory of recent events;
- oscillators and action sequencing;
- context-dependent responses;
- potential working memory for observed actions and object states;
- straightforward fixed-tick deterministic evaluation;
- local plasticity can depend on pre/post recurrent activity.

Risks:

- recurrent attractors can saturate or become chaotic;
- tiny weight mutations can have long-horizon effects;
- apparent “learning” may be transient neural state rather than synaptic change;
- reset semantics become experimentally important;
- recurrent state expands checkpoint data.

A discrete recurrent controller should be the default. It should use explicit delays and bounded activation functions, not implicit recursive evaluation.

**Recommendation strength: Strongly supported.**

## 5.5 Continuous-time recurrent neural networks

CTRNNs model neuron state with differential equations, commonly in a form such as:

\[
 \tau_i \frac{dy_i}{dt} = -y_i + \sum_j w_{ji}\sigma(y_j + b_j) + I_i.
\]

They have a substantial evolutionary-robotics history and can generate rich low-dimensional dynamics, including oscillation, switching, and sensorimotor coordination. [R31–R33]

Potential advantages:

- time constants evolve independently of the world tick;
- smooth dynamical behavior;
- compact central-pattern-generator-like circuits;
- closer relation to dynamical-systems analysis.

Costs for Genesis:

- the numerical integrator, step size, substep count, rounding, and saturation become controller semantics;
- variable or adaptive solvers are inappropriate for exact replay unless every branch is fully specified and serialized;
- fixed-point integration can be stable but requires carefully bounded coefficients;
- more arithmetic per neuron per tick;
- stiffness or unstable time constants can cause overflow or saturation.

If implemented, use a fixed-step explicit integrator with a versioned equation and fixed number of substeps. Do not silently switch integrators or exploit platform math libraries. Compare it against a discrete leaky RNN with equivalent per-node time constants; CTRNN should enter production only if it yields consistent behavioral or evolvability gains.

**Recommendation strength: Supported in narrower conditions.**

## 5.6 Dynamics can mimic learning

Evolved recurrent systems can change behavior during a trial without changing synaptic weights. Yamauchi and Beer demonstrated sequential behavior and learning-like adaptation in evolved dynamical networks. [R33] This creates an important experimental distinction:

- **stateful adaptation:** behavior changes because neuron activations or attractor state changed;
- **synaptic learning:** behavior changes because persistent plastic variables changed;
- **developmental change:** behavior changes because topology or neuron identity changed;
- **environmental scaffolding:** behavior changes because the world retains information.

Genesis telemetry must label these separately. A “plasticity off” ablation is insufficient if recurrent state remains. Use newborn-state resets, recurrent-state resets, and learned-weight resets as separate interventions.

**Recommendation strength: Strongly supported.**

## 5.7 Classical developmental encodings

### Graph-generation systems

Kitano’s graph-generation approach encoded rewriting rules that generated network connectivity. [R12] The main benefit is reuse: a compact rule can produce repeated structure. The main cost is developmental sensitivity: a mutation in a high-level production can alter a large fraction of the phenotype.

### Cellular encoding

Gruau’s cellular encoding represented a developmental program in which cells divide and transform to generate modular neural networks. It demonstrated that grammars could produce families of structured networks and later integrated learning and the Baldwin effect. [R13–R14, R56]

### Artificial embryogeny

Taxonomies of artificial embryogeny distinguish explicit versus implicit, centralized versus distributed, and direct versus emergent developmental processes. [R16] The taxonomy is useful because “developmental encoding” is not one algorithm. Genesis would need to choose:

- whether every cell runs the same program;
- whether development is synchronous;
- what local information a cell receives;
- whether cell positions are discrete or continuous;
- whether growth has fixed rounds or stops by local condition;
- how identity and homology survive division;
- how resource conflicts are resolved;
- what happens when growth exceeds budgets.

Every choice affects evolvability and replay.

### Developmental neural dynamics

Astor and Adami modeled neural development using autonomous neurons and local chemical interactions. [R15] Such systems are relevant scientifically, but asynchronous local updates are a direct mismatch with Genesis unless converted into deterministic synchronous rounds or a canonical event queue.

**Assessment: Plausible but unproven for Genesis.**

## 5.8 Neural cellular automata and Neural Developmental Programs

Neural cellular automata use a shared local neural update to grow or maintain a spatial pattern. “Growing Neural Cellular Automata” demonstrated regeneration and self-maintenance of target images under gradient training. [R21] HyperNCA and NDP work extended related ideas to growing controller networks, while later NDP work identified maintenance of neuronal diversity as a key challenge. [R18–R19, R22]

The important findings for Genesis are not that NCA should be adopted immediately, but that:

- a shared local program can generate a much larger structure;
- symmetry breaking and persistent intrinsic identity are necessary to avoid homogeneous collapse;
- development can create robustness and repeated structure;
- the genotype-to-phenotype map can be optimized by evolution as well as gradient methods;
- developmental state itself may need persistence and versioning.

Recent Lifelong Neural Developmental Program work adds reward- and activity-dependent synaptic and structural plasticity. [R20] This is directly relevant to Genesis’s long horizon but remains a high-risk combination. It often relies on graph-neural or attention-like machinery whose per-organism cost is much higher than a small RNN, and its structural plasticity would make the organism’s active graph part of learned runtime state.

A Genesis NDP branch would require:

- fixed synchronous developmental rounds;
- stable cell lineage IDs;
- keyed choices for division and connection targets;
- deterministic conflict resolution;
- hard growth budgets;
- canonical graph materialization;
- checkpointing the developed graph and any ongoing developmental state;
- a frozen interpreter per policy version.

**Recommendation strength: Speculative for production; plausible but unproven as a research branch.**

## 5.9 Developmental encodings and crossover

Developmental programs often reduce the value of phenotype-level historical markers. Two homologous program rules can generate nonhomologous structures after divergence, while two different programs can converge on similar structures. Possible crossover levels include:

- program instruction or grammar-rule alignment;
- developmental lineage alignment;
- expressed phenotype alignment;
- module/interface alignment;
- no crossover, relying on mutation and asexual reproduction.

There is no universally correct choice. Genesis should not add a developmental encoding while assuming the direct graph crossover system will automatically apply. A developmental branch needs a separate reproduction policy and empirical offspring-validity study.

**Recommendation strength: Strongly supported.**

## 5.10 Recommended modularity roadmap

1. Measure structural and functional modularity in direct graphs.
2. Add connection-count cost and deletion; test whether useful modularity appears.
3. Add explicit motif/module duplication with ancestry-aware IDs.
4. Add module-preserving crossover and lesion telemetry.
5. Test modularly varying benchmark environments.
6. Only then test CPPN pattern generators or developmental programs.
7. Keep NCA/NDP and lifetime structural plasticity in isolated research policy versions.

This sequence preserves interpretability and produces a baseline against which claims of developmental advantage can be falsified.

---

# 6. Lifetime-plasticity models

## 6.1 Requirements specific to Genesis

A viable learning rule for many persistent organisms must satisfy more than benchmark performance. It should be:

- local or nearly local in information requirements;
- bounded in state and arithmetic;
- continuously applicable online;
- deterministic under a fixed update order;
- compactly parameterized in the genome;
- robust against runaway weights;
- checkpointable without replaying the organism’s life;
- interpretable enough for ablation;
- compatible with later observation-based social learning;
- inexpensive enough for thousands of organisms.

No learning rule simultaneously maximizes biological detail, computational efficiency, stability, and evolvability. Genesis should therefore implement a **small rule palette** behind a common state interface rather than one infinitely general equation.

## 6.2 Comparison matrix for learning models

Ratings are relative to the Genesis use case. “Low cost” assumes a small rate-based controller; spiking simulation changes the baseline.

| Model | Biological plausibility | Compute / memory | Deterministic implementation | Online continuity | Social-learning fit | Forgetting / stability | Global error or backprop | Genome compactness | Exact checkpoint |
|---|---|---|---|---|---|---|---|---|---|
| No synaptic learning; recurrent state only | Low as a complete model, high as a useful control | Very low / node state | Easy | Stateful but not learned | Limited; can react to demonstrations without retaining long-term changes | No synaptic forgetting; recurrent interference possible | None | Excellent | Easy; save node/buffer state |
| Plain Hebbian | Moderate as an abstraction | Low; one weight delta per plastic edge | Easy if update phase/order fixed | Excellent | Moderate; observed co-activity can be stored | High runaway/interference risk without bounds | None | Excellent | Easy; save deltas |
| Covariance/Oja-style | Moderate | Low; Oja adds normalization term | Easy | Excellent | Moderate | Better boundedness; still unsupervised and can overwrite | None | Excellent | Easy |
| STDP | High for timing-sensitive spiking synapses | Moderate-to-high; spike histories/traces | Moderate; tie timing and event order must be explicit | Excellent for spiking | Moderate; requires meaningful spike timing | Sensitive to rates/window; often needs homeostasis | None | Compact rule, more runtime state | Exact if all traces/spike queues saved |
| Reward-modulated STDP | High-to-moderate | Moderate-to-high; eligibility + modulator | Moderate | Excellent | Good if social outcomes modulate traces | Better credit assignment; still sensitive to signal design | Scalar/modulatory signal, no backprop | Compact | Exact with traces/modulator saved |
| Neuromodulated rate-based three-factor | Moderate | Moderate; trace per plastic edge or shared trace | Easy-to-moderate | Excellent | Good; demonstration/outcome channels can gate learning | Good potential with decay/bounds; catastrophic interference remains | Modulator, no backprop | Good, especially with shared coefficients | Exact |
| Differentiable plasticity | Mechanism can be local at runtime; training is not biologically local | Runtime moderate, offline training very high | Runtime manageable; training pipeline complex | Excellent | Potentially good | Task-dependent; can meta-learn memory | Backprop through episodes during optimization | Potentially very large per-synapse parameter set | Exact at runtime |
| Fully evolved arbitrary local equation | Unknown; depends on DSL | Variable; can become expensive | High unless instruction set is tightly bounded | Potentially excellent | Potentially excellent | High exploit/instability risk | None required | Program compact, state may not be | Exact only if VM state and semantics frozen |
| Self-referential/fast-weight network | Low-to-moderate abstraction | High; matrix updates | Moderate-to-high | Excellent | Potentially strong | Interference and stability difficult | Usually meta-trained | Often large | Exact but expensive |
| Lifetime structural plasticity | Biologically relevant | High; dynamic graph + state | Very high | Excellent | Potentially strong | Can both consolidate and catastrophically restructure | Not necessarily | Compact rule, large runtime state | Exact only with active graph saved |

## 6.3 Hebbian learning

The core Hebbian idea is that correlated activity strengthens association. [R34] A simple rate rule is:

\[
 \Delta w_{ij}=\eta x_i y_j.
\]

Advantages:

- local pre/post information only;
- one multiplication and accumulation per plastic synapse;
- compact genome encoding;
- continuous online operation;
- straightforward deterministic implementation;
- exact checkpoint through the effective weight or delta.

Failure modes:

- positive feedback can drive weights to bounds;
- high-activity features dominate;
- no inherent temporal credit assignment;
- correlations can be incidental rather than useful;
- new experience can overwrite prior associations;
- the rule can encode a fixed attractor rather than meaningful learning.

A plain unbounded Hebbian rule should not be a production default. It is valuable as an ablation and baseline.

**Recommendation strength: Strongly supported as a baseline; unsupported as the sole production rule.**

## 6.4 Oja-style normalization

Oja’s rule modifies Hebbian learning with a normalization term:

\[
 \Delta w_{ij}=\eta y_j(x_i-y_jw_{ij}).
\]

It can be interpreted as a local principal-component learning rule and prevents unconstrained growth under its assumptions. [R35]

Advantages over plain Hebbian learning:

- built-in weight normalization pressure;
- local variables only;
- low extra arithmetic;
- better stability for continuous operation.

Limitations:

- it extracts dominant correlation structure, not necessarily behaviorally valuable structure;
- it can still forget under nonstationarity;
- the rule’s assumptions do not directly match arbitrary recurrent, reward-driven control;
- recurrent use can produce dynamics not captured by feedforward analyses.

Genesis should include Oja-like normalization either as a rule or as an optional homeostatic term in the general local rule.

**Recommendation strength: Supported in narrower conditions.**

## 6.5 Spike-timing-dependent plasticity

STDP changes synaptic strength based on the relative timing of pre- and postsynaptic spikes. Canonical experiments and models established timing-dependent potentiation and depression windows. [R36–R38]

Advantages:

- biologically grounded for spiking systems;
- naturally temporal and local;
- can learn causal ordering at short timescales;
- compatible with eligibility and neuromodulation.

Costs:

- requires a spiking neuron model, spike event queues, or traces that approximate timing windows;
- the meaning of millisecond-scale timing is unclear if the world advances in coarse simulation ticks;
- exact tie semantics matter when spikes occur on the same tick;
- rate and timing homeostasis are often necessary;
- more per-synapse state and branches than a rate rule.

Genesis should not adopt spiking controllers merely to claim biological plausibility. STDP is appropriate only if spike timing creates a testable capability that rate-based recurrence and eligibility traces do not provide.

**Recommendation strength: Supported in narrower conditions for a future spiking branch; not recommended initially.**

## 6.6 Neuromodulated and three-factor plasticity

A two-factor rule uses pre- and postsynaptic activity. A three-factor rule adds a modulator such as dopamine-like reward, surprise, internal need, or contextual gating. Reward-modulated STDP and neuromodulated plasticity address distal reward by accumulating eligibility locally and applying change when a modulatory signal arrives. [R39–R41]

A Genesis rate-based version can use:

\[
 e_{ij,t+1}=\lambda e_{ij,t}+A x_i y_j + B x_i + C y_j + D
\]

\[
 \delta w_{ij,t+1}=\operatorname{bound}\left(
 (1-\rho)\delta w_{ij,t}+\eta m_{k,t}e_{ij,t+1}
 \right).
\]

Potential modulators:

- immediate energy gain or loss;
- prediction error computed by a genetically encoded critic-like neuron;
- pain/damage;
- internal homeostatic deficit;
- novelty or surprise;
- reproductive or social outcome;
- observed demonstrator outcome;
- a learned modulatory neuron output.

The engine should expose primitive internal signals, not a hidden omniscient fitness gradient. A modulator is part of the organism’s embodied information. If “fitness” is injected directly every tick, the engine has authored a learning objective rather than merely made learning possible.

Deterministic requirements:

- modulatory channels have stable IDs;
- signal sampling phase is explicit;
- trace decay and update use fixed numeric rules;
- all plastic edges update in canonical order or independently into separate slots;
- weight bounds and saturation are fixed;
- delayed reward arriving at a checkpoint boundary has an explicit pending-event representation.

**Recommendation strength: Supported in narrower conditions and the preferred initial family.**

## 6.7 Differentiable plasticity

Differentiable plasticity treats plasticity coefficients as trainable parameters and differentiates through lifetime updates. It has demonstrated that large recurrent plastic networks can be meta-trained for memory and adaptation tasks. [R44]

What this establishes:

- plasticity parameters can encode useful learning algorithms;
- runtime local updates can coexist with an outer optimization process;
- a plastic network can outperform a nonplastic equivalent on some tasks requiring rapid adaptation.

What it does not establish:

- that organisms should execute backpropagation;
- that gradient-based meta-training is necessary for Genesis;
- that millions of plasticity parameters are evolutionarily tractable;
- that task-trained plasticity transfers to an open-ended ecology;
- that gradient semantics are compatible with the engine’s fixed-point policy.

Genesis can borrow the **parameterization insight** while using evolution as the outer optimizer. Differentiable training may be used offline as a scientific baseline, but it should not silently define organism cognition.

**Recommendation strength: Supported in narrower conditions as evidence; not recommended as the organism runtime or default evolutionary mechanism.**

## 6.8 Evolved Hebbian rules

Najarro and Risi evolved large numbers of synapse-specific Hebbian-rule parameters and showed rapid self-organization and damage adaptation in selected reinforcement-learning tasks. [R45] Earlier evolutionary-robotics work also evolved plastic neurocontrollers and online self-organization. [R46–R50]

The central tradeoff is parameter granularity:

- **one global rule:** compact and robust, but low expressivity;
- **one rule per neuron type or module:** good compromise;
- **one rule per neuron:** expressive, moderate genome cost;
- **one rule per synapse:** maximum flexibility, but can encode the solution indirectly and explode search dimension.

A per-synapse rule with four or more coefficients can have more heritable parameters than directly evolving the weights. It may still be worthwhile if the rule transfers across lifetime conditions, but compactness cannot be assumed.

Recommended Genesis progression:

1. fixed global rule and fixed coefficients;
2. genome-evolved global coefficients;
3. coefficients by connection class or neuron type;
4. module-shared coefficients;
5. synapse-specific mutation only as an ablation or sparse exception.

**Recommendation strength: Strongly supported for staged sharing; per-synapse evolution is supported only in narrower conditions.**

## 6.9 Catastrophic forgetting and the stability–plasticity tradeoff

Local online rules do not automatically protect old behavior. Catastrophic interference can occur when the same synapses encode incompatible contexts. Relevant mitigations include:

- bounded and decaying updates;
- modulatory gating so learning occurs only in salient contexts;
- separate fast and slow traces;
- consolidation from plastic delta into a slower within-lifetime state;
- sparse plastic masks;
- modular pathways;
- context neurons or recurrent state;
- homeostatic normalization;
- rehearsal through environmental recurrence or internal replay;
- structural duplication in later policies.

Modularity has improved sequential skill learning in constructed evolutionary experiments, but that does not guarantee natural emergence in Genesis. [R28]

The first controller should not implement complex consolidation machinery. It should expose enough state to measure forgetting and add one mechanism at a time.

**Recommendation strength: Strongly supported as an experimental requirement; particular mitigations are conditional.**

## 6.10 Suitability for social learning

A plastic network can support social learning only if the organism can perceive information that distinguishes another organism’s relevant behavior. Necessary affordances may include:

- demonstrator identity or track continuity;
- observed action or body motion;
- object and environmental state before and after the action;
- temporal ordering;
- demonstrator outcome or learner-relevant consequence;
- attention or salience;
- memory long enough to link observation to later action;
- an opportunity to reproduce the behavior.

Robotic and artificial-life experiments demonstrate imitation, embodied transmission, and behavioral traditions when observation and copying mechanisms are present. [R66–R68] They do not show that a generic Hebbian network will spontaneously solve the correspondence problem between another body’s action and its own motor commands.

For Genesis, “social-learning readiness” means the controller can associate observed action-context-outcome sequences with later choices. It does not mean hardcoding an imitation action or cultural objective.

**Recommendation strength: Strongly supported.**

## 6.11 Exact checkpointability by model

All listed models can be checkpointed exactly **if and only if their full runtime state is stored**:

- Hebbian/Oja: effective weight or base-plus-delta;
- trace-based rules: every eligibility trace and decay phase;
- STDP: spike histories, filtered traces, pending events, membrane state;
- neuromodulation: channel values, delays, filters, and pending rewards;
- differentiable plasticity at runtime: plastic weights and recurrent state;
- structural plasticity: the complete active graph, IDs, disabled state, and developmental queues;
- consolidation: fast and slow weights plus consolidation phase.

A checkpoint may store a dense array keyed by compiled edge index only if the save also records the exact stable-ID-to-index mapping or can deterministically reconstruct it from the serialized graph under the same compiler policy.

**Finding: Strongly supported.**

---

# 7. Evolution of learning rules

## 7.1 What the genome should encode

The initial genome should be able to encode:

### Network priors

- node and connection topology;
- base weights and biases;
- activation functions from a bounded enum;
- leak/time constants;
- delays;
- modulatory node and channel types.

### Plasticity placement

- plastic versus fixed connection;
- rule-family ID;
- coefficient-sharing group;
- modulator channel;
- whether the edge reads pre/post activity, derivative, or trace variables allowed by the rule.

### Plasticity parameters

- learning rate;
- eligibility decay;
- Hebbian/Oja/generalized-local coefficients;
- weight-delta decay;
- minimum/maximum effective weight;
- update threshold or deadband;
- homeostatic target where supported;
- sensitivity and sign for each allowed modulator.

### Developmental parameters, later

- module duplication propensity;
- developmental instruction parameters;
- growth rounds and resource budgets;
- cell identity or coordinate signals;
- structural-plasticity thresholds.

The genome should **not** initially encode arbitrary bytecode, unbounded expressions, heap allocation, recursion, or calls into engine state. Those features make validation, determinism, and exploit resistance unnecessarily difficult.

**Recommendation strength: Strongly supported.**

## 7.2 Initial weights and plasticity are complementary

There are three broad strategies:

1. **Evolve initial weights only; plasticity adapts them.** This gives inherited priors and rapid lifetime adjustment.
2. **Start from random weights; evolve only learning rules.** This maximizes developmental self-organization but can require many plasticity parameters and may sacrifice initial competence. [R45]
3. **Evolve both initial weights and learning rules.** This is the most flexible and biologically analogous high-level pattern, but it increases genetic search dimension.

Genesis should begin with strategy 3 while using coefficient sharing. An organism can be competent at birth and still adapt. Random-at-birth weights should be an explicit experiment, not a default assumption.

**Recommendation strength: Supported in narrower conditions.**

## 7.3 A safe local-rule palette

A versioned palette might include:

```text
Rule 0: Fixed
    no synaptic update

Rule 1: HebbianBounded
    e <- x_pre * y_post

Rule 2: OjaBounded
    e <- y_post * (x_pre - y_post * w_eff)

Rule 3: GeneralLocal
    e <- A*x*y + B*x + C*y + D

Rule 4: CovarianceTrace
    e <- (x-x_bar)*(y-y_bar)

Rule 5: ModulatedGeneralLocal
    trace <- lambda*trace + GeneralLocal(...)
    delta <- delta + eta*modulator*trace
```

Every rule uses the same outer safety envelope:

- fixed update phase;
- fixed-point coefficient ranges;
- saturating arithmetic;
- explicit deadband;
- bounded delta;
- optional deterministic decay;
- no access to variables outside its declared inputs.

The palette is less expressive than a free-form evolved equation, but far easier to test. New rules require a new rule-policy version and replay lineage fork.

**Recommendation strength: Strongly supported.**

## 7.4 Local equation evolution

After the palette works, Genesis may evolve a tiny expression tree or stack program over a safe instruction set:

- inputs: `pre`, `post`, `weight`, `trace`, selected modulator, running activity averages;
- arithmetic: saturated add/subtract/multiply, fixed shifts, min/max, absolute value;
- bounded nonlinearities: versioned LUT functions;
- state: a fixed number of scalar registers;
- no loops, dynamic memory, indirect engine access, or variable instruction count beyond a hard cap.

Mutation can replace an instruction, operand, constant, or connection to a state register. Crossover aligns instruction lineage or expression-tree nodes.

Risks:

- programs exploit rounding or saturation artifacts;
- neutral code accumulates;
- equivalent programs have different syntax;
- crossover destroys semantics;
- instruction execution costs more than the neural update itself;
- evolved rules memorize task-specific behavior rather than general learning.

A rule-VM should therefore be a research policy with extensive differential tests, not the first plasticity implementation.

**Recommendation strength: Plausible but unproven.**

## 7.5 Neuromodulation architecture

There are three increasingly endogenous designs:

1. **Engine-provided scalar:** energy delta, pain, or other primitive internal signal gates learning.
2. **Genetically weighted signal mix:** the genome combines several primitive signals into modulatory channels.
3. **Evolved modulatory neurons:** network nodes compute modulators from perception, recurrent state, and internal state.

Begin with 1 and 2. Design 3 becomes more open-ended but can produce self-reinforcing loops in which the network learns to emit its own “reward.” That is not automatically invalid—biological systems have endogenous reinforcement—but it can disconnect learning from adaptive outcomes.

Recommended safeguards:

- distinguish exogenous/internal primitive channels from network-generated channels;
- log all modulatory values and their sources;
- cap magnitude and rate of change;
- allow negative as well as positive modulation;
- include a zero-modulation ablation;
- prevent modulatory nodes from directly accessing post hoc reproductive fitness unless explicitly intended.

**Recommendation strength: Supported in narrower conditions.**

## 7.6 Eligibility traces

Eligibility traces bridge the delay between local activity and later consequence. Each plastic edge can store one trace, or traces can be shared by neuron/module to reduce memory.

Per-synapse traces provide precision but cost `O(P)` memory for `P` plastic edges. Neuron-level traces cost `O(N)` but cannot distinguish which incoming association caused the outcome. Module-level traces are an intermediate design.

Recommended order:

1. per-synapse trace for small controllers to establish capability;
2. compare neuron-shared and module-shared traces;
3. retain the smallest representation that does not materially reduce adaptation.

Trace decay should use a fixed coefficient, not a wall-clock duration or platform exponential. If a half-life interface is exposed to the genome, compile it through a versioned deterministic lookup table.

**Recommendation strength: Strongly supported.**

## 7.7 Synapse-specific versus neuron-specific rules

| Granularity | Evolvability | Genome size | Runtime | Interpretability | Recommended use |
|---|---:|---:|---:|---:|---|
| Global | Low-to-medium | Minimal | Minimal | High | First baseline |
| Connection class | Medium | Low | Minimal | High | Preferred early production |
| Module | Medium-to-high | Low-to-medium | Minimal | Medium-high | After module support |
| Postsynaptic neuron | High | Medium | Minimal | Medium | Useful experiment |
| Synapse | Very high nominally | High | Minimal-to-medium | Low | Sparse exceptions or research only |

“More evolvable parameters” does not necessarily mean better evolvability. A huge per-synapse rule genome can reduce mutational locality and create competing conventions as severe as directly evolved weights.

**Recommendation strength: Strongly supported.**

## 7.8 Learning-rate and stability meta-parameters

The genome may encode learning rates, but unrestricted rates create immediate saturation. Use a bounded representation such as a small signed fixed-point mantissa plus discrete exponent class. Mutation can step locally among representable values.

Recommended evolved meta-parameters:

- learning rate;
- trace decay;
- weight-delta decay;
- update deadband;
- modulator gain;
- homeostatic target;
- plasticity onset/critical-period window;
- fast versus slow plastic component ratio, later.

A critical-period parameter is especially relevant: organisms may learn rapidly early in life and consolidate later. This could reduce forgetting but also lock in accidents. It should be an experimental parameter, not a default biological assumption.

## 7.9 No hidden global error requirement

Hebbian, Oja, STDP, and neuromodulated local rules do not require a vector-valued global error or weight transport. A scalar modulator can shape learning without backpropagation. This aligns with Genesis’s goal of embodied adaptation rather than supervised task training.

Backpropagation may remain useful outside the organisms for:

- fitting a diagnostic model;
- constructing a comparison baseline;
- testing whether a controller class has sufficient representational capacity;
- optimizing a plasticity rule in an offline research branch.

It should not be introduced merely because it is conventional in machine learning.

**Recommendation strength: Strongly supported.**

---

# 8. Interaction between genes and learning

## 8.1 The Baldwin effect

The Baldwin effect describes how lifetime learning can change evolutionary outcomes even when acquired traits are not inherited. Learning allows individuals with imperfect inherited behavior to reach higher fitness, changing which genotypes survive and reproduce. Hinton and Nowlan’s influential computational model illustrated how learning can guide evolution through a difficult search space. [R51–R52]

The effect is not magic transmission of learned weights. It operates because genotypes differ in:

- the range of behaviors they can learn;
- the speed and cost of learning;
- their initial priors;
- their sensitivity to environmental signals;
- their probability of discovering an adaptive behavior during life.

For Genesis, a Baldwin effect is present if plastic organisms cause genetic evolution to move toward genotypes that more reliably or cheaply acquire useful behavior, without acquired state being copied.

**Finding: Strongly supported as a possible evolutionary mechanism; its direction and magnitude are conditional.**

## 8.2 How learning changes the fitness landscape

Learning maps a birth genotype to a distribution of post-learning phenotypes. This can:

- smooth a rugged landscape because nearby genotypes can learn toward similar competent behavior;
- enlarge the basin around a useful strategy;
- expose genetic variants that would otherwise die before expressing a behavior;
- hide genetic differences because learning compensates for them;
- introduce stochasticity if learning outcomes depend on experience;
- create new local optima that exploit the learning process rather than the environment.

A useful formal distinction is:

\[
F(g)=\mathbb{E}_{e,\xi}[\text{lifetime reproductive success}(\operatorname{Learn}(g,e,\xi))] - C_{learn}(g),
\]

where `e` is lifetime experience and `xi` is any keyed stochastic variation. Even under deterministic simulation, the distribution arises across different ecological contexts and world seeds.

Genesis should measure both newborn performance and post-experience performance. A single lifetime fitness number cannot reveal whether evolution improved priors, learning speed, or final competence.

## 8.3 Genetic assimilation

Waddington’s genetic-assimilation experiments showed that a phenotype initially produced under an environmental perturbation could become expressed without that perturbation after selection. [R53–R54] In computational terms, behavior that initially requires learning may become increasingly specified by inherited structure or initial weights.

Assimilation is favored when:

- the environment is stable across generations;
- the learned target recurs reliably;
- learning has time, energy, or error costs;
- early-life competence affects survival;
- the inherited representation can encode the behavior compactly;
- plasticity exposes a selectable path toward the behavior.

Assimilation is weakened when:

- environments vary unpredictably;
- multiple behaviors are useful in different contexts;
- learning is cheap and reliable;
- inherited specialization creates large mismatch costs;
- social information changes faster than genes.

**Finding: Supported in narrower conditions.** Genetic assimilation should be measured, not presumed.

## 8.4 When lifetime learning reduces selection pressure

Plasticity can “shield” deleterious or mediocre genotypes by compensating during life. Consequences include:

- slower improvement in newborn performance;
- maintenance of higher genetic diversity;
- reduced specialization;
- accumulation of cryptic genetic variation;
- dependence on environmental opportunities to learn;
- catastrophic failure when learning cues disappear.

This is not necessarily harmful. In variable environments, maintaining plastic generalists can be adaptive. The correct criterion is not whether genes become less optimized, but whether lifetime and lineage-level performance improve under the actual environmental distribution.

A Genesis experiment should compare:

- genetic fitness variance;
- newborn behavioral variance;
- post-learning variance;
- reaction norms across environments;
- survival when learning is suddenly disabled;
- adaptation after environment change.

**Recommendation strength: Strongly supported as a risk and measurement requirement.**

## 8.5 When plasticity improves adaptation

Plasticity is most likely to help when:

- relevant conditions vary within a lifetime;
- the organism can observe informative cues;
- the same genotype experiences multiple contexts;
- damage or body changes require compensation;
- social or environmental information is newer than the genome;
- the cost of a fixed wrong behavior is high;
- a compact local rule can exploit recurring causal structure.

Evolutionary robotics has repeatedly demonstrated online adaptation and self-organization under bounded task conditions. [R41, R45–R50]

Plasticity may not help when:

- lifetimes are too short to recover learning cost;
- feedback is delayed beyond retained eligibility;
- the environment is stable and behavior is cheaply inherited;
- cues are noisy or adversarial;
- the controller’s capacity is too small;
- mutation continually changes the substrate faster than learning rules can exploit it.

**Finding: Supported in narrower conditions.**

## 8.6 When plasticity prevents stable specialization

High plasticity can keep behavior context-sensitive when a stable specialist would perform better. It can also make selection favor broad but mediocre learning ability over high-performing innate structure. Mechanisms that can restore specialization include:

- explicit learning cost;
- limited lifetime;
- critical periods;
- lower plastic-edge fraction;
- stable niches;
- inherited base weights close to the learned attractor;
- modular plasticity confined to context-dependent pathways.

Do not force specialization as a design goal. Instead, create environments where specialization and plasticity have measurable tradeoffs and observe which evolves.

## 8.7 Inherited priors and learned behavior

The most useful conceptual model is not “genes versus learning,” but **genes specifying a prior and an update process**.

Genes can encode:

- what is sensed;
- which pathways exist;
- initial action biases;
- which synapses are plastic;
- which correlations matter;
- what internal outcomes modulate change;
- how long memory lasts;
- how quickly weights saturate or decay.

Learning then updates the organism within this inherited hypothesis space. A genome with excellent plasticity but no useful sensory distinction cannot learn the missing relation. A genome with perfect initial behavior may not need learning until the environment changes.

Genesis should log separate contributions:

- base-weight output;
- plastic-delta contribution;
- recurrent-state contribution;
- modulatory activity;
- developmental changes, later.

## 8.8 Lamarckian inheritance models

A Lamarckian evolutionary algorithm copies some acquired solution state back into the heritable representation. Possible Genesis policies include:

1. **Full weight write-back:** offspring base weights begin from the parent’s final effective weights.
2. **Partial interpolation:** offspring base weight is a fixed fraction between inherited base and parental learned value.
3. **Germline snapshot:** write back only state acquired before a reproductive cutoff.
4. **Selected consolidation:** copy only learned changes that exceed a persistence or confidence threshold.
5. **Rule assimilation:** do not copy weights; mutate inherited priors toward repeatedly learned states through a separate evolutionary operator.

Advantages:

- rapid transfer of successful lifetime adaptation;
- fewer generations to retain a repeatedly useful solution;
- useful in stable optimization problems.

Risks:

- overfitting to one parent’s idiosyncratic experience;
- reduced genetic diversity;
- propagation of transient errors;
- weaker separation between evaluation and inheritance;
- harder lineage interpretation;
- acquired state size can dominate genome size;
- parent age and experience become inheritance biases;
- changing environments can make inherited acquired state maladaptive.

Sasaki and Tokoro’s changing-environment comparison found that lower or absent inheritance of acquired characters could be more stable and adaptable, illustrating that Lamarckian advantage is conditional. [R57] Classical evolutionary-computation comparisons likewise distinguish faster local optimization from long-run robustness. [R58]

**Recommendation strength: Strongly supported to exclude from the initial default; supported in narrower conditions as a later experiment.**

## 8.9 Cultural behavior and genetic assimilation

Culturally transmitted behavior can be genetically assimilated if:

- the behavior recurs across many generations;
- learners reliably observe it;
- learning has a cost or delay;
- genetic variants approximate it at birth;
- the ecological payoff is stable;
- cultural availability itself is sufficiently predictable.

It may remain cultural when:

- behaviors change faster than genetic evolution;
- different groups maintain different traditions in the same genetic background;
- local environments favor different variants;
- social learning is cheap;
- copying provides access to cumulative information unavailable to individual genes.

It may also produce **gene–culture coevolution**: genes evolve attention, imitation, teaching, conformity, or selectivity rather than directly encoding the behavior. Artificial-life models show that social learning and non-genetic evolutionary dynamics can arise under specific transmission conditions, but the result depends heavily on how imitation and behavioral variants are represented. [R67]

For Genesis, the plausible early signal of gene–culture interaction is not a named “culture gene.” It is a heritable change in observation, memory, learning rate, demonstrator preference, or action-copying success that correlates with persistent group-specific behaviors.

**Recommendation strength: Plausible but unproven.**

## 8.10 Distinguishing evolved, learned, and socially transmitted behavior

Use a factorial intervention suite:

### Newborn common-garden test

Place genetically identical newborns in a standardized environment with no demonstrators. Behavior present immediately is primarily inherited or generated by initial recurrent dynamics.

### Plasticity-off clone

Clone the genome and initial state, but set learning rates to zero. Differences from the normal clone quantify synaptic plasticity effects.

### Learned-state reset

Take an experienced organism, preserve genome and body, reset only learned weights/traces. Loss of behavior indicates acquired synaptic dependence.

### Recurrent-state reset

Reset neuron activations and delay buffers without changing learned weights. This separates short-term dynamical memory from synaptic learning.

### Cross-fostering

Raise the same genotype with different demonstrator groups. Group-correlated divergence indicates social transmission or environmental scaffolding.

### Demonstrator shuffle

Preserve the amount of observed motion and reward but break the relationship between action, agent, and outcome. If learning remains, the mechanism may be nonspecific arousal or environmental correlation rather than social information.

### Observation occlusion

Block observation while preserving environmental outcomes. This distinguishes direct environmental learning from social observation.

### Genotype swap / reaction norm

Evaluate multiple genotypes across the same learning histories and multiple histories for the same genotype.

### Transmission-chain test

Remove original demonstrators after one generation and test whether behavior persists across learner-to-learner transmission with measurable mutation and fidelity.

### Counterfactual replay

Replay the same organism history with plasticity updates disabled or alternative modulators in an analysis-only branch. Do not feed post hoc labels back into the live simulation.

**Recommendation strength: Strongly supported.**

# 9. Architecture comparison matrix

## 9.1 Rating legend

- **High** — favorable for the criterion.
- **Medium** — workable with meaningful qualifications.
- **Low** — unfavorable or expensive for the criterion.
- **Variable** — dominated by design details rather than family identity.

“Long-horizon potential” means capacity to support increasing behavioral complexity in principle, not evidence that open-ended evolution will occur.

## 9.2 Engineering and scaling matrix

| Architecture family | Structural evolvability | Crossover feasibility | Deterministic implementation | Compute cost | Save-state complexity | Thousands of organisms |
|---|---|---|---|---|---|---|
| **1. Fixed topology, evolved weights** | Low: structure is an experimenter choice | High: positional alignment | High | Low-to-medium; dense fixed arrays vectorize well | Low without plasticity; medium with recurrent/plastic state | High if network kept small |
| **2. NEAT-style direct graph** | High for incremental node/edge change | Medium: historical alignment helps; duplication remains difficult | High if stable IDs, canonical compile, and explicit delays are used | Low-to-medium for small sparse graphs; graph overhead matters | Medium; topology plus runtime state | High with hard budgets and compiled sparse arrays |
| **3. HyperNEAT / CPPN indirect** | Medium-to-high for coherent geometric pattern change | Medium at generator level; weak correspondence at expressed-edge level | Medium: generator is simple, substrate generation/thresholds add policy | Low genome cost; phenotype evaluation may be high | Medium-to-high when expressed plastic state is large | Medium; good only if generated phenotype remains bounded |
| **4. Developmental / grammatical encoding** | Potentially high, but mutation effects are often nonlocal | Low-to-medium; syntax can align while phenotype does not | Low-to-medium; interpreter and growth ordering are extensive semantics | Variable, often high due to development | High: genome, developed graph, and possibly developmental state | Low initially; possible later with tightly bounded development |
| **5. Direct graph with module duplication** | High for reuse and divergence | Medium; requires copy-aware homology and interface closure | Medium-to-high | Medium; duplication can cause step changes in size | Medium | Medium-to-high with module and total budgets |
| **6. Large fixed network with evolved sparse mask** | Medium: effective topology varies within a fixed ceiling | High by array position | Very high | Medium; sparse execution or dense masked execution tradeoff | Medium; full mask plus learned state | High if ceiling is modest and memory preallocated |
| **7. Recurrent Cartesian Genetic Programming ANN** | Medium-to-high within fixed genotype/grid bounds | Low-to-medium; often mutation-centric rather than crossover-centric | High: fixed-length decode and explicit positional rules | Low-to-medium; inactive genes add decode overhead but not execution | Medium | High with bounded grid; good experimental comparator |
| **8. Hybrid CPPN initialization plus direct refinement** | High: regular base plus local exceptions | Medium-to-low; two representations must be reconciled | Medium | Medium | High if both generator and overrides are hereditary and learned | Medium |

## 9.3 Scientific and behavioral matrix

| Architecture family | Modularity | Suitability for lifetime learning | Long-horizon potential | Research maturity | Characteristic failure modes |
|---|---|---|---|---|---|
| **1. Fixed topology, evolved weights** | Low-to-medium; can evolve functional modules but no structural operator supports them | High; stable indexed synapses simplify plasticity | Medium; ceiling and fixed organization can become limiting | High | Oversized search space, fixed ceiling, inactive baggage, experimenter-imposed inductive bias |
| **2. NEAT-style direct graph** | Medium by default; higher with cost, duplication, or module operators | High; plasticity metadata attaches directly to edges | High in principle for incremental structural growth | High | Bloat, species fragmentation, destructive crossover, poorly tuned innovations, direct-encoding size |
| **3. HyperNEAT / CPPN indirect** | Medium-to-high for regular modules; not automatic functional modularity | Medium; initial plasticity can be generated, but learned state is expressed-scale | High where geometry/repetition remains useful | High for geometric neuroevolution | Coordinate bias, global pleiotropy, inability to express irregular exceptions, threshold sensitivity |
| **4. Developmental / grammatical encoding** | Potentially high through reuse and lineage | Medium-to-high in principle | Very high in principle | Medium historically; low for modern lifelong developmental controllers | Developmental explosion, symmetry collapse, poor mutation locality, opaque crossover, interpreter exploitation |
| **5. Direct graph with module duplication** | High potential if duplicated subgraphs are functional | High | High; reuse and divergence are plausible routes to complexity | Medium | Arbitrary module boundaries, duplicated interference, rapid bloat, copy-homology ambiguity |
| **6. Large fixed sparse mask** | Medium; masks can segregate pathways | High | Medium-to-high within ceiling | High in sparse-network research; medium in open-ended ALife | Ceiling saturation, neutral inactive parameters, positional homology unrelated to function |
| **7. Recurrent Cartesian GP ANN** | Medium; repeated computational genes possible but positional | Medium-to-high | Medium-to-high within genotype bounds | Medium | Inactive-code bloat, positional bias, weak crossover, sensitivity to level-back/grid choices |
| **8. Hybrid CPPN + direct refinement** | High potential for regular modules plus exceptions | Medium-to-high | High | Medium | Representation conflict, duplicate sources of truth, complex mutation accounting, larger save schema |

## 9.4 Decision interpretation

### Best production foundation

**NEAT-style direct graph** is the strongest overall fit because it offers the most balanced combination of structural evolution, learning support, deterministic implementation, auditability, and research maturity.

### Best low-risk baseline

**Large fixed network with a sparse mask** is the most important comparator. It can represent variable effective topology while retaining fixed memory and positional crossover. If it matches the direct graph on adaptation and complexity at lower cost, the burden of proof remains on graph evolution.

### Best indirect research branch

**HyperNEAT/CPPN** is the first indirect branch to test because its assumptions are explicit and its evidence base is much stronger than general developmental grammars.

### Best modular extension

**Direct module duplication** should precede a full developmental encoding.

### Best alternative encoding comparator

**Recurrent Cartesian Genetic Programming** is credible because it provides bounded variable structure, neutral inactive genes, and deterministic decoding without NEAT’s historical-marker machinery. [R63–R64]

### Highest-upside, highest-risk branch

**Developmental/NDP architecture** has the greatest compact-generative ambition but the weakest current case for immediate deployment in a deterministic many-organism engine.

---

# 10. Recommended architecture for Genesis

## 10.1 Recommendation status

**Overall recommendation: Strongly supported.**  
**Open-ended-complexity outcome: Plausible but unproven.**  
**Specific mutation rates, species policy, and plasticity coefficients: empirical open questions.**

The architecture should be named and versioned as a controller policy, for example `ControllerPolicy::DirectPlasticGraphV1`. The name must denote semantics, not a claim that it is literally NEAT.

## 10.2 Design goals

The policy should provide:

- variable node and connection count;
- deterministic structural mutation and crossover;
- direct historical auditability;
- recurrent state and explicit temporal delays;
- fixed and plastic synapses in the same network;
- modulatory neurons/channels;
- bounded online local learning;
- exact checkpoint/restore;
- compiled sparse evaluation;
- stable behavior across supported platforms;
- later extension to module duplication and indirect initialization.

## 10.3 Non-goals for the first policy

- arbitrary evolved learning code;
- spiking neurons or millisecond STDP;
- lifetime topology mutation;
- developmental growth;
- Lamarckian write-back;
- unbounded network size;
- hidden use of post hoc fitness as a neural error signal;
- an LLM or any external generative model as organism decision engine;
- silent policy migration.

## 10.4 Genome header

A canonical genome begins with a fixed header:

```rust
struct ControllerGenomeHeaderV1 {
    schema_version: u32,
    controller_policy_version: u32,
    mutation_policy_version: u32,
    crossover_policy_version: u32,
    numeric_policy_version: u32,
    activation_policy_version: u32,
    plasticity_policy_version: u32,
    id_hash_policy_version: u32,

    genome_lineage_id: Id128,
    birth_event_id: Id128,

    max_nodes: u16,
    max_enabled_edges: u16,
    max_total_gene_records: u16,
    max_plastic_edges: u16,
    max_delay_ticks: u8,

    flags: GenomeFlagsV1,
}
```

The caps may be world-configured rather than individually evolvable at first. If organisms can evolve resource allocation to controller size later, the globally enforced safety maximum remains separate from the organism-visible allocation.

## 10.5 Node gene

```rust
struct NodeGeneV1 {
    node_id: Id128,
    gene_lineage_id: Id128,
    homology_class_id: Id128,
    origin_event_id: Id128,

    role: NodeRoleV1,          // sensor, hidden, modulator, output, bias
    activation: ActivationV1, // bounded enum
    enabled: bool,

    bias: ScalarV1,
    leak: UnitScalarV1,
    response_gain: ScalarV1,
    intrinsic_state_seed: u32, // reserved/optional; no hidden RNG use

    module_instance_id: Option<Id128>,
    regulatory_flags: NodeFlagsV1,
}
```

Sensors and outputs should bind to capability IDs, not array positions inferred from insertion order. A sensor binding such as `SensorCapabilityId::VisionSector(3)` must have a stable canonical representation. Missing capabilities on load or reproduction should be resolved by an explicit genome/capability compatibility policy.

## 10.6 Connection gene

```rust
struct ConnectionGeneV1 {
    connection_id: Id128,
    gene_lineage_id: Id128,
    homology_class_id: Id128,
    origin_event_id: Id128,

    src_node_id: Id128,
    dst_node_id: Id128,
    enabled: bool,
    delay_ticks: u8,

    base_weight: ScalarV1,
    plasticity_rule: PlasticityRuleIdV1,
    plasticity_group_id: Id128,
    plasticity: PlasticityParamsV1,

    module_instance_id: Option<Id128>,
    connection_flags: ConnectionFlagsV1,
}
```

Connection IDs identify records. The structural signature used by validators and homology derivation is a canonical tuple, not the connection ID itself.

## 10.7 Plasticity parameter block

```rust
struct PlasticityParamsV1 {
    learning_rate: LearningRateV1,
    trace_decay: UnitScalarV1,
    weight_delta_decay: UnitScalarV1,

    coeff_a: ScalarV1,
    coeff_b: ScalarV1,
    coeff_c: ScalarV1,
    coeff_d: ScalarV1,

    modulator_channel: ModulatorChannelIdV1,
    modulator_gain: ScalarV1,
    update_deadband: UnitScalarV1,

    min_delta: ScalarV1,
    max_delta: ScalarV1,
    min_effective_weight: ScalarV1,
    max_effective_weight: ScalarV1,
}
```

To reduce genome size, multiple edges may point to a canonical shared plasticity-group record. Mutation may duplicate a group before changing it, analogous to copy-on-write. This provides module- or class-level learning rules without repeating all coefficients.

## 10.8 Runtime phenotype

The genome is compiled into densely indexed arrays for execution:

```text
CompiledControllerV1
  node_ids_sorted[]
  active_node_records[]
  edge_ids_sorted[]
  active_edge_records[]
  inbound_zero_delay_ranges[]
  inbound_delayed_ranges[]
  canonical_zero_delay_topological_order[]
  output_binding_order[]
  stable_id_to_runtime_index[]  // serialized or deterministically rebuilt
```

Runtime mutable state is separate:

```text
ControllerStateV1
  node_activation_current[]
  node_activation_next[]
  node_auxiliary_state[]
  delay_ring_values[]
  learned_weight_delta_by_edge[]
  eligibility_trace_by_edge[]
  modulator_values[]
  pending_modulator_values[]
  controller_tick_counter
  update_phase
```

The compiler may omit disabled and output-irrelevant dormant genes from hot arrays while retaining them in the genome. Compilation output must be deterministic from canonical genome bytes and compiler policy version.

## 10.9 Tick semantics

Recommended full-world tick sequence:

1. **World snapshot:** freeze all organism-visible state for tick `t`.
2. **Sensor sampling:** derive each organism’s sensor vector from the snapshot in canonical sensor-ID order.
3. **Controller forward phase:** evaluate every organism using current sensor values, prior delayed state, and current learned weights.
4. **Action proposal:** emit bounded action records tagged with organism and action-channel IDs.
5. **Canonical action resolution:** sort proposals by world-defined conflict keys and commit to obtain world state `t+1`.
6. **Outcome derivation:** compute organism-visible internal changes and primitive modulatory signals from the transition.
7. **Eligibility/plasticity phase:** update traces and learned weight deltas using the versioned temporal convention.
8. **State latch:** commit next recurrent activations and delay rings.
9. **Checksum boundary:** hash canonical state; a checkpoint is safe here.

An alternative can update eligibility during phase 3 and apply the modulator during phase 7. The exact convention matters. It must be fixed and tested against one-tick reward delay cases.

**Recommendation strength: Strongly supported.**

## 10.10 Recurrent semantics

Preferred first policy:

- Sensors are available at tick `t`.
- `delay=0` edges propagate within tick `t` through an acyclic subgraph.
- `delay=k` edges read the source activation from exactly `k` completed controller ticks earlier.
- Self-connections require `delay>=1`.
- A recurrent cycle must contain at least one delayed edge.
- Output actions are read only after the canonical zero-delay evaluation completes.

This combines immediate sensor-to-action pathways with explicit memory. It is more complex than making every edge delayed but avoids an unnecessary one-tick latency for feedforward reflexes.

The graph compiler detects zero-delay cycles using canonical Kahn topological sorting. Among all ready nodes, select the smallest stable node ID. If not all active nodes are emitted, compilation fails with a zero-delay-cycle error.

## 10.11 Activation policy

Initial bounded activation set:

- identity with saturation;
- hard tanh or piecewise-linear symmetric activation;
- deterministic sigmoid/tanh lookup table if needed;
- threshold/step;
- optional sinusoid only if implemented by a versioned lookup table.

Do not call platform `tanh`, `exp`, `sin`, or `powf` in a strict cross-platform policy. Rust documents that several floating-point methods may vary in precision across platforms and versions. [R71]

Mutating activation functions should begin disabled or very low-rate. It increases structural diversity but also changes scale and can destabilize learned coefficients.

## 10.12 Structural mutation operators

### Weight and parameter mutation

- perturb base weight;
- reset base weight from a bounded distribution;
- perturb bias/leak/response;
- perturb plasticity coefficient;
- toggle plasticity fixed/plastic;
- change plasticity group through duplication or reassignment.

### Add connection

- build canonical list of legal `(src, dst, delay, type)` candidates;
- select by keyed uniform index;
- derive event, lineage, and homology IDs;
- initialize weight and plasticity using separate named draws;
- validate budget and commit atomically.

### Split connection / add node

- select enabled connection canonically;
- disable it;
- add a new hidden node whose homology derives from the split ancestral edge;
- add source-to-new and new-to-destination edges;
- initialize the first edge near identity and second near the original effective base relationship under a versioned rule;
- do not copy the parent’s acquired weight delta into the child genome under Darwinian reproduction.

### Disable/enable connection

- select from canonical enabled/disabled candidate lists;
- apply no-op if list empty;
- log event and realized outcome.

### Delete connection

- remove one eligible record or mark a persistent tombstone according to the genome-history policy;
- ensure outputs remain structurally valid if that is a policy invariant.

### Prune node

- select an eligible hidden node;
- remove it and incident edges atomically;
- reject if required output reachability would be lost under the configured rule.

### Duplicate module, later

Use the design in Section 5.3 only after basic mutations pass long-run tests.

## 10.13 Crossover policy

Recommended initial crossover:

1. Canonically order parents by a tuple such as selection rank, then genome lineage ID. Parent order must not depend on call order.
2. Build sorted maps from homology class to alleles.
3. For one-to-one matches, choose allele fields with named draws or a defined blend for numeric values.
4. For duplicate alleles within a homology class, align exact lineage first; unresolved copies are treated as disjoint.
5. Inherit disjoint/excess genes using an explicit rule based on parent selection rank, not floating comparison accidents.
6. Apply disabled-gene inheritance with one named draw per matching gene keyed by homology ID.
7. Add all required endpoint nodes.
8. Compile and validate.
9. If invalid, apply only the documented deterministic repair sequence. Otherwise reject the crossover offspring and fall back to a versioned asexual-copy policy.

The fallback itself must be keyed and logged; repeated retries until success create variable draw consumption and hidden selection pressure.

**Recommendation strength: Supported in narrower conditions.** Sexual recombination is useful but not mandatory. An asexual mutation-only baseline is required.

## 10.14 Protection of structural innovation

Original NEAT uses speciation and fitness sharing. Genesis can start with a simplified explicit mechanism:

- compatibility distance based on unmatched homology, matched parameter difference, and optionally module difference;
- deterministic species representatives chosen by stable rank and ID;
- new structural genes receive an age/probation counter;
- offspring allocation uses fixed-point adjusted fitness and canonical tie-breaking;
- species threshold adaptation is versioned and deterministic.

Alternative or complementary mechanisms:

- lineage-local competition;
- age-layered populations;
- island/world demes;
- novelty or QD archives;
- structural innovation bonuses used only in experiments.

Ablate the protection mechanism. If new useful structures survive equally without it, the complexity is not justified.

## 10.15 Complexity policy

Use three independent limits:

### Engine safety cap

A hard non-evolvable maximum that protects memory and tick budgets.

### Heritable allocation

A future organism may evolve how much energetic/resource budget it devotes to its controller, within the safety cap.

### Selection cost

An explicit measured cost used by experiments or world energetics. Do not hide an arbitrary complexity penalty inside controller fitness reporting.

The first policy can use a lexicographic selection comparison:

1. higher primary reproductive outcome;
2. if within a versioned equivalence band, lower active edge operations;
3. then lower active node count;
4. then stable genome ID.

This avoids choosing an arbitrary scalar coefficient before cost tradeoffs are understood.

## 10.16 Recommended staged feature set

| Stage | Features | Evidence status |
|---|---|---|
| V2-A | Direct graph, fixed weights, discrete recurrence, add/disable/delete, asexual reproduction | Strongly supported |
| V2-B | Historical crossover and innovation protection | Strongly supported, implementation-specific |
| V2-C | One bounded fixed plasticity rule, exact learned-state save | Supported in narrower conditions |
| V2-D | Evolved shared plasticity coefficients and primitive modulators | Supported in narrower conditions |
| V2-E | Evolved modulatory neurons, multiple rule groups | Plausible but unproven |
| V2-F | Module duplication and module-level crossover | Supported in narrower conditions |
| Research H | CPPN/HyperNEAT or hybrid initialization | Supported only when geometry is useful |
| Research D | NDP/developmental growth | Plausible but unproven |
| Research L | Lamarckian write-back | Supported only in narrow stable conditions |
| Research S | Lifetime structural plasticity | Speculative for Genesis production |

---

# 11. Deterministic mutation and evaluation design

## 11.1 Canonical graph serialization

Canonical serialization is a semantic contract, not whatever ordering a serializer happens to emit.

Recommended byte order:

1. fixed magic and schema header;
2. policy versions in fixed-width little-endian integers;
3. genome identity and caps;
4. plasticity-group records sorted by group ID;
5. node genes sorted by `node_id` bytes;
6. connection genes sorted by `connection_id` bytes;
7. optional tombstones sorted by tombstone ID;
8. trailing integrity checksum over all previous bytes.

For every enum, serialize an explicit numeric discriminant. For optional fields, serialize a presence byte followed by the canonical value. Do not serialize Rust struct memory, padding, enum layout, `usize`, pointer values, or platform-native endianness.

Unknown required fields or unsupported policy versions fail closed. Extension fields can use length-delimited tagged records only if skip/retain semantics are specified.

**Recommendation strength: Strongly supported.**

## 11.2 Data structures in Rust

Use hash maps only as noncanonical internal indexes. Rust’s default `HashMap` is randomly seeded and does not promise a stable hashing algorithm or canonical iteration order. [R69] Safe patterns are:

- store canonical records in sorted `Vec`s;
- use `BTreeMap`/`BTreeSet` where ordered iteration is required; Rust documents key-order iteration for `BTreeMap`; [R70]
- if a `HashMap` is used for lookup, never derive serialization, mutation candidates, tie-breaking, checksums, or RNG draw sequence from its iteration order;
- convert any discovered set to a stable sorted vector before a semantically observable operation.

Do not rely on current incidental order in tests. Add tests that deliberately randomize insertion order.

## 11.3 Stable identifiers

Use fixed-width IDs, preferably 128 bits internally for hot records and 256-bit preimage/hash evidence where collision diagnosis is needed.

ID domains must be separated:

```text
"genesis.world-lineage.v1"
"genesis.reproduction-event.v1"
"genesis.mutation-event.v1"
"genesis.node-lineage.v1"
"genesis.edge-lineage.v1"
"genesis.node-homology.v1"
"genesis.edge-homology.v1"
"genesis.module-instance.v1"
```

Each hash invocation encodes field lengths and fixed byte order. Concatenating variable-length strings without lengths is unsafe. The ID-hash algorithm and truncation rule are policy-versioned and accompanied by fixed test vectors.

Do not reuse organism IDs as gene IDs or innovation IDs. Identity scopes must remain explicit.

## 11.4 Named keyed RNG streams

Every random draw belongs to a semantic stream. Example names:

```text
reproduction.parent_selection
reproduction.crossover.matching_allele
reproduction.crossover.disabled_inheritance
mutation.operator_selection
mutation.add_edge.target
mutation.add_edge.weight_init
mutation.split_edge.target
mutation.parameter.edge_weight
mutation.parameter.plasticity_rate
mutation.delete_edge.target
controller.exploration.action_noise
controller.development.cell_action    // future
```

A draw key includes all identity needed to make the result independent of execution order:

```text
RNGKey {
    world_lineage_id,
    policy_version,
    event_id,
    stream_name_id,
    subject_id,
    draw_ordinal,
}
```

Prefer a counter-based or splittable deterministic RNG whose output is a pure function of key and counter. If a stateful generator is used, serialize its state per named stream and never share it across organisms or operators.

Changing how many draws one operator consumes must not shift every later operator. Separate streams and explicit ordinals prevent that cascade.

**Recommendation strength: Strongly supported.**

## 11.5 Separating event identity from traversal order

A common bug is:

```text
for edge in hash_map_edges {
    if rng.next() < p { mutate(edge); }
}
```

This makes mutation depend on container order and causes draw-shift cascades when edges are inserted or removed.

Use either:

### One target per event

```text
candidates = canonical_legal_candidates(genome)
if candidates.is_empty(): no_op
else:
    index = keyed_uniform(event_id, "target", candidates.len())
    mutate(candidates[index])
```

### Independent Bernoulli by subject

```text
for subject in stable_sorted_subjects:
    draw = keyed_u64(event_id, "bernoulli", subject.id, 0)
    if draw < threshold: mutate(subject)
```

The second gives each gene an independent stable draw and remains invariant if unrelated genes are inserted. It changes the number of realized mutations with genome size, so that scaling must be intentional.

## 11.6 Uniform bounded sampling

Do not compute `random_u64 % n` unless modulo bias is accepted and versioned. Use a deterministic rejection or multiply-high method with fixed semantics. Rejection draw ordinals belong to the same target event and are bounded or mathematically guaranteed. Include test vectors for edge values of `n`.

For continuous mutation, avoid platform distributions such as a library normal sampler whose algorithm may change. Use a versioned integer-domain distribution or lookup table. Store the distribution policy ID.

## 11.7 Deterministic structural mutation algorithm

Example add-edge procedure:

```text
fn mutate_add_edge(parent, reproduction_event): Result<Genome, MutationOutcome> {
    event = derive_event("add_edge", reproduction_event, invocation_ordinal)

    candidates = enumerate_legal_edges(parent)
    stable_sort(candidates, by=(src_id, dst_id, delay, connection_type))

    if candidates.is_empty():
        return NoOp(NoLegalCandidate, event)

    target = candidates[keyed_uniform(event, "target", len(candidates))]

    weight = sample_weight(key=(event, "weight_init"))
    rule = sample_or_inherit_rule(key=(event, "plasticity_init"))

    edge = construct_edge(
        connection_id = derive_id("edge-lineage", event),
        homology_class_id = derive_edge_homology(target),
        origin_event_id = event,
        ...
    )

    child = parent.with_edge(edge)
    validate(child)
    return Applied(child, event)
}
```

This procedure performs no container-order-dependent retry. If enumerating all pairs becomes too costly at larger caps, implement a deterministic indexed candidate space with a proof that invalid exclusions and sampling remain stable; do not silently replace it with random trial-and-error.

## 11.8 Deterministic crossover

Crossover must be a pure function of:

- canonical parent genomes;
- canonical parent selection metadata;
- reproduction event ID;
- crossover policy version.

All matching-gene decisions are keyed by homology ID, not consumed sequentially. Thus, adding an unrelated gene does not change allele choices for existing matches.

Example:

```text
allele_choice = keyed_bit(
    reproduction_event_id,
    "crossover.matching_allele",
    homology_class_id
)
```

Numeric blending should use fixed-point arithmetic and a keyed blend coefficient. Parent equality and ties use stable ID order.

## 11.9 Canonical graph compilation

Compiler pipeline:

1. validate canonical gene ordering and IDs;
2. filter enabled nodes and connections;
3. verify endpoints and capability bindings;
4. identify nodes reachable from sensors and able to reach outputs;
5. materialize active intersection;
6. partition zero-delay and delayed edges;
7. topologically sort zero-delay graph with stable-ID tie breaking;
8. sort each node’s inbound edges by connection ID;
9. assign dense runtime indexes in stable node/edge order;
10. allocate delay-ring offsets in stable edge order;
11. emit compiler manifest and hash.

The compiler manifest should include counts, active IDs, dormant IDs, zero-delay order, delay allocation, and policy versions. This is invaluable for replay diagnosis.

## 11.10 Deterministic accumulation

Even integer or fixed-point addition can overflow, so order and width matter.

Recommended rule:

- multiply in a specified widened type;
- accumulate inbound products in connection-ID order into an even wider signed accumulator;
- add bias at a specified point;
- apply one specified rounding operation;
- saturate once to the activation input range;
- apply the versioned activation function.

If the accumulator is mathematically proven not to overflow under configured caps and coefficient bounds, document the proof and assert bounds. Otherwise use checked or saturating behavior explicitly. Never rely on release-mode integer overflow behavior.

Parallel reductions within one neuron should be avoided initially. A tree reduction changes rounding/saturation behavior unless the arithmetic is exactly associative in the chosen widened domain.

## 11.11 Synchronous versus asynchronous neural updates

### Synchronous update

All neurons read a defined old/current state and write a separate next state. This is easy to replay and parallelize.

### Asynchronous update

Neurons update one at a time, and later neurons may see earlier updates. Behavior depends on order. Asynchronous biological inspiration does not justify an unspecified order.

Genesis should use synchronous updates or the explicit zero-delay acyclic schedule described above. If asynchronous dynamics are ever studied, encode a canonical update schedule or keyed event times as part of the controller policy and save its queue.

**Recommendation strength: Strongly supported.**

## 11.12 Numeric policy

Preferred route: fixed-point controller arithmetic compatible with the world’s deterministic numeric philosophy.

A numeric policy must specify:

- storage widths and signedness;
- fractional bits or scaling;
- multiplication widening;
- division and rounding mode;
- saturation/overflow behavior;
- activation input/output domains;
- LUT contents and interpolation;
- trace decay arithmetic;
- weight bounds;
- comparison and deadband semantics;
- conversion from sensors and to actuators.

Do not choose `Q16.16`, `Q8.24`, or another format without range profiling. Recurrent sums and eligibility traces can require more headroom than feedforward weights. A likely implementation uses 32-bit stored scalars, 64-bit products/accumulators, and possibly 128-bit audit/reference arithmetic in tests.

If floating point is retained, define a strict platform set, disable contraction/FMA where necessary, forbid noncanonical NaNs, avoid unspecified library functions, and maintain cross-platform conformance tests. Rust’s `f32` documentation explicitly warns that NaN payload behavior and the accuracy of several mathematical functions are not fully deterministic across platforms or versions. [R71]

**Recommendation strength: Strongly supported.**

## 11.13 Plasticity update determinism

Each plastic edge update writes only its own trace and delta. It reads immutable tick-phase snapshots of:

- pre activation;
- post activation;
- prior trace;
- prior weight delta;
- selected modulator.

This makes edge updates embarrassingly parallel in principle. Nevertheless, the first implementation should update edges in stable order to simplify reference testing. A later parallel path must be bit-compared against the scalar reference.

If homeostasis depends on a neuron-wide norm, compute the norm in canonical edge order, then apply per-edge updates in a second phase. Never update weights in place and let later edges observe earlier weight changes unless that exact asynchronous rule is intentional.

## 11.14 Parallel organism evaluation

Safe world-level parallelism:

1. immutable world snapshot;
2. organisms partitioned arbitrarily among workers;
3. each organism reads only snapshot and its private controller state;
4. each produces action and learning-pretrace records tagged by stable IDs;
5. merge outputs and sort canonically;
6. resolve world interactions deterministically;
7. compute organism outcomes;
8. update each private controller state;
9. checksum in organism-ID order.

Worker partition, completion order, and core count must not appear in any key or tie-breaker.

Conflicting world actions require a domain rule—priority, simultaneous physics, auction, or collision resolution—not “first thread wins.” Controller determinism cannot compensate for nondeterministic world commits.

## 11.15 Cross-platform risks

- floating-point contraction and extended precision;
- different transcendental implementations;
- NaN payload/sign selection;
- SIMD reduction order;
- architecture-dependent integer width through `usize`;
- endianness;
- struct padding and enum layout;
- randomized hash seeds;
- unstable sort tie behavior when comparator returns equality for distinct records;
- dependency upgrades changing RNG/distribution algorithms;
- compiler optimizations around undefined or unchecked overflow;
- parallel work scheduling;
- serialization library changes;
- Unicode or string normalization in stream names if names are serialized as text.

Mitigation: fixed-width binary formats, fixed test vectors, canonical total ordering, vendored or version-pinned algorithms where needed, scalar reference evaluator, and replay testing across every supported target.

## 11.16 Policy-version successors and replay lineage

Controller semantics inevitably evolve. A successor policy may change activation, trace timing, crossover, or numeric format. Do not reinterpret old saved bytes under new semantics.

Use one of:

- retain old interpreters and continue old organisms exactly;
- explicit migration at a world fork, producing new controller lineage IDs and a migration manifest;
- allow reproduction across policy versions only through a documented compatibility bridge that creates a successor genome.

A replay crossing a semantic migration records the migration event and expected pre/post checksums. Silent “bug fixes” that alter controller output invalidate exact historical replay and must create a new policy version.

**Recommendation strength: Strongly supported.**

---

# 12. Save, restore, checksum, and replay requirements

## 12.1 Fundamental rule

> Learned weights cannot be recomputed from the birth genome during restoration.

They depend on the organism’s exact lifetime sequence of perceptions, actions, internal outcomes, modulatory signals, update phases, and numerical saturation. Replaying that entire history is neither a robust checkpoint format nor a bounded restore operation. Store the learned state explicitly.

**Requirement strength: Strongly supported and non-negotiable.**

## 12.2 Controller save-state inventory

For every organism, save:

### Identity and policy

- organism ID and lineage ID;
- controller genome lineage ID;
- genome schema version;
- controller evaluation policy version;
- numeric and activation policy versions;
- plasticity rule policy version;
- compiler policy version;
- mutation/crossover versions relevant to future reproduction;
- ID-hash and RNG policy versions.

### Heritable genome

- complete node genes;
- complete connection genes;
- disabled genes and any required tombstones;
- shared plasticity groups;
- module records;
- complexity caps/allocation;
- canonical genome checksum.

### Compiled identity mapping

Either save or deterministically reconstruct:

- active stable node IDs in runtime index order;
- active stable edge IDs in runtime index order;
- dormant gene lists;
- topological order;
- delay-ring offsets;
- compiler manifest checksum.

### Neural state

- current and next activation buffers if checkpoint phase can expose both;
- leak/membrane auxiliary state;
- recurrent context state;
- every delayed-edge ring value and cursor;
- oscillator phase or time accumulator if present;
- current controller tick.

### Learned state

- learned weight delta or effective learned weight for every plastic edge;
- eligibility traces;
- running pre/post activity averages;
- homeostatic state;
- consolidation fast/slow state if present;
- plasticity onset/critical-period counters.

### Modulatory state

- current channel values;
- filtered channel state;
- pending delayed modulators/rewards;
- learned modulatory-neuron state;
- source-channel mapping if mutable.

### Stochastic state

- state/counters for any controller runtime stream;
- developmental event counters, later;
- exploration-noise counter or explicit tick-based key convention.

### Phase state

- world tick;
- controller subphase;
- whether actions have been proposed or committed;
- whether eligibility has been accumulated;
- whether modulation has been applied;
- any pending action/outcome record.

The preferred checkpoint boundary makes pending phase state empty, but the save format should still identify the boundary and reject inconsistent state.

## 12.3 Atomic checkpoint boundary

The safest boundary is after:

- world action resolution;
- outcome/modulator derivation;
- plasticity update;
- recurrent state latch;
- all births/deaths for the tick;
- RNG/event counters committed;
- tick checksum calculated.

The engine should pause world mutation, serialize a canonical immutable snapshot, write to a temporary file, verify checksum, then atomically replace the checkpoint pointer. If checkpointing from a copy-on-write snapshot, the snapshot epoch itself must be deterministic.

Mid-tick checkpoints are possible but significantly expand phase state and restore testing. Avoid them unless operational requirements demand them.

## 12.4 Canonical learned-state serialization

Never serialize learned arrays without identity context. Recommended record:

```text
PlasticEdgeStateV1 {
    connection_id,
    learned_weight_delta,
    eligibility_trace,
    running_pre_average,
    running_post_average,
    auxiliary_registers[],
}
```

Sort by `connection_id`. On restore:

1. compile the genome;
2. match every saved plastic-edge record to exactly one compiled edge;
3. reject missing, duplicate, or unexpected records;
4. range-check values;
5. populate runtime arrays;
6. verify controller-state checksum.

If an edge changed from plastic to fixed through a migration, that migration must explicitly define how its learned delta is handled. Do not silently discard it.

## 12.5 Checksums

Use layered checksums:

- **genome checksum:** canonical hereditary bytes;
- **controller runtime checksum:** activations, delays, learned deltas, traces, modulators, phase;
- **organism checksum:** body/internal state plus controller checksums;
- **world checksum:** canonical world mutable state and organisms in stable ID order;
- **tick hash chain:** `H(previous_tick_hash, tick_number, world_checksum, policy_manifest_hash)`.

A Merkle-style tree can localize divergence to one organism or subsystem without changing the canonical root. The hashing algorithm and tree layout are versioned with test vectors.

Include learned state in both per-controller and world checksums. A replay that matches genome hashes but differs in traces is already divergent even if action output has not yet changed.

## 12.6 Restore validation

Restore should fail closed on:

- unsupported versions;
- checksum mismatch;
- duplicate or missing stable IDs;
- invalid graph;
- compiler-manifest mismatch;
- learned state for nonexistent or fixed edges;
- absent learned state for plastic edges;
- delay-ring length mismatch;
- out-of-range scalar values;
- inconsistent tick/update phase;
- unknown RNG stream state;
- policy manifest not matching the world save;
- noncanonical record ordering when canonicality is required.

A separate offline salvage tool may inspect or transform a damaged save. The simulation loader should not guess.

## 12.7 Save migration

A migration is a deterministic transformation:

```text
(old_bytes, migration_policy_version) -> (new_bytes, migration_manifest)
```

The manifest records:

- old and new checksums;
- source and destination schema/policy versions;
- every field transformation;
- dropped data, if any;
- default values inserted;
- lineage fork IDs;
- migration tool build and test-vector version.

If semantic equivalence cannot be proven, call it a **world/controller successor fork**, not a format-only migration. Preserve the original checkpoint.

## 12.8 Replay equivalence levels

Define explicit levels:

1. **Byte-identical restore:** serializing immediately after restore reproduces canonical bytes.
2. **Tick-identical replay:** every subsequent tick checksum matches.
3. **Behavior-identical replay:** actions and world state match, but internal bytes may differ; insufficient for Genesis’s strict contract.
4. **Statistical equivalence:** distributions match; useful for scientific comparisons but not replay.

Genesis should require levels 1 and 2 for supported policy/platform combinations.

## 12.9 Replay lineage changes

A lineage change occurs when:

- controller semantics migrate;
- numeric policy changes;
- old genomes are translated;
- learned state is intentionally reset;
- Lamarckian write-back is introduced;
- an analysis branch alters plasticity or observation;
- a checkpoint is resumed under a different world configuration.

Assign a new world lineage ID derived from the parent checkpoint hash and fork policy. The old and new runs can be compared, but they are not the same replay lineage.

## 12.10 Event logs as supplements, not substitutes

An append-only event log is valuable for audit:

- births and parent IDs;
- mutation/crossover events;
- structural IDs created/deleted;
- policy migrations;
- checkpoints;
- checksum divergence diagnostics.

It is not a substitute for current learned state. Event-sourcing every sensor value and world interaction would create unbounded restore cost and duplicate the simulation history. Use checkpoints plus bounded logs between checkpoints.

## 12.11 Golden save corpus

Maintain a repository of small canonical saves covering:

- no hidden nodes;
- recurrent delayed loops;
- disabled and dormant genes;
- every activation function;
- every plasticity rule;
- saturated positive/negative weights;
- eligibility traces at boundaries;
- module duplication, later;
- policy migration fixtures;
- malformed inputs expected to fail.

For each fixture, store canonical bytes, parsed semantic dump, expected compiler manifest, and expected tick hashes for a fixed replay horizon.

**Recommendation strength: Strongly supported.**

# 13. Dependency-ordered implementation research plan

## 13.1 Principle

Do not implement structural evolution, plasticity, developmental growth, social observation, and new save semantics in one step. Each layer changes the causal interpretation of behavior. The plan below creates a deterministic reference at every boundary and prevents later features from concealing lower-level defects.

## 13.2 Phase 0 — Formal policy specifications

**Deliverables**

- Controller scalar/numeric policy specification.
- Stable-ID preimage formats and hash test vectors.
- Canonical genome and runtime-state binary schemas.
- Exact controller tick phase diagram.
- Mutation and crossover operator specifications.
- Invalid-graph taxonomy.
- Replay-equivalence definition.

**Research questions**

- What scalar ranges occur under current sensor and actuator normalization?
- What maximum node/edge budgets fit the world tick budget?
- Are same-tick feedforward pathways required, or can all recurrence use one-tick delay?
- Which organism-visible primitive signals are legitimate modulators?

**Exit gate**

Two independent implementations or one implementation plus an executable specification produce identical test vectors for IDs, arithmetic, activation, serialization, and one-tick evaluation.

## 13.3 Phase 1 — Canonical fixed graph runtime

Implement the new graph schema while keeping topology and weights fixed.

**Deliverables**

- loader and fail-closed validator;
- canonical compiler;
- scalar reference evaluator;
- recurrent/delay semantics;
- compiled sparse runtime;
- per-tick controller checksums;
- golden graph corpus.

**Exit gate**

Bit-identical results across insertion-order permutations, thread counts, release/debug builds where supported, and target platforms.

## 13.4 Phase 2 — Structural mutation without crossover

Use asexual reproduction to isolate mutation.

**Deliverables**

- add-edge;
- split-edge/add-node;
- disable/enable;
- delete-edge;
- prune-node;
- canonical mutation event log;
- mutation property tests;
- hard complexity budgets.

**Exit gate**

At least 100,000 generated mutation events per operator policy pass validation and deterministic replay; all no-op/rejection frequencies are explainable from candidate availability.

## 13.5 Phase 3 — Structural evolution baseline

Run closed benchmark tasks and small ecological worlds.

**Deliverables**

- fixed-topology baseline;
- large-fixed sparse-mask baseline;
- direct-graph variable-topology treatment;
- bloat and innovation-survival telemetry;
- mutation-rate sweep;
- complexity-cost sweep.

**Exit gate**

Variable topology either demonstrates a repeatable advantage on at least one predeclared unknown-structure task or is retained only as a research option. A failure to beat the fixed sparse baseline is informative and must not be concealed by changing benchmarks post hoc.

## 13.6 Phase 4 — Historical crossover and innovation protection

**Deliverables**

- homology alignment;
- copy-aware identity tests;
- deterministic crossover;
- species/probation or alternative protection mechanism;
- asexual control;
- invalid-offspring and repair telemetry.

**Exit gate**

Crossover plus protection increases useful structural-innovation survival or solution attainment over mutation-only controls without unacceptable species fragmentation or compute cost.

## 13.7 Phase 5 — Exact recurrent-state persistence

Before plasticity, checkpoint every form of dynamic neural state.

**Deliverables**

- activation and delay-ring save state;
- restore-to-byte identity;
- long-horizon replay suite;
- cross-platform golden saves;
- phase-boundary assertions.

**Exit gate**

No divergence over the declared replay horizon for all golden and randomized controllers.

## 13.8 Phase 6 — One fixed plasticity rule

Start with one bounded modulated general-local or Oja-like rule. Plastic placement may be fixed by configuration or encoded by a single edge flag.

**Deliverables**

- learned delta and trace arrays;
- deterministic plasticity phase;
- exact learned-state serialization;
- fixed versus plastic benchmarks;
- saturation and forgetting telemetry.

**Exit gate**

Plasticity improves a predeclared changing-environment task and exact restoration preserves both immediate and delayed future behavior.

## 13.9 Phase 7 — Evolution of plasticity parameters

**Deliverables**

- shared plasticity groups;
- coefficient mutation;
- modulator sensitivity;
- fixed-rule and nonplastic controls;
- newborn versus post-learning assays.

**Exit gate**

Evolved coefficients outperform fixed coefficients across held-out environmental schedules, not only training schedules, and do not merely saturate weights or exploit a modulator artifact.

## 13.10 Phase 8 — Multiple modulators and rule ablations

Add primitive internal channels, then optionally evolved modulatory neurons.

**Deliverables**

- channel provenance logging;
- neuromodulation ablation;
- eligibility-trace ablation;
- reward-delay sweep;
- anti-Hebbian/Oja/general-local comparison;
- fixed and shuffled modulator controls.

**Exit gate**

The added mechanism has a reproducible causal contribution and acceptable memory cost.

## 13.11 Phase 9 — Module duplication

**Deliverables**

- motif/module record;
- atomic duplication;
- copy-aware homology;
- module-preserving crossover;
- lesion and reuse analysis.

**Exit gate**

Duplicated modules survive, diverge, and improve adaptation on modularly varying tasks more often than matched groups of edge-level mutations.

## 13.12 Phase 10 — Quality diversity and novelty

**Deliverables**

- behavior descriptor definitions separated from live organism state;
- novelty/QD archive;
- objective-only and random-descriptor controls;
- archive determinism and bounded memory;
- descriptor-alignment tests.

**Exit gate**

QD improves behavioral coverage or discovery under predeclared descriptors without merely optimizing an artifact of descriptor choice. [R59–R62]

## 13.13 Phase 11 — Social-learning readiness

**Deliverables**

- observation channels for agent, action, object, and outcome;
- demonstrator tracking;
- cross-fostering and shuffle controls;
- transmission-chain analysis;
- no built-in imitation objective.

**Exit gate**

Learners acquire behavior faster or more reliably from informative demonstrators than from matched nonsocial experience, and the effect survives demonstrator removal for at least one transmission step.

## 13.14 Phase 12 — Indirect/developmental branches

Run CPPN/HyperNEAT first, then NDP/developmental programs if justified.

**Deliverables**

- regularity continuum benchmark;
- substrate-coordinate sensitivity;
- expressed-state checkpoint analysis;
- developmental interpreter and growth budget;
- direct-encoding comparator at matched phenotype cost.

**Exit gate**

The branch demonstrates a reproducible advantage that survives phenotype-size, runtime, and save-state accounting.

## 13.15 Phase 13 — Lamarckian and lifetime structural-plasticity research

These are separate research policies, not upgrades to the default.

**Exit gate**

Benefits must persist under environmental change and held-out ecological schedules, with lineage semantics and replay verified.

---

# 14. Experiments, controls, and ablations

## 14.1 General experimental protocol

### Seed structure

Every run begins with a declared 256-bit root seed. Derive separate named streams for:

- world generation;
- weather/resource schedules;
- founder genomes;
- reproduction and mutation;
- organism runtime exploration;
- demonstrator assignment;
- analysis sampling.

Treatments in a paired comparison share exogenous world and schedule streams. Their evolutionary streams derive from the same root but include the treatment/policy ID so divergence in one treatment cannot shift another treatment’s draws.

### Replication

- **Property and determinism tests:** exhaustive enumeration for tiny graphs plus at least 100,000 randomized events per operator/policy.
- **Standard evolutionary experiments:** at least 30 independent root seeds per condition.
- **Deceptive, social, or high-variance experiments:** 50 or more independent roots when feasible.
- **Cross-platform replay:** at least 20 world roots, 100–1,000 checkpoints per root across the test horizon.

These are starting minima, not power guarantees. Use pilot variance to conduct a prospective power analysis for the chosen primary endpoint.

### Statistical reporting

Report:

- full distributions and individual seed traces;
- median and bootstrap confidence intervals;
- paired differences where roots are paired;
- effect sizes such as Cliff’s delta or Vargha–Delaney `A`;
- time-to-threshold/attainment curves;
- survival analysis for censored success times;
- predeclared primary metrics and correction for multiple comparisons;
- treatment-by-task interactions rather than only pooled averages.

### Compute accounting

Report organism-ticks, births, controller edge operations, plastic updates, wall-clock time, peak memory, checkpoint bytes, and failed mutation attempts. A method that uses twice the evaluations to reach the same result has not tied on efficiency.

## 14.2 Experiment 1 — Structural mutation correctness

**Question:** Do structural operators always produce the specified graph and event identities independent of insertion order and thread scheduling?

**Conditions and controls**

- Each operator on exhaustive graphs up to a small node/edge bound.
- Random valid graphs near every hard budget.
- Same semantic genome constructed with many insertion orders.
- Single-thread, multiple thread counts, and shuffled worker partitions.
- Reference pure implementation versus optimized implementation.

**Metrics**

- byte-identical offspring genome;
- event and gene IDs;
- validation outcome;
- realized target distribution;
- no-op/rejection reason;
- candidate count;
- graph invariants and zero-delay acyclicity.

**Seed requirements**

Exhaustive small cases plus at least 100,000 keyed events per operator, including boundary values and empty candidate lists.

**Falsification conditions**

Any semantic input produces different bytes, IDs, target, or validity outcome under a different insertion order, core count, or platform. Any operator produces an undocumented invalid graph.

**Compute implications**

Low relative to evolutionary runs; high-value continuous-integration workload. Retain failing preimages as golden regression fixtures.

## 14.3 Experiment 2 — Structural innovation survival

**Question:** Does innovation protection allow useful new structure enough time to tune, and does it outperform simpler age protection?

**Conditions and controls**

- Direct graph with no protection.
- NEAT-like species protection.
- Fixed innovation probation/age protection.
- Asexual lineages with matched mutation counts.
- Inject or identify the same add-node/add-edge innovation under paired roots.

**Metrics**

- survival of the new gene at 1, 5, 20, and 100 descendant generations;
- descendant count;
- time to recover pre-mutation fitness;
- time to exceed parent fitness;
- lineage contribution to eventual champions;
- species fragmentation and offspring allocation.

**Seed requirements**

At least 50 roots because survival events are sparse; stratify by innovation type and immediate fitness effect.

**Falsification conditions**

Protection does not increase the probability that initially deleterious but eventually useful innovations survive, or its gain disappears after matching total evaluations. Protection that preserves mostly useless structure and increases bloat without attainment benefit is also falsified.

**Compute implications**

Medium-to-high; lineage tracking and counterfactual re-evaluation add storage. Use bounded benchmark tasks before persistent worlds.

## 14.4 Experiment 3 — Topology growth versus bloat

**Question:** Which combination of deletion, caps, and cost keeps networks efficient without suppressing innovation?

**Conditions and controls**

Factorial sweep over:

- add rates;
- disable/delete rates;
- no cost, edge-count tie-break, and explicit operation cost;
- hard caps at several levels;
- deletion absent versus present.

Use both stationary tasks and a task that genuinely requires increasing memory/structure.

**Metrics**

- active/dormant/disabled nodes and edges over lineage age;
- operations per tick;
- fitness per active edge;
- checkpoint bytes;
- successful innovation rate;
- neutral drift in size;
- size after task simplification;
- fraction of cap-rejected mutations.

**Seed requirements**

At least 30 roots per cell; use a fractional design initially to screen interactions, then confirm selected configurations with full replication.

**Falsification conditions**

A proposed complexity policy prevents required growth, or network cost grows approximately with lineage age after fitness plateaus. Deletion is not justified if it reduces performance and does not reduce cost after matched budgets.

**Compute implications**

High because the factorial space is large. Controller-operation accounting is essential; wall-clock alone confounds implementation paths.

## 14.5 Experiment 4 — Fixed versus variable topology

**Question:** Does direct structural evolution outperform appropriately chosen fixed alternatives?

**Conditions and controls**

- Small fixed topology.
- Large fixed dense topology.
- Large fixed sparse-mask topology with the same maximum nodes/edges.
- Direct variable graph without crossover.
- Direct variable graph with protection/crossover.

Tasks should include:

- one known-small mapping;
- one memory/recurrent task;
- one task with unknown useful size;
- one deceptive structural stepping-stone task;
- one regular high-dimensional task.

Match evaluation count and report operation cost.

**Metrics**

- success probability;
- evaluations and organism-ticks to threshold;
- final performance;
- active operations;
- genome size;
- robustness to sensor permutation and environmental change.

**Seed requirements**

At least 30 paired roots per task; 50 for deceptive tasks.

**Falsification conditions**

Variable topology shows no task-specific advantage after accounting for operations and tuning budget, or only beats a deliberately undersized fixed network. A universal superiority claim is falsified by any stable task class where a fixed baseline is clearly better.

**Compute implications**

High but foundational. This experiment determines whether variable topology belongs in the production path or remains optional research.

## 14.6 Experiment 5 — Plastic versus nonplastic controllers

**Question:** Does lifetime synaptic change improve adaptation rather than merely add parameters?

**Conditions and controls**

- Nonplastic recurrent controller.
- Plastic controller with identical topology and heritable parameter budget.
- “Sham plastic” controller with extra heritable parameters but zero update.
- Plastic controller with learned state reset periodically.
- Fixed network state-memory control with no weight change.

Use stationary and nonstationary environments, damage, and cue-switch tasks.

**Metrics**

- newborn performance;
- learning curve within life;
- final performance;
- cumulative lifetime reward/survival;
- weight-delta magnitude and saturation;
- adaptation after change;
- forgetting of prior context;
- energetic/operation cost.

**Seed requirements**

At least 30 roots; pair lifetime schedules within roots.

**Falsification conditions**

Plasticity does not outperform sham capacity controls on changing tasks, or improvements vanish when recurrent-state effects are separated. Persistent saturation without context-sensitive benefit falsifies the rule configuration.

**Compute implications**

Medium. Plastic updates add `O(P)` operations per tick and checkpoint state.

## 14.7 Experiment 6 — Evolved versus fixed plasticity

**Question:** Does evolving plasticity parameters produce transferable learning rather than overfitting the training schedule?

**Conditions and controls**

- Fixed hand-selected coefficients.
- Evolved global coefficients.
- Evolved connection-class coefficients.
- Per-synapse coefficients with matched total heritable parameter control.
- Nonplastic and random-coefficient controls.

Train/evolve on a set of environmental schedules; evaluate champions and populations on held-out schedules, cue timings, and damage patterns.

**Metrics**

- held-out adaptation speed;
- final held-out performance;
- coefficient complexity;
- generalization gap;
- parameter sensitivity;
- proportion of plastic edges;
- newborn competence.

**Seed requirements**

At least 30 evolutionary roots and multiple held-out schedule roots per evolved champion; hierarchical analysis must treat champions nested within evolution roots.

**Falsification conditions**

Evolved rules improve only on seen schedules, encode fixed behavior through synapse-specific coefficients, or fail to beat a fixed-rule baseline after equal parameter and compute budgets.

**Compute implications**

High because each genome must be evaluated across multiple lifetime schedules to select for general learning rather than one trajectory.

## 14.8 Experiment 7 — Learning-rule ablation

**Question:** Which local terms are causally necessary?

**Conditions and controls**

Start from a successful evolved or fixed three-factor rule and ablate:

- pre×post term;
- pre-only and post-only terms;
- Oja/homeostatic term;
- eligibility memory;
- decay;
- negative updates;
- weight bounds, in a safely isolated run;
- activity averages;
- plasticity on recurrent versus feedforward edges.

**Metrics**

Performance, adaptation latency, saturation, oscillation, forgetting, and learned-state entropy.

**Seed requirements**

At least 30 paired evaluation roots for frozen controllers; if re-evolving after each ablation, at least 30 evolutionary roots per condition.

**Falsification conditions**

A claimed essential term can be removed without consistent loss, or a simpler rule equals the complex rule at lower cost. Unbounded variants that diverge numerically confirm a safety need but do not prove behavioral utility of a particular bound.

**Compute implications**

Medium for frozen-controller causal assays; high for re-evolution, which distinguishes immediate dependence from evolutionary compensation.

## 14.9 Experiment 8 — Neuromodulation ablation

**Question:** Does the modulator provide temporal/contextual credit assignment?

**Conditions and controls**

- Normal modulator.
- Modulator fixed to zero.
- Modulator fixed positive.
- Time-shuffled modulator.
- Organism-shuffled modulator.
- Sign-inverted modulator.
- Eligibility trace retained versus removed.
- Primitive engine signal versus evolved modulatory-neuron signal.

Vary reward/outcome delay.

**Metrics**

- performance versus delay;
- causal alignment between eligibility and outcome;
- weight change following true versus shuffled outcomes;
- modulator autocorrelation and saturation;
- exploit indicators such as endogenous positive-feedback loops.

**Seed requirements**

At least 30 paired roots; 50 if social outcomes are included.

**Falsification conditions**

True modulation is no better than shuffled or constant controls, or an evolved modulator improves fitness by self-stimulation disconnected from adaptive outcomes.

**Compute implications**

Medium. Logging full traces can dominate storage; sample deterministically or retain focused windows around outcomes.

## 14.10 Experiment 9 — Social-learning readiness

**Question:** Can a learner use another organism’s action–context–outcome information without a hardcoded imitation objective?

**Conditions and controls**

- Skilled demonstrator visible.
- Unskilled demonstrator visible.
- Same environmental transitions without a demonstrator.
- Demonstrator actions temporally shuffled.
- Identity shuffled while motion preserved.
- Outcome hidden.
- Action hidden but outcome visible.
- Cross-fostered learner genotypes.
- Individual-learning-only control with equal exposure time.

Use a task that can be discovered individually but is costly, allowing social information to improve speed rather than make the task impossible without scripting.

**Metrics**

- trials/time to first successful behavior;
- probability of matching demonstrator variant;
- transfer to a new location/object;
- persistence after demonstrator removal;
- learner-to-learner transmission fidelity;
- group-specific behavioral divergence;
- observation attention and plastic changes.

**Seed requirements**

At least 50 roots, multiple demonstrator–learner pairings per genotype, and transmission chains long enough to estimate fidelity.

**Falsification conditions**

Informative demonstrators do not improve acquisition over matched environmental controls; behavior follows environmental residue rather than observation; or apparent copying disappears under action/outcome shuffle.

**Compute implications**

High. Pairing and transmission chains multiply evaluations and require larger world populations. Begin in a controlled arena.

## 14.11 Experiment 10 — Learned-state save and restore

**Question:** Does a checkpoint preserve all causal learning state exactly?

**Conditions and controls**

Create controllers with:

- nonzero weight deltas;
- nonzero positive and negative traces;
- delayed rewards pending;
- recurrent buffers at multiple positions;
- saturated and boundary values;
- multiple plasticity rules.

Checkpoint at the official atomic boundary. Restore and compare against uninterrupted execution. Separately attempt forbidden mid-phase saves and confirm rejection.

**Metrics**

- canonical bytes before/after immediate restore;
- per-controller checksum;
- per-tick actions, weights, traces, and world checksum;
- restore latency and bytes.

**Seed requirements**

At least 1,000 generated controller states plus 20 evolving-world roots with checkpoints sampled throughout organism ages.

**Falsification conditions**

Any restored trace/weight differs, any future tick diverges, or save size omits state known to affect later output.

**Compute implications**

Low-to-medium and mandatory for continuous integration.

## 14.12 Experiment 11 — Replay equivalence

**Question:** Is the entire controller pipeline invariant to execution environment?

**Conditions and controls**

- one versus multiple threads;
- varied worker partition;
- x86-64 and ARM64 supported targets;
- debug and release reference profiles;
- randomized insertion order;
- scalar and optimized evaluator;
- checkpoint/resume at many ticks;
- dependency/compiler upgrade candidate versus frozen baseline.

**Metrics**

Tick hash chain, first divergent subsystem, canonical saves, event logs, and compiler manifests.

**Seed requirements**

At least 20 world roots, 100–1,000 checkpoints per root, and long horizons sufficient to amplify one-bit divergence.

**Falsification conditions**

Any supported configuration diverges. Statistical similarity is not a pass.

**Compute implications**

Medium in CI/nightly infrastructure. Store hashes every tick and full state only near divergence to control disk use.

## 14.13 Experiment 12 — Effects of learning on genetic diversity

**Question:** Does plasticity shield genetic variation, collapse it, or redirect it toward learning parameters?

**Conditions and controls**

- Nonplastic evolution.
- Fixed plasticity.
- Evolved plasticity.
- Plasticity with explicit learning cost.
- Stable and changing environments.

**Metrics**

- nucleotide/gene-level analogs: homology occupancy, allele entropy, pairwise genome distance;
- topology diversity;
- plasticity-parameter diversity;
- newborn behavioral diversity;
- post-learning behavioral diversity;
- species count and effective population size;
- genotype–phenotype mutual information estimates.

**Seed requirements**

At least 30 roots per condition over enough generations to observe equilibrium or trend; archive periodic population snapshots.

**Falsification conditions**

A proposed “plasticity maintains diversity” claim fails if genetic diversity is unchanged or lower after controlling for population performance and effective size. Conversely, diversity increase without adaptive breadth does not establish benefit.

**Compute implications**

High storage and analysis cost. Use canonical sketches for online metrics and full genomes at fixed intervals.

## 14.14 Experiment 13 — Effects of learning on adaptation speed

**Question:** Does learning accelerate adaptation to a new environment across lifetimes and generations?

**Conditions and controls**

Evolve populations in environment A, switch to B, and compare:

- nonplastic;
- fixed plastic;
- evolved plastic;
- plasticity disabled only after the switch;
- learned state reset at switch;
- newborn-only cohorts.

**Metrics**

- within-lifetime recovery time;
- generations to recover a performance threshold;
- area under performance loss curve;
- genetic change in base weights/topology;
- plastic contribution over generations;
- survival bottleneck and extinction rate.

**Seed requirements**

At least 30 paired roots; multiple A→B switch times and B variants.

**Falsification conditions**

Plasticity does not reduce initial loss or evolutionary recovery time, or it accelerates short-term recovery but prevents eventual high performance relative to nonplastic controls.

**Compute implications**

High because long pre-adaptation and post-switch phases are required.

## 14.15 Experiment 14 — Retention after environmental change

**Question:** Does adaptation to B erase competence in A?

**Conditions and controls**

A→B→A schedules with:

- no plasticity;
- plasticity;
- modulation ablated;
- modularity/duplication enabled;
- separate context cue present versus absent.

Also test interleaved A/B rather than blocked schedules.

**Metrics**

- performance on first A, B, and returned A;
- savings: relearning speed relative to first learning;
- weight and trace recovery;
- pathway/module reuse;
- catastrophic-forgetting index;
- behavior immediately after recurrent-state reset.

**Seed requirements**

At least 30 paired roots and multiple dwell times in B.

**Falsification conditions**

A claimed retention mechanism does not improve returned-A performance or only does so by failing to learn B.

**Compute implications**

Medium-to-high; frozen-controller assays can reduce evolutionary cost after candidate mechanisms are found.

## 14.16 Experiment 15 — Topology mutation-rate sweep

**Question:** What realized structural mutation regime balances exploration and tuning?

**Conditions and controls**

Sweep add-node, add-edge, disable, delete, and re-enable rates over log-spaced values, including zero. Hold weight-mutation opportunity and total birth count fixed.

**Metrics**

Realized operator frequency, immediate fitness effect, innovation survival, species count, topology size, success rate, and operations to threshold.

**Seed requirements**

Screen with 10–15 roots per cell, then confirm promising and boundary regimes with at least 30.

**Falsification conditions**

A chosen default lies on a fragile peak, produces cap pressure, or fails across task classes. The result should be a robust region, not one magic rate.

**Compute implications**

Very high if fully factorial. Use sequential model-based experimental design only for selecting configurations; final claims require independent confirmatory runs.

## 14.17 Experiment 16 — Module duplication and hierarchy

**Question:** Does duplication produce reusable specialization more effectively than equivalent edge-level mutation?

**Conditions and controls**

- No duplication.
- Atomic module duplication.
- Same number of new nodes/edges added independently.
- Random subgraph duplication.
- Duplication of lesion-validated modules.
- Connection-cost pressure on/off.

Use modularly varying goals and a transfer task recombining learned subproblems.

**Metrics**

- functional lesion modularity;
- duplication survival/divergence;
- transfer speed;
- hierarchy measures;
- crossover damage;
- size and operation cost;
- reuse across behaviors.

**Seed requirements**

At least 50 roots because useful duplication events may be rare.

**Falsification conditions**

Duplication’s advantage disappears after matching added structure and evaluation cost, or duplicated copies rarely specialize/reuse.

**Compute implications**

High; causal lesion testing can multiply evaluations. Apply lesions to sampled champions and lineage nodes, not every organism.

## 14.18 Experiment 17 — Direct versus indirect encoding across regularity

**Question:** Does a CPPN/HyperNEAT bias match Genesis interfaces?

**Conditions and controls**

Construct a continuum:

- perfectly symmetric/repeated sensor–actuator geometry;
- local irregular exceptions;
- permuted coordinates preserving task function;
- fully irregular mapping;
- changing body/sensor geometry.

Compare direct graph, HyperNEAT/CPPN, and hybrid CPPN plus direct offsets at matched expressed-network caps.

**Metrics**

Genome bytes, expressed edges, evaluations to threshold, robustness to resolution/coordinate change, plastic-state bytes, and final performance.

**Seed requirements**

At least 30 roots per regularity level.

**Falsification conditions**

Indirect encoding does not gain as regularity increases, or gains vanish after expressed compute and checkpoint cost are included. Coordinate permutation sensitivity reveals dependence on an arbitrary substrate.

**Compute implications**

High but bounded; this should precede any integration into the persistent world.

## 14.19 Experiment 18 — Darwinian versus Lamarckian inheritance

**Question:** Under what environmental stability does acquired-weight inheritance help or harm?

**Conditions and controls**

- Darwinian: inherit base genome only.
- Full Lamarckian final-weight write-back.
- Partial interpolation at several fixed fractions.
- Early-life/germline snapshot.
- Learned-state inheritance with and without mutation.

Test stable, periodically changing, randomly changing, and spatially heterogeneous environments.

**Metrics**

- adaptation speed;
- held-out performance;
- genetic diversity;
- extinction risk after change;
- parent-age effect;
- inherited-state size;
- lineage replay complexity;
- newborn versus learned competence.

**Seed requirements**

At least 30 roots per condition; 50 for stochastic environment schedules.

**Falsification conditions**

Any claim of general Lamarckian superiority is falsified if benefits reverse under plausible nonstationarity. A production proposal fails if it cannot define exact parent-state sampling and lineage semantics.

**Compute implications**

High and long-horizon. Run only after the Darwinian system is stable.

## 14.20 Experiment 19 — Numeric-policy and platform conformance

**Question:** Does the chosen fixed-point or strict-float policy preserve behavior and enough dynamic range?

**Conditions and controls**

- High-precision reference arithmetic.
- Candidate fixed-point formats.
- Scalar and SIMD implementations.
- Supported CPU architectures.
- Extreme recurrent/plastic parameter cases.

**Metrics**

Bit identity, deviation from reference, saturation frequency, attractor changes, learning success, and performance cost.

**Seed requirements**

Exhaustive arithmetic boundary vectors plus at least 10,000 random controllers and 20 evolutionary roots per candidate format.

**Falsification conditions**

Cross-platform mismatch, frequent unintended saturation, or material loss of controller capability versus the reference.

**Compute implications**

Medium and front-loaded. It is much cheaper than changing numeric semantics after long-running worlds exist.

---

# 15. Performance and memory implications

## 15.1 Asymptotic controller cost

For an active controller with:

- `N` active nodes;
- `E` active edges;
- `P` plastic edges;
- `D = Σ delay_ticks` stored delayed scalar slots;

one tick is approximately:

\[
T_{tick}=O(N+E+P)
\]

with memory:

\[
M_{hot}=O(N+E+P+D).
\]

Structural complexity affects every world tick, while genome mutation occurs only at reproduction. Therefore, a small reduction in active edge count can matter more than a large reduction in reproduction-time encoding cost.

## 15.2 Illustrative compact runtime budget

The following is an illustration, not a Rust ABI guarantee. Suppose a compiled representation uses approximately:

- 12 bytes of mutable node state per node across activation/next/auxiliary arrays;
- 20 bytes of compiled/static and index data per active edge;
- 8 bytes of learned delta plus eligibility trace per plastic edge;
- 4 bytes per delayed scalar slot.

Then:

\[
M \approx 12N + 20E + 8P + 4D.
\]

For `N=32`, `E=96`, `P=48`, and `D=96`:

\[
M \approx 384 + 1,920 + 384 + 384 = 3,072\text{ bytes}
\]

or roughly 3 KiB of hot controller data before allocator, genome, organism, and index overhead. Ten thousand such organisms would require roughly 29.3 MiB of hot controller data. Stable 128-bit IDs need not be present in every hot arithmetic record; dense runtime indexes can reference a separate stable-ID table.

Actual measurement must use `size_of`, allocator accounting, and serialized-byte telemetry under the final layout.

## 15.3 Illustrative operation volume

At 10,000 organisms and 96 active edges each, one world tick performs approximately 960,000 edge accumulations. If 48 edges per organism are plastic, it also performs approximately 480,000 plastic-edge updates. At 30 world ticks per second, that is about 28.8 million edge accumulations and 14.4 million plastic updates per second, excluding sensors, world physics, action resolution, reproduction, and analysis.

These counts are not a hardware performance prediction. They show why hard budgets and sparse compilation are necessary.

## 15.4 Genome versus runtime representation

The canonical genome can be object-rich and audit-friendly; the hot phenotype should be array-oriented.

Recommended split:

- **Genome:** sorted records with stable IDs, ancestry, homology, disabled history, and policy metadata.
- **Compiled phenotype:** dense node/edge indexes, inbound ranges, compact flags, fixed-point parameters.
- **Mutable state:** structure-of-arrays buffers for activations, traces, and deltas.

Compilation occurs at birth, structural mutation, load, or policy migration—not every tick.

## 15.5 Fixed graph versus sparse mask

A fixed sparse-mask network can preallocate maximum arrays and avoid per-organism graph compiler overhead. Its cost depends on whether masked edges are skipped:

- dense masked evaluation retains fixed dense cost;
- sparse index lists approach direct-graph cost;
- mask mutation is simpler but stores the full ceiling genotype or bitset;
- positional crossover is deterministic but may combine functionally unrelated pathways.

This is why it is a necessary baseline rather than an obviously inferior design.

## 15.6 Plasticity memory

Minimum per plastic edge:

- learned delta;
- eligibility trace.

Common additions:

- pre/post running averages;
- multiple traces at different timescales;
- fast and slow weights;
- rule registers;
- last-spike times for STDP.

A rule that uses four 32-bit runtime scalars per plastic edge adds 16 bytes. At one million plastic edges across the world, each additional scalar costs approximately 3.8 MiB. Rule design should therefore distinguish essential state from biologically decorative state.

## 15.7 Delay memory

Explicit integer delays require a ring buffer. Allocate one shared contiguous delay array per controller and assign stable edge offsets. Delay cost is proportional to the sum of delays, not merely recurrent-edge count.

Set a small initial maximum, such as a policy-chosen single-digit number of ticks after sensor-timescale analysis. Large delays can be represented by neuron memory dynamics more cheaply than long per-edge rings in many cases.

## 15.8 CTRNN cost

A CTRNN with `S` fixed integration substeps multiplies much of the node/edge arithmetic by approximately `S`. It may achieve behavior with fewer nodes, but that tradeoff must be measured. If `S=4`, a 24-edge CTRNN is not automatically cheaper than a 64-edge discrete RNN once activation and integration operations are counted.

## 15.9 HyperNEAT cost

HyperNEAT can make the hereditary genome compact while generating many expressed edges. Costs include:

- substrate query/generation at birth;
- threshold/expression logic;
- potentially large runtime phenotype;
- plastic state for every expressed plastic edge;
- regeneration and mapping during restore.

Report genome bytes, phenotype bytes, hot-state bytes, and operations separately.

## 15.10 Developmental cost

Developmental programs shift work to birth/development:

- multiple growth rounds;
- local communication;
- graph materialization;
- conflict resolution;
- validation;
- possible failed or over-budget development.

If organisms reproduce frequently, development cost can be material. A compact genome that takes thousands of operations to grow a small controller may be worse than direct encoding.

## 15.11 Save-state cost

Approximate controller checkpoint bytes are:

\[
M_{save}=M_{genome}+M_{node\ state}+M_{delay}+M_{plastic}+M_{modulator}+M_{identity}+M_{framing}.
\]

Compression is safe only if lossless and versioned. Similar genomes across a lineage invite deduplication or content-addressed storage, but checkpoints must remain self-validating. A base-genome blob plus per-organism learned state can reduce duplicate storage if the reference graph is immutable and hash-addressed.

## 15.12 Optimization priorities

1. Bound network size before micro-optimization.
2. Compile to dense arrays.
3. Remove dormant structure from hot execution.
4. Keep plastic and fixed edges in separate spans where useful.
5. Use contiguous state and avoid per-edge heap objects.
6. Batch controllers only when batching preserves exact per-organism semantics.
7. Maintain a scalar reference path.
8. Profile sensor/world cost before assuming controllers dominate.
9. Avoid GPU dependence for strict replay unless custom integer kernels are proven bit-identical across supported devices.
10. Track operations in simulation metrics so a faster implementation does not conceal a more expensive evolved architecture.

## 15.13 Population-search cost

Quality diversity, speciation, and crossover add population-level overhead:

- pairwise compatibility distances can be `O(population²)` if naive;
- novelty archives can grow without bounds;
- MAP-Elites descriptor grids consume archive memory;
- lineage and counterfactual tests add offline evaluations.

For persistent worlds, reproduction may be distributed over time rather than batched generations. Use bounded local neighborhoods, deterministic archive limits, and incremental indexes whose observable results are canonical.

---

# 16. Major risks and likely failure modes

| Risk | Mechanism | Early warning | Mitigation |
|---|---|---|---|
| **Topology bloat** | Additions accumulate faster than deletion/cost removes them | Edge count follows lineage age after fitness plateaus | Hard caps, deletion, dormant compilation, explicit operation metrics |
| **Innovation cemetery** | Speciation protects useless novelty | Many old species, low contribution to champions | Probation limits, extinction rules, ablation against age protection |
| **Premature structural convergence** | Too little protection or mutation | Topology diversity collapses early | Larger population/demes, protection, QD experiment, rate tuning |
| **False homology** | Equal signatures from duplicated/convergent genes are merged | Crossover creates interface swaps or duplicate collapse | Separate origin, homology, copy, and structural IDs |
| **False nonhomology** | Equivalent ancestral operations receive unrelated IDs | Excess/disjoint counts explode | Deterministic ancestral homology derivation |
| **Crossover damage** | Graph recombination breaks recurrent computation | High invalid/low-fitness offspring rate | Asexual baseline, module-aware crossover, deterministic closure/repair |
| **Mutation target bias** | Retry sampling favors easy legal targets | Nonuniform target frequencies | Enumerate legal candidates or prove indexed sampler distribution |
| **Plasticity saturation** | Positive feedback drives all deltas to bounds | High bound occupancy, behavior insensitive to context | Oja/homeostasis, decay, deadband, lower rates, modulation |
| **Catastrophic forgetting** | Shared synapses overwritten by new context | A→B→A failure | Modularity, gating, sparse plasticity, consolidation experiments |
| **Plasticity shielding** | Learning compensates for weak genes | Newborn performance stagnates, high learning dependence | Learning cost, stable-environment assays, newborn selection pressure |
| **Modulator hacking** | Evolved network creates positive reinforcement unrelated to adaptation | Self-generated modulator decouples from outcome | Channel provenance, caps, shuffled controls, primitive signals |
| **Hidden task objective** | Engine exposes post hoc fitness as reward | Controllers optimize researcher label directly | Only embodied primitive signals; audit signal graph |
| **Social-learning false positive** | Environmental residue rather than observation causes transfer | Effect survives observation occlusion | Shuffles, occlusion, cross-fostering, clean arenas |
| **Correspondence failure** | Observed action cannot map to own motor system | Attention without imitation transfer | Body-relative observation, action/outcome representation, controlled tasks |
| **Recurrent chaos/saturation** | Feedback and plasticity destabilize dynamics | Activations at bounds, high sensitivity to one-bit changes | Bounded gains, leak ranges, delayed-cycle rules, stability telemetry |
| **Determinism drift** | container order, floats, dependency update, parallel reduction | First checksum divergence across builds | Canonical structures, fixed-point, test vectors, version pinning |
| **Incomplete checkpoint** | trace/buffer/pending reward omitted | Divergence only after several ticks | Full state inventory, randomized checkpoint testing |
| **Save bloat** | disabled history and plastic state accumulate | Checkpoint bytes grow faster than population | History budgets, content-addressed genome dedup, bounded traces |
| **Developmental explosion** | grammar/NDP grows beyond budget | high failed births, cap-bound phenotypes | fixed rounds, resource budget, separate research branch |
| **Symmetry collapse** | shared developmental program produces identical cells | homogeneous node states and repeated useless structure | intrinsic lineage state, deterministic symmetry breaking [R19] |
| **Indirect encoding mismatch** | substrate geometry does not reflect useful computation | performance collapses under coordinate permutation | regularity continuum benchmark, hybrid offsets, reject branch |
| **Research overfitting** | architecture tuned to a small benchmark set | gains vanish in ecological or held-out schedules | preregistered task suite, held-out environments, operation matching |
| **Analysis feedback contamination** | post hoc labels influence live selection | “era/culture” detector becomes objective | strict offline-only analysis boundary |
| **Legacy policy burden** | every semantic version must remain replayable | maintenance cost grows without plan | frozen interpreters, migration manifests, supported-version policy |
| **Compute collapse** | successful evolution fills caps across thousands of organisms | world tick misses budget | active-operation cost, cap telemetry, admission control, profiling |

## 16.1 Highest-priority risks

The first four risks to eliminate are:

1. nondeterministic structural identity;
2. incomplete learned-state persistence;
3. uncontrolled topology/plastic-state growth;
4. false claims of learning caused by recurrent state or environmental residue.

These can invalidate every later scientific conclusion even if behavior appears interesting.

---

# 17. Unsupported or overly optimistic claims

## 17.1 “NEAT generally beats fixed-topology neuroevolution”

**Unsupported.** The original result was strong on specific pole-balancing benchmarks with particular baselines and ablations. Later NEAT-family success is broad but heterogeneous. Use task- and cost-matched comparisons. [R1–R3]

## 17.2 “Innovation numbers solve homology”

**Overstated.** Historical markers substantially improve alignment, but duplication, convergence, gene loss, and module copies create ambiguous correspondence. Innovation identity is evidence of history, not a complete biological homology oracle.

## 17.3 “The same structural mutation should always get the same ID”

**Ambiguous.** It may deserve the same crossover homology class while retaining a distinct origin event and gene lineage. Identical phenotype does not imply identical ancestry.

## 17.4 “Starting minimal automatically prevents bloat”

**False over long horizons.** Minimal start reduces initial dimension; it does not provide simplification. Long-running systems need deletion, caps, and cost controls.

## 17.5 “Indirect encodings automatically produce modularity”

**Unsupported.** They readily produce regularity and repetition when the generator bias matches the task. Functional modularity depends on task structure, cost, variation, and representation. [R7–R11, R24–R29]

## 17.6 “A compact developmental genome means a cheap organism”

**False.** Development, expressed phenotype evaluation, plastic state, and checkpoint storage can dominate genome bytes.

## 17.7 “Developmental encodings are more biologically plausible and therefore better”

**Unsupported.** Plausibility is multidimensional, and the added semantics can reduce evolvability or create implementation artifacts. Genesis needs capability and falsification evidence.

## 17.8 “Hebbian plasticity is sufficient for learning”

**Unsupported.** Plain Hebbian updates can store correlations but often require normalization, gating, homeostasis, recurrence, and appropriate sensory/action structure.

## 17.9 “STDP is preferable because it is biologically realistic”

**Unsupported for a rate-based world.** STDP is appropriate when spike timing has a meaningful modeled timescale and produces a measured benefit. It otherwise adds cost and semantics without necessity.

## 17.10 “Differentiable plasticity means organisms should use backpropagation”

**False.** The cited work uses backpropagation as an outer optimization method for plasticity parameters; runtime plastic updates remain local. [R44]

## 17.11 “Plasticity automatically enables imitation or culture”

**False.** Social learning requires perception, correspondence, attention, memory, opportunity, and transmission ecology. Plasticity is one mechanism for retaining information. [R66–R68]

## 17.12 “A behavioral tradition proves social learning”

**Unsupported without controls.** Persistent environmental traces, common genetics, or shared cues can produce group similarity. Demonstrator shuffles, occlusion, and cross-fostering are required.

## 17.13 “The Baldwin effect always accelerates genetic evolution”

**False.** Learning can smooth search and expose adaptive regions, but it can also shield genotypes, reduce selection gradients, and maintain plasticity rather than assimilation. [R51, R55–R57]

## 17.14 “Lamarckian inheritance is always faster and therefore better”

**False.** It may accelerate stable local optimization while harming adaptation to change, diversity, and interpretability. [R57–R58]

## 17.15 “CTRNNs are inherently more capable than discrete RNNs”

**Unsupported.** They provide a useful dynamical bias and evolved time constants; discrete leaky/delayed networks may represent the needed behavior at lower deterministic cost.

## 17.16 “Novelty search or MAP-Elites guarantees open-endedness”

**False.** Results depend on the behavior descriptor, archive, variation, environment, and bounded search space. QD can improve coverage without creating indefinitely expanding complexity. [R59–R62]

## 17.17 “Learned weights can be reconstructed from the genome”

**False for restoration.** Exact reconstruction requires the entire causal lifetime history and update state. Store learned state.

## 17.18 “IEEE-754 means floating-point replay is automatically cross-platform”

**False.** Operation order, contraction, transcendentals, NaNs, library algorithms, and compiler behavior remain risks. Rust documents several such caveats. [R71]

## 17.19 “A deterministic RNG is enough for deterministic evolution”

**False.** Draw assignment, candidate ordering, retry count, event identity, commit order, and numeric behavior must also be deterministic.

## 17.20 “Variable topology will cause increasingly complex behavior”

**Unsupported.** It removes a representational ceiling but does not create ecological demand, stepping stones, or selection for complexity.

---

# 18. Open questions

## 18.1 Architecture and representation

- What controller-size distribution can the engine sustain at target population and tick rate?
- Should all non-sensor edges be delayed in the first policy, or is a zero-delay acyclic subgraph worth the complexity?
- Does the current sensor–actuator interface contain meaningful geometry for a CPPN substrate?
- Are stable neuron coordinates part of organism embodiment or merely an experimenter annotation?
- How should topology adapt when future genomes add/remove sensors or action capabilities?
- What module boundary definition correlates best with causal lesion tests?
- Is sexual crossover beneficial after mutation-only structural evolution is tuned?
- Should historical disabled genes remain in the reproductive genome, move to an archive, or expire?
- How should homology be represented after repeated module duplication and deletion?

## 18.2 Plasticity

- Which primitive internal signals are legitimate modulators without encoding a hidden fitness function?
- Is per-synapse eligibility affordable at expected world scale?
- Does Oja-like normalization preserve adaptive recurrent dynamics or suppress useful amplification?
- What fraction of edges should initially be eligible for plasticity?
- Should recurrent, sensor, motor, and modulatory edges use different rule classes?
- Are one or two timescales of plasticity enough for retention?
- Can evolved rule groups transfer across body changes and ecological niches?
- Does a critical period evolve under realistic lifespan and learning costs?
- Can endogenous modulatory neurons remain grounded in outcomes rather than self-reward loops?

## 18.3 Evolution–learning interaction

- Does plasticity accelerate environmental adaptation or mainly shield poor genotypes?
- Under what world stability does genetic assimilation appear?
- Does learning increase or reduce structural/genetic diversity over very long runs?
- Are innate priors, learning rates, or sensory attention the primary target of selection?
- How does lifespan alter the balance between inherited and learned behavior?
- Does partial Lamarckian inheritance ever outperform Darwinian evolution under spatially heterogeneous niches?
- Can learned traditions persist after founder genotypes disappear?
- Does culture favor genetic assimilation of the behavior or of social-learning capacity?

## 18.4 Modularity and complexity

- Does connection cost create useful modules under Genesis ecology or only smaller networks?
- Can duplication create stable specialization without hardcoded module roles?
- Do modularly varying ecological demands arise naturally, or must benchmark environments provide them?
- Does structural complexity increase behavioral repertoire, robustness, or merely neutral redundancy?
- What prevents lineages from filling hard caps once controller cost becomes affordable?
- Can hierarchy be detected causally rather than by graph statistics alone?

## 18.5 Determinism and persistence

- Which fixed-point format has adequate headroom for recurrence and plastic traces?
- What is the smallest save-state representation that remains exactly causal?
- Can content-addressed genome deduplication reduce save size without complicating atomic checkpoints?
- How many legacy controller-policy interpreters will be supported simultaneously?
- What migration changes can be proven semantics-preserving?
- What cross-platform target matrix is contractually supported?
- Should controller compiler output be serialized as authoritative state or treated as a verified cache?
- How will deterministic archives/species be maintained under distributed world simulation?

## 18.6 Social learning

- What perceptual representation lets one organism distinguish action, object, actor, and outcome without authoring imitation?
- How is the correspondence problem between demonstrator and learner bodies addressed?
- Can attention and demonstrator preference evolve from primitive observation?
- What transmission fidelity is required for traditions to persist?
- How do artifact traces interact with direct observation and teaching?
- Can behavior spread without genetically fixed imitation machinery?
- How will analysis distinguish social transmission from shared environment and common descent at scale?

## 18.7 Open-endedness

- What ecological processes continually create new adaptive niches?
- Are controller costs, body costs, and world modification coupled in a way that supports rather than suppresses complexity?
- Can selection preserve stepping stones that have no immediate reproductive benefit?
- Which diversity descriptors remain meaningful without becoming implicit designer objectives?
- How will the project distinguish increasing complexity, increasing diversity, and mere accumulation?
- What evidence would justify calling the process open-ended rather than long-running optimization?
---

# 19. Annotated bibliography

The bibliography emphasizes original algorithm papers, primary experiments, canonical biological studies, and official implementation documentation. Reviews are included where they provide field-level synthesis. DOI links identify the version of record where available; stable proceedings or archive links are used when no DOI was assigned.

## 19.1 Variable topology, indirect encoding, modularity, and recurrent controllers

### [R1] Stanley & Miikkulainen — NEAT

Stanley, K. O., & Miikkulainen, R. (2002). Evolving neural networks through augmenting topologies. *Evolutionary Computation, 10*(2), 99–127. [https://doi.org/10.1162/106365602320169811](https://doi.org/10.1162/106365602320169811)

**Annotation.** Canonical source for historical markings, topology complexification, minimal starts, and speciation in NEAT. Its comparative evidence is strong for the paper’s pole-balancing tasks, but it does not establish universal superiority over fixed-topology evolution.

### [R2] Papavasileiou, Cornelis, & Jansen — systematic review of NEAT successors

Papavasileiou, E., Cornelis, J., & Jansen, B. (2021). A systematic literature review of the successors of NeuroEvolution of Augmenting Topologies. *Evolutionary Computation, 29*(1), 1–73. [https://doi.org/10.1162/evco_a_00282](https://doi.org/10.1162/evco_a_00282)

**Annotation.** Maps the large and heterogeneous family of NEAT extensions. It supports treating NEAT as a design lineage rather than a single frozen implementation and highlights uneven comparison practices across the literature.

### [R3] Stanley, Clune, Lehman, & Miikkulainen — neuroevolution landscape

Stanley, K. O., Clune, J., Lehman, J., & Miikkulainen, R. (2019). Designing neural networks through neuroevolution. *Nature Machine Intelligence, 1*, 24–35. [https://doi.org/10.1038/s42256-018-0006-z](https://doi.org/10.1038/s42256-018-0006-z)

**Annotation.** Broad synthesis of architecture evolution, indirect encodings, novelty, and hybrid neuroevolution. Useful for field context; individual engineering choices in Genesis still require primary-paper and system-level validation.

### [R4] Yao — early evolutionary neural-network review

Yao, X. (1993). A review of evolutionary artificial neural networks. *International Journal of Intelligent Systems, 8*(4), 539–567. [https://doi.org/10.1002/int.4550080406](https://doi.org/10.1002/int.4550080406)

**Annotation.** Establishes the historical breadth of evolving weights, architectures, and learning rules before NEAT. It is valuable for avoiding the false impression that NEAT introduced variable-topology neuroevolution as a whole.

### [R5] Angeline, Saunders, & Pollack — structurally evolving recurrent networks

Angeline, P. J., Saunders, G. M., & Pollack, J. B. (1994). An evolutionary algorithm that constructs recurrent neural networks. *IEEE Transactions on Neural Networks, 5*(1), 54–65. [https://doi.org/10.1109/72.265960](https://doi.org/10.1109/72.265960)

**Annotation.** Presents GNARL, an early direct method for evolving structurally unconstrained recurrent networks. It supports mutation-only structural evolution as a credible baseline independent of NEAT-style crossover.

### [R6] Yao & Liu — EPNet

Yao, X., & Liu, Y. (1997). A new evolutionary system for evolving artificial neural networks. *IEEE Transactions on Neural Networks, 8*(3), 694–713. [https://doi.org/10.1109/72.572107](https://doi.org/10.1109/72.572107)

**Annotation.** EPNet combines weight evolution, partial training, node/connection deletion, and addition. It is especially relevant to Genesis’s decision to support simplification rather than add-only topology growth.

### [R7] Stanley — CPPNs

Stanley, K. O. (2007). Compositional pattern producing networks: A novel abstraction of development. *Genetic Programming and Evolvable Machines, 8*(2), 131–162. [https://doi.org/10.1007/s10710-007-9028-8](https://doi.org/10.1007/s10710-007-9028-8)

**Annotation.** Defines CPPNs as function compositions that generate spatial patterns such as symmetry, repetition, and gradients. It supplies the conceptual basis for HyperNEAT but not a reason to assume every controller has useful geometric regularity.

### [R8] Stanley, D’Ambrosio, & Gauci — HyperNEAT

Stanley, K. O., D’Ambrosio, D. B., & Gauci, J. (2009). A hypercube-based encoding for evolving large-scale neural networks. *Artificial Life, 15*(2), 185–212. [https://doi.org/10.1162/artl.2009.15.2.15202](https://doi.org/10.1162/artl.2009.15.2.15202)

**Annotation.** Canonical HyperNEAT paper. It demonstrates how a CPPN can generate connectivity over a geometric substrate and is the main evidence for a later Genesis indirect-encoding branch when sensor–actuator geometry is meaningful.

### [R9] Risi & Stanley — ES-HyperNEAT

Risi, S., & Stanley, K. O. (2012). An enhanced hypercube-based encoding for evolving the placement, density, and connectivity of neurons. *Artificial Life, 18*(4), 331–363. [https://doi.org/10.1162/ARTL_a_00071](https://doi.org/10.1162/ARTL_a_00071)

**Annotation.** Extends HyperNEAT so neuron placement and connection density can emerge rather than being entirely prescribed. This reduces one substrate-design limitation but increases phenotype-generation and validation complexity.

### [R10] Clune, Stanley, Pennock, & Ofria — regularity continuum

Clune, J., Stanley, K. O., Pennock, R. T., & Ofria, C. (2011). On the performance of indirect encoding across the continuum of regularity. *IEEE Transactions on Evolutionary Computation, 15*(3), 346–367. [https://doi.org/10.1109/TEVC.2010.2104157](https://doi.org/10.1109/TEVC.2010.2104157)

**Annotation.** Directly tests when an indirect encoding’s regularity bias helps or hurts. It is central to the recommendation that Genesis compare encodings across controlled regularity levels rather than assume compactness implies better search.

### [R11] Helms & Clune — hybrid indirect/direct refinement

Helms, J. A., & Clune, J. (2017). Improving HybrID: How to best combine indirect and direct encoding in evolutionary algorithms. *PLOS ONE, 12*(3), e0174635. [https://doi.org/10.1371/journal.pone.0174635](https://doi.org/10.1371/journal.pone.0174635)

**Annotation.** Studies hybrid strategies in which an indirect encoding creates regular structure and direct encoding refines exceptions. It supports a future “generator plus explicit edits” branch, not using HyperNEAT as the first production controller.

### [R12] Kitano — graph-generation encoding

Kitano, H. (1990). Designing neural networks using genetic algorithms with graph generation system. *Complex Systems, 4*(4), 461–476. [https://www.complex-systems.com/abstracts/v04_i04_a06/](https://www.complex-systems.com/abstracts/v04_i04_a06/)

**Annotation.** Early grammar-based indirect encoding intended to improve scalability by generating connectivity from compact rules. It illustrates both the reuse advantage and the nonlocal mutation effects of developmental representations.

### [R13] Gruau — cellular encoding and modular networks

Gruau, F. (1994). Automatic definition of modular neural networks. *Adaptive Behavior, 3*(2), 151–183. [https://doi.org/10.1177/105971239400300202](https://doi.org/10.1177/105971239400300202)

**Annotation.** Demonstrates a cellular graph grammar that can define and reuse neural subnetworks. It is a canonical source for developmental module reuse, while also showing the additional interpreter and genotype–phenotype machinery such encodings require.

### [R14] Gruau — cellular-encoding thesis

Gruau, F. (1994). *Neural network synthesis using cellular encoding and the genetic algorithm* (Doctoral dissertation, Laboratoire de l’Informatique du Parallélisme, École Normale Supérieure de Lyon, France). [https://theses.fr/1994LYO10019](https://theses.fr/1994LYO10019)

**Annotation.** Full technical treatment of cellular encoding, hierarchical subnetworks, and evolutionary synthesis. It remains a foundational developmental-encoding source, although its computational setting differs sharply from a modern persistent ALife engine.

### [R15] Astor & Adami — local neural development

Astor, J. C., & Adami, C. (2000). A developmental model for the evolution of artificial neural networks. *Artificial Life, 6*(3), 189–218. [https://doi.org/10.1162/106454600568834](https://doi.org/10.1162/106454600568834)

**Annotation.** Uses autonomous neurons and local chemical interactions to develop network structure. It is relevant to distributed developmental rules but would need synchronous rounds or a canonical event queue to meet Genesis determinism.

### [R16] Stanley & Miikkulainen — taxonomy of artificial embryogeny

Stanley, K. O., & Miikkulainen, R. (2003). A taxonomy for artificial embryogeny. *Artificial Life, 9*(2), 93–130. [https://doi.org/10.1162/106454603322221487](https://doi.org/10.1162/106454603322221487)

**Annotation.** Separates developmental systems along dimensions such as explicitness, locality, and emergence. It is useful for specifying what a proposed Genesis “developmental encoding” actually means instead of treating the term as one architecture.

### [R17] Hornby — generative representation and scalability

Hornby, G. S. (2004). Functional scalability through generative representations: The evolution of table designs. *Environment and Planning B: Planning and Design, 31*(4), 569–587. [https://doi.org/10.1068/b3015](https://doi.org/10.1068/b3015)

**Annotation.** Shows how generative representations can scale repeated design structure. The evidence is from an engineered design domain, so its relevance to neural control is representational rather than a direct controller comparison.

### [R18] Najarro, Sudhakaran, & Risi — Neural Developmental Programs

Najarro, E., Sudhakaran, S., & Risi, S. (2023). Towards self-assembling artificial neural networks through Neural Developmental Programs. *Artificial Life Conference Proceedings* (ALIFE 2023). [https://doi.org/10.1162/isal_a_00697](https://doi.org/10.1162/isal_a_00697)

**Annotation.** Introduces local neural growth controlled by another learned program and evaluates multiple optimization settings. It is directly relevant to later self-assembly research but is too early and computationally involved to displace a direct graph as Genesis’s first successor.

### [R19] Nisioti, Plantec, Montero, Pedersen, & Risi — neuronal diversity in growth

Nisioti, E., Plantec, E., Montero, M., Pedersen, J. W., & Risi, S. (2024). Growing artificial neural networks for control: The role of neuronal diversity. In *Proceedings of the Genetic and Evolutionary Computation Conference Companion*. ACM. [https://doi.org/10.1145/3638530.3654356](https://doi.org/10.1145/3638530.3654356)

**Annotation.** Identifies symmetry collapse and loss of neuronal diversity as practical developmental problems and tests intrinsic state and lateral inhibition as remedies. This is a useful warning against assuming shared local programs automatically produce differentiated circuits.

### [R20] Plantec, Pedersen, Montero, Nisioti, & Risi — lifelong developmental plasticity

Plantec, E., Pedersen, J. W., Montero, M. L., Nisioti, E., & Risi, S. (2024). Evolving self-assembling neural networks: From spontaneous activity to experience-dependent learning. *Artificial Life Conference Proceedings* (ALIFE 2024), article 37. [https://doi.org/10.1162/isal_a_00755](https://doi.org/10.1162/isal_a_00755)

**Annotation.** Combines self-assembly with activity- and reward-dependent synaptic and structural plasticity. It is highly relevant to the long-term vision but combines several hard problems that Genesis should validate separately first.

### [R21] Mordvintsev, Randazzo, Niklasson, & Levin — neural cellular automata

Mordvintsev, A., Randazzo, E., Niklasson, E., & Levin, M. (2020). Growing neural cellular automata. *Distill, 5*(2), e23. [https://doi.org/10.23915/distill.00023](https://doi.org/10.23915/distill.00023)

**Annotation.** Demonstrates local neural update rules that grow and regenerate spatial patterns. It establishes the expressive potential of neural cellular development, but the method is gradient-trained and not itself an ALife controller-evolution result.

### [R22] Najarro, Sudhakaran, Glanois, & Risi — HyperNCA

Najarro, E., Sudhakaran, S., Glanois, C., & Risi, S. (2022). HyperNCA: Growing developmental networks with neural cellular automata. In *From Cells to Societies: Collective Learning Across Scales, ICLR 2022 Workshop*. [https://arxiv.org/abs/2204.11674](https://arxiv.org/abs/2204.11674)

**Annotation.** Applies neural cellular automata as a hypernetwork that grows controller weights and explores developmental metamorphosis. It is a useful modern comparator for Genesis’s eventual developmental branch, not yet a mature production choice.

### [R23] Calabretta, Nolfi, Parisi, & Wagner — module duplication

Calabretta, R., Nolfi, S., Parisi, D., & Wagner, G. P. (2000). Duplication of modules facilitates the evolution of functional specialization. *Artificial Life, 6*(1), 69–84. [https://doi.org/10.1162/106454600568320](https://doi.org/10.1162/106454600568320)

**Annotation.** Provides direct experimental support for duplication followed by regulatory and functional divergence. It is the strongest primary source for adding an explicit module-duplication operator after basic topology evolution is stable.

### [R24] Clune, Mouret, & Lipson — connection cost and modularity

Clune, J., Mouret, J.-B., & Lipson, H. (2013). The evolutionary origins of modularity. *Proceedings of the Royal Society B: Biological Sciences, 280*(1755), 20122863. [https://doi.org/10.1098/rspb.2012.2863](https://doi.org/10.1098/rspb.2012.2863)

**Annotation.** Shows that performance pressure plus connection cost can evolve modular networks in studied tasks. It supports explicit controller cost, while not proving that cost alone will produce useful modularity in Genesis ecology.

### [R25] Kashtan & Alon — modularly varying goals

Kashtan, N., & Alon, U. (2005). Spontaneous evolution of modularity and network motifs. *Proceedings of the National Academy of Sciences, 102*(39), 13773–13778. [https://doi.org/10.1073/pnas.0503610102](https://doi.org/10.1073/pnas.0503610102)

**Annotation.** Demonstrates that environments composed of recombining subgoals can favor modular structure. Its task construction is an experimental mechanism, not evidence that modularly varying goals will arise automatically in an open world.

### [R26] Huizinga, Mouret, & Clune — modularity plus regularity

Huizinga, J., Mouret, J.-B., & Clune, J. (2014). Evolving neural networks that are both modular and regular: HyperNEAT plus the connection cost technique. In *Proceedings of the Genetic and Evolutionary Computation Conference* (pp. 697–704). ACM. [https://doi.org/10.1145/2576768.2598232](https://doi.org/10.1145/2576768.2598232)

**Annotation.** Combines a generative encoding’s regularity bias with connection-cost pressure. It supports evaluating modularity and regularity as separate properties rather than assuming one implies the other.

### [R27] Mengistu, Huizinga, Mouret, & Clune — hierarchy

Mengistu, H., Huizinga, J., Mouret, J.-B., & Clune, J. (2016). The evolutionary origins of hierarchy. *PLOS Computational Biology, 12*(6), e1004829. [https://doi.org/10.1371/journal.pcbi.1004829](https://doi.org/10.1371/journal.pcbi.1004829)

**Annotation.** Provides evidence that connection-cost pressure can promote hierarchical organization in constructed networks. It motivates hierarchy metrics and lesion tests but does not establish a general route to hierarchical behavior.

### [R28] Ellefsen, Mouret, & Clune — modularity and forgetting

Ellefsen, K. O., Mouret, J.-B., & Clune, J. (2015). Neural modularity helps organisms evolve to learn new skills without forgetting old skills. *PLOS Computational Biology, 11*(4), e1004128. [https://doi.org/10.1371/journal.pcbi.1004128](https://doi.org/10.1371/journal.pcbi.1004128)

**Annotation.** Directly links evolved modularity to improved sequential learning and reduced catastrophic forgetting in specific tasks. It supports measuring retention and modularity together in Genesis rather than treating graph modularity as an end in itself.

### [R29] Tosh — limits of computational-efficiency explanations

Tosh, C. R. (2016). Can computational efficiency alone drive the evolution of modularity in neural networks? *Scientific Reports, 6*, 31982. [https://doi.org/10.1038/srep31982](https://doi.org/10.1038/srep31982)

**Annotation.** Finds that gradual structural evolution can remain trapped in nonmodular attractors even when modular networks would perform well, while duplication can open another path. This is an important counterweight to simple “modularity is efficient” claims.

### [R30] Elman — discrete recurrent networks

Elman, J. L. (1990). Finding structure in time. *Cognitive Science, 14*(2), 179–211. [https://doi.org/10.1207/s15516709cog1402_1](https://doi.org/10.1207/s15516709cog1402_1)

**Annotation.** Canonical demonstration that recurrent state enables temporal structure learning. It supports recurrence as a compact memory mechanism, although its supervised training method is not proposed for organism runtime.

### [R31] Beer — CTRNN dynamics

Beer, R. D. (1995). On the dynamics of small continuous-time recurrent neural networks. *Adaptive Behavior, 3*(4), 469–509. [https://doi.org/10.1177/105971239500300405](https://doi.org/10.1177/105971239500300405)

**Annotation.** Provides a dynamical-systems analysis of small CTRNNs. It establishes their expressive behavioral dynamics but also implies additional numerical-integration semantics that a deterministic engine must version precisely.

### [R32] Beer & Gallagher — evolved dynamical controllers

Beer, R. D., & Gallagher, J. C. (1992). Evolving dynamical neural networks for adaptive behavior. *Adaptive Behavior, 1*(1), 91–122. [https://doi.org/10.1177/105971239200100105](https://doi.org/10.1177/105971239200100105)

**Annotation.** Early evolutionary-robotics evidence that small recurrent dynamical networks can generate adaptive behavior. It supports recurrent controllers while offering no reason to hide update timing or numerical integration from the policy contract.

### [R33] Yamauchi & Beer — sequential behavior and dynamic adaptation

Yamauchi, B. M., & Beer, R. D. (1994). Sequential behavior and learning in evolved dynamical neural networks. *Adaptive Behavior, 2*(3), 219–246. [https://doi.org/10.1177/105971239400200301](https://doi.org/10.1177/105971239400200301)

**Annotation.** Shows that recurrent dynamics can produce learning-like behavioral change without synaptic plasticity. It motivates recurrent-state reset controls when Genesis experiments attempt to identify genuine weight learning.

## 19.2 Synaptic plasticity, evolved learning, and gene–learning interaction

### [R34] Hebb — associative plasticity

Hebb, D. O. (1949). *The organization of behavior: A neuropsychological theory*. Wiley.

**Annotation.** Foundational statement of activity-dependent association. It motivates local correlation rules but does not by itself provide normalization, temporal credit assignment, reward gating, or stability.

### [R35] Oja — normalized Hebbian learning

Oja, E. (1982). A simplified neuron model as a principal component analyzer. *Journal of Mathematical Biology, 15*(3), 267–273. [https://doi.org/10.1007/BF00275687](https://doi.org/10.1007/BF00275687)

**Annotation.** Derives a local normalization term that prevents the unbounded growth of a simple Hebbian rule under its assumptions. It supports including normalized rule primitives, not assuming Oja’s objective is appropriate for every recurrent synapse.

### [R36] Bi & Poo — experimental STDP

Bi, G.-Q., & Poo, M.-M. (1998). Synaptic modifications in cultured hippocampal neurons: Dependence on spike timing, synaptic strength, and postsynaptic cell type. *Journal of Neuroscience, 18*(24), 10464–10472. [https://doi.org/10.1523/JNEUROSCI.18-24-10464.1998](https://doi.org/10.1523/JNEUROSCI.18-24-10464.1998)

**Annotation.** Canonical biological evidence that relative spike timing affects synaptic change. It supports STDP’s plausibility but not its computational necessity for rate-based Genesis organisms.

### [R37] Gerstner, Kempter, van Hemmen, & Wagner — temporal coding rule

Gerstner, W., Kempter, R., van Hemmen, J. L., & Wagner, H. (1996). A neuronal learning rule for sub-millisecond temporal coding. *Nature, 383*, 76–78. [https://doi.org/10.1038/383076a0](https://doi.org/10.1038/383076a0)

**Annotation.** Early theoretical formulation of timing-dependent synaptic learning. It is most relevant if Genesis later adopts event-based spiking semantics and should not be imported merely as a biological ornament.

### [R38] Song, Miller, & Abbott — competitive STDP

Song, S., Miller, K. D., & Abbott, L. F. (2000). Competitive Hebbian learning through spike-timing-dependent synaptic plasticity. *Nature Neuroscience, 3*, 919–926. [https://doi.org/10.1038/78829](https://doi.org/10.1038/78829)

**Annotation.** Shows how STDP can drive competition and structure in a modeled network. It also illustrates that plasticity outcomes depend on weight dependence, spike statistics, and bounds rather than the timing window alone.

### [R39] Izhikevich — dopamine-modulated STDP

Izhikevich, E. M. (2007). Solving the distal reward problem through linkage of STDP and dopamine signaling. *Cerebral Cortex, 17*(10), 2443–2452. [https://doi.org/10.1093/cercor/bhl152](https://doi.org/10.1093/cercor/bhl152)

**Annotation.** Canonical three-factor demonstration linking an eligibility-like synaptic trace to delayed neuromodulatory reward. It strongly motivates eligibility traces and modulatory gating, while its spiking details are not mandatory for Genesis.

### [R40] Farries & Fairhall — modulated STDP reinforcement learning

Farries, M. A., & Fairhall, A. L. (2007). Reinforcement learning with modulated spike timing–dependent synaptic plasticity. *Journal of Neurophysiology, 98*(6), 3648–3665. [https://doi.org/10.1152/jn.00364.2007](https://doi.org/10.1152/jn.00364.2007)

**Annotation.** Analyzes reinforcement learning with a global modulatory factor and timing-local synaptic updates. It supports three-factor architectures while warning that signal timing and baseline terms are integral to stability.

### [R41] Soltoggio, Bullinaria, Mattiussi, Dürr, & Floreano — evolved neuromodulation

Soltoggio, A., Bullinaria, J. A., Mattiussi, C., Dürr, P., & Floreano, D. (2008). Evolutionary advantages of neuromodulated plasticity in dynamic, reward-based scenarios. In S. Bullock, J. Noble, R. Watson, & M. A. Bedau (Eds.), *Artificial Life XI: Proceedings of the Eleventh International Conference on the Simulation and Synthesis of Living Systems* (pp. 569–576). MIT Press.

**Annotation.** Primary evolutionary evidence that modulatory neurons can outperform unmodulated plasticity in dynamic reward tasks. Its benefits are conditional on task structure and reward representation.

### [R42] Risi, Hughes, & Stanley — novelty and plasticity

Risi, S., Hughes, C. E., & Stanley, K. O. (2010). Evolving plastic neural networks with novelty search. *Adaptive Behavior, 18*(6), 470–491. [https://doi.org/10.1177/1059712310379923](https://doi.org/10.1177/1059712310379923)

**Annotation.** Shows that objective search can exploit static shortcuts instead of evolving learning and that novelty search can recover adaptive solutions in studied tasks. It motivates deceptive-task diagnostics, not novelty as a universal replacement for selection.

### [R43] Risi & Stanley — indirect encoding of learning rules

Risi, S., & Stanley, K. O. (2010). Indirectly encoding neural plasticity as a pattern of local rules. In S. Doncieux, B. Girard, A. Guillot, J. Hallam, J.-A. Meyer, & J.-B. Mouret (Eds.), *From Animals to Animats 11* (Lecture Notes in Computer Science, Vol. 6226, pp. 533–543). Springer. [https://doi.org/10.1007/978-3-642-15193-4_50](https://doi.org/10.1007/978-3-642-15193-4_50)

**Annotation.** Extends geometric indirect encoding from weights to local plasticity rules. It supports compact rule-pattern generation as a later branch, especially when plasticity itself has spatial regularity.

### [R44] Miconi, Stanley, & Clune — differentiable plasticity

Miconi, T., Stanley, K. O., & Clune, J. (2018). Differentiable plasticity: Training plastic neural networks with backpropagation. In *Proceedings of the 35th International Conference on Machine Learning* (PMLR 80, pp. 3559–3568). [https://proceedings.mlr.press/v80/miconi18a.html](https://proceedings.mlr.press/v80/miconi18a.html)

**Annotation.** Demonstrates meta-optimization of local plasticity coefficients by differentiating through lifetime updates. It supports the expressive value of plastic networks; it does not imply that Genesis organisms should run backpropagation.

### [R45] Najarro & Risi — evolved synapse-specific Hebbian rules

Najarro, E., & Risi, S. (2020). Meta-learning through Hebbian plasticity in random networks. *Advances in Neural Information Processing Systems, 33*. [https://proceedings.neurips.cc/paper/2020/hash/ee23e7ad9b473ad072d57aaa9b2a5222-Abstract.html](https://proceedings.neurips.cc/paper/2020/hash/ee23e7ad9b473ad072d57aaa9b2a5222-Abstract.html)

**Annotation.** Evolves a very large set of synapse-specific rule parameters and demonstrates rapid self-organization and damage adaptation in selected tasks. It establishes possibility, while its parameter count argues for shared rule groups in a many-organism simulation.

### [R46] Floreano & Urzelai — online self-organization in robots

Floreano, D., & Urzelai, J. (2000). Evolutionary robots with on-line self-organization and behavioral fitness. *Neural Networks, 13*(4–5), 431–443. [https://doi.org/10.1016/S0893-6080(00)00032-0](https://doi.org/10.1016/S0893-6080(00)00032-0)

**Annotation.** Compares inherited parameters with inherited self-organization mechanisms in evolutionary robotics. It supports evolving learning mechanisms but does not settle how much initial structure versus lifetime plasticity Genesis should encode.

### [R47] Nolfi & Parisi — adaptation to changing environments

Nolfi, S., & Parisi, D. (1996). Learning to adapt to changing environments in evolving neural networks. *Adaptive Behavior, 5*(1), 75–98. [https://doi.org/10.1177/105971239600500104](https://doi.org/10.1177/105971239600500104)

**Annotation.** Studies evolutionary emergence of lifetime adaptation in embodied agents under environmental change. It is directly relevant to Genesis’s environmental-switch and retention experiments.

### [R48] Nolfi, Parisi, & Elman — learning and evolution

Nolfi, S., Parisi, D., & Elman, J. L. (1994). Learning and evolution in neural networks. *Adaptive Behavior, 3*(1), 5–28. [https://doi.org/10.1177/105971239400300102](https://doi.org/10.1177/105971239400300102)

**Annotation.** Early primary work on interaction between evolutionary and lifetime adaptation in neural agents. It helps ground the report’s separation of inherited priors, learning dynamics, and acquired state.

### [R49] Floreano & Mondada — evolved plastic neurocontrollers

Floreano, D., & Mondada, F. (1996). Evolution of plastic neurocontrollers for situated agents. In P. Maes, M. J. Mataric, J.-A. Meyer, J. Pollack, & S. W. Wilson (Eds.), *From Animals to Animats 4: Proceedings of the Fourth International Conference on Simulation of Adaptive Behavior* (pp. 401–410). MIT Press.

**Annotation.** Embodied evolutionary-robotics evidence for plastic controllers in situated tasks. The result supports ecological evaluation of learning rather than testing plasticity only on abstract supervised problems.

### [R50] Stanley, Bryant, & Miikkulainen — adaptive synapses in neuroevolution

Stanley, K. O., Bryant, B. D., & Miikkulainen, R. (2003). Evolving adaptive neural networks with and without adaptive synapses. In *Proceedings of the 2003 Congress on Evolutionary Computation* (Vol. 4, pp. 2557–2564). IEEE. [https://doi.org/10.1109/CEC.2003.1299410](https://doi.org/10.1109/CEC.2003.1299410)

**Annotation.** Directly compares evolved adaptive and nonadaptive neural controllers. It supports task-dependent evaluation and reinforces that plasticity is not automatically beneficial.

### [R51] Hinton & Nowlan — learning guiding evolution

Hinton, G. E., & Nowlan, S. J. (1987). How learning can guide evolution. *Complex Systems, 1*, 495–502. [https://www.complex-systems.com/abstracts/v01_i03_a06/](https://www.complex-systems.com/abstracts/v01_i03_a06/)

**Annotation.** Canonical computational illustration of a Baldwin-effect mechanism in a stylized search problem. It shows how learning can smooth selection, but its result is highly dependent on representation and learning cost.

### [R52] Baldwin — original organic-selection argument

Baldwin, J. M. (1896). A new factor in evolution, Parts I and II. *The American Naturalist, 30*(354–355), 441–451, 536–553. Part I: [https://doi.org/10.1086/276408](https://doi.org/10.1086/276408). Part II: [https://doi.org/10.1086/276428](https://doi.org/10.1086/276428).

**Annotation.** Original source for the idea that plastic accommodation can change which hereditary variants survive without inheritance of acquired characters. Modern algorithmic uses should not collapse this into Lamarckian inheritance.

### [R53] Waddington — genetic assimilation of an acquired character

Waddington, C. H. (1953). Genetic assimilation of an acquired character. *Evolution, 7*(2), 118–126. [https://doi.org/10.1111/j.1558-5646.1953.tb00070.x](https://doi.org/10.1111/j.1558-5646.1953.tb00070.x)

**Annotation.** Classic experimental foundation for genetic assimilation through selection on developmental variation. It motivates reaction-norm and plasticity-cost measurements rather than inferring assimilation from behavioral convergence alone.

### [R54] Waddington — bithorax assimilation

Waddington, C. H. (1956). Genetic assimilation of the bithorax phenotype. *Evolution, 10*(1), 1–13. [https://doi.org/10.1111/j.1558-5646.1956.tb02824.x](https://doi.org/10.1111/j.1558-5646.1956.tb02824.x)

**Annotation.** A second canonical artificial-selection study of environmentally induced phenotypes becoming expressed without the original induction. It reinforces that assimilation is a population-level evolutionary result, not direct copying of learned weights.

### [R55] Crispo — distinguishing Baldwin effect and assimilation

Crispo, E. (2007). The Baldwin effect and genetic assimilation: Revisiting two mechanisms of evolutionary change mediated by phenotypic plasticity. *Evolution, 61*(11), 2469–2479. [https://doi.org/10.1111/j.1558-5646.2007.00203.x](https://doi.org/10.1111/j.1558-5646.2007.00203.x)

**Annotation.** Clarifies differences among organic selection, genetic accommodation, and reduced plasticity through genetic assimilation. It is a key terminological safeguard for Genesis experiments.

### [R56] Gruau & Whitley — learning in cellular development

Gruau, F., & Whitley, D. (1993). Adding learning to the cellular development process: A comparative study. *Evolutionary Computation, 1*(3), 213–233. [https://doi.org/10.1162/evco.1993.1.3.213](https://doi.org/10.1162/evco.1993.1.3.213)

**Annotation.** Integrates learning with a developmental neural encoding and compares evolutionary consequences. It is an important precursor to later evo-devo learning systems, though not a template for Genesis’s first implementation.

### [R57] Sasaki & Tokoro — Darwinian versus Lamarckian inheritance under change

Sasaki, T., & Tokoro, M. (1999). Evolving learnable neural networks under changing environments with various rates of inheritance of acquired characters: Comparison of Darwinian and Lamarckian evolution. *Artificial Life, 5*(3), 203–223. [https://doi.org/10.1162/106454699568746](https://doi.org/10.1162/106454699568746)

**Annotation.** Directly shows that the best acquired-trait inheritance rate can depend on environmental dynamics and that strong Lamarckian inheritance may reduce stability. It strongly supports making inheritance mode an explicit experiment rather than a default.

### [R58] Whitley, Gordon, & Mathias — Baldwinian and Lamarckian optimization

Whitley, D., Gordon, V. S., & Mathias, K. (1994). Lamarckian evolution, the Baldwin effect and function optimization. In Y. Davidor, H.-P. Schwefel, & R. Männer (Eds.), *Parallel Problem Solving from Nature—PPSN III* (Lecture Notes in Computer Science, Vol. 866, pp. 6–15). Springer. [https://doi.org/10.1007/3-540-58484-6_245](https://doi.org/10.1007/3-540-58484-6_245)

**Annotation.** Demonstrates that Baldwinian and Lamarckian hybrid search can excel on different landscapes and that local-search speed is not the only criterion. It supports measuring generalization and environmental transfer, not only immediate fitness.

## 19.3 Diversity search, alternative graph encodings, social transmission, and deterministic implementation

### [R59] Lehman & Stanley — novelty search

Lehman, J., & Stanley, K. O. (2011). Abandoning objectives: Evolution through the search for novelty alone. *Evolutionary Computation, 19*(2), 189–223. [https://doi.org/10.1162/EVCO_a_00025](https://doi.org/10.1162/EVCO_a_00025)

**Annotation.** Canonical novelty-search paper showing that behavioral divergence can escape deceptive objective landscapes. It does not guarantee open-endedness and depends critically on behavior characterization and archive policy.

### [R60] Mouret & Clune — MAP-Elites

Mouret, J.-B., & Clune, J. (2015). Illuminating search spaces by mapping elites. *arXiv preprint arXiv:1504.04909*. [https://arxiv.org/abs/1504.04909](https://arxiv.org/abs/1504.04909)

**Annotation.** Introduces MAP-Elites as a simple archive of high-performing solutions across behavior or phenotype descriptors. It is relevant as an analysis and diversity-maintenance tool, but its descriptor choices can become strong designer-imposed biases.

### [R61] Cully, Clune, Tarapore, & Mouret — behavioral repertoires and damage adaptation

Cully, A., Clune, J., Tarapore, D., & Mouret, J.-B. (2015). Robots that can adapt like animals. *Nature, 521*, 503–507. [https://doi.org/10.1038/nature14422](https://doi.org/10.1038/nature14422)

**Annotation.** Demonstrates rapid robot damage recovery using a precomputed behavioral repertoire and online search. It is strong evidence for QD-generated repertoires, but the archive is an external adaptation mechanism rather than an organism-local neural learning rule.

### [R62] Pugh, Soros, & Stanley — quality-diversity framework

Pugh, J. K., Soros, L. B., & Stanley, K. O. (2016). Quality diversity: A new frontier for evolutionary computation. *Frontiers in Robotics and AI, 3*, 40. [https://doi.org/10.3389/frobt.2016.00040](https://doi.org/10.3389/frobt.2016.00040)

**Annotation.** Formalizes QD, compares representative algorithms, and shows sensitivity to behavior-characterization alignment. It supports careful descriptor ablations rather than treating QD as a neutral add-on.

### [R63] Turner & Miller — recurrent Cartesian genetic programming

Turner, A. J., & Miller, J. F. (2017). Recurrent Cartesian Genetic Programming of artificial neural networks. *Genetic Programming and Evolvable Machines, 18*, 185–212. [https://doi.org/10.1007/s10710-016-9276-6](https://doi.org/10.1007/s10710-016-9276-6)

**Annotation.** Provides a credible alternative variable-graph representation with inactive genes, positional structure, and recurrence. It is a useful architecture comparator because its homology and mutation semantics differ from NEAT.

### [R64] Khan, Ahmad, Khan, & Miller — CGP and fast-learning networks

Khan, M. M., Ahmad, A. M., Khan, G. M., & Miller, J. F. (2013). Fast learning neural networks using Cartesian Genetic Programming. *Neurocomputing, 121*, 274–289. [https://doi.org/10.1016/j.neucom.2013.04.005](https://doi.org/10.1016/j.neucom.2013.04.005)

**Annotation.** Evolves CGP-based neural structures with learning behavior and provides evidence that alternate graph genomes can support adaptation. Its benchmark setting should not be generalized to world-scale ALife without direct comparison.

### [R65] Saunders, Angeline, & Pollack — GNARL conference paper

Saunders, G. M., Angeline, P. J., & Pollack, J. B. (1993). Structural and behavioral evolution of recurrent networks. In *Advances in Neural Information Processing Systems 6* (pp. 88–95). Morgan Kaufmann. [https://papers.nips.cc/paper/1993/hash/c8ed21db4f678f3b13b9d5ee16489088-Abstract.html](https://papers.nips.cc/paper/1993/hash/c8ed21db4f678f3b13b9d5ee16489088-Abstract.html)

**Annotation.** Introduces structurally unconstrained recurrent-network evolution in a behavior task and precedes the expanded journal treatment. It supports a mutation-driven recurrent graph baseline with no dependence on innovation-number crossover.

### [R66] Billard & Dautenhahn — embodied imitation and communication

Billard, A., & Dautenhahn, K. (1999). Experiments in learning by imitation—Grounding and use of communication in robotic agents. *Adaptive Behavior, 7*(3–4), 415–438. [https://doi.org/10.1177/105971239900700311](https://doi.org/10.1177/105971239900700311)

**Annotation.** Demonstrates embodied imitation, joint context, and associative learning in groups of robotic agents. It supports the need for observable action and shared context, while explicitly not proving a general biological imitation mechanism.

### [R67] Gonzalez, Watson, & Bullock — evolution of social learning

Gonzalez, M., Watson, R. A., & Bullock, S. (2017). Minimally sufficient conditions for the evolution of social learning and the emergence of non-genetic evolutionary systems. *Artificial Life, 23*(4), 493–517. [https://doi.org/10.1162/ARTL_a_00244](https://doi.org/10.1162/ARTL_a_00244)

**Annotation.** Shows in a simple agent-based model that social learning can evolve under particular survival-selection, variation, and transmission conditions. It supports genotype–phenotype disengagement and transmission-chain metrics but does not establish that neural agents will discover imitation without suitable observability.

### [R68] Winfield & Erbas — behavioral traditions in robots

Winfield, A. F. T., & Erbas, M. D. (2011). On embodied memetic evolution and the emergence of behavioural traditions in robots. *Memetic Computing, 3*(4), 261–270. [https://doi.org/10.1007/s12293-011-0063-x](https://doi.org/10.1007/s12293-011-0063-x)

**Annotation.** Provides embodied multi-robot experiments in repeated imitation with variation and selection of behaviors. It is relevant to later Genesis cultural-transmission tests but relies on a deliberately implemented imitation pathway.

### [R69] Rust Project Developers — `HashMap`

Rust Project Developers. (2026). `std::collections::HashMap`. *The Rust Standard Library documentation*. Accessed August 4, 2026. [https://doc.rust-lang.org/stable/std/collections/struct.HashMap.html](https://doc.rust-lang.org/stable/std/collections/struct.HashMap.html)

**Annotation.** Official documentation states that the default map is randomly seeded and does not specify a canonical hashing or iteration order. It supports prohibiting map traversal from defining mutation, serialization, evaluation, or accumulation order.

### [R70] Rust Project Developers — `BTreeMap`

Rust Project Developers. (2026). `std::collections::BTreeMap`. *The Rust Standard Library documentation*. Accessed August 4, 2026. [https://doc.rust-lang.org/stable/std/collections/struct.BTreeMap.html](https://doc.rust-lang.org/stable/std/collections/struct.BTreeMap.html)

**Annotation.** Official ordered-map documentation supports key-ordered iteration when canonical traversal is needed. Genesis should still define its own serialized key encoding and must not treat an implementation container as the persistence format.

### [R71] Rust Project Developers — `f32`

Rust Project Developers. (2026). Primitive type `f32`. *The Rust Standard Library documentation*. Accessed August 4, 2026. [https://doc.rust-lang.org/stable/std/primitive.f32.html](https://doc.rust-lang.org/stable/std/primitive.f32.html)

**Annotation.** Documents IEEE-754 properties alongside caveats for NaNs and the precision of several mathematical functions across platforms or versions. It supports an explicit fixed-point or tightly constrained, versioned floating-point policy with conformance tests.

---

## Bibliographic synthesis

No single cited system combines all properties required by Genesis: variable heritable topology, local lifetime plasticity, exact learned-state persistence, deterministic parallel structural mutation, cross-platform replay, module duplication, and later social transmission. The recommended architecture is therefore an engineering synthesis constrained by the strongest recurring findings:

- historical alignment and protection can make structural complexification workable, but are not synonymous with one NEAT implementation;
- indirect encodings help when their regularity bias matches the problem;
- deletion, cost, and duplication materially affect bloat and modularity;
- recurrent dynamics and synaptic learning must be experimentally separated;
- bounded three-factor plasticity is a credible local learning foundation, but signal design remains environment-dependent;
- Darwinian and Lamarckian inheritance have conditional tradeoffs;
- novelty and quality-diversity methods preserve alternatives but do not supply open-endedness;
- exact replay requires controller runtime state and deterministic numeric, ordering, and persistence policies beyond a deterministic PRNG.

