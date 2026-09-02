# ADR-0032: The Multicellularity Transition's Concrete Design (Phase 16)

Status: accepted 2026-09-02. The design authority for Phase 16 remains
`specifications/unicellular-regime.md` (ADR-0020, ADR-0019, ADR-0018) and
`planning/phase-16-multicellularity-transition.md`; this record pins the
concrete choices those documents deliberately left open, so the
implementation cannot pick them silently. Where this record and the
specification disagree, the disagreement is a defect in this record.

## What this phase is, restated so nothing else creeps in

Phase 16 implements **one representation change**: microbial density in
the field regime becomes individual organisms in the individual regime.
It implements no multicellularity mechanic, no threshold that reads a
module count, and no reward for crossing anything (plan, Non-Goals;
C16.7). A materialized organism is a one-module body in Phase 10's
morphospace, and everything after admission is Phase 10 doing what it
already does.

## Origin mode `scratch` (ADR-0021, delivered here)

`OriginMode::Scratch` is added to the enum (codec id 3, name `scratch`,
after `random` = 1 and `seeded` = 2; ids are permanent). A scratch world
generates **zero founders**: `generate_founders` returns an empty set,
`place_founders` places nothing, and `next_entity_id` starts at 1 exactly
as it would after zero founders. Validation:

- `scratch` requires `initial_organisms == 0`, and every other mode still
  requires `initial_organisms >= 1` (the Phase 1 rule, unchanged for
  every existing config).
- `scratch` requires `chemistry.enabled`, `chemistry.microbial_enabled`
  and `chemistry.abiogenesis_enabled`: a scratch world with no source of
  density is a valid but permanently empty world, and the plan wants that
  case reachable only by *disabling* abiogenesis deliberately, which
  stays legal (`origin.mode = scratch` with `abiogenesis_enabled = false`
  is refused; the empty-world case is `scratch` with the transition off,
  which C15.5 already covers and C16.8 re-covers).
- `scratch` does not require the transition section. A field-only
  scratch world is the transition-disabled control (C16.5's arm T0).

The origin section already enters the config hash whenever the mode is
not the Phase 1 default, so a scratch config hashes under the existing
`lifesim-origin-config` tag with the mode name; no fixture moves.

**Extinction under scratch.** The extinction latch (`ids.is_empty()`) is
unchanged and fires at tick 1 in a scratch world, honestly: there are no
organisms. A materialization that admits an organism into an extinct
world **clears the latch** - the only path that can repopulate an empty
world is this one, so no earlier fixture can reach the clearing branch.
The `Extinction` event stays single-shot per latch; repopulation is
evented by the `Materialized` records themselves.

## The transition section (`transition.*`, `lifesim-transition-v1`)

A new top-level config section, disabled by default, hashed only when
enabled under the tag `lifesim-transition-config` together with
`TRANSITION_POLICY_VERSION` and `GENOME_MAP_VERSION` (D-014). Enabling it
requires `chemistry.microbial_enabled`, `phase2.enabled`,
`genome2.enabled` and `morphology.enabled` - the organism it produces is
a schema-2 genome with a developed body, and nothing less can be
admitted.

| Field | Meaning | Validation |
|---|---|---|
| `enabled` | the gate | - |
| `check_interval_ticks` | the trigger is evaluated every N world ticks | >= 1 |
| `density_floor_milli` | a slot must hold at least this density at a check | > 0 and >= `organism_energy_milli` |
| `persistence_checks` | consecutive checks at or above the floor before a slot triggers | >= 1 |
| `aggregation_step_min` | only classes at or above this aggregation-axis position can trigger | < `chemistry.aggregation_axis` |
| `organism_energy_milli` | energy credited per materialized organism, debited 1:1 from the slot's density | > 0, and <= the unicell body's energy capacity (checked at `World::new`, where the body exists) |
| `max_organisms_per_event` | organisms one `(cell, class)` trigger may produce | >= 1 |
| `max_materializations_per_tick` | organisms admitted per world tick across all triggers; the rest defer | >= 1 |

Defaults (used only when enabled): interval 100, floor 20,000 milli
(twenty seedings; the Phase 15 campaign's standing densities are ~53x
seeded, so the floor is reachable by growth and unreachable by a lone
firing), persistence 5, `aggregation_step_min` 1 (the top step of the
default two-position axis - the plan's "aggregation-tendency value above
a threshold"), energy 4,000 milli (`offspring_energy_milli`'s value, so a
materialized organism starts where a born one does), 4 organisms per
event, 64 per tick.

### The trigger: a physical condition with a memory

At every check tick, for every slot in ascending `(cell, class)`:

1. if `density >= density_floor_milli` the slot's persistence counter
   increments (saturating), else it resets to zero;
2. the slot triggers iff its counter `>= persistence_checks`, its class's
   `aggregation_step >= aggregation_step_min`, and the cell is
   traversable for organisms (`effective_traversable`) - density in a
   water cell stays density, recorded rather than smoothed over;
3. a triggered slot that materializes resets its counter to zero (what
   remains must persist again); a slot deferred by either cap keeps its
   counter and is re-evaluated at the next check.

The per-slot counters are **real state** (the plan's "persistence over a
window" cannot be recomputed from densities), so they are saved and
hashed. This is a recorded deviation from the plan's "no new checksum
section": `SECTION_TRANSITION` (21) carries the counters and the
transition's own counters, hashed under `lifesim-transition-state-v1`,
appended after the microbial section, present only when enabled. Cost:
`u32` per slot (131 KB plain at 64x64 x 8 classes), measured by the
snapshot benchmark rather than assumed.

### Materialization: the conversion, exactly

For each triggering slot, in ascending `(cell, class)`:

- `biomass = min(density, max_organisms_per_event * organism_energy_milli)`;
  `n = biomass / organism_energy_milli` (floor; `n >= 1` because the floor
  is at least one organism's energy); `remainder = biomass - n * energy`.
- The per-tick cap binds here: if admitting `n` would exceed
  `max_materializations_per_tick` for this tick, or `population + n`
  would exceed `max_entities`, the slot is **deferred whole** (never a
  partial event): `deferred_cap_total` or `deferred_capacity_total`
  increments - two counters, because "the world was full" and "the tick
  was full" are different findings (D-074).
- Each organism is admitted through the **same function the schema-2
  birth path uses** (`admit_schema2_child`, extracted from `lifecycle`
  by pure code motion - the Phase 14/15 fixtures pin that the extraction
  moved nothing), with the ordinary born-complete ontogeny record, zero
  learned state, zero cooldown, age 0, depth 0, parents `[0, 0]`.
- Energy: organism `k` of the event receives `organism_energy_milli`;
  the **lowest new entity ID** additionally receives the remainder,
  capped at its own energy capacity - the plan's stated convention, the
  same one reproduction investment follows. What is debited from the
  slot is exactly what was credited: `n * energy + remainder_credited`.
  The uncredited part of a remainder (only when it would overflow the
  capacity) simply stays in the field.
- Ledger: the credited energy enters a new term
  `TransitionState::materialized_milli`, which joins the organism energy
  identity (`initial + assimilated + materialized - spent - removed ==
  sum`) and the field identity (`produced + deposited - materialized_out
  == chemistry + microbial`). One number, counted on both sides, so
  the conversion is checked by `World::check_invariants` at every
  campaign check interval (C16.1's in-run half). Population and entity-ID
  identities gain `materialized_total` on the same terms.
- Position: the cell's interior, jittered by the `Transition` stream (20)
  keyed `(seed, tick, slot_index, ordinal * 4 + {0, 1})`, heading at
  draw `ordinal * 4 + 2`. **Keyed on the slot and ordinal, never on the
  entity ID or a running count**, so a cell's organisms are identical
  whether or not another cell triggered the same tick - that is the
  content of C16.3, and it is what the test breaks.
- Event `Materialized { id, cell, class, energy_milli }` (tag 29, event
  schema 11), plus the ordinary `GrowthCompleted` at admission because a
  one-module body is born complete (the D-127 lesson).

Materialization runs inside `lifecycle`, after the paired births of the
same tick, so IDs stay strictly increasing (births, then materializations)
and the density it reads is this tick's committed field.

### The class-to-genome map: constant by design (`GENOME_MAP_VERSION` 1)

The map takes a class and returns a genome, deterministically and
versioned (C16.4). **In v1 it returns the same genome for every class**:
the minimal schema-2 founder (`minimal_founder` at every trait 0.5 - the
founder midpoint, three nodes, two edges, both bindings) plus a
one-rule growth program that differentiates the origin module into a
digestive module, plus whatever loci the world's other gates layer onto
founders (the preference band under mate choice, the marker under the
probe) so a materialized genome is structurally identical to a founder's
in that world. The reasoning:

- no locus corresponds to any class axis: substrate preference has no
  organism analogue under coupling v1 (biomass is the food), replication
  rate is a field-regime parameter, and the aggregation axis gates the
  *trigger*, which is the physical condition, not the organism;
- a map that read the axes into traits would be exactly the risk the plan
  names ("the map quietly encodes a good starting organism");
- the registry stays consequential where it should: which classes
  trigger, and where.

The unicell body: one digestive module at the origin, scale 1,000.
Derived attributes fall where Phase 10's arithmetic puts a body with no
motor and no sensor - speed and sensing at their floors, body scale at
its floor - which is the honest state of a unicell in this morphospace,
not a penalty, and the reason the plan's C16.6 expects a null.

### Neutrality (C16.2), stated as a structural property and then tested

After admission the world holds **no provenance**: no flag, no depth, no
parent marks a materialized organism (parents `[0, 0]` and depth 0 are a
founder's values). Both admission kinds run the one function, so every
per-organism row - energy, phenotype, body, grown prefix, learned state,
mate-choice weights, health, action census, object band - is computed by
the same code. The test admits the same synthesized child into two clones
of one world, once as a birth and once as a materialization, asserts every
row equal, then steps both 1,000 ticks and asserts the organism's
trajectory equal in both (under coupling v1 the field feeds nothing back,
so the worlds differ only in counters and the debited density).

## Formats, streams, sections, schemas

- `RngSystem::Transition = 20`, the number ADR-0031 reserved.
- ALIF format 14: the transition config block appended to the prefix
  chain (`FORMAT14_CONFIG_BYTES` = 41: gate, interval u64, floor i64,
  persistence u32, aggregation step u32, energy i64, two caps u32), the
  origin-mode byte gains value 3, `SECTION_TRANSITION` (21). Retained
  format-13 writer/reader, `refuse_format14_state`, `FORMAT13_TO_CURRENT`
  registered, the chain test gains one row. A format-13 file reads as a
  transition-disabled world (resolution, not invention: no build that
  wrote format 13 could materialize).
- Event schema 11: tag 29 `Materialized`. The decoder keeps accepting
  older schemas.
- Fixture schema 12: `lifesim fixture --transition` (a scratch world at
  the field trace's rates with the transition on; the verify script
  refuses `materialized`, `population` and `births` at zero, closes both
  identities from the printed totals, and pins the Phase 15 fixture).
- Benchmark schema 10 unchanged: no new tick phase. The record gains the
  per-tick check cost, the burst tick and the per-transition cost, and
  the snapshot growth per slot.
- Field series `field-series 2`: the Phase 15 columns unchanged, then
  `materialized= materialized_milli= max_modules= multi_module=`
  appended (C16.6's observables); the Phase 15 reduction's regex accepts
  the trailing columns so the archived run stays recomputable.

## Campaign shape (pre-registered before any world runs)

Conditions on 30 matched seeds: T0 (scratch, transition off - the
mechanism control: zero organisms ever, by construction), N (transition
on, uniform production), S2 (2x contrast, the one interior point below
the 4x saturation D-128 found) and S4 (fully concentrated). C16.5 reads
`materialized_total > 0` per world; C16.6 reads the module-count columns.
Expectations are stated in the pre-registration, including the one the
Phase 15 ceiling forces: materialization is expected in nearly every N
world too, so the scaffold contrast is expected flat and the T0-versus-N
difference is the interpretable one. C16.6's null is expected and stated.

## Consequences

- One config section, one save section, one event tag, one stream; the
  disabled path is the Phase 15 path byte for byte (C16.8).
- The admission refactor touches the birth path. It is code motion, and
  four fixtures (13, 14, 15 and the kernel pair) are the proof.
- Coupling v1 stays as narrow as ADR-0031 left it: chemistry-as-food is
  still deferred, and C16.2's survival clause leans on that narrowness
  (the field cannot favour anyone). The follow-on phase that wires
  feeding to the field must re-run the neutrality test with a coupled
  field, and this record says so.

## As built (2026-09-02 amendment)

Where the implementation and this record's letter diverged, the record
owns it here rather than silently:

- **`PhenotypeAtBirth` for materialized organisms.** Under `social.enabled`
  a materialized organism emits tag 26 at admission exactly as a born one
  does (the record above named only `GrowthCompleted`). The recognition
  classifier reads tag 26 for organisms that die before a snapshot; a
  materialized organism is a newborn for that purpose. Founders still
  carry none.
- **The unicell's numbers.** One digestive module at scale 1,000: mass
  800 milli, basal cost 200 milli, intake 1,000 milli, energy capacity
  12,000 milli. The default `organism_energy_milli` (4,000) and the
  shipped pairing threshold (7,000) both fit inside it; a materialized
  organism starts below the pairing threshold and must eat up to it.
- **No births in the fixture, and why.** The `--transition` trace
  materializes 4,086 organisms in 4,000 ticks and ends with 282 alive
  and zero births: a unicell is a slow (half the speed floor, throttle
  unbound), blind gut that spends about six times what it assimilates
  and starves in roughly 200 ticks, far short of its trait-derived
  maturity (800 ticks at the midpoint). Lowering the pairing threshold
  to 2,000 changed nothing observable, so the shipped value stays and
  the verify script refuses `births` at nothing - it pins
  materialization, feeding, spending, death and the deposits. Whether
  any materialized lineage reproduces at the campaign's horizon is part
  of what C16.5 and C16.6 measure, and the field series gained a
  `births` column so a null with no reproduction is distinguishable
  from one with it.
- **The per-tick cap binds hard at the fixture's rates.** At production
  20 milli per cell-step the fixture deferred 244,735 slot-triggers over
  4,000 ticks against 4,032 admitted events; the cap is a deterministic,
  counted rate limit exactly as the plan asked, and the campaign's
  pre-registration states its value.
- **Production in tests and the fixture.** The integration tests and the
  fixture run abiotic production at 20 milli per cell-step (ten times
  the shipped 2) so an eligible class reaches one organism's worth of
  density inside a test horizon; the campaign runs the shipped rate.
- **A test-writing trap worth keeping.** The field step runs inside
  `environment`, before `lifecycle`, so density planted by save surgery
  loses about a third to death and mutation flow before the trigger
  reads it in the same tick. The surgical tests freeze the field
  (death, mutation and growth rates zero); the dynamic tests do not.
- **Field series.** `field-series 2` appends five columns, not four:
  `materialized`, `materialized_milli`, `max_modules`, `multi_module`,
  `births`.
- **Benchmarks** (`experiments/results/phase16-benchmark-measurements.txt`):
  the check scan is within noise of the disabled world at interval 100
  and at interval 1 (550 and 583 against 565 microseconds per tick at
  64x64); a 64-slot burst tick costs 2.09 ms against 0.23 ms for the
  ticks after it, 7.3 microseconds per materialized organism; the
  section adds exactly 4.0 bytes per slot plain and is invisible under
  zstd (67,073 against 67,050 bytes).
