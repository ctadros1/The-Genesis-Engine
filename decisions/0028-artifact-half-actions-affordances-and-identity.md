# ADR-0028: The Artifact Half - Five Verbs, Derived Affordances, Composite Identity, And A World-Gated Channel Registry

Status: Proposed
Date: 2026-08-16
Author: Coding agent (autonomous session)

## Context

Phase 12's mutable-world half shipped on 2026-08-10 (D-097). The artifact
half - material registry, objects as first-class entities, `pick_up` /
`drop` / `place` / `strike` / `combine`, carrying, fracture, combination,
object perception, carcasses as objects - is not started, and C12.1-C12.3
cannot be measured until it exists.

AGENTS.md requires the commissioned review
(`.claude/skills/genesis-mutable-world-tool-use/references/genesis_mutable_world_artifacts_and_tool_use.md`)
to be consulted before any spec, criterion or ADR for objects, and requires a
contradiction between a review and a recorded decision to be resolved in an
ADR rather than silently either way. This is that ADR. Thirteen independent
read passes were made over the review and the code before it was written;
the findings that changed the design are cited inline.

The recorded decisions the review is being reconciled against are
`specifications/artifact-and-material-ontology.md` (design spec, not
implemented), `planning/phase-12-mutable-world-and-artifacts.md` (criteria
C12.1-C12.8, four-condition design), `specifications/determinism-extensions.md`
(Rules 1-10, RNG streams 13/14, checksum tag `lifesim-object-state-v1`,
policy versions `lifesim-material-v1`/`lifesim-artifact-v1`) and ADR-0022
(A2, A3, A4, D2).

**Two facts about the current engine, found by reading rather than assumed,
that constrain everything below.**

1. **The channel registry cannot grow without breaking every schema-2 world.**
   `CHANNEL_REGISTRY_VERSION` is hashed unconditionally inside the genome2
   config block (`config.rs:1784`), so bumping it moves the Phase 9 and 11
   fixtures; and every ALG2 genome stamps the version and the reader refuses a
   mismatch outright (`genome2.rs:783`, `:816`), so bumping it makes every
   schema-2 genome on disk - the 120 format-4 campaign artifacts included -
   undecodable. `registry.rs:11-14` promises the opposite ("does not
   invalidate a single existing schema-2 genome"); the promise is not what the
   code does.
2. **No mutation operator can bind a new channel.** Point mutation on an
   `IoBinding` touches only `gain` (`structmut.rs:706`); insertion adds
   edges or nodes, never bindings (`structmut.rs:928-992`); duplication
   copies a binding's `channel_id` unchanged. `minimal_founder` binds channel
   1 (`energy_fraction`) and channel 101 (`turn`) and nothing else
   (`structmut.rs:1057-1150`). **The set of channels a schema-2 lineage binds
   is therefore fixed at the founder, forever.** Every unbound action fires at
   its neutral value's side of its threshold - `eat` always, `throttle` at
   half - and every unbound sense reads zero. Adding five object action
   channels to this engine, as the spec asks, would add five channels no
   organism could ever request: trap 16 of the evidence-traps memory ("a
   reserved schema slot documented as a working mechanism"), and C12.1's null
   would be mechanically guaranteed and read as ecology.

## Options Considered

**On the action layer.**
- A. The review's four primitives (`grasp`, `release`, `apply_force`,
  `ingest_or_exchange`) with quantized direction/magnitude/duration bins and
  passive bond formation (review 1.4, 7.2, 8.5).
- B. The spec's five verbs as guaranteed semantic operations.
- C. The spec's five verbs, each realised as the review's corresponding
  primitive under grid-world constraints, with `combine` as the review's
  own fallback - a non-guaranteed, energy-costing "generic bond attempt"
  (review 8.5's engineering caveat), and every deviation recorded here.

**On affordances.**
- A. The spec's per-material `affordances` u16 bitset (carryable, strikable,
  combinable, consumable, placeable, blocks_movement).
- B. No stored capability tags: every affordance is a predicate over
  physical quantities, evaluated at the moment of the action.

**On composite identity.**
- A. Review 6.7 / 14.3: members keep their IDs, the assembly is a transient
  `(assembly_root, revision_hash)` handle, no new object ID.
- B. Spec: the composite is a new object from the shared counter; constituents
  are owned by it, leave world iteration, and are retained so fracture
  restores them exactly.

**On the channel registry.**
- A. Bump `CHANNEL_REGISTRY_VERSION` to 2 (moves two fixtures, breaks every
  schema-2 genome on disk).
- B. A world-gated registry: the object channels are registry version 2
  entries that exist only in a world whose artifact section is enabled; a
  genome declares the registry version its bindings need; readers accept 1
  and 2.

**On binding mutability.**
- A. Leave it: the founder binds the object channels directly.
- B. A new structural operator, `bind`, that inserts an `IoBinding` locus
  for a uniformly drawn node and a uniformly drawn channel from the world's
  registry, at a rate that is zero by default and hashed only when nonzero.

## Proposed Decision

### 1. Action layer: option C

The five channels are `pick_up` (113), `drop` (114), `place` (115), `strike`
(116), `combine` (117), all `Output`. IDs 109-112 stay unallocated forever:
`registry.rs:67-81` documents 101..=112 as schema 1's output range with
109-112 the memory outputs that must never become channels, and an ID that
is ambiguous between "reserved" and "free" is not worth four values.

| Verb | Review primitive it realises | What makes it physics rather than a verb |
|---|---|---|
| `pick_up` | grasp | Succeeds only if the target's mass fits the organism's remaining carry capacity and its held count is under `max_held_objects`; contested by (priority, distance squared, organism ID); costs energy whether or not it succeeds |
| `drop` | release | Releases the lowest-ID held object at the organism's own position; refused into a full cell |
| `place` | controlled release | Releases the lowest-ID held object into the adjacent cell the organism faces; refused if that cell is out of bounds, non-traversable for organisms, or at its occupancy cap; **never snaps to a neighbouring free cell** (review 13.5) |
| `strike` | apply_force, brief impulse | Force is a broad monotone function of body scale plus the hardness and mass of what is held; **all strikes on one target in a tick are summed before the fracture test** (review 13.7); a strike that fractures nothing still wears the target by its durability and wears the striker's held object by its own; a strike with no object in reach strikes the terrain cell under the organism and extracts material |
| `combine` | the generic bond attempt of review 8.5's caveat | Not guaranteed: the joint holds with a quality drawn on the pair key and scaled by body scale, and a draw under the floor fails the attempt; costs energy either way; refused only for the physical reasons review 8.10 permits - depth cap, breadth cap, world object cap, nothing held, no target |

`combine` is named in review 18.5's anti-pattern list. It is kept because
(a) the acceptance criteria that were pre-registered against this phase name
it (C12.1 "pick-up, place, and combine", C12.3 "combination depth", condition
D "combination disabled"), and (b) the review's own preferred alternative -
bonds forming from sustained compatible contact - has no physical trigger in
a world whose objects are points with no contact geometry; review 8.5
anticipates exactly this and permits "a generic effector capability such as
secretion or clamping" provided it consumes energy and obeys strength
equations. The deviation and its reason are recorded here so that no reader
of the code takes `combine` for a sanctioned primitive. Review 19.2's
experiment (passive vs pressure-triggered vs clamp bonding) is **not run** in
this phase and is recorded as an open question.

**All Phase 12 actions are single-tick.** No `ActionInstance` record exists
and none is saved; the review's action-persistence machinery (7.5) is not
needed by anything here and its absence is deliberate.

**Every attempted action costs energy** (`action_cost_milli`; `strike` costs
`strike_cost_milli`), success or failure, and losers of a contested pick-up
pay too. Review 7.6: "otherwise agents can probe the world for free."

### 2. Affordances: option B, derived predicates, no bitset

| Spec bit | Replaced by |
|---|---|
| carryable | `mass_milli <= remaining carry capacity` of the organism asking - relational, evaluated per pick-up (review 3.1: affordances are relational) |
| strikable | every object; nothing is exempt from force |
| combinable | every object, subject to depth/breadth/world caps (the breadth cap is degenerate under binary combine - D-116) |
| consumable | `energy_milli > 0` |
| placeable | every held object |
| blocks_movement | `mass_milli >= blocking_mass_milli` - a broad monotone relation on mass, not a tag |

The spec's clause "plus any affordance the combination function grants from
the property combination itself" is deleted: it was a property-to-capability
map, which is a compatibility matrix in disguise (review 18.3). The composite
property function takes property vectors and nothing else.

### 3. Composite identity: option B, recorded as a deviation

A composite is a new object with a fresh ID from the shared counter; its
constituents keep their IDs, are marked owned by it, leave world iteration
(not perceived, not targetable, not decayed), and are retained in full so
fracture restores them at the composite's position with mass and energy
exact. Chosen over review 6.7 because the criteria are stated in terms of
composite objects and their depth, and because exact ledger restoration on
fracture needs the constituent records. What is taken from the review: member
identity persists (a constituent's ID never changes through combine and
un-combine), and there is no separate artifact-only counter or artifact ID
(review 14.3).

Depth: a simple object is depth 0; a composite is `1 + max(constituent
depth)`. C12.3's "depth two or more" is a composite that contains a composite.
This is structural nesting, **not** review 10.2 Level 7's causal-dependency
depth and not Level 10's cumulative culture; the pre-registration must say so.

### 4. Object model

Fields, all fixed point (Rule 7): `id`, `material_id`, `x_fp`, `y_fp`,
`integrity_q16` (i32, 65536 = whole), `mass_milli`, `energy_milli`,
`hardness_q16`, `durability_q16`, `decay_q16` (the last three stored per
object - material's for a simple object, derived for a composite - and
checked against their derivation by `check_invariants`), `holder_id` (0 = in
the world), `owner_id` (0 = free, else the composite that owns it), `depth`,
`created_tick`, `creator_id`, `cause` (Extracted / Fractured / Combined /
Carcass), `parent_id`, and a composition list. **No shape, no orientation,
no height.** Review 5.5 marks geometry required; ADR-0022 D2 defers height
and the collider palette together on cost grounds and this ADR does not
reopen it. The consequences are accepted and named: derived sharpness,
fracture templates keyed to shape, and geometry-based compatibility are all
out of reach in this phase, so review 10.2 Level 5 (pre-use modification)
cannot be claimed here.

Composite properties: mass and energy sum; hardness max; durability min;
decay max; integrity `min(constituent integrity) * joint_quality >> 16`.

Provenance keeps `created_tick`, `creator_id`, and adds `cause` and
`parent_id` (the fractured parent, or the source organism of a carcass), which
is the part of review 12.10's record that the artifact-genealogy analytics
cannot reconstruct from the object table alone. The rest stays in the event
log.

### 5. Material registry

`lifesim-material-v1`, `MATERIAL_REGISTRY_VERSION = 1`, hashed inside the
artifact config section. Four materials, ids 1-4: stone, wood, fiber,
carcass, each with `hardness_q16`, `density_milli`, `durability_q16`,
`decay_per_tick_q16`, `energy_content_milli` (per 1,000 milli-mass). No
toughness, no friction, no adhesion, no toxicity: the review's admission
gate (5.2) asks each to earn its place by enabling a distinguishable
behaviour, and in a point-object world without contact geometry none of the
four does yet. Wear is the material's durability applied to the object's
integrity per strike; there is no separate wear accumulator.

`carcass` is the review's sanctioned "schema-level exceptional matter such as
organism tissue" (5.7) and is documented as such.

Material IDs select parameter records; no rule branches on a material ID.
This is asserted by a rule-coverage test that perturbs each material's
properties and confirms every outcome moves continuously.

### 6. Perception: cues, not labels

Five input channels, 17-21: `object_present`, `object_distance`,
`object_bearing`, `object_heft` (target mass over the perceiver's own carry
capacity, clamped - relational by construction), `object_hardness` (target
hardness over the registry maximum), plus 22 `carried_load` (held mass over
capacity, the review's "proprioceptive lifting effort"). The spec's
`object_material` and `object_composite_depth` are **not** exposed: a
material ID is the internal label review 1.3/5.1 forbid organisms to
perceive, and composite depth is an observer-side quantity (ADR-0022 A3
forbids observer labels as cues). K = 1 (nearest free object within
`perception_range_m`), reduced with the `(distance_squared, id)` key that
`nearest_within` already uses for organisms; for K = 1 a min-reduction with
that key is the sorted-truncated set of Rule 5.

### 7. The world-gated channel registry: option B

`CHANNEL_REGISTRY_VERSION` stays 1 and keeps its meaning. The registry gains
`CHANNELS_V2`, the eleven entries above, and a constant
`CHANNEL_REGISTRY_VERSION_ARTIFACT = 2`. A world's registry version is a
function of its config: 2 when the artifact section is enabled, else 1. The
genome2 config-hash block hashes the world's registry version, which is 1
for every world that exists today, so no fixture moves. The ALG2 encoder
stamps the smallest version that covers the genome's bindings - 1 for every
genome that exists today, so every byte-identity test and every artifact on
disk is unchanged - and the decoder accepts 1 and 2 and validates each
binding against the declared version's channel set. A genome that binds a
version-2 channel is refused at world construction and at restore in a
version-1 world (typed, fail closed), and cannot arise by mutation there
because `bind` draws only from the world's own registry.

### 8. Binding mutability: option B, the `bind` operator

`genome2.mutation.binding_q16`, default 0, hashed only when nonzero, at
field granularity per D-014's precedent for `plasticity_enabled`. When it
fires, `bind` inserts one `IoBinding` locus - node uniform over present
nodes, channel uniform over the world's registry (inputs and outputs alike;
direction is the channel's property), gain uniform in [-1, 1], homology ID
hash-derived as `insert` does. Deletion already removes bindings. This is
recorded as the artifact half's most consequential change to the engine
outside objects: **it is the first time a schema-2 lineage can bind a
channel its founder did not.** All four campaign conditions carry it at the
same rate, so it is common-mode for C12.1-C12.3, and it is not enabled by
default. Whether Phases 9-11 should be re-run with it on is an open
question, recorded in `docs/21-open-questions.md`, not answered here.

### 9. Carcasses become objects

With the artifact section enabled, death creates a carcass object with a
fresh ID from the shared counter (`parent_id` = the organism, `creator_id` =
0, `cause` = Carcass), energy = the contest section's `carcass_energy_q16`
share of remaining energy, mass = energy, integrity 65536. It decays by the
registry's carcass rate (energy exponentially, exactly as `contest.carcasses`
does today; integrity linearly), is consumed passively by any organism within
`consume_reach_m` with room and `intent_eat` set (the Phase 7 rule, gated on
the existing `eat` channel so that consumption is an organism's action and
not the world's), and is destroyed at zero energy. `ContestState.carcasses`
is bypassed in such a world and untouched otherwise. Requires the contest
section, as carcasses always have. Held consumables are within reach by
definition, so an organism can carry food and eat it later.

### 10. Ledger

Objects get their own exact ledger, mirroring the carcass pool: mass in
(extracted, carcass), mass out (decayed, consumed, dust); energy in
(carcass, extracted energy content), energy out (decayed, consumed, dust).
"Dust" is the named sink for a fragment below `min_fragment_mass_milli`, an
object refused by the world cap, and a simple object whose integrity reaches
zero. Consumption credits the organism's `assimilated_milli` exactly as
carcasses do today. `check_invariants` asserts both identities to the milli.
Terrain yield regenerates outside the object ledger, as biomass regrows
outside the energy ledger; extraction is the source term.

### 11. Fracture and wear

Force `f = strike_force_q16 * body_scale / 1000 + sum(held hardness *
held mass / strike_mass_reference)`. Per target per tick, forces are summed.
If `sum >= hardness * fracture_margin_q16 >> 16`: a composite comes apart
(constituents restored to the world at its position; the composite is
destroyed; mass and energy exact by construction), a simple object fractures
into `k` fragments, `k` drawn on the Artifact stream keyed `(tick,
target_id)`, each `mass / k` with the remainder to the lowest new ID, energy
likewise, and any fragment under `min_fragment_mass_milli` goes to dust.
**The fracture draw is keyed on the target, not on the striker-target pair**
as the spec says: damage is aggregated across strikers (review 13.7), so
"the striker" is not well defined when two strike at once; the review's own
key (8.3) is striker-independent. Otherwise integrity drops by the target's
durability; at zero a composite comes apart and a simple object goes to
dust. The striker's held objects each lose their own durability per strike.

### 12. Blocking, occupancy and the reserved terrain layer

An object with `mass_milli >= blocking_mass_milli` in a cell blocks
*entry* to that cell; it does not evict or trap the organism already there,
and `check_invariants`'s "no organism on a non-traversable cell" is unchanged
because objects are not composed into `effective_traversable`. Blocking is
answered from the object cell index, not by writing `LAYER_TRAVERSABLE`; the
layer's producer stays "digging", which is not built. `LAYER_MATERIAL_YIELD`
gets its first producer and consumer: a terrain strike depletes the cell's
remaining yield (absent = baseline `terrain_yield_milli`) and yield
regenerates on `yield_regen_interval_ticks` toward baseline. Which material a
cell yields is a function of its elevation band, a physical rule with two
config thresholds and no table.

### 13. Persistence and identity numbers

- Snapshot section `SECTION_OBJECTS = 15`, guarded on `FORMAT_VERSION_7`,
  optional, present iff the artifact section is enabled.
- **ALIF format 7**: the artifact config block and `mutation.binding_q16`
  are appended to the positional config block. `format6.rs`'s chain table
  gains a row and its assertion becomes "the older body is a byte prefix of
  the newer, and the newer adds exactly the declared number of bytes", since
  this format adds a block rather than one byte (D-112 anticipated that a
  non-one-byte extension "would need its own reasoning"; this is it).
  Retained format-6 reader and writer, `FORMAT6_TO_CURRENT` migration,
  `encode_snapshot_format6` refuses an artifact-enabled state with
  `FieldNotInFormat`.
- `SAVE_STATE_VERSION` stays 2. The next-ID counter keeps its field name
  `next_entity_id`; Rule 2's "becomes `next_object_id`" is satisfied
  semantically - objects draw from it - and a rename would move nothing
  but text. `check_invariants`'s allocation identity gains an
  objects-allocated term.
- Checksum section `lifesim-object-state-v1` is appended **after**
  `lifesim-action-census-v1`, i.e. last, not in the position the Rule 8
  table lists it (before terrainmod). Appending never moves a world that
  lacks the section; inserting would move every worldmod world. The table is
  amended.
- Event schema 6, tags 14-22. Benchmark schema unchanged (no new tick
  phase: object actions run inside `Apply` after `contest_phase`, decay
  inside `Lifecycle`).
- RNG: `Artifact = 13` (fragment count, joint quality), `MaterialYield = 14`
  (extraction variance). Contested acquisition has no lottery in this
  version; ties resolve on organism ID, so no stream is consumed and the
  question of which stream a lottery would use is left open rather than
  guessed.

### 14. Config

One section, `artifact`, hashed only when enabled, appended after `probe`.
The `experiment-config-schema.md` line listing `materials` and `artifacts`
as two sections is corrected: the registry is a build constant versioned
inside the section, and a world with materials but no object system means
nothing. Fields are listed in the specification. Every field is registered
in `FIELD_NAMES`. `artifact.enabled` requires `genome2.enabled` (schema 1's
fixed topology cannot bind the channels) and `worldmod.enabled` (the yield
layer lives there). A world cap on objects exists (`max_objects`) because
C12.7 names object count as a cap that must reject visibly; the
`long-horizon-soak.md` line saying the world total is deliberately uncapped
is corrected.

## Consequences

Positive: C12.1-C12.3 become measurable on physics that names no outcome;
every deviation from the review is written down beside its reason; four
fixtures are preserved by the same D-014 mechanism every phase has used; the
one change outside objects (`bind`) is off by default and common-mode in
the campaign.

Negative: `combine` remains a verb the review would rather not see, and the
experiment that would settle whether a passive alternative is learnable is
not run. Objects have no geometry, which caps the tool-use hierarchy this
phase can reach at Level 4. A format bump costs a retained reader, a
migration and a chain-table row.

Compatibility: schema-2 genomes on disk decode unchanged; the config hash of
every existing world is unchanged; the Phase 1, 2, 9, 11 fixtures are asserted
unchanged by tests and by the verify scripts.

## Performance Implications

Hypothesised, to be measured in `bench_phase12`: object actions are
O(population x candidates within reach) per tick; the object cell index is
O(objects) per tick; decay is O(objects) per tick; perception adds one
bucket scan per organism. The snapshot gains a section whose size is
measured beside the checkpoint budget (plan: "re-verified here, not
assumed").

## Operational Implications

None outside the repository. Rollback is `artifact.enabled = false`, which
runs the pre-existing code paths byte for byte.

## Revisit Conditions

- The Phase 12 cost measurement comes in under the D2 gate, which reopens
  height/support and, with it, geometry.
- A measured null on C12.1 whose diagnosis is "the actions are the wrong
  shape", which would motivate review 19.2's experiment.
- An accepted ADR for a self-describing config block, which would retire
  format bumps for config fields.

## Evidence Required To Accept

- All four fixtures reproduce with the section disabled (tests and verify
  scripts, both hosts).
- Ledger exactness over 10^6 ticks (C12.6).
- Every cap rejects, counts and events (C12.7), each verified by driving it.
- The registry-gating test: a genome that binds a v2 channel is refused in a
  v1 world; every schema-2 artifact on disk decodes.
- Mutation testing by an agent that did not write the tests.
