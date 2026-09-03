# Phase 22: Lineages Under The Other Order

Status: planned 2026-09-03, not started. Policy versions: none new -
`lifesim-intake-order-v2` (Phase 21's probe), `lifesim-lineage-index-v1`
(Phase 20's census), event schema 13. Decisions: ADR-0035 (the lineage
endpoint and census), ADR-0036 (the intake order), ADR-0037 (this
phase's concrete design), ADR-0018 (a probe beside its control).

## Problem

Phase 20 found that multi-module lineages exist in the shipped physics
at a rarity set by one term: a second module arises only in born
organisms, and a born organism reproduces with probability ~0.07
before it starves (D-134). Phase 21 found what that term is made of:
the youngest eats last, and reversing the order lifts the born cohort's
median life tenfold (D-135). The lineage census over Phase 21's own
confirmatory - an observation the campaign was not pre-registered to
decide - shows what follows: under the shipped order 2 second-
generation two-module organisms in 1 of 30 worlds; under youngest-first
244 in 16 of 30, a largest per-world count of 77, on the same seeds.

That observation is the pilot of this phase and nothing more. The
claim - that the order in which co-located organisms eat is what
limits multicellular lineages in this ecology - needs a directed
contrast on seeds disjoint from every world that has been read, with
its endpoint, SESOI and bar fixed before it runs.

## Scope

- No new mechanism, record, census or format: Phase 20's lineage
  census, Phase 21's birth-site record and intake-order probe are the
  whole instrument. The one new artifact is the reduction script
  (`experiments/results/phase22-lineages-reduction.py`), which is
  checked before the lock by reproducing the pilot's numbers from the
  archived Phase 21 run and by refusing a manifest with a world's
  census line removed.
- **A pre-registered campaign**: O1 (the shipped intake order, the
  control) against O2 (youngest-first, the probe) on matched seeds
  disjoint from 20001..20060 and 21001..21060, 100,000 ticks, events on.
  **Fifty seeds per arm** - 22001..22052 without 22010 and 22028, which
  preflight refused; every other seed probed generable - because the
  endpoint is the rare one (ADR-0022's clause; Phase 20 invoked it for
  the same endpoint). Pilot seeds 22901..22904 (probed) are reserved
  but not needed: the Phase 21 confirmatory's lineage census is this
  phase's pilot, on seeds disjoint from these.
- The primary endpoint is Phase 20's, unchanged: second-generation
  multi-module organisms per world (born, two or more modules, a
  parent with two or more, inherited not fresh).

## Non-Goals

- No change to the shipped order, the duplication rate, the pairing
  gate, the field or the mouth.
- No claim about a body that divides labour, a third module, or a
  lineage that outlives its world: the endpoint is one inherited
  duplication across one reproduction, at the rate the physics gives.
- No reading of the observation above as a result.

## Acceptance Criteria

**Primary endpoint: C22.1.** Acceptance is conjunctive.

- [ ] **C22.1 Lineages under the other order (primary).** Worlds with
      at least one second-generation multi-module organism, O2 minus
      O1, seed-paired directed count (worlds where O2 has one and O1
      none), against a **bar of 15 of 50** fixed here: under no directed
      effect it has probability 0.002 even at Phase 20's shipped-order
      baseline of 0.16 worlds-with-any (and ~0 at Phase 21's 1 of 30),
      and it detects a treatment per-world probability of 0.45 with
      power 0.90 and 0.53 (the pilot's 16 of 30) with 0.99, so a null
      at the bar reads as "fewer than nine worlds in twenty under the
      probe hold a lineage the shipped order lacks" - and says nothing
      about rates near one in three, where power is only ~0.5 (0.35:
      0.52), which is stated so the null is not read as no effect; the birth-normalized rate beside it in both
      arms, because youngest-first also changes the birth count.
      **Expected, stated in advance**: met - the pilot shows 16 of 30
      against 1 of 30.
- [ ] **C22.2 The lineage's shape.** Per arm: the distribution of
      second-generation counts per world, the largest per-world second-generation count, the
      multi-module parents' fraction against the born cohort's, the
      compositions; descriptive.
- [ ] **C22.3 The cohort beneath it.** The born cohort's median
      lifespan per arm (Phase 21's census) reported beside the lineage
      count, so a reader sees the term and its consequence in one
      table.
- [ ] **C22.4 Determinism and neutrality.** No kernel change: every
      pinned fixture unchanged (verify 13-21); the campaign's two arms
      differ in exactly one hashed field.

## Test Plan

No kernel or census code; the campaign runs on Phase 21's base with a
new seed line. The reduction is new and is checked as the plan's Scope
says (pinned reproduction of the pilot; refusal on a removed line).
Pre-registration -> confirmatory.

## Benchmark Impact

None.

## Documentation Updates

`docs/22`, `docs/25` (the chain's next link), this plan's criteria,
`docs/19` and `planning/backlog.md` rows.

## Risks

| Risk | Mitigation |
|---|---|
| The observation is read as the result | It is named the pilot; the claim rests on disjoint seeds under a locked record |
| Youngest-first raises births as well as lineages | The birth-normalized rate is reported beside the count, as Phase 20 did |
| A reader takes the probe order for a recommendation | Both orders are equally unauthored (D-135); the shipped order stays shipped and the phase measures a consequence |

## Rollback

Nothing to roll back: no code changes.
