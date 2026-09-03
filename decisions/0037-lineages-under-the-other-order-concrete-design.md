# ADR-0037: Lineages Under The Other Order, Concrete Design (Phase 22)

Status: accepted 2026-09-03. The design authority is
`planning/phase-22-lineages-under-the-other-order.md`; this record pins
the concrete choices. It discharges the revisit condition D-135 wrote
down ("when the multicellular lineage question (D-134) is re-asked
under youngest-first"). Where this record and the plan disagree, the
disagreement is a defect in this record.

## What the increment is, and is not

One pre-registered campaign and nothing else: Phase 21's probe order
against the shipped order on fifty matched seeds, read by Phase 20's
lineage census. No kernel change, no record, no format, no census. It
does not recommend an order - both are equally unauthored (D-135) -
and it does not claim multicellularity in any sense beyond one
inherited duplication across one reproduction.

## The campaign, exactly

`experiments/phase22-lineages-confirmatory.campaign`: Phase 21's
confirmatory base (`phase21-born-cohort-confirmatory-gate.campaign`)
with `seeds 22001..22009 22011..22027 22029..22052` (fifty; 22010 and
22028 refused at preflight), `workers 6`, `check-interval 10000`,
events on, arms O1 and O2 with `set O2 physiology.intake_order
descending` and `vary physiology.intake_order`.

## The reduction, exactly

`experiments/results/phase22-lineages-reduction.py`: reads the
manifest, `lineage.txt` (`lifesim lineage --manifest`) and
`cohort.txt` (`lifesim cohort --manifest`); per world and arm prints
second-generation count, births, the rate per 10,000 births,
multi-module total and parents, the largest per-world second-generation count, born-parent
counts, the born median lifespan; then the seed-paired directed count
(O2 >= 1 and O1 == 0) against the bar, the count of worlds with any in
each arm, the median rate per arm, and the entity-cap gate. It counts;
it does not decide. It is committed with the locked pre-registration
before any world runs.

## The rules, exactly

Two, conjunctive, fixed from the pilot (the Phase 21 confirmatory's
lineage census, an observation) before this record: the directed count
of O2-only pairs at least 15 of 50 (an unconditional binomial tail
whose false-positive rate depends on the shipped arm's true rate -
0.002 at 0.16, restated by the reduction at the upper end of the
observed O1 rate; power 0.90 at 0.45, 0.99 at 0.53, ~0.5 at 0.35), and
the exact conditional sign test on the discordant pairs (O2-only
against O1-only, p = 0.5, one-sided) below 0.01, which assumes nothing
about either rate. The pilot: 16 against 1, p = 1.3e-4.

## Fixture and verify

None new; verify 13-21 assert that nothing pinned moves, which is the
whole determinism claim of a no-code phase.

## Consequences

- The chain's next link is decided by the cheapest experiment the
  record allows: a contrast the instruments already support.
- If met, the sentence the project can write is exactly what the
  rules decide and no more: on fifty matched seeds, reversing the
  intake order produced a second-generation two-module organism in at
  least fifteen worlds where the shipped order produced none, and the
  discordant pairs fell the probe's way beyond chance. The mechanism
  attributed is the one D-135 measured - the order acting through the
  born cohort's lifespan - and this contrast does not separate the
  order from any other route to a longer born life; C22.3 reports the
  born median beside the count so the reader sees the term and its
  consequence together, and the claim is about the order because the
  order is the only thing that differs between arms.

## As built

(Amended at the end of the phase with every divergence.)
