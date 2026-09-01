# Social Signal Channel Specification

Status: implementing, Phase 13 (2026-09-01). Policy version
`lifesim-social-v1`. Realized by ADR-0029, which records every place the
implementation departs from this document's text beside its reason; the
as-built notes at the end of this file summarize them. Depends on genome
schema 2 (channel registry), the artifact section (the registry version
scheme is a total order, ADR-0029 section 1), and plasticity
(`specifications/plasticity-and-learning.md`).

## Problem

There is no transmission path between organisms. No organism can perceive
what another just did, no signal can be emitted or received, and imitation
is impossible. Cumulative culture requires a transmission path whose
fidelity is high enough that improvements accumulate rather than decay.

## What Is Authored And What Is Evolved

Authored:

- That an organism's recent action is visible to nearby organisms.
- That a bounded vector of continuous signal channels can be emitted at a
  cost and perceived within a radius.
- The physics of attenuation, cost, and range.

Never authored:

- What any signal means. Signals have no semantics in the kernel. No code
  reads a signal channel and does anything specific with it. Meaning, if it
  arises, is a correlation between emission and world state that receivers
  have come to exploit.
- Whether signals are honest. Nothing forces emission to correlate with the
  emitter's state, so deception is possible by construction. That is the
  correct authored physics, not an oversight.
- Whether imitation happens.

## Perception Of Others' Actions

Extends the sense phase. For each observer, the candidate set is every
organism within `perception_radius_m`, materialized, sorted by
`(distance_squared, object_id)`, and truncated to `perception_k` (config,
small, default 4). See `specifications/determinism-extensions.md` Rule 5.

For each of the K slots, the following input channels become available in
the registry:

| Channel | Value |
|---|---|
| `neighbour_present[k]` | 1.0 if slot k is occupied, else 0.0 |
| `neighbour_distance[k]` | Normalized distance |
| `neighbour_bearing[k]` | Relative heading, normalized |
| `neighbour_motion[k]` | Speed and turn magnitude, normalized. Body motion, not an action name |
| `neighbour_contact[k]` | Whether the neighbour is in contact with an object, a cell resource, or another organism. Contact, not intent |
| `neighbour_object_delta[k]` | Whether an object near the neighbour changed state last tick, and by how much. The *outcome*, not the act |
| `neighbour_carried[k]` | Material class of any object the neighbour is carrying |
| `neighbour_visible_trait[k][j]` | A small fixed set of externally visible phenotype values (pigmentation, body scale, health fraction, size) |

Every value is read from the **previous tick's committed state**
(`determinism-extensions.md` Rule 4). An organism never perceives the
current tick.

**There is no kinship channel and no action-label channel.** ADR-0022 A3
and A4 remove both:

- A normalized genetic distance handed to an organism short-circuits
  recognition entirely. Real recognition is solved from perceptible cues,
  and if it cannot be solved from cues here then it is not solved.
- A one-hot action class is a privileged action identifier, which scripts
  the channel: it supplies the very categorization that an observer would
  otherwise have to infer from motion and outcome.

What remains is deliberately harder to use and is the only thing that makes
a positive result mean anything. Phenotype cues are heritable, so kin
*assortment* remains reachable through phenotype matching without any
genotype access.

Unbound channels cost nothing: an organism whose genome contains no
`IoBinding` to `neighbour_action[2]` never has it gathered. Sense-phase cost
therefore scales with what evolved, not with the registry size. This is a
material performance property, because the naive alternative is gathering
`perception_k` times a dozen channels for every organism every tick.

## Signal Emission And Field

Signals are `signal_channels` continuous values (config, default 4), emitted
as action channels from the registry.

Emission:

- Amplitude per channel is the clamped action-channel value in [0, 1].
- Range is `signal_range_m = base_range * amplitude * phenotype_signal_gain`,
  capped by config. `phenotype_signal_gain` is an expressed trait, so signal
  reach is heritable and under selection.
- Cost is `signal_cost_milli * sum_of_amplitudes * range_factor`, deducted
  through the existing energy ledger as an action cost. Signalling is never
  free; a free signal channel evolves into noise.

Propagation:

- Emitted signals are written to a per-cell signal accumulation field during
  `apply`, committed at `finalize`, and read by the next tick's sense phase.
  Two-phase commit again makes emission order irrelevant.
- Attenuation is a configured monotone function of distance, evaluated in
  fixed point over the raster cells within range. Contributions from
  multiple emitters into the same cell are summed in ascending emitter
  object-ID order, then clamped.
- The field decays per tick by a configured factor, so a signal is a
  transient local event and not a permanent world marking. Permanent
  marking is what artifacts are for, and conflating the two would blur the
  Phase 13 and Phase 12 results.

Reception:

- Input channels `signal_in[c]` report the committed field value at the
  receiver's cell for channel `c`, scaled by an expressed
  `phenotype_signal_sensitivity` trait.
- The `Signal` RNG stream supplies optional channel corruption. Default
  corruption is zero; nonzero corruption is the fidelity-sweep experimental
  condition, because transmission fidelity is exactly the variable the
  accumulation question turns on.

## Imitation

There is no `imitate` action. Imitation, if it occurs, is a consequence of
two authored capacities:

1. `neighbour_action[k]` is an ordinary input channel.
2. Plasticity rule 5 (Observational) allows a plastic edge's presynaptic
   term to be the perceived action of the selected neighbour rather than a
   node activation.

The selected neighbour for rule 5 is slot 0 of the sorted candidate set, or
a slot chosen by a genome-encoded preference gene, with ties broken by the
`Perception` stream keyed on the observer's ID. It is never "the first one
found in the bucket".

### The honest judgment call

Rule 5 authors the capacity for a synapse to be driven by an observed
conspecific action. A stricter reading of the project philosophy would omit
it and require imitation to be discovered from generic plasticity plus
perception alone.

We do not resolve this by assertion. Phase 13 runs both as experimental
conditions:

- **Condition P (permissive):** rule 5 available in the registry.
- **Condition S (strict):** rule 5 removed from the registry; only rules 1
  through 4 available.

If S produces transmission, the permissive rule was unnecessary and the
project should record that and drop it. If S produces nothing across the
full seed set and P produces transmission, we have learned something
specific and reportable about how much scaffolding observational learning
needs, which is a more interesting result than either a null or a success
alone. If both produce nothing, the phase has a clean null result with the
scaffolding question already controlled for.

The risk, stated plainly: condition S is the more likely of the two to be
unfalsifiable in practice, because the search may simply never find the
configuration. The seed count and run length that would make an S null
result meaningful are recorded in the phase plan and are not small.

## Determinism Summary

| Concern | Resolution |
|---|---|
| Emission order | Two-phase: accumulate in `apply`, commit at `finalize` |
| Perception order | Reads previous tick's committed field and state only |
| Multiple emitters per cell | Summed in ascending emitter object-ID order, then clamped |
| Candidate selection | Sorted by `(distance_squared, object_id)`, truncated, then drawn |
| Pairwise draws | `lifesim-pairkey-v1` canonical pair key |
| Mutual learning in one tick | Both read frozen prior state; symmetric and order-free |
| Checksum | Section `lifesim-social-state-v1`, present only when enabled (the section carries perception state as well as the field, so the broader name is used; ADR-0029 section 6) |

## Metrics And Events

New bounded events: `SignalEmitted` (emitter, channel mask, amplitude
summary, cost), `PerceptionFault` (non-finite neutralized). Signal reception
is not evented per organism per tick; that would be unbounded. Reception is
summarized in metrics.

New metrics: `lifesim_signals_emitted_total`,
`lifesim_signal_energy_spent_milli_total`,
`lifesim_perceived_neighbours` (gauge, mean occupied slots).

Signal *content* is never a metric label; that would be unbounded
cardinality and would also encourage reading meaning into channels that have
none.

## Test Requirements

- Field commit ordering: permuting stored organism order leaves the
  committed signal field and all checksums identical.
- Attenuation is exact fixed point with no accumulation drift over a long
  run.
- Energy ledger stays exact with signalling costs flowing through it.
- Perception reads only committed prior state: a test that mutates an
  organism mid-phase must not be observable by any other organism that tick.
- Disabled-section equality: social disabled reproduces the Phase 11 fixture
  exactly.
- Non-finite neutralization on every input and output path, counted and
  evented, no panic.
- Save round trip with a nonzero committed signal field.

## As-Built Notes (2026-09-01, ADR-0029)

The kernel realizes this specification with the following recorded
departures; the reasoning for each is in ADR-0029 and none weakens a
stated criterion:

- **Registry version 3**, channels 23..=58 (nine cues per neighbour slot,
  four slots), `signal_in` 59..=62, `signal_emit` 118..=121;
  `perception_k` and `signal_channels` are validated `1..=4` against the
  registry's width. `social.enabled` requires `artifact.enabled`.
- **`neighbour_carried[k]`** is the neighbour's carried-load fraction, not
  a material class (a material id is a label; ADR-0022 A3, ADR-0028).
- **Visible phenotype** is body scale and health fraction; pigmentation
  does not exist in this engine and size duplicates scale. A heritable
  badge locus is a recorded follow-up needing its own ADR.
- **`neighbour_motion[k]`** is the committed speed fraction; the turn
  intent is not committed state.
- **`neighbour_object_delta[k]`** is the committed magnitude of
  object-state change the neighbour caused last tick (extraction,
  consumption, combination), normalized against 1,000 milli.
- **Heritable amplitude and sensitivity are the `IoBinding` gains** on the
  emission and reception channels - the trait table is frozen at 14, and
  the evolvable per-channel scalar already exists. Production and response
  genes are separate loci by construction.
- **Cost** is `signal_cost_milli * amplitude` summed over channels, exact
  to the bit with a Q16-fractional-milli remainder per organism; the
  `range_factor` term is absent because range already scales with
  amplitude and a separate factor would price the same knob twice.
- **Attenuation** is `1 - (d/range)^2` - monotone in distance as required,
  square-root-free in fixed point. The field commit is decay-then-add at
  `Finalize` with `signal_retain_q16` strictly below one whole (a
  non-decaying field is refused: permanent marking is what artifacts are
  for).
- **Corruption** draws on the `Signal` stream keyed
  `(receiver_id, channel, tick)`, taken for every organism and channel
  whenever `signal_corruption_q16 > 0` so the draw pattern cannot depend
  on what evolved; zero corruption takes no draw. The condition-D
  scrambled delivery draws on the same stream keyed `(emitter_id, tick)`
  at draw index 16.
- **Events**: schema 7, `SignalEmitted` (tag 24: emitter, channel mask,
  peak amplitude, cost) and `PerceptionFault` (tag 25). Reception is
  summarized in metrics, never evented per organism per tick.
- **Rule 5 (Observational)** is not yet implemented; ADR-0029 section 5
  fixes its form (rule 1's arithmetic with the presynaptic term replaced
  by the perceived `neighbour_motion[slot 0]` cue, gated by
  `social.observational_enabled`, verified by counter in the S arm).
