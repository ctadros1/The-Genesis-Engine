---
name: genesis-experimental-methodology
description: Expert guidance for designing, reviewing, and planning deterministic Genesis Engine artificial-life experiments. Use this skill when working on experimental methodology, open-ended evolution evaluation, replication strategy, metrics, validation, reproducibility, scaling, event logging, persistence, or scientific claims.
---

# Genesis Experimental Methodology

## Activation

Use this skill when an AI assistant is helping with:

- Genesis Engine research experiments.
- Open-ended evolution claims.
- Multi-seed experiment design.
- Scientific validation plans.
- Metrics for emergence, culture, tools, traditions, complexity, or adaptation.
- Deterministic simulation architecture.
- Event logging and offline analysis.
- Large-scale execution planning.
- Save/replay/reproducibility decisions.

Do not use this skill to design organism behavior directly. It governs how Genesis experiments and interpretations should be structured.

---

# Core Genesis Principles

## Author mechanisms, not progress

Genesis should define:

- Physics.
- Affordances.
- Perception.
- Actions.
- Learning mechanisms.
- Inheritance.
- Environmental feedback.

Genesis should not define:

- Technology trees.
- Civilization stages.
- Era unlocks.
- Named cultural states.
- Goals such as "invent tools" or "build societies".

Emergent labels belong only to offline analysis.

## Observation never becomes control

Offline systems may detect:

- Traditions.
- Groups.
- Eras.
- Tool-use candidates.
- Cultural lineages.

These outputs must never influence:

- Organism decisions.
- Selection.
- Rewards.
- Physics.
- Simulation state.

## Determinism is experimental infrastructure

Every recommendation must consider:

- RNG keying.
- Ordering.
- Stable identifiers.
- Serialization.
- Replay.
- Checkpoint restoration.
- Versioning.

A feature that works visually but breaks reproducibility is not complete.

## Claims require evidence levels

Avoid statements such as:

- "Culture emerged."
- "Technology evolved."
- "Civilization formed."
- "Open-ended evolution was achieved."

unless supported by:

- Defined operational criteria.
- Independent worlds.
- Controls and ablations.
- Statistical analysis.
- Reproducible evidence.

---

# Experimental Design Rules

## Experimental unit

Default assumption:

The independent replicate is the world or seed block.

Do not treat:

- organisms,
- births,
- actions,
- ticks,
- encounters,

as independent samples unless the intervention truly occurs at that level.

Use hierarchical analysis when analyzing nested observations.

## Discovery vs confirmation

Separate:

1. Exploration.
2. Pilot calibration.
3. Locked confirmation.
4. Independent replication.

Do not use the most interesting seed as proof.

## Required experiment components

Before proposing a major Genesis experiment, define:

- Hypothesis.
- Mechanism.
- Treatment.
- Control.
- Ablation.
- Experimental unit.
- Seed registry.
- Primary metric.
- Smallest meaningful effect.
- Statistical decision rule.
- Determinism requirements.
- Performance budget.

---

# Measuring Emergence

No single metric proves open-ended evolution.

Use metric panels covering:

- Novelty.
- Adaptive consequence.
- Persistence.
- Diversity.
- Evolvability.
- Ecological change.
- Complexity.
- Saturation behavior.

Avoid confusing:

- novelty with innovation,
- size with complexity,
- clustering with culture,
- contact with tool use,
- improvement with cumulative culture.

---

# Causal Standards

## Social transmission

A cultural claim requires more than group differences.

Require evidence separating:

- genetics,
- shared environment,
- independent invention,
- social transmission.

Prefer:

- randomized arbitrary variants,
- exposure records,
- social-channel ablations,
- donor removal,
- transmission analysis.

## Tool use

Object interaction is insufficient.

Require:

- external object mediation,
- functional benefit,
- removal or function-destroying controls.

## Cumulative culture

Require:

- social transmission,
- retained improvements,
- modification over time,
- predecessor dependence,
- comparison against asocial/genetic alternatives.

---

# Deterministic Engineering Review

When reviewing architecture, check:

## RNG

Ask:

- Is randomness semantic or consumption-ordered?
- Could adding an unrelated random draw change outcomes?
- Are draws keyed by world/entity/event purpose?

Prefer counter-based semantic RNG.

## Ordering

Reject:

- hash-map iteration affecting behavior,
- thread completion affecting results,
- first-writer wins races,
- unstable sorting without tie breakers.

Require canonical ordering.

## Serialization

Check:

- all future-affecting state is saved,
- learned state is included,
- scheduler state is included,
- event cursors are included,
- migrations are explicit.

## Replay

Require:

- deterministic checkpoints,
- state hashes,
- event hashes,
- restore verification.

---

# Scaling Guidance

Prefer scaling independent worlds before distributing one world.

Recommended order:

1. Optimize one world.
2. Run many worlds.
3. Improve storage/checkpointing.
4. Add deterministic intra-world parallelism.
5. Evaluate GPU only with end-to-end benchmarks.

GPU suitability must be proven. Irregular evolving systems often have poor GPU characteristics.

---

# Common Anti-Patterns

Avoid:

- Adding civilization stages.
- Rewarding desired outcomes.
- Feeding analysis back into organisms.
- Calling one impressive run evidence.
- Counting organisms as replicates.
- Using novelty alone as progress.
- Using LLMs as organism controllers.
- Hiding RNG/order dependencies.
- Replacing experiments with screenshots.
- Removing failed worlds from analysis.

---

# Decision Framework

When proposing a Genesis feature:

1. What mechanism does this add?
2. What affordance does it create?
3. What behaviors become possible?
4. What alternative explanations exist?
5. How would we test the mechanism?
6. What ablation would falsify it?
7. What state must be serialized?
8. What determinism risks appear?
9. What metrics could be misleading?
10. What claim level would success justify?

---

# Genesis Relevance

## Supports

This skill supports:

- Open-ended evolution evaluation.
- Experimental methodology.
- Culture and technology research phases.
- Scaling experiments.
- Deterministic execution infrastructure.
- Scientific reporting.

## Interacts with

- Simulation kernel.
- Experiment controller.
- Observer protocol.
- Event logging.
- Persistence system.
- Offline analysis pipeline.
- Headless execution infrastructure.

## Does not decide

This skill does not define:

- Neural architectures.
- Genome encodings.
- Physics rules.
- Organism motivations.
- Specific world mechanics.

Those require separate Genesis design skills.

---

# Limitations

This research does not prove:

- That open-ended evolution is achievable.
- That Genesis will produce civilization-like behavior.
- That specific mechanisms guarantee culture or technology.
- That larger simulations necessarily create richer evolution.

Evidence limitations:

- Many artificial-life findings are system-specific.
- Metrics remain representation-dependent.
- High-level emergence claims require Genesis-specific validation.

Some recommendations are engineering judgments rather than scientific consensus, especially:

- metric panels,
- architecture choices,
- persistence strategies,
- scaling priorities.

All major conclusions require validation through controlled Genesis experiments.

---

# Reference Material

Detailed research, bibliography, comparison tables, and technical notes are preserved in:

`references/genesis_open_ended_evolution_experiments_and_scale.md`
