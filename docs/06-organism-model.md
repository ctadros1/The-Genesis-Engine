# Organism Model

## Phase 2 Implementation Status

The single body plan now carries a validated genome (schema 1) whose trait
genes derive bounded runtime attributes: body scale 0.6-1.6x, maximum speed
0.5-3.0 m/s, sensor range 4-12 m, sensor sensitivity 0.5-1.5x, basal
metabolic multiplier 0.6-1.6x, intake multiplier 0.8-1.2x, maturity age
400-1,200 ticks, per-parent offspring investment basis 3,000-6,000 milli-EU,
and reproduction cooldown 200-600 ticks (all linear maps from normalized
genes, deterministically rounded to fixed point). Pigmentation, thermal
preference, and defense tendency are stored and inherited for analysis but
behaviorally inert in Phase 2. Health, combat, and carcasses remain
unimplemented; the health input channel reads a neutral 1.0.

## Planned Successors (Phases 7 To 14)

The state table below gains entries, each config-gated and absent when its
section is disabled:

| Category | Added state | Phase |
|---|---|---|
| Life | Health, accumulated damage (fixed point) | 7 |
| Genetics | Diploid chromosomal genome; expression recomputed, never persisted as truth | 9 |
| Controller | Per-node activation vector (world state under synchronous evaluation), replacing the fixed 4-value memory vector | 9 |
| Learning | Per-plastic-edge Q16 learned delta and eligibility trace; **reset to zero at birth** | 11 |
| Perception | Nothing persistent; perception reads the previous tick's committed state | 13 |
| Morphology | Module lattice; body derived from the genome, never stored | 10 |
| Inventory | Held object IDs, carried mass | 12 - *as built: `ObjectState::held[i]`, ascending ids, capped by `artifact.max_held_objects`; carried mass is summed from the table, never stored; movement is scaled by `carry_move_cost_q16` and holding costs `hold_cost_milli_per_s`; what a dead organism held is dropped where it died* |
| Demography | Accumulated hazard (fixed point) | 8 |
| Development | Developmental clock, disease load (fixed point) | 14 |

Three documented gaps in this file close: health and carcasses become real
in Phase 7, and the "health input channel reads a neutral 1.0" note stops
being true in the same phase. Ontogeny replaces the age-threshold maturity
model and senescence replaces the hard `max_age_ticks` cutoff in Phase 8,
with developmental growth of a module body following in Phase 14.

Carrying is a genuine tradeoff rather than a free ability: capacity scales
with body scale, and carried mass adds to movement cost through the existing
`C_move` ledger path.

## Initial Body Plan

All initial organisms share one simple body plan: a top-down mobile creature with an evolving color/pattern, size, movement capacity, sensory range, metabolism, diet affinity, attack/defense tendency, and reproductive traits. Pixel-art sprites map visible phenotype bands to genome-derived traits; the scientific overlay shows exact normalized values and trait labels.

This keeps early evolution interpretable. It does not claim a biological species model or promise dramatic morphology evolution.

## State

| Category | Required State | Boundaries |
|---|---|---|
| Identity | Stable organism ID, world lineage, birth tick | Never reused |
| Life | Age, health, energy, alive/dead state, death cause | All finite and bounded |
| Position | x/y, heading, velocity | Clamped to world/terrain constraints |
| Genetics | Genome version, trait genes, neural genes, parent IDs | Validated on birth/load |
| Phenotype | Derived mass, speed, senses, metabolic rates, colors | Recomputed from genome/config |
| Controller | Memory vector and last bounded outputs | Fixed-size only |
| Relationships | Parent IDs, offspring counters, lineage depth | Avoid unlimited in-memory graph expansion |

## Lifecycle

~~~mermaid
stateDiagram-v2
  [*] --> Juvenile
  Juvenile --> Adult: age and energy thresholds
  Adult --> Reproductive: compatible readiness
  Reproductive --> Adult: cooldown or failed mate
  Adult --> Dead: starvation, damage, age, disaster
  Juvenile --> Dead: starvation, damage, disaster
  Dead --> Carcass: edible residual if configured
  Carcass --> [*]: decayed or consumed
~~~

## Sensing And Action

Sensors are bounded local summaries, not unlimited world queries. Initial channels include energy fraction, health fraction, age fraction, local food gradient, water/terrain suitability, nearest organism categories, local temperature, speed, reproductive readiness, and memory values. The spatial index returns bounded candidate sets; exact range/line-of-sight rules are specified in specifications/simulation-tick.md.

Outputs request turn, speed, eat, attack, flee/approach through movement, rest, mate/reproduce intent, and memory writes. The resolver validates every request against state, proximity, energy, cooldown, and terrain.

## Emergent Diet And Predation

Diet affinity is a continuous trait. Organisms can receive energy from plant biomass and, later, carcasses/attacks based on trait compatibility and action outcomes. No organism receives a permanent predator/herbivore class. Reports may classify ecological role from observed intake ratios over a documented time window, never from a static label.

## Aging And Death

Age is tick-derived. Baseline mortality, age-related decline, starvation, combat, environmental stress, and disasters are separate causes. Death is one-way. Carcasses are finite resource entities with decay and consumption controls; they must not duplicate energy.

## Testing

Test lifecycle transitions, death idempotency, no-action-after-death, bounds, parent-child IDs, sensor range limits, invalid controller output rejection, and energy conservation through food/carcass transfers.
