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

## Planned Additions (Phases 5 To 18)

All follow the existing conventions: `lifesim_` prefix, `_total` counters,
bounded label enumerations, no high-cardinality labels.

| Phase | Metric | Type |
|---|---|---|
| 5 | `lifesim_worlds_active`, `lifesim_world_ticks_total{world}` | gauge, counter |
| 5 | `lifesim_event_log_bytes_total`, `lifesim_event_log_dropped_total` | counter |
| 7 | `lifesim_damage_events_total`, `lifesim_deaths_total{cause="damage"}`, `lifesim_carcasses` | counter, counter, gauge |
| 9 | `lifesim_genome_bytes`, `lifesim_controller_nodes`, `lifesim_controller_edges`, `lifesim_structural_mutations_total{operator}`, `lifesim_structural_rejected_total{cap}` | gauge, gauge, gauge, counter, counter |
| 11 | `lifesim_plastic_edges`, `lifesim_plasticity_faults_total`, `lifesim_plasticity_energy_spent_milli_total` | gauge, counter, counter |
| 13 | `lifesim_signals_emitted_total`, `lifesim_signal_energy_spent_milli_total`, `lifesim_perceived_neighbours` | counter, counter, gauge |
| 12 | `lifesim_objects{material}`, `lifesim_object_actions_total{action,result}`, `lifesim_composition_depth`, `lifesim_terrain_modified_cells` | gauge, counter, gauge, gauge. *Status 2026-08-16: not exported by `sim-server` yet (it exports the base gauges only, as for every phase since 7); the kernel-side numbers exist and reach the operator through the run manifest - `artifact_<counter>` for all 30 `ObjectCounters`, `artifact_ledger_<term>` for the ten ledger terms, `artifact_objects_total/free`, `artifact_composites_depth2`, `artifact_placed_total`, `artifact_organism_ticks` - and through `lifesim artifact`* |
| 8 | `lifesim_median_lifespan_ticks`, `lifesim_juvenile_fraction`, `lifesim_deaths_total{cause}` distribution | gauge, gauge, counter |
| 10 | `lifesim_modules`, `lifesim_nonviable_births_total` | gauge, counter |
| 14 | `lifesim_modules_grown_total`, `lifesim_growth_energy_spent_milli_total`, `lifesim_juveniles_growing`, `lifesim_mate_choices_total`, `lifesim_scrambled_choices_total` | counter, counter, gauge, counter, counter. *Status 2026-09-02: kernel-side in `MetricsSnapshot` (`modules_grown_total`, `growth_spent_milli_total`, `juveniles_growing`, `choices_total`, `scrambled_choices_total`), exported through the run manifest and the schema-10 fixture line; `sim-server` export follows the phase-12 note above* |
| 14 | `lifesim_disease_load` | gauge. *Deferred with the disease slice (ADR-0030 decision 3)* |

Two label rules specific to the new goal:

- **Signal channel content is never a label.** Unbounded cardinality, and it
  encourages reading meaning into channels that have none.
- **Detected segments and tradition findings are never metrics.** They are
  analysis report contents. A metric is an operational or scientific
  time series; an analysis finding is a versioned artifact with provenance
  and, for traditions, a mandatory genotype-matched control.

## Phase 11 Manifest Column: `action_samples`

A `run` record gains `action_samples`, the number of `.alac` segments the
run wrote, beside the existing `spatial_samples`. It is a count column
rather than an optional block (see
`specifications/experiment-config-schema.md`), so `0` means the run wrote no
per-individual action samples and nothing reconstructs a rate from it.

It is load-bearing rather than informational: `lifesim plasticity` refuses a
world whose decoded `.alac` segment count disagrees with this column, along
with two further provenance guards pinned separately by their own
diagnostics - the log's config hash and its terrain checksum against the
run's. A truncated or partly-written artifact therefore fails closed instead
of arriving as a thin world, which the analysis would otherwise report as
`too_few_individuals` - a refusal that reads like a property of the world.

## Phase 11 Analysis Reports

Both follow the Analysis Report Provenance rules below. Neither analysis
version is in the config hash (D-022, ADR-0016).

### `plasticity-report 1` (`lifesim-plasticity-analysis-v1`)

Line-oriented `key=value`, emitted by `lifesim plasticity`. Line kinds, in
order:

| Line | Carries |
|---|---|
| `plasticity-report 1 campaign <id>` | Format version and campaign id |
| `analysis-version` | `lifesim-plasticity-analysis-v1` |
| `census-policy` | `lifesim-action-census-v1`, the convention the columns mean something under |
| `plan` | Every threshold verbatim: `burn_in_ticks`, `min_individuals`, `shift_bar`, `control_ceiling`, `drift_bar`, `drift_margin_milli`, `permutations`, `analysis_seed` |
| `columns` | The action-class names, in column order |
| `world` | One per world: `rho_milli`, `null_p95_milli`, `boundaries`, `individuals`, `pairs`, `discarded`, the event/control median distances, `event_wins`/`control_wins`/`ties`, `distinct_distances`, `varying_columns`, `column_totals`, `no_variance`, and the two decisions `shift` (directed) and `associated` (two-sided). A refused world prints `refused=<reason>` instead and never a zero |
| `drift` | One per world: the C11.2 allele census, both excesses, both `moved_*` counts, and `selected` |
| `condition` | One per arm: counts and medians over its worlds |
| `contrast` | Seed-paired between-arm comparisons with intervals and an equivalence result |
| `criterion` | `treatment_count`, `control_count`, `bar`, `ceiling`, `treatment_undefined`, `met` |

Three properties of the format are deliberate. **Every threshold is echoed
verbatim** so a reader checks the report against the campaign source rather
than trusting a summary. **A refusal is a typed reason, never a zero**, so
"this world could not be read" and "this world showed no change" are never
the same line. And **the directed and two-sided decisions are printed side
by side**, with only the directed one reaching `criterion`, so a
sign-reversed association is visible rather than hidden by a directed rule.

### `conjunction-census 1` (`lifesim-conjunction-census-v1`)

Emitted by `lifesim conjunction`. Descriptive only, and the format says so
on its own third line: `kind DESCRIPTIVE CENSUS of already-collected
snapshots. No threshold, no null, no verdict. Not a hypothesis test and must
not be reported as one.` That line is printed on every report and not only
in the archived results file, because a table of counts detached from it
reads exactly like a test result - and this one is computed with the
campaign's outcome already known.

A `conditions` line names what each counted condition is, then three line
kinds per world: `world` (per edge **allele**, both haplotypes, with a
conjunction-depth histogram and a rule histogram), `expressed` (per compiled
plastic edge, which is what the learn phase actually visited), and `learned`
(the learned-state section, including the count of nonzero rows, the
maximum, quantiles, and both the recomputed and the reported mean - the pair
that makes D-098's truncation visible in the report itself). Arm totals
follow.

Because it has no verdict, it may never appear in a criterion's evidence
chain. It bounds interpretation of a null; it cannot establish one.

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

## Phase 5 Metrics

Two server gauges/counters added with asynchronous checkpointing:

| Metric | Type | Labels | Meaning |
|---|---|---|---|
| `lifesim_checkpoint_capture_seconds` | gauge | `world`, `mode` | Wall-clock cost of the last state capture. In asynchronous mode this is the **only** part of a checkpoint the tick thread pays for, so it is exported separately from `lifesim_save_duration_seconds`, which covers the whole write |
| `lifesim_checkpoints_skipped_total` | counter | `world` | Automatic checkpoints refused because the previous one was still being written |

`lifesim_checkpoints_skipped_total` is deliberately a metric rather than a
log line. A nonzero value is a configuration fact — the checkpoint interval
is shorter than a checkpoint takes — and the alternative designs both lie
about it: an unbounded queue turns a latency problem into a memory problem,
and a silent drop makes the configured interval untrue.

The batch runner and the event-log verifier emit single-line JSON summaries
(`batch_schema_version`, `event_log_schema_version`) on stdout, in the same
style as the existing `run`/`fixture` summaries. Campaign results proper
live in the manifest, not in metrics: a campaign is not a time series.

## Benchmark Record

Every benchmark result stores: benchmark schema version, git revision, date, host/VM profile, OS, CPU flags, toolchain/build profile, config hash, seed, world size, observed clients, warmup/run duration, raw sample path, p50/p95/p99, RSS, allocation and bandwidth notes. Benchmark IDs cannot be reused for different conditions.
