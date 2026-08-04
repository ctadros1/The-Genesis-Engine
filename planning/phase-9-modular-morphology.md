# Phase 9: Modular Morphology And Development

Status: planned, not started. Policy versions `lifesim-morphology-v1`,
`lifesim-develop-v1`; genome schema 3. Specification:
`specifications/morphology-and-development.md`. Decision: ADR-0019.

## Problem

Body plan is a small fixed parameter set. Every organism has the same
structure and only its numbers differ. Nothing about shape can evolve.

This blocks the goal in two distinct ways. Morphological radiation is not
reachable when structure is fixed. And there is no representation in which
"one cell" and "many differentiated cells" are the same kind of object,
which is exactly what Phase 15's transition requires.

## Scope

- An organism as typed modules on a discrete integer lattice.
- A bounded versioned module type registry: structural, sensory, motor,
  digestive, storage, reproductive, neural.
- Derived phenotype: mass, speed, sensor range, intake, storage capacity,
  basal cost, and controller node budget all become consequences of the
  module set.
- Genome schema 3: regulatory loci carrying a developmental growth program,
  allocating the locus type ADR-0013 reserved.
- Deterministic bounded development from a single origin module.
- Non-viable bodies rejected at birth with typed reasons and counters.

## Non-Goals

- **No physical body simulation.** No rigid body dynamics, no joints, no
  torque, no soft tissue, no module-to-module collision. Modules confer
  capability and cost; they do not move relative to each other. This
  boundary is what keeps determinism exact and cost tractable.
- **No multicellularity mechanic.** A one-module body is a unicell and many
  modules is multicellular; the difference is a region of the same
  morphospace reached by ordinary structural mutation. Nothing detects,
  flags, or rewards the transition.
- No float geometry anywhere in morphology.
- No storage of bodies. They are derived from the genome, like phenotypes.
- No claim that any evolved body resembles any real organism.

## Prerequisites

- Phase 8 (genome schema 2 supplies the locus machinery, innovation IDs, and
  meiosis that schema 3 extends).

## Determinism Notes

- New stream `Morphogenesis` (17), unused under the default fully
  deterministic development policy; it exists so adopting developmental
  noise later cannot renumber.
- Regulatory loci evaluate in ascending `homology_id` order; actions apply
  in ascending `(locus_homology_id, lattice_index)` order.
- Every module sum iterates in ascending lattice index, pinning float
  summation order exactly as Rule 6 does for controller edges.
- Lattice positions are integers; morphology is exactly hashable.
- Development is a pure function of `(genome, config)`, which is why bodies
  are excluded from the save and recomputed on load.
- Checksum section `lifesim-morphology-state-v1` carries only the
  developmental clock and non-viability counters.

## Acceptance Criteria

- [ ] **C9.1 Development is pure and order-independent.** Same genome and
      config gives the same body; a body recomputed after save and restore
      is identical; permuting regulatory locus storage order gives an
      identical body.
- [ ] **C9.2 The unicellular case works.** A one-module body is legal,
      viable, and produces sane bounded derived attributes. Phase 15 depends
      on this and it is verified here rather than discovered there.
- [ ] **C9.3 Morphological change has consequence, not just variance
      (primary).** Structural divergence from the founder distribution is
      necessary but not sufficient: a diverging module count is novelty, and
      novelty is not progress (ADR-0022 A13). The criterion requires
      divergence **and** that the diverged morphologies show a measurable
      fitness or ecological difference **and** that the change persists
      beyond the stated window, in at least 20 of 30 worlds. Control: a
      fixed-morphology condition with regulatory loci inert, matched on total
      mutational input. Divergence without consequence is reported as
      structural drift, which is a real and different finding.
- [ ] **C9.4 Discontinuity gate.** Report the distribution of phenotypic
      distance produced by single-locus mutations. **This is a gate, not a
      metric** (ADR-0022 D1): if the median single-locus mutation produces a
      body beyond the stated dissimilarity threshold, the developmental
      encoding has failed its own premise, selection cannot act on it, and
      the specified direct parameterized body-plan fallback is taken. Two
      commissioned reviews recommend against developmental encodings as a
      baseline; this gate is the concession to that advice and it is
      pass/fail, not advisory.
- [ ] **C9.5 Non-viability rate is bounded and reported.** The fraction of
      births rejected for invalid bodies is a first-class metric. If it is
      high, effective fecundity drops and the ecology shifts, so a stated
      ceiling must hold or the growth grammar is reconsidered.
- [ ] **C9.6 Structural freedom does not destabilize the ecology.** Median
      population and lifespan under evolvable morphology are within the
      stated tolerance of the fixed-morphology control, or the difference is
      explained by a reported mechanism, in at least 20 of 30 seeds.
- [ ] **C9.7 Brain costs body.** Controller node budget derives from neural
      modules, and neural modules carry upkeep. Verify that controller size
      and metabolic cost are coupled as specified, so cognition is expensive
      structurally rather than by stipulation.
- [ ] **C9.8 Bounds and caps.** No derived attribute leaves its clamp for
      any body reachable within the caps; every cap rejects
      deterministically, counts, and events.
- [ ] **C9.9 Ledger exactness** with growth energy flowing through it over a
      10^6-tick run.
- [ ] **C9.10 Snapshot size is unaffected.** Bodies are derived, so
      per-organism snapshot bytes do not grow relative to Phase 8. This is
      a deliberate design property and is verified rather than assumed.
- [ ] **C9.11 Fixtures preserved.** Morphology disabled reproduces the
      Phase 8 fixture exactly.

## Test Plan

- Unit: derivation formulas at module-count and scale boundaries;
  connectivity validation; growth-step termination; cap enforcement.
- Property: every body reachable within caps produces in-bounds derived
  attributes; development always terminates.
- Determinism: C9.1 as automated tests; clean-process fixture; lattice
  summation order independence from storage layout under compaction.
- Integration: retired trait loci are genuinely unused; sensory and motor
  modules bind channels correctly; a one-module organism completes a full
  lifecycle.
- Statistical: C9.3 and C9.4 as automated tests with recorded tolerances,
  seeds, and sample sizes.
- Long run: multi-generation stability with morphological churn, exact
  ledgers, bounded module counts.

## Benchmark Impact

The second largest cost change in the programme after Phase 8.

Record: development cost per birth against genome regulatory-locus count;
per-organism per-tick cost against module count **as a distribution**, since
evolved sizes will be skewed; the interaction with controller cost, because
neural modules drive node budget and the two skews multiply; non-viability
rate; and confirmation that snapshot size per organism is unchanged.

Caps are set from this measurement, not before it.

Benchmark schema 6.

## Documentation Updates

`docs/06-organism-model.md` (body plan section is substantially rewritten),
`docs/07-neural-network-design.md` (node budget), `docs/08-genetics-and-evolution.md`
(schema 3), `docs/02-scope-and-non-goals.md` (rich morphology moves out of
deferred), `docs/09-species-and-lineage.md` (morphological distance),
`specifications/genome-schema-2.md` (locus type 5 allocated),
`specifications/entity-component-model.md`,
`specifications/neural-network-schema.md`,
`specifications/websocket-protocol.md` (module summary in render records),
`docs/10-observer-interface.md`, decision log, ADR-0019.

## Risks

| Risk | Mitigation |
|---|---|
| Genotype-phenotype discontinuity is severe enough that selection cannot act | C9.4 measures it directly. If severe, a more constrained growth grammar or the parameterized fallback is reconsidered, and that is a real possible outcome of this phase |
| High non-viability rate collapses effective fecundity | C9.5 with a stated ceiling; growth grammar constrained so most programs produce connected bodies |
| Development cost per birth dominates the tick | Bounded `max_growth_steps` and `max_modules`; measured before caps are fixed; development runs at birth, not per tick, unless ontogeny is enabled |
| Module-count skew multiplied by topology skew makes tick time unpredictable | Both distributions measured; a per-organism evaluation budget is the fallback policy and would itself be a selection pressure toward small bodies, which must be reported if used |
| The lattice proves too restrictive for a research question | That is an argument for physical morphology and therefore a different project. Recorded in ADR-0019 rather than solved here |
| Retired trait genes leave dead code paths that quietly still apply | Retired trait IDs are never reused; an integration test asserts they are unread |

## Rollback

One config section selecting genome schema. Disabled, schema 2 organisms use
the Phase 8 body-plan code path and reproduce its fixture. Schema 1, 2, and 3
decoders and fixtures all stay in the build permanently, and there is no
migration between them.
