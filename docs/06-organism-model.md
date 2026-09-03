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
| Perception | Nothing persistent; perception reads the previous tick's committed state | 13 - *as built: nine cues per neighbour slot for up to `perception_k` (max 4) nearest conspecifics (registry channels 23..58), a committed signal field on `signal_in[0..4]` (59..62), and emission on `signal_emit[0..4]` (118..121) that costs energy remainder-exact; see Social Perception And Signaling below* |
| Morphology | Module lattice; body derived from the genome, never stored | 10 |
| Inventory | Held object IDs, carried mass | 12 - *as built: `ObjectState::held[i]`, ascending ids, capped by `artifact.max_held_objects`; carried mass is summed from the table, never stored; movement is scaled by `carry_move_cost_q16` and holding costs `hold_cost_milli_per_s`; what a dead organism held is dropped where it died* |
| Demography | Accumulated hazard (fixed point) | 8 |
| Development | Grown-module prefix count and payment toward the next module (fixed point, `lifesim-ontogeny-state-v1`); the growth *order* is a pure BFS function of the body and is never stored. Disease load deferred with its slice (ADR-0030 decision 3) | 14 - *as built: `OntogenyState`; founders admitted fully grown, children at `birth_modules_min`* |

Three documented gaps in this file close: health and carcasses become real
in Phase 7, and the "health input channel reads a neutral 1.0" note stops
being true in the same phase. Ontogeny replaces the age-threshold maturity
model and senescence replaces the hard `max_age_ticks` cutoff in Phase 8,
with developmental growth of a module body following in Phase 14.

*As built, 2026-09-02 (ADR-0030, D-127):* the developed body is revealed
one module at a time in canonical BFS order from the origin, each
activation paid through the ledger, and every juvenile constraint - speed,
sensor range, carry, energy capacity - is the partially grown body's own
derived attribute recomputed through the same `Phenotype::apply_body` the
birth path uses. Maturity requires a fully grown body on top of the Phase
8 age gate, so growth stalled by energy shortage delays reproduction. The
controller is the recorded exception: compiled at birth from the adult
body's neural budget (lifetime topology change is a first-policy non-goal
of the learning stack). Mate choice, in the same phase: pairing selects
the candidate with the highest evolved-preference score over its nine
perceived cue values (the sense path's own formulas, one shared function)
rather than the nearest, ties and the all-neutral preference reproducing
proximity pairing exactly; the preference is nine trait-band loci
(`docs/08`), the act is the unchanged `intent_mate` output, and each
formed pairing is recorded with its true cues and candidate-set cue sums
(event tag 27).

Carrying is a genuine tradeoff rather than a free ability: capacity scales
with body scale, and carried mass adds to movement cost through the existing
`C_move` ledger path.

## Initial Body Plan

All initial organisms share one simple body plan: a top-down mobile creature with an evolving color/pattern, size, movement capacity, sensory range, metabolism, diet affinity, attack/defense tendency, and reproductive traits. Pixel-art sprites map visible phenotype bands to genome-derived traits; the scientific overlay shows exact normalized values and trait labels.

This keeps early evolution interpretable. It does not claim a biological species model or promise dramatic morphology evolution.

*As built (2026-09-02, Phase 16, ADR-0032): a `scratch` world has no
initial body plan at all. Its organisms enter by materialization from
the microbial field as one-module bodies - a single digestive module,
the unicellular case of the Phase 10 morphospace - with the minimal
schema-2 founder genome at every trait's midpoint, built by the same
admission code a born organism is and carrying no mark of how they
arrived (parents `[0, 0]` and depth 0 are a founder's values). A gut
with no motor and no sensor sits at the speed and sensing floors; that
is the honest state of a unicell here, not a penalty, and leaving it is
ordinary structural mutation under ordinary selection - there is no
second multicellularity mechanic anywhere in the kernel.*

*As built (2026-09-03, Phase 19, ADR-0034): the same gut can eat the
field. One gut, one intake rate: biomass fills the digestive capability
first and the substrate only what biomass leaves, at a yield below one.
Nothing about the body changes - the pass reads the capability every
body already has - so a one-module unicell that starved at ~200 ticks
under coupling v1 is, under v2, an organism with a food source; whether
that carries it to maturity is Phase 19's measurement, not a design
choice.*

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

## Social Perception And Signaling

Phase 13 (`specifications/social-signal-channel.md`, ADR-0029) adds a social
surface alongside the sensing and action channels above. It is config-gated
on `social.enabled`, which requires `artifact.enabled` in turn, and it is
offered only as channel registry version 3.

Perception gathers up to `perception_k` (config, capped at `PERCEPTION_K_MAX
= 4`) nearest conspecifics within `perception_radius_m` - materialized,
sorted by `(distance, id)`, and truncated to `perception_k` before any cue
is read (Rule 5). Each slot carries nine cues, registry channels 23..58,
slot-major: present, distance, bearing, motion (the neighbour's committed
speed fraction - never its turn intent, since a cue may only read what
Rule 4 lets it read), contact (its committed prior-tick contact flag),
object delta (the committed magnitude of object-state change it caused),
carried load fraction (of its own carry capacity - the spec's "material
class" narrowed to a cue rather than a label, ADR-0029's recorded
deviation), body scale, and health. `signal_in[0..4]` (channels 59..62)
read the committed signal field at the receiver's own cell, one value per
channel, always one tick behind whatever was emitted (Rule 4).

Emission binds `signal_emit[0..4]` (channels 118..121); each active
channel costs `signal_cost_milli * amplitude` in energy, billed
remainder-exact in Q16 fractional milli so nothing is rounded away tick
over tick. No channel carries authored meaning anywhere in the kernel:
there is no imitate action and no signal vocabulary, only numbered
channels whatever an organism's own genome learns to make of them.

Plasticity gains a fifth rule form, Observational (`RULE_OBSERVATIONAL =
5`, within a rule id space of `RULE_SPACE = 6`): rule 1's arithmetic with
its presynaptic term replaced by the perceived motion cue of the nearest
conspecific. It sits in the live rule space only when the world offers it
(`social.observational_enabled`); the rule-registry constant itself stays
at 1, for the same reason the channel registry constant stays at 1 while a
social world's *offered* channel version reaches 3 - it is the offered
plasticity version (2) that enters the config hash, not the constant.

## Emergent Diet And Predation

Diet affinity is a continuous trait. Organisms can receive energy from plant biomass and, later, carcasses/attacks based on trait compatibility and action outcomes. No organism receives a permanent predator/herbivore class. Reports may classify ecological role from observed intake ratios over a documented time window, never from a static label.

## Aging And Death

Age is tick-derived. Baseline mortality, age-related decline, starvation, combat, environmental stress, and disasters are separate causes. Death is one-way. Carcasses are finite resource entities with decay and consumption controls; they must not duplicate energy.

## Testing

Test lifecycle transitions, death idempotency, no-action-after-death, bounds, parent-child IDs, sensor range limits, invalid controller output rejection, and energy conservation through food/carcass transfers.
