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

Worker count is execution policy, not experiment identity, and is excluded
from the campaign hash. A5.2 asserts that results do not depend on it, so
folding it into the hash would assert the opposite.

The manifest embeds the campaign source **verbatim** and records its hash. A
manifest pointing at an external campaign file would silently change meaning
when that file was edited, which is exactly the failure the config-hash
discipline exists to prevent; an edited manifest is refused on load.

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
