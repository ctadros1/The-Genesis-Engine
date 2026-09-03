# Phase 19: Chemistry As Food (Coupling v2) And The Unicell Ecology

Status: planned 2026-09-02, not started. Policy version
`lifesim-chemistry-coupling-v2`. Specification:
`specifications/unicellular-regime.md` ("Reverse coupling": organisms
consume field chemistry and excrete into it). Decisions: ADR-0020,
ADR-0031 (which deferred this exact increment and named it), ADR-0032
(whose neutrality clause leans on its absence and says so), ADR-0018.

## Problem

Phase 16 delivers organisms from the field, and Phase 15 delivers the
field's side of the exchange (excretion and remains), but the organism
still cannot eat the substrate it condensed out of: ADR-0031's coupling
v1 shipped consumption from the chemistry field at zero and left the
biomass field as the only food. A materialized unicell is therefore a
gut standing in a sea of the one thing it cannot digest. The Phase 16
pilot measured the consequence: standing populations of order 150 held
by the influx cap against starvation, births in tens per world at most,
and a C16.6 null that is a birth-limit null (D-073's shape) rather than
a statement about morphology.

Three records point here. ADR-0031 names chemistry-as-food as "its own
future increment with its own conservation tests". ADR-0032 records that
C16.2's survival clause leans on coupling v1's narrowness ("the field
cannot favour anyone") and that "the follow-on phase that wires feeding
to the field must re-run the neutrality test with a coupled field".
D-128's ceiling means the field is rich everywhere; the missing piece is
the mouth.

## Scope

- A second food path beside biomass: an organism's feeding pass may take
  substrate (S_PRIMORDIAL and S_MONOMER) from its own cell's chemistry
  and assimilate it as energy, through the ledger, at a rate bounded by
  the body's digestive capability (the same `intake_milli` that bounds
  biomass feeding) and a configured conversion fraction. What leaves the
  field is exactly what the organism gains plus a counted metabolic loss
  deposited as S_WASTE - every term names its source and sink.
- The field identity gains the consumption term:
  `produced + deposited - materialized - consumed == chemistry +
  microbial`, enforced by `check_invariants` and re-derived by the
  reductions.
- C15.6's exchange test in its full form (excretion, remains,
  materialization and consumption), and C16.2's neutrality test re-run
  with the field coupled in both directions.
- The C16.6 question re-asked under a viable unicell ecology, with the
  v1 arm as the control.

## Non-Goals

- **No preference for any body.** The same feeding pass serves every
  organism; the digestive module's capability is the only thing that
  scales intake, and it already exists (Phase 10). If a reviewer can find
  code that reads "is this a unicell" or a module count to set an intake,
  that is a defect.
- No new substrate, no reaction added, no change to the abiotic table or
  the class registry.
- No tuning of the pairing threshold, maturity, or any life-history
  constant to make reproduction happen. If materialized lineages still
  do not reproduce when fed, that is the measurement.
- No claim about real metabolism. Two abstract substrates convert to an
  energy currency at a configured yield; that is the whole model.

## Prerequisites

- Phase 16 complete (materialization exists; the fixture and the
  campaign record are the baseline this phase is measured against).

## Determinism Notes

- Consumption runs inside the existing feeding pass in ascending entity
  ID over the organism's own cell; no draws; fixed point throughout. Two
  organisms in one cell take in ID order from what remains - stated, so
  the outcome cannot depend on traversal.
- A new chemistry ledger term `consumed_milli` (i128) and a config
  section `chemistry.consumption_fraction_q16` plus `chemistry.consumption_yield_q16`:
  ALIF format bump with a retained writer, refusal and migration on the
  Phase 14/15/16 pattern; hashed only when the fraction is nonzero (D-014),
  so every earlier fixture reproduces.
- No new RNG stream, no new event (consumption is a rate, censused not
  evented, on the feeding pass's existing terms).

## Acceptance Criteria

Conditions per ADR-0018 on matched seeds; the control arm is coupling v1
(consumption fraction zero) on the same seeds and the same scratch
world; a scaffold sweep is not needed (the neutral field is the
question, D-128 having made it rich enough).

- [ ] **C19.1 Conservation across the bidirectional exchange.** Both
      identities, with the consumption term, exact to the milli-unit over
      a 10^6-tick scratch run in which materialization, feeding from the
      field, excretion, death and remains all run. Extends C15.1 and C16.1
      to the closed loop.
- [x] **C19.2 Neutrality with a coupled field.** ADR-0032's obligation:
      the C16.2 tests re-run with both fractions and consumption nonzero -
      the materialized organism's rows equal the founder path's for the
      same genome, no provenance survives admission, and the relabelled
      twin shares one future. Any divergence is authored progress and
      fails the phase.
      *Met 2026-09-03: `phase19_consumption.rs` - the relabelled twin shares one future for 400 ticks while eating (consumed > 0 asserted), and the materialized organism under v2 has the founder path's phenotype and trait genes with parents [0, 0], depth 0, age 0.*
- [ ] **C19.3 The unicell ecology is viable, or reported as not.** The
      primary endpoint: median completed lifespan of materialized
      organisms (from the event log's `Materialized` and `Death` records)
      under v2 versus v1, seed-paired directed count, SESOI and bar
      stated in the pre-registration from a pilot on disjoint seeds.
      Expected direction: longer under v2 - a fed gut outlives a starving
      one - and if it does not, the finding is that intake cannot outrun
      basal cost at this body, which is a fact about the physics.
- [ ] **C19.4 Reproduction from the field.** Births per world under v2
      versus v1, seed-paired directed count with its own SESOI; the
      pre-registration states whether the shipped pairing threshold
      (7,000) is reachable from the field alone before any world runs,
      because a unicell holds 12,000 and starts at 4,000.
- [ ] **C19.5 C16.6 re-asked.** The fraction of seeds in which any
      lineage exceeds one module under v2 and v1, with the module-count
      distribution over time. **Expected to return null again, stated in
      advance**; reported with the birth counts beside it so a null with
      reproduction is distinguishable from the Phase 16 null without it.
- [x] **C19.6 The exchange test in full.** Excretion, remains,
      materialization and consumption each move a counted term, all four
      arms exercised alone and together, identities exact.
      *Met 2026-09-03: `every_exchange_arm_moves_its_own_counted_term_alone_and_together` - control moves nothing; excretion, remains (old-age deaths > 0), materialization and consumption each move only their own term; all four together; both identities exact at every 100-tick check in every arm.*
- [x] **C19.7 Determinism and fixtures.** Clean-process fixture replay
      with consumption live; consumption disabled reproduces the Phase 16
      fixture exactly.
      *Met 2026-09-03: `scripts/verify-phase19-determinism.sh` - two-process replay of `--transition --coupled` (config 0x7cfe66d39cda2e2b, state 0x2137b2286076cd63), every mechanism refused at zero, both identities from the printed totals, the v1 fixture equal to the Phase 16 pins, the Phase 15 fixture untouched.*
- [ ] **C19.8 Cost.** The consumption pass priced per organism and per
      tick against the v1 world; the field's cost stays per-cell.

## Test Plan

- Unit: the consumption arithmetic at boundaries (empty cell, more
  appetite than substrate, yield rounding); the field identity with the
  new term under adversarial concentrations.
- Property: conservation for arbitrary appetite/substrate pairs.
- Determinism: two organisms in one cell take in ID order (storage
  permutation of the organism arrays, restored, same result); the
  fixture; the disabled-reproduces-Phase-16 clause.
- Integration: the full exchange test; the neutrality tests re-run under
  v2 config.
- Campaign: pilot (four disjoint seeds) -> locked pre-registration ->
  confirmatory (30 seeds, v1 and v2 arms) with the reduction script
  committed first; soak for C19.1.

## Benchmark Impact

One more read and two writes per feeding organism per tick; measured
against the v1 world at 200 and 1,000 organisms. Benchmark schema 10
unchanged unless a named phase is added, which is not planned.

## Documentation Updates

`docs/04-simulation-model.md` (energy accounting gains the field intake
term), `docs/06-organism-model.md` (diet), `specifications/unicellular-
regime.md` (reverse coupling status), `specifications/world-save-format.md`,
`specifications/metrics-schema.md`, `docs/25-emergence-and-epistemic-
position.md` (the from-scratch chain's status), decision log, ADR-0031's
and ADR-0032's revisit conditions discharged.

## Risks

| Risk | Mitigation |
|---|---|
| A free lunch: field intake with no cap makes every organism immortal and the population pins on `max_entities` (the Phase 2 starvation trap in reverse) | Intake is bounded by the digestive capability and the cell's actual substrate; the campaign reports the death-cause mix and the population against capacity, and a world pinned on the guard is reported as such |
| Conservation defect across a two-direction exchange | The consumption term is counted on both sides in one operation; `check_invariants` every campaign interval; the soak |
| The unicell still cannot reach maturity even when fed, so C19.4 and C19.5 return nulls for a reason unrelated to food | C19.3 measures lifespan first; the pre-registration states the arithmetic (basal cost against maximum intake) before the run |
| Consumption quietly favours one body type | The pass reads the digestive capability every body already has; C19.2 and a code-review clause on the Non-Goals |

## Rollback

One config fraction. Zero, no substrate is ever taken, the field
identity's new term stays at zero, and the Phase 16 fixture reproduces
exactly.
