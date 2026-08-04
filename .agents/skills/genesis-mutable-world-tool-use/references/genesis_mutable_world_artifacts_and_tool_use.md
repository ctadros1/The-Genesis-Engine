# The Genesis Engine: Mutable Worlds, Artifacts, and Emergent Tool Use

**Engineering-oriented scientific review**  
**Date:** 2026-08-04  
**Encoding:** UTF-8  
**Primary design constraint:** deterministic, versioned, replayable artificial life without scripted technology trees or crafting recipes

---

## Evidence calibration

This report distinguishes three kinds of claims:

- **Evidence-backed** — directly supported by empirical, theoretical, or established systems literature. Citations identify the relevant source.
- **Engineering synthesis** — a design conclusion inferred from several evidence-backed principles and the stated constraints of The Genesis Engine.
- **Engineering hypothesis** — a plausible but unvalidated proposal that requires benchmark or evolutionary testing inside the engine.

The distinction matters. Biological and cognitive-science findings can justify design goals, but they rarely determine a unique software architecture. Likewise, a mechanism that produces impressive behavior in a task-specific robotics paper may still be inappropriate for an open-ended artificial-life world if the target behavior was built into the reward, object labels, or construction rules.

---

## Decision summary

The strongest first implementation is **not** a miniature chemistry simulator, full voxel world, or general 3D rigid-body sandbox. It is a deliberately constrained hybrid:

1. A **sparse, chunked, mutable 2D terrain substrate** with elevation or height bands, material, compaction, damage, and a small number of field values.
2. **Discrete physical bodies** in continuous fixed-point 2D space, each with stable identity, simple geometry, material composition, mass, wear, and provenance.
3. A **2.5D support model** for stacking, barriers, pits, low roofs, caches, and occlusion without the control and computational burden of unconstrained 3D.
4. An explicit **bond graph** introduced only after carrying and placement work; composites are connected components, not named crafted items.
5. Four core manipulation capabilities: **grasp/release, apply force, ingest/exchange matter, and locomotion/body orientation**. Most named actions should be observer interpretations of these primitives.
6. Runtime types for matter and mechanics, but **no privileged `Tool`, `Structure`, `Resource`, `Recipe`, `Technology`, or `Owner` types**. Those are relational or historical classifications derived by observers.
7. **Property-based, local, conservative transformations**. Rules must depend on physical state and broad material relations, not object names, material IDs, or recognized action sequences.
8. A deterministic tick protocol based on **snapshot-read intents, canonical conflict groups, fixed-point accumulation, fixed solver iteration counts, canonical spawn order, and counter/keyed random draws**.
9. A successor save format that stores mutable world state explicitly. Procedural regeneration may remain an optimization for untouched chunks, but never an implicit substitute for missing state.
10. A staged experimental program that does not reward “tool use.” It creates ecological problems and measures, after the fact, whether artifacts causally mediate access, are reused, modified, assembled, socially transmitted, inherited environmentally, or improved cumulatively.

The principal scientific risk is **semantic leakage**: implementing a recipe system while calling it physics. The principal engineering risk is **state explosion** from fragments, contacts, bonds, dirty terrain chunks, environmental fields, and event history. The principal evolutionary risk is **control-space overload**: a physically general action interface that no evolving controller can learn to use.

---

## Table of contents

1. [Executive summary](#1-executive-summary)
2. [Design philosophy](#2-design-philosophy)
3. [Affordances and embodied cognition](#3-affordances-and-embodied-cognition)
4. [Lessons from existing artificial worlds](#4-lessons-from-existing-artificial-worlds)
5. [Minimal material ontology](#5-minimal-material-ontology)
6. [Object and artifact ontology](#6-object-and-artifact-ontology)
7. [Action primitives](#7-action-primitives)
8. [Property-based transformations](#8-property-based-transformations)
9. [Structures, stigmergy, and niche construction](#9-structures-stigmergy-and-niche-construction)
10. [Tool-use hierarchy and measurement](#10-tool-use-hierarchy-and-measurement)
11. [World-representation comparison](#11-world-representation-comparison)
12. [Recommended minimal world model](#12-recommended-minimal-world-model)
13. [Deterministic interaction-resolution protocol](#13-deterministic-interaction-resolution-protocol)
14. [Stable identity, save, restore, and migration](#14-stable-identity-save-restore-and-migration)
15. [Dependency-ordered experiment plan](#15-dependency-ordered-experiment-plan)
16. [Metrics and ablations](#16-metrics-and-ablations)
17. [Performance and state-growth risks](#17-performance-and-state-growth-risks)
18. [Anti-patterns that would script progress](#18-anti-patterns-that-would-script-progress)
19. [Open questions](#19-open-questions)
20. [Annotated bibliography](#20-annotated-bibliography)

---

# 1. Executive summary

## 1.1 What the world must make possible

A mutable world for The Genesis Engine should make the following causal loop possible without naming any step:

> An organism perceives a persistent environmental feature, acts on matter through its body, changes the future action possibilities available to itself or others, and those changes remain available long enough to affect later behavior, reproduction, learning, or social transmission.

That loop is more fundamental than “crafting.” Tool use, construction, caching, trail formation, nest building, territorial marking, traps, external memory, and inherited artifacts are different observer descriptions of the same broad phenomenon: **agents altering the distribution of affordances through persistent environmental action**.

Ecological psychology treats affordances as action possibilities that arise in the relation between environmental features and an organism’s abilities, not merely as labels attached to objects [R1](#r1) [R2](#r2) [R3](#r3). Extended and distributed cognition research likewise shows why stable environmental structures can participate in memory and problem solving rather than serving only as passive scenery [R4](#r4) [R5](#r5) [R6](#r6). Niche-construction theory adds the evolutionary consequence: organism-caused environmental changes can persist and modify selection pressures for later individuals, creating ecological inheritance [R24](#r24) [R25](#r25).

**Evidence-backed implication:** persistent, perceivable consequences of action are scientifically more important than a large inventory of semantic object classes.

**Engineering synthesis:** the first mutable-world architecture should prioritize stable identity, placement, resistance, support, damage, and persistence before adding chemistry, heat, fluids, flexible materials, or elaborate geometry.

## 1.2 Minimal runtime ontology

The recommended runtime ontology contains six primary concepts:

| Runtime concept | Purpose | What it deliberately does not mean |
|---|---|---|
| `TerrainCell` / mutable chunk cell | Spatial substrate: elevation, material, compaction, damage, occupancy/support data | Not a named biome feature or resource node |
| `MaterialDef` | Versioned physical and metabolic property bundle | Not a recipe ingredient or technology tier |
| `PhysicalBody` | Stable-ID bounded lump with geometry, pose, state, material, wear, provenance | Not automatically an item, tool, resource, or artifact |
| `Bond` | Physical constraint between two bodies or a body and terrain | Not a recipe edge or semantic attachment socket |
| `Organism` | Living body, effectors, metabolism, controller, learned state, lineage | Not exempt from world mechanics unless explicitly justified |
| `FieldLayer` | Coarse scalar/vector environmental influence such as light, heat, odor, moisture, or deposited signal | Not a direct instruction or named waypoint |

Everything else should normally be derived:

- A **resource** is matter that an organism can exploit under current ecological circumstances.
- An **item** is a portable body under a user-interface or observer convention.
- A **tool** is an external object whose controlled use causally mediates an effect or information transfer.
- A **structure** is a persistent spatial or bonded arrangement with a demonstrated environmental function.
- A **container** is a body or arrangement that creates containment through geometry and collision.
- A **composite** is a connected component of the bond graph.
- **Debris** is a fragment or aggregate state; it does not need a separate metaphysical category.
- **Ownership** is, at most, an organism- or observer-level social relation inferred from possession, defense, marking, or repeated use. Physics should not enforce supernatural property rights.

This design is intentionally austere. It prevents the engine from deciding in advance that a particular body is a hammer, wall, food resource, storage vessel, or cultural symbol.

## 1.3 Minimal material and body properties

The recommended first material layer needs only enough differentiation to support meaningful selection among objects and persistent environmental modification:

- density;
- hardness;
- toughness/fracture threshold;
- static and dynamic friction;
- bond affinity or adhesion class, initially optional;
- nutritional energy;
- toxicity or physiological penalty, if ecological tradeoffs require it;
- compressive/support strength;
- wear rate.

The body layer adds geometry, volume, pose, derived mass, damage, provenance, and—if 2.5D support is used—height interval and support load. Sharpness should initially be **derived from geometry, hardness, and wear**, not stored as a semantic “blade” flag. Buoyancy should be derived from density if fluids are later introduced. Conductivity, permeability, thermal capacity, combustion, elasticity, and complex phase behavior should be deferred until experiments show they create new behavior rather than merely more state.

Values should be quantized fixed point or small categorical bands. Organisms should perceive noisy or coarsened cues such as apparent extent, outline, resistance, heft, texture, temperature, odor, and taste—not internal material IDs or exact numerical attributes.

## 1.4 Minimal action layer

The smallest practical action set is neither a list of verbs such as `cut`, `dig`, `stack`, and `build`, nor unrestricted joint torques. The first approach scripts semantics; the second produces a control problem that is likely too difficult for evolving neural controllers.

The recommended compromise is a set of **physics-grounded mid-level motor primitives**:

1. `grasp(target, effector, grip_level)`;
2. `release(effector)`;
3. `apply_force(target_or_world, contact_region, direction_bin, magnitude_bin, duration_bin)`;
4. `ingest_or_exchange(target, amount_bin)`;
5. existing locomotion, body orientation, and optional emission/deposition mechanisms.

From these primitives:

- pick up = grasp plus upward or transport motion;
- carry = maintained grasp plus locomotion;
- place = controlled low-speed release;
- push, pull, strike, scrape, and dig = different force profiles and contact geometries;
- stack = placement followed by stable support;
- cut = concentrated tangential force whose pressure and geometry exceed a material threshold;
- break = accumulated damage or impulse exceeding toughness;
- attach = sustained compatible contact that forms a bond;
- store = placement within a containing geometry;
- give/take = transfer or contested grasp;
- point = body orientation or gesture perceived by another organism.

No high-level `use_tool`, `craft`, `build`, `give`, `store`, or `teach` action should exist in the simulation core.

## 1.5 Recommended representation

A hybrid 2.5D representation best balances expressiveness and tractability:

- sparse chunked terrain grid;
- continuous fixed-point body positions in the horizontal plane;
- simple collider palette: discs, capsules, boxes, and a small set of convex polygons;
- quantized orientation;
- discrete or fixed-point height intervals for support, stacking, pits, low roofs, and occlusion;
- spatial hash or uniform query grid;
- explicit bond/support graph;
- low-resolution environmental fields;
- entity-component storage internally, but canonical iteration by stable identity.

A full voxel world makes construction intuitive but tends to turn blocks into semantic atoms and dramatically increases mutable-state size. A continuous 3D rigid-body world is expressive but multiplies perception, manipulation, collision, stability, controller output dimensionality, and cross-platform determinism problems. Particle or cellular-material systems can produce rich material dynamics, but they make stable artifact identity and bounded state growth difficult. A graph world is efficient but weak at grounded spatial affordances. The hybrid model is not maximally realistic; it is the smallest credible platform for evolutionary experimentation with artifacts.

## 1.6 Deterministic persistence

The existing invariant—terrain is regenerated from seed and configuration—must be replaced by a stricter one:

> Every state variable capable of affecting future simulation behavior is either serialized explicitly or reconstructible by a version-pinned, verified deterministic derivation whose inputs are serialized explicitly.

Touched terrain chunks must therefore be stored as materialized state or exact deltas against a versioned generator. Objects, bonds, fields, learned organism state, ID allocators, pending events, and mutable provenance must be stored. Absence of a section must never silently mean “empty,” “default,” or “regenerate.”

The practical architecture is authoritative snapshots plus a bounded event journal, not pure event sourcing. Snapshots permit fail-closed restore and bounded replay; event logs support auditing, lineage analysis, causal branching, and incremental checkpointing. Each section should be canonically ordered and checksummed, with a root manifest hash. Migration should be one-way, offline, version-dispatched, never in-place, and should refuse ambiguity.

## 1.7 Experimental order

The dependency order is important. Composite tools and culture cannot be evaluated credibly before basic perception, manipulation, identity, and persistence are validated.

1. basic object perception;
2. carrying;
3. choice by physical property;
4. object-assisted foraging;
5. repeated reuse;
6. placement and caching;
7. persistent barriers or structures;
8. stigmergic coordination;
9. composite objects;
10. social transmission;
11. environmental inheritance;
12. cumulative modification.

Every experiment should include deterministic counterfactual replays. If removing, substituting, relocating, or resetting an object does not reduce access, efficiency, or persistence, the event should not be counted as tool use or structural function merely because an organism touched an object.

---

# 2. Design philosophy

## 2.1 Author invariants and gradients, not outcomes

The governing design principle should be stated more precisely than “author physics, not progress”:

> Author conserved quantities, local interactions, bodily constraints, perceptual channels, ecological gradients, and persistence rules. Do not author privileged semantic solutions, named action sequences, or stage transitions.

No artificial world is neutral. Choosing gravity, friction, object sizes, organism effectors, resource renewal, mortality, sensing radius, and spatial dimensionality already shapes what can evolve. The goal is therefore not to eliminate authorship. It is to keep authorship at a level where many behavioral solutions are possible and where the simulator does not recognize or reward the designer’s intended interpretation.

A useful test is **counterfactual substitutability**. Suppose an experiment appears to elicit a “hammer.” Ask whether another object with comparable mass, hardness, and geometry could work. If only a body carrying an internal `HAMMER` tag works, the system is scripted. If a continuum of objects works with graded efficiency, the system exposes an affordance.

A second test is **behavioral route multiplicity**. A breakable barrier may be crossed by striking, repeated scraping, pushing another heavy object, digging around it, waiting for environmental weakening, or social cooperation. A design with one exact sequence is a puzzle recipe, even if each step has a physical-sounding name.

A third test is **observer independence**. Offline analysis may call an arrangement a wall or a body a tool. That label must not be read by the organism controller, physics engine, mutation system, resource-spawn logic, or reward function.

## 2.2 Minimality means causal sufficiency, not fewest lines of code

A minimal world is not the world with the fewest properties. It is the smallest world in which distinct causal mechanisms can be expressed and distinguished experimentally. For example:

- Mass without friction may make pushing possible but object placement unstable.
- Friction without persistent identity permits barriers but not artifact reuse or traditions.
- Stable identity without provenance permits reuse measurement but weakens analysis of modification lineages.
- Placement without support or collision may create visual piles that have no physical function.
- Social observation without controlled exposure cannot distinguish imitation from independent rediscovery.

The right target is a **minimal causal basis**. Each added property or action should answer all of the following:

1. What previously impossible behavior does it enable?
2. Can that behavior be enabled by an already existing primitive?
3. Does it add a new perceptual variable that organisms can plausibly access?
4. Does it multiply the interaction state space or solver complexity?
5. Can it be represented coarsely without losing its behavioral role?
6. Does it create a hidden recipe or arbitrary compatibility relation?
7. Can its effect be isolated by ablation?

## 2.3 Separate simulation semantics from observer semantics

The engine should maintain three distinct layers:

### Layer A — physical state

Material, pose, geometry, velocity, force, contact, damage, bond, support, temperature if modeled, and field values. These variables determine state transition.

### Layer B — organism-accessible state

Noisy, local, body-relative perception derived from physical state. Examples include apparent size, motion, tactile resistance, estimated load, contact pressure, odor, and temperature. This layer constrains embodied behavior.

### Layer C — observer and analytics state

Derived classifications such as tool episode, structure function, cache, territory, artifact lineage, tradition, teaching candidate, dependency chain, or cumulative improvement. This layer must never feed back into Layers A or B unless a future scientific experiment explicitly introduces a social belief representation inside organisms.

This separation is an engineering defense against semantic leakage. It also improves scientific validity: classifications can be revised offline without changing the world’s causal rules.

## 2.4 Preserve causal transparency

Open-ended systems can become analytically opaque. The world should therefore log low-level causal events rather than only high-level summaries:

- contacts and impulses;
- grasps and releases;
- body displacement under control;
- force applications;
- material transfer;
- damage accumulation;
- fractures and fragment provenance;
- bond creation and failure;
- support changes;
- terrain deltas;
- field deposits and decay;
- object visibility and observation opportunities;
- organism action intents and resolved outcomes.

High-level interpretations can be reconstructed from these records. The reverse is not true. A log entry saying “organism used tool” is useless for auditing unless the physical causal chain is available.

## 2.5 Evolutionary accessibility is a first-class constraint

A physically expressive action space can be evolutionarily inaccessible. Sims’s virtual creatures demonstrated that rich morphology and behavior can evolve in simulated physics, but those systems also illustrate how strongly objective functions, body encodings, and simulator details structure the resulting behavior [R26](#r26) [R27](#r27). Evolutionary robotics further shows that body and controller constraints can change search difficulty and robustness [R28](#r28).

The Genesis Engine should therefore avoid both extremes:

- **Too semantic:** `CRAFT_SPEAR`, `BUILD_WALL`, `USE_TOOL_ON_RESOURCE`.
- **Too low level:** continuous torques for every joint, arbitrary contact trajectories, and unrestricted 3D manipulation from the first implementation.

The proposed mid-level manipulation interface should be quantized, body-grounded, and compositional. Its expressiveness should be increased only after populations demonstrate reliable use of the previous layer.

## 2.6 Determinism is part of the scientific method

Determinism is not merely an implementation preference. Exact replay enables:

- branching counterfactual interventions at a precise tick;
- separating object causation from correlation;
- reproducing rare innovations;
- testing save/restore equivalence;
- verifying migration;
- comparing ablations from identical histories;
- attributing divergence to an intended intervention rather than update order or hidden randomness.

Counter-based random-number generators are particularly compatible with parallel deterministic simulation because random outputs can be addressed by key and counter rather than consumed from one mutable global stream [R45](#r45). Off-the-shelf physics engines can offer determinism under controlled construction order, compiler settings, platform constraints, and fixed algorithms, but their own documentation emphasizes that ordering and build configuration matter [R46](#r46) [R47](#r47). For a long-lived scientific simulator, determinism must be specified as an explicit protocol, not assumed from a library claim.

---

# 3. Affordances and embodied cognition

## 3.1 Affordances are relational

Gibson’s ecological approach emphasizes directly available opportunities for action in an organism’s environment [R1](#r1). Later relational accounts make the key engineering point explicit: an affordance is not just a feature of an object and not just an ability of an organism; it exists in the relation between environmental features and action capabilities [R2](#r2). A gap affords passage for one body size but not another. A body affords carrying only relative to grip strength, geometry, and locomotor stability. A protruding hard edge may afford scraping for one effector and merely collision for another.

**Evidence-backed design implication:** do not store `affordances = [CARRYABLE, CUTTING_TOOL]` on objects as authoritative simulation state.

Instead, compute or learn affordance-relevant relations from:

- object size and shape;
- organism reach and effector geometry;
- relative mass and strength;
- surface friction and grip;
- target material hardness/toughness;
- current pose and accessibility;
- energetic cost;
- learned history.

The simulator may cache broadphase feasibility such as “within reach” or “grasp geometry valid,” but these are mechanical predicates, not semantic affordance labels.

## 3.2 Affordance landscapes and ecological context

Rietveld and Kiverstein argue for a landscape of affordances embedded in sociomaterial practices [R3](#r3). The Genesis Engine does not need to implement human practices, but the broader insight is useful: the functional meaning of an artifact can depend on historical, social, and environmental context.

A stone-like body can be:

- ballast in a composite;
- a projectile;
- a barrier component;
- a cache marker;
- a substrate for scratching;
- a territorial signal;
- a heat reservoir if temperature is later modeled;
- irrelevant debris.

The physical body remains the same. What changes is its relation to organisms, targets, and learned conventions. This is why “tool” and “structure” should be post-hoc classifications based on causal use, not intrinsic classes.

## 3.3 Embodied cognition and control difficulty

Embodied cognition treats behavior as arising from interactions among brain/controller, body, and environment rather than from a detached symbolic planner. In engineering terms, morphology and environmental dynamics can offload control. A broad flat object may stabilize a stack; a cavity may constrain movement; friction can preserve placement without active control; a path can reduce locomotor cost.

Kirsh and Maglio’s distinction between pragmatic actions, which directly advance a physical goal, and epistemic actions, which simplify cognition or reveal information, broadens the design target [R6](#r6). Organisms might move an object not because the displacement is intrinsically useful, but because it reveals hidden food, marks a location, changes line of sight, tests weight, or externalizes a decision.

This creates two requirements:

1. Object actions should generate **informational consequences** as well as mechanical ones.
2. Analytics should not assume that an apparently inefficient manipulation is meaningless. It may be exploratory or epistemic.

## 3.4 Extended and distributed cognition

Clark and Chalmers argue that stable external resources can become functionally integrated into cognitive processes [R4](#r4). Hutchins shows how cognition can be distributed across people and material structure [R5](#r5). These theories do not prove that simulated agents will develop external memory, but they identify the enabling conditions:

- persistent external marks or arrangements;
- reliable access to them;
- perception-action coupling;
- lower internal-memory cost or higher reliability than purely internal representation;
- social visibility when coordination is relevant.

In the engine, possible external-memory substrates include:

- object orientation or arrangement;
- piles at branch points;
- terrain scratches or compaction;
- deposited chemical or visual fields;
- caches whose contents encode state;
- repeated structural motifs;
- maintained paths.

**Engineering hypothesis:** external memory is more likely to emerge if internal recurrent memory is costly, noisy, capacity-limited, or subject to lifetime decay. If neural memory is effectively free and perfect, environmental memory may never be selected despite being physically possible.

## 3.5 Perception should expose cues, not truth tables

An organism should not receive a vector such as:

```text
object_type = TOOL
hardness = 0.8125
can_break_target = true
owner = organism_412
```

A more embodied interface might expose:

- angular extent and coarse outline;
- distance and relative motion;
- apparent height/occlusion;
- local surface contrast or texture class;
- tactile contact force;
- slip under grip;
- proprioceptive effort while lifting or dragging;
- deformation or damage feedback;
- odor/taste cues;
- observed actions of another organism.

Exact internal properties may be unavailable or only inferable through interaction. This makes object selection a learned sensorimotor problem rather than a lookup.

Perception must still be evolutionarily tractable. A raw pixel camera is unnecessarily expensive for the first artifact experiments. A structured egocentric object/contact representation is acceptable if it contains physical cues rather than semantic function labels. A good initial representation is a fixed number of local percept slots sorted by a deterministic salience rule, with object identity hidden or represented only through temporally stable tracking tokens that do not reveal material type.

## 3.6 Affordance discovery as an experimental target

The engine should test whether organisms learn relations that generalize across bodies and contexts. Evidence of affordance discovery is stronger when behavior transfers to:

- novel object identities with familiar physical properties;
- interpolated property values;
- new combinations of size, mass, hardness, and shape;
- relocated objects;
- altered target geometry;
- different but functionally equivalent action sequences.

Failure to generalize suggests memorized identifiers, locations, or exact task configurations. This is one of the most important controls against hidden recipes.

---

# 4. Lessons from existing artificial worlds

Existing systems provide mechanisms and warnings, not a ready-made blueprint. Many successful artificial-life and embodied-AI environments deliberately encode tasks or object semantics because their research goal is benchmarking. The Genesis Engine has a different goal: observe whether persistent object-mediated behavior emerges under evolutionary and lifetime-learning dynamics.

## 4.1 Tierra and Avida: low-level possibility, authored ecological meaning

Tierra created an ecology of self-replicating programs competing for memory and CPU time [R31](#r31). Avida developed a rigorous platform for digital evolution and demonstrated evolutionary origins of complex features [R29](#r29) [R30](#r30). These systems show the scientific value of low-level hereditary programs, resource competition, mutation, lineage, and exact experimental control.

Their limitation for the present objective is embodiment. Digital organisms operate in computational instruction spaces rather than a persistent manipulable material world. In Avida experiments, rewarded Boolean tasks are explicitly selected by the experimenter. This is legitimate for studying evolutionary dynamics, but it illustrates a central caution: complex lineages can emerge while the functional task ontology remains authored.

**Lesson:** preserve Avida-like lineage rigor and experimental repeatability, but do not translate named logical tasks into named material technologies.

## 4.2 Echo and Sugarscape: simple rules can produce macro-patterns, but not necessarily open-ended artifacts

Echo and Sugarscape explored adaptive agents, resources, trade, migration, and population-level social patterns [R33](#r33) [R41](#r41). Such models demonstrate that relatively simple local rules can produce segregation, inequality, exchange, and ecological dynamics.

However, tokenized resources and fixed exchange rules do not automatically create grounded construction or hierarchical artifact organization. Analyses of Echo questioned whether its apparent organization satisfied stronger criteria for complex adaptive structure [R33](#r33).

**Lesson:** aggregate social complexity is not evidence of material open-endedness. The ontology must support persistent causal reconfiguration of the world, and the analytics must distinguish labels from mechanisms.

## 4.3 Creatures and Polyworld-like systems: embodied behavior needs persistent world consequences

The Creatures ecosystem combined neural control, genetics, physiology, and a persistent interactive environment [R32](#r32). Polyworld and related systems explored embodied neural agents in spatial ecologies. Their importance lies in coupling evolving agents to perception and action rather than treating fitness as a static function.

The common limitation is that object interactions and world semantics are usually authored for a game-like environment. Agents may learn to approach, eat, push, or interact with predetermined object classes, but the world rarely supports generic material transformation, composite assembly, and ecological inheritance.

**Lesson:** persistent embodiment is necessary but not sufficient. Objects must have mechanically consequential, transformable state rather than only interaction handlers.

## 4.4 Sims and evolutionary robotics: physics enables novelty but objectives shape it strongly

Sims evolved virtual creatures with variable morphology and controllers in simulated physics [R26](#r26) [R27](#r27). The work remains a strong demonstration that interaction among morphology, control, and physics can produce unexpected locomotor strategies. Bongard and later evolutionary-robotics work further show that morphology can alter search trajectories and robustness [R28](#r28).

Yet these systems also reveal the power of task design. A locomotion objective selects for movement; a competitive objective selects for contest-specific strategies. Novelty within an authored objective is not equivalent to open-ended material culture.

**Lesson:** use mechanical simulation and morphology-action coupling, but avoid direct rewards for tool use, construction, or named structural outcomes. Let ecological consequences affect survival and reproduction indirectly.

## 4.5 Open-ended evolution and environment generation

Open-ended-evolution research emphasizes continued production of novelty, complexity, or interesting adaptations, while also acknowledging the difficulty of defining and sustaining open-endedness [R34](#r34). POET and related algorithms co-generate environments and solutions, allowing transfer to create stepping stones that direct optimization misses [R35](#r35).

For The Genesis Engine, generative environments can help prevent overfitting to one obstacle layout or material distribution. But environment generators can also become hidden curricula that encode intended solution classes.

**Lesson:** procedural generation should vary causal conditions—geometry, material distributions, resource timing, hazards—without generating labeled “tool puzzles.” Environment difficulty should be measured through organism-independent physical descriptors where possible.

## 4.6 Minecraft/Malmo and embodied-AI simulators: rich worlds with semantic shortcuts

Malmo exposes Minecraft as an AI experimentation platform [R36](#r36). AI2-THOR, ThreeDWorld, and OPEn provide interactive embodied environments with objects, physics, and standardized tasks [R37](#r37) [R38](#r38) [R39](#r39). These platforms are useful evidence that object-rich worlds can support manipulation research and reproducible scenarios.

They are poor direct ontological templates for Genesis because:

- Minecraft contains named blocks, item types, recipes, inventories, and explicit crafting semantics.
- Household simulators expose human-designed object categories and affordance annotations.
- Benchmark tasks provide externally specified goals.
- Many physics interactions assume floating-point engines, task resets, and finite episodes rather than multi-generation persistence.

**Lesson:** borrow instrumentation, scene querying, reproducibility practices, and task-control methodology—not their semantic object taxonomies.

## 4.7 MASON, NetLogo, and agent-based simulation frameworks

MASON emphasizes performance, model/visualization separation, repeatability, and flexible scheduling for large agent-based simulations [R40](#r40). NetLogo and similar tools demonstrate the practical power of simple discrete update rules and seeded randomness.

Their scheduling models also expose a determinism hazard: random agent iteration, container order, and event-queue behavior can become part of the simulation semantics. A seeded PRNG is insufficient if random draws are consumed in an order that changes with parallelization or data structure layout.

**Lesson:** make update ordering, conflict resolution, and random-address construction explicit protocol elements. Do not let framework iteration order define physics.

## 4.8 Stigmergic robot construction: local coordination can build structures, but targets may be scripted

Theraulaz and Bonabeau describe stigmergy as environment-mediated coordination in which traces of action influence later action [R20](#r20). Termite-inspired robot systems demonstrate that decentralized local rules can construct large structures [R21](#r21). Biological and modeling work on termite mounds shows how excavation, aggregation, environmental flow, and structure-dependent cues can organize construction [R22](#r22) [R23](#r23).

The key caution is that many engineering demonstrations generate local rules to guarantee a designer-specified target structure. That is impressive collective robotics, but it is not emergent construction in the Genesis sense.

**Lesson:** expose deposition, removal, support, flow, and persistent cues. Do not provide blueprints, target-shape fields, or progress rewards.

## 4.9 Artificial chemistries: a conceptual template, not a first implementation

Artificial chemistry commonly separates molecules, reaction rules, and a reactor algorithm [R42](#r42). This abstraction is useful for analyzing how local interactions can generate higher-level organization. Swarm Chemistry and artificial metabolic systems show that complex spatiotemporal organization can arise from simple local rules and conservation constraints [R43](#r43) [R44](#r44).

However, unrestricted reaction spaces create combinatorial explosion, opaque dynamics, unstable state counts, and weak artifact identity. A material world for thousands of organisms should not begin as a general artificial chemistry.

**Lesson:** borrow locality, conservation, and rule symmetry. Start with a small property-based transformation kernel and add reaction dimensions only when a planned experiment requires them.

## 4.10 Consolidated design lessons

| Source tradition | Mechanism worth borrowing | Failure mode to avoid |
|---|---|---|
| Ecological psychology | Relational affordances and body-environment coupling | Affordance labels stored on objects |
| Extended/distributed cognition | Persistent environmental support for memory and coordination | Assuming external memory without internal-memory tradeoffs |
| Digital evolution | Lineage, mutation, replication, controlled experiments | Named rewarded tasks as proxies for open-ended function |
| Evolutionary robotics | Physics, morphology-controller coupling, unexpected solutions | Objective-specific novelty mistaken for open-endedness |
| Agent-based modeling | Scalable local updates, clear scheduling | Update order or agent containers defining outcomes implicitly |
| Embodied-AI simulators | Instrumentation, object queries, scenario controls | Human semantic categories, scripted goals, finite reset episodes |
| Stigmergy research | Environment-mediated coordination and persistent traces | Blueprints, target fields, or hand-authored local building policy |
| Niche construction | Ecological inheritance and modified selection pressures | Calling any persistent mess a functional niche modification |
| Artificial chemistry | Local conservative transformation and emergent organization | Unbounded combinatorics and unstable object identity |

---
# 5. Minimal material ontology

## 5.1 Distinguish matter definitions from body state

A recurring modeling mistake is to place geometry, function, and material identity in one object type. The engine should instead distinguish:

- **Material definition:** an immutable, versioned record describing how a unit of matter responds to broad classes of interaction.
- **Body state:** a bounded physical lump with geometry, amount of matter, pose, velocity, damage, temperature if modeled, and provenance.
- **Composition:** initially one dominant material per body; later, a small fixed-capacity mixture or layered composition if experiments require it.

This separation allows two bodies of the same material to differ by shape, size, wear, or history, and two similarly shaped bodies to differ mechanically because of material.

A material definition should have a stable schema ID and a stable simulation-semantic version. Organisms should not perceive that ID. Physics may use a dense registry index internally after load, but canonical saves and event logs should preserve stable identifiers or canonical registry ordering.

## 5.2 Property-selection criteria

A property belongs in the first mutable-world implementation only if it passes most of these tests:

1. It enables a qualitatively new behavior or tradeoff.
2. The behavior cannot be represented adequately by an existing property.
3. Its effect can be local and deterministic.
4. It has a plausible perceptual correlate.
5. A coarse representation preserves the relevant ordering or threshold.
6. It does not require a large new solver subsystem.
7. It does not encode a named solution.

The following tables evaluate the candidate properties in those terms. Cost labels are relative to the proposed hybrid world, not absolute measurements.

## 5.3 Core material properties

| Property | New behavioral possibilities | Runtime and interaction cost | Precision needed | Plausible organism perception | Recipe risk | Recommendation and evidence status |
|---|---|---|---|---|---|---|
| **Density** | Distinguishes light portable bodies from heavy ballast, projectiles, supports, and difficult-to-move obstacles; enables load and buoyancy later | Low. Mass is derived once from density × volume and used by mechanics | 8–16-bit unsigned fixed point is sufficient; avoid arbitrary floating precision | Heft inferred from lifting effort, acceleration, contact force, or displacement | Low if used continuously; high only if exact density values become equality keys | **Include in phase 1. Engineering synthesis.** It creates fundamental manipulation tradeoffs without semantic labels |
| **Hardness** | Selective scratching, cutting, indentation, wear, and use of one body to modify another | Low to moderate. Adds material comparison to contact/damage calculation | 8-bit or 12-bit ordered scale; broad differences matter more than fine precision | Tactile resistance, deformation, sound/vibration proxy, observed damage | Moderate if coded as a material-pair lookup table; low if a broad monotonic relation | **Include. Engineering synthesis.** Necessary for property-selective modification |
| **Toughness / fracture energy** | Repeated damage, brittle versus resistant targets, breakage through accumulated effort, differential durability | Moderate. Requires accumulated damage and deterministic fracture event | 12–16-bit fixed point; threshold plus bounded damage accumulator | Crack/damage appearance, change in resistance, fragment production | Low if threshold is physical; moderate if every target has a unique magic threshold | **Include. Engineering synthesis.** Prefer energy/impulse accumulation over a generic hit-point label |
| **Wear rate / abrasion resistance** | Reuse costs, tool degradation, maintenance, path formation, changing sharpness, finite artifact lifetime | Low after damage exists; one additional state update on qualifying contacts | Coarse 8-bit material coefficient plus 16-bit body wear state | Visible surface change, reduced effectiveness, tactile change | Low | **Include, possibly phase 1b. Engineering hypothesis.** Valuable for distinguishing one-use exploitation from durable artifact reuse |
| **Static friction** | Stable placement, stacking, climbing constraints, dragging thresholds, caches that stay put | Moderate because contact solver already needs it | 8-bit coefficient or small categorical bands; fixed-point lookup | Slip/no-slip, required effort, tactile surface cue | Low | **Include. Engineering synthesis.** Essential to persistent placement |
| **Dynamic friction** | Sliding cost, path efficiency, dragging, scraping, transport tradeoffs | Moderate; paired with contact solver | 8-bit coefficient; may initially derive as a fixed function of static friction | Continued resistance during motion | Low | **Include, but consider deriving from static friction initially.** Avoid two independent parameters unless behavior benefits |
| **Compressive/support strength** | Load-bearing piles, walls, roofs, platforms, collapse, material choice in structures | Moderate. Requires support graph/load propagation or local stress approximation | Coarse fixed-point threshold; exact continuum mechanics unnecessary | Deformation, cracking, collapse under observed load | Low if force based; moderate if “supports roof” flag | **Include with 2.5D support, possibly phase 2. Engineering hypothesis.** Needed for structures to have differentiated function |
| **Nutritional energy** | Matter can sustain organisms; supports object-assisted foraging, caching, selection among consumables | Low. Existing metabolism can consume a fixed-point energy amount | 16-bit fixed point or integer energy units | Taste/odor, learned post-ingestive effects, visible class cues if physically justified | Moderate if directly visible as exact value; low if inferred | **Include for edible matter. Evidence-compatible engineering choice.** Do not call bodies “food” in physics |
| **Toxicity / physiological penalty** | Foraging tradeoffs, processing opportunities, avoidance learning, storage and dilution questions | Low to moderate depending on physiology | Small categorical or 8-bit dose coefficient | Taste/odor, post-ingestive effect; exact value should be hidden | Moderate: unique antidote pairs can become recipes | **Optional phase 1.** Include only if current ecology needs nontrivial resource tradeoffs; otherwise defer |
| **Bond affinity / adhesion potential** | Persistent attachment, composites, repair, wall construction, handles, markers | Moderate to high. Adds bond creation conditions and bond-state growth | Coarse class plus continuous strength derived from pressure/contact area; avoid exact pair matrices | Stick/slip during contact, bond formation feedback, visible attachment | High if material-specific compatibility table; lower with broad, symmetric equations | **Defer until composite experiment. Engineering hypothesis.** Introduce as a broad physical relation, not connector tags |

## 5.4 Deferred material properties

| Property | Behavior enabled | Why not initially | Suitable reduced representation | Trigger for adding it |
|---|---|---|---|---|
| **Elasticity / compliance** | Springs, flexible traps, shock absorption, rebound, elastic storage | Requires stable compliant constraints and creates solver stiffness; limited value to first carrying/caching experiments | 3–5 compliance categories or bounded spring constant; avoid deformable meshes | A specific experiment requires reusable elastic energy or flexible joints |
| **Restitution** | Bouncing, throwing, collision-mediated transport | Easy to implement but can create numerical energy exploits and adds little to early construction | 3–5 categories; cap returned kinetic energy | Projectile or rebound behavior becomes scientifically relevant |
| **Brittleness** | Sudden fracture with little plastic deformation; fragmentation differences | Can be represented initially through low toughness and fracture template | Derived ratio of hardness to toughness | Need to distinguish brittle from soft failure modes |
| **Permeability** | Shelters, filters, drainage, containers, gas/fluid flow | Meaningless without modeled flow or diffusive field transport through structures | Categorical permeability per field type | Shelter, smoke, moisture, filtration, or fluid containment experiment |
| **Thermal capacity** | Heat storage, cooking-like processing, thermal shelter | Adds temperature state and energy accounting to every body | 8–12-bit fixed point | Temperature creates an ecological pressure that cannot be modeled as a static field alone |
| **Thermal conductivity** | Heat transfer, insulation, heat-mediated transformations | Requires contact and field heat solver; increases update cost | 8-bit coefficient and low-rate diffusion | Thermal niche construction or heat processing is an explicit research phase |
| **Combustibility / ignition threshold** | Fire, clearing, heat production, cascading landscape change | High state-growth and runaway-process risk; easily becomes a “fire technology” attractor authored by the designer | Ignition threshold, energy release, burn rate, residue fraction; all conservative and local | Stable thermal model, bounded reaction budget, and ecological need exist |
| **Phase / melting point** | Molding, liquids, joining by heat, material cycles | Requires multiple material regimes, flow, and large solver expansion | A few discrete phases and hysteresis thresholds | Composite and heat experiments justify it |
| **Buoyancy** | Floating transport, rafts, water crossings | Buoyancy is largely derived from density and displaced fluid; no need for an intrinsic flag | Derived relation when a fluid surface exists | Water or fluid ecology is introduced |
| **Electrical conductivity** | Signaling, circuits, energy transport | Strongly invites a designer-authored technology ladder and requires a new physics domain | Categorical network conductance | Only after naturally selected signaling or energy-transfer questions justify it |
| **Magnetism** | Remote attraction, alignment, attachment | Adds nonlocal force and arbitrary material classes; high recipe risk | Few polarity and strength bands | A scientifically motivated experiment needs noncontact assembly |
| **Chemical reactivity** | Processing, adhesives, dissolution, synthesis | General chemistry creates combinatorial explosion; pair tables become recipes | Small reaction-feature vector and local conservative rules | Later artificial-chemistry phase with explicit complexity budget |
| **Optical reflectance/color** | Camouflage, signaling, object recognition, marking | Color is useful perceptually but can become a shortcut for hidden material identity | Low-dimensional reflectance channels independent of function | Visual communication or camouflage is required; randomize correlation with utility in controls |

## 5.5 Geometry- and body-level properties

These properties belong to bodies rather than materials.

| Body property | Behavior enabled | Cost and precision | Perception | Recommendation |
|---|---|---|---|---|
| **Geometry / footprint** | Grasping, blocking, stacking, concentrated contact, containment, reach extension | Simple collider palette is moderate cost. Quantized dimensions and orientation suffice | Visual outline, tactile boundary | **Required.** Start with disc, capsule, box, and a small convex-polygon set |
| **Volume / size** | Portability limits, containment, mass derivation, visual salience, occlusion | Low; derive from collider geometry and height | Apparent extent and grasp span | **Required.** Avoid independent “size class” if exact geometry already exists |
| **Pose and orientation** | Placement, alignment, edge presentation, external memory, construction | Required by mechanics. Fixed-point position; quantized or lookup-based orientation | Direct visual/tactile cue | **Required.** Canonicalize angles and transforms |
| **Height interval / layer** | Pits, low barriers, stacking, roofs, vertical occlusion in 2.5D | Moderate; far cheaper than full 3D | Apparent height, line-of-sight blockage | **Recommended engineering hypothesis.** Core to structures without full 3D |
| **Mass** | Momentum, force cost, carrying limits | Derived from density × volume to prevent contradictory state | Inferred through effort | **Derived, not authored independently** except temporary cached value |
| **Center of mass** | Stability, tipping, tool balance | Can derive from simple uniform geometry; composite COM from members | Inferred through rotational response | **Derived.** Do not store arbitrary balance tags |
| **Damage / integrity** | Persistent history, breakage, repair, reuse cost | One bounded accumulator or small regional array | Visible/tactile state | **Required.** Use material-scaled damage, not game-style semantic health |
| **Edge acuity / sharpness** | Cutting, scraping, piercing, efficient concentrated force | Derived from local curvature/shape feature, material hardness, and wear | Visual edge, tactile consequence | **Derive.** Do not assign `is_sharp` or `cut_power` |
| **Surface area / contact region** | Pressure, adhesion, friction, thermal transfer | Approximate from collider/contact manifold | Contact feedback | **Derived approximation.** Avoid expensive exact geometry |
| **Contents / containment relation** | Caches, transport, storage, hiding | Requires geometric point-in-container or cavity cells; can be expensive | Occlusion, access opening | **Defer until placement is stable.** Container is a relation, not a class |
| **Provenance** | Artifact lineage, ecological inheritance, causal analysis | Storage cost only; no physics cost | Usually not organism-visible | **Required for science.** Parent IDs, creation tick, cause, material ancestry |
| **Controller possession state** | Carrying, contested grasp, transfer | Small state attached to effector and body | Proprioceptive and visual | **Required.** Possession is a current mechanical relation, not ownership |

## 5.6 Fixed-point and categorical representation

Continuous physical concepts do not require floating-point storage. A practical approach is:

- positions: signed fixed point, for example Q32.32 or a narrower world-specific format;
- velocity and impulse: signed fixed point with explicit saturation policy;
- scalar material coefficients: 8–16-bit unsigned integers mapped through versioned lookup functions;
- angles: unsigned integer turns with table-driven sine/cosine;
- damage, energy, mass, and quantities: integer or fixed-point units with checked or saturating arithmetic;
- categorical phases or surface classes: small enums whose semantics are versioned.

The exact bit widths require profiling and numerical stress tests. The key requirement is that every scale, rounding mode, saturation rule, and lookup-table version be part of the simulation semantic version and save header.

**Engineering hypothesis:** most evolutionarily relevant differences will survive quantization into 16–256 meaningful levels. Fine numerical precision is less valuable than reliable monotonic response and perceptible consequence.

## 5.7 Avoid material identity as a recipe key

A material registry is unavoidable, but transformations should not usually branch on stable material identity:

```text
BAD:
if actor_material == FLINT && target_material == WOOD:
    spawn(FIRE)

BETTER:
if local_temperature >= ignition_threshold(target)
   and oxidizer_available
   and target.energy_release > 0:
       begin_bounded_combustion(target)
```

Even the better rule can encode a narrow technology if ignition can be reached by only one special object pair. The engine should audit the reachable state space through randomized property perturbations and alternate causal routes.

A safe rule of thumb is:

- material IDs select parameter records;
- physical equations consume parameter values;
- named IDs do not select interaction handlers except for schema-level exceptional matter such as organism tissue, and even those exceptions should be minimized and documented.

---

# 6. Object and artifact ontology

## 6.1 Minimum useful distinctions

The requested terms mix physical categories, biological categories, user-interface conventions, and observer judgments. Treating all of them as peer runtime types would create semantic duplication and contradictions.

| Term | Recommended status | Operational meaning |
|---|---|---|
| **Terrain** | Runtime primitive | Spatial substrate attached to world coordinates; may be mutable and material-bearing |
| **Material** | Runtime primitive | Versioned physical/metabolic properties of matter |
| **Resource** | Observer/organism-relative classification | Matter whose acquisition or transformation produces current utility or fitness consequences |
| **Object** | Runtime primitive (`PhysicalBody`) | Discrete bounded body with identity and mechanics |
| **Item** | UI/observer convenience only | A portable object under a chosen scale threshold or current interaction convention |
| **Tool** | Post-hoc causal classification | Controlled external body that mediates an effect or information relation |
| **Structure** | Post-hoc spatial-functional classification | Persistent arrangement whose configuration changes flows, access, support, risk, or information |
| **Container** | Derived geometric relation | A body or assembly with a cavity and access boundary capable of retaining other bodies or material |
| **Composite** | Derived graph state | Connected component of the bond graph, with no assumption of function |
| **Debris** | Derived size/state category | Fragments below an analysis or simulation scale; may remain bodies or be aggregated into conserved rubble |
| **Living organism** | Runtime primitive layered on physical body | Metabolizing, sensing, acting, reproducing entity with controller and lineage |
| **Environmental field** | Runtime primitive | Spatial scalar/vector state evolving by local rules and influenced by bodies/terrain/organisms |

## 6.2 Why `Tool` should not be a runtime type

Animal-tool-use definitions emphasize controlled use of an external manipulable object to alter another object, substance, surface, or the user, or to mediate information [R7](#r7) [R8](#r8) [R9](#r9). The function is relational and episode-specific. The same object may be used as a tool in one event, carried ornamentally in another, and ignored later.

A runtime `Tool` component creates several problems:

- It presupposes function before observing use.
- It encourages special action handlers or bonuses.
- It gives organisms a semantic cue if exposed through perception.
- It cannot represent opportunistic use of ordinary objects without dynamically converting them into “tools.”
- It conflates an object’s history with its current physical state.

The observer should instead identify a **tool-use episode** when all of the following are supported:

1. an organism controls an external body;
2. the body participates in a causal chain affecting a target or information relation;
3. the effect differs from an appropriate no-object, removed-object, or substitute-object counterfactual;
4. the body is not simply part of the organism unless the study explicitly includes bodily tools.

## 6.3 Why `Structure` should normally not be a runtime type

A structure can be a single moved boulder, a pile, a bonded assembly, a dug trench, compacted path, altered field boundary, or mixed terrain-object configuration. A single class cannot capture these without becoming a semantic container for special logic.

Instead, define a structure analytically as a spatially persistent configuration satisfying one or more measured functions:

- changes traversability or connectivity;
- reduces exposure to a field or hazard;
- retains matter;
- redirects movement or flow;
- supports load;
- changes visibility;
- records information;
- affects resource renewal or species distribution;
- persists beyond the immediate action episode.

A runtime may still need **mechanical assemblies** and **support islands** for performance. Those are physics constructs, not functional structure labels.

## 6.4 Terrain versus object

The distinction should be operational, not metaphysical:

- Terrain is spatially indexed matter whose identity is tied to a cell/chunk coordinate and whose movement is represented as local material transfer or elevation change.
- Objects are bounded bodies with independent pose and stable identity.

Matter may transition between the two representations:

- digging removes terrain quantity and spawns a body or adds to a carried aggregate;
- deposition converts a small body into cell material or a pile body;
- a large collapsed body may become terrain rubble after deterministic aggregation;
- erosion or repeated traffic modifies terrain without spawning every grain.

Every conversion must conserve modeled quantity and preserve provenance at an appropriate aggregate level.

## 6.5 Resource is a role, not a substance

A body with nutritional energy is not necessarily a resource if inaccessible, toxic at the current dose, already abundant, or irrelevant to current physiology. A hard non-nutritive body can become a resource if it enables access to food or shelter. A persistent marker can be informational capital.

Analytics can classify a body as a resource for organism or lineage `L` over interval `T` when its acquisition or availability has a measurable causal effect on energy, survival, reproduction, information, or task access. The classification may be probabilistic and context-dependent.

## 6.6 Container as topology

A container should not be represented as `is_container = true`. Containment arises from:

- closed or mostly closed boundary geometry;
- collision/support relations;
- an opening smaller than retained bodies or requiring a particular orientation;
- gravity or field direction;
- body position relative to the cavity.

The first implementation can use a constrained cavity model rather than arbitrary computational geometry. For example, a concave container may be represented by a small set of convex wall bodies and a deterministically defined interior region. The interior relation is derived from geometry; it must not grant infinite inventory capacity or ignore mass.

## 6.7 Composite identity

A composite is a connected component in the bond graph. Two identities must be kept distinct:

- **Member identity:** each physical body retains its stable entity ID throughout attachment and detachment.
- **Assembly identity:** a transient structural handle derived from current membership and bonds.

Recommended assembly handle:

```text
assembly_root = minimum(member_entity_ids)
assembly_revision_hash = H(
    sorted(member_entity_ids),
    sorted(canonical_bond_records)
)
```

The pair `(assembly_root, assembly_revision_hash)` identifies a particular assembly state for logging and analytics. It is not a permanently allocated object ID. On merge or split, log explicit assembly-lineage events. This avoids arbitrary identity decisions such as whether a handle attached to a stone creates a new object or transforms one parent into another.

## 6.8 Debris and scale transitions

Unbounded fragmentation is incompatible with long-running worlds. The engine should distinguish simulation scales:

1. **Resolved body:** independent pose, collision, identity, and provenance.
2. **Aggregate pile:** one body or terrain feature storing material quantity, size distribution summary, compaction, and coarse shape.
3. **Cellular rubble/dust:** terrain quantity or field-like deposit with no individual IDs.

Deterministic transition rules should use mass/size and local density thresholds, not camera distance or wall-clock load. When fragments are aggregated, retain:

- total material quantity;
- weighted composition;
- aggregate provenance hash or bounded ancestor summary;
- creation interval;
- physical parameters needed for later excavation or consolidation.

Disaggregation should be bounded and deterministic. It should not recreate exact lost fragment histories; the representation transition itself is authoritative state.

## 6.9 Organisms should share physics where possible

Organisms should collide, push, carry load, block paths, fall into pits, and interact with fields through the same broad mechanics as nonliving bodies. Exceptions may be necessary for performance or locomotion stability, but every exception weakens embodied grounding.

Avoid:

- organisms phasing through objects because navigation is handled separately;
- carried bodies disappearing into an inventory;
- organisms having unlimited manipulator reach;
- “use range” interactions without contact or plausible force transfer;
- structures affecting agents through tags while having no collision or field effect.

**Engineering hypothesis:** a simplified kinematic organism body can coexist with deterministic physical objects if all cross-system contacts are resolved through a single canonical protocol. This is less physically pure than fully dynamic bodies but may be necessary for thousands of agents.

## 6.10 Environmental fields

Fields represent spatial influences not efficiently modeled as discrete bodies. Candidate fields include:

- light/exposure;
- temperature;
- moisture;
- odor or resource cue;
- deposited signaling chemical;
- smoke or toxin concentration;
- flow direction;
- vegetation/regrowth potential;
- traffic/compaction memory.

The initial field set should be minimal. Every field adds memory, diffusion/advection work, save growth, and possible semantic shortcuts. A deposited signal field is justified for stigmergy only if organisms can emit, perceive, and alter it through generic physiology; it should not encode “go to food” or “build here.”

Recommended initial set:

1. existing ecological fields required by the current world;
2. one generic deposited scalar signal with decay, if needed for the stigmergy experiment;
3. one exposure/occlusion field only if shelter is tested;
4. terrain compaction as cell state rather than a continuously diffused field.

---

# 7. Action primitives

## 7.1 The control-interface tradeoff

Action primitives determine both openness and search difficulty. A large semantic action vocabulary narrows causal interpretation in advance. A fully general force-and-joint interface is open but may be inaccessible to evolution and expensive to simulate.

The design goal is **controlled physical generality**: primitives should correspond to bodily capabilities, while named human interpretations emerge from sequences and contact outcomes.

## 7.2 Recommended core action set

### 7.2.1 Grasp

```text
grasp(target_id, effector_id, grip_level)
```

Preconditions are mechanical: target is within reach, the effector is free or allows multiple grasp, contact geometry is valid, and required grip does not exceed capability. A successful grasp creates a temporary constraint whose maximum force depends on effector strength, grip level, surface friction, and geometry.

**Enables:** pickup, carrying, pulling, controlled striking, object orientation, transfer, retention, reuse, and assembly.

**Complexity:** moderate. Requires persistent constraints, contested acquisition, load feedback, and deterministic break/slip behavior.

**Justification:** indispensable. Without persistent control of a discrete body, strong definitions of tool use cannot be operationalized.

### 7.2.2 Release

```text
release(effector_id)
```

Release removes the grasp constraint at the commit phase. The body retains its current pose and momentum.

**Enables:** dropping, placement, throwing, transfer, caching, stacking, and leaving artifacts.

**Complexity:** low once grasp exists.

**Justification:** required. Placement should be an outcome of controlled release, not a magic teleport.

### 7.2.3 Apply force / directed manipulation

```text
apply_force(
    target_or_world,
    effector_id,
    contact_region,
    direction_bin,
    magnitude_bin,
    duration_bin
)
```

The engine translates the action into a bounded force or impulse at a local contact region. Directions should be body-relative and quantized. Magnitudes should be limited by morphology, energy, stance, and grip. Duration may represent a fixed number of ticks rather than arbitrary real time.

**Enables:** pushing, pulling through grasp, striking, scraping, digging, levering, compression, object testing, terrain marking, and bond formation pressure.

**Complexity:** moderate to high. Requires contact resolution, energy accounting, and an action representation controllers can use.

**Justification:** include, but not as unconstrained continuous vectors at first. Use 8–16 direction bins, 4–8 magnitude bands, 2–4 duration bands, and a small contact-region vocabulary defined by the organism’s effector geometry.

### 7.2.4 Ingest or exchange matter

```text
ingest_or_exchange(target_id_or_cell, amount_bin)
```

This represents matter transfer through an organism interface. It can consume a body, remove a quantity, deposit waste, or later secrete an adhesive or signal. It should be constrained by contact, physiology, aperture, and transfer rate.

**Enables:** feeding, partial consumption, excavation by ingestion if biologically plausible, deposition, secretion, and material processing.

**Complexity:** low to moderate, depending on composition and conservation.

**Justification:** required for ecology, but keep ingestion separate from semantic `consume_food`.

### 7.2.5 Locomotion, orientation, and gesture

The existing locomotor interface remains. Body orientation and effector pose should be perceptible to others. A separate `point` action is unnecessary if organisms can orient a limb, gaze proxy, or body axis. A generic emission channel may support signaling, but its meaning must be learned or evolved.

**Enables:** approach, positioning, transport, demonstrations, deictic behavior, territorial display, and social observation.

**Complexity:** mostly existing; social observation increases sensing cost.

## 7.3 Evaluation of named candidate actions

| Candidate action | Implement directly? | Higher-level realization | Behavior enabled | Complexity and risk | Decision |
|---|---:|---|---|---|---|
| **Pick up** | No separate primitive | Grasp + lift/locomotor displacement | Portable control | Redundant with grasp; semantic shortcut if it ignores dynamics | Derive |
| **Drop** | No separate primitive | Release while unsupported | Deposition, accidental loss | Redundant | Derive |
| **Carry** | No | Maintain grasp during locomotion | Transport and reuse | Must impose load and energy cost | Derive and log |
| **Place** | No magic placement | Move + controlled release at low speed | Caches, stacks, markers | Teleport placement would hide manipulation | Derive and classify by outcome |
| **Push** | No | Contact force without grasp | Moving obstacles, aggregation | Straightforward force profile | Derive |
| **Pull** | No | Grasp + force away from target | Retrieval, dragging | Requires grasp and traction | Derive |
| **Strike** | No | Brief high-magnitude impulse | Fracture, signaling, defense | Can produce numerical exploits; cap impulse by energy | Derive |
| **Scrape** | No | Sustained tangential force | Wear, cutting, marks, digging | Requires contact geometry and wear | Derive |
| **Cut** | No | Concentrated edge contact + tangential force + material relation | Separation and shaping | An explicit verb would encode solution semantics | Derive from physics |
| **Dig** | No | Force/material removal at terrain contact | Pits, shelter, caches, barriers | Requires terrain quantity transfer | Derive from excavation rule |
| **Stack** | No | Repeated placement + support equilibrium | Walls, towers, caches | Needs 2.5D support | Derive and analyze |
| **Attach** | Prefer no semantic action | Sustained contact/pressure under bond-compatible properties | Composites | Purely passive adhesion may be too hard to control | Start passive; add generic bond attempt only if needed |
| **Detach** | No | Applied force exceeds bond or organism releases a reversible clamp | Reconfiguration and reuse | Requires bond failure mechanics | Derive |
| **Break** | No | Damage exceeds toughness | Access, shaping, debris | Explicit action would bypass material differences | Derive |
| **Combine** | No | Contact + bond/reaction | Composite objects | “Combine” often hides recipe systems | Never a generic semantic verb |
| **Separate** | No | Fracture, bond failure, cutting, matter transfer | Disassembly | Same as above | Derive |
| **Heat** | No at first | Contact with hot body/field, friction, reaction | Processing, thermal ecology | Requires thermal subsystem | Defer |
| **Cool** | No | Heat flow to colder body/field | Preservation, phase changes | Requires thermal subsystem | Defer |
| **Consume** | Yes only as matter transfer interface | Ingest/exchange | Metabolism | Must not expose food label | Include as physiological primitive |
| **Store** | No | Place within containing geometry, later retrieve | Caching and external memory | Inventory semantics would be scripted | Derive |
| **Give** | No | Voluntary release within recipient reach or coordinated transfer | Prosocial exchange | Social meaning must be inferred | Derive and classify |
| **Take** | No | Contested or unopposed grasp | Competition, theft, transfer | Requires deterministic conflict resolution | Derive |
| **Point/signal toward object** | No named semantic pointer | Orient effector/body, gaze proxy, or generic signal | Social attention and demonstration | Explicit target pointer transmits object identity perfectly | Use embodied orientation or noisy signal |

## 7.4 Generic force versus many verbs

A generic force action is more open-ended because its meaning is determined by contact, timing, geometry, and material response. It also increases controller difficulty in four ways:

1. **Continuous parameter search:** direction, magnitude, contact point, and duration form a large action space.
2. **Delayed consequences:** a useful scrape or stack requires coordinated multi-tick control.
3. **State estimation:** the controller must infer object pose, load, slip, and target response.
4. **Credit assignment:** ecological benefit may occur long after manipulation.

The proposed quantization reduces these costs while preserving composition. An initial action encoding might be:

- target slot: 0–7 local perceived bodies plus terrain/contact target;
- effector: 0–1;
- mode: grasp, release, force, ingest;
- direction: 8 body-relative bins;
- magnitude: 4 levels;
- duration: 2 levels;
- contact region: center, near edge, far edge, current contact.

This is still large. Evolution may need hierarchical controllers, action persistence, motor-neuron modules, or developmental curricula. The engine should not solve the problem by replacing the interface with human verbs.

## 7.5 Action persistence and cancellation

Multi-tick actions should compile into explicit persistent intents with stable IDs:

```text
ActionInstance {
    actor_id,
    actor_action_seq,
    start_tick,
    end_tick_exclusive,
    action_kind,
    quantized_parameters,
    cancellation_policy
}
```

The controller can cancel or replace an action according to deterministic rules. Persisted actions must be serialized. Their effects are recomputed only from authoritative state and the stored action instance, not from wall-clock time.

## 7.6 Energy and force budgets

Every manipulation should have a cost linked to force, duration, movement, and morphology. Otherwise organisms may apply maximum force continuously and turn object interaction into costless remote control.

A simple deterministic budget can use:

```text
work_cost ≈ base_effector_cost
          + force_magnitude × effective_displacement
          + grip_force × duration × grip_metabolic_factor
```

This need not be a physically exact joule model. It must be monotonic, bounded, and versioned. Failed actions should usually cost something if force or movement was attempted; otherwise agents can probe the world for free.

## 7.7 Perceivable action consequences

For learning and social transmission, actions must yield feedback:

- grasp success/failure and slip;
- applied effort;
- contact location and normal/tangential force bands;
- object movement and rotation;
- damage or fracture cue;
- bond formation/failure cue;
- ingestion amount and delayed physiological effect;
- observed actor, target, and coarse action trajectory for nearby organisms.

Do not expose a success bit such as `cut_succeeded` or `tool_effective`. Expose physical consequences from which usefulness can be learned.

---

# 8. Property-based transformations

## 8.1 Transformation design goals

A transformation system must satisfy six constraints simultaneously:

1. local enough to evaluate efficiently;
2. deterministic and canonically ordered;
3. broad enough to allow alternate causal routes;
4. conservative where matter or energy is represented;
5. bounded against combinatorial explosion;
6. free of semantic recipe recognition.

The appropriate first model is not a generic crafting graph. It is a small family of local transformation mechanisms.

## 8.2 Contact and damage

For each canonical contact, compute a bounded damage contribution from quantities already available to mechanics:

```text
normal_pressure  = normal_impulse / max(contact_area_proxy, area_floor)
tangential_work  = tangential_force × slip_distance
impact_energy    = bounded_relative_kinetic_energy

indent_component = f(normal_pressure - target_hardness_response)
abrasion_component = g(tangential_work, attacker_hardness, target_hardness,
                       edge_acuity, target_wear_resistance)
impact_component = h(impact_energy, target_toughness, support_condition)

damage_delta = clamp(indent_component + abrasion_component + impact_component)
```

The exact functions should be monotonic, piecewise fixed-point, and simple enough for audit. Contact events are accumulated for the tick, sorted canonically, then applied in a transformation phase. This avoids first-contact-wins behavior.

**Engineering hypothesis:** one scalar integrity value plus a small number of predeclared fracture regions per shape is sufficient for early experiments. Per-voxel damage or arbitrary meshes are unnecessary.

## 8.3 Fracture templates

Arbitrary mesh fracture is expensive and creates unbounded fragments. Instead, each simple shape family can define a small set of deterministic fracture templates:

- split along long axis;
- split along short axis;
- chip one edge;
- radial 2–4 fragment split;
- crumble into aggregate material.

Template selection should depend on contact region, force direction, material brittleness proxy, parent geometry, and a keyed random draw addressed by `(world_epoch, tick, parent_id, fracture_count)`. Children are emitted in canonical template order and receive IDs during the spawn commit phase.

Conservation requirements:

- sum of fragment material quantity equals parent quantity minus explicitly modeled dust loss;
- dust loss becomes rubble/terrain quantity, not deletion;
- linear and angular momentum are distributed by a versioned deterministic rule;
- provenance records the parent and fracture event;
- fragment count is capped.

## 8.4 Terrain excavation and deposition

Terrain mutation should operate on material quantity rather than semantic actions.

A cell may store:

```text
material_id
surface_elevation
loose_quantity
compaction
integrity_or_cohesion
damage
```

Qualifying contact applies an excavation delta based on pressure, tangential work, contact geometry, material hardness/cohesion, and current compaction. Removed quantity is transferred into:

- a carried aggregate attached to an effector;
- a nearby resolved fragment body if above threshold;
- a loose pile body;
- an adjacent cell deposit according to deterministic displacement.

Deposition changes elevation, compaction, or loose quantity. Repeated traffic can compact terrain and alter friction or regrowth. This supports paths and pits without a `DIG` or `MAKE_PATH` command.

## 8.5 Bond formation and failure

A bond is a physical constraint record:

```text
Bond {
    bond_id_or_canonical_key,
    endpoint_a,
    endpoint_b,
    anchor_a,
    anchor_b,
    rest_transform,
    tensile_strength,
    shear_strength,
    torsional_strength,
    compliance,
    contact_area_proxy,
    creation_tick,
    cause_event,
    material_provenance
}
```

A bond may form when:

- surfaces remain in valid contact for a minimum duration;
- relative velocity is below a limit;
- pressure lies within a formation window;
- geometry is compatible within broad tolerance;
- a generic adhesive quantity is present, or material adhesion potential is sufficient;
- local temperature or moisture condition falls in range, if those systems exist.

Bond strength should be a function of contact area, pressure history, material adhesion values, and curing state—not a lookup saying material A connects to material B.

Bond failure accumulates tensile, shear, and torsional load. All loads for the tick are aggregated before break decisions. This prevents update-order bias when several organisms pull a composite simultaneously.

### Should there be an explicit `attempt_bond` action?

**Preferred answer:** not initially. Let bond formation arise from sustained contact and pressure.

**Engineering caveat:** evolving controllers may find passive bond conditions too difficult to discover, especially when precise alignment is required. If experiments fail after control-oriented simplification, introduce a generic effector capability such as secretion or clamping. It must consume material/energy and still obey geometry and strength equations. Do not add `attach(A, B)` as a guaranteed semantic operation.

## 8.6 Shape compatibility

Geometry may make some assemblies easier than others. Safe compatibility derives from:

- overlapping or adjacent surface intervals;
- contact normal alignment;
- insertion depth;
- cavity and protrusion dimensions;
- friction and adhesion;
- tolerances expressed in world units.

Unsafe compatibility uses opaque sockets or tags:

```text
BAD: connector_type = HANDLE_SOCKET
BAD: compatible_with = STONE_HEAD
BAD: recipe_slot_2 = WOOD_SHAFT
```

A limited shape palette can still create meaningful compatibility. Rod-like bodies fit narrow gaps and extend reach. Flat bodies create barriers. Concave arrangements retain smaller bodies. Broad surfaces create stable support. None needs a named role.

## 8.7 Reaction systems

A later reaction kernel can be defined as broad predicates over local physical state:

```text
ReactionRule {
    required_feature_ranges,
    local_temperature_range,
    pressure_or_contact_range,
    catalyst_feature_ranges,
    rate_function,
    conserved_inputs,
    outputs_as_material_features,
    energy_delta
}
```

This still risks becoming a recipe graph. A rule is safer when:

- it covers a region of feature space rather than exact IDs;
- rates vary smoothly with conditions;
- alternative energy sources can reach the same state;
- outputs remain matter with physical properties rather than named crafted objects;
- mass and modeled elements/features are conserved;
- the rule is symmetric where physics suggests symmetry;
- small perturbations produce graded effects rather than total failure.

Artificial-chemistry literature provides a useful conceptual decomposition into entities, interaction/reaction rules, and a reactor or scheduling algorithm [R42](#r42). It also warns, by implication, that rule space and population dynamics can become the primary system rather than a support for embodied artifacts. Reactions should therefore be a later bounded subsystem.

## 8.8 Learned action sequences are not recipes

An organism may learn a reliable sequence such as:

1. obtain a hard body;
2. orient an edge;
3. strike or scrape a target repeatedly;
4. retrieve the exposed nutrient body.

This is not a scripted recipe if the simulator never recognizes the sequence and every step succeeds through generic mechanics. The distinction is causal:

- **Recipe system:** the engine matches a symbolic configuration or sequence and emits a predefined result.
- **Learned procedure:** the organism controls a sequence whose physical effects accumulate under ordinary state-transition rules.

Event analytics may later infer recurring procedures, but those labels remain outside the causal loop.

## 8.9 Detecting hidden recipe graphs

The engine should automatically test its own transformation rules.

### 8.9.1 Property perturbation tests

Randomly perturb size, hardness, toughness, mass, geometry, and contact conditions around successful episodes. A physical system should usually show graded changes in success probability or efficiency. A narrow all-or-nothing boundary tied to arbitrary identities is suspicious.

### 8.9.2 Substitution tests

Replace each participant with bodies sampled to match subsets of its properties. Determine which properties are causally necessary. If only exact identity works after controlling for physical state, inspect for semantic branching.

### 8.9.3 Route enumeration

Use bounded automated search or fuzzing to find alternate state-action paths to the same physical outcome. A transformation reachable through exactly one hidden sequence is recipe-like; an outcome reachable through several mechanical routes is more open.

### 8.9.4 Rule-coverage analysis

For each transformation rule, estimate the volume of reachable property and state space that activates it. Rules with singleton material combinations or extremely thin manifolds require explicit justification.

### 8.9.5 Observer-blind replay

Remove all observer classifications and rerun golden scenarios. Physical outcomes must remain identical. This guards against accidental feedback from analytics.

## 8.10 Controlling combinatorial explosion

The transformation layer should impose physical and numerical bounds, not semantic validity checks:

- maximum resolved fragments per fracture;
- minimum resolved-body mass or area;
- maximum active bond degree per body;
- maximum constraints per local spatial bucket;
- bounded reaction participants, normally two bodies plus field/terrain context;
- bounded number of composition components per body;
- deterministic sleeping and aggregate conversion;
- fixed maximum fracture depth per tick;
- fixed local material-transfer budget per tick;
- bounded event records with aggregate summaries for high-frequency microcontacts.

“Invalid composites” should not be rejected because they do not correspond to a known item. They should fail only for physical overlap, impossible geometry, exceeded degree/solver limits, insufficient bond strength, or numerical safety constraints.

## 8.11 Runaway object counts

Object count can grow through fracture, excavation, organism secretion, reproduction, and abandoned artifacts. Controls should form a representation hierarchy rather than deleting matter:

1. resolved object while behaviorally salient;
2. sleeping resolved object after inactivity;
3. aggregate pile when many small nearby bodies share compatible state;
4. terrain rubble when below manipulation scale;
5. re-resolution only when excavation or separation crosses a threshold.

Aggregation and re-resolution must be functions of simulation state, not frame rate, camera visibility, or current server load. Otherwise replay changes with observation conditions.

---
# 9. Structures, stigmergy, and niche construction

## 9.1 Persistent environmental modification as the common substrate

Construction, caching, trail formation, nest building, territorial marking, and infrastructure differ in function, but they share four requirements:

1. an organism changes spatially persistent state;
2. the changed state is perceivable or mechanically consequential later;
3. the consequence lasts beyond the immediate motor action;
4. the consequence affects future action selection, ecology, or selection pressure.

Niche-construction theory describes organisms as modifying environments in ways that can alter selection, and identifies persistence of those modifications as ecological inheritance [R24](#r24) [R25](#r25). This supports a broad design objective: artifacts should not merely remain visible; they should alter costs, risks, access, information, or resource dynamics.

## 9.2 Structures possible from placement alone

Generic carrying and placement, with collision and friction, can produce more than it first appears.

| Emergent arrangement | Minimal physics needed | Potential function | Additional caveat |
|---|---|---|---|
| **Pile** | Gravity/support approximation, friction, persistent bodies | Marker, cache cover, barrier, elevated support, aggregate material | Must distinguish deliberate placement from incidental accumulation |
| **Linear barrier** | Placement, collision, stable support | Movement restriction, channeling, territorial boundary | 2D agents may simply route around it unless geometry and costs make it meaningful |
| **Ring/enclosure** | Placement and collision | Retention, exclusion, trap-like channeling | True containment may require size-sensitive openings and 2.5D height |
| **Cache** | Placement, persistence, spatial memory | Deferred consumption, reduced transport cost, social theft or sharing | Requires object identity and resource scarcity/decay tradeoff |
| **Marker** | Perceivable object arrangement or orientation | External memory, territorial signal, route cue | Must not provide semantic map labels |
| **Stepping support** | Stable placement, height bands | Crossing soft/hazardous terrain, reaching elevated targets | Requires body support and traversal cost differences |
| **Simple wall** | Repeated placement, friction, support | Barrier, line-of-sight occlusion, shelter edge | Multi-layer height may require 2.5D stacking |
| **Path** | Terrain compaction or vegetation suppression from repeated movement | Lower locomotor cost, route memory, coordination | Placement alone is insufficient; terrain must retain traffic effects |
| **Resource redistribution** | Carrying and placement | Centralized feeding, provisioning, larder formation | Function depends on metabolic and social ecology |

These arrangements should not acquire a `Structure` component when they cross a designer-defined shape threshold. Offline observers can cluster persistent configurations and test their causal effects.

## 9.3 Structures that require more than placement

Some functions require additional physics:

| Function | Additional mechanism | Why placement alone is insufficient |
|---|---|---|
| **Roofed shelter** | Height/support, load propagation, collapse, directional exposure/occlusion | A 2D footprint cannot distinguish overhead cover from a floor obstacle |
| **Bridge** | Support span, body load, gap/hazard traversal | Pure collision cannot represent a traversable elevated surface over a gap |
| **Pit trap** | Mutable elevation, falling/escape constraints, occlusion | A visual mark does not constrain movement |
| **One-way trap** | Flexible/reorientable parts, asymmetric geometry, or terrain slope | Static symmetric collision lacks direction-dependent access |
| **Sealed container** | Cavity topology, opening, retained contents, possibly permeability | A pile near food is not containment |
| **Insulated shelter** | Temperature field, occlusion, permeability/conductivity | Geometry alone cannot alter thermal exposure |
| **Dam/channel** | Fluid or directional flow and terrain permeability | No flow means no causal hydraulic function |
| **Rope/net** | Flexible constraints and many-body contact | Rigid bonds cannot reproduce flexible capture or tension networks efficiently |
| **Door/gate** | Hinge or reversible constraint, stateful support | Requires controlled articulation rather than static bond |
| **Adhesive nest** | Bond formation, curing, material transfer | Stable complex geometry is unlikely through friction alone |

The engine should not add these mechanisms preemptively. Each should enter through a dependency-ordered experiment with a clear ablation.

## 9.4 Stigmergy

Stigmergy is coordination through changes in the environment: one action leaves a trace that modifies the probability or form of later actions [R20](#r20). The concept is often divided into:

- **Quantitative stigmergy:** more trace changes action intensity or probability, such as a stronger deposited trail.
- **Qualitative stigmergy:** the kind or configuration of work already present changes what action is performed next, such as the geometry of a growing structure.

A related useful distinction is:

- **Trace-based stigmergy:** a deposited field or mark directly persists.
- **Sematectonic stigmergy:** the partially modified physical structure itself changes future affordances.

The Genesis Engine should support both eventually, but sematectonic coordination is especially aligned with the “physics, not progress” principle. A partially dug depression makes further excavation mechanically and perceptually different; a pile changes support and visibility; a path lowers movement cost.

## 9.5 Minimal stigmergic mechanisms

A minimal stigmergy test needs:

1. a persistent, local, perceivable trace;
2. agent behavior capable of changing the trace;
3. trace decay or transformation over time;
4. ecological consequence of coordinated action;
5. controls separating trace use from direct observation or common attraction.

Possible trace channels:

- terrain compaction;
- loose material accumulation;
- scratch/damage marks;
- object arrangement;
- generic deposited scalar field;
- local odor from stored matter;
- structural occlusion or altered flow.

The trace should not encode a designer message. A field value may mean “concentration of secretion,” not “food direction.” Its behavioral meaning is learned or evolved.

## 9.6 Lessons from biological and robotic construction

Termite-inspired construction research demonstrates that local, environment-mediated rules can coordinate many agents [R21](#r21). Yet target structures in engineered robot studies are frequently specified by the designer. Biological and modeling work indicates that excavation, aggregation, structural feedback, and environmental transport can organize mound construction [R22](#r22) [R23](#r23).

**Evidence-backed lesson:** structure-dependent cues and local material feedback can coordinate construction without centralized planning.

**Engineering caution:** a target-shape field, blueprint map, “correct placement” reward, or fixed local policy generated from a desired structure would violate the Genesis objective even if agents execute it locally.

## 9.7 Niche construction and ecological inheritance

A persistent environmental change becomes evolutionarily important when it modifies conditions faced by later organisms. Examples include:

- paths reducing travel cost;
- barriers changing predator/prey encounter rates;
- caches altering resource variance;
- shelters reducing hazard exposure;
- excavation changing moisture or temperature;
- waste deposits changing toxicity or resource renewal;
- artifact fields changing what juveniles observe;
- construction changing population density or spatial assortment.

Ecological inheritance should be measured through intervention. Compare descendants placed into:

- the modified parental environment;
- a reset environment with the same seed and current exogenous state;
- an environment modified by another lineage;
- a transplanted artifact configuration;
- a geometry-preserving but function-destroying sham configuration.

This separates inherited physical capital from genetic adaptation, population density, and simple resource abundance.

## 9.8 External memory

A configuration qualifies as external memory only if evidence supports a memory-like causal role. Candidate criteria:

1. state is written by an organism;
2. state persists across a delay during which internal state may change;
3. organism later senses the state;
4. sensing changes action in a way related to earlier information;
5. altering or removing the external state impairs performance;
6. a visually similar but information-shuffled state does not provide the same benefit.

Examples include branch markers, object counts encoding choice, cache markers, or maintained route cues. A body left near a resource is not automatically memory; it may be accidental debris or a direct physical aid.

## 9.9 Territorial markers and ownership

Territorial behavior can emerge from defense, repeated occupancy, signaling, and exclusion. The world does not need a global ownership table. A physical marker can matter if:

- organisms perceive it;
- marker state correlates with a defender or group;
- receivers alter behavior based on learned or evolved associations;
- false or displaced markers can be tested;
- enforcement depends on behavior, not engine permission.

A supernatural rule such as “only owner can pick up” would convert a social convention into physics. If later research requires beliefs about ownership, those beliefs belong in organism state and may be wrong, contested, or culturally transmitted.

## 9.10 Distinguishing construction from environmental noise

Offline classification should require more than persistence. A candidate construction episode should show some combination of:

- non-random spatial organization relative to organism scale or ecological features;
- repeated directed placement/removal;
- maintenance after perturbation;
- reduced entropy along a task-relevant structural descriptor;
- causal improvement in access, exposure, storage, movement, signaling, or reproduction;
- convergence among independent agents or lineages;
- reuse by the builder or others.

Intent cannot be read directly from a neural controller. Use behavioral evidence and counterfactuals rather than anthropomorphic interpretation.

---

# 10. Tool-use hierarchy and measurement

## 10.1 Operational definition

A conservative operational definition follows animal-tool-use research: an organism exerts control over a freely manipulable external object in order to alter a target, the user, another organism, or an information relation [R7](#r7) [R8](#r8) [R9](#r9). For simulation analytics, “in order to” should not be inferred from presumed intent. Replace it with observable criteria:

- controlled manipulation;
- temporally linked target effect;
- causal dependence under intervention;
- repeatability or policy sensitivity when stronger evidence is needed.

A body that is merely pushed accidentally, stood upon because it happens to be present, or swallowed is not automatically a tool.

## 10.2 Proposed hierarchy

The levels below are **observer classifications only**. They should not be represented as organism achievements, unlocks, rewards, or era labels.

### Level 0 — incidental object interaction

An organism contacts, displaces, or consumes an object without evidence that external control mediates another effect.

**Examples:** bumping debris; carrying an object accidentally because it adheres; moving food directly into the mouth.

**Metric:** physical interaction count. Do not call this tool use.

### Level 1 — controlled transport

The organism establishes control over a body and moves it across a meaningful distance or interval.

**Required evidence:** grasp/control relation, displacement beyond body-scale threshold, and maintained control.

**Caution:** carrying demonstrates manipulation but not yet use of the body to affect a separate target.

### Level 2 — object-assisted access or action

The controlled body causally improves access, extraction, defense, movement, information, or modification relative to no-object or substitute controls.

**Examples:** using a hard body to fracture a shell-like barrier; placing a support to cross hazardous terrain; probing a hidden cavity.

**Required evidence:** deterministic branch replay in which removing or replacing the object reduces the relevant outcome.

### Level 3 — repeated reuse

The same stable object ID is retrieved and used in separated episodes, rather than one opportunistic event.

**Required evidence:** distinct use episodes separated by release, delay, travel, or context; object identity preserved.

**Interpretive strength:** suggests value tracking, memory, or persistent artifact preference, but not necessarily manufacture.

### Level 4 — property-selective object choice

The organism chooses among available bodies according to behaviorally relevant physical properties and transfers this choice to novel identities or locations.

**Required evidence:** choice remains predicted by mass, hardness, shape, or size after controlling for distance, color, prior exposure, and object ID.

### Level 5 — pre-use modification

The organism changes the object before using it, and the modification is causally necessary or improves effectiveness.

**Examples:** sharpening through wear, shortening, removing obstructive material, changing orientation permanently, or creating a useful fracture.

**Required evidence:** provenance links pre-use transformation to later use; unmodified counterfactual performs worse.

### Level 6 — composite tool or functional assembly

Two or more bodies are physically connected or jointly arranged and the configuration provides a causal benefit not available from the components separately.

Compound tool construction has been demonstrated in New Caledonian crows, illustrating why assembly should be distinguished from mere object selection [R11](#r11).

**Required evidence:** bond/support graph, assembly-state identity, component-only controls, and configuration perturbation.

### Level 7 — metatool or multi-step dependency chain

One object is used to acquire, produce, modify, or access another object that is then used for a later outcome. Metatool behavior in crows provides a biological reference point [R10](#r10).

**Required evidence:** causal dependency directed acyclic graph with depth at least two, temporal ordering, and intervention on intermediate artifacts.

### Level 8 — socially acquired object-use variant

An observer’s probability of adopting an object choice, transformation, or action sequence increases because of exposure to another organism’s behavior.

**Required evidence:** controlled demonstrations, visibility manipulation, genetic and environmental controls, and distinction from local or stimulus enhancement.

### Level 9 — tool tradition

A population-level object-use variant persists across multiple cohorts or generations through social transmission, beyond what genes and shared ecology explain. Chimpanzee cultural-variation and conformity research provides relevant empirical models [R14](#r14) [R15](#r15).

**Required evidence:** diffusion structure, persistence after original innovators disappear, cross-fostering or environment controls, and loss under social-transmission ablation.

### Level 10 — cumulative modification

Successive socially transmitted changes produce a ratchet-like increase in performance, complexity, dependency depth, or accessible ecological outcomes that individuals rarely reconstruct independently. Cumulative-culture research emphasizes that accumulation requires more than mere persistence or copying [R16](#r16) [R17](#r17) [R18](#r18) [R19](#r19).

**Required evidence:** lineage-linked improvements, social dependence, retention across generations, culture-loss tests, and a performance or capability envelope that exceeds isolated rediscovery.

## 10.3 Why the hierarchy should not be a ladder inside the world

The levels are not inevitable stages, a universal sequence, or a technology tree. A lineage may show strong caching and environmental inheritance without composite tools. Another may assemble objects but not transmit the behavior socially. A third may use stigmergic paths with no discrete tools.

The hierarchy is an analytic rubric for evidence strength. It must never:

- unlock actions or mutations;
- alter fitness directly;
- change resource generation;
- appear in organism observations;
- trigger an “era” transition;
- increase physics capabilities;
- become a target for curriculum generation.

## 10.4 Event-log primitives for tool analysis

The log should record enough information to reconstruct episodes:

```text
PerceptionOpportunity(actor_or_observer, target_ids, cue_summary, occlusion, distance)
ActionIntent(actor, action_instance, target, quantized_parameters)
GraspAttempt(actor, effector, target, contenders)
GraspEstablished(actor, effector, target, constraint_id)
GraspReleased(actor, effector, target, cause)
Contact(pair, manifold_key, impulse_bands, slip, region)
ForceApplied(actor, effector, target, quantized_force, duration)
BodyTransform(entity, old_pose_hash, new_pose_hash, cause)
DamageApplied(target, source_event, amount, region)
Fracture(parent, children, template, cause)
BondCreated/Failed(endpoints, parameters, cause)
TerrainDelta(cell, material_delta, elevation_delta, compaction_delta, cause)
MatterTransfer(source, destination, quantity, composition, cause)
PhysiologicalOutcome(actor, energy_delta, damage_delta, delayed_effect_id)
VisibilityEpisode(observer, actor, target, action_summary)
```

High-frequency contacts may be compressed, but compression must preserve causal totals and ordering needed for analysis.

## 10.5 Causal measurement through deterministic branched replay

The engine’s determinism enables unusually strong causal analysis. At a candidate episode start tick, restore the same checkpoint and run branches such as:

- remove candidate object;
- replace with equal-size but different-mass body;
- replace with matched mass but lower hardness;
- relocate object outside reach;
- preserve object but randomize orientation;
- weaken or remove one bond;
- reset a terrain modification;
- hide the demonstration from observers;
- replace demonstrator actions with a yoked non-agent trajectory.

Use the same exogenous keyed RNG coordinates for unaffected events. Interventions should allocate a separate branch namespace so new branch-specific events do not collide with baseline random addresses.

Causal effect can be reported as:

```text
ACE_artifact = outcome_baseline - outcome_intervention
```

across matched episodes or seeds, with uncertainty over populations and environments. For one deterministic history, the difference is an individual counterfactual, not a population estimate.

## 10.6 Metrics by hierarchy level

| Level | Primary metrics | Strong controls |
|---|---|---|
| Controlled transport | controlled displacement, grip duration, load-relative movement, release context | accidental adhesion, target drift, organism-body collision |
| Object-assisted action | energy/time/survival uplift; access probability; target-state change | no object, property-matched substitute, body-only action, direct-access condition |
| Reuse | same-ID use count, inter-use interval, retrieval distance, artifact lifetime utility | respawned equivalent objects, identity tracking ablation, location controls |
| Property selection | conditional choice slope, mutual information, generalization to novel IDs/combinations | location/color randomization, property homogenization, semantic cue masking |
| Modification | modification-to-use lineage, performance delta, modification necessity | sham modification, unmodified clone, wear-only control |
| Composite | component count, bond graph depth, assembly persistence, whole-minus-parts effect | components alone, randomly assembled configuration, weak-bond control |
| Metatool | dependency graph depth, intermediate-object necessity, sequence flexibility | intermediate removal, direct target access, action-order perturbation |
| Social acquisition | adoption hazard after exposure, sequence similarity, object-choice convergence | no demo, view blocked, ghost/yoked demo, genetic clone controls |
| Tradition | persistence across cohorts, diffusion-network fit, between-group variation | environmental transplant, cross-fostering, social-learning disablement |
| Cumulative modification | performance frontier, lineage edit depth, dependency depth, ratchet retention | artifact reset, social isolation, lineage mixing, bottleneck, independent rediscovery rate |

## 10.7 Avoiding false positives

### Object-assisted access versus body-assisted access

An organism may use its own body as a wedge, battering ram, or bridge. This can be sophisticated behavior, but under conservative definitions it is not external tool use. Log it separately as embodied environmental modification.

### Reuse versus camping

Repeated action near an object does not demonstrate retrieval or durable value tracking. Require separated episodes or displacement away from the artifact.

### Property selection versus location learning

Randomize object locations and superficial cues. Test novel identities and interpolated property values.

### Social learning versus local enhancement

An observer may approach the place where a demonstrator was active without copying an action. Use ghost controls, object-only motion, demonstrator visibility without target visibility, and relocated apparatuses.

### Tradition versus genetic divergence

Use cross-fostering, common-garden environments, genotype matching, social-network interventions, and artifact transplantation.

### Cumulative culture versus cumulative wear

An artifact can change over generations because it degrades or because each user leaves random modifications. Improvement must be directional relative to a measured function and depend on socially preserved information.

## 10.8 Tool manufacture and modification

Tool manufacture should be reserved for episodes in which an organism intentionally—or, more cautiously, systematically—alters an object before later use. Biological examples include hook-tool manufacture and spontaneous construction by naïve juvenile crows [R12](#r12), while later work links hook form to foraging efficiency [R13](#r13).

In the simulation, intent should be inferred only from policy evidence such as:

- modification occurs preferentially before contexts where the altered object is useful;
- the organism stops modifying at a functional property range;
- failed modifications are corrected;
- modified objects are retained or retrieved;
- intervention on the modification changes later behavior;
- the pattern generalizes across raw object identities.

One accidental fracture followed by opportunistic use is better classified as **opportunistic use of a modified object** than manufacture.

---

# 11. World-representation comparison

## 11.1 Evaluation criteria

The representation must support thousands of organisms and potentially many more artifacts while preserving:

- deterministic state transition;
- stable identity;
- collision and support;
- explicit persistence;
- bounded save growth;
- canonical parallel update;
- behavior that evolving controllers can discover;
- enough geometry for meaningful manipulation and construction.

Ratings below are relative and assume careful implementation. “Determinism” refers to ease of specifying exact semantics, not whether any one implementation happens to be deterministic.

## 11.2 Comparison matrix

| Representation | Determinism | Performance at scale | Collision/support | Stable IDs and persistence | Save size | Parallel updates | Expressiveness for artifacts | Evolvability of behavior | Overall fit |
|---|---|---|---|---|---|---|---|---|---|
| **Dense 2D grid** | Excellent | Excellent for bounded worlds; cost proportional to full area | Simple occupancy; weak continuous contact | Cells stable, discrete objects awkward | Predictable but large for huge worlds | Excellent with fixed partitions | Moderate: digging, walls, paths; coarse manipulation | High accessibility, low motor complexity | Good substrate, insufficient alone |
| **Sparse/chunked grid** | Excellent if chunk order is canonical | Excellent when active area sparse | Similar to dense grid; chunk boundaries need care | Stable coordinates; explicit chunk state | Good; touched chunks dominate | Good with canonical chunk merges | Moderate to high for mutable terrain | High | Strong terrain choice |
| **Full voxels** | Good | Moderate to poor as world and mutable volume grow | Natural support and construction; many contacts | Cell identity stable; artifacts often become block sets | High, especially 3D dirty chunks | Good locally, costly across boundaries | High for block construction; low for smooth shapes | Moderate; manipulation often becomes block semantics | Too expensive/semantic for first model |
| **Continuous 2D rigid bodies** | Good with custom fixed-point protocol | Good with spatial broadphase | Strong planar collision, weak roofs/over-under | Excellent object IDs | Moderate | Moderate; contact islands complicate ordering | High for portable objects, barriers, projectiles | Moderate; manageable controls | Strong object layer |
| **Continuous 3D rigid bodies** | Difficult across platforms and parallel solvers | Poorer by large constant factors | Most expressive; contact instability and stacking difficult | Excellent IDs | Moderate to high | Difficult to make exact | Very high | Low initial accessibility due to perception/control dimensionality | Premature |
| **Graph world** | Excellent | Excellent | Abstract adjacency only; no grounded contact geometry | Excellent | Low | Excellent | Low to moderate; relationships easy, physical affordances weak | High for planning, weak embodiment | Poor primary world, useful auxiliary structure |
| **Hybrid grid + discrete objects** | Excellent to good | Good to excellent with simple bodies | Strong enough for terrain, contact, support, and placement | Excellent | Moderate and controllable | Good with phased protocol | High for target phenomena | Moderate to high | **Best balance** |
| **Entity-component system** | Depends on iteration discipline | Excellent storage/query potential | Not a physics representation by itself | Excellent if IDs stable | Efficient | Good but determinism must override archetype order | Neutral | Neutral | Recommended implementation architecture only |
| **Cellular materials / cellular automata** | Excellent | Good to moderate | Local transformation natural; rigid-body motion awkward | Cell identity stable, artifact identity emergent | Potentially high | Excellent | High for growth/chemistry, lower for portable persistent objects | Moderate | Later material subsystem, not primary |
| **Particle-based matter** | Difficult if neighbor order/float solvers vary | Poor with large particle counts | Rich fluids/deformation; expensive contacts | Weak artifact identity unless clustered | Very high | Parallelizable but exact merges hard | Very high material expressiveness | Low due to noisy high-dimensional dynamics | Unsuitable initially |

## 11.3 Dense and sparse grids

Grid worlds are deterministic, easy to serialize, and evolutionarily accessible. Terrain digging, deposition, paths, barriers, local chemistry, and fields map naturally to cells. Their weakness is object manipulation: rotation, leverage, continuous placement, grasp geometry, and composite identity become awkward or heavily discretized.

A sparse chunked grid retains the benefits while allowing large procedural worlds. It also supports a useful persistence policy:

- untouched chunk: version-pinned procedural recipe reference;
- touched chunk: materialized state or exact canonical delta;
- modified field chunk: explicit state;
- retired/far chunk: content-addressed compressed snapshot.

Chunk iteration must use coordinates in canonical lexicographic or Morton order, never hash-map order.

## 11.4 Voxels

Voxels make digging, stacking, walls, tunnels, and volume intuitive. They also tempt the design toward Minecraft-like semantics: every block becomes a named item, construction becomes placing blocks on a lattice, and manipulation becomes inventory transfer rather than force.

Computationally, 3D mutable chunks multiply cells, neighbor interactions, lighting/flow work, and save size. Voxel structures are easy to identify geometrically but individual artifacts may become large sets of cell coordinates with expensive lineage tracking.

**Recommendation:** do not use full voxels as the primary representation. A 2D terrain grid with height/elevation and a limited object layer captures most early research questions at far lower cost.

## 11.5 Continuous 2D and 2.5D

Continuous 2D bodies enable rotation, contact, pushing, pulling, striking, object choice by geometry, and stable identity. A 2.5D extension adds height intervals, support surfaces, and limited over/under relationships while retaining a planar broadphase.

The main challenge is defining support and stacking without smuggling in full 3D. A tractable model can use:

- each body has base height and top height;
- contacts are horizontal footprint overlaps plus compatible height intervals;
- support edges connect a body to terrain or bodies beneath it;
- load propagates through a canonical support graph;
- locomotion can traverse a top surface if slope/step constraints permit;
- line of sight and field occlusion use height bands;
- tipping is approximated from center-of-mass projection and support polygon/category.

This is an engineering abstraction, not classical rigid-body physics. It should be documented as such and validated against intended qualitative invariants.

## 11.6 Continuous 3D

Full 3D would permit arbitrary tools, roofs, bridges, containers, throwing, and articulated mechanisms. It would also impose:

- 3D sensory representations;
- multi-axis manipulation;
- unstable stacking and constraint solving;
- far more collision candidates and contact manifolds;
- higher save and visualization complexity;
- stronger dependence on floating-point and library versions;
- significantly more difficult evolutionary search.

The scientific question is not whether 3D is more realistic. It is whether the extra degrees of freedom are necessary to test the target hypotheses. For the first twelve experiments, most are not.

## 11.7 Graph representations

Graphs are excellent for social networks, bonds, support relationships, paths, lineage, and dependency analysis. They are poor as the only physical world because spatial arrangement, force, occlusion, containment, and embodied movement must be encoded as arbitrary edge semantics.

Use graphs as secondary structures:

- bond graph;
- support graph;
- social observation graph;
- artifact lineage graph;
- causal dependency graph;
- region connectivity graph derived from terrain.

## 11.8 Entity-component systems

An ECS can store bodies, materials, organisms, bonds, and fields efficiently. It does not guarantee deterministic semantics. Archetype movement, sparse-set iteration, parallel query order, and entity recycling can all produce divergence.

Rules for a deterministic ECS implementation:

- external stable IDs are never recycled;
- internal handles may change but cannot affect semantics;
- all semantically relevant iteration materializes and sorts stable IDs or uses intrinsically ordered storage;
- additions/removals are deferred to canonical commit phases;
- component serialization is schema-versioned and field-order canonical;
- no pointer address, allocation order, or archetype index enters random keys or tie-breaking.

## 11.9 Cellular and particle systems

Cellular material models are attractive for growth, diffusion, local chemistry, and morphology. Particle systems are attractive for fluids, granular matter, and deformation. Both conflict with the immediate need for persistent, reusable artifact identity.

A later hybrid may represent:

- terrain and diffuse material cellularly;
- large portable bodies discretely;
- transitions between representations at deterministic thresholds.

The first implementation should not carry millions of grains to simulate a pile that can be represented by one aggregate.

## 11.10 Procedural and generative environments

Procedural generation remains useful after terrain becomes mutable. It should generate initial conditions and untouched regions, not overwrite history.

A good generator exposes distributions over:

- terrain geometry and connectivity;
- material-property fields;
- resource renewal and hazards;
- object sizes and shapes;
- seasonal or exogenous cycles;
- occlusion and visibility;
- spatial correlation lengths.

It should not generate “hammer puzzle,” “bridge task,” or “crafting station” labels. Difficulty descriptors should be physical: barrier toughness, gap width, required reach, transport distance, hazard exposure, resource depth, and material availability.

Generator version is part of save semantics. A save that references pristine chunks must preserve the exact generator implementation or an equivalent frozen data representation.

---

# 12. Recommended minimal world model

## 12.1 Architectural overview

The recommended model is a **deterministic sparse 2.5D material terrain plus discrete-body system**.

```text
World
├── WorldHeader / semantic versions / seed / tick / ID allocator
├── MaterialRegistry
├── ChunkStore
│   ├── Terrain cells
│   ├── Field layers
│   └── touched/pristine/materialized status
├── EntityStore
│   ├── PhysicalBody components
│   ├── Organism components
│   ├── Effector and grasp state
│   ├── Learned/controller state
│   └── Provenance
├── BondStore
├── SpatialIndex (derived and rebuildable)
├── SupportGraph (derived or validated cache)
├── PendingActionStore
├── EventJournal
└── CheckpointManifest
```

## 12.2 Terrain cell schema

A minimal cell might contain:

```rust
struct TerrainCellV1 {
    material: MaterialKey,
    elevation_q: i32,
    loose_quantity_q: u16,
    compaction_q: u8,
    damage_q: u16,
    flags: TerrainFlagsV1,
}
```

Optional fields belong in explicit later schema versions. Avoid a generic unversioned property map; it makes fail-closed loading and deterministic layout harder.

`flags` should represent mechanical state such as impermeable boundary or generator-reserved world edge, not semantic labels such as path, mine, wall, or farm.

## 12.3 Material schema

A first material schema could be:

```rust
struct MaterialDefV1 {
    stable_key: MaterialKey,
    density_q: u16,
    hardness_q: u8,
    toughness_q: u16,
    wear_resistance_q: u8,
    static_friction_q: u8,
    dynamic_friction_q: u8,
    support_strength_q: u16,
    nutritional_energy_q: u16,
    toxicity_q: u8,
    adhesion_q: u8,       // inactive until bonds are enabled
    perceptual_surface: SurfaceCueProfileV1,
}
```

The perceptual surface profile should not be a one-to-one encoding of stable material identity. Multiple materials may share cues; one material may vary by body state. Experimental controls should decorrelate superficial cues from utility.

## 12.4 Physical body schema

```rust
struct PhysicalBodyV1 {
    id: EntityId,
    shape: ShapeV1,
    pose: PoseFixedV1,
    velocity: VelocityFixedV1,
    base_height_q: i16,
    height_q: u16,
    material: MaterialKey,
    quantity_q: u32,
    integrity_q: u16,
    wear_q: u16,
    sleep_state: SleepStateV1,
    provenance: ProvenanceRef,
}
```

Mass, center of mass, collider bounds, and derived edge acuity should be cached but recomputable from authoritative state. Derived caches should either be excluded from saves and rebuilt canonically or stored with validation hashes and ignored if mismatched.

## 12.5 Shape palette

Recommended initial shapes:

- disc;
- axis-aligned or quantized-orientation box;
- capsule/rod;
- triangle or wedge;
- one shallow concave container assembled from convex parts rather than a general concave collider.

Each shape needs:

- canonical parameter order;
- fixed-point area/volume approximation;
- deterministic support footprint;
- a small set of contact regions;
- deterministic fracture templates;
- perceptual outline features.

Do not allow arbitrary user meshes in authoritative simulation. Visualization meshes can be derived and non-authoritative.

## 12.6 Organism effectors

A minimal organism may have one or two effectors with:

- body-relative anchor;
- reach radius or short kinematic segment;
- grip strength;
- maximum force;
- aperture/size range;
- contact region;
- current grasp constraint;
- energy cost coefficients;
- tactile feedback.

Variable morphology can later evolve these values. For the first experiments, fixed effectors reduce confounding. If morphology is already evolvable, experimental analysis must control for effector differences when measuring object affordances.

## 12.7 2.5D support model

A practical support step:

1. detect horizontal footprint overlap or contact;
2. determine candidate lower supports by height interval;
3. sort support candidates by top height, overlap area proxy, then stable ID;
4. assign load fractions with a versioned deterministic rule;
5. propagate load in canonical topological order where acyclic;
6. detect cycles and resolve through a fixed canonical cycle policy;
7. compare load to body/material support strength;
8. accumulate compression damage or trigger collapse next commit phase.

Bodies may be traversable surfaces if top area and step height satisfy organism morphology. “Bridge” is not a property; traversal emerges from support and hazard geometry.

## 12.8 Spatial indexing

Use a deterministic uniform spatial hash or chunk-local broadphase:

- derive occupied buckets from fixed-point AABBs;
- store bucket membership sorted by stable entity ID;
- generate candidate pairs and canonicalize as `(min_id, max_id)`;
- sort/deduplicate pair keys before narrowphase;
- rebuild or update the index through canonical commit order;
- treat the index as a derived cache, not authoritative save state.

A dynamic tree can be faster in some cases but insertion order and balancing become part of exact semantics unless carefully specified. A uniform grid is easier to audit and parallelize for the expected object scale.

## 12.9 Fields

Fields should use chunked low-resolution grids, potentially coarser than terrain. Update cadence can be every `N` simulation ticks, where `N` is fixed and versioned. Diffusion/advection uses integer stencils, canonical neighbor order, and double buffering.

Do not update fields asynchronously based on available compute. Do not skip faraway chunks based on camera or current population unless the skip rule is deterministic and based only on serialized world state.

## 12.10 Matter and provenance

Every matter-changing event should preserve quantity at the modeled resolution. Provenance should be compact but sufficient for artifact genealogy:

```rust
struct ProvenanceRecordV1 {
    record_id: ProvenanceId,
    creation_tick: Tick,
    cause_kind: CauseKindV1,
    primary_parent: Option<EntityId>,
    secondary_parent: Option<EntityId>,
    source_cells_hash: Option<Hash256>,
    actor_id: Option<EntityId>,
    material_quantity_summary: MaterialQuantitySummaryV1,
}
```

For aggregates with many ancestors, use a bounded ancestry summary or Merkle root plus retained recent parents. Full ancestry can be stored in an analytics archive separate from the minimum restore state.

## 12.11 Initial feature phases

### Phase A — object permanence and carrying

- stable IDs and provenance;
- simple bodies;
- density, friction, nutritional energy;
- object perception;
- grasp/release;
- pushing and controlled force;
- explicit mutable-world save format.

### Phase B — placement, terrain change, and structures

- hardness, toughness, wear;
- excavation/deposition;
- damage and bounded fracture;
- 2.5D support and height occlusion;
- caches, barriers, paths;
- structure analytics.

### Phase C — bonds and composites

- broad adhesion/bond formation;
- bond graph and assembly revision identity;
- composite collision/support approximation;
- metatool/dependency analytics;
- strict object-count controls.

### Phase D — stigmergy and ecological inheritance

- generic deposited field or sufficiently rich sematectonic traces;
- observer visibility logs;
- social-learning controls;
- artifact transplantation and branch replay at scale.

### Phase E — optional advanced material domains

- temperature and heat transfer;
- permeability/flow;
- bounded reactions;
- flexible constraints;
- phase changes.

No phase is justified merely because the mechanism sounds realistic. Progression depends on experiment results and measured performance headroom.

## 12.12 Why this model is minimal but expressive

This model can support, without named recipes:

- carrying and sorting by physical properties;
- object-assisted access to matter;
- persistent reuse;
- caching and retrieval;
- piles, barriers, paths, pits, supports, and simple cover;
- object arrangements as signals or memory;
- bounded fracture and shaping;
- rigid composites;
- stigmergic action through physical traces;
- inherited artifact environments;
- social observation of manipulation;
- multi-step dependency chains.

It cannot initially support realistic fluids, fire, cloth, ropes, complex articulated machines, sealed vessels, or arbitrary 3D architecture. That limitation is deliberate. Adding those systems before simpler object-mediated behavior evolves would make failures uninterpretable and performance problems harder to isolate.

---
# 13. Deterministic interaction-resolution protocol

## 13.1 Determinism contract

The mutable-world subsystem should publish a formal contract stronger than “same seed usually produces the same result.” A suitable contract is:

> Given identical authoritative state, simulation-semantic version, configuration, keyed random inputs, and external command stream, the engine produces byte-identical canonical authoritative state and event hashes after every tick on every supported execution configuration covered by the determinism tier.

The contract should define tiers if cross-platform identity is not immediately feasible:

- **Tier 1 — same binary/build:** exact replay on the same executable and architecture.
- **Tier 2 — supported platform family:** exact replay across supported machines using the same compiler target and feature set.
- **Tier 3 — cross-platform semantic determinism:** exact canonical state across supported architectures and operating systems.

The Genesis ambition implies Tier 3 is preferable for authoritative simulation. If a third-party physics engine prevents that, freeze the simulation environment and state the weaker tier explicitly rather than claiming more.

## 13.2 Tick phases

A deterministic tick should separate reading, proposing, resolving, and committing.

### Phase 0 — checkpointable tick boundary

Authoritative state at tick `t` is immutable for readers. All pending multi-tick actions, field schedules, and delayed physiological effects are already part of state.

### Phase 1 — derive caches

Rebuild or incrementally update non-authoritative caches in ways that cannot change results:

- spatial buckets;
- broadphase AABBs;
- local percept query indices;
- derived mass and collider values;
- support candidate indices.

Any cache miss must recompute the same value; cache presence cannot affect semantics.

### Phase 2 — sensing

For each organism in ascending stable ID order, or in deterministic parallel partitions whose outputs are merged canonically:

1. query physical state at tick `t`;
2. generate candidate percepts in canonical spatial and entity order;
3. apply deterministic occlusion and salience;
4. address perceptual noise by named keyed RNG coordinates;
5. write a fixed-schema observation buffer.

Organisms never see partially resolved actions from the current tick.

### Phase 3 — controller evaluation

Controllers consume observations and their authoritative internal/learned state. Controller updates must obey the existing determinism contract. Outputs are quantized into intents.

```rust
struct IntentV1 {
    actor_id: EntityId,
    actor_intent_seq: u32,
    phase: IntentPhaseV1,
    kind: IntentKindV1,
    target: Option<EntityId>,
    cell_target: Option<CellCoord>,
    params: QuantizedIntentParamsV1,
}
```

`actor_intent_seq` is deterministic within the actor for the tick. Intents are not applied immediately.

### Phase 4 — validation against the tick snapshot

Validate reach, effector state, target existence, physiological capacity, and parameter ranges against state at tick `t`. Invalid intents become explicit failure events or no-ops according to versioned policy. They are never silently retargeted.

Validation does not reserve targets. Two organisms may both submit valid grasp attempts for the same body.

### Phase 5 — canonical grouping and conflict resolution

Sort intents by a total key such as:

```text
(intent_phase,
 conflict_locus_kind,
 conflict_locus_id_or_coord,
 actor_id,
 actor_intent_seq)
```

Build conflict groups for exclusive or capacity-limited state:

- body grasp slots;
- effector occupancy;
- discrete placement/support slot if used;
- material withdrawal from one finite source;
- bond endpoint capacity;
- exclusive organism ingestion aperture;
- terrain quantity requested beyond availability.

Resolve each group from the same snapshot and write resolution records. Do not mutate authoritative state yet.

### Phase 6 — force and contact proposal

Convert accepted ongoing actions and locomotion into force/kinematic proposals. Accumulate proposals per body and terrain locus in canonical order using fixed-point arithmetic.

Broadphase produces canonical pair keys `(min_id, max_id)`; narrowphase produces contacts sorted by a deterministic manifold key. Contact normal conventions must be fixed by endpoint order.

### Phase 7 — mechanics solve

Resolve movement, contact, grasp constraints, bonds, and support with:

- a fixed number of solver iterations;
- fixed constraint ordering or deterministic graph coloring;
- no convergence-dependent early exit;
- explicit rounding after every defined operation;
- no platform transcendental functions in authoritative paths;
- saturating or checked overflow behavior defined by schema version.

If deterministic parallel solving is introduced, partition constraint islands by canonical root and merge only commutative or canonically ordered outputs.

### Phase 8 — aggregate physical consequences

From solved contacts and actions, accumulate:

- damage;
- wear;
- terrain excavation/deposition;
- matter transfer;
- bond load and formation progress;
- physiological work cost;
- field deposits;
- observed-action records.

All contributions are based on the same post-mechanics/pre-transformation state and are sorted or combined by associative integer operations with defined saturation.

### Phase 9 — transformations

Apply, in canonical order:

1. bounded matter transfers;
2. damage and wear;
3. bond failures;
4. fractures/destruction;
5. terrain mutation;
6. bond formations;
7. aggregate/pile transitions;
8. organism ingestion and physiological effects;
9. field source updates.

The order must be specified because it changes outcomes. For example, whether a bond breaks before fracture or whether ingestion precedes object destruction cannot be left to component-system scheduling.

### Phase 10 — spawn/despawn commit

Collect all create/destroy requests with causal keys:

```text
(cause_phase,
 source_event_id,
 parent_or_cell_key,
 child_ordinal)
```

Sort requests, allocate new stable IDs monotonically in that order, instantiate entities, then process tombstones. IDs are never reused.

### Phase 11 — field update and delayed systems

Update scheduled field chunks with double buffering and fixed stencils. Process delayed physiology, decay, regrowth, and multi-tick action advancement in explicit phase order.

### Phase 12 — canonical event log and hashes

Emit events in a canonical total order. Compute:

- per-section incremental hashes;
- mutable-world state hash;
- organism learned-state hash;
- event-batch hash;
- root tick hash chaining the previous tick hash.

Advance to tick `t + 1`.

## 13.3 Randomness protocol

A single sequential PRNG stream is fragile. Adding a new random draw for one organism shifts every later draw and makes parallelism order-dependent. Use a counter/keyed generator, following the general approach of counter-based random-number work [R45](#r45).

A random address should contain a named stream domain and stable coordinates:

```text
RandomAddress {
    world_epoch,
    semantic_rng_version,
    domain,              // mutation, perception noise, tie break, fracture, etc.
    tick_or_generation,
    primary_entity_id,
    secondary_entity_id_or_zero,
    event_ordinal,
    draw_ordinal,
}
```

The output is a pure function of the world RNG key and address. Domain separation prevents a fracture draw from colliding with a perception draw. Random addresses must not contain memory addresses, internal ECS handles, thread IDs, or iteration indices derived from unstable containers.

For branch replay, include a branch policy:

- unaffected baseline events retain baseline addresses;
- intervention-created events use a branch namespace;
- analytics records whether common-random-number coupling is preserved.

## 13.4 Contested pickup

Contested pickup is an exclusive claim over a grasp slot. A deterministic resolution policy should be physical first and random only for unresolved ties.

### Eligibility

A contender is eligible if, at tick `t`:

- target exists and is graspable by geometry;
- effector is available;
- target is within reach;
- line/contact path is not blocked;
- organism has sufficient immediate capacity to establish minimum grip;
- target has an available grasp capacity or multi-grasp is enabled.

### Priority score

A physical score may include:

- contact already established;
- grip geometry quality;
- distance within reach;
- available grip force;
- relative target motion;
- action initiation tick for ongoing attempts.

Avoid always choosing the lowest entity ID. That is deterministic but introduces a persistent arbitrary fitness advantage. If physical scores tie, use a keyed permutation or keyed random rank based on `(world_epoch, tick, target_id, contender_id, grasp_slot)`.

### Commit

Only the winner creates the grasp constraint. Losers receive explicit failure feedback and pay any attempted-action cost. A target destroyed or moved by earlier specified phases invalidates the grant according to a documented policy; the resolution should not silently transfer to the next contender unless that fallback is part of the protocol.

## 13.5 Contested placement

Preferred placement is physical release, so most conflicts resolve through collision and support rather than discrete occupancy. If a grid-aligned placement mode is retained for performance, it must be treated as a reservation problem:

1. validate candidate poses against tick-`t` state;
2. build overlap conflict groups among candidate placements and existing immovable bodies;
3. rank by physical feasibility and keyed tie-break;
4. commit winners simultaneously;
5. reject losers without snapping to arbitrary adjacent cells.

A hidden “find nearest free slot” routine is dangerous because search order becomes semantics and can teleport matter.

## 13.6 Collisions

Collision determinism requires more than fixed point:

- candidate pairs sorted canonically;
- shape-pair dispatcher symmetric or endpoint-order specified;
- contact points sorted by feature IDs and quantized coordinates;
- normal orientation fixed from lower to higher entity ID;
- warm-start impulses either serialized or deterministically rebuilt;
- fixed solver iterations;
- sleeping transitions based on integer thresholds over fixed intervals;
- no nondeterministic SIMD reductions;
- canonical handling of simultaneous penetration correction.

Box2D documents that creation order, event order, compiler options, and mathematical implementation can affect deterministic behavior even when the library is designed to be deterministic [R46](#r46). Rapier similarly documents construction-order requirements and tradeoffs among cross-platform determinism, SIMD, and parallelism [R47](#r47). These are useful warnings: library integration must be wrapped by a Genesis-specific semantic contract and golden replay tests.

## 13.7 Object destruction and simultaneous damage

Suppose several organisms strike one body in the same tick. Applying each hit sequentially could cause the first threshold-crossing hit to determine fracture direction or credit arbitrarily.

Recommended protocol:

1. compute all damage contributions from the common contact state;
2. canonicalize by `(target_id, region, source_event_id)`;
3. sum with defined saturation;
4. determine whether integrity thresholds are crossed;
5. derive fracture-driving impulse from the vector or categorical aggregate of all qualifying contributions;
6. choose a fracture template with a keyed draw only if physical state underdetermines it;
7. attribute causal credit fractionally or retain all source event IDs.

A body destroyed this phase remains a valid target for all contributions already resolved from the tick snapshot. New actions cannot target its fragments until the next tick.

## 13.8 Terrain mutation conflicts

Terrain updates should be expressed as commutative deltas whenever possible:

```text
CellDelta {
    remove_quantity_by_material,
    add_quantity_by_material,
    compaction_impulse,
    damage_energy,
    elevation_mass_delta,
    field_sources
}
```

If requested removal exceeds available quantity, allocate proportionally or through a canonical priority policy. Proportional integer allocation should use a deterministic largest-remainder method with ties resolved by stable event key.

Avoid last-writer-wins. It makes thread or intent order determine who obtained material.

## 13.9 Bond creation conflicts

Bodies may have bounded bond degree or surface capacity. Concurrent bond attempts should:

1. compute candidate contact patches from the same state;
2. rank by contact area/pressure/duration and physical compatibility;
3. enforce geometric non-overlap of anchor regions where relevant;
4. use keyed tie-break only for exact residual ties;
5. allocate bond capacity canonically;
6. charge attempted resources even if formation fails, when physically appropriate.

## 13.10 Composite motion and collision

Treating every member of a large rigid composite as a separate collider can make contact cost grow rapidly. A two-tier method is appropriate:

- small assemblies: solve member colliders and bonds directly;
- stable sleeping rigid assemblies: build a derived compound broadphase proxy while retaining member-level narrowphase on contact;
- very large static structures: optionally promote to a structure collision island keyed by assembly revision, without changing member identity.

Promotion/demotion thresholds must be deterministic and state-based. Derived proxies are rebuildable caches and do not replace authoritative bonds or bodies.

## 13.11 Parallel update strategy

Exact determinism and parallelism are compatible if work partitioning is semantically inert.

Recommended pattern:

1. partition chunks or constraint islands by stable key ranges;
2. read immutable state;
3. produce thread-local sorted delta streams;
4. merge streams by a global canonical key;
5. apply authoritative mutations serially or in disjoint key partitions proven conflict-free;
6. verify parallel and single-thread tick hashes in continuous testing.

Avoid atomic floating-point accumulation, work stealing whose completion order affects event order, and hash-based reduction.

## 13.12 Deterministic failure policy

Every impossible or corrupted operation needs a specified outcome:

- invalid target: explicit failure/no-op;
- arithmetic overflow: fail closed in debug/validation; production may use defined saturation only where scientifically acceptable;
- impossible geometry or NaN equivalent: abort authoritative tick and preserve last valid checkpoint;
- exceeded local body/constraint budget: deterministic rejected operation or aggregate transition, never silent deletion;
- unknown action/schema enum: restore failure, not default mapping;
- dangling entity reference: restore failure;
- inconsistent material quantity: restore failure or explicit offline repair tool, never runtime guess.

---

# 14. Stable identity, save, restore, and migration

## 14.1 Stable entity identity

Use a stable nonzero `EntityId`, preferably 64-bit if exhaustion analysis is comfortable or 128-bit if world epochs and long horizons justify it.

Properties:

- monotonically allocated within a world epoch;
- never reused, including after destruction;
- allocation occurs only in canonical spawn commit order;
- `next_entity_id` is authoritative saved state;
- internal ECS handles are not serialized as identity;
- entity IDs do not encode mutable type or location;
- import/migration cannot collide with existing IDs.

A 64-bit monotonic ID supports approximately 1.84 × 10^19 values. At one million allocations per simulated second, exhaustion would still take hundreds of thousands of years of simulated allocation time; actual disk and compute limits will dominate. A 128-bit ID offers easier namespacing but doubles storage in contact/event-heavy systems. This is an engineering tradeoff, not a scientific requirement.

## 14.2 World epoch and imported identity

A `WorldEpochId` or save-lineage UUID can distinguish independent worlds. Random-address construction and external analytics may use `(world_epoch, entity_id)`.

For branch simulations:

- baseline entities retain their IDs;
- branch-created IDs can use the same local ID sequence inside an isolated branch context if branches never merge;
- if branches may merge into one analytics store, include branch ID in the external compound identity rather than contaminating authoritative in-world IDs.

## 14.3 Artifact identity

Do not allocate a separate artifact ID merely because an observer finds a body interesting. The physical body ID is sufficient. Analytics may create an `ArtifactEpisodeId` for a use episode and an `ArtifactLineageId` for a provenance cluster, but these are observer records.

When matter changes representation:

- fracture: parent body ends; children receive new IDs and parent provenance;
- merge by bond: members retain IDs; assembly handle changes;
- split by bond failure: members retain IDs; assembly handles change;
- terrain extraction: new body receives ID and references source cells/provenance;
- aggregation into rubble: member bodies end; aggregate body/cell receives new or existing aggregate identity plus ancestry summary;
- re-resolution: new bodies receive IDs from the aggregate source; do not resurrect old fragment IDs.

## 14.4 Structure identity

A persistent “structure ID” is usually unnecessary and can become semantically misleading. Structures may change continuously through maintenance, partial collapse, and accretion. Recommended observer representation:

```text
StructureObservation {
    observation_id,
    interval,
    member_body_ids,
    terrain_region,
    assembly_revision_hashes,
    spatial_signature,
    measured_functions,
    lineage_links_to_previous_observations
}
```

For runtime physics, use assembly roots, support islands, or static collision-island keys. These are derived mechanical identities and may change when membership changes.

## 14.5 Canonical component ordering

Canonical serialization is part of the save contract:

- entities sorted by stable ID;
- components emitted in schema-defined type order;
- terrain chunks sorted by canonical coordinate order;
- cells within chunks in fixed row-major or Morton order;
- material registry sorted by stable material key;
- bonds sorted by canonical endpoint tuple and bond key;
- maps serialized as sorted arrays, never raw hash-table iteration;
- variable-length lists sorted if order is semantically irrelevant;
- semantically ordered lists preserve explicit ordinal fields;
- all integers use fixed endianness;
- fixed-point scales are schema constants or header fields;
- text, if any, uses normalized UTF-8 and is excluded from physics unless strictly specified.

Canonical order supports reproducible hashes, binary diffing, corruption detection, and deterministic migration.

## 14.6 Successor save-format header

A save header should contain at least:

```text
magic
container_format_version
simulation_semantic_version
world_schema_hash
build_or_engine_compatibility_id
endianness_marker
fixed_point_profile_id
math_lookup_table_version
rng_algorithm_and_version
world_epoch_id
world_seed_or_rng_key_reference
world_configuration_hash
initial_generator_id_and_version
current_tick
next_entity_id
checkpoint_kind
parent_checkpoint_hash_if_incremental
section_manifest_offset
root_state_hash
```

The header must not be sufficient on its own to accept a save. All required sections and invariants must validate.

## 14.7 Required authoritative sections

A complete checkpoint should include:

1. **Generation recipe and pristine-region manifest** — exact generator version and inputs for still-unmaterialized regions.
2. **Material registry** — all material definitions and active schema versions.
3. **Mutable terrain chunks** — materialized cells or exact deltas, plus explicit pristine/materialized state.
4. **Field layers** — current buffers, phase/cadence state, and source accumulators if they affect the future.
5. **Physical bodies** — complete authoritative body state.
6. **Organisms** — morphology, physiology, neural/genome state, lifetime-learned state, action state, lineage.
7. **Grasps and constraints** — persistent control relations.
8. **Bonds** — all endpoints, anchors, strength state, curing/damage state.
9. **Pending actions and schedulers** — multi-tick actions, delayed effects, regrowth/decay queues if not derivable solely from current state and tick.
10. **Identity allocators** — next entity, provenance, event, and other authoritative ID counters.
11. **Provenance and lineage** — minimum state required for future mechanics and scientific traceability.
12. **Journal cursor and hash chain** — relation to bounded events after the snapshot.
13. **Configuration and feature gates** — exact active mechanics and limits.

Spatial indexes, render meshes, navigation caches, contact broadphase trees, and observer classifications should be rebuildable and omitted unless retaining them is required for exact solver warm-start semantics. If solver caches affect results, they become authoritative and must be serialized or deterministically reconstructed.

## 14.8 Mutable terrain and the regeneration invariant

The old invariant can survive only in a narrowed form:

- a chunk explicitly marked `Pristine(generator_id, generator_version, input_hash)` may be regenerated;
- a chunk marked `Materialized` must load its stored authoritative state;
- a chunk marked `Delta(base_hash, delta_schema)` must regenerate the exact base with the exact version, verify the base hash, then apply the canonical delta;
- missing chunk metadata is corruption, not permission to regenerate;
- a generator mismatch is a restore failure unless an explicit migration materializes the old result.

A chunk becomes touched when any future-relevant state differs from its generated baseline, including terrain material, elevation, compaction, damage, field state, scheduled change, embedded object relation, or any generator-dependent ecological state not otherwise serialized.

## 14.9 State snapshots versus event sourcing

### Pure snapshots

Advantages:

- direct restore;
- bounded recovery time;
- simple authoritative semantics;
- easier fail-closed validation.

Disadvantages:

- large repeated writes;
- weaker causal history unless separately logged;
- expensive frequent checkpoints.

### Pure event sourcing

Advantages:

- complete audit trail;
- compact when events are sparse;
- natural branching and lineage analysis.

Disadvantages:

- replay time grows without compaction;
- event semantics must remain executable forever;
- corruption early in the chain invalidates later state;
- high-frequency contacts and fields can exceed snapshot size;
- migrations across semantic versions become difficult.

### Recommended hybrid

Use authoritative full or incremental snapshots plus a bounded event journal:

- periodic full snapshot;
- content-addressed incremental snapshots of changed sections/chunks;
- journal since the latest durable snapshot;
- periodic compaction into a new full snapshot;
- separate long-term analytics archive that may store richer events but is not required to restore.

The restore journal should contain only events needed to reconstruct the current state between checkpoints. Analytics logs can be larger, schema-evolving, and independently compressed.

## 14.10 Incremental snapshots and compaction

A content-addressed checkpoint can store immutable blobs for:

- each terrain chunk;
- each field chunk;
- ranges of sorted entities;
- bond blocks;
- organism learned-state blocks;
- provenance blocks.

The manifest maps canonical section keys to blob hashes. Unchanged blobs are reused. Checkpoint publication should be transactional:

1. serialize and hash new blobs;
2. write blobs to temporary paths;
3. flush and verify;
4. write manifest with parent reference and root hash;
5. flush manifest;
6. atomically publish the new checkpoint pointer;
7. retain prior checkpoint until new checkpoint has been reopened and validated;
8. garbage-collect unreferenced blobs only after retention policy permits.

Compaction creates a self-contained full manifest and may coalesce event journals and provenance summaries. Compaction must not change canonical authoritative state; verify pre/post state hashes.

## 14.11 Checksums and state hashes

Use cryptographic hashes for corruption detection and identity of canonical state, not as a substitute for validation.

Recommended hierarchy:

- blob hash for each serialized chunk/block;
- section Merkle root or ordered hash over blob keys and hashes;
- mutable terrain root;
- physical-body root;
- organism-genetic root;
- organism-learned-state root;
- bonds/constraints root;
- scheduler root;
- configuration root;
- whole-state root;
- tick event-chain hash.

A separate learned-state hash is valuable because lifetime plasticity must restore exactly. A mutable-world hash lets replay tests isolate world divergence from controller divergence.

Hash inputs must include schema/version tags and canonical lengths to avoid ambiguous concatenation.

## 14.12 Fail-closed restore

Restore must reject, not repair silently, when it finds:

- unknown required container or semantic version;
- schema hash mismatch;
- missing required section;
- duplicate stable IDs;
- noncanonical ordering where canonical form is required;
- dangling bond, grasp, scheduler, or provenance references;
- invalid material key;
- ID allocator not greater than all allocated IDs;
- impossible fixed-point range or overflowed quantity;
- inconsistent aggregate material totals beyond explicit tolerance;
- invalid chunk state such as both pristine and materialized;
- delta base hash mismatch;
- unsupported generator version for a pristine/delta chunk;
- learned-state shape incompatible with controller schema;
- checksum or root-hash mismatch;
- unknown enum variant without an explicit forward-compatible envelope;
- unresolved migration marker;
- journal parent/hash discontinuity.

An offline diagnostic or repair utility may inspect and produce a new explicitly marked repaired file. The simulator should never guess during ordinary load.

## 14.13 Save/restore equivalence tests

Required continuous tests:

1. run `N` ticks, snapshot, continue `M`; restore snapshot, run `M`; compare per-tick hashes;
2. snapshot with active grasps, bonds, sleeping bodies, field cadence phases, pending fracture, and learned plastic state;
3. compare full snapshot and incremental-chain restore;
4. restore on every supported determinism platform;
5. round-trip canonical serialization and verify byte identity;
6. deliberately permute internal container insertion order before serialization and verify identical bytes;
7. inject corruption in every section and verify fail-closed behavior;
8. verify obsolete/unknown versions are refused rather than reinterpreted;
9. compact and compare canonical state roots;
10. restore a branch checkpoint and verify baseline random addresses remain stable where intended.

## 14.14 Migration to the successor format

The new mutable-world format must have a different magic/version path from the immutable-terrain format. It must not treat missing mutable sections as an empty world.

Recommended migration process:

1. read old file using the old schema implementation;
2. verify old checksums and invariants;
3. resolve the exact old terrain generator and configuration;
4. regenerate the old immutable terrain into an explicit baseline state;
5. translate old organisms, controllers, learned state, lineage, and scheduler state field by field;
6. assign world epoch and initialize a monotonic entity allocator without collisions;
7. create empty-but-explicit mutable object, bond, field, and provenance sections;
8. materialize or explicitly mark pristine chunks under the old generator version;
9. write a new file; never modify the source in place;
10. record input hash, migrator version/build, output hash, and all assumptions;
11. restore the output under the new engine and run migration golden tests.

If the exact old generator implementation is unavailable, ambiguous, platform-dependent, or known to have changed under the same nominal version, migration should refuse. A user may choose an explicit approximate import into a new world, but that output must not claim exact lineage or replay continuity.

## 14.15 Versioning policy

Distinguish:

- **container format version:** framing, manifests, compression, section encoding;
- **state schema version:** fields and types in authoritative records;
- **simulation semantic version:** state-transition meaning;
- **generator version:** initial/procedural world generation;
- **RNG version:** algorithm and random-address mapping;
- **math profile version:** fixed-point scales and lookup tables;
- **analytics schema version:** observer event and classification format.

A file may be container-compatible but simulation-incompatible. Never equate successful parsing with valid continuation.

## 14.16 Long-term replay strategy

Exact replay across decades requires more than migrations. Options include:

1. preserve old executable/container images;
2. preserve semantic modules per major version;
3. materialize complete snapshots at migration boundaries and declare a new replay epoch;
4. maintain deterministic reference interpreters for old event journals;
5. retain golden histories with per-tick roots.

**Recommendation:** treat major simulation-semantic migration as a new replay epoch unless the migrator proves state equivalence. Preserve the old save and executable environment for historical replay. The new world may continue lineage only when the mapping is explicit and verified.

---
# 15. Dependency-ordered experiment plan

## 15.1 Program design

The experiments are ordered so that each stage validates prerequisites for the next. A failed stage should block interpretation of later results. For example, social transmission cannot be inferred if observers cannot reliably perceive actor-object relations; cumulative modification cannot be inferred if artifact identity and provenance are unstable.

All experiments should use:

- multiple world seeds and independent evolutionary replicates;
- held-out procedural environments;
- identical baseline checkpoints for deterministic branch interventions;
- explicit negative and sham controls;
- predeclared success criteria where practical;
- event-level logs plus checkpoint hashes;
- no direct reward, fitness bonus, or unlock for the observer label under study.

### Compute-cost notation

These are **engineering planning hypotheses**, not measured results:

- **L — low:** up to approximately 1.2× the current baseline tick cost; short behavioral evaluations.
- **M — medium:** approximately 1.2–2× baseline tick cost or moderate evolutionary replication.
- **H — high:** approximately 2–5× tick cost, substantial branch replays, or long evolutionary runs.
- **VH — very high:** more than approximately 5× effective experiment cost, multi-generation cultural controls, or extensive branching and lineage analysis.

Actual cost depends primarily on organism count, object density, contact rate, controller complexity, observation bandwidth, and checkpoint frequency. The multipliers should be replaced by benchmark results before scheduling long runs.

## 15.2 Experiment 1 — basic object perception

### Research question

Can organisms detect and behaviorally discriminate persistent bodies through physical cues without receiving semantic object or material labels?

### World configuration

Place simple bodies around organisms in ecologically neutral and relevant contexts. Vary independently:

- size;
- shape family;
- orientation;
- motion;
- coarse surface cue;
- distance and partial occlusion;
- material properties that are not all directly visible.

Include some bodies with nutritional value and some inert bodies, but randomize superficial appearance-property correlations across runs.

### Required mechanics

- stable body IDs internally;
- structured egocentric perception;
- deterministic occlusion and salience;
- contact/tactile sensing;
- locomotion and orientation;
- no grasp requirement yet.

### Controls

1. **No-object control:** same terrain and fields without bodies.
2. **Percept-shuffle control:** preserve percept count and marginal cue distributions while shuffling body-relative locations.
3. **Semantic-mask control:** verify material IDs and observer labels are absent from observation tensors.
4. **Appearance randomization:** remap superficial cues across physical properties between generations or evaluation worlds.
5. **Occlusion control:** hide bodies behind geometry while preserving remote field conditions.

### Ablations

- remove visual/object-range sensing;
- remove tactile feedback;
- remove temporal tracking token;
- reduce percept-slot count;
- randomize salience ordering;
- eliminate object motion cues.

### Metrics

- probe accuracy for decoding body presence, size band, motion, and shape from controller state;
- mutual information between physical cues and action changes;
- approach/avoidance response conditional on ecological consequence;
- discrimination accuracy on held-out identities and cue combinations;
- time to orient and contact;
- false-positive response to shuffled percepts;
- generalization to interpolated sizes and novel positions.

### Success criteria

Strong evidence requires behavior that depends on object cues, transfers to novel IDs/locations, and degrades under relevant sensory ablation. Merely approaching one visually distinctive body is insufficient.

### Expected failure modes

- superficial color/texture becomes a perfect proxy for utility;
- object slots reorder discontinuously and confuse controllers;
- perceptual bandwidth is too high for evolution;
- organisms ignore objects because ecological consequences are weak;
- collision reveals objects only after contact, producing reactive but not anticipatory behavior;
- stable tracking tokens accidentally expose global identity.

### Compute cost

**L–M.** Perception queries and occlusion dominate. Initial tests should use a small local body cap and no pairwise body dynamics beyond simple collision.

## 15.3 Experiment 2 — carrying

### Research question

Can organisms establish persistent control over a body, transport it under load, and release it at a new location?

### World configuration

Provide bodies spanning a smooth mass/size range. Create ecological opportunities where transporting nutritive matter or moving an obstacle changes later access, but do not reward carrying itself.

### Required mechanics

- grasp/release constraints;
- load-dependent locomotion and energy cost;
- object-body collision;
- grasp slip/break;
- explicit possession relation;
- save/restore with active grasp.

### Controls

1. **Grasp-disabled control:** bodies remain pushable but cannot be held.
2. **Inventory control:** optional benchmark variant where carried bodies become mass-aware abstract attachments; used only to quantify what physical carrying costs, not as final design.
3. **Zero-load control:** carried mass does not affect energy or movement.
4. **Teleport-placement control:** body can be relocated by a semantic action; demonstrates how much easier a scripted interface is.
5. **No-benefit control:** transporting bodies has no ecological consequence.

### Ablations

- remove grip feedback;
- remove proprioceptive load cue;
- remove action persistence;
- reduce or increase grip-force bands;
- remove transport energy cost;
- remove target tracking after grasp.

### Metrics

- controlled displacement normalized by organism body length;
- grip duration and slip frequency;
- fraction of body displacement attributable to grasp rather than collision;
- transport distance by mass band;
- energetic efficiency;
- successful release away from origin;
- load-sensitive policy adaptation;
- active-grasp save/restore equivalence;
- accidental dragging and entanglement rate.

### Success criteria

At least some lineages should transport bodies farther and more selectively than collision-only controls, adjust behavior to mass/load, and preserve grasp behavior after exact restore.

### Expected failure modes

- grasp oscillation every tick;
- controllers always choose maximum grip regardless of cost;
- bodies become effectively welded to organisms;
- large bodies clip or destabilize locomotion;
- organisms exploit constraint impulses for free propulsion;
- contested grasp creates stable ID-based dominance;
- carried bodies are visually tracked through hidden global IDs.

### Compute cost

**M.** Persistent constraints and extra contacts increase mechanics cost. Keep one effector and one grasp slot initially.

## 15.4 Experiment 3 — object choice by physical property

### Research question

Do organisms choose bodies because of useful physical properties rather than identity, location, or cosmetic cue?

### World configuration

Create repeated choices among bodies varying factorially in mass, size, hardness, shape, and distance. The downstream ecological effect should vary smoothly, for example:

- heavier bodies move a pressure plate-like physical barrier farther but cost more to transport;
- harder bodies cause more target wear;
- rod-like bodies reach deeper but are harder to carry;
- larger bodies block wider gaps.

Avoid an apparatus with one exact correct object.

### Controls

1. **Property homogenization:** all options have equal relevant mechanics but retain visual differences.
2. **Location randomization:** resample positions independently each episode.
3. **Cue permutation:** decorrelate color/texture from mechanics.
4. **Identity renewal:** spawn novel body IDs every episode.
5. **Utility reversal:** alter the target so the formerly useful property becomes costly or irrelevant.
6. **No-choice condition:** provide one body to separate competence from selection.

### Ablations

- hide apparent size;
- remove load feedback;
- remove tactile hardness/damage feedback;
- eliminate long-term memory;
- quantize properties more coarsely;
- reduce action exploration noise.

### Metrics

- conditional choice probability as a function of physical properties;
- regression or generalized linear mixed-model coefficients controlling for location, cue, and ID;
- conditional mutual information between chosen property and outcome;
- transfer to novel identities and property combinations;
- interpolation rather than only endpoint performance;
- speed of reversal when utility changes;
- causal substitution effect in branch replays.

### Success criteria

Choice should track a physically relevant property after controlling for shortcuts and should reverse or generalize when environmental requirements change.

### Expected failure modes

- location or appearance shortcut;
- exact threshold induced by apparatus geometry acts as a recipe;
- all options are functionally equivalent because force model is too coarse;
- one high-property object dominates despite transport cost;
- controllers cannot integrate property cues with future utility;
- selection acts on an innate material cue rather than learned sensorimotor relation.

### Compute cost

**M.** Most cost comes from replication and factorial controls rather than per-tick mechanics.

## 15.5 Experiment 4 — object-assisted foraging

### Research question

Can organisms use external bodies to gain access to nutritional matter that is otherwise costly or inaccessible?

### World configuration

Use a family of procedurally varied physical access problems, such as:

- nutrient matter behind a breakable crust whose toughness varies;
- nutrient body in a recess deeper than direct reach;
- heavy cover that can be levered, pushed, or struck;
- hazardous substrate crossed by placing supports;
- buried matter accessible through excavation.

Each problem should permit multiple broad strategies and have direct-access variants.

### Controls

1. **No-object control.**
2. **Direct-access control:** remove the barrier/recess to verify motivation and baseline foraging.
3. **Property-matched substitutes:** vary one property at a time.
4. **Body-only control:** allow organisms to strike/push with their own bodies.
5. **Pre-positioned-effect control:** place the target in the post-tool state without an object, measuring the value of the outcome separately from manipulation skill.
6. **Random-object control:** provide irrelevant bodies with similar salience.

### Ablations

- disable grasp but retain pushing;
- disable force concentration/edge effects;
- disable damage accumulation;
- remove object persistence between attempts;
- remove tactile damage feedback;
- increase/decrease carrying costs.

### Metrics

- probability and latency of access;
- net energetic return after manipulation cost;
- target-state change attributable to object-mediated contact;
- deterministic branch effect of removing/substituting the object;
- strategy diversity across lineages and environments;
- transfer to novel barrier properties and layouts;
- number and depth of action episodes before access;
- use of body-only alternatives.

### Success criteria

A candidate tool episode must show controlled external-object involvement and causal improvement over no-object and substitute controls. Evolution of a body-only strategy is a valid scientific outcome but should not be mislabeled.

### Expected failure modes

- one object shape becomes a de facto key;
- task reward is so steep that population collapses before innovation;
- direct body ramming solves every case;
- physics permits clipping or impulse amplification;
- organisms learn to wait for spontaneous fracture;
- target cue, not material response, drives behavior;
- apparatus is too puzzle-like and does not generalize.

### Compute cost

**M–H.** Damage, repeated contacts, and counterfactual branch replays dominate.

## 15.6 Experiment 5 — repeated reuse

### Research question

Do organisms retain, retrieve, and repeatedly use the same artifact across separated episodes?

### World configuration

Make useful bodies persistent and moderately scarce or costly to locate. Separate acquisition sites from repeated use sites. Include wear and replacement tradeoffs. Ensure carrying indefinitely has a cost so caching or retrieval can be selected.

### Controls

1. **Respawning-equivalent control:** a fresh equivalent body appears at each use site.
2. **Identity-scramble analytics control:** preserve mechanics but remove persistent tracking information from the organism.
3. **Artifact relocation:** move the artifact after storage to test memory versus local cue following.
4. **Abundant-object control:** equivalent bodies are common.
5. **No-wear control:** assess whether durable retention depends on degradation.
6. **World-reset control:** remove persistent objects between episodes.

### Ablations

- remove recurrent/lifetime memory;
- remove stable perceptual tracking tokens;
- remove carrying cost;
- remove wear;
- remove spatial cues;
- shorten object lifetime.

### Metrics

- same-ID use episodes;
- inter-use interval and travel distance;
- retrieval success after delay;
- proportion of uses involving previously controlled bodies;
- artifact age and lifetime net benefit;
- maintenance or protective behavior;
- abandonment threshold as wear increases;
- memory-ablation effect;
- branched replay effect of relocating or removing the artifact.

### Success criteria

Reuse should involve the same stable physical ID across distinct episodes and should exceed what local abundance or site fidelity predicts.

### Expected failure modes

- camping at one use site is mistaken for retrieval;
- organism carries the body continuously, avoiding delayed memory;
- a persistent visual beacon reveals location perfectly;
- object is immortal and costless, eliminating tradeoffs;
- tracking token acts as a global object address;
- reuse reflects scarcity only, not learned utility.

### Compute cost

**M.** Long behavioral horizons and identity analytics matter more than new mechanics.

## 15.7 Experiment 6 — placement and caching

### Research question

Can organisms place matter for later use, choose cache locations, and retrieve contents after a delay under ecological tradeoffs?

### World configuration

Nutritional matter occurs in pulses. Carrying has cost; unconsumed matter may decay; other organisms may take exposed matter; terrain provides varying visibility, distance, and access. Caches must be physical placements, not inventories.

### Controls

1. **No-persistence control:** placed bodies vanish or reset after a short interval.
2. **Random-relocation control:** cached bodies are moved to matched locations.
3. **No-delay control:** immediate consumption is always optimal.
4. **No-theft control:** compare private versus competitive ecology without ownership rules.
5. **Prebuilt-cache control:** useful cavities exist but organisms do not have to choose placement.
6. **Sham-cache control:** visually similar inert bodies occupy cache locations.

### Ablations

- remove spatial memory;
- remove object identity tracking;
- disable controlled release, allowing only drop;
- eliminate decay;
- eliminate carrying cost;
- remove occlusion or cavities;
- reset terrain marks.

### Metrics

- fraction of acquired matter stored rather than consumed;
- delay to retrieval;
- retrieval precision and route efficiency;
- cache-site selectivity relative to visibility, distance, and theft risk;
- retained nutritional value;
- theft, sharing, and defense rates;
- same-ID recovery;
- counterfactual effect of cache removal/relocation;
- external-marker dependence;
- reproductive or survival benefit after full energy accounting.

### Success criteria

Strong caching requires deferred placement and later retrieval with a measurable benefit, not accidental dropping near a home location.

### Expected failure modes

- piles form at locomotor bottlenecks accidentally;
- organisms cannot distinguish their caches from natural concentrations;
- decay or theft makes caching never adaptive;
- perfect internal coordinates make environmental markers irrelevant;
- caches become inaccessible because collision/containment is too strict;
- supernatural ownership accidentally prevents contest.

### Compute cost

**M–H.** Persistent object density and longer ecological runs increase state and analytics cost.

## 15.8 Experiment 7 — persistent barriers or structures

### Research question

Can organisms create and maintain persistent arrangements that alter traversability, exposure, access, or resource flow?

### World configuration

Provide movable bodies and mutable terrain in environments where barriers, supports, pits, or cover can produce graded ecological effects. Examples:

- reducing encounters with a mobile hazard;
- channeling prey or competitors;
- decreasing exposure to a directional environmental field;
- creating a stable route across costly terrain;
- retaining stored matter.

Do not score structure shape. Consequences operate through collision, support, and fields.

### Controls

1. **Placement-disabled control.**
2. **Persistence-reset control:** erase modifications at controlled intervals.
3. **Random-prebuilt structures:** separate ability to exploit a structure from ability to create it.
4. **Geometry-preserving sham:** retain visible arrangement but remove collision/support or field effect.
5. **Function-preserving alternate geometry:** test whether one intended shape is overfit.
6. **No-hazard/no-gradient control:** structure provides no ecological benefit.

### Ablations

- disable bonds while retaining stacking;
- disable terrain mutation;
- disable height/occlusion;
- remove support collapse;
- remove wear and maintenance need;
- remove social observation;
- reduce object supply.

### Metrics

- persistence time of modified configuration;
- directed placement/removal count;
- region connectivity and shortest-path cost change;
- permeability to organisms or other bodies;
- exposure reduction;
- retained matter and escape probability;
- maintenance after perturbation;
- builder and non-builder use;
- causal outcome under structure removal or sham physics;
- structural diversity and convergence across lineages;
- material and energetic construction cost.

### Success criteria

A structure classification requires persistent configuration plus measured function. A visually regular pile with no causal effect is not sufficient.

### Expected failure modes

- world jamming through indiscriminate piles;
- complete resource exclusion causes ecological collapse;
- one predesigned corridor makes a wall obvious;
- structures are stable only because collision sleep freezes impossible arrangements;
- agents exploit solver penetration or spawn blocking;
- automatic pathfinding ignores physical barriers;
- observer shape heuristic creates false positives.

### Compute cost

**H.** Body density, support graphs, persistent contacts, dirty terrain chunks, and branch interventions all rise.

## 15.9 Experiment 8 — stigmergic coordination

### Research question

Can organisms coordinate through persistent environmental traces rather than direct centralized control or explicit messages?

### World configuration

Use a collective problem whose efficiency improves when earlier actions alter later local action probabilities. Candidate settings:

- repeated transport along routes where traffic compacts terrain;
- excavation where depressions or loose-material piles guide further work;
- distributed resource collection using a generic deposited signal;
- barrier maintenance where damaged regions create local cues;
- aggregation at multiple possible sites.

### Controls

1. **Fast-decay trace:** trace vanishes before later agents can use it.
2. **Trace-removal intervention:** erase selected traces while preserving other state.
3. **Trace-shuffle:** relocate or permute trace values while preserving distribution.
4. **Direct-observation control:** agents can see prior actors but not traces, and vice versa.
5. **Solitary control:** one organism operates alone.
6. **Common-attractor control:** environmental attractors remain but organism-created traces are disabled.
7. **Explicit-communication comparison:** optional separate channel quantifies whether coordination is truly environmental.

### Ablations

- remove trace perception;
- remove trace deposition;
- alter decay rate;
- remove physical consequence while preserving perceptual cue;
- remove perceptual cue while retaining physical consequence;
- disable memory;
- prevent reuse of modified sites.

### Metrics

- conditional dependence of action on local prior trace after controlling for exogenous environment;
- collective throughput and travel cost;
- spatial concentration and route entropy;
- time to recover after trace perturbation;
- information flow or transfer-entropy-like measures from prior action → trace → later action;
- sematectonic versus deposited-field contribution;
- scaling with group size;
- robustness to agent turnover;
- false positive rate under trace shuffling.

### Success criteria

Coordination should degrade when the relevant environmental trace is removed or shuffled, while direct observation and common attraction controls remain matched.

### Expected failure modes

- agents independently follow the same resource gradient;
- direct imitation is mistaken for stigmergy;
- trace field is effectively a designer-provided waypoint map;
- positive feedback saturates the world;
- old traces never decay and lock behavior permanently;
- trace update cadence introduces delayed nondeterminism;
- spatial autocorrelation inflates information measures.

### Compute cost

**H.** Field updates, social exposure logs, perturbation branches, and group-level analysis dominate.

## 15.10 Experiment 9 — composite objects

### Research question

Can organisms create, manipulate, and exploit bonded or supported assemblies whose function exceeds that of isolated components?

### World configuration

Introduce broad bond formation with simple rods, plates, wedges, and masses. Create ecological gradients where reach, contact area, mass distribution, or rigidity can matter, but preserve alternate solutions.

Possible opportunities:

- extending reach into a recess;
- increasing impact mass while retaining a graspable component;
- creating a wider barrier from smaller pieces;
- making a stable support from complementary shapes;
- retaining matter through an enclosure.

### Controls

1. **No-bond control:** components remain physically identical but cannot attach.
2. **Weak-bond control:** assemblies form but fail under expected load.
3. **Component-only control:** provide each part separately at use time.
4. **Random-assembly control:** generate matched component counts with random geometry.
5. **Preassembled control:** separate assembly learning from use competence.
6. **Compatibility perturbation:** smoothly vary geometry and adhesion.
7. **Degree-cap variants:** test whether useful behavior depends on large graphs.

### Ablations

- remove bond feedback;
- remove orientation perception;
- remove action persistence;
- remove reuse/persistence of assemblies;
- simplify or eliminate compound collision proxy;
- remove wear/bond degradation;
- disable disassembly.

### Metrics

- assembly frequency and success;
- connected-component size and bond graph structure;
- assembly duration and reuse;
- whole-assembly causal benefit minus best component benefit;
- alignment error at formation;
- number of modification/assembly steps;
- recursive or hierarchical assembly evidence;
- disassembly/reconfiguration rate;
- state growth per successful function;
- transfer to novel component identities and property ranges.

### Success criteria

A composite tool or structure requires a configuration-level benefit demonstrated by component and random-assembly controls. Two bodies stuck together accidentally are not sufficient.

### Expected failure modes

- exact connector geometry forms a hidden recipe;
- bonds are too strong and produce irreversible clutter;
- bonds are too weak to survive use;
- controllers cannot align parts;
- constraint islands explode solver cost;
- assemblies exploit mass or energy duplication bugs;
- arbitrary overlap creates impossible compound geometry;
- preassembled bodies are used but construction never evolves.

### Compute cost

**H–VH.** Bond constraints, contact complexity, object proliferation, and long evolutionary search dominate.

## 15.11 Experiment 10 — social transmission of object use

### Research question

Does observing another organism cause acquisition of an object-use variant beyond genetic similarity, local enhancement, or shared environmental discovery?

### World configuration

Use populations with demonstrators and naïve observers. Candidate variants should be mechanically equivalent or near-equivalent where possible, such as:

- two different object-property choices that solve the same access problem;
- two manipulation directions or action sequences;
- two cache-marker conventions;
- two assembly geometries with similar performance.

Ensure observers can perceive actor, object, contact, and outcome at controlled resolution.

### Controls

1. **No-demonstration control.**
2. **View-blocked control:** demonstrator acts but observer cannot see the critical relation.
3. **Ghost control:** objects move through the same trajectory without an acting organism.
4. **Yoked demonstrator control:** actor is visible but action-outcome contingency is broken.
5. **Local-enhancement control:** observer sees the location or object after demonstration but not action.
6. **Stimulus-enhancement control:** object salience is increased without showing use.
7. **Genetic clone/common-garden control.**
8. **Cross-fostering:** observers experience another lineage’s demonstrators and environment.
9. **Outcome-only control:** observer sees the result but not the action.

### Ablations

- remove actor identity cues;
- remove target-object relation from perception;
- remove observer lifetime learning/plasticity;
- remove demonstrator visibility after first contact;
- reduce temporal resolution;
- remove social attention mechanism if one exists;
- randomize demonstrator success.

### Metrics

- adoption hazard after exposure;
- first-attempt object choice and action similarity;
- sequence alignment under time warping or event-grammar comparison;
- exposure-dose response;
- persistence and generalization of learned variant;
- social-network diffusion fit;
- difference between ghost/local-enhancement and full demonstration;
- demonstrator skill dependence;
- observer performance gain versus independent exploration;
- branched replay with demonstration removed at exact exposure tick.

### Success criteria

Strong evidence requires a demonstrator-specific causal effect beyond attraction to location or object, with appropriate genetic and ecological controls. Exact imitation is not required; social facilitation, emulation, and result copying should be classified separately.

### Expected failure modes

- observer simply approaches the demonstrator’s location;
- inherited controller bias produces the same variant;
- both agents respond to an unlogged environmental cue;
- observation representation directly transmits action labels;
- ghost controls are physically distinguishable in unintended ways;
- demonstrator consumes or modifies the opportunity before observer testing;
- selection favors innate conformity rather than lifetime learning.

### Compute cost

**H.** Controlled exposure design, observer logs, branch replay, and many social-network replicates dominate.

## 15.12 Experiment 11 — environmental inheritance

### Research question

Do organism-created artifacts or terrain modifications alter the development, behavior, or fitness of later generations independently of genetic inheritance?

### World configuration

Allow parents or prior cohorts to modify the world through caches, paths, shelters, barriers, or artifacts. Offspring encounter those modifications after creators are absent or behaviorally inactive.

### Controls

1. **World-reset control:** restore exogenous terrain/resource state while removing organism-caused modifications.
2. **Artifact transplant:** move the parental artifact configuration into another lineage’s environment.
3. **Cross-fostering:** offspring genotype develops in another lineage’s modified world.
4. **Geometry-preserving sham:** retain appearance but remove mechanical function.
5. **Resource-matched control:** equalize total resource quantity without preserving arrangement.
6. **Population-density control:** match number and spatial distribution of organisms.
7. **Creator-presence control:** compare inherited state with and without living demonstrators.
8. **Random-history control:** create matched amounts of environmental disturbance without organized lineage action.

### Ablations

- disable world persistence across generations;
- remove artifact perception;
- remove artifact use actions;
- remove social observation while retaining artifacts;
- shorten artifact lifetime;
- remove terrain compaction/damage inheritance;
- scramble provenance labels in analytics only.

### Metrics

- offspring survival, energy, reproductive timing, and learning speed;
- artifact use and modification rates;
- development trajectory differences;
- genotype × inherited-environment interaction;
- causal effect of artifact transplantation/reset;
- persistence across number of generations after creator removal;
- distribution of benefits and harms across kin/non-kin;
- selection-gradient changes in modified versus reset environments;
- descendant dependence on particular artifact lineages.

### Success criteria

Environmental inheritance requires a later-generation effect caused by persistent organism-modified state. Merely inheriting a resource-rich location or higher population density must be ruled out through matched controls.

### Expected failure modes

- survivor bias: only successful parents leave both genes and artifacts;
- resource quantity, not configuration, explains benefit;
- offspring copy living adults rather than use inherited artifacts;
- world reset changes unrelated ecological variables;
- artifact transplant alters spatial context;
- inherited barriers harm outsiders and destabilize population;
- persistence is too short relative to generation time.

### Compute cost

**VH.** Multi-generation matched lineages, state transplantation, and long-horizon branch replays are expensive.

## 15.13 Experiment 12 — cumulative modification

### Research question

Can artifact or environmental modifications accumulate across social generations so that later configurations outperform or enable behavior beyond isolated individual rediscovery?

### World configuration

Use open families of ecological challenges whose demands vary continuously rather than a finite sequence of locks. Candidate pressures include:

- variable reach depths;
- shifting barrier properties;
- transport across changing gaps;
- shelter under fluctuating directional exposure;
- resource distributions favoring increasingly efficient storage or paths.

Artifacts persist with wear, maintenance, and reconfiguration. New individuals can observe use and modification but receive no technology labels.

### Controls

1. **Artifact-reset control:** each cohort starts with pristine matter.
2. **Social-isolation control:** artifacts persist but observation of previous users is removed.
3. **Modification-disabled control:** artifacts can be reused but not changed.
4. **Fresh-individual reconstruction control:** isolated individuals receive raw materials and equivalent time.
5. **Lineage-mixing control:** artifacts and learners are randomly reassigned among populations.
6. **Cultural bottleneck:** reduce number of demonstrators or transmission opportunities.
7. **Artifact-only inheritance:** retain artifact, remove demonstrations.
8. **Demonstration-only inheritance:** show behavior, reset artifact.
9. **Wear-matched sham:** apply random changes matching magnitude but not historical direction.
10. **Performance-blind environment:** remove ecological advantage while preserving manipulation opportunities.

### Ablations

- remove stable provenance;
- remove lifetime learning;
- reduce observation fidelity;
- increase/decrease artifact durability;
- cap assembly depth;
- remove metatool dependencies;
- eliminate population turnover;
- reduce ecological variability;
- remove maintenance costs.

### Metrics

- best and median functional performance by cultural generation;
- capability envelope: range of physical challenges solved;
- artifact-lineage modification depth;
- number of retained, reverted, and novel edits;
- assembly graph depth and dependency DAG depth;
- social-transmission fidelity;
- independent rediscovery probability;
- performance after bottleneck or isolated reconstruction;
- culture-loss and recovery dynamics;
- ratio of inherited modification benefit to raw-material baseline;
- structural complexity adjusted for number of parts and state cost;
- robustness across held-out environments;
- maker/user specialization and division of labor, if it appears;
- whether later behavior depends causally on earlier lineage states.

### Success criteria

Evidence for cumulative modification should require all of the following:

1. successive lineage-linked changes;
2. retained functional gains or expanded capability;
3. dependence on social or environmental inheritance;
4. performance difficult for isolated individuals to reconstruct within matched time;
5. persistence through at least one population turnover;
6. robustness to held-out physical conditions;
7. no engine-recognized stage, recipe, or direct construction reward.

### Expected failure modes

- one innovator creates a durable artifact that later agents merely exploit;
- random wear is mistaken for cumulative refinement;
- a fixed benchmark has a low ceiling and is solved once;
- artifacts accrete parts without functional improvement;
- improvements come from genetic evolution only;
- perfect copying eliminates exploration, or poor copying destroys retention;
- population bottlenecks erase variants faster than selection can act;
- observer analytics choose a complexity measure that rewards useless elaboration;
- hidden curriculum orders environments into a technology ladder;
- state and compute growth become unbounded.

### Compute cost

**VH.** This is the culmination of the program and should not be attempted until lower-level mechanics, causal analytics, and state growth are stable.

## 15.14 Experiment dependency graph

```text
Object perception
    ↓
Carrying
    ↓
Property-selective choice
    ↓
Object-assisted foraging
    ↓
Repeated reuse ───────→ Social observation prerequisites
    ↓                              ↓
Placement and caching        Social transmission
    ↓                              ↓
Persistent structures       Traditions (analytic milestone)
    ↓                              ↓
Stigmergic coordination ──────────┤
    ↓                              ↓
Composite objects ─────────→ Environmental inheritance
    └───────────────→ Metatool dependencies
                                   ↓
                         Cumulative modification
```

The graph is not a claim that biological evolution follows this sequence. It is an experimental dependency structure for avoiding invalid inference.

## 15.15 Stop/go gates

Before advancing, require engineering and scientific gates.

### Gate A — mechanics and persistence

- exact save/restore under active manipulation;
- zero unexplained matter drift;
- bounded penetration and constraint error;
- stable object IDs and provenance;
- single-thread/parallel tick-hash equivalence.

### Gate B — behavioral accessibility

- at least one controller family can perceive, grasp, transport, and release without semantic action labels;
- performance generalizes beyond one object ID/layout;
- energy costs prevent maximum-force trivial policies.

### Gate C — causal classification

- automated branch interventions can remove/substitute artifacts;
- false-positive rate is characterized on accidental-object controls;
- observer classifications do not affect authoritative hashes.

### Gate D — state scalability

- object/fragment counts remain bounded under stress;
- snapshot growth and compaction are measured;
- contact and bond graphs meet throughput targets;
- long-run worlds do not accumulate unrecoverable journal debt.

### Gate E — social inference

- observation opportunities are logged;
- local enhancement, ghost, genetic, and environmental controls are implemented;
- social metrics recover known synthetic ground truth before use on evolved populations.

---

# 16. Metrics and ablations

## 16.1 Metric taxonomy

The engine should avoid one scalar “technology score.” Use a multidimensional measurement system with physical, behavioral, ecological, social, evolutionary, and computational domains.

## 16.2 Physical-integrity metrics

| Metric | Definition | Purpose |
|---|---|---|
| **Matter balance error** | serialized material quantity at `t+1` minus prior quantity plus modeled sources/sinks | Detect deletion/duplication through fracture, transfer, aggregation, or migration |
| **Energy-accounting residual** | if energy is modeled, input minus output/storage/dissipation | Detect impulse, reaction, or carrying exploits |
| **Maximum penetration** | largest post-solve collider overlap at tick boundary | Mechanics stability |
| **Constraint residual** | grasp/bond/rest-transform error after fixed solver iterations | Detect unstable composites |
| **Support overload residual** | unaccounted load or cyclic support anomaly | Validate 2.5D structure mechanics |
| **Terrain delta balance** | removed terrain quantity versus spawned/deposited quantity | Validate digging/deposition |
| **ID integrity** | duplicates, reuse, dangling references, allocator monotonicity | Persistence correctness |
| **Canonical replay divergence tick** | first tick at which root hashes differ | Localize determinism failures |

These are engineering metrics, not organism-fitness signals.

## 16.3 Artifact-ecology metrics

- resolved body count;
- sleeping body count;
- aggregate/pile count;
- body creation and destruction rates;
- fragment-size distribution;
- artifact age distribution;
- use episodes per artifact;
- fraction of bodies ever controlled;
- abandoned-artifact density;
- active bond count and degree distribution;
- assembly connected-component size;
- dirty terrain chunk count;
- modified-cell fraction;
- provenance depth;
- matter represented at resolved versus aggregate scale;
- artifact spatial clustering;
- artifact turnover and half-life;
- builder/user identity overlap.

These reveal whether the world supports a meaningful artifact ecology or merely accumulates junk.

## 16.4 Behavioral metrics

### Manipulation competence

- grasp success conditioned on reach and geometry;
- time to stable grasp;
- control retention under load;
- transport displacement;
- controlled release accuracy;
- force targeting accuracy;
- energy per unit target-state change.

### Object choice

- selection conditional on physical property;
- generalization to novel IDs and cue permutations;
- property-outcome calibration;
- exploration versus exploitation;
- policy sensitivity to changed ecological requirements.

### Reuse and planning

- inter-episode same-ID reuse;
- retrieval after delay;
- travel to stored artifact;
- artifact retention versus wear;
- preparatory modification before target availability;
- deferred benefit and opportunity cost.

### Construction

- directed material displacement;
- arrangement persistence;
- maintenance response;
- function under perturbation;
- configuration diversity;
- energetic and material construction cost.

## 16.5 Causal artifact metrics

A robust observer should calculate causal effects, not only correlations.

### Artifact necessity

```text
necessity_score = P(outcome | baseline history)
                - P(outcome | candidate artifact removed)
```

For deterministic single episodes, probabilities arise over matched seeds/populations, while the branch difference is recorded per episode.

### Property contribution

Use substitutions that match all but one property. Estimate a dose-response curve rather than a binary necessary/not-necessary label.

### Configuration contribution

Compare:

- complete assembly;
- same components unbonded;
- random assembly;
- geometry-preserving weak-bond assembly;
- function-preserving alternate geometry.

### Historical contribution

Compare the current artifact with a reconstructed raw-material baseline and prior lineage revisions. This is essential for cumulative modification.

## 16.6 Structural-function metrics

A persistent configuration may be evaluated along independent dimensions:

- **barrier effect:** change in crossing rate or path cost;
- **shelter effect:** change in exposure to directional field/hazard;
- **storage effect:** retained matter over time and retrieval accessibility;
- **trap effect:** differential entry and exit probability;
- **path effect:** locomotor work/time reduction;
- **support effect:** load carried or traversable gap spanned;
- **signal effect:** receiver behavior change after controlling for underlying ecology;
- **memory effect:** delayed decision performance lost after marker shuffle;
- **infrastructure effect:** benefit distributed across multiple users or episodes;
- **territorial effect:** altered intruder behavior and defense interactions;
- **resource-renewal effect:** change in local production, depletion, or regrowth.

Do not combine these into a single structure score unless the weights are explicitly analysis-specific.

## 16.7 Stigmergy metrics

Stigmergy requires mediation through environmental state. Suggested analyses:

1. fit later action from exogenous environment and agent state;
2. add organism-created trace state;
3. measure incremental predictive information;
4. intervene by removing or shuffling trace;
5. test whether group performance changes;
6. verify that direct observation does not explain the effect.

Useful measures include:

- trace-conditioned action probability;
- lagged mutual information;
- transfer entropy or directed-information approximations, interpreted cautiously;
- route entropy;
- spatial autocorrelation adjusted against null models;
- perturbation recovery time;
- collective throughput scaling;
- trace lifetime relative to inter-agent arrival time;
- sematectonic versus explicit-field contribution.

Information-theoretic association alone is insufficient. Intervention provides the stronger evidence.

## 16.8 Social transmission metrics

- exposure count and duration;
- actor-target visibility quality;
- observer attention allocation;
- adoption hazard after exposure;
- demonstrator–observer action-sequence similarity;
- object-property choice convergence;
- diffusion path through the social network;
- between-group and within-group variance;
- persistence after demonstrator removal;
- performance gain from exposure;
- local/stimulus enhancement control effects;
- genotype, morphology, and ecological covariates;
- innovation, retention, and loss rates.

Network-based diffusion analyses are useful, but simulation permits stronger controlled interventions than observational field studies. Use them.

## 16.9 Environmental-inheritance metrics

- descendant outcome difference in modified versus reset world;
- artifact-transplant effect across genotypes;
- cross-foster effect;
- number of generations an effect persists;
- fraction of benefit mediated by structure, resources, social observation, or altered selection;
- genotype × environment interaction;
- inherited-state dependence of developmental learning speed;
- maker–beneficiary relatedness;
- public versus excludable benefit;
- ecological side effects on other species or lineages.

## 16.10 Cumulative-modification metrics

No single complexity measure is reliable. Use a panel:

- functional performance frontier across continuous challenge descriptors;
- artifact-lineage revision depth;
- retained versus reverted changes;
- component and bond count, adjusted for redundancy;
- causal dependency DAG depth;
- number of distinct material transformations;
- construction time and efficiency;
- robustness to held-out worlds;
- isolated rediscovery probability;
- transmission fidelity and error distribution;
- culture-loss under bottlenecks;
- recovery after partial artifact damage;
- division of labor and specialization;
- novelty relative to prior lineage configurations;
- description length of the physical configuration, used cautiously and never as fitness.

Increasing part count without improved function is elaboration, not cumulative culture. Increasing performance through genetic morphology alone is biological evolution, not cultural accumulation. Both may coexist and should be decomposed.

## 16.11 Core ablation library

The engine should implement reusable, declarative ablations that operate on branch checkpoints.

| Ablation | Removes or changes | Inference supported |
|---|---|---|
| **Semantic-cue ablation** | material/object IDs and observer labels from perceptions | Detects leaked labels |
| **Appearance permutation** | remaps superficial cues independently of mechanics | Detects cosmetic shortcuts |
| **Property homogenization** | equalizes one or more mechanical properties | Identifies property dependence |
| **Artifact removal** | deletes selected body/assembly at branch tick | Tests causal necessity |
| **Artifact substitution** | replaces with matched alternatives | Identifies causal properties |
| **Artifact relocation/orientation** | moves candidate while preserving state | Separates memory, location, and geometry effects |
| **Persistence reset** | restores terrain/objects to baseline | Measures environmental inheritance and structure function |
| **Object-identity masking** | removes stable perceptual tracking while preserving physics | Tests reuse/memory mechanisms |
| **Grasp ablation** | permits contact but no persistent control | Distinguishes carrying/tool control from pushing |
| **Force-band ablation** | reduces magnitude/direction resolution | Measures action-space requirements |
| **Bond ablation** | disables formation or weakens strength | Tests composite necessity |
| **Fracture ablation** | prevents damage-driven division | Tests shaping/access mechanisms |
| **Wear ablation** | removes degradation | Tests maintenance and artifact lifetime selection |
| **Carry-cost ablation** | removes or increases load cost | Tests transport/caching tradeoffs |
| **Internal-memory ablation** | removes/reduces recurrent or learned memory | Tests external memory and cue dependence |
| **Trace-deposition ablation** | prevents organism-created fields/marks | Tests stigmergy |
| **Trace-perception ablation** | retains trace mechanics but hides cue | Separates physical from informational effects |
| **Trace-shuffle** | preserves distribution but breaks history/location | Tests trace-specific information |
| **Social-visibility ablation** | blocks observation of actor-object events | Tests social acquisition |
| **Ghost/yoked demonstration** | preserves motion or salience without agency contingency | Separates imitation/emulation from enhancement |
| **Genetic common garden** | controls genotype/environment | Tests culture and ecological inheritance |
| **Artifact transplant** | moves structures across lineages/environments | Tests inherited environmental causation |
| **Deterministic tie-break swap** | changes keyed tie policy under controlled version | Detects arbitrary identity advantage |
| **Field-decay sweep** | varies persistence timescale | Identifies stigmergic operating regime |
| **Controller-bandwidth sweep** | changes percept/action slot counts | Measures evolutionary accessibility |

## 16.12 Null models

Offline analytics need null models that preserve obvious confounds:

- spatially constrained random walks matching locomotor speed;
- action-time shuffles preserving per-agent action frequencies;
- object-location permutations within equivalent accessibility bands;
- trace-field rotations/translations preserving autocorrelation;
- network rewiring preserving degree distribution;
- artifact-history shuffles preserving age and material;
- random assembly graphs preserving component count and degree;
- random modification lineages preserving edit magnitude;
- matched resource distributions without organized placement.

Null models should be deterministic under an analysis seed and versioned independently from simulation semantics.

## 16.13 Statistical design

Long-running evolutionary systems produce autocorrelation, lineage dependence, extinction censoring, and heavy-tailed innovation times. Avoid treating ticks or organisms as independent samples.

Recommended units:

- independent world/evolutionary replicate as the primary inferential unit;
- lineage or population as clustered units;
- event episodes nested within organism and world;
- held-out seeds for generalization;
- survival/time-to-event models for innovations;
- mixed models or hierarchical Bayesian models for property choice and social diffusion;
- permutation tests using whole-replicate or lineage blocks;
- effect sizes and uncertainty, not only significance.

Predefine how extinct runs and runs with no innovation are handled. Excluding them can severely bias estimates.

## 16.14 Synthetic validation of analytics

Before applying classifiers to evolved behavior, generate scripted **test fixtures** outside the evolutionary world whose ground truth is known:

- accidental object collision;
- carrying without target effect;
- object-assisted access;
- same-ID reuse;
- location shortcut;
- true property-selective substitution;
- accidental composite;
- functionally necessary composite;
- local enhancement;
- ghost imitation control;
- true trace-mediated coordination;
- inherited resource abundance without configuration effect;
- lineage-linked cumulative improvements;
- random wear with no improvement.

The scripts are tests of the observer, not behaviors available to organisms. Observer precision/recall and causal classification errors should be reported before claims about emergent tool use.

## 16.15 Compute and storage metrics

Log per tick or interval:

- ticks per second and wall time by phase;
- active/sleeping body counts;
- candidate and narrowphase collision pairs;
- contact manifolds and solver constraints;
- support edges and bond-island sizes;
- controller/perception cost;
- field cells updated;
- event bytes before/after compression;
- dirty chunks and snapshot bytes;
- incremental checkpoint reuse ratio;
- journal replay time;
- branch-replay storage and CPU;
- hash/checksum overhead;
- single-thread versus deterministic-parallel scaling;
- peak memory;
- compaction time;
- restore validation time.

Scientific experiments should include compute cost in their results. A behavior that requires an artifact density incompatible with long-run scale may not be a practical platform capability.

---
# 17. Performance and state-growth risks

## 17.1 Artifact and fragment explosion

### Risk

Every fracture, excavation, secretion, dropped resource, abandoned composite, and organism death can create persistent bodies. Even a low per-organism creation rate becomes unbounded in a long-running world.

### Consequences

- broadphase and observation cost rises;
- save size grows monotonically;
- object IDs and provenance logs grow;
- pathfinding and movement become cluttered;
- accidental world jamming becomes a dominant ecological pressure;
- analytics spend most time on irrelevant debris.

### Mitigations

- minimum resolved-body mass/area;
- deterministic fragment caps and templates;
- sleeping after a fixed low-energy interval;
- spatial aggregation into pile bodies;
- conversion of microdebris to conserved terrain rubble;
- biodegradation/weathering or erosion only if ecologically justified and mass-accounted;
- bounded secretion and excavation rates;
- object-count stress tests before bonds are enabled;
- prohibit camera- or load-dependent deletion.

### Scientific caution

Aggressive cleanup can erase ecological inheritance. Aggregation must preserve mechanically relevant quantity and coarse provenance, and must be reversible enough for later excavation where required.

## 17.2 Contact-pair explosion

### Risk

Dense piles and structures create many broadphase pairs and contact manifolds. Worst-case pair count is quadratic in local body density.

### Mitigations

- uniform spatial buckets sized to collider scale;
- sorted, deduplicated pair generation;
- sleeping/static classification;
- compound broadphase proxies for stable assemblies;
- terrain aggregation for tiny bodies;
- cap resolved bodies per cell through deterministic scale transition;
- collision layers based on physical representation, not semantic type;
- profile pathological piles and narrow corridors.

## 17.3 Constraint-graph blowup

### Risk

Grasps, bonds, support edges, hinges, and flexible constraints can form large connected islands. Fixed-iteration solving cost grows with constraints, and stiff systems may become unstable.

### Mitigations

- begin with rigid bonds only;
- bound bond degree and local anchor capacity;
- restrict simultaneous grasps per body initially;
- deterministic island decomposition;
- compound proxies for sleeping rigid assemblies;
- coarse support rather than general 3D contact stacking;
- no ropes, cloth, or large flexible networks in early phases;
- monitor largest island, degree distribution, and solver residual.

## 17.4 Dirty-chunk and save growth

### Risk

A single persistent cell modification can force a procedural chunk to be stored forever. Fields and traffic may dirty nearly every visited chunk.

### Mitigations

- choose chunk sizes based on mutation locality, not rendering convenience;
- store canonical deltas when smaller than materialized chunks;
- periodically compare a chunk with its exact generated baseline and permit a deterministic return to `Pristine` only if byte-equivalent and no pending state references it;
- use content-addressed blob reuse;
- separate low-resolution fields from terrain resolution;
- compact high-frequency cell history into authoritative current state;
- quantify dirty-chunk growth under random-walk stress.

### Caveat

Approximate “close enough to pristine” compaction is not acceptable for authoritative state unless the approximation itself is an explicit semantic transition.

## 17.5 Event-journal growth

### Risk

Contacts, perception opportunities, and object movements can produce far more event bytes than state bytes.

### Mitigations

- distinguish restore journal from analytics archive;
- aggregate microcontacts into canonical per-pair/per-tick summaries when full contact order is unnecessary;
- record intents and resolved outcomes rather than every solver iteration;
- use columnar or block compression for analytics;
- checkpoint frequently enough to bound restore replay;
- retain causal source IDs for transformations;
- sample only non-authoritative diagnostics, never authoritative restore events;
- define retention tiers for full, summarized, and lineage-critical history.

## 17.6 Field-update cost

### Risk

Diffusion or advection over large maps can dominate tick cost even when agents are sparse.

### Mitigations

- coarser field grids;
- fixed lower update cadence;
- active-chunk fronts determined solely by nonzero state and deterministic margins;
- integer stencil kernels;
- exact sparse representation where zero is absorbing;
- avoid many independent signal channels;
- cap field lifetime through decay where scientifically appropriate;
- store source accumulators canonically between updates.

## 17.7 Observation bandwidth

### Risk

Thousands of agents observing many bodies and each other can produce a query and neural-input bottleneck. Rich perception may also make evolution harder.

### Mitigations

- local spatial queries;
- deterministic salience cap;
- egocentric quantized features rather than raw global state;
- temporal tracking only for a small active set;
- multi-rate perception for distant versus contact cues, with fixed schedules;
- cache visibility only when cache results cannot affect semantics;
- log observation opportunities compactly;
- ablate percept slots to estimate marginal value.

### Scientific caution

A salience rule is an authored attention prior. It should favor physically general cues such as proximity, motion, size, novelty, and current contact—not “tool relevance.”

## 17.8 Action-space burden

### Risk

Target selection, direction, force, duration, effector, and contact region create a combinatorial action space. Evolution may fail before discovering useful manipulation.

### Mitigations

- quantized parameters;
- one effector initially;
- action persistence;
- hierarchical or modular controller outputs;
- lifetime learning/plasticity;
- staged feature gates;
- morphology-scaled action limits;
- ecological gradients where partial manipulation has partial benefit;
- avoid sparse binary success conditions;
- benchmark scripted test controllers without making them available to evolution.

## 17.9 Determinism versus parallel performance

### Risk

Canonical sorting and fixed-order merges add cost. Parallel physics libraries may disable cross-platform determinism or use order-dependent SIMD reductions [R47](#r47).

### Mitigations

- parallel read-only sensing and controller evaluation;
- deterministic chunk/island partitions;
- thread-local delta buffers;
- radix sort or ordered merge on integer keys;
- fixed-point arithmetic;
- single-thread reference implementation;
- continuous hash comparison across thread counts;
- feature-gated acceleration only after semantic equivalence is proven.

## 17.10 Composite collision cost

### Risk

An assembly of `n` bodies interacting with another assembly of `m` bodies can generate many candidate contacts. Frequent assembly revision invalidates compound proxies.

### Mitigations

- member AABB hierarchy with deterministic construction;
- compound proxy only after stability interval;
- cache keyed by assembly revision hash;
- coarse broadphase, exact member narrowphase;
- cap component count in early experiments;
- promote huge stable assemblies to static collision islands while retaining members for provenance;
- measure proxy rebuild rate.

## 17.11 Provenance growth

### Risk

Full ancestry of fragmented, merged, aggregated, and re-resolved matter can become a massive directed acyclic graph.

### Mitigations

- retain immediate parents and creation event in authoritative state;
- use content-addressed immutable provenance records;
- summarize deep ancestry with a Merkle root and bounded material-origin histogram;
- move full event history to analytics archive;
- garbage-collect provenance only when no live state or retained archive references it;
- separate scientific lineage fidelity from every-grain identity.

## 17.12 Causal replay cost

### Risk

Artifact classification through branch interventions can multiply compute by many counterfactuals per candidate event.

### Mitigations

- first-pass observational filters with high recall;
- branch only high-value candidate episodes;
- checkpoint at candidate boundaries;
- share immutable checkpoint blobs;
- use common-random-number coupling;
- prioritize minimal interventions based on causal graph;
- run heavy analytics offline;
- estimate uncertainty from sampled candidates rather than every event;
- retain a deterministic intervention manifest.

## 17.13 Numerical range and fixed-point overflow

### Risk

Long worlds accumulate coordinates, quantities, impulses, damage, and tick counters. Silent wraparound is catastrophic.

### Mitigations

- formal range analysis;
- checked conversions;
- explicit saturation only where semantically acceptable;
- world-coordinate chunking to keep local fixed-point values bounded;
- wide accumulators for canonical reductions;
- property-based stress testing at extrema;
- save validation of every numerical range;
- invariant that no authoritative arithmetic uses language-default overflow behavior.

## 17.14 Ecological runaway

### Risk

Artifacts can create positive feedback: barriers monopolize resources, paths amplify one lineage’s access, caches buffer starvation indefinitely, or inherited structures cause permanent lock-in.

### Mitigations

- wear, maintenance, erosion, or disturbance at ecologically justified rates;
- finite material and energetic costs;
- multiple spatial routes and resource sources;
- hazards and exogenous variation;
- no invulnerable structures;
- monitor concentration of artifact benefits;
- analyze collapse as a valid outcome rather than automatically rescuing populations;
- use controls to distinguish open-ended niche construction from one absorbing exploit.

## 17.15 Suggested benchmark envelope

The following are **targets for profiling, not promises**:

- 2,000–10,000 organisms;
- 5–20 resolved artifacts per organism on average in mature worlds;
- a much larger quantity of matter represented in sleeping aggregates or terrain;
- object-contact density bounded so that the 99th-percentile local bucket remains tractable;
- full authoritative checkpoint restore in operationally acceptable time;
- incremental checkpoints dominated by genuinely changed chunks;
- deterministic parallel speedup measured against a single-thread reference;
- branch interventions affordable for a sampled subset of candidate artifact episodes.

The architecture should be benchmarked at several adversarial distributions, not only uniform random placement:

- one enormous dense pile;
- long bonded chain;
- high-degree bond hub;
- crowded narrow corridor;
- simultaneous contested resource access;
- widespread low-level terrain modification;
- field active across the whole world;
- repeated fracture and aggregation cycles;
- save during maximum active constraints.

---

# 18. Anti-patterns that would script progress

## 18.1 Semantic runtime classes with privileged mechanics

**Anti-pattern:** `Tool`, `Weapon`, `Building`, `Road`, `Storage`, `Resource`, `CraftingMaterial`, or `Technology` components that change mechanics or fitness.

**Why it fails:** function is decided before behavior. Ordinary bodies cannot acquire novel roles without type conversion, and organisms may receive leaked semantic cues.

**Replacement:** physical bodies and observer-derived functional classifications.

## 18.2 Fixed crafting recipes

**Anti-pattern:** named input combinations produce named outputs.

```text
2 STONE + 1 STICK -> AXE
```

**Why it fails:** it defines a graph of intended technologies. Search discovers recipes rather than physical procedures.

**Replacement:** local geometry, force, material transfer, damage, and broad bond/reaction equations.

## 18.3 Recipes hidden as compatibility matrices

**Anti-pattern:** opaque material-pair or connector-pair tables.

```text
FLINT can_attach_to WOOD
HANDLE_SOCKET accepts HEAD_TYPE_A
```

**Why it fails:** the recipe graph remains, only renamed.

**Replacement:** compatibility from surface geometry, pressure, contact area, adhesion features, and smooth strength relations.

## 18.4 Exact action-sequence recognizers

**Anti-pattern:** if the organism strikes three times, scrapes twice, and heats, spawn an object.

**Why it fails:** physics is bypassed by symbolic sequence matching.

**Replacement:** every action changes physical state incrementally; learned procedures exist only in controller behavior.

## 18.5 High-level manipulation verbs

**Anti-pattern:** `BUILD`, `CRAFT`, `USE_TOOL`, `STORE`, `GIVE`, `CUT`, `DIG`, or `COMBINE` directly produces the interpreted result.

**Why it fails:** semantic success is guaranteed, motor control and material differences disappear, and the engine recognizes the designer’s ontology.

**Replacement:** grasp, release, force, matter transfer, locomotion, and generic signaling.

## 18.6 Inventory abstraction that erases embodiment

**Anti-pattern:** picked-up objects disappear from space, have no mass or volume, and can be placed anywhere in range.

**Why it fails:** carrying cost, collision, visibility, theft, transfer, and tool-body geometry vanish.

**Replacement:** persistent grasped bodies with load, pose, and collision. A temporary simplified attachment is acceptable only if it preserves the relevant physical consequences.

## 18.7 Requirement tags

**Anti-pattern:** a barrier has `requires_pickaxe = true` or a resource has `required_tool_level = 3`.

**Why it fails:** named capability gates create a technology ladder.

**Replacement:** toughness, geometry, access depth, and force/edge relations that permit graded alternate solutions.

## 18.8 Technology levels, eras, and unlocks

**Anti-pattern:** Bronze Age, construction skill, tool tier, research points, or lineage unlocks.

**Why it fails:** progress becomes a state machine authored by the engine.

**Replacement:** observer reports may describe historical periods after the fact, with no feedback to simulation.

## 18.9 Direct fitness rewards for tool use or construction

**Anti-pattern:** grant fitness whenever the observer detects a tool or structure.

**Why it fails:** agents optimize the detector and may create meaningless manipulations or clutter.

**Replacement:** fitness consequences arise through energy, risk, survival, reproduction, access, and ecology. Observer metrics remain noncausal.

## 18.10 One intended apparatus geometry

**Anti-pattern:** every experiment uses one puzzle box with one exact successful object.

**Why it fails:** evolution overfits identities and geometry; apparent cognition may be a narrow sensorimotor routine.

**Replacement:** procedurally vary physical descriptors and test held-out combinations and alternate routes.

## 18.11 Semantic perception

**Anti-pattern:** controllers receive `object_kind`, exact hardness, tool utility, owner, recipe membership, or “demonstrator used object.”

**Why it fails:** the key inferential problem is solved by the observation encoder.

**Replacement:** local cues, tactile feedback, observed motion/contact, and delayed outcome.

## 18.12 Supernatural ownership

**Anti-pattern:** only the owner can move, consume, or enter an artifact.

**Why it fails:** a social convention becomes an inviolable law and eliminates theft, contest, signaling, and enforcement.

**Replacement:** possession, defense, access geometry, marking, and learned social beliefs.

## 18.13 Blueprint or target-field construction

**Anti-pattern:** agents follow a gradient encoding the desired final structure or receive reward for matching a target bitmap.

**Why it fails:** decentralized execution does not make the target emergent.

**Replacement:** local physical consequences and ecology. Robotic construction research using specified targets remains useful for mechanism engineering but not as the Genesis objective [R21](#r21).

## 18.14 Observer labels fed back into the world

**Anti-pattern:** once analytics detects a path, its friction is reduced; once a body is labeled a tool, it becomes easier to use.

**Why it fails:** post-hoc interpretation becomes a self-fulfilling causal category.

**Replacement:** friction changes only through traffic/compaction; ease of use follows physical state.

## 18.15 Arbitrary object bonuses

**Anti-pattern:** a held object multiplies damage because it is “weapon-like,” independent of mass, hardness, velocity, or contact.

**Why it fails:** function is encoded semantically.

**Replacement:** impulse, pressure, edge acuity, target support, material response, and organism work.

## 18.16 Full 3D before scientific need

**Anti-pattern:** adopting general 3D rigid-body simulation because it appears more realistic.

**Why it fails:** it expands perception, control, numerical, performance, and determinism problems before basic hypotheses are tested.

**Replacement:** 2.5D first; add degrees of freedom when a blocked experiment justifies them.

## 18.17 Unbounded fracture and chemistry

**Anti-pattern:** every impact creates arbitrary shards; every material pair can react into new named materials.

**Why it fails:** state explodes and dynamics become opaque.

**Replacement:** bounded templates, aggregation hierarchy, small conservative reaction families, and state-space audits.

## 18.18 Nondeterministic container or thread order

**Anti-pattern:** “the seed is fixed, so replay is deterministic,” while iterating hash maps, consuming global RNG draws, or using race-dependent reductions.

**Why it fails:** update order silently changes outcomes.

**Replacement:** keyed random addresses, canonical total orders, fixed iterations, and thread-count hash tests.

## 18.19 Save fallback and silent reinterpretation

**Anti-pattern:** if a mutable section is missing, regenerate terrain or initialize empty objects; if a field is unknown, use a default.

**Why it fails:** corrupted or old saves become plausible but false histories.

**Replacement:** version dispatch, required-section manifest, schema hashes, exact migration, and fail-closed restore.

## 18.20 Camera-, frame-rate-, or load-dependent simulation

**Anti-pattern:** far objects simplify, debris disappears, fields skip updates, or collisions use lower quality when the server is busy.

**Why it fails:** observation conditions and hardware load alter authoritative history.

**Replacement:** deterministic state-based representation transitions and fixed update schedules.

## 18.21 Recycled stable IDs

**Anti-pattern:** destroyed object IDs are reused to save space.

**Why it fails:** artifact lineage, event references, and counterfactual analysis become ambiguous.

**Replacement:** monotonic never-reused IDs and bounded internal handles separate from external identity.

## 18.22 Scripted teaching

**Anti-pattern:** a `TEACH(tool_recipe, student)` action copies a behavior module.

**Why it fails:** social transmission is implemented rather than studied.

**Replacement:** observable action, attention, generic signaling, and lifetime learning. Candidate teaching can be classified offline only under demanding behavioral criteria.

## 18.23 Complexity for its own sake

**Anti-pattern:** adding conductivity, fire, fluids, magnetism, chemistry, and articulated machinery because future technology might need them.

**Why it fails:** each domain expands state and creates designer-salient solution paths before evidence that simpler mechanisms support open-ended behavior.

**Replacement:** dependency-ordered feature additions tied to falsifiable experiments.

---

# 19. Open questions

## 19.1 Is 2.5D sufficient?

A 2.5D support model likely covers carrying, barriers, caches, pits, basic shelter, stacking, and some bridges. It may fail for arbitrary interlocking, overhead manipulation, knots, flexible traps, and complex containers.

**Decision test:** implement the first seven experiments and catalog behaviors blocked specifically by missing vertical degrees of freedom rather than by controller or ecology. Upgrade only when the blocked class is scientifically important.

## 19.2 Should bond formation be passive or action-mediated?

Passive adhesion is physically clean but may be too difficult for evolving agents to control. A generic secretion/clamp action is accessible but may become a disguised `attach` verb.

**Experiment:** compare passive contact-duration bonding, pressure-triggered bonding, and resource-consuming generic clamp/secretion under identical component tasks. Measure discovery rate, solution diversity, and recipe-like narrowing.

## 19.3 What fixed-point precision is enough?

Too little precision causes jitter, dead zones, and coarse property thresholds. Too much precision increases range/overflow burden without behavioral benefit.

**Benchmark:** sweep position, velocity, angle, and coefficient precision on adversarial contacts and evolved policies. Select the lowest profile preserving qualitative invariants and ranking of physical alternatives.

## 19.4 What perception representation best balances grounding and accessibility?

Raw pixels are expensive; structured slots can leak objectness and impose a salience prior.

**Candidates:** egocentric body slots, radial occupancy/material-cue bins, learned visual embeddings from deterministic rasterization, and hybrid contact/object channels.

**Required control:** verify transfer under cue permutation and novel object identities.

## 19.5 How many environmental fields are justified?

Fields can support shelter, odor, signaling, moisture, and thermal niche construction, but each adds state and possible shortcuts.

**Recommendation:** require a named experiment and an ablation plan before adding a field. Prefer fields with broad ecological roles over task-specific messages.

## 19.6 How should fracture be abstracted?

One integrity scalar is cheap but may not permit shaping. Regional damage and fracture templates permit edges and pieces but add complexity.

**Experiment:** compare scalar, regional, and coarse-cell damage on modification/reuse tasks. Measure state growth and whether evolved behavior uses the extra degrees of freedom.

## 19.7 What artifact lifetime regime supports reuse without lock-in?

Artifacts that decay too quickly cannot be inherited; immortal artifacts cause clutter and first-mover lock-in.

**Experiment:** sweep wear, environmental decay, repairability, and disturbance relative to organism generation time. Measure reuse, maintenance, turnover, and ecological concentration.

## 19.8 How should internal memory be constrained?

External memory is unlikely to evolve when internal recurrent state is free, exact, and unlimited. Artificially penalizing memory can itself author a desired outcome.

**Options:** metabolic cost per recurrent activity, bounded recurrent state, noise, developmental reset, plasticity limits, or no explicit penalty but genuinely long horizons. The choice requires separate study.

## 19.9 How should social observation be represented?

An observer needs actor-object relations without receiving semantic action labels. Possible encodings include local motion/contact events, body-relative trajectories, attention-gated slots, or a low-resolution visual field.

**Risk:** an engineered action summary can solve imitation. Use physical trajectories and contact consequences where feasible.

## 19.10 What constitutes teaching?

Teaching is stronger than social learning and generally requires behavior by a knowledgeable individual that is modified in the presence of a learner, incurs cost or lacks immediate benefit, and facilitates learning.

**Recommendation:** do not implement teaching as a mechanism. Log exposure, actor cost, learner state, and contingent behavior so offline analyses can test candidate teaching under explicit criteria.

## 19.11 How should morphology and effectors coevolve with artifacts?

Variable morphology changes affordances. Larger grips, stronger bodies, longer effectors, or different sensory organs can substitute for tools or specialize around them.

**Research opportunity:** analyze reciprocal evolution of morphology and artifact ecology. **Confound:** tool-use comparisons must condition on body capabilities.

## 19.12 How can ecological gradients avoid becoming puzzles?

An experiment needs selection pressure, but a narrow apparatus encodes intended behavior.

**Approach:** procedural families parameterized by physical descriptors, multiple causal routes, graded benefit, and held-out combinations. Measure route diversity and substitution robustness.

## 19.13 How should observer classifications handle ambiguous function?

One arrangement can provide barrier, signal, storage, and memory functions simultaneously. Binary labels lose this structure.

**Recommendation:** store evidence vectors and causal effect estimates with uncertainty, not one canonical label.

## 19.14 Can deterministic parallel constraint solving scale far enough?

Canonical merges preserve semantics but may bottleneck. Off-the-shelf engines document determinism constraints that often conflict with SIMD or parallel acceleration [R46](#r46) [R47](#r47).

**Benchmark:** compare serial reference, chunk-disjoint parallel, island-parallel, and deterministic graph-color solvers. Require tick-hash identity across thread counts.

## 19.15 How much provenance is scientifically necessary?

Full material ancestry may be unaffordable; too little provenance makes cumulative modification hard to establish.

**Recommendation:** preserve immediate causal parents and creation events authoritatively, with content-addressed deep history in an analytics archive and bounded summaries in live state.

## 19.16 Should structures ever be promoted to optimized runtime entities?

Large stable assemblies may need compound collision and sleeping optimization.

**Answer:** promotion is acceptable as a derived mechanical cache keyed by membership/revision. It must not grant semantic function, and deconstruction must preserve member identities and bonds.

## 19.17 How should flexible matter enter later?

Rope, fibers, cloth, and nets greatly expand construction and tool possibilities but create large constraint graphs.

**Possible compromise:** chain or strip primitives with a strict segment cap and coarse bending stiffness. Add only after rigid composites are behaviorally accessible.

## 19.18 What save-compatibility horizon is realistic?

Exact replay across semantic versions may be impossible without preserving old executables. The project must choose whether saves promise continuation, historical replay, or both.

**Recommendation:** preserve original saves and versioned executables for historical replay; use explicit migration checkpoints and new replay epochs for continued worlds.

## 19.19 How should branch interventions interact with random events?

Common random numbers improve causal comparison but an intervention changes the set of events and entities.

**Recommendation:** retain address-based draws for unaffected coordinates and assign intervention-created events a branch namespace. Document when trajectories become structurally incomparable.

## 19.20 How should open-endedness be evaluated?

Continued object count or visual complexity is not open-ended evolution. Open-ended-evolution research itself highlights unresolved definitions and measurement challenges [R34](#r34).

Potential evidence includes sustained production of novel causal dependencies, expanded ecological capability, recurring innovation after perturbation, and no fixed externally enumerable solution set. None is sufficient alone.

## 19.21 What would falsify the recommended architecture?

The hybrid model should be reconsidered if well-controlled trials show that:

- controllers cannot discover manipulation despite simplified graded ecologies;
- 2.5D prevents essential target behaviors;
- fixed-point contact approximations erase meaningful property differences;
- state aggregation destroys reuse or inheritance evidence;
- deterministic canonical solving cannot meet scale targets;
- broad bond rules still collapse into a narrow recipe graph;
- object persistence reliably causes ecological lock-in rather than continued innovation;
- social observation cannot be represented without semantic leakage.

A negative result is useful. It identifies which assumption, not merely which implementation, failed.

---
# 20. Annotated bibliography

The bibliography prioritizes primary research, peer-reviewed reviews, and official technical documentation. The annotations state what each source supports and where extrapolation to The Genesis Engine remains an engineering judgment. DOI links resolve to the version of record where available.

## 20.1 Affordances, embodiment, and distributed cognition

<a id="r1"></a>
**R1. Gibson, J. J. (1979). *The Ecological Approach to Visual Perception*. Boston: Houghton Mifflin.**  
**Contribution:** The foundational ecological-psychology account of direct perception and affordances: action possibilities are properties of organism–environment relations rather than detached semantic descriptions.  
**Use here:** Supports making affordances observer- or organism-relative rather than storing labels such as `tool`, `shelter`, or `path` in world state. It does not specify a computational ontology or prove that any particular simulation representation is adequate.

<a id="r2"></a>
**R2. Chemero, A. (2003). “An Outline of a Theory of Affordances.” *Ecological Psychology*, 15(2), 181–195. [doi:10.1207/S15326969ECO1502_5](https://doi.org/10.1207/S15326969ECO1502_5).**  
**Contribution:** Develops a relational account in which affordances depend on features of environments and abilities of organisms.  
**Use here:** Supports computing candidate affordances from body capabilities, geometry, and material state instead of defining them as intrinsic object classes. Mapping that philosophical relation to fixed-point thresholds is an engineering hypothesis.

<a id="r3"></a>
**R3. Rietveld, E., & Kiverstein, J. (2014). “A Rich Landscape of Affordances.” *Ecological Psychology*, 26(4), 325–352. [doi:10.1080/10407413.2014.958035](https://doi.org/10.1080/10407413.2014.958035).**  
**Contribution:** Extends affordance theory to socially and culturally shaped practices and stresses that relevance depends on forms of life and abilities.  
**Use here:** Motivates distinguishing physical possibility from historically learned relevance. The simulation should expose properties and events; cultural significance should emerge in agents and offline analysis.

<a id="r4"></a>
**R4. Clark, A., & Chalmers, D. (1998). “The Extended Mind.” *Analysis*, 58(1), 7–19. [doi:10.1093/analys/58.1.7](https://doi.org/10.1093/analys/58.1.7).**  
**Contribution:** Argues that external resources can participate in cognitive processes when they are reliably integrated into action.  
**Use here:** Provides conceptual support for persistent marks, caches, arrangements, and artifacts as external memory. It is a philosophical argument, not evidence that a given artifact log demonstrates cognition; causal interventions are still required.

<a id="r5"></a>
**R5. Hutchins, E. (1995). *Cognition in the Wild*. Cambridge, MA: MIT Press. [doi:10.7551/mitpress/1881.001.0001](https://doi.org/10.7551/mitpress/1881.001.0001).**  
**Contribution:** Shows how cognition can be distributed across people, artifacts, representations, and structured environments.  
**Use here:** Supports treating shared infrastructure and artifact-mediated coordination as system-level cognitive organization. Direct transfer from human ethnography to artificial organisms is interpretive, so the report recommends event-level evidence rather than anthropomorphic labels.

<a id="r6"></a>
**R6. Kirsh, D., & Maglio, P. (1994). “On Distinguishing Epistemic from Pragmatic Action.” *Cognitive Science*, 18(4), 513–549. [doi:10.1207/s15516709cog1804_1](https://doi.org/10.1207/s15516709cog1804_1).**  
**Contribution:** Distinguishes actions that directly advance an external goal from actions that simplify cognition, perception, or planning.  
**Use here:** Motivates metrics for object rearrangements that reduce future search or control cost, even when they do not immediately increase resource gain. Whether an evolved action is epistemic must be established through counterfactual benefit, not appearance.

## 20.2 Animal tool use, manufacture, social learning, and cumulative culture

<a id="r7"></a>
**R7. St Amant, R., & Horton, T. E. (2008). “Revisiting the Definition of Animal Tool Use.” *Animal Behaviour*, 75(4), 1199–1208. [doi:10.1016/j.anbehav.2007.09.028](https://doi.org/10.1016/j.anbehav.2007.09.028).**  
**Contribution:** Refines tool-use definitions around the externally employed, manipulable means by which an animal alters a target, another organism, or itself.  
**Use here:** Supports a conservative observer definition that requires causal mediation by a controlled external object. It also shows why simple contact or incidental displacement should not automatically count as tool use.

<a id="r8"></a>
**R8. Bentley-Condit, V. K., & Smith, E. O. (2010). “Animal Tool Use: Current Definitions and an Updated Comprehensive Catalog.” *Behaviour*, 147(2), 185–221. [doi:10.1163/000579509X12512865686555](https://doi.org/10.1163/000579509X12512865686555).**  
**Contribution:** Reviews definitional disputes and catalogs tool behavior across taxa.  
**Use here:** Demonstrates that “tool use” is not a single uncontested category and supports a graded hierarchy. The catalog is descriptive; it does not establish a universal computational classifier.

<a id="r9"></a>
**R9. Seed, A., & Byrne, R. (2010). “Animal Tool-Use.” *Current Biology*, 20(23), R1032–R1039. [doi:10.1016/j.cub.2010.09.042](https://doi.org/10.1016/j.cub.2010.09.042).**  
**Contribution:** Reviews variation in selection, manufacture, deployment, and cognition associated with animal tools.  
**Use here:** Supports separating object carrying, property-sensitive selection, modification, manufacture, and sequential planning instead of collapsing them into one technology score.

<a id="r10"></a>
**R10. Taylor, A. H., Hunt, G. R., Holzhaider, J. C., & Gray, R. D. (2007). “Spontaneous Metatool Use by New Caledonian Crows.” *Current Biology*, 17(17), 1504–1507. [doi:10.1016/j.cub.2007.07.057](https://doi.org/10.1016/j.cub.2007.07.057).**  
**Contribution:** Reports use of one tool to obtain another tool, demonstrating a dependency between object-mediated actions.  
**Use here:** Grounds the proposed distinction between ordinary tool mediation and multi-step tool chains. A simulation metric should reconstruct the dependency graph and test necessity through branch replay rather than infer metatool use from temporal proximity alone.

<a id="r11"></a>
**R11. von Bayern, A. M. P., Danel, S., Auersperg, A. M. I., Mioduszewska, B., & Kacelnik, A. (2018). “Compound Tool Construction by New Caledonian Crows.” *Scientific Reports*, 8, 15676. [doi:10.1038/s41598-018-33458-z](https://doi.org/10.1038/s41598-018-33458-z).**  
**Contribution:** Shows construction of functional compound tools from individually insufficient components.  
**Use here:** Supports a strong composite-tool level requiring assembly, retained component identity, and causal superiority over unassembled parts. The controlled apparatus is task-specific, so the engine should test the capability across procedural physical families.

<a id="r12"></a>
**R12. Kenward, B., Weir, A. A. S., Rutz, C., & Kacelnik, A. (2005). “Tool Manufacture by Naive Juvenile Crows.” *Nature*, 433, 121. [doi:10.1038/433121a](https://doi.org/10.1038/433121a).**  
**Contribution:** Documents spontaneous manufacture-like behavior in juveniles without direct adult demonstration of the tested act.  
**Use here:** Warns against assuming that population-typical artifact behavior is necessarily socially transmitted. Genesis experiments therefore need isolated controls, lineage comparisons, and diffusion analyses.

<a id="r13"></a>
**R13. St Clair, J. J. H., Klump, B. C., Sugasawa, S., Higgott, C. G., Colegrave, N., & Rutz, C. (2018). “Hook Innovation Boosts Foraging Efficiency in Tool-Using Crows.” *Nature Ecology & Evolution*, 2, 441–444. [doi:10.1038/s41559-017-0429-7](https://doi.org/10.1038/s41559-017-0429-7).**  
**Contribution:** Links a modified tool form to improved foraging efficiency.  
**Use here:** Supports measuring tool modification by causal performance gain, not merely by detecting shape change. The specific hook geometry must not be encoded as a privileged recipe.

<a id="r14"></a>
**R14. Whiten, A., Goodall, J., McGrew, W. C., Nishida, T., Reynolds, V., Sugiyama, Y., Tutin, C. E. G., Wrangham, R. W., & Boesch, C. (1999). “Cultures in Chimpanzees.” *Nature*, 399, 682–685. [doi:10.1038/21415](https://doi.org/10.1038/21415).**  
**Contribution:** Compares geographically patterned behavioral variants across chimpanzee communities and argues that some are cultural after ecological and genetic explanations are considered.  
**Use here:** Motivates population-level tradition metrics, but also the need to control environmental opportunity, genotype, and independent innovation before calling a pattern cultural.

<a id="r15"></a>
**R15. Whiten, A., Horner, V., & de Waal, F. B. M. (2005). “Conformity to Cultural Norms of Tool Use in Chimpanzees.” *Nature*, 437, 737–740. [doi:10.1038/nature04047](https://doi.org/10.1038/nature04047).**  
**Contribution:** Reports group-specific persistence and convergence in alternative tool-use techniques.  
**Use here:** Supports measuring within-group convergence, between-group divergence, and resistance to equally effective alternatives. In the engine, these patterns remain evidence for social influence, not proof of norm psychology.

<a id="r16"></a>
**R16. Mesoudi, A., & Thornton, A. (2018). “What Is Cumulative Cultural Evolution?” *Proceedings of the Royal Society B*, 285, 20180712. [doi:10.1098/rspb.2018.0712](https://doi.org/10.1098/rspb.2018.0712).**  
**Contribution:** Clarifies cumulative culture through repeated modification, transmission, and measurable improvement or elaboration across time.  
**Use here:** Grounds the report’s requirement for sequential, socially linked, causally beneficial modifications rather than mere artifact persistence or complexity growth.

<a id="r17"></a>
**R17. Tennie, C., Call, J., & Tomasello, M. (2009). “Ratcheting Up the Ratchet: On the Evolution of Cumulative Culture.” *Philosophical Transactions of the Royal Society B*, 364, 2405–2415. [doi:10.1098/rstb.2009.0052](https://doi.org/10.1098/rstb.2009.0052).**  
**Contribution:** Examines why high-fidelity social learning and preservation of improvements matter for cumulative culture.  
**Use here:** Supports distinguishing recurrent reinnovation from ratcheted inheritance. The proposed artifact-provenance graph is an engineering instrument for testing that distinction.

<a id="r18"></a>
**R18. Dean, L. G., Kendal, R. L., Schapiro, S. J., Thierry, B., & Laland, K. N. (2012). “Identification of the Social and Cognitive Processes Underlying Human Cumulative Culture.” *Science*, 335(6072), 1114–1118. [doi:10.1126/science.1213969](https://doi.org/10.1126/science.1213969).**  
**Contribution:** Experimental comparison of cumulative solution acquisition, highlighting combinations of social learning, communication, and prosocial processes.  
**Use here:** Motivates factorial social-transmission experiments rather than assuming observation alone is sufficient. Human child results do not establish necessary mechanisms for artificial organisms.

<a id="r19"></a>
**R19. Davis, S. J., Vale, G. L., Schapiro, S. J., Lambeth, S. P., & Whiten, A. (2016). “Foundations of Cumulative Culture in Apes: Improved Foraging Efficiency Through Relinquishing and Combining Witnessed Behaviours in Chimpanzees (*Pan troglodytes*).” *Scientific Reports*, 6, 35953. [doi:10.1038/srep35953](https://doi.org/10.1038/srep35953).**  
**Contribution:** Tests behavioral conservatism, switching to more efficient socially demonstrated solutions, and recombination of known action components.  
**Use here:** Supports measuring both innovation and abandonment of established methods. It also cautions that cumulative-looking recombination can fall short of full multi-generational cumulative culture.

## 20.3 Stigmergy, collective construction, and niche construction

<a id="r20"></a>
**R20. Theraulaz, G., & Bonabeau, E. (1999). “A Brief History of Stigmergy.” *Artificial Life*, 5(2), 97–116. [doi:10.1162/106454699568700](https://doi.org/10.1162/106454699568700).**  
**Contribution:** Reviews indirect coordination through environmental traces and local response rules.  
**Use here:** Provides the central basis for artifact- and field-mediated coordination without messages or central plans. The engine should still distinguish true trace causation from common response to the same resource gradient.

<a id="r21"></a>
**R21. Werfel, J., Petersen, K., & Nagpal, R. (2014). “Designing Collective Behavior in a Termite-Inspired Robot Construction Team.” *Science*, 343(6172), 754–758. [doi:10.1126/science.1245842](https://doi.org/10.1126/science.1245842).**  
**Contribution:** Demonstrates decentralized construction from local sensing and local rules in a robotic system.  
**Use here:** Shows that persistent structures can be produced without global plans or centralized control. However, the target structures and policies were engineered, so this is evidence for feasibility of local coordination, not emergence of construction goals.

<a id="r22"></a>
**R22. Green, B., Bardunias, P., Turner, J. S., Nagpal, R., & Werfel, J. (2017). “Excavation and Aggregation as Organizing Factors in De Novo Construction by Mound-Building Termites.” *Proceedings of the Royal Society B*, 284, 20162730. [doi:10.1098/rspb.2016.2730](https://doi.org/10.1098/rspb.2016.2730).**  
**Contribution:** Studies how local excavation and deposition dynamics can organize collective construction.  
**Use here:** Supports exposing generic removal, transport, and placement plus environmental feedback. Directly copying measured termite rules would script one biological solution rather than create a general artifact ecology.

<a id="r23"></a>
**R23. Ocko, S. A., Heyde, A., & Mahadevan, L. (2019). “Morphogenesis of Termite Mounds.” *Proceedings of the National Academy of Sciences*, 116(9), 3379–3384. [doi:10.1073/pnas.1818759116](https://doi.org/10.1073/pnas.1818759116).**  
**Contribution:** Connects mound growth to coupled geometry, transport, and environmental flows.  
**Use here:** Supports later introduction of coarse fields such as airflow, temperature, or humidity when they create feedback between structure and fitness. It does not justify adding all such fields in the minimal model.

<a id="r24"></a>
**R24. Odling-Smee, F. J., Laland, K. N., & Feldman, M. W. (1996). “Niche Construction.” *The American Naturalist*, 147(4), 641–648. [doi:10.1086/285870](https://doi.org/10.1086/285870).**  
**Contribution:** Formalizes the idea that organisms modify selection-relevant environmental states rather than merely adapt to them.  
**Use here:** Supports treating organism-caused terrain and artifact changes as part of the evolutionary dynamics, not cosmetic world detail.

<a id="r25"></a>
**R25. Odling-Smee, F. J., Erwin, D. H., Palkovacs, E. P., Feldman, M. W., & Laland, K. N. (2013). “Niche Construction Theory: A Practical Guide for Ecologists.” *The Quarterly Review of Biology*, 88(1), 3–28. [doi:10.1086/669266](https://doi.org/10.1086/669266).**  
**Contribution:** Reviews mechanisms, ecological consequences, and empirical approaches to niche construction.  
**Use here:** Grounds environmental inheritance metrics based on modified exposure and fitness effects across generations. Whether a particular persistent change is adaptive niche construction must be tested against neutral environmental persistence.

## 20.4 Evolutionary robotics, digital evolution, and open-ended systems

<a id="r26"></a>
**R26. Sims, K. (1994). “Evolving Virtual Creatures.” In *Proceedings of SIGGRAPH ’94*, 15–22. [doi:10.1145/192161.192167](https://doi.org/10.1145/192161.192167).**  
**Contribution:** A landmark demonstration of jointly evolved virtual morphology and control in a physically simulated environment.  
**Use here:** Supports embodied evaluation and the possibility that useful behavior emerges from morphology–controller–environment coupling. It does not address persistent artifacts or strict cross-platform determinism.

<a id="r27"></a>
**R27. Sims, K. (1994). “Evolving 3D Morphology and Behavior by Competition.” *Artificial Life*, 1(4), 353–372. [doi:10.1162/artl.1994.1.4.353](https://doi.org/10.1162/artl.1994.1.4.353).**  
**Contribution:** Shows competition-driven evolution of varied bodies and behaviors in simulated physics.  
**Use here:** Reinforces the need to condition artifact-use metrics on body capabilities and to avoid treating tools independently of morphology.

<a id="r28"></a>
**R28. Bongard, J. (2011). “Morphological Change in Machines Accelerates the Evolution of Robust Behavior.” *Proceedings of the National Academy of Sciences*, 108(4), 1234–1239. [doi:10.1073/pnas.1015390108](https://doi.org/10.1073/pnas.1015390108).**  
**Contribution:** Reports that staged morphological change can facilitate robust controller evolution.  
**Use here:** Supports developmental or curriculum hypotheses in which manipulation difficulty is introduced gradually. It does not imply that a predetermined complexity ladder is appropriate for an open-ended world.

<a id="r29"></a>
**R29. Ofria, C., & Wilke, C. O. (2004). “Avida: A Software Platform for Research in Computational Evolutionary Biology.” *Artificial Life*, 10(2), 191–229. [doi:10.1162/106454604773563612](https://doi.org/10.1162/106454604773563612).**  
**Contribution:** Describes a mature digital-evolution platform with explicit genomes, mutation, execution, lineage, and controlled experimentation.  
**Use here:** Supports rigorous replay, ancestry, intervention, and experiment infrastructure. Avida’s instruction-based organisms and abstract tasks do not directly solve spatial artifact physics.

<a id="r30"></a>
**R30. Lenski, R. E., Ofria, C., Pennock, R. T., & Adami, C. (2003). “The Evolutionary Origin of Complex Features.” *Nature*, 423, 139–144. [doi:10.1038/nature01568](https://doi.org/10.1038/nature01568).**  
**Contribution:** Uses digital evolution and lineage replay to reconstruct incremental pathways to a complex function.  
**Use here:** Strongly supports ancestry-preserving event logs and historical replay for determining whether multi-step artifact capabilities arose through selectable intermediates. Its reward environment was explicitly authored, so it is not a model of unscripted technology.

<a id="r31"></a>
**R31. Ray, T. S. (1991). “An Approach to the Synthesis of Life.” In C. G. Langton, C. Taylor, J. D. Farmer, & S. Rasmussen (eds.), *Artificial Life II*, 371–408. Redwood City, CA: Addison-Wesley.**  
**Contribution:** Introduces Tierra and demonstrates ecological interactions among self-replicating programs sharing computational resources.  
**Use here:** Supports endogenous ecology, persistent lineage competition, and the value of resource-mediated interactions. Tierra lacks embodied spatial artifacts and therefore serves mainly as an evolutionary-systems precedent.

<a id="r32"></a>
**R32. Cliff, D., & Grand, S. (1999). “The Creatures Global Digital Ecosystem.” *Artificial Life*, 5(1), 77–93. [doi:10.1162/106454699568683](https://doi.org/10.1162/106454699568683).**  
**Contribution:** Describes a large-scale artificial-life ecosystem with embodied agents, learning, genetics, and persistent user-mediated worlds.  
**Use here:** Offers lessons about integrating nervous systems, development, and environment at product scale, while also illustrating how opaque authored systems complicate scientific inference.

<a id="r33"></a>
**R33. Smith, T. M. C., & Bedau, M. A. (2000). “Is Echo a Complex Adaptive System?” *Evolutionary Computation*, 8(4), 419–442. [doi:10.1162/106365600568248](https://doi.org/10.1162/106365600568248).**  
**Contribution:** Analyzes Echo, an agent-based artificial ecology with resources, combat, mating, and exchange.  
**Use here:** Provides a precedent for property-bearing resources and ecological interactions, but Echo’s tokens and interaction rules are more semantic and abstract than the proposed physical artifact model.

<a id="r34"></a>
**R34. Taylor, T., Bedau, M., Channon, A., Ackley, D., Banzhaf, W., Beslon, G., Dolson, E., Froese, T., Hickinbotham, S., Ikegami, T., McMullin, B., Packard, N., Rasmussen, S., Virgo, N., Agmon, E., Clark, E., McGregor, S., Ofria, C., Ropella, G., Spector, L., Stanley, K. O., Stanton, A., Timperley, C., Vostinar, A., & Wiser, M. (2016). “Open-Ended Evolution: Perspectives from the OEE Workshop in York.” *Artificial Life*, 22(3), 408–423. [doi:10.1162/ARTL_A_00210](https://doi.org/10.1162/ARTL_A_00210).**  
**Contribution:** Surveys conceptual and measurement challenges in open-ended evolution.  
**Use here:** Justifies caution against equating rising object count, visual novelty, or elapsed generations with open-endedness. It supports a portfolio of novelty, dependency, ecological impact, and continued-innovation measures rather than one scalar.

<a id="r35"></a>
**R35. Wang, R., Lehman, J., Clune, J., & Stanley, K. O. (2019). “POET: Open-Ended Coevolution of Environments and Their Optimized Solutions.” In *Proceedings of the Genetic and Evolutionary Computation Conference*, 142–151. [doi:10.1145/3321707.3321799](https://doi.org/10.1145/3321707.3321799).**  
**Contribution:** Coevolves environmental challenges and agents while transferring solutions across environments.  
**Use here:** Supports procedural challenge families and the role of stepping stones. POET still generates parameterized tasks under an optimization framework; it should not be copied as a hidden technology curriculum.

## 20.5 Artificial and agent-based worlds

<a id="r36"></a>
**R36. Johnson, M., Hofmann, K., Hutton, T., & Bignell, D. (2016). “The Malmo Platform for Artificial Intelligence Experimentation.” In *Proceedings of the 25th International Joint Conference on Artificial Intelligence*, 4246. [IJCAI proceedings entry](https://www.ijcai.org/Proceedings/16/Abstracts/643.html).**  
**Contribution:** Exposes Minecraft’s varied 3D world and action space as an AI experimentation platform.  
**Use here:** Demonstrates the research value of editable terrain, discrete blocks, objects, and multi-agent observation. Minecraft’s block taxonomy, recipes, and game semantics are precisely the kinds of authored progression the Genesis runtime should avoid.

<a id="r37"></a>
**R37. Kolve, E., Mottaghi, R., Han, W., VanderBilt, E., Weihs, L., Herrasti, A., Gordon, D., Zhu, Y., Gupta, A., & Farhadi, A. (2017). “AI2-THOR: An Interactive 3D Environment for Visual AI.” [arXiv:1712.05474](https://arxiv.org/abs/1712.05474).**  
**Contribution:** Provides an interactive embodied environment with manipulable household objects and standardized perception/action interfaces.  
**Use here:** Useful for studying object-state APIs and interaction logging. Its semantic object classes and scripted state transitions are unsuitable as the core ontology for emergent technologies.

<a id="r38"></a>
**R38. Gan, C., Schwartz, J., Alter, S., Mrowca, D., Schrimpf, M., Traer, J., De Freitas, J., Kubilius, J., Bhandwaldar, A., Haber, N., Sano, M., Kim, K., Wang, E., Lingelbach, M., Curtis, A., Feigelis, K., Bear, D., Gutfreund, D., Cox, D., Torralba, A., DiCarlo, J., Tenenbaum, J., & Isola, P. (2020). “ThreeDWorld: A Platform for Interactive Multi-Modal Physical Simulation.” [arXiv:2007.04954](https://arxiv.org/abs/2007.04954).**  
**Contribution:** Describes a rich physical-simulation platform with multimodal observations and object interactions.  
**Use here:** Informs observer tooling, physics-rich evaluation, and synthetic sensory channels. Its fidelity and 3D cost are substantially beyond the recommended minimal deterministic world.

<a id="r39"></a>
**R39. Gan, C., Bhandwaldar, A., Torralba, A., Tenenbaum, J. B., & Isola, P. (2021). “OPEn: An Open-Ended Physics Environment for Learning Without a Task.” [arXiv:2110.06912](https://arxiv.org/abs/2110.06912).**  
**Contribution:** Studies task-independent exploration and reusable physical representations in an open-ended physics environment.  
**Use here:** Supports evaluating whether organisms learn transferable material and interaction regularities rather than overfitting one apparatus. “Open-ended” here means task-unspecified exploration, not demonstrated open-ended evolution.

<a id="r40"></a>
**R40. Luke, S., Cioffi-Revilla, C., Panait, L., Sullivan, K., & Balan, G. (2005). “MASON: A Multiagent Simulation Environment.” *Simulation*, 81(7), 517–527. [doi:10.1177/0037549705058073](https://doi.org/10.1177/0037549705058073).**  
**Contribution:** Presents a high-performance discrete-event and agent-based simulation framework designed for large populations and reproducibility.  
**Use here:** Supports separation of model state from visualization and careful scheduling. MASON’s generic scheduler does not itself define deterministic physical conflict semantics.

<a id="r41"></a>
**R41. Epstein, J. M., & Axtell, R. (1996). *Growing Artificial Societies: Social Science from the Bottom Up*. Washington, DC/Cambridge, MA: Brookings Institution Press and MIT Press.**  
**Contribution:** Introduces Sugarscape models in which simple local rules, spatial resources, movement, trade, and inheritance generate population-level patterns.  
**Use here:** Demonstrates the analytical power and scalability of grid-based agent worlds. Its resource and exchange abstractions are intentionally simplified and do not provide general artifact mechanics.

## 20.6 Artificial chemistries and local transformation systems

<a id="r42"></a>
**R42. Dittrich, P., Ziegler, J., & Banzhaf, W. (2001). “Artificial Chemistries—A Review.” *Artificial Life*, 7(3), 225–275. [doi:10.1162/106454601753238636](https://doi.org/10.1162/106454601753238636).**  
**Contribution:** Provides a framework for artificial chemistries in terms of molecular representation, reaction rules, and reactor algorithms, and reviews their uses in emergence research.  
**Use here:** Supports thinking explicitly about conservation, locality, reaction scheduling, and representational closure. It also highlights that an artificial chemistry can conceal a large authored rule graph; therefore chemistry should not be added merely as a more scientific-looking crafting system.

<a id="r43"></a>
**R43. Sayama, H. (2009). “Swarm Chemistry.” *Artificial Life*, 15(1), 105–114. [doi:10.1162/artl.2009.15.1.15107](https://doi.org/10.1162/artl.2009.15.1.15107).**  
**Contribution:** Demonstrates emergent macroscopic structures from mixtures of locally interacting particle types governed by compact behavioral parameters.  
**Use here:** Supports local, property-driven interactions and observer-level pattern classification. Swarm Chemistry is primarily a collective-motion model and does not establish a tractable rigid-artifact implementation.

<a id="r44"></a>
**R44. Kruszewski, G., & Mikolov, T. (2021). “Emergence of Self-Reproducing Metabolisms as Recursive Algorithms in an Artificial Chemistry.” *Artificial Life*, 27(3–4), 277–299; published online 2022. [doi:10.1162/artl_a_00355](https://doi.org/10.1162/artl_a_00355).**  
**Contribution:** Shows emergent metabolic and self-reproducing structures in a minimal artificial chemistry with conservation laws and generic rewriting dynamics.  
**Use here:** Demonstrates that broad local rules can support unexpected organization, but also that computationally universal chemistries can create severe analysis, performance, and state-growth risks. The report therefore defers open-ended chemistry until rigid artifact mechanics are validated.

## 20.7 Deterministic random numbers, physics, and serialization

<a id="r45"></a>
**R45. Salmon, J. K., Moraes, M. A., Dror, R. O., & Shaw, D. E. (2011). “Parallel Random Numbers: As Easy as 1, 2, 3.” In *Proceedings of SC ’11*, Article 16, 1–12. [doi:10.1145/2063384.2063405](https://doi.org/10.1145/2063384.2063405).**  
**Contribution:** Introduces counter-based random-number generators whose outputs are addressable by keys and counters rather than mutable traversal-dependent state.  
**Use here:** Strongly supports named, keyed random streams for deterministic parallel mutation and world events. Correct key-space design, collision avoidance, and versioning remain implementation responsibilities.

<a id="r46"></a>
**R46. Box2D Project. “Simulation.” Official Box2D documentation. [box2d.org/documentation/md_simulation.html](https://box2d.org/documentation/md_simulation.html).**  
**Contribution:** Documents practical rigid-body simulation, collision, constraint solving, event ordering, and determinism caveats.  
**Use here:** Evidence from a mature engine that input/body ordering, compiler behavior, floating-point details, and transcendental functions can affect replay. This is technical documentation, not peer-reviewed evidence that Box2D meets Genesis’s cross-platform determinism contract.

<a id="r47"></a>
**R47. Dimforge. “Determinism.” Official Rapier documentation. [rapier.rs/docs/user_guides/rust/determinism](https://rapier.rs/docs/user_guides/rust/determinism/).**  
**Contribution:** Describes Rapier’s determinism guarantees, ordering requirements, enhanced-determinism feature, and tradeoffs involving SIMD and parallelism.  
**Use here:** Provides a concrete warning that deterministic physics is conditional and configuration-sensitive. The recommended fixed-point, bounded-iteration reference solver is an engineering response to stricter requirements, not a claim derived from Rapier.

<a id="r48"></a>
**R48. Dimforge. “Serialization.” Official Rapier documentation. [rapier.rs/docs/user_guides/templates/serialization](https://rapier.rs/docs/user_guides/templates/serialization/).**  
**Contribution:** Documents serialization of physics-world state and the relationship between snapshots and deterministic continuation.  
**Use here:** Useful as a practical comparison for complete-state persistence. Genesis requires a stronger schema: explicit semantic versions, canonical encoding, independent checksums, fail-closed validation, and refusal to infer missing mutable state from the procedural seed.

---

## Bibliographic synthesis

The evidence is strongest for the following claims:

1. **Affordances are relational.** Useful action possibilities depend jointly on environmental properties and organism abilities; semantic object labels are not necessary to make those possibilities available [R1](#r1)–[R3](#r3).
2. **Artifacts can mediate cognition and coordination.** External arrangements can reduce cognitive work, preserve information, and coordinate agents through persistent traces [R4](#r4)–[R6](#r6), [R20](#r20).
3. **Tool use is graded and causally definable.** Selection, reuse, modification, manufacture, composites, and dependency chains are meaningfully distinct [R7](#r7)–[R13](#r13).
4. **Tradition and accumulation require stronger controls than recurrence.** Ecological opportunity, genotype, independent rediscovery, social exposure, fidelity, and cross-generational improvement must be separated [R14](#r14)–[R19](#r19).
5. **Local construction and environmental feedback can generate global structure.** Decentralized placement, excavation, fields, and traces can organize collective outcomes [R21](#r21)–[R25](#r25).
6. **Embodiment and ecology materially shape evolved behavior.** Morphology, controller, environment, and resource structure co-determine what can evolve [R26](#r26)–[R35](#r35).
7. **Rich worlds are useful but semantic APIs can pre-script solutions.** Existing platforms demonstrate manipulation and persistent environments while also exposing the danger of object-class and recipe leakage [R36](#r36)–[R41](#r41).
8. **Generic local transformations can produce unexpected organization, but not for free.** Artificial chemistry increases possibility space while sharply increasing computational, interpretive, and runaway-growth risks [R42](#r42)–[R44](#r44).
9. **Strict determinism must be designed across the whole stack.** Addressable randomness, canonical ordering, numerical semantics, solver iteration, serialization, and versioning are coupled requirements [R45](#r45)–[R48](#r48).

The central architecture proposed in this report—a chunked 2.5D mutable terrain, fixed-point bodies, small material property vectors, generic manipulation, explicit bonds, observer-derived artifact functions, canonical interaction resolution, and snapshot-plus-journal persistence—is therefore an **engineering synthesis**, not a directly validated package from any one source. Its individual principles are evidence-backed; its combined scalability and evolvability must be established by the dependency-ordered experiments in Sections 15 and 16.
