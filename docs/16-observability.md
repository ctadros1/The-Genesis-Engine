# Observability

## Purpose

Operational metrics answer whether the simulation is healthy; scientific metrics describe what the simulated ecosystem did. Keep the two labels distinct so a drop in population is not mistaken for a host outage.

## Prometheus Metrics

| Metric Family | Key Labels | Meaning |
|---|---|---|
| lifesim_tick_duration_seconds | world, phase | Tick latency histogram by phase |
| lifesim_tick_rate_hz | world | Effective completed tick rate |
| lifesim_organisms | world, life_state | Live and terminal entity count |
| lifesim_species_clusters | world, algorithm_version | Analytical cluster count |
| lifesim_births_total / deaths_total | world, cause | Lifecycle rates |
| lifesim_food_biomass | world, biome | Resource availability |
| lifesim_neural_evaluations_total | world | Controller work rate |
| lifesim_mutation_events_total | world, genome_version | Mutation activity |
| lifesim_genetic_diversity | world, analysis_version | Documented sampled diversity metric |
| lifesim_average_energy | world | Mean live-organism energy |
| lifesim_average_lifespan_seconds | world | Mean terminal lifespan over reporting interval |
| lifesim_predator_prey_intake_ratio | world | Observed animal/carcass to plant intake ratio |
| lifesim_stream_bytes_total | world, client_class | Observer bandwidth |
| lifesim_observer_dropped_updates_total | world, reason | Backpressure behavior |
| lifesim_save_duration_seconds / save_bytes | world, format_version | Persistence cost |
| process_* | service | Process RSS, CPU, file descriptors |

Avoid high-cardinality labels such as organism ID, client ID, genome hash, arbitrary seed, or request path parameters.

## Events And Logs

Structured logs include timestamp, world ID, tick, component, severity, correlation ID, and safe error code. Major simulation events include births, deaths, extinctions, config changes, interventions, saves, restores, and capacity limits. Logs cannot replace append-only event records for replay/audit.

## Grafana Dashboard Plan

Create one operational dashboard for tick rate, p95 phase duration, RSS, save health, observer bandwidth, and errors. Create a separate world dashboard for population, births/deaths, resources, diversity, lifespan, and predator/prey intake ratio. Do not edit existing dashboards until a named owner, datasource, folder, and rollback export are confirmed live.

## Alert Philosophy

Alert on sustained failure to tick, repeated crash/restart, checkpoint failure, disk-space danger, invalid-save spike, persistence latency, or unexpected resource exhaustion. Population collapse is an experiment event unless a configured safety rule says otherwise. Establish thresholds from Phase 0/4 baselines, not guesses.
