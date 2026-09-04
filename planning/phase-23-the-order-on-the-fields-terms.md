# Phase 23: The Order On The Field's Terms

Status: planned 2026-09-03, not started. Policy versions: none new -
the shipped intake order and its Phase 21 probe
(`lifesim-intake-order-v2`), `lifesim-chemistry-v1`'s production knob
(`chemistry.production_milli_per_step`, swept once before in Phase 15,
D-128), Phase 21's birth-site record and cohort census, Phase 20's
lineage census. Decisions: ADR-0031 (the field and its production),
ADR-0036 (the order), ADR-0037 (lineages under the other order),
ADR-0038 (this phase's concrete design), ADR-0018 (a probe beside its
control).

## Problem

Phases 21 and 22 found that the shipped intake order - the youngest
eats last - is a sufficient cause of the born cohort's short life and
of the rarity of multi-module lineages, and that a permutation of who
eats first removes both (D-135, D-136). The record kept the shipped
order, because both orders are equally unauthored and choosing between
them is not a question physics answers. What physics can answer is
why the order matters at all: the field produces 2 milli per cell per
step at one step per tick, and a fed organism takes up to 200 per
tick, so a cell an elder has emptied refills at one percent of a
single take per tick and a newborn beside it finds nothing for a
hundred ticks. The order matters because the refill is slow against
the take.

So the question this phase puts, on the field's terms: **at what
production does the order stop mattering?** D-136 wrote the revisit
condition down in exactly those words - does any production rate make
a cell refill within the tick so the order stops mattering - and the
direct test of "the order stops mattering at a dose" is the two orders
side by side at that dose, on the same seeds. A dose sweep under the
shipped order says how far production alone carries the born cohort;
the other order at the top dose says whether the order still moves it
there; the other order at the shipped dose is the number the doses are
read against, measured on the same seeds instead of quoted from
another campaign.

The honest prior: production raises every cohort's life together and
the born cohort's relative disadvantage - eating last - persists at
every dose below a cell refilling within a tick; past that point it
should shrink, but the cell a newborn shares with more than one elder
still empties before the newborn eats, so the order may keep mattering
above one take. The doses past a take may change the ecology beyond
recognition (more materialization, a denser field, the cap), which the
record must report rather than hide - and if no such dose runs inside
the entity cap, the record says the question cannot be put there.

## Scope

- No new mechanism, record, census or format. The instrument is Phase
  21's cohort census and Phase 20's lineage census; the knobs are
  `chemistry.production_milli_per_step` and Phase 21's order probe.
- **One experiment-harness change**: the campaign manifest's run line
  gains the transition's three counters (`transition_materialized`,
  `transition_deferred_cap`, `transition_deferred_capacity`). Without
  them the entity-cap gate could see only births refused at the cap;
  a dose whose cap binds through materialization alone was invisible.
  Additive columns, absent-tolerant on parse, every archived manifest
  still parses.
- **A pre-registered campaign of seven arms** on 30 matched seeds
  disjoint from every world read so far (23001..23031 without 23011,
  which preflight refused; the pilot runs 23901..23904), 100,000 ticks,
  events on. Under the shipped order: P1 (2, the shipped production,
  the control), P4 (8), P16 (32), P64 (128 - 64 percent of a take per
  tick) and **PT, the top dose past one take** - P128 (256) or P256
  (512), whichever is the highest at which every pilot world is
  generable and none trips the entity-cap gate. Under the other order
  (youngest first, `physiology.intake_order descending`): **P1D** at
  the shipped production (Phase 21's O2 on these seeds - the in-campaign
  reference) and **PTD** at the top dose (the order contrast on the
  field's terms). A nine-arm, four-seed pilot on disjoint seeds licenses
  the top dose, calibrates the SESOI and measures the cost per world.
- **30 seeds per arm** (ADR-0022: the primary endpoint is a per-world
  median, neither rare nor fixation-driven); 210 worlds.

## Non-Goals

- No change to the order, the mouth, the transition, the duplication
  rate or the pairing gate.
- No claim that a production rate is "right": the field's shipped
  constant stays shipped; the phase measures a dose-response and
  names the dose, if any, at which the order stops mattering.
- No reading of a higher dose's lineage count as multicellularity in
  any broader sense than Phases 20 and 22 allowed.

## Acceptance Criteria

**Primary endpoint: C23.1.** Acceptance is conjunctive; C23.2-C23.6
are reported beside it and never rescue it. Every count is seed-paired;
the SESOI, the bar, the interval and the readings are fixed in the
pre-registration before any confirmatory world runs.

- [ ] **C23.1 The dose-response of the born cohort's life (primary).**
      The born cohort's median completed lifespan per world (Phase
      21's census) at the top dose against the control: the count of
      pairs where PT clears P1 by the SESOI, against 22 of 30 (alpha
      0.008; power 0.87 at a per-pair clearing rate of 0.8), with the
      median pair difference and its bootstrap interval; every
      intermediate rung (P4, P16, P64) reported the same way.
      **Expected, stated in advance**: met at every rung - production
      raises every life.
- [ ] **C23.2 The order at the top dose (D-136's question).** PTD
      against PT, seed-paired, with three readings stated in advance:
      (i) *the order still matters* - the directed count of pairs where
      PTD clears PT by the SESOI is at or above 22 of 30; (ii) *the
      order stopped mattering* - the bootstrap interval of the median
      paired difference lies within plus or minus the SESOI
      (equivalence), with the count of pairs within it reported; (iii)
      neither - undecided, reported as such. P1D against P1 reported
      the same way (Phase 21's contrast replicated on these seeds).
      **Expected**: (i) or (iii) - the effect shrinks past a take but
      the newborn's cell still empties ahead of it.
- [ ] **C23.3 The reference, on the same seeds.** PT against P1D: the
      shipped order at the top dose reaches what the other order
      reached at the shipped dose when the bootstrap interval's lower
      bound is at or above minus the SESOI (non-inferiority); the count
      of pairs at or above the reference less the SESOI reported. P1D's
      born median is read against Phase 21's 2,191 (30 seeds) and Phase
      22's 2,172.5 (50 seeds), the reference's own seed-set spread,
      declared. **Expected**: not reached.
- [ ] **C23.4 The lineages beneath it.** Per arm, worlds with any
      second-generation two-module organism and the pooled count
      (Phase 20's census); descriptive, read against Phase 22's 8 of
      50 and 36 of 50.
- [ ] **C23.5 The ecology at each dose.** Per arm: population,
      materialized count (the cohort census's completed plus censored,
      checked equal to the manifest's `transition_materialized` and the
      field series' final `materialized`), births, the field mass (the
      field series' final chem plus microbial), born-site food and born
      occupants medians (the birth-site record), and the entity-cap
      gate - final population at the cap, or any capacity rejection,
      or any materialization deferred for capacity - with the per-tick
      materialization deferral beside it; descriptive, so a dose that
      changed the world beyond recognition is reported as such.
- [ ] **C23.6 Determinism and neutrality.** No kernel change: every
      pinned fixture unchanged (verify 13-21); the manifest's new
      columns round-trip with distinct values, parse as zero when
      absent, and every archived manifest still parses; the arms differ
      in exactly the two varied fields.

## Test Plan

The manifest columns: `manifest_transition_columns_are_the_worlds_counters`
runs a scratch world tuned to materialize from its first check under
two entity caps and checks each column against a solo run's metrics -
the capacity deferral fires at a cap of two and stays zero with the cap
out of reach while the per-tick deferral counts, which tells the two
deferral wires apart; the round-trip guard's distinct values cover the
three columns; `every_archived_manifest_still_parses_at_this_version`.
The reduction (new) is pinned before the lock by reproducing Phase 21's
confirmatory from its archive - born medians, materialized medians and
counts, field mass, the 30 of 30 order contrast with its interval - and
by refusing a cohort line removed, a manifest without the transition
columns, a field series with a broken identity or a short sample count.
Pilot -> locked pre-registration -> confirmatory.

## Benchmark Impact

None in the kernel. A P256 world produces 256 times the shipped field
mass and may run far slower per tick; the pilot measures the cost per
world at every rung.

## Documentation Updates

`docs/22`, `docs/25`, this plan's criteria, `docs/19` and
`planning/backlog.md` rows; ADR-0038 as built.

## Risks

| Risk | Mitigation |
|---|---|
| A rung past one take is ungenerable or hits the entity cap | The pilot licenses the top dose by a rule stated before it runs; a capped arm is reported, never dropped silently; if no rung past a take runs inside the cap, PT is P64 and the record says D-136's question cannot be put inside the cap |
| The dose raises materialization and births so much that the born cohort is a different population | C23.5 reports the ecology at each dose beside the endpoint, from the field series and the manifest's transition columns, not from a proxy; the pre-registration names this confound |
| The reference is a number from another campaign's seeds | P1D measures it on these seeds; the cross-campaign spread (2,191 against 2,172.5) is declared as the reference's own uncertainty |
| The phase is read as tuning the field to produce lineages | The shipped production stays shipped; the doses are probes beside their control (ADR-0018) and the question is stated as "when does the order stop mattering", not "how to make lineages" |

## Rollback

The manifest columns are additive and absent-tolerant; removing them
restores the previous run line exactly. Nothing else changes in code.
