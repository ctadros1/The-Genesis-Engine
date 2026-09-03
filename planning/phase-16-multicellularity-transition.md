# Phase 16: The Multicellularity Transition

Status: COMPLETE 2026-09-02 (D-130, D-131): every criterion decided. Policy version `lifesim-transition-v1`.
Specification: `specifications/unicellular-regime.md`. Decisions: ADR-0020,
ADR-0019, ADR-0018, and ADR-0032 (the concrete design: the trigger's
persistence window, the constant class-to-genome map, the admission path,
the ledger terms, formats and schemas).

## Problem

Phase 15 produces microbial populations in the field regime. Phase 10
produces organisms whose bodies are modules on a lattice. Nothing connects
them, so `scratch` cannot reach an individual-based organism.

This phase is the handoff, and the handoff is the single most defect-prone
piece of the programme: two representations of the same matter, converting
under a threshold.

## The Design That Makes This Small

Most of the work that a multicellularity phase would normally need has
already been done by Phase 10's representation choice.

A one-module organism **is** a unicell. Multicellularity is evolving past
one module with more than one type present. It is a region of the same
morphospace, reached by ordinary structural mutation under ordinary
selection.

Therefore this phase implements **no multicellularity mechanic at all**.
There is no threshold that detects multicellularity, no flag, no grade, and
nothing that rewards adding a second module. What this phase implements is
only the representation change from field density to individual entity. What
happens after that is Phase 10's morphospace doing what it already does.

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

- Phase 15 (field regime) and Phase 10 (morphology). Both are hard
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

- [x] **C16.1 Conservation across the conversion.** Mass and energy are
      invariant to the milli-unit across every materialization, verified
      over a 10^6-tick run containing many transitions. Extends C15.1 to the
      conversion itself, which is where a defect is most likely.
      *Met 2026-09-02 (D-131 addendum): both identities (field with the
      materialized term subtracted, organism energy with it added) are
      enforced by `check_invariants`, re-derived from the save in
      `phase16_transition.rs`, closed from the fixture's printed totals by
      `verify-phase16-determinism.sh`, and soaked exact over 10^6 ticks
      with 623,597 materializations under live excretion and remains
      (`phase16-c161-ledger-soak`, 200 in-run checks, 0 failures).*
- [x] **C16.2 Transition neutrality.** An organism produced by
      materialization has no advantage over an otherwise identical organism
      born normally. Verified by direct comparison of derived attributes,
      starting energy, and subsequent survival distribution. **This is the
      criterion that proves the threshold is a representation change and not
      an authored achievement**, and a failure here means the phase has
      accidentally implemented authored progress.
      *Met 2026-09-02 (ADR-0032, D-130): one admission function for births
      and materializations (code motion, fixtures intact); the
      founder-path phenotype twin equals the materialized phenotype field
      for field and the organism carries no provenance; the relabelled
      twin (same state, materialized term folded into the founders'
      endowment) shares an identical 600-tick future. The survival clause
      leans on coupling v1's narrowness, as the ADR records.*
- [x] **C16.3 Materialization is order-independent.** Two cells triggering
      in the same tick materialize identically regardless of storage or
      iteration order, verified under storage permutation.
      *Met 2026-09-02: draws are keyed on the slot and ordinal, never the
      entity ID; the test materializes a cell alone and beside another and
      requires identical organisms (and caught the mutation that keys on
      the ID). The field's storage is canonical by construction, so there
      is no alternative layout to permute - the invariance to which other
      cells trigger is the falsifiable content (trap 13/14).*
- [x] **C16.4 The map is deterministic and versioned.** The same genotype
      class under the same config always synthesizes the same genome, across
      clean processes.
      *Met 2026-09-02: `GENOME_MAP_VERSION` 1 in the config hash; the unit
      clause pins the encoded genome equal across classes and its body
      equal to the unicell; the fixture's two-process replay pins it across
      clean processes.*
- [x] **C16.5 `scratch` runs end to end**, or reports precisely where it
      stops. A run from chemistry to at least one materialized individual
      organism, in at least N of 30 seeds under the best scaffold condition,
      with N stated before the campaign. Under the unscaffolded control the
      expected result is 0 of 30, and that comparison is the finding.
      *Met 2026-09-02 (D-131): N stated as 28 of 30 in the locked
      pre-registration; measured 30 of 30 under N, S2 and S4, 0 of 30
      under the transition-disabled control T0. The pre-registration
      recorded, before the campaign, that D-128's ceiling makes the
      unscaffolded field rich enough to materialize too, so the
      interpretable control is T0 rather than N; the scaffolds bring the
      first materialization forward by 1,000-2,000 ticks. Findings:
      `experiments/results/phase16-transition-findings.txt`.*
- [x] **C16.6 Multicellularity is reached, or reported as not reached.** The
      fraction of seeds in which any lineage exceeds one module, and the
      distribution of module counts over time, for scaffolded and
      unscaffolded conditions. **This is expected to return null and is
      stated so in advance.** A null here is a measured result about
      reachability, reported as such and not weakened after the fact.
      *Measured 2026-09-02 (D-131): the pre-registered expected null - 0
      of 30 in every arm, peak module count 1 wherever organisms existed.
      Reported as a birth-limit null: births occur but rarely (N median
      8.5 per world against ~22,800 materialized), which the
      pre-registration named ahead of time as the reason to expect. The
      named lever is Phase 19 (chemistry as food), which re-asks this
      criterion under a viable unicell ecology.*
- [x] **C16.7 No multicellularity mechanic exists.** A code review criterion
      stated as an acceptance criterion because it is the phase's central
      claim: no rule reads module count to grant, deny, reward, or penalize
      anything beyond the ordinary structural costs Phase 10 already
      specifies.
      *Met 2026-09-02 (D-130): the transition code reads no module count;
      the only new module-count reads are the two metrics gauges
      (`max_modules`, `multi_module_organisms`), which nothing in the tick
      reads. The independent review pass re-checks this claim.*
- [x] **C16.8 Determinism and fixtures.** Clean-process fixture replay; save
      round trip with a mid-transition world; transition disabled reproduces
      the Phase 15 fixture exactly.
      *Met 2026-09-02: `verify-phase16-determinism.sh` (two-process replay
      of `lifesim fixture --transition`, pinned constants; the
      `--transition-off` control replays and stays empty; the Phase 15
      fixture's constants unchanged), the mid-transition round trip with a
      hashed-counter mutation check in `phase16_transition.rs`, and
      verify-phase13/14/15 green after the admission refactor.*

## Test Plan

- Unit: threshold evaluation at boundaries; class-to-genome map at every
  registered class; ledger arithmetic including the rounding remainder.
- Property: conservation holds for arbitrary densities and class mixes.
- Determinism: C16.3 and C16.4 as automated tests; clean-process fixture;
  save round trip mid-transition.
- Integration: a materialized organism completes a full lifecycle; its death
  returns matter to the field; simultaneous multi-cell transitions.
- Neutrality: C16.2 as a direct A/B comparison test, not an inference from
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
| **Conservation defect at the conversion.** Two representations of the same matter converting under a threshold is the most defect-prone shape in the programme | C16.1 over 10^6 ticks; conservation by construction; the remainder convention is explicit rather than incidental |
| The threshold accidentally becomes an achievement that confers advantage | C16.2 tests neutrality directly by A/B comparison; C16.7 makes the absence of a mechanic an acceptance criterion |
| Materialization bursts blow the tick budget | Measured; deterministic capped deferral with a counter if needed |
| **The transition never occurs, or occurs and then multicellularity never does** | Expected. C16.5 and C16.6 are written so both are measurements. The unscaffolded control makes the negative interpretable rather than merely disappointing |
| A scaffolded success is reported as though it were unscaffolded | ADR-0018 requires the scaffold in every report; the intensity sweep makes a single-point result visibly weaker |
| The class-to-genome map quietly encodes a good starting organism | The map is versioned and reviewed; C16.2 neutrality catches an advantage; the synthesized organism is a one-module body with no special parameters |

## Rollback

One config section. Disabled, no transition occurs, `scratch` produces a
field-only world that stays valid and observable, and the Phase 15 fixture
reproduces exactly.
