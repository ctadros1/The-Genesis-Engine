# Project Vision

## Intent

Artificial Life Simulation is a persistent evolutionary-systems laboratory with a readable pixel-art observer. It should feel alive because organisms compete, feed, reproduce, mutate, age, and die under shared environmental constraints. It should remain intelligible because every major outcome can be inspected through state, lineage, configuration, events, and metrics.

The long-term ambition, adopted 2026-08-04, is that organisms can evolve from simple foragers toward tool use, persistent structures, transmitted knowledge, technological accumulation, territoriality, and organized inter-group conflict, with none of it scripted as stages. The governing philosophy is emergence inside an authored possibility space: we author physics, never progress. See `25-emergence-and-epistemic-position.md`, which is the authority on what the project claims, hopes, and predicts, and `26-biological-realism-policy.md`, which governs how faithfully biology is modelled.

## Experience Statement

A user can open a private browser dashboard, watch a bounded continent change across days and seasons, select a single organism, inspect its traits, neural inputs/outputs, parents, offspring, and local conditions, then compare the result with a controlled seeded experiment. Sandbox interventions are deliberate experiments with logged parameters rather than invisible god-mode edits.

## Success Criteria

- A user can explain what inputs and state led to a visible organism action.
- A seeded run can be rerun under the documented determinism policy.
- A slow or disconnected observer cannot stall the world.
- A save can be validated, restored, and associated with its schema/config/build provenance.
- New rules can be introduced without pretending prior worlds used them.

## Scientific Honesty

The project creates the conditions for adaptation and emergent behavior. It does not guarantee that any particular outcome follows, and it makes no claim about intelligence, language, consciousness, or society. Open-ended evolution is an unsolved grand challenge in artificial life; no system has produced a technological era progression from genuine evolution, and this project should not be described as one before it is.

Two distinctions this document previously blurred, now separated:

- **Mechanism versus outcome.** Biological mechanisms are modelled as faithfully as determinism and budget allow (`26-biological-realism-policy.md`), and their faithfulness is tested against textbook results the mechanisms were not tuned to produce. That is a claim about the model, not about the world. Simulation outcomes remain simulation outcomes and are never evidence about real ecology or real evolution.
- **Ambition versus prediction.** The project aims at a possibility space that includes tool use and transmitted knowledge. It predicts that some of that will not appear, and a null result is an acceptable and reportable outcome of any phase.

Parameters are simulation design choices unless explicitly supported by a cited model or validation experiment. No parameter value corresponds to a real organism.

## Design Pillars

1. **Author physics, not progress.** The simulation defines what is physically possible. It never defines a technology tree, a research graph, an era, a recipe, or a civilization stage. Fitness comes from survival and reproduction, not a hand-authored score.
2. **Analysis observes, it never instructs.** Offline analysis is recorded in reports and can never reach a rule, proven by checksum equality with analysis on and off.
3. **Observability over spectacle.** Pixel art makes the world legible; markers and charts make it auditable.
4. **Reproducibility over accidental novelty.** Seeds, configs, interventions, and versions are recorded.
5. **Measurement over anecdote.** Every behavioral claim is a multi-seed result with a stated control or ablation.
6. **Evidence over dogma.** Rules remain tunable after measurement.
7. **Single-node proof before scale.** Optimize and distribute only after benchmarks justify it.

## Related Documents

- Emergence and epistemic position: 25-emergence-and-epistemic-position.md
- Biological realism policy: 26-biological-realism-policy.md
- Requirements: 01-user-requirements.md
- Scope: 02-scope-and-non-goals.md
- System architecture: 03-system-architecture.md
- Product roadmap: 19-implementation-roadmap.md
