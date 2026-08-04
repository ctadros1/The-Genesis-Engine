# Scope And Non-Goals

The authority on what the project claims, hopes, and predicts is
`25-emergence-and-epistemic-position.md`. This document says what is in
scope, deferred, and permanently out.

## In Scope

### Delivered (Phases 0 to 4)

- A persistent single-world simulation with controlled multi-world experiments later.
- Continuous organisms on a bounded 2D continent represented by environmental fields.
- Terrain, water, elevation, food, and the specified climate/season model where implemented.
- Energy, movement, sensing, feeding, reproduction, mutation, aging, death, competition, and lineage.
- Compact inherited neural networks; no per-creature language model.
- A private web observer with pixel art, scientific overlays, charts, controls, selection, save/restore, and audit records.
- Deterministic/replayable modes, exportable statistics, benchmarks, Prometheus metrics, and safe deployment planning.

### Planned (Phases 5 to 16)

Each item is a named phase in `19-implementation-roadmap.md` with falsifiable
acceptance criteria and a stated ablation.

- **Phase 5** Headless accelerated execution, an independent-world scheduler, the append-only event log, and a multi-seed multi-condition experiment harness.
- **Phase 6** Moisture and temperature fields, biome classification, long-timescale climate drift, and three world origin modes with founder demes and archetypes.
- **Phase 7** Health, damage, contest, carcasses, and kin-biased grouping.
- **Phase 8** A diploid chromosomal genome with meiosis, linkage, dominance, and structural mutation, producing variable network topology.
- **Phase 9** Modular morphology: typed modules on a lattice grown by a developmental genome, with mass, speed, sensing, and controller node budget all derived from the module set.
- **Phase 10** Genome-encoded synaptic plasticity with evolved neuromodulation.
- **Phase 12** Perception of other organisms' actions and a costly local signal channel with no authored meaning.
- **Phase 11** Materials, objects, carrying, placing, striking, combining, and persistent world modification, with save format 2.
- **Phase 13** Allometric metabolism, thermoregulation, ontogeny, senescence, sexual selection, and optionally disease.
- **Phase 14** Abiogenesis and a field-regime unicellular phase, coupled to the individual engine.
- **Phase 15** The multicellularity transition as a representation change, with multicellularity itself reached by ordinary morphological evolution.
- **Phase 16** Offline era and tradition detection over the event log.

## Explicitly Deferred

- Broad multi-body-plan ecosystems as an authored feature. Structural
  morphology becomes evolvable in Phase 9 (ADR-0019), but as a lattice of
  typed modules, deliberately not as rigid-body or soft-body physics.
  Full biomechanics remains a different project.
- Elevation mutability. It feeds coastline derivation, drainage, and the temperature lapse term, and the generator validates land fraction and connectivity against it. Deferred with its reasoning in `specifications/mutable-world-state.md`, not permanently excluded.
- Molecular-level genetics: codons, transcription, protein folding, or gene regulatory dynamics at chemical timescales. The regulatory locus type reserved by schema 2 is allocated in Phase 9 for development, at a coarse level only.
- GPU inference, public multi-user access, PostgreSQL, and intra-world parallelism. Each requires its own benchmark-backed decision. Structural neural-topology evolution moves out of this list and into Phase 8 under ADR-0013.
- Distributed multi-host experiment scheduling. Phase 5 covers one host only.

## Out Of Scope

These are permanent. The change of ambition on 2026-08-04 does not reopen
any of them, and several are load-bearing for the new philosophy rather than
merely inherited.

- **A language model as an organism decision engine.** Optional narration may consume recorded events after the fact and may never influence a tick.
- **Authored progress of any kind.** No technology tree, research prerequisite graph, era state, building recipe, crafting table, tool taxonomy, civilization mechanic, or reward for inventing anything. This is the direct consequence of ADR-0012 and it is what makes the ambition meaningful rather than circular.
- **Training neural networks with reinforcement learning against a hand-authored reward.** Lifetime learning is in scope from Phase 10, and this non-goal is sharpened rather than relaxed by it: there is no reward function, no fitness signal delivered to any network, and no gradient against an objective we chose. The signal that gates plasticity is an ordinary output of the organism's own evolved network. See ADR-0014.
- **Analysis output feeding back into simulation state.** Not as an input channel, not as a config trigger, not as an intervention. Enforced by crate dependency direction, not by convention. See ADR-0016.
- Real-world ecological prediction or scientific claims about real biology without separate validation. Modelling a mechanism faithfully is a claim about the model, never about the world.
- A massive multiplayer game, public social network, marketplace, or account system.
- Hard real-time guarantees.
- Perfect replay across incompatible builds, CPU architectures, or floating-point implementations. ADR-0011's same-build, same-platform policy stands.
- Modifying existing homelab infrastructure during planning.

## Scope Guardrails

If a feature does not advance a named phase's acceptance criteria, it belongs
in the backlog until a phase explicitly adopts it. Visual richness cannot
substitute for measurement. Systems that make a world look alive but cannot
be inspected, saved, replayed, or benchmarked are not release work.

One guardrail specific to the new goal: **before adding any mechanism, ask
whether you can name the specific outcome it makes more likely.** If you can,
it is authored progress and it does not belong here. A material registry
entry is physics because it favors no particular use; a recipe is progress
because it names one.

That test has exactly one recorded exception, ADR-0018: environmental and
selective structure may be shaped toward the major transitions in the
`scratch` origin mode. It is scoped to Phases 14 and 15, it requires a
scaffold description that names no target, and it requires an unscaffolded
control on the same seeds with both results reported. Invoking it anywhere
else needs its own ADR, not an appeal to this one.

## Change Policy

Rules are intended to evolve. Changes must be explicit and versioned, but the
project may replace a weak model after evidence. Preserve old configs and
results; never call two incompatible rule sets the same experiment.
