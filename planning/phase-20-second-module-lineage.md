# Phase 20: The Second Module - Why No Lineage

Status: planned 2026-09-03, not started. Policy versions
`lifesim-morphology-v1` (unchanged), event schema 12 (one additive
observation record), `lifesim-lineage-index-v1` (the census). Decisions:
ADR-0019 (the typed module registry and the founder-referenced
phenotype), ADR-0032 (the constant one-module map every materialized
organism starts from), ADR-0034 (the mouth), ADR-0035 (this phase's
concrete design), ADR-0018 (a probe arm is reported beside its control,
never alone), ADR-0016 (analysis observes).

## Problem

Phase 19 released the birth-limit null that Phase 16 had recorded: with
the mouth open a materialized unicell lives ~1,400 ticks instead of
~200, reproduces thousands of times per world instead of a handful, and
the first bodies above one module appear (17 of 30 worlds, none under
the v1 control). What Phase 19 also recorded, and read carefully, is
that in every one of those worlds it was **one two-module organism at a
time**, appearing late (median first sample 44,500 of 60,000 ticks) and
gone by the horizon in most worlds; over the 10^6-tick soak, with
149,352 births, a second module appeared exactly once. D-133 named the
open question and reserved it for a phase of its own: **why does a
second module never persist?**

The first draft of this plan asked whether the second module's *price*
is what stops it and proposed a uniform upkeep scale as the probe. The
arithmetic, checked before any code, says no, and the record starts
from that fact. A phenotype's basal cost is the config's basal rate
times the body's summed upkeep **relative to the founder body's**
(ADR-0019's re-centring: founder mass 2,400, upkeep 750, intake 1,000),
clamped to [0.6, 1.6]. A unicell (mass 800, upkeep 200) sits on the
floor: 200/750 = 0.27, clamped to 0.6. A second module of five of the
seven types - structural (300), sensory (350), digestive (400),
reproductive (450), storage (280) - leaves it on the floor; only a
motor (600 -> 0.8) or a neural module (700 -> 0.93) lifts it. Mass
rises (scale multiplier 0.63-0.83) but mass multiplies movement cost
with speed squared, and a body with no motor sits at the speed floor.
In the units the world actually charges (`physiology.basal_cost_milli_per_s`
100 at the campaign's tick length, times the clamped multiplier) a unicell
pays ~6 milli per tick against an intake capability of ~200 per tick
when fed; the largest second-module increment (neural, 0.6 -> 0.93)
is ~3 milli per tick, about 1.5 percent of that intake, and five types
add nothing. Meanwhile the module confers: a second gut raises intake 20 percent, a
storage module quintuples energy capacity (12,000 -> 60,000), a sensor
opens the sensing range, a motor gives four times the speed. **Under the
shipped physics a second module is nearly free and usually
beneficial.** Price is not what binds. A knob that scaled upkeep would
be a no-op under the floor (and the reference derives through the same
arithmetic), so it is not built.

What binds must then be in **transmission**: a body above one module
comes from Phase 9's structural duplication in a born organism's genome
(a materialized organism always starts from the constant one-module
map), and "one at a time, never a lineage" says either the organism
does not reproduce, or its children do not carry the module. Three
mechanisms are live and each leaves a different mark in the record:

- **Mate access.** Pairing refuses a candidate whose compatibility
  distance exceeds 0.5, half of which is the fraction of expressed
  network homology ids not shared. A duplication that changed the
  expressed network could isolate its carrier from every unicell mate.
  Mark: multi-module organisms with lifespans like their neighbours'
  and **no offspring**.
- **Viability at birth.** A child whose developed body fails validation
  (occupied cell, out of bounds, above the module cap, or a controller
  above its neural budget) is refused, and refusals are counted per
  world (`nonviable_bodies`, `refused_node_budget` in the manifest).
  Mark: multi-module parents **with** offspring, no multi-module
  children, refusal counters rising in those worlds.
- **Segregation.** A heterozygous duplication transmits to about half
  the offspring and may not express when it lands elsewhere after
  crossover (D-103's merged-rank crossover). Mark: multi-module parents
  with offspring, few or no multi-module children, refusals flat.

Two of the three were then measured before the pilot, by the
reproduction test this plan requires (`crates/sim-core/tests/
phase20_reproduction.rs`, the kernel's own recombine-mutate-develop
sequence with keyed draws, printed for the record): a two-module genome
found from the unicell at keyed draw 6,398 (a duplicated gut; the
unicell x unicell cross yields a second module in 0 of 1,000 draws)
reproduces against a unicell into **488 of 1,000 two-module children**
and against itself into 767 of 1,000 - Mendelian segregation of a
heterozygous duplication, a quarter lost when both parents carry it -
with **0 refused as non-viable** and 0-2 refused on node budget; and
its compatibility distance to the unicell is **0.0000** against the 0.5
pairing threshold. So segregation transmits the module to half the
children, viability refuses none, and mate compatibility bars nothing.
What is left is whether a two-module organism **pairs at all** in the
world - the pairing gate's other preconditions (energy above the
threshold, maturity, cooldown, a mate in range) and the late, grazed
epoch it is born into - and whether Phase 19's "one at a time" was a
500-tick sampling artefact that missed a parent-child overlap. The
event-based census answers both exactly, which is why the pilot decides
the branch and not this record.

The honest prior is docs/25's: unfavorable, and the likeliest result is
that the second module is rare (of order one per 10,000 births) and its
carriers few and late enough that lineages start and end inside the
noise - a fact about the ecology as built, which this phase measures
and names rather than tunes.

## Scope

- **One additive observation record**: `BodyComposition { id, counts:
  [u16; 7] }` (tag 30, event schema 12), emitted at every admission -
  birth and materialization - with the whole body's module counts by
  type, and again at growth completion when ontogeny is on. It does not
  depend on ontogeny: the Phase 14 `GrowthCompleted` record is emitted
  only on the ontogeny path, and every Phase 16 and 19 campaign ran
  with ontogeny off, so those logs hold **no body record at all** - a
  fact found while building the census, recorded here so no later
  reader looks for one. The census keys on this record. No rule reads
  it (ADR-0016; the Phase 17 neutrality test's cadence argument covers
  it).
- **One census command**: `lifesim lineage --manifest FILE`
  (`lifesim-lineage-index-v1`), per world: organisms whose body has two
  or more modules, their origin (born or materialized), completed
  lifespan against one-module born contemporaries, offspring, parents
  with offspring, **second-generation** multi-module organisms (a
  multi-module child of a multi-module parent), the composition of
  every multi-module body, the first multi-module tick and the last
  multi-module death. It decides nothing.
- **One fact test**: the clamp arithmetic above pinned as a test
  (founder reference 2,400/750/1,000; the unicell at 600/600/1,000; a
  second module of each type and the multiplier it lands on), so the
  physics the record reasons from is the physics that runs, and a
  registry change re-centres the phase's reading instead of silently
  invalidating it.
- **A pilot**, then a **pre-registered campaign** whose contrast is
  chosen by the decision tree below, written here before the pilot.
  Base: the Phase 19 v2 world with the record on, 100,000 ticks (the
  second module arrives late; the horizon must leave it time to
  reproduce), events on. **Fifty seeds per arm** (20001..20039 20041..20051 - 20040 refused
  at preflight; every other seed probed generable): C20.1 counts a rare outcome - the shipped world's baseline
  is expected at or near zero - so ADR-0022's rare-outcome clause (50,
  not 30) applies and is invoked here rather than argued around; the
  pre-registration carries a simulation-based power statement from the
  pilot's per-birth rate. Pilot seeds 20901..20904.
- Fixture `lifesim fixture --transition --coupled` at schema 14 prints
  the composition record count and the largest module count;
  `scripts/verify-phase20-determinism.sh`.

## The decision tree, declared before the pilot

The pilot's U1 census (four seeds, the shipped world) is read against
this tree and the pre-registration records which branch was taken and
why. The primary endpoint is the same on every branch.

- **Branch A - no offspring**: multi-module organisms have completed
  lifespans within 25 percent of their matched one-module cohort's and
  `multi_parents` is zero in every pilot world. Compatibility is ruled
  out by the reproduction test (distance 0.0), so the lever is the
  pairing gate's economics: contrast arm `physiology` pairing energy
  threshold lowered from the shipped 7,000 to 5,000 milli (an existing
  config field, hashed already; the two-module bodies' capacities are
  24,000-60,000 and the shipped threshold is absolute) against the
  shipped world. Expected: second-generation organisms appear under the
  lower threshold if the gate is what a two-module life never clears;
  if `multi_parents` is zero and the multi-module lifespans are SHORT
  (below 75 percent of the cohort's), the gate is not the reason and
  the phase reports the shipped world alone (as B/C) with the lifespan
  gap as the named fact.
- **Branch B - offspring, refused children**: `multi_parents` above
  zero in at least two pilot worlds, `second_generation` zero, and the
  manifest's `nonviable_bodies` or `refused_node_budget` higher in
  worlds with multi-module parents than without. No config lever is
  clean here; the phase adds a **deterministic reproduction test** that
  takes a two-module genome from a pilot snapshot, pairs it against a
  unicell genome and against itself for 1,000 draws, and counts the
  children by outcome (refused non-viable, refused budget, one module,
  two modules). The campaign runs the U1 world alone on 30 seeds with
  an equivalence bound (below) and the test's counts are the named
  mechanism.
- **Branch C - offspring, viable one-module children**: as B but the
  refusal counters flat. Same reproduction test; the mechanism named is
  segregation or non-expression after crossover, and the test's
  two-module fraction is the transmission rate.
- **Branch D - second-generation organisms exist**: the lineage endpoint
  is already met in the shipped world at this horizon and Phase 19's
  reading was a horizon artefact. The campaign runs U1 on 30 seeds and
  reports lineage lengths; the contrast becomes horizon (60,000 versus
  100,000 ticks) to show it.

If the pilot's four worlds produce no multi-module organism at all,
the pilot is extended to eight seeds once (recorded) before a branch is
read; if still none, the phase reports the appearance rate with its
bound and stops there.

**Read 2026-09-03 against the pilot** (`runs/phase20-lineage-pilot-
0x06fb2fdcf55662df`, four worlds, 100,000 ticks): 17 multi-module
organisms, all born, 16 of them a doubled gut and one gut-plus-motor;
`multi_parents` 0 in every world, so Branch A by its letter. But the
branch's lever was premised on a refusal that never happened - the
energy gate refused 0 pairings in all four worlds (capacity 0,
placement 0-1) - and the census's born-parent counts say what does
bind: born organisms reproduce (1,803 / 1,303 / 834 / 822 distinct
born parents; 72 percent of births have a born parent) but each one
rarely - about one in twelve, 8.3 percent pooled - with world-wide born median lifespans of
210-293 ticks against a trait-derived maturity of at least 400. A
two-module organism (lifespans 133-261, matched cohort 140-259) has the
reproductive prospects of any born organism, and 0 of 17 reproducing
has probability ~0.09 at the cohort's rate. Phase 19's "one at a time"
under-counted appearances about eightfold: two-module organisms live
~200 ticks and the series sampled every 500. The confirmatory
therefore runs the **shipped world alone** on 50 seeds
(`phase20-lineage-confirmatory-alone.campaign`) with the second-
generation rate predicted in advance from the cohort and an equivalence
bound - the one-arm shape - and the two-arm gate contrast is not run,
because a contrast on a gate that refuses nobody is a null bought in
advance. The unused campaign file is removed with this reading.

## Non-Goals

- No mechanism that makes a second module more likely, more heritable
  or cheaper: no per-module cost rule, no transmission bias, no change
  to the registry, the duplication rate, the pairing threshold (except
  as the pre-registered Branch A contrast, reported beside the shipped
  control), the influx cap or the mouth.
- No claim about multicellularity as a transition. A second-generation
  two-module organism is a lineage that carried a duplication across one
  reproduction; the record says that and no more.
- No new body type, sense or tissue rule; no config knob.

## Prerequisites

Phase 19 complete (D-133); the census tool (built, unit-tested,
mutation-checked, not yet reading the composition record).

## Determinism Notes

- The composition record is pushed inside `admit_schema2_child`, the
  one function births and materializations share, immediately after
  the organism's admission records, in entity-ID order like every other
  record; the reconciliation walk ignores it.
- Its counts are read from the body the world already holds for the
  organism (the morphology state's body list), never recomputed.
- No config change, no hash change, no ALIF change: at schema 12 a
  world with events on and off has one state checksum (the Phase 17
  neutrality shape), and every pinned fixture is byte-identical.
- Campaign worlds single-threaded (ADR-0023).

## Acceptance Criteria

**Primary endpoint: C20.1.** Acceptance is conjunctive; C20.2-C20.4 are
reported beside it and never rescue it.

- [x] **C20.1 Second-generation multi-module organisms (primary).**
      Per world, from `lifesim lineage`: born organisms with two or more
      modules whose parent had two or more **and whose module count does
      not exceed that parent's** - inherited, not a fresh duplication in a
      multi-module lineage (the child-minus-parent count difference, which
      the census computes wherever both records exist, is the guard). On
      Branch A: a seed-paired
      directed count, open threshold versus shipped, bar from the pilot.
      On Branches B and C: the shipped world alone on 30 seeds with a
      pre-registered **equivalence bound** - second-generation organisms
      per 10,000 births below a SESOI stated from the pilot's birth
      counts - so "no transmission" is distinguishable from
      "underpowered", plus the reproduction test's counts as the named
      mechanism. On Branch D: the count at both horizons. Whichever
      branch, the birth-normalized rate is reported beside the count.
      **Expected, stated in advance**: Branches B or C, a null under the
      bound, and a mechanism named by the test. *Revised at the pilot
      (recorded above): the shipped world alone, with the rate predicted
      from the cohort's own numbers - appearances per birth x the
      born-parent fraction x offspring per born parent x one half - so
      the confirmatory decides whether multi-module organisms reproduce
      like their cohort (the prediction holds), less (the second module
      costs reproduction), or not at all (a gate).*
      *DECIDED 2026-09-03 (`runs/phase20-lineage-confirmatory-0xa8d0b4c2ab68ba74`, 50 worlds, the shipped world alone): 127 second-generation organisms in 8 worlds, 1.943 per 10,000 births with an upper bound of 2.312 against the SESOI of 0.5 - the equivalence null is rejected and lineages exist; the cohort prediction 1.204 per 10,000 (78.7) is exceeded because lineages compound (seed 20024: 80 multi-module organisms from one founding at tick 54,996). Multi-module organisms reproduce like their cohort (5.5 vs 6.9 percent). Findings: `experiments/results/phase20-lineage-findings.txt`.*
- [x] **C20.2 What a second module costs, measured.** Within the shipped
      world, the completed lifespan of multi-module organisms against
      their **matched one-module cohort**: born one-module organisms
      whose admission tick lies within 2,000 ticks of a multi-module
      organism's admission in the same world (the census defines and
      computes it), with completed and censored counts for both groups
      beside the two lower medians. Descriptive, no bar: a multi-module
      organism arrives late into a grazed field and a late cohort is
      more censored, so the matching removes the epoch and the counts
      show the censoring; the arithmetic above predicts a difference near
      zero for five of seven types and the measurement tests it.
      *MEASURED 2026-09-03: median gap -23 ticks (range -229..+263, n = 45) between multi-module and matched-cohort lifespans - no systematic cost, as the arithmetic predicted.*
- [x] **C20.3 What the extra module is.** From the composition
      records: the composition multiset of every multi-module body per
      arm (a count array carries no order, so "the second module" is not
      a fact the record can name), and wherever a child's and a parent's
      records both exist, the child-minus-parent type-count difference -
      which type a duplication added - over multi-module children.
      Descriptive; no expectation is stated because any stated one would
      be a guess about which duplication is viable, which is what the
      census exists to replace.
      *MEASURED 2026-09-03: 221 of 238 multi-module bodies are a doubled gut, 17 a gut plus a structural module; added types over second-generation bodies digestive 111, structural 17; no body above two modules in any world.*
- [x] **C20.4 The starting fact and the reproduction test.** The
      pilot's U1 census recorded in the pre-registration before the
      lock, the branch taken and the tree it was read against; on
      Branches B and C the reproduction test's four counts over 1,000
      draws against a unicell and 1,000 against itself.
      *MET 2026-09-03: the starting fact is the pilot's census (`runs/phase20-lineage-pilot-0x06fb2fdcf55662df`: 17 multi-module organisms, all born, 16 gut+gut and 1 gut+motor, none reproducing; born parents 8.3 percent of births; born median lifespan 210-293 against maturity >= 400) and the reproduction test's counts (488/1,000 and 767/1,000 two-module children, 0 refused, compatibility 0.0). Phase 19's logs carry no body record - recorded as the trap.*
- [x] **C20.5 The clamp arithmetic is the physics that runs.** The fact
      test pins the founder reference, the unicell's multipliers and
      the seven two-module cases; a registry change that moves any of
      them fails the test and is recorded, never absorbed.
      *MET 2026-09-03: `tests/phase20_lineage.rs` pins the founder reference (2,400 / 750 / 1,000 / 1,000), the unicell's multipliers (600, 600, 1,000) and the seven two-module cases (five on the floor; motor 800; neural 933).*
- [x] **C20.6 Neutrality and determinism.** The composition record moves
      no state checksum (events on and off, one checksum); every pinned
      fixture byte-identical; the schema-12 coupled fixture replays
      across two processes and its record count equals its admissions;
      schema-12 logs verify and reconcile; schema-11 logs still decode.
      *MET 2026-09-03: one record per admission whose sum is the module count and whose living records match the max_modules gauge; no checksum moves (the schema-14 fixture's state equals the Phase 19 pin; events on and off, one checksum); a mid-run save round trip restores the same next records; a bodiless world emits none; `scripts/verify-phase20-determinism.sh` five clauses PASS; schema-11 logs decode unchanged (persist suite).*
- [x] **C20.7 Cost.** One record per admission: the log's growth per
      admission recorded (23 bytes: a one-byte tag, the u64 id and
      seven u16 counts, fixed width, no packing) and the Phase 19
      confirmatory's log size as the baseline; no tick-path change, so
      no benchmark re-run is owed beyond stating that.
      *RECORDED 2026-09-03: 23 bytes per admission (tag, u64 id, seven u16), no tick-path change; the pilot's four 100,000-tick logs are in the archive for the size.*

## Test Plan

- Unit: the record's counts equal the body's `count_of` for every type
  on a hand-built body; the registry-order test (index equals
  `ModuleType::id()`); the eventlog round trip and reconcile with a
  sample record; a schema-11 log without the tag decodes unchanged.
- Fact test: C20.5's numbers.
- Determinism: the fixture; events on versus off, one checksum; a save
  round trip mid-run restores the same future and the same next record.
- Integration: a coupled scratch world runs 3,000 ticks with the
  record on; every admission has exactly one record, in ID order, whose
  sum equals the organism's module count; both identities exact.
- Census: the four existing unit tests plus one reading module counts
  from `BodyComposition` where `GrowthCompleted` is absent, and one
  where both are present and agree; mutation-checked.
- Reproduction test (Branches B/C): deterministic, seeded, 1,000 draws
  each way, the four counts asserted to sum to the draws.
- Campaign: pilot -> branch read against the tree -> locked
  pre-registration with the reduction script committed first ->
  confirmatory.

## Benchmark Impact

No tick-path change. One event push per admission; the log grows by
the record size per admission and that number is recorded (C20.7).

## Documentation Updates

`docs/04-simulation-model.md` (an as-built note: the record and the
clamp fact), `docs/06-organism-model.md` (what a second module costs
under the founder-referenced clamps - the arithmetic, so the reader
does not repeat the first draft's mistake), `specifications/
event-schema.md` (tag 30, schema 12), `docs/22-decision-log.md`,
`docs/25` (the chain's next link, whichever way it falls), this plan's
criteria, `docs/19` and `planning/backlog.md` rows.

## Risks

| Risk | Mitigation |
|---|---|
| The pilot's four worlds show too few multi-module organisms to read a branch | The tree says what happens: one extension to eight seeds, recorded; then the appearance rate is the result |
| The branch read from the pilot is wrong and the confirmatory measures the wrong lever | The primary endpoint is branch-independent; the census on the confirmatory re-reads the marks; a wrong branch is reported as such with the marks beside it |
| The open-threshold arm (Branch A) is read as authoring reproduction | It is an existing config field varied as a probe against the shipped control, reported beside it (ADR-0018); it favours no morphology |
| The record grows the log | One record per admission, 23 bytes; recorded; campaigns that do not need it run with events off |
| 100,000-tick worlds at ~900 organisms take longer than the wall-clock budget | At Phase 19's rate (~1,600 ticks/s per world) 50-100 worlds are 9-18 min at 6 workers; the pilot measures it; anything longer runs on the server |

## Rollback

The record is observation only: removing it is deleting a tag; no rule
reads it. The census is a report. Nothing else changes.
