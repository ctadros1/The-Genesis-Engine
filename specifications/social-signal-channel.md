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

*As built (2026-09-01, ADR-0029): the sort key is `(distance_squared,
organism_id)`. `object_id` in this sentence predates Phase 12, which gave
objects an id space of their own; the value sorted on is the conspecific's
organism id (`world/social_tick.rs`, `conspecifics_within`). Perception is
also a config sub-gate: with `social.perception_enabled` false the channels
stay offered and bindable and every cue reads zero, so a condition-C arm
differs from a condition-A arm in nothing but its named variable.*

For each of the K slots, the following input channels become available in
the registry:

| Channel | Value |
|---|---|
| `neighbour_present[k]` | 1.0 if slot k is occupied, else 0.0 |
| `neighbour_distance[k]` | Normalized distance |
| `neighbour_bearing[k]` | Relative heading, normalized |
| `neighbour_motion[k]` | Speed and turn magnitude, normalized. Body motion, not an action name. *As built: the committed speed fraction alone - turn intent is a per-tick controller output, not committed state, and a cue may read only what Rule 4 allows* |
| `neighbour_contact[k]` | Whether the neighbour is in contact with an object, a cell resource, or another organism. Contact, not intent |
| `neighbour_object_delta[k]` | Whether an object near the neighbour changed state last tick, and by how much. The *outcome*, not the act. *As built: the change the neighbour itself **caused**, not what happened near it - a spatial index of last tick's deltas would price every organism for events nobody perceives* |
| `neighbour_carried[k]` | Material class of any object the neighbour is carrying. *As built: carried load as a fraction of capacity - a material id is a label, refused on ADR-0022 A3's cues-not-labels rule* |
| `neighbour_visible_trait[k][j]` | A small fixed set of externally visible phenotype values (pigmentation, body scale, health fraction, size). *As built: two channels, `neighbour_scale[k]` and `neighbour_health[k]` - pigmentation does not exist in this engine and size is body scale under another name; nothing was invented to fill the list* |

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

*As built (2026-09-01, ADR-0029): there is no `neighbour_action` channel at
all (ADR-0022 A4 removes action labels); read the example as
`neighbour_motion[2]`, channel 44. **And the claim holds only for the
controller gather, not for the sense scan.** An unbound channel is never
requested during evaluation, but with `perception_enabled` the K-nearest
scan and the cue fill run for every organism whatever its bindings, so a
world where nothing binds a social channel still pays the scan. That is
measured rather than assumed - the Phase 13 benchmark's quiet arm exists for
this - and the census-gated skip that would make the original sentence true
end to end is a recorded backlog item, not yet built.*

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

*As built (2026-09-01, ADR-0029): range is `signal_base_range_m *
amplitude`, with no separate cap and no `phenotype_signal_gain` factor - the
heritable gain is the emitting `IoBinding`'s own `gain`, which scales the
requested amplitude before it is clamped. Cost is `signal_cost_milli *
amplitude` summed over channels with no `range_factor` term: range already
scales with amplitude, so a range factor would price the same knob twice.
The charge is billed whether or not any receiver exists, and is exact to the
bit - the sub-milli part is carried per organism in Q16 fractional milli,
because a per-channel cost truncated per tick lands on zero (D-094). The
whole signal half is a config sub-gate: with `social.signal_enabled` false
the channels stay offered, emission requests deposit nothing and are charged
nothing, which is condition B.*

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

*As built (2026-09-01, ADR-0029): attenuation is `amplitude * (r^2 - d^2) /
r^2` for `r = signal_base_range_m * amplitude` - the same monotone
`1 - (d/range)^2`, written so fixed point needs no square root. Emitters sum
into an `i64` staging field in ascending emitter organism id and the clamp
happens once, at the commit; the commit at `Finalize` is decay-then-add,
`committed * signal_retain_q16 >> 16` plus the staged value, so what the next
tick's sense reads is the field committed at the end of the previous one.
Reception applies the receiving `IoBinding`'s `gain` rather than a
`phenotype_signal_sensitivity` trait - the trait table is frozen at 14, and
the evolvable per-channel scalar already exists - so production and response
are separate loci by construction. The corruption draw is keyed
`(receiver_id, channel, tick)` on the `Signal` stream and is taken for every
organism and channel whenever `signal_corruption_q16 > 0`, so the draw
pattern cannot depend on what evolved; zero corruption takes no draw at all.*

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

*As built (2026-09-01, ADR-0029): the first of the two capacities is
`neighbour_motion[slot 0]`, not `neighbour_action[k]` - no action-label
channel exists - and rule 5 is rule 1's arithmetic with the presynaptic term
replaced by that cue, an ordinary declared input rather than a privileged
imitation pathway. Slot 0 is fixed; the preference gene is a follow-up if a
campaign motivates one, so the `Perception` stream (12) is allocated and
takes no draw under the shipped policy, and the candidate order is settled
by `(distance_squared, organism_id)` before any slot is read. The stream is
reserved anyway so that a preference gene or a salience lottery later cannot
renumber the streams already in use.*

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

*As built (2026-09-01, ADR-0029): both are the one config gate
`social.observational_enabled`, and the arm is verified by counter rather
than by flag - the S arm asserts `rule5_updates_total == 0`, so the ablation
is checked by the mechanism's own record. `RULE_OBSERVATIONAL = 5` sits in a
`RULE_SPACE` of 6 while `RULE_COUNT` stays 5; where the gate is off an allele
naming 5 reduces into the base space before ADR-0027's remap, so no
pre-Phase-13 genotype-to-phenotype map moves. P and S therefore do not share
that map either, and genomes may not be transplanted between the arms.*

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
| Multiple emitters per cell | Summed in ascending emitter object-ID order, then clamped. *As built: ascending emitter **organism** id, into `i64` staging; the clamp is applied once at the commit, not per contribution* |
| Candidate selection | Sorted by `(distance_squared, object_id)`, truncated, then drawn. *As built: `(distance_squared, organism_id)`, truncated, and not drawn - slot 0 is fixed* |
| Pairwise draws | `lifesim-pairkey-v1` canonical pair key. *As built: this channel takes no pairwise draw. Its two draws are single-sided on the `Signal` stream (11) - corruption keyed `(receiver_id, channel, tick)` at draw index = channel, and the condition-D receiver keyed `(emitter_id, tick)` at draw index 16, disjoint because one organism can be both emitter and receiver in a tick* |
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

*As built (2026-09-01, ADR-0029): all three are exported, and **only when
the section is enabled** - a disabled world renders no social series at all
rather than a wall of zeros (D-014's inert rule applied to observability).
The first two read the checksummed `SocialCounters`, so a metric and a
fixture trace cannot disagree about the same quantity.
`lifesim_perceived_neighbours` is the count of filled neighbour slots summed
over living organisms at the last sense pass - taken from the `present` cue
the controllers actually saw rather than recounted, and a sum rather than
the mean this line names. A fourth series,
`lifesim_perception_faults_total`, is exported beside them.*

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

*As built (2026-09-01, ADR-0029): these live in
`crates/sim-core/tests/phase13_social.rs`, except the storage-permutation
requirement, which is carried by the standing per-organism permutation
harness in `crates/sim-core/tests/phase9_determinism.rs` now that it
reorders the three social per-organism arrays; the save round trip is
pinned in both places, in-world there and byte-wise in
`crates/sim-persist/tests/format8.rs`. The disabled-section clause
is checked in-crate against the two fixture configs reachable there;
ADR-0029's evidence list additionally requires all five fixtures to
reproduce under the verify scripts on both hosts, which is a phase-exit
item and not this file's claim to make.
**Not yet built:** the long-horizon check - the bill's exactness is pinned
over 400 ticks here, and the 10^6-tick ledger soak C13.11 asks for has no
test yet.*

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
- **The condition arms are config sub-gates**, not analysis after the fact:
  `perception_enabled` off is condition C, `signal_enabled` off is B,
  `scramble_delivery` is D, `observational_enabled` is P against S. Every
  off-state keeps the registry width - the channels stay offered and
  bindable - so the arms share a mutation spectrum and differ only in the
  named variable, which is the D-118 lesson applied in advance. Validation
  refuses `social.enabled` without `artifact.enabled`, a sub-gate moved
  with the section off, `observational_enabled` without plasticity, and
  `scramble_delivery` without `signal_enabled`.
- **Events**: schema 7, `SignalEmitted` (tag 24: emitter, channel mask,
  peak amplitude, cost) and `PerceptionFault` (tag 25). Reception is
  summarized in metrics, never evented per organism per tick.
- **Persistence**: the social table - the committed field, the two
  one-tick cue records, the emission remainders and the counters - is
  snapshot section `SECTION_SOCIAL = 16` at **ALIF format 8**, guarded on
  `FORMAT_VERSION_8` by name. The format-7 writer is retained and the pre-8
  writers refuse a state with `social.enabled` by named field.
  `SAVE_STATE_VERSION` stays 2: fields were added, no meaning changed. The
  checksum section `lifesim-social-state-v1` is appended after
  `lifesim-object-state-v1`, last, so no pinned fixture moves.
- **RNG streams**: `Signal = 11` carries both draws; `Perception = 12` is
  allocated and unused under the shipped policy, reserved so that a
  preference gene or a salience lottery later cannot renumber a stream
  already in use.
- **Rule 5 (Observational)** is built: `RULE_OBSERVATIONAL = 5` in a
  `RULE_SPACE` of 6, rule 1's generalized-Hebbian arithmetic with the
  presynaptic term replaced by the observer's own perceived
  `neighbour_motion[slot 0]` cue - no trace, no modulator, same clamps,
  same eta, same pricing as every other rule. It is offered only when
  `social.observational_enabled`. `RULE_COUNT` stays 5 and the hashed
  constant `RULE_REGISTRY_VERSION` stays 1; a world's *offered* version is
  `RULE_REGISTRY_VERSION_OBSERVATIONAL` (2) only when the gate is on, which
  is what keeps the Phase 11 fixture where it is for a rule no Phase 11
  world could reach. The S arm asserts `rule5_updates_total == 0` by
  counter rather than by flag.

The mechanism is built; no Phase 13 criterion is measured yet, and this
section records only what the kernel does. Still unbuilt and named here
rather than deleted: the census-gated skip that would keep the sense scan
off organisms with no social binding, the 10^6-tick ledger soak C13.11 asks
for, a heritable badge locus, and a genome-encoded slot-preference gene for
rule 5. Each is a follow-up wanting its own decision, not an oversight.
