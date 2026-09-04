# ADR-0038: The Order On The Field's Terms, Concrete Design (Phase 23)

Status: accepted 2026-09-03. The design authority is
`planning/phase-23-the-order-on-the-fields-terms.md`; this record pins
the concrete choices. It puts the question D-136 wrote down as its
revisit condition - does any production rate make a cell refill within
the tick so the order stops mattering - on the branch where a dose past
one take runs inside the entity cap, and says so if none does. Where
this record and the plan disagree, the disagreement is a defect in this
record.

## What the increment is, and is not

One pre-registered seven-arm campaign and its reduction, and one
experiment-harness change that the campaign's cap gate needs. The
field's own production constant is swept fourfold under the shipped
intake order to a top dose past one take, and the other order (Phase
21's probe) is run at the shipped dose and at the top dose, all read by
Phase 21's cohort census and Phase 20's lineage census. No kernel,
record, census or format. It does not tune the field: the shipped
production stays shipped, the doses are probes beside their control
(ADR-0018), and the question is when the order stops mattering, stated
in the field's units.

## The arithmetic, stated before the run

Production is `production_milli_per_step` per cell per field step,
weighted per cell to a whole-world total of exactly production x cells
per step (ADR-0031), at `field_steps_per_tick` 1 in the campaign base.
A fed unicell's intake capability is 2,000 milli per second at a
100 ms tick, 200 per tick, and the mouth may fill the whole of it from
substrate (coupling fraction Q16_ONE). At the shipped 2 milli per
step, a cell an elder has emptied refills at one percent of a single
take per tick; at 8, four percent; at 32, sixteen; at 128, sixty-four
percent - still under a take; at 256, 128 percent - the first rung at
which a cell refills within a tick for one eater; at 512, 256 percent -
two eaters' worth. A newborn's cell refills for it only after every
elder ahead of it in the order has taken, so "refills within a tick"
means one take per elder sharing the cell; Phase 21's born occupants
median at the shipped dose is 1, so P128 is the rung at which the
arithmetic first says the order should stop mattering for the median
born site, and P256 the rung with a margin. Nothing bounds the knob
above (the config rejects only a negative value; the field accumulates
in i64), so both rungs are expressible; whether they run inside the
entity cap is what the pilot decides.

## The harness change, exactly

`crates/sim-experiment/src/manifest.rs`: the run line gains
`transition_materialized`, `transition_deferred_cap` and
`transition_deferred_capacity`, the world's own Phase 16 counters,
rendered after `refused_node_budget`, parsed as zero when absent, wired
in the scheduler from the metrics snapshot. `capacity_rejections`
counts only births refused at the entity cap; the materialization path
defers instead, on two counters of its own, and neither reached any
campaign artifact before this. Test:
`manifest_transition_columns_are_the_worlds_counters` (each column
against a solo run's metrics, under a cap of two and a cap out of
reach). Every archived manifest still parses.

## The campaign, exactly

`experiments/phase23-production-confirmatory.campaign`: Phase 22's base,
`seeds 23001..23010 23012..23031` (thirty), `workers 10`,
`check-interval 10000`, events on, field series every 1,000 ticks.
Arms: P1 (no set line: the shipped 2, the control), P4 `set P4
chemistry.production_milli_per_step 8`, P16 (32), P64 (128), PT (256
or 512, the pilot's licence), P1D `set P1D physiology.intake_order
descending`, PTD (PT's production and the descending order); `vary
chemistry.production_milli_per_step`, `vary physiology.intake_order`.
The pilot (`phase23-production-pilot.campaign`, seeds 23901..23904,
`workers 8`) runs P1, P4, P16, P64, P128, P256, P1D, P128D, P256D and
decides three things by rules stated before it runs: the top dose is
the highest of P256, P128 at which all four worlds under both orders
are generable and none trips the entity-cap gate (if neither, PT is
P64, and the phase reports that D-136's question cannot be put inside
the cap); the SESOI is the larger of 20 ticks and a tenth of the
pilot's median born median over the PTD worlds; the cost per world at
every rung is recorded. A dose that caps or fails preflight is reported
and dropped before the lock, never after.

## The endpoint, exactly

Per world, `lifesim cohort`'s `born_median_lifespan_ticks`. Every
contrast is seed-paired on the born median; the bar is 22 of 30 (alpha
0.008 under no directed effect; power 0.87 at a per-pair clearing rate
of 0.8); intervals are 95 percent percentile intervals of the median
paired difference from 10,000 seeded resamples (seed 23), so the same
numbers always give the same interval.

- C23.1 (primary): PT against P1, the count of pairs where PT's born
  median exceeds P1's by the SESOI, against the bar; P4, P16, P64
  against P1 reported the same way.
- C23.2: PTD against PT, three readings stated in advance - (i) the
  directed count at or above the bar: the order still matters; (ii)
  the interval within plus or minus the SESOI: the order stopped
  mattering; (iii) neither: undecided. P1D against P1 the same way.
- C23.3: PT against P1D, reached when the interval's lower bound is at
  or above minus the SESOI; P1D's median read against 2,191 (Phase 21,
  30 seeds) and 2,172.5 (Phase 22, 50 seeds).

## The reduction, exactly

`experiments/results/phase23-production-reduction.py`: reads the
manifest (ticks, births, population, capacity rejections, the three
transition columns), each world's `.alfd` field series (final chem plus
microbial as the field mass, final materialized, the field identity and
the sample count checked at every row), `cohort.txt` and `lineage.txt`;
refuses a missing line, a manifest without the transition columns (the
`--pin-archive` flag tolerates their absence for the archive pin only
and says so in the header), a materialized count that differs between
the cohort census, the manifest and the field series, a broken identity
or a short series. Per world and arm it prints the born and materialized
medians, materialized count, births, population, field mass, born-site
food and occupants, second-generation and multi-module counts, the
entity-cap gate (final population at the cap, or any capacity
rejection, or any materialization deferred for capacity) and the
per-tick deferral beside it; then C23.1, C23.2 and C23.3 as above. It
counts and does not decide. Pinned before the lock on the Phase 21
confirmatory archive
(`runs/phase21-born-cohort-confirmatory-gate-0xca1805044815a9f2`, O1 as
the control and O2 as the order arm): born medians 222 against 2,191,
materialized medians 1,752.5 against 1,190.5, materialized counts 2,528
against 2,525.5, field mass 171,055,313.5 against 112,578,576.5 milli,
born occupants 1 against 1, births 13,044 against 11,914, population
452 against 750.5; the order contrast 30 of 30 pairs above a 20-tick
SESOI, 0 of 30 within it, delta median 1,969, interval [1,944, 2,007];
worlds with any second-generation organism 1 against 16; the cap gate
0 of 30 in both; the field identity exact on all 60 series - and it
refuses the archive read without the pin flag (60 refusals) and a
cohort line removed (1 refusal).

## Fixture and verify

None new; verify 13-21 assert that nothing pinned moves. The manifest
change is covered by its unit tests, not by a fixture: no verify script
pins manifest text.

## Consequences

- The question of the order is answered in the field's units, with the
  other order beside the shipped one at the dose where the arithmetic
  says it should stop mattering: either the two orders read the same
  there (equivalence, stated), or the order still moves the born
  cohort past one take (the count), or the campaign cannot decide
  (said so) - and if no dose past a take runs inside the cap, that is
  the finding.
- The shipped constants stay shipped.
- Every campaign manifest from here on carries the transition's
  counters, so a materialization-side cap is visible to every later
  reduction.

## As built

(Amended at the end of the phase with every divergence.)
