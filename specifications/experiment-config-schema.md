# Experiment Config Schema

## Phase 1 Implementation Notes

Phase 1 implements the in-process subset as `sim_core::SimConfig`
(`CONFIG_SCHEMA_VERSION = 1`): world seed, world/cell dimensions, population
and `max_entities`, `dt_ms`, resource, energetics, crowding, lifecycle, and
worldgen parameters. Validation rejects out-of-range and inconsistent values
before a world can exist, and the canonical hash covers every field plus the
behavior-policy, RNG-algorithm, and worldgen version strings. The CLI sets
fields through flags; a serialized config-file format, intervention lists,
observer budgets, and output policy join in later phases. Any field change
produces a new hash and therefore a new experiment lineage.

Phase 2 adds a `phase2` section (variation probability/magnitudes, pairing
range/compatibility/energy/overhead, controller output thresholds, turn
rate, and similarity-analysis policy). The section participates in the
canonical hash only when `phase2.enabled` is true, together with the
`phase2-behavior-v1`, genome, and controller policy version strings and the
genome schema/topology IDs. A disabled section is behaviorally inert by
construction, so phase2-disabled configs hash and replay identically to
Phase 1 configs; enabling Phase 2 always starts a new replay lineage.

## Phase 5 Implementation Notes: Conditions And Campaigns

Implemented in `crates/sim-experiment`. A campaign file is line-oriented and
hand-parsed, matching this repository's policy of hand-written codecs with
typed rejections rather than a serialization dependency. Every directive is
a whole line, so there is no indentation semantics and no ambiguity about
which condition a `set` belongs to:

~~~
campaign contest-pilot
ticks 20000
workers 4
seeds 1..23 25..28 30..32
base preset phase2
base cells_x 128
condition control
condition treatment
set treatment basal_cost_milli_per_s 160
vary basal_cost_milli_per_s
output events on
output snapshots on
~~~

Four things are validated at **load** time, before any compute is spent,
because a campaign that is wrong is far cheaper to reject than to run:

1. Every condition's effective config validates.
2. All conditions produce pairwise-distinct effective config hashes. Two
   conditions that hash the same are one experiment under two names, and a
   `set` that assigns a field the value it already had is that defect
   wearing a delta.
3. Every field on which two conditions actually differ is declared by a
   `vary` directive, and every `vary` names a field something actually
   varies. This is what lets the comparison report check its aggregation
   precondition against a declaration the author wrote down rather than
   against whatever the data happens to show.
4. **Preflight** (D-046): every declared world is constructed at tick 0. World
   generation rejects seeds whose land fraction falls outside bounds, so a
   declared 30-seed design would otherwise execute as a 27-seed one — and the
   dropped seeds are not dropped at random, they are dropped by a terrain
   property that may correlate with whatever is being measured. A campaign
   that cannot build all of its declared worlds is refused.

`world_seed` is deliberately **not** a settable field. Seeds are the
replicate axis, declared by the `seeds` directive; letting a condition set
one would let a treatment and its control run different worlds.

### Output Directives

`output` selects which artifacts a run writes. It is execution policy, not
experiment identity, so it is outside the effective config hash.

| Directive | Value | Artifact |
|---|---|---|
| `output events` | `on` / `off` | `.alev` event log |
| `output snapshots` | `on` / `off` | `.alif` final snapshot |
| `output compress` | level / `off` | zstd level for the snapshot |
| `output spatial` | ticks / `off` | `.alss` position samples (Phase 7) |
| `output morphology` | ticks / `off` | text morphology series (Phase 10) |
| `output actions` | ticks / `off` | `.alac` per-individual action samples (Phase 11) |

`output actions <ticks>` is off by default and takes a positive tick
interval; `0` is refused with a message pointing at `off`, so a campaign
cannot silently declare an artifact it does not get. It exists because
C11.1's clause is a statement about **two points in one lifetime** - "this
organism's action distribution changed after the patch relocated" - and a
run that records only a terminal census has no *before*. It is binary
rather than a readable series, unlike `output morphology`, on an arithmetic
argument rather than a stylistic one: a morphology sample is six
world-level scalars while an action sample is population x classes.

The exact alignment condition is that `2 x action_interval` divides
`worldmod.relocate_interval_ticks`: the matched control boundary sits at
`relocate / 2` past the event, so `relocate / 2` must be a whole number of
sample windows or the control falls between two samples. The analysis
refuses that world by name (`Misaligned`) rather than interpolating, which
would invent counts. The confirmatory campaign samples every 500 ticks
against a 2,000-tick relocation interval, so `relocate / 2 = 1,000` is two
windows.

`genome2.mutation.point_delta_q16` is settable for a different kind of
reason and is worth stating as a rule. **A campaign that states a threshold
computed from a constant must be able to pin that constant.** Phase 11's
C11.2 bar is one expected mutational step - `point_delta_q16 / 65536` of
the value's range, halved - so leaving the field unsettable would let a
later revision of a default silently move a pre-registered threshold. That
is the coupling D-078 removed for Phase 9's caps. The codec had always
carried the field; only the `FIELD_NAMES` registry entry was missing, which
under standing rule 3 is a visible gap rather than a silent one, and adding
it puts the field under the `config_field_coverage` sweep automatically.

Worker count is execution policy, not experiment identity, and is excluded
from the campaign hash. A5.2 asserts that results do not depend on it, so
folding it into the hash would assert the opposite.

The manifest embeds the campaign source **verbatim** and records its hash. A
manifest pointing at an external campaign file would silently change meaning
when that file was edited, which is exactly the failure the config-hash
discipline exists to prevent; an edited manifest is refused on load.

### Manifest Run Columns: Optional Blocks, Not New Versions

A `run` record is a flat `key=value` line. Columns are **appended, never
renumbered or removed**, and a column that a subsystem owns is emitted only
when that subsystem was enabled for the run. `MANIFEST_VERSION` therefore
does not move when columns are added: phases 8, 9 and 10 each added columns
under version 1, and the archived manifests under `experiments/results/`
still parse.

That works because the reader gates each block on a **sentinel key of that
block's own**, not on a version number:

| Block | Sentinel | Absent means |
|---|---|---|
| Phase 2 pairing | `paired_births` | `phase2` disabled, or the manifest predates the block |
| Structural mutation, 13 classes | `structmut_point_applied` | `genome2` disabled, or predates the block |
| Development, 12 classes | `develop_bodies_grown` | `morphology` disabled, or predates the block |

`action_samples` (Phase 11) is a **count column, not a block**, and it sits
beside `spatial_samples`: it records how many `.alac` segments the run
wrote, and it is `0` when `output actions` is off. Zero is the right value
here rather than absent, because the run genuinely wrote no samples and
nothing reconstructs a rate from it. It is a cross-check with teeth -
`lifesim plasticity` refuses to proceed when a decoded `.alac` scan has a
segment count that disagrees with the manifest's `action_samples`, which is
what catches a truncated or partly-written artifact before it becomes a
thin world rather than a refusal.

The parsed field is `Option`, and **absent is never read as zero**. A cap
rejection and a self-loop draw both land in the summed `structmut_rejected`
column, so "no cap ever bound" is a claim only the per-class columns can
support; a block of thirteen zeros invented for a run that had no schema-2
section would assert it without evidence. `structmut_applied` and
`structmut_rejected` stay alongside the per-class columns because the
archived manifests carry them and a manifest is a record, not a cache.

The key space is flat, so each block prefixes its columns with the name of
the counter set it came from (`structmut_`, `develop_`). Without that,
`rejected_cap` from structural mutation and any future rejection counter
elsewhere would be one key.

Both the renderer and the reader destructure their counter structs with no
`..` (D-077), so adding a counter fails to compile on the writing side -
which is the side that can lose data - rather than being emitted by one and
dropped by the other.

## Original Plan: Conditions And Campaigns (Phase 5)

Every acceptance criterion from Phase 7 onward is a multi-seed,
multi-condition claim, so conditions become a first-class config concept
rather than a convention.

- **A condition is a named config delta with its own canonical hash.** A
  control and a treatment are therefore never confusable for the same
  experiment, and the comparison report refuses to aggregate runs whose
  hashes differ in any field the report does not explicitly name as the
  varied field.
- **A campaign** declares a seed set, a condition set, a run length, and an
  output policy. Its manifest records every effective config hash, per-seed
  final checksums, build provenance, and the analysis versions applied.
- Each new subsystem adds a config section that participates in the
  canonical hash **only when enabled**, following the D-014 precedent:
  `contest`, `genome2`, `plasticity`, `social`, `materials`, `artifacts`,
  `worldmod`, `physiology`. A disabled section is behaviorally inert by
  construction, so a world with all of them disabled hashes and replays
  identically to its predecessor phase and preserves that phase's fixtures.
- Structural and safety caps are config, never constants:
  `max_chromosomes`, `max_loci_per_chromosome`, `max_nodes`, `max_edges`,
  `max_edges_per_node`, `max_genome_bytes`, `max_composition_depth`,
  `max_composition_breadth`, cell occupancy, carry capacity, and
  `dense_threshold_q16` for the terrain-modification representation.
- `lamarckian_fraction_q16` defaults to zero. Any nonzero value is an
  experimental condition that must be reported in every derived result.

The existing Change Rules already say a comparison report may group only
runs that state the relevant differing config fields and must not label them
as identical experiments. Phase 5 makes that a check the tooling performs
rather than a discipline the operator remembers.

## Required Top-Level Fields

| Field | Purpose |
|---|---|
| schema_version | Validate config meaning |
| experiment_id | Stable human/machine identifier |
| world_seed | Reproducible generation/RNG root |
| parent_world/save | Provenance when branching |
| simulation_version | Required build/model compatibility |
| world | dimensions, generator, climate, resources |
| organisms | initial population, genome schema, trait/mutation/reproduction policy |
| tick | dt, strictness, capacity limits |
| observer | default safe stream budgets only |
| interventions | ordered declarative allowed scenario actions |
| output | save/checkpoint/export/metrics policy |

## Validation

Validation rejects unknown required semantic version, non-finite values, inconsistent ranges, unsafe capacities, duplicate intervention IDs, and unsupported feature combinations. It computes a canonical config hash after defaults are materialized; a run stores both the requested config and effective config.

## Change Rules

Config edits before run start create a new hash. Mid-run changes are interventions/config-change events with tick and actor. A comparison report may group only runs that state the relevant differing config fields; do not label them as identical experiments.
