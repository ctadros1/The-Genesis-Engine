# Phase 15: The Multicellularity Transition

Status: planned, not started. Policy version `lifesim-transition-v1`.
Specification: `specifications/unicellular-regime.md`. Decisions: ADR-0020,
ADR-0019, ADR-0018.

## Problem

Phase 14 produces microbial populations in the field regime. Phase 9
produces organisms whose bodies are modules on a lattice. Nothing connects
them, so `scratch` cannot reach an individual-based organism.

This phase is the handoff, and the handoff is the single most defect-prone
piece of the programme: two representations of the same matter, converting
under a threshold.

## The Design That Makes This Small

Most of the work that a multicellularity phase would normally need has
already been done by Phase 9's representation choice.

A one-module organism **is** a unicell. Multicellularity is evolving past
one module with more than one type present. It is a region of the same
morphospace, reached by ordinary structural mutation under ordinary
selection.

Therefore this phase implements **no multicellularity mechanic at all**.
There is no threshold that detects multicellularity, no flag, no grade, and
nothing that rewards adding a second module. What this phase implements is
only the representation change from field density to individual entity. What
happens after that is Phase 9's morphospace doing what it already does.

That is the payoff of ADR-0019's expensive encoding choice, and it is the
argument that justified it.

## Scope

- The aggregation and complexity threshold that triggers materialization.
- Deterministic materialization: field density becomes individual entities
  with synthesized schema-3 genomes producing one-module bodies.
- Exact ledger balance across the conversion.
- `origin.mode = scratch` becomes end-to-end usable.
- Transition-neutrality verification.

## Non-Goals

- **No multicellularity mechanic.** See above. If a reviewer can find code
  that reads "is this organism multicellular" and changes a rule, that is a
  defect.
- No reward, energy bonus, or survival advantage for crossing the threshold.
- No reverse transition from individual back to field. Death returns matter
  to the field through the ordinary carcass and decay paths.
- No claim that an observed transition resembles any real evolutionary
  transition.

## Prerequisites

- Phase 14 (field regime) and Phase 9 (morphology). Both are hard
  prerequisites; there is nothing useful to build here without either.

## Determinism Notes

- New stream `Transition` (20).
- Materialization runs in ascending `(cell_index, class_id)` order; entity
  IDs come from the existing shared monotonic counter in that same order, so
  the transition cannot introduce order-dependence into the individual
  regime.
- The class-to-genome map is documented, deterministic, and versioned. A
  given class under a given config always synthesizes the same genome.
- Ledger: field density debited and organism energy and mass credited in one
  operation, with the rounding remainder assigned to the lowest new entity
  ID, following the existing convention for reproduction investment.
- Checksum: no new section. The transition moves state between two sections
  that already exist.

## Acceptance Criteria

Conditions per ADR-0018, matched on seeds (30) and run length, every
scaffolded condition paired with its unscaffolded control.

- [ ] **C15.1 Conservation across the conversion.** Mass and energy are
      invariant to the milli-unit across every materialization, verified
      over a 10^6-tick run containing many transitions. Extends C14.1 to the
      conversion itself, which is where a defect is most likely.
- [ ] **C15.2 Transition neutrality.** An organism produced by
      materialization has no advantage over an otherwise identical organism
      born normally. Verified by direct comparison of derived attributes,
      starting energy, and subsequent survival distribution. **This is the
      criterion that proves the threshold is a representation change and not
      an authored achievement**, and a failure here means the phase has
      accidentally implemented authored progress.
- [ ] **C15.3 Materialization is order-independent.** Two cells triggering
      in the same tick materialize identically regardless of storage or
      iteration order, verified under storage permutation.
- [ ] **C15.4 The map is deterministic and versioned.** The same genotype
      class under the same config always synthesizes the same genome, across
      clean processes.
- [ ] **C15.5 `scratch` runs end to end**, or reports precisely where it
      stops. A run from chemistry to at least one materialized individual
      organism, in at least N of 30 seeds under the best scaffold condition,
      with N stated before the campaign. Under the unscaffolded control the
      expected result is 0 of 30, and that comparison is the finding.
- [ ] **C15.6 Multicellularity is reached, or reported as not reached.** The
      fraction of seeds in which any lineage exceeds one module, and the
      distribution of module counts over time, for scaffolded and
      unscaffolded conditions. **This is expected to return null and is
      stated so in advance.** A null here is a measured result about
      reachability, reported as such and not weakened after the fact.
- [ ] **C15.7 No multicellularity mechanic exists.** A code review criterion
      stated as an acceptance criterion because it is the phase's central
      claim: no rule reads module count to grant, deny, reward, or penalize
      anything beyond the ordinary structural costs Phase 9 already
      specifies.
- [ ] **C15.8 Determinism and fixtures.** Clean-process fixture replay; save
      round trip with a mid-transition world; transition disabled reproduces
      the Phase 14 fixture exactly.

## Test Plan

- Unit: threshold evaluation at boundaries; class-to-genome map at every
  registered class; ledger arithmetic including the rounding remainder.
- Property: conservation holds for arbitrary densities and class mixes.
- Determinism: C15.3 and C15.4 as automated tests; clean-process fixture;
  save round trip mid-transition.
- Integration: a materialized organism completes a full lifecycle; its death
  returns matter to the field; simultaneous multi-cell transitions.
- Neutrality: C15.2 as a direct A/B comparison test, not an inference from
  aggregate statistics.
- Long run: 10^6 ticks with many transitions and exact conservation.

## Benchmark Impact

Materialization is bursty: cheap when nothing triggers, expensive in a tick
where many cells do. Record the per-transition cost, the worst-case tick
containing a materialization burst, and whether a per-tick materialization
cap is needed. If a cap is introduced it is a deterministic deferral with a
counter, never a silent drop.

Benchmark schema 9.

## Documentation Updates

`docs/04-simulation-model.md`, `docs/06-organism-model.md`,
`specifications/unicellular-regime.md` (status),
`specifications/event-schema.md` (transition events),
`specifications/metrics-schema.md`, `docs/25-emergence-and-epistemic-position.md`
(status of the from-scratch prediction), decision log.

## Risks

| Risk | Mitigation |
|---|---|
| **Conservation defect at the conversion.** Two representations of the same matter converting under a threshold is the most defect-prone shape in the programme | C15.1 over 10^6 ticks; conservation by construction; the remainder convention is explicit rather than incidental |
| The threshold accidentally becomes an achievement that confers advantage | C15.2 tests neutrality directly by A/B comparison; C15.7 makes the absence of a mechanic an acceptance criterion |
| Materialization bursts blow the tick budget | Measured; deterministic capped deferral with a counter if needed |
| **The transition never occurs, or occurs and then multicellularity never does** | Expected. C15.5 and C15.6 are written so both are measurements. The unscaffolded control makes the negative interpretable rather than merely disappointing |
| A scaffolded success is reported as though it were unscaffolded | ADR-0018 requires the scaffold in every report; the intensity sweep makes a single-point result visibly weaker |
| The class-to-genome map quietly encodes a good starting organism | The map is versioned and reviewed; C15.2 neutrality catches an advantage; the synthesized organism is a one-module body with no special parameters |

## Rollback

One config section. Disabled, no transition occurs, `scratch` produces a
field-only world that stays valid and observable, and the Phase 14 fixture
reproduces exactly.
