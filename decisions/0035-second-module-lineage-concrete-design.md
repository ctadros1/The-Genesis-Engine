# ADR-0035: The Second Module - Why No Lineage, Concrete Design (Phase 20)

Status: accepted 2026-09-03. The design authority is
`planning/phase-20-second-module-lineage.md` and ADR-0019 (the typed
module registry and the founder-referenced phenotype); this record pins
the concrete choices the plan leaves open so the implementation cannot
pick them silently. It discharges the revisit condition D-133 wrote down
("when an endpoint defined on persistence is pre-registered"). Where
this record and the plan disagree, the disagreement is a defect in this
record.

## What the increment is, and is not

Phase 19 left one question open: bodies above one module appear once
reproduction runs, one organism at a time, and no multi-module lineage
establishes itself. The arithmetic checked before this record (plan,
"Problem") rules price out - under ADR-0019's founder-referenced clamps
a second module of five of seven types costs nothing in upkeep and
confers capability - so the phase measures **transmission**: whether a
multi-module organism reproduces, whether its children are admitted,
and whether they carry the module. It adds one observation record, one
census, one fact test, one deterministic reproduction test, and runs
one pre-registered campaign whose contrast a declared decision tree
picks from the pilot. It adds no config knob and no mechanism.

The first draft of this record designed a uniform upkeep scale. It is
withdrawn here, with the reason: the phenotype's basal multiplier is
`derived.basal / reference.upkeep` clamped to [0.6, 1.6], the unicell
sits on the floor, and the reference derives through the same
accumulation - a scale would move nothing that the clamp does not
absorb. Building it would have produced a null about the knob, not
about the organism. If price ever needs a lever, one already exists on
the charged quantity: `physiology.basal_cost_milli_per_s`, hashed, and
already a campaign treatment elsewhere in the repository - no new
field would be justified.

## The composition record, exactly

- `EventKind::BodyComposition { id: u64, counts: [u16; 7] }`, tag 30,
  event schema **12**; `counts` indexed by `ModuleType::id()` in
  registry order (structural, sensory, motor, digestive, storage,
  reproductive, neural); a test asserts the index equals the id so the
  record and the registry cannot disagree on order.
- Pushed inside `admit_schema2_child`, after the admission's existing
  records for the organism (`PhenotypeAtBirth`, and `GrowthCompleted`
  when ontogeny applies), same tick, same organism, for births and
  materializations alike - the one function both use. Counts come from
  the body the morphology state holds for the child (`count_of` per
  type), never recomputed from the genome.
- Emitted again after a `GrowthCompleted` that completes a grown body
  when ontogeny is on, with the completed body's counts; not per
  activation.
- A world without morphology emits none (there is no body).
- Reconciliation ignores it (it changes no count); a schema-11 log
  decodes unchanged; the eventlog round trip gains one sample record;
  `verify-events` accepts it.
- No rule reads it. The Phase 17 neutrality test's cadence argument
  (state checksum identical with events on and off) covers it, and
  C20.6 asserts it directly.

## The census, exactly

`lifesim lineage --manifest FILE` (`lifesim-lineage-index-v1`, in
`sim-analysis`, decides nothing): per world, from Birth/PairedBirth/
Materialized/BodyComposition/GrowthCompleted/Death, an organism's module
count is the maximum over its `BodyComposition` sums and its
`GrowthCompleted` counts; multi-module means two or more. It reports
the multi-module organisms' origin split, completed lifespan (lower
median) and censoring, the same lifespan over the **matched one-module cohort** (born
one-module organisms admitted within 2,000 ticks of any multi-module
organism's admission; completed and censored counted for both groups),
offspring of multi-module parents (each birth counted once), parents
with offspring, **second-generation** organisms (born, multi-module, at
least one multi-module parent, and a module count not above that
parent's - inherited, not a fresh duplication), the largest module
count, the first multi-module tick, the last multi-module death tick,
and with schema-12 logs the **composition multiset** of multi-module
bodies (counts by type, since a count array carries no order and "the
second module" is not a fact the record can name) and, wherever a
child's and a parent's records both exist, the **child-minus-parent
type-count difference** - the duplication's signature - over
multi-module children. Every count is per world; the reduction pairs
worlds by seed.

## The fact test, exactly

`crates/sim-core/tests/phase20_lineage.rs` pins: founder reference mass
2,400, upkeep 750, intake 1,000, thrust ratio 1,000; the unicell's
derived (800, 200, 1,000, capacity 12,000) and its multipliers (600,
600, 1,000); and for each of the seven types the two-module body's
derived basal and its multiplier (structural 300 -> 600, sensory 350 ->
600, motor 600 -> 800, digestive 400 -> 600 with intake 1,200, storage
280 -> 600 with capacity 60,000, reproductive 450 -> 600, neural 700 ->
933). A registry or reference change fails it and is recorded.

## The reproduction test, exactly (Branches B and C)

A deterministic test that takes a two-module schema-2 genome (from a
pilot snapshot, encoded into the test as bytes, or synthesized by
applying one duplication to the unicell genome with a fixed draw) and
runs the kernel's own meiosis-and-development path 1,000 times against
a unicell genome and 1,000 times against itself with keyed draws,
classifying each child: refused non-viable, refused node budget, one
module, two or more modules. The four counts sum to the draws and are
the named mechanism in the findings. It uses the same functions the
birth path uses (no re-implementation), so what it measures is what
the world does.

Run before the pilot (2026-09-03) it measured: unicell x unicell 999
one-module, 1 refused on budget, 0 two-module in 1,000 draws (the
two-module genome, a duplicated gut, was found at keyed draw 6,398);
two-module x unicell 488 two-module, 512 one-module, 0 refused;
two-module x itself 767 / 231 / 2 refused on budget; compatibility
distance two-module to unicell 0.0000. Segregation is Mendelian,
viability refuses nothing, compatibility bars nothing - so Branch A's
lever is not the compatibility threshold but the pairing energy gate
(`phase2.pairing_energy_threshold_milli`, shipped 7,000), and Branch D
(second-generation organisms exist; Phase 19's peak was a 500-tick
sampling artefact) is live. The test is kept as the phase's standing
measurement and is re-read against the pilot's census.

## Fixture and verify

`lifesim fixture --composition` (implies `--transition --coupled`)
prints schema **14**: the schema-13 line plus `event_schema_version`,
`composition_records` and `max_modules_seen`. The schema-13 line itself
does not move - `verify-phase19` pins it by schema number, and a moved
schema would be a re-pin without a reason (D-113).
`scripts/verify-phase20-determinism.sh`: two-process replay; the
record count equals materializations plus births; both identities from
the printed totals; the Phase 19 state checksum unchanged (the record
is not hashed); the Phase 16 fixture untouched; a schema-12 log written
by a short run verifies, reconciles and decodes.

## Campaign shape (pre-registered before any confirmatory world runs)

The Phase 19 confirmatory's v2 base with events on, 100,000 ticks,
**fifty seeds per arm** (20001..20039 20041..20051, 20040 being ungenerable; ADR-0022's rare-outcome clause,
invoked because the shipped world's baseline for the primary endpoint
is expected at or near zero) with a simulation-based power statement
in the pre-registration; pilot 20901..20904. The pilot's U1 census is read
against the plan's decision tree; the pre-registration records the
branch, the arms (Branch A: `phase2.pairing_energy_threshold_milli` 5,000 against the
shipped 7,000; B and C: the shipped world alone
with an equivalence bound and the reproduction test; D: two horizons),
the SESOI and bar or bound, and the expected outcome. Primary endpoint
on every branch: second-generation multi-module organisms per world,
with the birth-normalized rate beside it.

## Consequences

- One event tag, one analysis command, two tests, one campaign. No
  config, no hash, no ALIF change; everything pinned stays pinned.
- The persistence endpoint is defined on lineage, closing the trap
  Phase 19 named.
- The record carries the clamp arithmetic so the "price" question is
  answered once, in numbers, and not asked again without a registry
  change.

## As built

Amended 2026-09-03 with every divergence so far; the campaign's
paragraph is appended when it is measured.

- **The record.** `EventKind::BodyComposition { id, counts: [u16; 7] }`,
  tag 30, `EVENT_SCHEMA_VERSION` 12; pushed in `admit_schema2_child`
  right after the body is stored (births and materializations alike)
  and beside every `GrowthCompleted` in the ontogeny completion loop;
  counts from `morphology::composition_counts(&body)` (registry order by
  `ModuleType::id()`). Persist: `TAG_BODY_COMPOSITION` 30, fixed width
  (tag, u64, seven u16 = 23 bytes), an all-zero record refused on
  decode, reconciliation no-op, the round-trip sample count 48 -> 51
  (three segments). Verified: one record per admission whose sum is the
  module count (3,000-tick coupled scratch world), records of the living
  agree with the `max_modules` gauge, no checksum moves, a mid-run save
  round trip restores the same next records, a bodiless schema-2 world
  emits none.
- **`founder_reference` made public** for the clamp fact test.
- **The census** (`crates/sim-analysis/src/lineage.rs`,
  `lifesim-lineage-index-v1`): built by a delegated agent on the
  GrowthCompleted-only definition and extended here to read the
  composition record, the inheritance guard (child count <= best
  parent's), the matched cohort (`COHORT_WINDOW_TICKS` 2,000) and the
  two histograms; eight unit tests; the CLI line carries `cohort_*`,
  `multi_compositions` and `added_modules`.
- **The fixture** is a new flag, `--composition` (schema 14), not a move
  of the schema-13 line; its parser arm initially omitted the index
  advance and spun forever - the kind of defect a fixture pin cannot
  see, caught by the run hanging.
- **The reproduction test** ran before the pilot; its numbers are in
  the section above and in the pre-registration.
- **Two confirmatory shapes** are committed (`-alone` and `-gate`); the
  locked pre-registration names which runs.
- **Cost (C20.7).** 23 bytes per admission; no tick-path change.
