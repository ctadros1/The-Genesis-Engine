# Artifact And Material Ontology Specification

Status: implementing, Phase 12 artifact half (2026-08-16). Policy versions
`lifesim-material-v1`, `lifesim-artifact-v1`. Governing decision: ADR-0028,
which records every place this specification departs from the commissioned
review and why. The design specification this replaces (2026-08-04) is
recoverable from git history; the changes are listed at the end.

## Problem

There is no object ontology of any kind. No materials, no items, no
carrying, no combination. Structures and tools require objects that exist
independently of organisms, persist after their makers die, and have
physical properties an organism can exploit.

## The Authored Boundary, Restated For This Spec

Authored: materials have hardness, density, durability, decay and energy
content. Objects can be picked up, carried, dropped, placed, struck, and
combined. Composite properties derive from constituent properties by a stated
function. Carrying costs energy in proportion to mass. Every action costs
energy whether or not it succeeds.

Never authored: a recipe list, a crafting table, a tool taxonomy, a
technology tree, a set of "valid" combinations, a per-material capability
tag, or any reward for making anything. There is no table anywhere in the
system that says `stone + stick = axe`, and there is no bit anywhere that
says a material is "combinable" or "consumable". There is a combination
function over material properties, and whatever it produces is what exists.
No rule branches on a material ID; material IDs select parameter records and
physical equations consume the parameters (review 5.7).

## Material Registry

A bounded versioned registry, `MATERIAL_REGISTRY_VERSION = 1`, whose version
enters the config hash inside the artifact section. Materials are world
physics, not content: adding one is a config change with a new lineage, not
a feature toggle.

| Property | Type | Meaning |
|---|---|---|
| `material_id` | u16 | Registry key, permanent, never reused; 0 is invalid |
| `hardness_q16` | u32 | Resistance to fracture: the force a strike must reach |
| `density_milli` | i64 | Mass per unit of extracted volume |
| `durability_q16` | u32 | Integrity lost per strike, by the struck object and by a held object used to strike |
| `decay_per_tick_q16` | u32 | Passive integrity loss per tick (linear); for a material with energy content, also the per-tick fraction of remaining energy lost (exponential, mirroring Phase 7's carcass decay exactly) |
| `energy_content_milli` | i64 | Assimilable energy per 1,000 milli-mass of extracted material; zero for stone |

| id | name | hardness | density | durability | decay/tick | energy/1000 mass |
|---|---|---|---|---|---|---|
| 1 | stone | 8.0 (524,288) | 2,500 | 32 | 0 | 0 |
| 2 | wood | 2.0 (131,072) | 700 | 512 | 1 | 0 |
| 3 | fiber | 0.5 (32,768) | 300 | 2,048 | 8 | 500 |
| 4 | carcass | 0.25 (16,384) | 1,000 | 4,096 | 328 | 1,000 |

Provisional and recorded as such (review 19.7 says the decay regime relative
to organism lifespan is an unswept axis; C12.2's pass condition sits on it).
Wood's linear decay gives a lifetime of 65,536 ticks, 1.8 default lifespans;
fiber 8,192; carcass 200 ticks of integrity and the same exponential energy
half-life (~139 ticks) `contest.carcass_decay_q16_per_s = 3,277` gives today.
Stone does not decay.

`carcass` is the review's sanctioned "schema-level exceptional matter such as
organism tissue" (5.7): it is the only material that enters the world from
an organism rather than from terrain.

Materials enter the world from terrain: every land cell yields one material
by its relative elevation above the coastline (`(elevation - land_threshold)
/ (65536 - land_threshold)`, Q16): stone at or above `stone_relative_q16`,
wood at or above `wood_relative_q16`, fiber below. Water yields nothing.
Extraction is a `strike` against the cell an organism stands on when no
object is within reach; it depletes the cell's remaining yield, stored as a
`LAYER_MATERIAL_YIELD` override on the mutable-world state (absent means the
baseline `terrain_yield_milli`), and yield regenerates by
`yield_regen_milli` every `yield_regen_interval_ticks` toward baseline, the
override being cleared when it reaches it. Terrain yield is outside the
object ledger, as biomass regrowth is outside the energy ledger; extraction
is the source term.

## Object Model

An object is a first-class entity in the shared object ID space
(`specifications/determinism-extensions.md` Rule 2): it takes its ID from
the same monotonic counter organisms do, `World::next_entity_id`, whose name
is kept and whose meaning is unchanged - it is the next ID to allocate for
anything.

| Field | Type | Notes |
|---|---|---|
| `id` | u64 | From the shared counter; never reused |
| `material_id` | u16 | Registry key; a composite carries its heaviest constituent's, for the observer only - no rule reads it |
| `x_fp`, `y_fp` | i32 | Position, organism fixed-point units; meaningful only when free (`holder_id == 0 && owner_id == 0`) |
| `integrity_q16` | i32 | Fixed point, 65,536 = whole; decays; zero destroys a simple object and un-combines a composite |
| `mass_milli` | i64 | Extracted volume x density for a simple object; sum for a composite |
| `energy_milli` | i64 | Assimilable energy remaining; sum for a composite |
| `hardness_q16` | u32 | Material's for a simple object; max over constituents for a composite. Stored, and `check_invariants` re-derives it |
| `durability_q16` | u32 | Material's; min over constituents |
| `decay_q16` | u32 | Material's; max over constituents |
| `holder_id` | u64 | 0 when in the world, else the carrying organism |
| `owner_id` | u64 | 0 when free, else the composite that owns it |
| `depth` | u8 | 0 for a simple object; `1 + max(constituent depth)` |
| `created_tick` | u64 | Provenance |
| `creator_id` | u64 | The organism that placed or combined it; 0 for terrain-derived, fragments and carcasses. Read by no rule; the observer's "placed" flag |
| `cause` | u8 | 1 Extracted, 2 Fractured, 3 Combined, 4 Carcass |
| `parent_id` | u64 | The fractured parent, or the source organism of a carcass; else 0 |
| composition | bounded list of u64 | Constituent IDs, ascending; empty for a simple object |

No shape, no orientation, no height (ADR-0022 D2, ADR-0028 section 4).

Objects are stored struct-of-arrays sorted by `id`, compacted on removal in
the same manner as organisms, with the ordering verified by
`check_invariants`. Two derived indices are rebuilt, never saved and never
hashed: the per-cell list of free objects (sorted by ID; built in the
`SpatialIndex` phase) and the per-organism held list (rebuilt on restore
and cross-checked against `holder_id` by `check_invariants`).

## Actions

Five output channels in the channel registry, version 2 (see "Registry
Version" below): `pick_up` 113, `drop` 114, `place` 115, `strike` 116,
`combine` 117. An organism binds them or does not; **an unbound action is
never requested.** A bound channel requests its action when its value
exceeds `action_threshold_q16`; the value, in milli, is the action's
priority in contests. All actions are single-tick.

| Action | Effect | Refusals, all typed, counted, evented |
|---|---|---|
| `pick_up` | Acquire the nearest free object within `reach_m` whose mass fits the remaining carry capacity | `NoTarget`, `CapacityExceeded` (nothing in reach fits), `HeldCap` (`max_held_objects` reached), `Contested` (lost to a higher claim) |
| `drop` | Release the lowest-ID held object at the organism's position | `NothingHeld`, `OccupancyCap` |
| `place` | Release the lowest-ID held object into the adjacent cell the organism faces, setting `creator_id` | `NothingHeld`, `InvalidCell` (off map or not traversable for organisms), `OccupancyCap` |
| `strike` | Apply force to the nearest free object within reach, else extract from the cell underfoot | `Depleted` (terrain yield exhausted), `ObjectCap`, `NoYield` (water) |
| `combine` | Join the lowest-ID held object with the nearest free object within reach | `NothingHeld`, `NoTarget`, `DepthCap`, `BreadthCap`, `ObjectCap`, `Contested`, `JointFailed` |

Every attempt costs `action_cost_milli` (`strike`: `strike_cost_milli`),
success or failure. Contested acquisition (two organisms claiming one object
this tick, by `pick_up` or `combine`) is resolved by (priority descending,
distance squared ascending, organism ID ascending); losers are refused
`Contested` and pay. No lottery is configured in this version; a tie on the
first two keys resolves on ID and no stream is drawn.

Target selection uses the sorted-candidates rule: candidates within reach are
materialized, sorted by `(distance_squared, id)`, truncated to
`max_candidates`, and the first admissible one is chosen. Actions read
positions after this tick's movement and the object table as it stood at the
start of the artifact pass; nothing an action creates this tick is a target
this tick (review 13.7's one-tick latency).

Resolution order within the pass, each over organisms in ascending ID: drop,
place, then claims (`pick_up` and `combine`) resolved jointly per target,
then strikes aggregated per target, then terrain strikes. Compaction and
allocation happen once, at the end, in that order, so every new ID is
allocated in a total order that is a pure function of the state.

### Carrying

- Carry capacity is `carry_capacity_milli * body_scale_milli / 1000`, so
  carrying is a body-plan tradeoff rather than a free ability, and it is
  the same body-scale ratio contest and physiology use (D-085).
- Held objects move with the holder, are not in any cell list, are not
  perceived, and are not targets. They are within the holder's reach for
  consumption.
- Movement cost is multiplied by `1 + carry_move_cost_q16 * carried / capacity`
  in the existing `C_move` term, and `hold_cost_milli_per_s * carried /
  capacity` is charged per tick whether or not the holder moves, so carrying
  indefinitely costs something (review 15.6).
- On death, held objects are dropped at the death position in ascending
  ID order, bypassing the occupancy cap (an object cannot vanish because
  its holder died in a crowded cell).
- The `carried_load` input channel reports `carried / capacity`.

### Striking and fracture

Force is `strike_force_q16 * body_scale_milli / 1000 + sum over held
objects of (hardness_q16 * mass_milli / strike_mass_reference_milli)`. Every
strike on one target in a tick is summed before the fracture test (review
13.7). If the sum reaches `hardness_q16 * fracture_margin_q16 >> 16` the
target fractures:

- a composite comes apart: its constituents return to the world at its
  position as free objects, unchanged, and the composite is destroyed - mass
  and energy exact by construction;
- a simple object becomes `k` fragments, `k` drawn uniformly in
  `[2, max_fragments]` on the `Artifact` stream keyed `(tick, target_id)`,
  each with `mass / k` and `energy / k`, the remainders to the lowest new ID,
  material, hardness, durability and decay inherited, `cause` Fractured,
  `parent_id` the target, `creator_id` 0; any fragment under
  `min_fragment_mass_milli` is not created and its share goes to the dust
  sink; the target is destroyed.

Otherwise the target's integrity drops by its `durability_q16`; at zero a
composite comes apart and a simple object goes to dust. Every held object
of a striker loses its own `durability_q16` per strike, so striking with a
soft object wears it and striking with a hard one does not (much).

Striking with no object in reach strikes the terrain cell under the
organism: if the cell yields a material and has yield left, an object of
that material is created at the organism's position with mass
`extraction_milli * (32,768 + draw % 32,768) >> 16 * density / 1000`, the
draw on the `MaterialYield` stream keyed `(tick, cell)`, energy `mass *
energy_content / 1000`, `cause` Extracted; the cell's yield override drops
by the extracted volume.

### Combination

`combine` joins the lowest-ID held object `H` with the nearest free target
`T`. It is refused for physical reasons only: `1 + max(depth)` above
`max_composition_depth`, constituent count above
`max_composition_breadth`, the world at `max_objects`, or the joint failing.
There is no compatibility check of any kind.

`joint_quality_q16 = min(65,536, draw_q16 * body_scale_milli / 1000)` with
the draw on the `Artifact` stream keyed on `pair_key(combiner, target)`. If
it is below `joint_floor_q16` the attempt fails (`JointFailed`), costs its
energy, and nothing changes. Otherwise the composite is a new object at
`T`'s position, free (not held), with:

    mass        = H.mass + T.mass
    energy      = H.energy + T.energy
    hardness    = max(H.hardness, T.hardness)
    durability  = min(H.durability, T.durability)
    decay       = max(H.decay, T.decay)
    integrity   = min(H.integrity, T.integrity) * joint_quality >> 16
    depth       = 1 + max(H.depth, T.depth)
    composition = [H.id, T.id] sorted, with H's or T's own composition
                  lists retained on them, not flattened
    creator     = the combiner; cause Combined; parent 0

`H` and `T` get `owner_id` = the composite's ID, `holder_id` 0, and leave
world iteration. Because the composite is placed rather than held, an
organism must pick it up again to build deeper, and its mass against the
organism's carry capacity is what bounds depth in practice.

## Persistence In The World

Free objects remain until integrity reaches zero through decay or damage,
or, for an object with energy, until its energy is consumed or decays to
zero. This is the property that makes Phase 12 different from every earlier
phase: an object outlives the organism that made it, and later organisms
encounter a world that earlier organisms shaped.

### Consumption

An organism with `intent_eat` set and room below its energy capacity
consumes from the nearest object with energy within `consume_reach_m` -
held objects included - at most `intake_tick` raw per tick, assimilated at
`assimilation_q16` exactly as biomass and Phase 7 carcasses are; the
unassimilated remainder is ledgered as decay. Organisms consume in ascending
ID order, one object each per tick. `consumable` is not a bit; it is
`energy_milli > 0`.

### Decay

In `Lifecycle`, after deaths: every free or held object with `decay_q16 >
0` loses `decay_q16` integrity (saturating at 0) and, if it has energy,
`max(1, energy * decay_q16 >> 16)` energy to the decay sink. An object at
zero integrity, or with energy content and zero energy, is destroyed: a
composite comes apart, a simple object's remaining mass and energy go to the
decay sink. Owned constituents do not decay; their composite does.

### Perception

Six input channels, registry version 2: 17 `object_present` (1 if a free
object is within `perception_range_m`), 18 `object_distance` (1 - d /
range), 19 `object_bearing` (relative heading, as
`nearest_organism_relative_heading`), 20 `object_heft` (target mass over
the perceiver's carry capacity, clamped to 1), 21 `object_hardness` (target
hardness over the registry maximum), 22 `carried_load` (held mass over
capacity). K = 1: the nearest free object by `(distance_squared, id)`.
**No channel exposes a material ID or a composite depth** (review 1.3, 5.1;
ADR-0022 A3).

## Cell Occupancy And Movement

An object with `mass_milli >= blocking_mass_milli` blocks *entry* to its
cell for organisms; movement into such a cell is rejected in the movement
resolver beside the traversability check, and the organism pays no
movement cost, exactly as for water. It does not evict or trap an organism
already there, so `check_invariants`'s "no organism on a non-traversable
cell" is unchanged. Blocking is answered from the object cell index; the
reserved `LAYER_TRAVERSABLE` override is not written by objects.

Free objects per cell are capped at `max_objects_per_cell`; a `drop` or
`place` into a full cell is refused, counted and evented. Objects in the
world are capped at `max_objects`; creation past the cap is refused, counted
and evented, and the mass and energy that would have been created go to the
dust sink so the ledger stays exact.

## Carcasses

With the artifact section enabled, `spawn_carcass` creates a carcass object
instead of a `ContestState` carcass: fresh ID, `parent_id` the organism,
`creator_id` 0, `cause` Carcass, energy the contest section's
`carcass_energy_q16` share of remaining energy, mass equal to energy,
integrity 65,536, at the death position. It is consumed and decays under
the rules above and is destroyed at zero energy. `ContestState.carcasses`,
its ledger and its invariant are bypassed in such a world and untouched in
every other, which is what preserves the Phase 7 behaviour byte for byte
when the section is disabled. The contest section is required, as it always
was for carcasses.

## Registry Version

`CHANNEL_REGISTRY_VERSION` stays 1 and keeps its meaning. The eleven
channels above are `CHANNELS_V2`, registry version 2, and a world's
registry version is 2 when its artifact section is enabled and 1 otherwise;
that value is what the genome2 config block hashes. An ALG2 genome stamps
the smallest version that covers its bindings, and the reader accepts 1 and
2, validating each binding against the declared version's channel set. A
genome that binds a version-2 channel is refused, typed and fail-closed, at
world construction and at restore in a version-1 world.

Binding is evolvable through the `bind` structural operator
(`specifications/genome-schema-2.md`), rate `genome2.mutation.binding_q16`,
zero by default and hashed only when nonzero, which draws a node and a
channel from the world's registry and inserts an `IoBinding` locus. Without
it no organism could ever request an object action (ADR-0028, context).

## Configuration

Section `artifact`, hashed only when enabled, appended after `probe`.
Requires `genome2.enabled`, `worldmod.enabled` and, for carcasses,
`contest.enabled`. Every field is settable and in `FIELD_NAMES`. Defaults
are provisional and recorded as such.

| Field | Default | Meaning |
|---|---|---|
| `enabled` | false | Section gate |
| `max_objects` | 4,096 | World cap on live objects, owned constituents included |
| `max_objects_per_cell` | 8 | Free objects per cell |
| `max_composition_depth` | 4 | Depth of a composite |
| `max_composition_breadth` | 8 | Constituents of a composite |
| `max_held_objects` | 1 | Objects one organism holds |
| `max_candidates` | 8 | Truncation of every sorted candidate set |
| `carry_capacity_milli` | 4,000 | Capacity at body scale 1,000 |
| `carry_move_cost_q16` | 65,536 | Movement-cost multiplier per unit of carried / capacity |
| `hold_cost_milli_per_s` | 20 | Per-second hold cost at full load |
| `action_cost_milli` | 60 | Per attempted pick_up / drop / place / combine |
| `strike_cost_milli` | 120 | Per attempted strike |
| `action_threshold_q16` | 32,768 | Request value above which a bound channel fires |
| `reach_m` | 2 | Action reach |
| `consume_reach_m` | 2 | Consumption reach |
| `perception_range_m` | 8 | Object perception range |
| `strike_force_q16` | 262,144 | Bare force at body scale 1,000 (4.0) |
| `strike_mass_reference_milli` | 2,000 | Held mass that adds one full hardness to force |
| `fracture_margin_q16` | 65,536 | Multiplier on hardness the summed force must reach |
| `max_fragments` | 4 | Fragment count upper bound (lower bound 2) |
| `min_fragment_mass_milli` | 400 | Below this a fragment is dust |
| `joint_floor_q16` | 16,384 | Joint draws below this fail the attempt |
| `blocking_mass_milli` | 3,000 | Free object mass that blocks entry |
| `terrain_yield_milli` | 6,000 | Baseline extractable volume per land cell |
| `extraction_milli` | 800 | Volume per terrain strike before variance and density |
| `yield_regen_milli` | 400 | Regeneration per interval |
| `yield_regen_interval_ticks` | 600 | Regeneration cadence |
| `stone_relative_q16` | 39,322 | Relative elevation at or above which a cell yields stone (0.6) |
| `wood_relative_q16` | 16,384 | ... wood (0.25); fiber below |

`condition C` of the plan ("actions permitted but physically inert") is
`artifact.inert = true`: every action resolves, costs, counts and events as
usual, but no object is created, moved, held, struck or combined - a
world-visible no-op with the same energy cost. Condition B ("ephemeral") is
`artifact.ephemeral = true`: a placed or dropped object is destroyed at the
end of the tick it lands in, ledgered to dust. Condition D is
`max_composition_depth = 0`. All three are settable fields in the section,
hashed with it, so the four arms differ in their config hashes.

## Invariants

- Object IDs strictly increase and never repeat within a lineage; storage
  index order equals ID order; the allocation identity is `initial +
  births + objects_allocated + 1 == next_entity_id`.
- Every constituent in a composition list exists, is owned by exactly that
  composite, and is neither held nor free; the stored hardness, durability,
  decay, depth, mass and energy of a composite equal their derivation.
- No object is simultaneously held and owned, or held and in a cell list;
  a holder exists and is alive; a dead organism holds nothing.
- Free objects per cell never exceed the cap; live objects never exceed the
  world cap.
- Mass and energy accounting through creation, combination, fracture,
  decay, and consumption is exact to the milli-unit, verified by
  `check_invariants` against the object ledger; combination and fracture are
  mass- and energy-neutral, and a rounding remainder goes deterministically
  to the lowest new object ID.

## Test Requirements

- Ledger exactness across a 10^6-tick run with heavy object churn.
- Fracture and combination round trip: combining then fracturing restores
  constituent mass, energy and IDs exactly.
- Cap enforcement for depth, breadth, occupancy, carry capacity, held count
  and world count, each deterministic, counted, and evented, each driven
  until it binds.
- Contested acquisition is order-independent under storage permutation.
- Save round trip with composites of depth greater than one, held objects,
  and mid-decay objects, then stepped.
- Disabled-section equality: artifacts disabled reproduces the Phase 1, 2,
  9 and 11 fixtures exactly (C12.8's list, corrected from "the Phase 13
  fixture" by D-096).
- Seeded malformed-input harness over the object section codec, with every
  declared count patched adversarially and CRCs resealed (standing rule 2).
- Registry gating: a genome bound to a version-2 channel is refused in a
  version-1 world; every retained-format encoder writes a version-1 stamp
  for a genome without such bindings.
- Rule coverage: perturbing each material's properties moves every outcome
  that reads them continuously, and no outcome changes when only the
  material ID changes with properties held fixed.

## Height And Support: The Deferred Subset

`artifacts` section 1.5 recommends a hybrid 2.5D representation with discrete
height intervals, a collider palette, and an explicit bond/support graph.
The recommendation is sound, and **stacking genuinely cannot be expressed
without height**: a 2D world can hold objects side by side but cannot build
upward, which limits what "persistent structure" can mean.

ADR-0022 D2 defers rather than declines it. The reasoning is cost
concentration: height changes collision, movement, perception, rendering,
and the save format simultaneously, arriving in a plan that already carries
four unmeasured cost multipliers (variable topology, morphology, learned
state, and object churn).

The recorded minimum viable subset, scheduled as a Phase 12 stretch item
gated on the Phase 12 cost measurement:

- one integer `height_interval` per object, not a continuous z coordinate;
- a `supported_by` object reference forming an explicit support graph,
  acyclic by construction and validated at decode;
- a support rule: an object whose supporter is destroyed either falls to the
  next supporting interval or is destroyed, by material policy;
- no rotation, no toppling, no torque, no partial overlap.

**Until this lands the plan does not claim stacked construction**, and no
document may describe a placed-object arrangement as a built structure in
the vertical sense. Object placement remains planar.

## Required Addition: Composite Geometry (ADR-0024)

This specification defines a composite as a bounded **list** of constituent
object IDs with derived scalar properties, and defines **no spatial
arrangement**. A depth-2 composite therefore has no shape.

That gap was surfaced by the rendering work
(`specifications/appearance-derivation.md`) but it is a gap in this
specification, not in the renderer: a renderer that invented an arrangement
would be authoring appearance, which ADR-0024 forbids.

The addition required before composites can be rendered as structures:

- Combination records a **relative lattice offset and orientation per
  constituent**, chosen deterministically at combination time from the
  combining organism's state and a draw from the `Artifact` stream keyed on
  the canonical pair key.
- The offset set is validated **connected and non-overlapping**, exactly as
  a morphology body is. An invalid arrangement fails the combination with a
  typed reason and a counter; it never produces an invalid object.
- Offsets are integer lattice coordinates, so composite geometry is exactly
  representable and hashable, with no float geometry.
- Constituents are stored in ascending offset order so composition is
  canonical and comparable.
- **Fracture restores constituents to independent objects at their offset
  positions**, preserving the existing exact mass and energy conservation
  including the rounding-remainder convention.

Until this lands, composites render as an aggregate sized by total mass and
coloured by dominant material, and the observer must not present that
aggregate as the object's structure. **Not built in this pass**; the
composition list is stored ascending by ID, which is canonical without
offsets.

## Changes From The 2026-08-04 Design Specification

Each is decided in ADR-0028 with its reason.

- The `affordances` bitset is gone; every affordance is a predicate over
  physical quantities.
- `object_material` and `object_composite_depth` are replaced by
  `object_heft` and `object_hardness`; `carried_load` is added.
- The fracture draw is keyed on the target, not the striker-target pair,
  because damage is aggregated across strikers.
- Failed actions cost energy; a per-tick hold cost exists beside the
  movement multiplier.
- The composite property function no longer grants affordances.
- `cause` and `parent_id` are added to provenance.
- The composite is placed at the target's position, never held.
- Consumption is gated on `intent_eat`.
- Blocking is a mass threshold answered from the object index, not a
  material bit written to `LAYER_TRAVERSABLE`.
- Channels are 17-22 and 113-117; 109-112 stay unallocated.
- The disabled-section fixture list is Phase 1, 2, 9, 11.
