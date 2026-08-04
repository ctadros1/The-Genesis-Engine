# Artifact And Material Ontology Specification

Status: design specification, not implemented. Phase 11. Policy versions
`lifesim-material-v1`, `lifesim-artifact-v1`.

## Problem

There is no object ontology of any kind. No materials, no items, no
carrying, no combination. Structures and tools require objects that exist
independently of organisms, persist after their makers die, and have
physical properties an organism can exploit.

## The Authored Boundary, Restated For This Spec

Authored: materials have hardness, mass, durability, energy content, and a
set of affordances. Objects can be picked up, carried, dropped, placed,
struck, and combined. Composite properties derive from constituents by a
stated function. Carrying costs energy proportional to mass.

Never authored: a recipe list, a crafting table, a tool taxonomy, a
technology tree, a set of "valid" combinations, or any reward for making
anything. There is no table anywhere in the system that says
`stone + stick = axe`. There is a combination function over material
properties, and whatever it produces is what exists.

## Material Registry

A bounded versioned registry. Registry version enters the config hash.
Materials are world physics, not content: adding one is a config change with
a new lineage, not a feature toggle.

| Property | Type | Meaning |
|---|---|---|
| `material_id` | u16 | Registry key |
| `hardness_q16` | u32 | Resistance to fracture when struck |
| `density_milli` | i64 | Mass per unit volume |
| `durability_q16` | u32 | Integrity loss rate under use |
| `decay_per_tick_q16` | u32 | Passive integrity loss in the world |
| `energy_content_milli` | i64 | Assimilable energy if consumed; zero for stone |
| `affordances` | u16 bitset | Carryable, strikable, combinable, consumable, placeable, blocks movement |

The starting registry is deliberately small and physical: stone (hard,
dense, no energy, negligible decay), wood (moderate hardness, moderate
decay, low energy), fiber (soft, light, fast decay), and carcass (soft, high
energy, fast decay). Carcass ties the object system to the existing
lifecycle: `docs/06-organism-model.md` already specifies carcasses as finite
resource entities, and Phase 11 is where they become real objects rather
than a documented gap.

Materials enter the world from terrain: cells carry a material yield
determined by terrain layers, drawn through the `MaterialYield` stream when
extracted. Extraction is a `strike` against the cell.

## Object Model

An object is a first-class entity in the shared object ID space
(`specifications/determinism-extensions.md` Rule 2).

| Field | Type | Notes |
|---|---|---|
| `object_id` | u64 | From the shared monotonic counter; never reused |
| `material_id` | u16 | For a simple object |
| `x_fp`, `y_fp` | i32 | Continuous position, same fixed-point units as organisms |
| `integrity_q16` | i32 | Fixed point; decays; zero destroys the object |
| `mass_milli` | i64 | Derived from material and size |
| `holder_id` | u64 | 0 when in the world, else the carrying organism |
| `composition` | bounded list | Constituent object IDs for a composite; empty for a simple object |
| `depth` | u8 | Combination depth; capped by `max_composition_depth` |
| `created_tick` | u64 | Provenance |
| `creator_id` | u64 | The organism that placed or combined it; 0 for terrain-derived |

Objects are stored struct-of-arrays sorted by `object_id`, compacted on
removal in the same manner as organisms, with the ordering verified by
`check_invariants`.

## Actions

New action channels in the versioned channel registry. An organism binds
them or does not; unbound actions are never requested.

| Action | Effect | Failure modes, all typed and evented |
|---|---|---|
| `pick_up` | Acquire the highest-priority reachable carryable object | Not reachable, not carryable, already held, capacity exceeded |
| `drop` | Release a held object at the organism's position | Nothing held |
| `place` | Release a held object at a chosen adjacent position, setting `creator_id` | Nothing held, invalid position, cell occupancy full |
| `strike` | Apply damage to a target object or terrain cell | Nothing in range, insufficient force |
| `combine` | Attempt to join the held object with a target object | Nothing held, no target, affordance mismatch, depth cap, breadth cap |

Target selection for every action uses the sorted-candidates rule: the
candidate set is materialized, sorted by `(distance_squared, object_id)`,
truncated, and selected from. Contested acquisition between two organisms in
the same tick is resolved by (action priority, distance squared, object ID),
then by a `lifesim-pairkey-v1` lottery if configuration enables one.

### Carrying

- Carry capacity is `phenotype_body_scale * carry_capacity_factor`, so
  carrying is a body-plan tradeoff rather than a free ability.
- Carried mass adds to movement cost through the existing `C_move` path in
  the energy ledger. Carrying a heavy object is genuinely expensive, which
  is what makes "put it down here and come back" a strategy that could pay.
- Held objects move with the holder and do not participate in cell
  occupancy.
- On death, held objects are dropped at the death position in ascending
  object-ID order.

### Striking and fracture

`strike` applies force derived from the striker's phenotype against the
target's hardness. If force exceeds hardness by the configured margin, the
target fractures: it is destroyed and replaced by a bounded number of
fragment objects whose material is determined by the target's material and
whose count and integrity are drawn from the `Artifact` stream keyed on the
canonical pair key of striker and target.

Striking terrain extracts material from a cell, subject to the cell's yield
and the `MaterialYield` stream. Extraction depletes the cell's yield, which
regenerates on a configured schedule. This is where raw material enters the
world.

### Combination

`combine` succeeds when both objects' affordance bitsets include
`combinable`, the resulting depth is within `max_composition_depth`, and
the resulting constituent count is within `max_composition_breadth`.

There is no recipe check. Composite properties derive from constituents:

    mass         = sum of constituent masses
    hardness     = max of constituent hardnesses
    durability   = min of constituent durabilities
    energy       = sum of constituent energy contents
    integrity    = min of constituent integrities, scaled by joint_quality
    affordances  = intersection of constituent affordances,
                   plus any affordance the combination function grants
                   from the property combination itself

`joint_quality` is a Q16 value derived from the combining organism's
phenotype and a draw from the `Artifact` stream on the canonical pair key.
A poor joint gives a composite that fractures easily; a good one gives a
durable object. Combination therefore has a skill gradient without any
authored notion of skill.

The composite is a new object; constituents become owned by it and are
removed from world iteration but retained in the composition list so
fracture can restore them. This keeps mass and energy accounting exact,
which matters because the energy ledger is a hard invariant.

## Persistence In The World

Placed objects remain until integrity reaches zero through decay or damage.
This is the property that makes Phase 11 different from every earlier phase:
an object outlives the organism that made it, and later organisms encounter
a world that earlier organisms shaped.

Objects are perceivable: the channel registry gains `object_present`,
`object_distance`, `object_bearing`, `object_material`, and
`object_composite_depth` channels for the K nearest objects, gathered under
the same sorted-candidate rule as neighbours.

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

The recorded minimum viable subset, scheduled as a Phase 11 stretch item
gated on the Phase 11 cost measurement:

- one integer `height_interval` per object, not a continuous z coordinate;
- a `supported_by` object reference forming an explicit support graph,
  acyclic by construction and validated at decode;
- a support rule: an object whose supporter is destroyed either falls to the
  next supporting interval or is destroyed, by material policy;
- no rotation, no toppling, no torque, no partial overlap.

**Until this lands the plan does not claim stacked construction**, and no
document may describe a placed-object arrangement as a built structure in
the vertical sense. Object placement remains planar.

## Cell Occupancy And Movement

An object whose material has the `blocks_movement` affordance occupies its
cell for movement purposes. Movement into a blocked cell is rejected by the
existing collision policy path, which already rejects water and out-of-bounds
movement. Nothing new is needed in the movement resolver beyond an added
rejection reason.

Occupancy per cell is capped by config. A `place` into a full cell is
rejected deterministically, counted, and evented.

## Invariants

- Object IDs strictly increase and never repeat within a lineage.
- Object storage index order equals object-ID order.
- Every constituent in a composition list exists and is owned by exactly one
  composite.
- No object is simultaneously held and placed.
- No object is in more than one cell occupancy list.
- Mass and energy accounting through creation, combination, fracture, decay,
  and consumption is exact to the milli-unit, verified by an extension of the
  existing ledger. Combination and fracture must be mass-neutral and
  energy-neutral; a rounding remainder is assigned deterministically to the
  lowest constituent object ID, following the existing convention for
  reproduction investment remainders.
- A dead organism holds nothing.

## Test Requirements

- Ledger exactness across a long run with heavy object churn.
- Fracture and combination round trip: combining then fracturing restores
  constituent mass and energy exactly.
- Cap enforcement for depth, breadth, occupancy, and carry capacity, each
  deterministic, counted, and evented.
- Contested acquisition is order-independent under storage permutation.
- Save round trip with composites of depth greater than one, and with held
  objects.
- Disabled-section equality: artifacts disabled reproduces the Phase 12
  fixture exactly.
- Seeded malformed-input harness over the object table codec.
