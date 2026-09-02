# World Save Format

## Phase 4 Implementation Notes (ALIF format 1)

Implemented in `crates/sim-persist` over `sim_core::SaveState` (logical
state version 1). Header: magic `ALIF`, format version, header length,
flags (bit 0 = zstd), world ID, parent world ID, tick, seed, config hash,
save-state and genome schema versions, build-version string reference,
event-log offset (zero until the event-log file exists), uncompressed and
stored lengths, payload CRC32, state checksum, and terrain checksum  - 
little-endian throughout, matching the kernel's canonical hashing.
Payload sections (tagged, length-prefixed, per-section CRC32): config,
world metadata, organism table, biomass field, ledger/counters, and the
Phase 2 table (genomes, controller memory, heading/speed, ancestry).
Static terrain is deliberately not stored: it regenerates from
`(seed, config)` on load and must match the recorded terrain checksum, so
a snapshot can never be silently reinterpreted against different terrain.
All lengths are capped before allocation or decompression; decoded state
passes full kernel validation (genome validity, ordering, bounds, exact
ledger conservation) and the recorded state checksum before a world
exists. Unknown format versions fail closed through the explicit
migration registry (`sim_persist::migration_for`); no transforms are
registered yet because only format 1 exists. Atomic write, catalog
ordering, recovery, and restore-verification behavior follow the Write
Contract and Restore Test sections below and are covered by the
`sim-persist` test suite.

## Phase 11 Implementation Notes: Section 14, The Action Census

Implemented in `crates/sim-persist/src/codec.rs` as
`SECTION_ACTION_CENSUS = 14`, present exactly when
`config.probe.action_census_enabled`. Body: `ACTION_CLASS_COUNT` `u32`
columns per organism, in organism order, preceded by the organism count.

**Dense, unlike the sparse learn section beside it (tag 12).** Only some
edges are plastic, so a learn record names the edge it belongs to; every
living organism has an action every tick, so a sparse histogram would carry
an index next to almost every entry and save nothing.

**The format version stays 4.** This follows tag 12's precedent rather than
tag 13's: format 4 was bumped because the logical state gained a meaning (a
composed terrain checksum in the header), not because a section was
appended. An absent optional section is readable by every existing build, so
a snapshot of a world without the probe is byte-identical to the one this
build would have written before section 14 existed, and all five fixtures
are untouched. The decoder still **refuses tag 14 in a format 3 file**, so a
legacy file carrying it is a typed error rather than a section read under a
framing that never defined it.

`ACTION_CENSUS_BYTES_PER_ORGANISM` bounds the allocation a declared organism
count implies and is never used to assert an exact body length (D-075);
exactness is enforced by the trailing-bytes check every section runs.

The census is also **state**, so it is hashed under
`lifesim-action-census-v1` in `World::state_checksum`, appended after Phase
12's section. See `specifications/determinism-extensions.md` Rule 8. One
consequence is deliberate and is stated rather than hidden: `reset` moves
the checksum, so a probe boundary is part of the replay lineage.

The per-individual sample series that C11.1 reads is a **separate artifact**
and not part of the snapshot: `.alac` format 1, described in
`crates/sim-persist/src/actionlog.rs`, with a self-checking 72-byte header,
per-segment CRC32, strictly ascending segment ticks, and the entity id in
every 44-byte record so an analysis keys on the organism rather than on the
array slot.

## Phase 11 Implementation Notes: ALIF Format 5, One Config Byte

Implemented in `crates/sim-persist/src/codec.rs`. `FORMAT_VERSION` is 5;
`FORMAT_VERSION_4` is retained, with `encode_snapshot_format4` and
`decode_snapshot_format4` permanent members of the build.

**The whole difference is one byte appended to the end of the config section
body**: `plasticity.live_rule_zero`, written unconditionally at format 5 and
absent at format 4.

### Why this one is a version bump when sections 12, 13 and 14 were not

Every optional *section* added since format 3 is absent from a world that
does not have it, so a build that predates one reads a file lacking it
unchanged. The config block has no such property. `encode_config` is
positional and unconditional, so one new field shifts `worldmod` and `probe`
by a byte and every existing format-4 file decodes as
`ValueOutOfRange("section trailing bytes")` or, worse, as plausible garbage.

The 120 format-4 campaign artifacts are still read for re-analysis, so
breaking them is not acceptable. The alternative - appending the field and
letting old files fail - is a mistake format 3 already made quietly: Phase 11
grew the config block by seventeen bytes *within* format 3, so the retained
format-3 reader can only read the format-3 files this build's own writer
produces. That is survivable there because no pre-Phase-11 format-3 file
exists. It would not be survivable here. This is recorded as an open item
rather than repaired; see `docs/21-open-questions.md`.

### The byte is appended, not filed with its own block

`live_rule_zero` belongs to the plasticity config and is written after
`probe`, which is the last block. The reason is a property worth more than
the grouping: **appended, the format-4 config body is a byte prefix of the
format-5 body for the same world.** That is one assertion a test can make
(`the_format_4_config_body_is_the_format_5_body_without_its_last_byte`) and a
reader can check by eye. Filed next to `plasticity.lamarckian_fraction_q16`,
the two bodies would diverge from that offset on, `worldmod` and `probe`
would sit at different offsets in the two formats, and the only way to state
the difference would be to re-describe the layout.

This ordering is **not** the config *hash* order and the two must not be
conflated. `SimConfig::stable_hash` appends for a different reason - its
order is the definition of every hash already issued - and gates each section
on being enabled. The codec block is unconditional.

### Both directions fail closed on the body, before the header

A format-5 body read as format 4 leaves one byte over and fails the
trailing-bytes check every section runs. A format-4 body read as format 5
runs out of body on the last field and fails `TruncatedSection`. Neither
depends on the header's version word, which is not checksummed - so a file
with a forged version word is refused too, in both directions.

### The logical state version does not move

`SAVE_STATE_VERSION` stays 2. Format 5 adds a config *field* and changes no
existing field's meaning. Version 2 was bumped when terrain stopped being a
pure function of `(seed, config)`, which is a change of meaning; this is not
one, and Phase 11 set the precedent in the other direction by adding four
config fields and a whole optional section without moving it.

### The registry's transforms all land on the current format

`FORMAT3_TO_CURRENT`, `FORMAT4_TO_CURRENT`, and - since format 6 -
`FORMAT5_TO_CURRENT`. None chains: `decode_snapshot_migrating` applies exactly
one transform, so a format-3 file goes to the current format in one hop and
never becomes an intermediate format-4 or format-5 file.
Both declare `expected_loss: ""` and both are entitled to. A format-4 world
ran with `live_rule_zero` false by construction, not by default: no build
that could write a format-4 file had a live rule 0 to switch on.

The transforms are named for their **source** and not their target, because
the target moves. `FORMAT3_TO_FORMAT4`'s `to_format` was already
`codec::FORMAT_VERSION`, so the day format 5 landed the constant said 5 while
the name said 4.

### A version guard names the format that introduced the section

`SECTION_WORLDMOD` and `SECTION_ACTION_CENSUS` are guarded against
`FORMAT_VERSION_4`, not `FORMAT_VERSION`. Written against the current
version, the comparison is correct only while the introducing format is
current; at format 5 it becomes "refuse these sections in every format-4
file", which is every campaign artifact on disk, reported as an error naming
the section rather than the comparison.

### The write side refuses what it cannot express

`encode_snapshot_format4` returns `CodecError::FieldNotInFormat` for a state
whose `live_rule_zero` is set, rather than dropping the field. That error is
distinct from `SectionNotInFormat` because a config field has no tag and the
refusal is write-side only. Silently writing the file would produce one that
describes a world with rule 0 dead - the "never alter meaning during load"
rule broken one step earlier, where nothing downstream can detect it.

### Status of the field itself

Encoded and migrated here; **not yet behavioural**. `SimConfig::validate`
refuses `live_rule_zero == true`, on the same grounds it refuses a nonzero
`lamarckian_fraction_q16`: an accepted flag that changed nothing would hand a
campaign an arm bit-identical to its own control and report the result as a
null. The refusal comes out in the change that remaps the rule in
`compile_with_budget` (D-107 option A3). The field is **not** in the config
hash until then, because a hash difference claims a replay-lineage split and
there is not one yet.

## Phase 11 Implementation Notes: ALIF Format 6, A Second Config Byte

`FORMAT_VERSION` is 6; `FORMAT_VERSION_5` is retained with
`encode_snapshot_format5` and `decode_snapshot_format5`, and
`FORMAT5_TO_CURRENT` joins the registry. The added field is
`plasticity.price_moved_edges_only` (D-111, the moat).

Everything in the format-5 section above applies unchanged, and the
**repetition is the point**: a positional, unconditional config block cannot
grow without a version bump, and that cost does not amortise. A third config
field should expect the same, or an ADR should propose a self-describing
config block - which would have to explain how an absent trailing field avoids
being meaning altered on load, the rule that makes defaulting one
unacceptable.

### The recurring test trap, and its single guard

D-108 recorded two instances of one shape: a test named for a format that
builds its subject with the *current-format* writer, which is the same thing
only until the next bump. Format 6 produced a third, in `format5.rs`.

`format6.rs::each_adjacent_format_adds_exactly_one_config_byte` replaces the
class. It walks a table of retained writers - format 4, format 5, current -
and asserts for every adjacent pair that the newer config body is the older
one plus exactly one byte, that the older body is a byte prefix of the newer,
that the appended byte is the flag's false value in a default world, and that
the whole file grows by exactly one byte.

**Adding format 7 means adding one row to that table.** A format that does not
extend its predecessor by exactly one byte names itself in the failure
message.

Format 3 is deliberately outside the chain: it differs from format 4 by a
*section* and a logical-state version, not by a config byte, and
`phase12_format4.rs` owns that comparison.

## Phase 12 Implementation Notes: ALIF Format 7 And Section 15, Objects

Implemented in `crates/sim-persist/src/codec.rs` (ADR-0028, D-115).
`FORMAT_VERSION` is 7; `FORMAT_VERSION_6` is retained with
`encode_snapshot_format6` and `decode_snapshot_format6`, and
`FORMAT6_TO_CURRENT` joins the registry (`store::resolve_format7_defaults`
supplies the absent artifact block and `binding_q16 = 0`, both of which are
"section off", so a format-6 world restores to the same replay lineage it
was saved from).

**The bump was forced by two config fields, not by the objects.** Format 7
appends to the positional config block, in this order: the artifact section
(`enabled`, `inert`, `ephemeral`, the six caps, the three carry fields, the
three costs and the threshold, the three reaches, the five strike/fracture
fields, `joint_floor_q16`, `blocking_mass_milli`, the five yield fields, the
two material weights - `FORMAT7_CONFIG_BYTES` in total, asserted by
`format7.rs::each_adjacent_format_adds_the_declared_config_bytes`, which is
the format-6 chain table with a per-row byte delta instead of the fixed
"one byte"), then `genome2.mutation.binding_q16` (u16). The schema-2
counters section gains one trailing u64, `binding_applied`, at format 7 and
above. Format 6 is refused as a write target for any world with the artifact
section enabled or a nonzero bind rate (`refuse_format7_state`), for the
same reason format 4 refused format 3: the older file could not say what the
world was.

**Section 15 (`SECTION_OBJECTS`), present when `artifact.enabled`**, did not
by itself need the bump (the tag-12/tag-14 precedent: a new section under a
new tag is only ever present when its config flag is on, and a file with the
flag off simply lacks it). Its body:

    u64 count
    count x { u64 id, u16 material, i32 x_fp, i32 y_fp, i32 integrity_q16,
              i64 mass_milli, i64 energy_milli, u32 hardness_q16,
              u32 durability_q16, u32 decay_q16, u64 holder_id, u64 owner_id,
              u8 depth, u64 created_tick, u64 creator_id, u8 cause,
              u64 parent_id, u64 composition_len, composition_len x u64 }
    u64 objects_allocated_total
    10 x i128 ledger (ObjectLedger::FIELD_NAMES order)
    30 x u64 counters (ObjectCounters::FIELD_NAMES order)
    u64 population; population x { u64 exposure_ticks, u64 carry_ticks, u8 birth_band }

Every count is bounded before allocation with the exact fixed minimum an
item occupies (`OBJECT_FIXED_BYTES = 100`, which *includes* the
composition-length word). D-115's evidence-trap entry records why that
sentence is in the spec: a first draft counted that word twice, the bound was
8 bytes per object too strict, and every table above roughly fifty objects
was refused as oversized while a three-object round-trip test passed.
`format7.rs::a_two_hundred_object_table_round_trips` is the guard.

Restore rebuilds the per-cell occupancy index and the held-object slots from
the table (`ObjectState::from_table`, `rebuild_held`, `rebuild_cell_index`)
and refuses, by name (`RestoreError::StateInvalid("object table: ...")`), a
table whose ids are not strictly ascending, whose composition names an
absent or unowned constituent, whose depth exceeds the cap or is not
1 + max(constituents), or whose ledger's expected mass and energy differ from
the table's (`ObjectTable::violation`); a `holder_id` naming an organism that
is not in the save is refused separately. Object-versus-organism id
collision is checked by `World::check_invariants`, which the restore tests
call. None of that is stored twice.

The Phase 1, 2, 9 and 11 fixtures are unmoved: a world without the section
hashes no artifact block (D-014) and reports channel-registry version 1.

## Phase 13 And 14 Implementation Notes: Formats 8, 9 And 10

Each follows the pattern formats 5 through 7 established - the config body
stays a byte prefix of its successor's, the delta is a constant asserted
by the chain test in `format7.rs`, every retained writer refuses a state
carrying what only a later format can express, and each migration resolves
the appended fields to the defaults every older world actually ran with:

- **Format 8** (Phase 13): the social config block
  (`FORMAT8_CONFIG_BYTES`) and `SECTION_SOCIAL` (16), the committed signal
  field and per-organism social table.
- **Format 9** (Phase 14, ADR-0030): the physiology-v2 ontogeny block
  (`FORMAT9_CONFIG_BYTES`, 21 bytes) and `SECTION_ONTOGENY` (17) - the
  per-organism grown-prefix counts and payments, D-091 allocation
  discipline on the count-led arrays. The growth *order* is not stored: it
  is a pure BFS function of the body, rebuilt on load exactly as bodies
  are.
- **Format 10** (Phase 14, ADR-0030): the two mate-choice gates
  (`FORMAT10_CONFIG_BYTES`, 2 bytes) and `SECTION_MATECHOICE` (18),
  counters only - the preference weights are expressed from the genomes on
  load, on the terms phenotypes are.
- **Format 11** (Phase 15, ADR-0031): the chemistry config block
  (`FORMAT11_CONFIG_BYTES`) and `SECTION_CHEMISTRY` (19) - per-cell
  substrate concentrations plus the field ledger, stored never recomputed
  (ADR-0020). The scratch buffer and production-weight map are rebuilt
  caches.
- **Format 12** (Phase 15, ADR-0031): the microbial config fields
  (`FORMAT12_CONFIG_BYTES`, 33 bytes) and `SECTION_MICROBIAL` (20) -
  per-cell per-class densities plus the attribution counters. The
  mutation scratch buffer is a rebuilt cache.
- **Format 13** (Phase 15, ADR-0031): the two coupling fractions
  (`FORMAT13_CONFIG_BYTES`, 8 bytes). Byte-shaped like formats 5 and 6 -
  no new section. ADR-0031 wrote "format 11" for the whole phase;
  implementation split it across three increments exactly as Phase 14
  split ADR-0030 across formats 9 and 10, and each landed with its own
  retained writer, refusal, and migration.
- **Format 14** (Phase 16, ADR-0032): the transition config block
  (`FORMAT14_CONFIG_BYTES`, 41 bytes: gate, check interval, density
  floor, persistence checks, aggregation step, organism energy, the two
  caps), the origin-mode byte's third value (`scratch` = 3, accepted only
  at format 14 and above), and `SECTION_TRANSITION` (21) - the per-slot
  persistence counters (`u32` each, real state: the trigger's window
  cannot be recomputed from densities) plus the six transition counters.
  The per-class eligibility table is a rebuilt cache. Retained format-13
  writer and reader, `refuse_format14_state` (a non-default transition
  config, a transition section, or a scratch origin is refused by every
  pre-14 writer), `FORMAT13_TO_CURRENT` resolving the section absent and
  the config default - no build that wrote format 13 could materialize.

## Planned Successor: ALIF Format 2 (Phase 12)

Design: `specifications/mutable-world-state.md`. Decision: ADR-0015.

The format 1 property that terrain is not stored, regenerates from
`(seed, config)`, and is checksum-verified is load-bearing and cannot
survive organism-modified terrain. It is **split, not abandoned**:

- `baseline_terrain_checksum`: the regenerated baseline is still verified
  against `(seed, config)` using the unchanged `lifesim-worldgen-v1`
  generator. This is byte-for-byte the format 1 check and still fails
  closed. A save still cannot be reinterpreted against a different generated
  world.
- `composed_terrain_checksum`: verified after the stored modification delta
  is applied in ascending `(layer_id, cell_index)` order.

New payload sections, each tagged, length-prefixed, and per-section
checksummed like the existing ones:

| Section | Contents | Present when |
|---|---|---|
| Contest | Health, damage counters | contest enabled |
| Genome 2 | Diploid variable-topology genomes. No ID allocator is stored: the four identity fields are derived by hash (ADR-0022 A8), so there is no counter to persist | schema 2 |
| Activations | Per-node activation vector | schema 2 |
| Learned state | Per-plastic-edge Q16 deltas and traces, sparse | plasticity enabled |
| Signal field | Committed signal field | social enabled |
| Objects | Artifact table, composition lists, per-cell occupancy | artifacts enabled |
| Terrain modification | Sparse or dense delta, flagged in the header | mutable world enabled |
| Physiology | Developmental stage, hazard, disease load | physiology enabled |
| Action census (tag 14, Phase 11) | Dense per-organism action histogram, `ACTION_CLASS_COUNT` u32 columns | `probe.action_census_enabled` |

Header changes: `next_entity_id` becomes `next_object_id` (organisms and
artifacts share one monotonic ID space); the baseline and composed terrain
checksums replace the single `terrain_checksum`; a representation flag
selects sparse or dense modification encoding; the event-log offset stops
being zero once Phase 5 lands the log file.

Save-state version increments to 2.

### Migration, and what it must not do

**Format 1 is never reinterpreted in place.** A format 1 file loads through
a registered transform in `sim_persist::migration_for` that produces an
empty modification set, an empty object table, and absent optional sections.
The acceptance requirement is byte identity: the migrated result must equal
the world produced by loading the same file under a format 1 reader. Format
1 readers and their tests stay in the build permanently so that comparison
is always available.

Unknown format versions continue to fail closed through the registry.

## File Layout

Header: magic ALIF, format version, header length, flags, world ID, parent world ID, tick, world seed, simulation/build version string reference, config hash, generator/genome schema versions, uncompressed length, compressed length, payload checksum.

Payload sections: world metadata; terrain/static fields; dynamic environmental fields; entity component tables; genome table; event-log checkpoint reference; deterministic RNG/config state; optional analytics summary. Sections have tagged IDs, lengths, and per-section checksums when added.

## Write Contract

Write temporary file in destination filesystem, flush, checksum, atomically rename, then commit catalog metadata. A catalog record never claims a successful save until the final file validates. Save format must be endian-defined; all decoded lengths are capped before allocation/decompression.

## Migration

A migration declares source/target format, supported semantic versions, transform, tests, expected loss if any, and rollback. Unknown save versions fail closed with an actionable error. Never deserialize raw Rust layout or rely on compiler struct order.

## Restore Test

Load a save in an isolated destination, validate checksum/header/schema, rebuild derived indexes, pause at recorded tick, compare documented state checksum, and only then make it eligible for a world branch or replacement.
