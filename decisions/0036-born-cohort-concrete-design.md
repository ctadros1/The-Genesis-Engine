# ADR-0036: The Born Cohort's Life, Concrete Design (Phase 21)

Status: accepted 2026-09-03. The design authority is
`planning/phase-21-born-cohort.md`; this record pins the concrete
choices the plan leaves open. It discharges the revisit condition D-134
wrote down ("when the born cohort's short life gets its own phase - it
is the rate-limiting term of every lineage"). Where this record and the
plan disagree, the disagreement is a defect in this record.

## What the increment is, and is not

One observation record at admission, one census, one fact test, and a
pre-registered campaign that measures what a born organism's life is
made of - the ground it is born on, the order it eats in, the company
it keeps - with the materialized cohort as the control. A second
intake order is designed here so that, if the pilot licenses it, it is
built to a record and not improvised; it is a probe, hashed as a policy
version, and the shipped order stays the default and the basis of every
claim.

It is not a rule about age, origin or need. The record reads; the
census counts; the probe permutes who among co-located organisms takes
first and grants nothing.

## The record, exactly

- `EventKind::BirthSite { id: u64, cell: u32, occupants: u16,
  maturity_ticks: u32, substrate_milli: [i64; 4], microbial_milli: i64,
  biomass_milli: i64 }`, tag 31, event schema **13**; 67 bytes fixed
  width. `maturity_ticks` is the admitted phenotype's own
  (`PendingChild.phenotype.maturity_ticks`, in scope at the emission
  site), so C21.3's maturity fraction is the real one; `substrate_milli`
  is indexed by the substrate constants (S_PRIMORDIAL 0, S_MONOMER 1,
  polymer 2, S_WASTE 3) so the census can separate food from what
  grazing itself deposits.
- Pushed in `admit_schema2_child` immediately after `BodyComposition`
  (same tick, same organism), for births and materializations alike.
  `cell` is the cell of the admitted position; `occupants` is the
  number of organisms in that cell before this admission - **not** read
  from the spatial buckets, which are built at `TickPhase::SpatialIndex`
  before movement and are stale by `lifecycle`, but from a per-cell
  count built once at the start of `lifecycle` from the positions the
  tick's feeding used and incremented by every admission that tick, so
  a second child born into the same cell sees the first (saturating at
  u16::MAX; one O(population) pass per tick, a rebuilt buffer, not
  state); the masses are the cell's four substrates
  (`chemistry.concentrations` at `cell * SUBSTRATE_COUNT + s`), the
  microbial total (`microbial.densities` summed over classes) and
  `biomass_milli[cell]`, read at admission - which is after the tick's
  feeding and field step and **before this admission's own debit**, so
  for a born organism the record is the cell as it finds it and for a
  materialized one the density it condensed from (its 4,000 lands after
  the record; the test asserts exactly that relation), and the plan says
  so.
- A world without chemistry emits the record with zero masses; a world
  without organisms in the cell emits `occupants` 0.
- Reconciliation ignores it; schema-12 logs decode unchanged; the
  round-trip sample count grows by three (three segments).

## The census, exactly

`lifesim cohort --manifest FILE` (`lifesim-cohort-index-v1`,
`sim-analysis`, decides nothing). Per world, from Birth/PairedBirth/
Materialized/BirthSite/Death (and PairedBirth's parent ids for
reproduction), two cohorts by origin:
- completed lifespans (lower median), completed and censored counts;
- the per-world Spearman rho (milli, ties averaged - `demography::
  spearman_milli`) over born organisms with a completed lifespan
  between lifespan and (a) **food at birth** = primordial + monomer +
  biomass and (b) occupants - computed **within fixed 20,000-tick
  admission blocks** (five per 100,000-tick run; a block with fewer
  than 30 completed born organisms is skipped and counted) and reported
  as the world's median block rho, so the run's own trend from empty
  ungrazed ground to a grazed crowded steady state cannot manufacture
  the sign pattern; the pooled whole-run rho beside it for the record;
  the partial of each exposure on the other within block as the rho of
  residual ranks; waste, polymer and microbial density reported as
  separate columns;
- the born and materialized birth-site food medians and their ratio
  (a magnitude reference: materialization selects the densest cells by
  construction); occupancy medians;
- the fraction of born organisms that lived to their own maturity
  (completed lifespan >= the record's `maturity_ticks`, or censored past
  it) and that reproduced (a parent id in a later birth), by food
  quartile and by occupant count 0, 1, 2, 3+.
Every count per world; the reduction pairs worlds by seed.

## The fact test, exactly

`crates/sim-core/tests/phase21_cohort.rs`: a planted coupled scratch
cell with two organisms, the elder admitted first, both hungry,
substrate for one appetite and no biomass: the elder's energy rises by
one appetite's yield and the newborn's by the remainder; then the same
with biomass instead of substrate. Under the probe order (if built) the
newborn takes first in both, and both identities are exact in both.

## The probe order, exactly (built only if the pilot licenses it)

`physiology.intake_order: IntakeOrder { Ascending (default),
Descending }`, codec id 0/1, hashed under `lifesim-intake-order-v2`
only when `Descending`. In this kernel age rank and entity-ID rank are
the same order (IDs are assigned at admission and never reused; two
organisms admitted in one tick tie in age and are broken by ID), so
"youngest first" is exactly "descending index", and both feeding
passes simply iterate `(0..population).rev()` under `Descending` -
every take is still made against the cell's state at the moment of the
take, the two passes stay coupled through `eaten[]` in the same order,
no queue exists, and at `Ascending` the loops are the shipped loops
byte for byte (the mutation that reversed them fails
`phase21_cohort.rs`, which is the proof it is the only thing that
changes). Building it costs **ALIF format 16** (the field rides in the
physiology config block; a retained format-15 writer refuses
`Descending` as "intake order"; format-15 files migrate as
`Ascending`) and its own fixture pin; the plan's "nothing pinned
moves" holds for the record alone, not for the probe. If the pilot
does not license it, this section is not built and the record says
so.

## Fixture and verify

`lifesim fixture --birthsite` (implies `--composition`), schema **15**:
the schema-14 line plus `birthsite_records`, `median_occupants_at_birth`
and `born_site_mass_median`. `verify-phase21`: two-process replay; the
record count equals admissions; the schema-14 state unchanged; the
Phase 16 fixture untouched; a schema-13 log verifies and the census
reads it; if the probe is built, its fixture pinned and the `Id` line
byte-identical to schema 15's.

## Campaign shape (pre-registered before any confirmatory world runs)

Phase 20's base (the shipped coupled world with the records on),
100,000 ticks, events on, **30 seeds** 21002..21031 (21001 refused at
preflight on 2026-09-03; every seed in 21002..21060 except 21052 and
21056 probed generable), pilot 21901..21904 (probed). The
pre-registration states the power basis for the shape that runs.
The pilot's census decides the licence by the plan's rule (median rho
for occupants at least as strong as for field mass); the
pre-registration records the shape, the SESOI in rho units (or in ticks
for the probe), the bars, and the expected outcome.

## Consequences

- One event tag, one analysis command, one fact test, one campaign; a
  policy version only if licensed. Nothing pinned moves.
- The rate-limiting term of every lineage gets a measured composition
  (place, order, company) and the next lever, if any, is named from
  the number rather than guessed.

## As built

Amended 2026-09-03 with every divergence so far; the campaign's
paragraph is appended when it is measured.

- **The record.** `EventKind::BirthSite { id, cell: u32, occupants:
  u16, maturity_ticks: u32, substrate_milli: [i64; 4], microbial_milli,
  biomass_milli }`, tag 31, `EVENT_SCHEMA_VERSION` 13, 67 bytes; pushed
  in `admit_schema2_child` right after `BodyComposition`. Occupants come
  from `World::cell_occupants`, a `Vec<u16>` rebuilt at the top of
  `lifecycle` from the tick's positions and incremented per admission
  (a same-tick sibling sees the first). A materialized organism's record
  shows the density it condensed from: its own debit - the base energy
  plus the remainder credit the lowest new id receives - lands after the
  record, and the test asserts that relation exactly rather than
  equality.
- **Tests.** `tests/phase21_birthsite.rs`: every admission has one
  record equal to the world's arrays (substrates, biomass, the born
  microbial density, the materialized bound), same-tick siblings count
  each other, no checksum moves, a save round trip restores the same
  next records. Two kernel mutations killed: no per-admission increment
  (the sibling clause fails), the wrong cell's biomass (the equality
  clause fails). `tests/phase21_cohort.rs` (C21.4): the elder takes the
  substrate first and the biomass first; reversing the two substrate
  loops fails the first clause and reversing the two biomass loops the
  second - a first mutation that missed the biomass loops (a different
  indent) survived uninformatively, and the exact loop heads are
  recorded in the test's doc.
- **Persist.** `TAG_BIRTH_SITE` 31, fixed width, negative masses refused
  on decode, reconciliation no-op, round-trip sample count 51 -> 54.
- **Fixture.** `--birthsite` (schema 15, implies `--composition`): the
  schema-14 line's identity fields plus `birthsite_records`,
  `median_occupants_at_birth`, `born_site_food_median` and
  `born_recorded`; its config, terrain and state equal the Phase 19
  pins. `verify-phase21` clauses 1-4 pass; clause 5 (a schema-13 log
  read by `lifesim cohort`) lands with the census.
- **The moved pin.** `verify-phase20`'s literal `event_schema_version`
  went 12 -> 13 with a note in the script; no config, terrain or state
  pin moved anywhere.
- **The census** (`crates/sim-analysis/src/cohort.rs`,
  `lifesim-cohort-index-v1`, built by a delegated agent to the
  definitions above; four unit tests; three mutations killed - waste
  folded into food, the pooled rho substituted for the block rho, a
  censored-before-maturity organism counted as matured): two choices
  the design left open are now fixed. Food quartiles use the lower-cut
  convention every median here uses (`sorted[(n-1)/4]`,
  `sorted[(n-1)/2]`, `sorted[3(n-1)/4]`, inclusive upper bounds).
  "Reproduced" is membership of the organism's id among any later
  birth's parents - unconditional membership, since a parent exists
  before it is named. The world line's split columns are
  `reached_maturity_food_quartile`, `reproduced_food_quartile`,
  `reached_maturity_occupants`, `reproduced_occupants` (four counts
  each, `/`-joined). `verify-phase21` clause 5 passes on a schema-13
  log.
- **The probe, built after the first pilot licensed it** (median block
  rho occupants -306 against food 274, |occupants| >= food in all four
  worlds): `sim_core::IntakeOrder { Ascending (default), Descending }`
  with `id()`, `from_id()`, `name()` and `index(step, population)`; the
  field `physiology.intake_order`; the four feeding loops (two biomass,
  two substrate) now run `for step in 0..population { let index =
  order.index(step, population); ... }` - at `Ascending` the identity,
  so the shipped loops are arithmetically the shipped loops; hashed
  under `lifesim-intake-order-v2` only when `Descending`, appended after
  every earlier block; the campaign field registry's choice
  `physiology.intake_order ascending|descending`; the fixture flag
  `--youngest-first` (schema 15 with `"intake_order":"descending"`,
  config 0xf3c1d35fa9b7c7da, state 0x3439df67a8cc1e88 at 4,000 ticks,
  pinned by `verify-phase21` clause 6, which also asserts the shipped
  line still says ascending and hashes as before). `tests/
  phase21_cohort.rs` gained the probe's clauses: under `Descending` the
  newcomer takes first in both passes with the identities exact, and
  the hash moves only for the probe. ALIF format 16 (the field appended
  as one byte at the end of the config body, a retained format-15
  writer refusing `Descending` as "intake order") is being built by a
  delegated agent to the pattern of formats 14 and 15; before it
  landed, the field-coverage sweep (`config_field_coverage.rs`, once
  taught the order's two choices) flagged exactly `physiology.
  intake_order` as set to descending and restored as ascending, and
  nothing else - the sweep D-065 and D-086 asked for, doing what it is
  for. Landed: `FORMAT_VERSION_16`, one byte appended after the
  format-15 consumption block (`FORMAT16_CONFIG_BYTES` 1), an unknown
  id refused as "intake order" on decode, `refuse_format16_state`
  threaded into all twelve retained pre-16 writers, retained
  `encode_snapshot_format15` / `decode_snapshot_format15`,
  `FORMAT15_TO_CURRENT` resolving the order to ascending; six tests in
  `tests/format16.rs` and the format chain and version-word tests
  extended; four codec mutations killed (skip the byte, read
  unconditionally, ignore the order in the refusal, drop the format
  guard). The field-coverage sweep passes with the field carried.
