---
name: genesis-mutable-world-tool-use
description: Expert guidance for designing mutable worlds, artifacts, and emergent tool use systems in The Genesis Engine artificial-life simulation. Use when planning or reviewing world physics, object mechanics, manipulation, structures, artifacts, stigmergy, niche construction, or experiments involving persistent environmental change.
---

# Genesis Engine Mutable Worlds, Artifacts, and Emergent Tool Use

## Activation

Use this skill when working on Genesis Engine systems involving:

- Mutable terrain or world state.
- Physical objects, artifacts, materials, or structures.
- Tool use, carrying, placement, construction, caching, or manipulation.
- Environmental inheritance or niche construction.
- Stigmergic coordination.
- Artifact analytics and causal experiments.
- Save formats affected by persistent world changes.

Do not use this skill as a generic game-engine physics guide. It exists to preserve Genesis Engine's artificial-life research methodology.

## Genesis Engine relevance

### Supported project phases

This skill supports:

- Mutable-world substrate development.
- Object permanence and manipulation.
- Carrying and placement experiments.
- Object-assisted behavior research.
- Artifact reuse and caching.
- Structures and environmental modification.
- Composite objects and bonds.
- Social transmission experiments involving artifacts.
- Environmental inheritance and cumulative modification studies.

### Interacts with

- Deterministic simulation kernel.
- Fixed-point physics and collision systems.
- Organism controllers and perception.
- Genome and learning systems.
- Persistence/checkpoint systems.
- Observer analytics and experiment infrastructure.

### Should influence

- Physics abstractions.
- Object/material schemas.
- Action primitives.
- Experiment design.
- Metrics and ablations.
- Serialization decisions.
- Performance architecture.

### Outside scope

This skill does not define:

- Organism neural architectures.
- Genetic encodings.
- Social learning algorithms.
- Evolutionary selection methods.
- Rendering/UI systems.

Those systems may interact with mutable worlds but require their own design rules.

## Core principles

### Author physics and affordances, not progress

The engine should create conditions where behavior can emerge. Do not implement:

- Technology trees.
- Crafting recipes.
- Civilization stages.
- Named tool classes with privileged mechanics.
- Research unlocks.

Implement physical possibilities:

- Mass.
- Friction.
- Hardness.
- Damage.
- Support.
- Contact.
- Persistence.
- Material transformation.

Let observer systems determine whether a behavior resembles tool use or construction.

### Preserve causal emergence

Every behavior must be explainable through mechanisms:

- Organisms perceive physical consequences.
- Controllers produce actions.
- Physics changes state.
- Future organisms encounter changed conditions.

Avoid systems where the simulator recognizes an intended behavior and rewards it.

### Observer isolation

Offline analysis may classify:

- tools;
- structures;
- traditions;
- technologies;
- eras.

Those classifications must never affect:

- organism decisions;
- physics;
- resource generation;
- mutation;
- fitness;
- world updates.

### Determinism is scientific infrastructure

Every recommendation must consider:

- RNG streams and keyed randomness.
- Canonical ordering.
- Serialization.
- Replay.
- Versioning.
- Parallel execution.
- Fixed-point behavior.

A feature that cannot be deterministically reproduced is not ready for scientific experiments.

## Design guidance

## World model

Prefer a constrained hybrid:

- Sparse chunked mutable terrain.
- Discrete physical bodies.
- Fixed-point spatial simulation.
- Limited 2.5D height/support model.
- Explicit bonds for composites.
- Derived observer classifications.

Avoid beginning with:

- Full voxel civilizations.
- General chemistry.
- Full 3D rigid-body simulation.
- Unbounded particles.

Add complexity only when an experiment requires it.

## Object ontology

Use:

- `MaterialDef` for physical properties.
- `PhysicalBody` for stable entities.
- `Bond` for physical connections.
- Provenance records for history.

Do not create privileged runtime classes:

- Tool.
- Weapon.
- Resource.
- Structure.
- Crafting item.
- Technology.

These should be derived analytically.

## Manipulation model

Prefer generic physical primitives:

- Grasp.
- Release.
- Apply force.
- Matter transfer.
- Locomotion/orientation.

Avoid:

- USE_TOOL.
- BUILD.
- CRAFT.
- CUT.
- DIG.
- STORE.

Those are interpretations of physical interactions, not simulation primitives.

## Transformations

Transformations should be:

- Local.
- Property-based.
- Conservative.
- Deterministic.
- Broadly applicable.

Good:

- Hardness influences damage.
- Force influences fracture.
- Geometry influences support.

Bad:

- Flint + wood = spear.
- Hammer object = extra damage.
- Wall tag = blocks movement.

## Structures and artifacts

A structure is not a type. It is a persistent arrangement whose function is measured.

Evaluate:

- Does it alter movement?
- Does it change exposure?
- Does it store matter?
- Does it preserve information?
- Does it affect later organisms?

Require causal tests:

- Remove it.
- Replace it.
- Relocate it.
- Preserve appearance but remove function.

## Experiment framework

When proposing a new capability, define:

1. Research question.
2. Minimal mechanism required.
3. Controls.
4. Ablations.
5. Metrics.
6. Deterministic replay strategy.
7. Failure criteria.

Do not accept demonstrations as proof. Measure across seeds, populations, and counterfactual interventions.

## Common mistakes

Avoid:

- Hidden recipe systems disguised as physics.
- Semantic object labels in organism perception.
- Fitness bonuses for tool use.
- Observer feedback loops.
- Infinite inventories.
- Object identity reuse.
- Camera-dependent simulation changes.
- Nondeterministic iteration order.
- Silent save migration.
- Complexity added without experimental justification.

## Decision framework

Before adding a mechanism ask:

### Scientific necessity

- What hypothesis requires this?
- What behavior becomes possible?
- Can an existing mechanism already explain it?

### Emergence safety

- Does this encode a solution?
- Could multiple strategies produce the same outcome?
- Would organisms need to discover the behavior?

### Engineering safety

- Does it preserve deterministic replay?
- Does it increase state growth?
- Does it complicate serialization?
- Can it be tested with ablations?

### Experimental value

- Can success be measured?
- Can failure be distinguished from lack of opportunity?
- Are causal interventions possible?

## Review checklist

When reviewing a design:

### Physics

- [ ] Uses physical properties rather than semantic categories.
- [ ] Conserves modeled matter and energy.
- [ ] Has deterministic resolution rules.
- [ ] Handles conflicts canonically.

### Emergence

- [ ] Does not reward observer labels.
- [ ] Allows multiple solutions.
- [ ] Separates runtime state from analytics.

### Persistence

- [ ] Saves all future-relevant mutable state.
- [ ] Maintains stable IDs.
- [ ] Supports exact restore.
- [ ] Has explicit migrations.

### Experiments

- [ ] Includes controls.
- [ ] Includes ablations.
- [ ] Uses branch replay where possible.
- [ ] Reports uncertainty.

## Limitations

This skill does not prove that open-ended technological evolution, culture, or civilization will emerge.

The research supports design principles, not guaranteed outcomes.

Weak or unresolved areas include:

- How artificial organisms discover useful manipulation strategies.
- Whether cumulative culture emerges in artificial systems.
- The correct amount of embodiment and physics detail.
- How much internal learning affects external memory evolution.
- Whether the proposed world representation scales to very long evolutionary runs.

Many recommendations are engineering judgments based on combining research fields with Genesis Engine constraints. They require validation through controlled experiments inside the simulation.

## Applying this skill

When creating plans or reviewing code:

1. Identify the physical mechanism being introduced.
2. Check that it does not encode the desired behavior.
3. Identify affected deterministic systems.
4. Propose experiments before claiming emergence.
5. Separate implementation convenience from scientific validity.
6. Preserve the distinction between:
   - simulation state,
   - organism perception,
   - observer interpretation.

The goal is not to make a world that looks advanced.

The goal is to make a world where advanced behavior, if it occurs, can be shown to have emerged.
