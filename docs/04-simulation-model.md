# Simulation Model

## Model Status

This is a proposed, adjustable model. Values are defaults for experimentation, not claims of biological accuracy. Every configuration has a version/hash; changing it creates a new experiment lineage.

## Phase 1 Implementation Status

`crates/sim-core` implements the Phase 1 subset of this model as versioned
policy `phase1-behavior-v1` (hashed into every config):

- Implemented: fixed 100 ms tick, canonical phase order, named RNG streams
  (`lifesim-rng-v1`), logistic vegetation regrowth with a one-milli-unit
  seeding floor, continuous fixed-point positions (1/1024 m) over raster
  cells, food-gradient movement with one-in-eight deterministic jitter,
  water/boundary movement rejection, basal/movement/crowding energy costs,
  intake-capped feeding, config-gated asexual reproduction with placement and
  capacity validation, aging, starvation/old-age death, latched extinction,
  and the `max_entities` birth-rejection ceiling (never culling).
- All state is integer fixed point: energy/biomass in milli-units, fractions
  in Q16. Energy and biomass changes flow through an explicit ledger and are
  verified exactly by `World::check_invariants`.
- Not yet implemented (deferred to their phases): neural controllers,
  sensing beyond the food gradient/crowding summary, combat, carcasses,
  thermal cost, climate/day/night/seasons, disasters, interventions, and
  persistence.

Every constant above is experimental policy, not doctrine; changing any of
them changes the config hash and starts a new replay lineage.

## Phase 2 Implementation Status

With `phase2.enabled` (policy `phase2-behavior-v1`), the same world adds:
genome-derived phenotypes scaling metabolism, movement cost, intake, speed,
sensing, and reproduction life history; controller-driven continuous
movement (heading in binary angular measure with integer Bhaskara
sine/cosine, throttle-scaled speed, water rejection unchanged);
controller-gated feeding; paired-parent reproduction replacing the Phase 1
single-source path (which remains available only with `phase2.enabled`
off); ancestry state; and controller-fault accounting. The Phase 2 config
section is hashed only when enabled, so disabled configs and worlds remain
bit-identical to Phase 1 — including fixtures and checksums. Enabling
Phase 2 always starts a new replay lineage. The tick gains an explicit
`controllers` phase between sense and apply. Float usage is confined to
genome values and controller evaluation with add/multiply/divide and a
rational tanh approximation only; world positions, energy, and biomass stay
fixed-point integers.

## Planned Successors (Phases 5 To 18)

The model below is unchanged for Phases 1 and 2. Phases 5 through 18 add
mechanisms whose designs live in `specifications/`; each is a config-gated
section, folded into the config hash only when enabled, and behaviorally
inert when disabled so all earlier fixtures reproduce exactly. The extended
determinism contract that governs all of them is
`specifications/determinism-extensions.md`, which is normative and takes
precedence over anything in this document that contradicts it.

| Addition | Policy version | Phase | Design |
|---|---|---|---|
| Health, damage, contest, carcasses | `contest-behavior-v1` | 7 | `planning/phase-7-territory-and-conflict.md` |
| Diploid variable-topology genome | `lifesim-genome-v2` | 9 | `specifications/genome-schema-2.md` |
| Modular morphology and development | `lifesim-morphology-v1` | 10 | `specifications/morphology-and-development.md` |
| Synaptic plasticity | `lifesim-plasticity-v2` | 11 | `specifications/plasticity-and-learning.md` |
| Perception and signalling | `lifesim-social-v1` | 13 | `specifications/social-signal-channel.md` |
| Materials, objects, world modification | `lifesim-material-v1`, `lifesim-artifact-v2`, `lifesim-worldmod-v1` | 12 | `specifications/artifact-and-material-ontology.md`, `specifications/mutable-world-state.md` |
| Allometry, thermoregulation, senescence, extrinsic mortality | `lifesim-demography-v1` | 8 (executes after 7) | `planning/phase-8-demography-and-life-history.md` |
| Developmental ontogeny, sexual selection, disease | `lifesim-physiology-v2` | 14 (executes after 13) | `planning/phase-14-ontogeny-and-sexual-selection.md` |

Three sections below become live rather than documented placeholders:
combat and damage in Phase 7, `C_thermal` in the energy equation in Phase 8, and carcasses in Phase 7.

The tick order gains phases: `learn` after `apply` (Phase 11), and object
decay plus terrain-modification application inside `lifecycle` (Phase 12).
*As built (2026-08-16): no new named phase for objects - the artifact half
runs inside the existing ones (yield regeneration in `environment`, the
object index in `spatial_index`, cues in `sense`, `artifact_phase` at the
end of `apply`, death drops, carcass objects and decay in `lifecycle`), so
`TickPhase::ALL` stays at 9 and benchmark schema 7 is unchanged.*
Each new phase is empty when its section is disabled, so simulation results
are unaffected; only the per-phase benchmark shape changes, which increments
the benchmark schema version.

## Coordinate And Time Model

- Organisms use continuous world coordinates (meters in simulation units).
- Environmental state uses square raster cells (default cell width is a tunable 4 simulation meters).
- World bounds are finite. Crossing a coastline or outer boundary is prevented by terrain/collision rules; there is no toroidal wrapping.
- The canonical tick is fixed. Prototype default: dt = 0.1 simulated seconds at 10 ticks/second.
- Live display rate is independent from tick rate. Headless mode may process ticks as fast as safely possible.
- Day/night and seasons are parametric periodic functions over simulation time, with their periods recorded in config.

## Deterministic Tick

~~~mermaid
flowchart LR
  A["Apply queued validated interventions"] --> B["Update clock, climate, resources"]
  B --> C["Build spatial index"]
  C --> D["Sense and evaluate neural controllers"]
  D --> E["Resolve intents in stable order"]
  E --> F["Movement, feeding, combat, reproduction"]
  F --> G["Aging, death, carcasses, cleanup"]
  G --> H["Events, metrics, snapshot scheduling"]
~~~

The server completes each phase for the whole world before the next phase. Intent resolution uses stable entity-ID ordering or a documented deterministic tie breaker. All random draws use named streams derived from world seed, tick, system, and entity/cell identity.

## Random Streams

The canonical random source is a pure derivation:

    rng_key = derive(world_seed, tick, system_id, subject_id, draw_index)

| Input | Range | Purpose |
|---|---|---|
| world_seed | unsigned 64-bit value | Root identity of a replay lineage |
| tick | unsigned 64-bit value | Separates time steps |
| system_id | bounded enum | Separates generator, weather, mutation, combat, and other domains. Values are permanent and appended only; the full registry including planned values 7 to 16 is in `specifications/determinism-extensions.md` |
| subject_id | entity/cell/stable scope ID | Separates individual draws within a system |
| draw_index | bounded unsigned integer | Separates multiple draws for one subject/action |

No system reads a shared mutable RNG. Adding a random draw in one system must not shift an unrelated system's sequence. The exact counter-based or hash-derived algorithm is an implementation decision, but its identifier/version belongs in world metadata and deterministic fixtures.

## Energy Accounting

Energy uses arbitrary Energy Units (EU), not calories. For organism i:

    E_next = clamp(E + I_food + I_carcass - C_basal - C_move - C_thermal - C_action - D_damage, 0, E_max)

| Variable | Unit | Default/Range | Meaning |
|---|---|---|---|
| E | EU | 0 to E_max | Current stored energy |
| I_food | EU/tick | >= 0 | Assimilated plant/resource intake |
| I_carcass | EU/tick | >= 0 | Assimilated carcass intake |
| C_basal | EU/tick | >= 0 | Metabolic maintenance |
| C_move | EU/tick | >= 0 | Movement cost |
| C_thermal | EU/tick | >= 0 | Climate discomfort cost |
| C_action | EU/tick | >= 0 | Attack/reproduction/other action cost |
| D_damage | EU/tick | >= 0 | Damage converted to energy/health loss |

Proposed forms:

    C_basal = dt * basal_rate * mass^metabolic_exponent
    C_move = dt * movement_factor * mass * speed^2

The exponent, rates, max energy, and speed caps are configuration values. Failure modes include runaway size advantage, starvation spirals, and numerical accumulation; test monotonicity, bounds, and long-run population behavior.

## Resource Growth

Each vegetation cell has biomass B, carrying capacity K, growth rate r, and harvested amount H:

    B_next = clamp(B + dt * r * B * (1 - B / K) - H, 0, K)

K and r depend on biome, temperature, moisture, season, and disturbance. Water is a terrain/environmental requirement initially, not a globally consumed volume model. Introduce water depletion only after a benchmarked/validated use case.

## Proximity, Movement, And Collision

The spatial index divides the world into cells at least as large as the configured largest interaction range. An organism queries its own and neighboring cells only. For organisms i and j:

    contact(i, j) = distance_squared(pos_i, pos_j) <= (radius_i + radius_j)^2

| Variable | Unit | Default/Range | Meaning |
|---|---|---|---|
| pos | simulation meters | within world bounds | Continuous x/y coordinate |
| radius | meters | genome-derived, > 0 | Collision/feeding reach body radius |
| distance_squared | meters squared | >= 0 | Avoids unnecessary square roots |
| interaction range | meters | versioned config cap | Sensor/action query limit |

Movement integrates a requested heading and speed only after terrain, boundary, and energy checks. A movement request that would enter water or leave the bounded continent is projected to the nearest valid result or rejected according to the versioned collision policy. It never produces coordinates outside the world arrays.

## Feeding, Combat, Damage, And Healing

Feeding and attack are validated intents, not direct controller effects. For a successful feeding interaction:

    intake = min(available_resource, intake_rate * dt, remaining_energy_capacity / assimilation_efficiency)
    energy_gain = intake * assimilation_efficiency

For a successful attack:

    raw_damage = max(0, attack_strength * attack_signal - defense_strength * defense_signal)
    health_next = clamp(health - raw_damage + healing, 0, health_max)
    healing = min(healing_rate * dt, available_energy / healing_energy_cost)

| Variable | Unit | Default/Range | Meaning |
|---|---|---|---|
| intake | biomass units/tick | >= 0 | Resource removed from plant/carcass target |
| assimilation_efficiency | fraction | (0, 1] | Energy conversion ratio |
| attack_strength/defense_strength | abstract force | bounded phenotype values | Trait-derived action capacity |
| raw_damage | health units/tick | >= 0 | Validated attack result |
| health | health units | 0 to health_max | Life integrity separate from energy |
| healing_rate | health units/second | >= 0 | Configuration/phenotype limit |

Combat resolution uses the same deterministic contention policy as other intents. Damage, healing, resource removal, and energy transfer are recorded as explicit events. A health value at zero creates a terminal death transition in the cleanup phase; it cannot be healed after death.

## Reproduction, Aging, Death, And Extinction

Reproduction requires valid adult state, energy reserve, compatible mate/parent policy, cooldown, proximity, and process capacity. Parent energy investment is debited before offspring allocation. A new organism starts at a bounded nearby valid position and records parent IDs, genome hash, tick, and schema/config versions.

Age advances by dt. Death can result from starvation, health depletion, age policy, disaster, or explicit validated removal. A death transitions once to a finite carcass or removal record; carcass energy decays and cannot exceed the source's recorded remaining transferable energy. A world is extinct when live organism count reaches zero; it remains a valid paused/observable/savable world and emits one extinction event per extinction transition.

## Climate

Temperature is an abstract Celsius-like field:

    T = base_temp - lapse_rate * elevation + season_amplitude * sin(season_phase) + weather_noise

Weather noise must use deterministic cell/time streams and bounded amplitude. Moisture evolves through rainfall, evaporation, terrain drainage, and sea proximity using a deliberately simplified model. The project should label it ecological simulation, not weather forecasting.

## Population Protection

The primary limiter is ecological: food, space, metabolism, predation, and reproduction cost. A process safety ceiling max_entities exists to prevent host exhaustion. At ceiling, the server rejects new births deterministically, emits a capacity event, and records that the experiment was resource-capped. It must never silently delete living organisms to meet capacity.

## Invariants

- Energy cannot appear except through explicit environmental/resource transfers.
- Dead entities do not sense, act, reproduce, or receive future intents.
- Each removed organism produces one terminal event with a cause.
- Entity IDs are never reused inside a world lineage.
- All mutation, birth, death, and intervention events include tick and schema/config version.

## Test Requirements

Unit-test every formula for normal, boundary, negative, and non-finite input. Seed tests must prove same-build deterministic replay. Long-run tests must detect NaN propagation, entity leaks, energy-accounting drift, and unbounded event buffers.
