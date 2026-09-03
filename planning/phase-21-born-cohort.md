# Phase 21: The Born Cohort's Life

Status: planned 2026-09-03, not started. Policy versions: event schema
13 (one additive observation record), `lifesim-cohort-index-v1` (the
census); a second intake-order policy (`lifesim-intake-order-v2`) is
named as a probe the pilot may or may not license, and is not built
unless it does. Decisions: ADR-0034 (the mouth; two organisms in one
cell take in ID order), ADR-0035 (the record-at-admission pattern and
the lineage census), ADR-0036 (this phase's concrete design), ADR-0018
(a probe arm beside its control), ADR-0016 (analysis observes).

## Problem

Phase 20 found the term that limits every multi-module lineage, and it
is not the module. In the shipped coupled world a materialized organism
lives a median ~1,784 ticks (50-world range 1,613-12,292) and every
organism a median ~268 (192-332) - the born cohort being ~85 percent
of completed lives, its own median is close to that - against a
trait-derived maturity of 400-1,200 (`lifesim demography` over
`runs/phase20-lineage-confirmatory-0xa8d0b4c2ab68ba74`, saved there as
`demography.txt` and carried in the Phase 20 findings);
about one born organism in fourteen reproduces before it starves
(D-134). A second module arises only in born organisms, at ~3.6 per
10,000 births, so a lineage forms about once per six worlds per 100,000
ticks - the rarity is the born cohort's life times the appearance rate,
and the first factor is the large one.

Why does a born organism live a sixth as long as a materialized one?
Not its start: a child begins with the mean of its parents' investments
(3,000-6,000 milli each, the pairing overhead of 500 per parent paid
separately) while a materialized organism begins with 4,000. Three
things differ, and each is a fact of the shipped physics rather than a
rule anyone wrote for the purpose:

- **Where it starts.** A child is placed within 2 m of its parents'
  midpoint in 4 m cells - its parents' cell or the one beside it - and
  its parents are there because they have been eating there. A
  materialized organism appears where the microbial field is densest
  (the trigger cell). The born cohort may simply be born into grazed
  ground.
- **When it eats.** Both feeding passes iterate organisms in entity-ID
  order, and a cell's biomass and substrate are taken in that order
  (ADR-0034: "two organisms in one cell take in ID order"). A newborn
  holds the highest ID in the world: in its own birth cell it eats
  after its parents and after every older occupant. The youngest eats
  last, always.
- **Whom it eats beside.** A child shares its cell with the parents
  that made it and with every sibling born there, each ahead of it in
  the order.

These are one hypothesis about place and one about order, with
crowding as the term that couples them. The record settles which
carries the lifespan difference by measuring, and only then asks
whether an order that does not privilege age changes the number - as a
probe against the shipped order, never as a replacement.

The honest prior (docs/25): the difference is real and mostly place -
a grazed cell is a grazed cell regardless of order - and order is the
smaller term; the pilot may say otherwise.

## Scope

- **One additive observation record**: `BirthSite { id, cell,
  occupants, maturity_ticks, substrate_milli: [i64; 4], microbial_milli,
  biomass_milli }` (tag 31, event schema 13), emitted at every admission
  - births and materializations alike, through the same function - with
  the cell the organism starts in, the number of organisms already in
  that cell (all of them ahead of it in the order), the organism's own
  trait-derived maturity, and the cell's four substrates (primordial,
  monomer, polymer, waste - only the first two are food), microbial
  density and biomass at that tick. No rule reads it. The materialized
  cohort gets the same record: it is a **magnitude reference** for
  starting ground, not an exogenous control - a materialized organism
  appears where the field is densest by construction (ADR-0032's
  trigger), and the plan says so.
- **One census**: `lifesim cohort --manifest FILE`
  (`lifesim-cohort-index-v1`), per world and per cohort (born,
  materialized): completed lifespan medians; the per-world Spearman
  rank correlation (milli) between a born organism's completed
  lifespan and (a) its birth cell's total field mass and (b) its
  occupant count at birth; the born-versus-materialized ratio of
  birth-site field mass; the fraction of born organisms that reached
  their maturity and that reproduced, by field-mass quartile. Decides
  nothing.
- **One fact test**: a planted cell with an older organism and a
  newborn, both hungry, substrate for one appetite: the older takes
  its whole appetite and the newborn the remainder (Phase 19's ID-order
  test re-read as an age statement), and the same with biomass.
- **A pre-registered campaign** on the shipped world (Phase 20's base,
  **30 seeds** 21002..21031 - probed generable; 21001 refused - 100,000
  ticks, events on) after a four-seed pilot on 21901..21904; per-world
  rhos and medians are neither rare nor fixation-driven, so ADR-0022's
  floor of 30 applies and the pre-registration carries the power
  statement for whichever shape runs (a binomial count-of-worlds bar
  for the observational shape; a seed-paired sign test on the born
  median with the pair spread from the pilot for the probe). The pilot
  decides whether the **probe arm** is licensed (below).
- Fixture `--birthsite` (schema 15) and `verify-phase21`.

## The probe arm, and when it is licensed

If the pilot's per-world median block rho between lifespan and
occupants-at-birth is at least as strong in magnitude as that between
lifespan and food-at-birth (median over the four worlds, both in
milli), order is not the smaller term and the phase adds
`lifesim-intake-order-v2`: both feeding passes visit organisms in
**descending** index order instead of ascending - in this kernel age
rank and ID rank coincide, so this is exactly youngest-first up to
same-tick ties - everything else identical; a policy version, hashed,
the shipped order the default and the control; it costs ALIF format 16
and its own fixture, stated in ADR-0036. It favours no
morphology and no lineage; it moves who eats first among co-located
organisms from "oldest" to "youngest", which is as much a fact of the
tick as the shipped choice and no more authored. The confirmatory then
runs two arms on matched seeds: shipped order (O1) and youngest-first
(O2), primary endpoint the born cohort's median completed lifespan
seed-paired, SESOI and bar from the pilot. If the pilot does not
license it, the confirmatory runs the shipped world alone and the phase
is observational: the associations with their intervals and the
born-versus-materialized site contrast.

**Read 2026-09-03**: licensed. Median block rho over the four pilot
worlds: occupants -306, food 274, and |occupants| >= food in every
world; the within-block partials (occupants -219 after food, food 135
after occupants) say the same. The median born organism starts in a
cell with zero food against ~12,650 for a materialized one, and with
two or more occupants at birth almost none reaches maturity. The probe
is built; a second four-seed pilot under both orders supplies the pair
spread the pre-registration needs (the first could not, having no probe
arm) before the lock.

**The second pilot, read 2026-09-03** (`runs/phase21-born-cohort-pilot-
gate-0xdcc22be2abd31f7e`): under youngest-first the born median
lifespan rises from ~210 to ~2,160 ticks in every pair (+1,846 to
+2,369), the materialized median falls by ~545, born organisms
reaching maturity go from ~1,600 to ~9,600 per world, and both
associations vanish. The plan's expectation that place carries most of
it was wrong; the order is the term. The pre-registration records the
revision with its reason and is locked on the probe shape: SESOI 971
ticks (half the median pair difference, by the rule fixed before the
pilot), bar 22 of 30.

## Non-Goals

- No change to placement, investment, maturity, the pairing gate, the
  field's production or the mouth.
- No rule that reads age, origin or module count to grant food; the
  probe's order is a permutation of co-located organisms, not a share.
- No claim that lifespan "should" be longer; the phase measures what
  the born cohort's life is made of and reports the number the order
  moves, if any.

## Prerequisites

Phase 20 complete (D-134); the record-at-admission seam
(`admit_schema2_child`) and the persist codec's tag pattern.

## Determinism Notes

- The record reads the cell's field masses from the world's own arrays
  at admission and counts occupants from a per-cell count built once at
  the start of `lifecycle` from the positions feeding used (the spatial
  buckets are built before movement and would be stale), incremented
  per admission; one pass per tick, nothing saved.
- The probe order, if built, sorts a cell's co-located indices by
  (age, ID) before the take - a total order, so the result is a
  function of the world and never of storage; the campaign fixture pins
  it.
- No config, hash or ALIF change for the record; the probe's policy
  version enters the hash only when selected.

## Acceptance Criteria

**Primary endpoint: C21.1.** Acceptance is conjunctive; C21.2-C21.4
are reported beside it and never rescue it.

- [ ] **C21.1 What the born cohort's life is made of (primary).**
      Observational shape: per world, the Spearman rho (milli) between
      a born organism's completed lifespan and the **food** in its birth
      cell (primordial + monomer + biomass; waste, polymer and microbial
      density reported separately), and between lifespan and occupants
      at birth - each computed within fixed 20,000-tick admission blocks
      and taken as the world's median block rho, because the run moves
      from empty ungrazed ground to a grazed crowded state and a pooled
      rho would carry that trend as if it were place or order; the count
      of worlds where each median block rho clears its SESOI - half the
      first pilot's smallest magnitude, 128 for food and 140 for
      occupants - against a bar of 22 of 30 (alpha 0.008), all fixed in
      the pre-registration from the first pilot before the second. Probe
      shape (licensed): the born cohort's median completed lifespan
      under youngest-first minus the shipped order, seed-paired directed
      count against a bar of 22 of 30 fixed before the second pilot
      (alpha 0.008; power 0.87 at a per-pair clearing rate of 0.8) and a
      SESOI in ticks set by the pre-registration's rule (the larger of
      20 ticks and half the second pilot's median pair difference); the
      materialized cohort's lifespan reported beside it (it should not
      move much - a materialized organism is rarely the youngest in its
      cell). **Expected, stated in advance**: place carries most of it
      (field-at-birth rho positive and large, occupants rho negative and
      smaller); if the probe runs, the born median rises but stays well
      below the materialized median.
- [ ] **C21.2 The site contrast.** Born-versus-materialized birth-site
      field mass per world (medians and their ratio), with the
      occupancy distributions; descriptive.
- [ ] **C21.3 Maturity and reproduction by site.** The fraction of born
      organisms living to their **own** trait-derived maturity (the
      record carries it) and reproducing, by birth-site food quartile
      and by occupant count; descriptive.
- [x] **C21.4 The order fact.** The kernel test: the older co-located
      organism takes first in both passes; under the probe order (if
      built) the youngest takes first and the identities are exact.
      *MET 2026-09-03: `tests/phase21_cohort.rs` - an elder (ID 1, aged 30) and a newcomer (ID 2, aged 0) in one cell, both hungry: the elder takes planted substrate first and planted biomass first; reversing the two substrate loops fails the first clause, reversing the two biomass loops the second (mutation-checked).*
- [ ] **C21.5 Neutrality and determinism.** The record moves no
      checksum: every pinned fixture's config hash, terrain checksum and
      state checksum unchanged (the schema-14 `--composition` line's
      literal `event_schema_version` moves 12 -> 13 and
      `verify-phase20`'s pin is updated and recorded, which is a text
      change, not a state change); the schema-15 fixture replays;
      schema-13 logs verify and reconcile; if the probe is built, its
      fixture is pinned and the `Ascending` line is byte-identical to
      schema 15's.
- [x] **C21.6 Cost.** The record's bytes per admission and the spatial
      count's cost per admission, recorded.
      *RECORDED 2026-09-03: 67 bytes per admission (tag, u64 id, u32 cell, u16 occupants, u32 maturity, four i64 substrates, i64 microbial, i64 biomass); the occupant count is one O(population) pass per tick at the top of `lifecycle` and one increment per admission.*

## Test Plan

- Unit: the record's fields equal the world's arrays at admission on a
  planted world; the codec round trip; a schema-12 log decodes.
- Fact test: C21.4.
- Determinism: fixture; events on and off, one checksum; save round
  trip mid-run.
- Census: hand-written event vectors for each statistic, including the
  rank correlation's ties, mutation-checked.
- Campaign: pilot (four disjoint seeds) -> the licence decision read
  against this plan -> locked pre-registration with the reduction
  committed -> confirmatory.

## Benchmark Impact

One record per admission; one spatial count per admission (the index
the tick already built). If the probe is built: one sort of a cell's
occupants per feeding pass - priced against the shipped order on the
Phase 19 benchmark's populated arm.

## Documentation Updates

`docs/04`, `docs/06` (the youngest eats last - a fact of the tick, and
what it costs), `specifications/event-schema.md` (tag 31, schema 13),
`scripts/verify-phase20-determinism.sh` (its event-schema pin 12 -> 13,
recorded in the decision log), `docs/22`, `docs/25`, this plan's
criteria, `docs/19` and `planning/backlog.md` rows.

## Risks

| Risk | Mitigation |
|---|---|
| The record's field masses are read after the tick's feeding, not before | The record is emitted at admission inside `lifecycle`, after feeding; the plan says which and the census reads it as "the cell as the newborn finds it" |
| The two associations are collinear (grazed cells are crowded cells) | Both are reported, with the partial of each on the other; the probe arm separates order from place causally if licensed |
| Youngest-first is read as authoring survival | It is a permutation of co-located organisms, hashed as a policy version, run only as a probe beside the shipped order, and it favours no morphology; the shipped order stays the basis of every claim |
| A rank correlation over ~13,000 born organisms per world is trivially "significant" | The decision is on a per-world rho against a SESOI in rho units, the world being the unit (ADR-0022), never on a p-value over organisms |

## Rollback

The record is observation only; the probe is config-gated and inert
at the default. Removing both is deleting a tag and a policy branch.
