# Metrics Schema

## Naming

Use prefix lifesim_. Counters end in _total; durations use seconds; bytes use bytes; gauges expose current bounded values. Labels have bounded enumerations only.

## Required Metrics

| Name | Type | Labels |
|---|---|---|
| lifesim_tick_duration_seconds | histogram | world, phase |
| lifesim_tick_rate_hz | gauge | world |
| lifesim_organisms | gauge | world, life_state |
| lifesim_births_total | counter | world |
| lifesim_deaths_total | counter | world, cause |
| lifesim_food_biomass | gauge | world, biome |
| lifesim_save_duration_seconds | histogram | world, result |
| lifesim_stream_bytes_total | counter | world, client_class |
| lifesim_observer_dropped_updates_total | counter | world, reason |
| lifesim_invalid_input_total | counter | component, reason |

## Phase 1 Implementation Notes

The headless CLI (`lifesim run --metrics-out`) emits the subset that exists
in Phase 1: `lifesim_organisms`, `lifesim_births_total`,
`lifesim_deaths_total{cause="starvation"|"old_age"}`, `lifesim_food_biomass`
(single `biome="all"` label until biomes exist), the host-measured
`lifesim_tick_duration_seconds` histogram, and `lifesim_tick_rate_hz`.
Phase 1 adds one metric to this schema: `lifesim_births_rejected_capacity_total`
(counter, label `world`), recording deterministic birth rejections at the
`max_entities` process-safety ceiling. Save/stream metrics start with their
phases.

Phase 2 adds (emitted only when Phase 2 is enabled):
`lifesim_paired_births_total` (counter, `world`),
`lifesim_pair_rejected_total` (counter, `world`,
`reason=capacity|placement|energy`), `lifesim_controller_faults_total`
(counter, `world`), and `lifesim_max_ancestry_depth` (gauge, `world`).

Phase 3: `lifesim-server`'s `/metrics` endpoint additionally emits
`lifesim_stream_bytes_total{client_class="observer"}`,
`lifesim_observer_dropped_updates_total{reason="backpressure"}`,
`lifesim_observers` (gauge), and `lifesim_ticks_total`.

Phase 4 adds `lifesim_saves_total{result="ok"|"error"}` (counter),
`lifesim_save_duration_seconds{result="last"}` (gauge; the histogram form
arrives with a longer-lived metrics pipeline), and `lifesim_save_bytes`
(gauge).

## Planned Additions (Phases 5 To 12)

All follow the existing conventions: `lifesim_` prefix, `_total` counters,
bounded label enumerations, no high-cardinality labels.

| Phase | Metric | Type |
|---|---|---|
| 5 | `lifesim_worlds_active`, `lifesim_world_ticks_total{world}` | gauge, counter |
| 5 | `lifesim_event_log_bytes_total`, `lifesim_event_log_dropped_total` | counter |
| 6 | `lifesim_damage_events_total`, `lifesim_deaths_total{cause="damage"}`, `lifesim_carcasses` | counter, counter, gauge |
| 7 | `lifesim_genome_bytes`, `lifesim_controller_nodes`, `lifesim_controller_edges`, `lifesim_structural_mutations_total{operator}`, `lifesim_structural_rejected_total{cap}` | gauge, gauge, gauge, counter, counter |
| 8 | `lifesim_plastic_edges`, `lifesim_plasticity_faults_total`, `lifesim_plasticity_energy_spent_milli_total` | gauge, counter, counter |
| 9 | `lifesim_signals_emitted_total`, `lifesim_signal_energy_spent_milli_total`, `lifesim_perceived_neighbours` | counter, counter, gauge |
| 10 | `lifesim_objects{material}`, `lifesim_object_actions_total{action,result}`, `lifesim_composition_depth`, `lifesim_terrain_modified_cells` | gauge, counter, gauge, gauge |
| 11 | `lifesim_median_lifespan_ticks`, `lifesim_juvenile_fraction`, `lifesim_disease_load` | gauge, gauge, gauge |

Two label rules specific to the new goal:

- **Signal channel content is never a label.** Unbounded cardinality, and it
  encourages reading meaning into channels that have none.
- **Detected segments and tradition findings are never metrics.** They are
  analysis report contents. A metric is an operational or scientific
  time series; an analysis finding is a versioned artifact with provenance
  and, for traditions, a mandatory genotype-matched control.

## Analysis Report Provenance

Every analysis report (similarity, era, tradition) records: analysis
algorithm version, analysis tick range, config hash, seed, world ID,
save-state and event-schema versions, simulation build version, sampling
policy and bounds, and every threshold and penalty used. Analysis versions
are deliberately **not** in the config hash, because an analysis can never
affect a world.

Tradition findings additionally record the genotype-matched control
statistic, its matching tolerance, and the cohort size. A finding without
them fails report validation.

Negative results are reported explicitly. A run with no segments and no
traditions produces a report saying exactly that; an empty report is not the
same as no report.

## Benchmark Record

Every benchmark result stores: benchmark schema version, git revision, date, host/VM profile, OS, CPU flags, toolchain/build profile, config hash, seed, world size, observed clients, warmup/run duration, raw sample path, p50/p95/p99, RSS, allocation and bandwidth notes. Benchmark IDs cannot be reused for different conditions.
