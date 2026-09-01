# ADR-0029: Social Channel - Perception Cues, Costly Signals, And Observational Plasticity

Status: Proposed
Date: 2026-09-01
Author: Phase 13 design pass

Realises `specifications/social-signal-channel.md` and
`planning/phase-13-social-channel.md` (policy `lifesim-social-v1`). Records
every place the implementation departs from the specification text, beside
its reason, the way ADR-0028 did for the artifact half. Five commissioned
reviews were consulted before this document was written - cumulative
culture, social organization, neuroevolution, mutable world, experimental
methodology - and the constraints extracted from them are cited inline by
the reviews' own section numbers.

## Context

Phase 12 closed with three measured nulls whose diagnosis is recorded in the
ladder (`planning/backlog.md`): the mechanisms were reachable and firing and
did not pay at that reach, cost and horizon. The ladder's fourth lever is
this phase, with an ordering constraint stated in advance: **imitation has
nothing to copy until some within-lifetime behavior is worth copying, so
Phase 13 ships its mechanism and its reachability census expecting the
transmission criteria to read null** until a learning campaign under levers
1-3 succeeds. D-120 (the reachability 2x2) supplies the one lever already
measured: `plasticity.live_rule_zero` makes the plasticity phenotype
assemblable (C 10 of 12 against its bar), so the Phase 13 campaign should
carry the chain on, and the pre-registration must cite D-120 when it does.

D-121 records why this phase proceeds without Phase 18: campaign worlds are
single-threaded by rule, so intra-world parallelism cannot change any
world that decides a criterion.

## The decisions, each with its reason

### 1. Registry version 3, and social requires artifact

The forty perception-and-signal input channels and four signal outputs are
**registry version 3** (`CHANNELS_V3`), offered only by a world whose
`social` section is enabled, exactly as `CHANNELS_V2` is offered only with
`artifact` (ADR-0028 section 7). `CHANNEL_REGISTRY_VERSION` stays 1 and
`CHANNEL_REGISTRY_VERSION_ARTIFACT` stays 2, for the reasons recorded on
those constants; ALG2 stamps the smallest version covering a genome's
bindings and the reader accepts 1..=3.

**`social.enabled` requires `artifact.enabled`, refused at validation.**
The version scheme is a total order - a genome's stamp is "the smallest
version that covers its bindings", which has no meaning over a lattice - so
a social world offers versions 1, 2 and 3, and a world offering the
artifact channels without the artifact section would accept a genome bound
to `pick_up` in a world with no objects: exactly the gating defect
`a_genome_bound_to_an_object_channel_is_refused_in_a_world_that_does_not_offer_it`
exists to refuse. The coupling is also what the roadmap's question needs:
Phase 13 asks whether a faster channel beats stigmergy (D-039), which
requires the stigmergic substrate present in every arm, common-mode. The
mutable-world review's own control battery (15.9 control 7,
"explicit-communication comparison") is this comparison run from the other
side.

### 2. The perception cue set: what the physics already makes visible

Nine input channels per neighbour slot, four slots (`PERCEPTION_K_MAX =
4`; `perception_k` is config, validated `1..=4`), IDs 23..58, plus
`signal_in[0..4]` at 59..62:

| Per-slot channel | Value | Spec row |
|---|---|---|
| `neighbour_present[k]` | 1.0 if occupied | as specified |
| `neighbour_distance[k]` | 1 - d/radius, clamped | as specified |
| `neighbour_bearing[k]` | signed cross-product bearing, the object-cue form | as specified |
| `neighbour_motion[k]` | the neighbour's committed speed fraction | as specified: body motion, not an action name. Narrowed from "speed and turn magnitude": the turn intent is a per-tick controller output, not committed state, and a cue must read only what Rule 4 lets it read |
| `neighbour_contact[k]` | the neighbour's committed prior-tick contact flag | as specified |
| `neighbour_object_delta[k]` | committed prior-tick object-state-change magnitude the neighbour caused | **deviation, see below** |
| `neighbour_carried[k]` | the neighbour's carried load as a fraction of its capacity | **deviation, see below** |
| `neighbour_scale[k]` | body scale, normalized | the spec's "visible phenotype", narrowed |
| `neighbour_health[k]` | health fraction | the spec's "visible phenotype", narrowed |

Candidates are every organism within `perception_radius_m`, materialized,
sorted by `(distance_squared, organism_id)`, truncated to `perception_k`
(Rule 5). Every value reads the previous tick's committed state; positions
and headings at sense time are exactly that, and the two cues that need
one tick of memory (`contact`, `object_delta`) read per-organism records
committed at the end of the prior tick and carried in the social state
(saved and checksummed - they cannot be recomputed, which is
`learnstate.rs`'s argument).

**Two deviations from the specification's table, both recorded here:**

- **`neighbour_carried[k]` reports carried load, not material class.** The
  spec says "material class of any object the neighbour is carrying"; a
  material id is a label, and ADR-0028 already refused `object_material`
  as a channel on ADR-0022 A3's cues-not-labels rule, delivering heft
  instead. The same rule applied consistently gives the neighbour's
  carried-load fraction - the cue `carried_load` (22) already gives an
  organism about itself. The cumulative-culture review's mechanism table
  (9.2) names "labels such as tool_contact" an accidental scripting risk;
  a material class is that risk wearing a load-bearing name.
- **"Visible phenotype" is body scale and health fraction, and nothing
  else.** The spec's parenthetical names "(pigmentation, body scale,
  health fraction, size)": pigmentation does not exist anywhere in this
  engine, and size is body scale under another name. Nothing is invented
  to fill the list, deliberately. The social-organization review wants a
  heritable continuous cue vector with independently recombining
  production and response genes (4.4, 11.5) - and its own history section
  (10.4: Riolo/Hammond tag models; Roberts & Sherratt's collapse) records
  that a clean authored badge manufactures faction-like dynamics that
  evaporate under exploitation. A dedicated badge locus is therefore a
  **recorded possible follow-up needing its own ADR**, not a default; with
  only body scale heritable among the visible cues, C13.7 (recognition
  from cues) may well return null, and the plan's open question already
  accepts that outcome as a result: "the honest answer may be no. C13.7
  reports either way."

- **`neighbour_object_delta[k]` reads what the neighbour caused, not what
  is near it.** The spec says "whether an object near the neighbour changed
  state last tick"; the implementation commits the magnitude of
  object-state change the neighbour itself caused (extraction volume,
  consumed energy, combined mass, normalized against 1,000 milli). A
  spatial index of last tick's object deltas would price every organism
  for events nobody perceives; the caused-by form is the same outcome-
  shaped cue (never an act label) at per-organism cost, and it is the
  reading the cumulative-culture review's mechanism table actually
  licenses (9.2: "object motion, deformation, damage, transfer" of the
  observed actor).

The K-nearest tie at the truncation boundary resolves by `(distance_squared,
organism_id)`, the project's standing Rule 5 form. The mutable-world review
(13.4) warns that raw-ID tie-breaking is a persistent fitness advantage;
the precedent recorded for objects (D-log open question on the
contested-acquisition lottery) applies here unchanged: if a campaign shows
organism ID predicting perceptual salience, that is the trigger for a keyed
lottery on the `Perception` stream, and not before.

### 3. The signal channel: an uninterpreted, costly, attenuating field

`signal_channels` (config, validated `1..=4`) continuous channels. Emission
is an output binding to `signal_emit[c]` (IDs 118..121; 109-112 stay
unallocated forever per `CHANNELS_V2`'s doc). Amplitude is the clamped
request in [0, 1]. Per tick, an emitting organism pays
`signal_cost_milli * amplitude` (summed over channels, exact to the bit
with a per-organism remainder carried in Q16 fractional milli - D-094's
lesson pre-applied: a per-channel cost truncated per tick lands on zero),
charged through the ledger whether or not any receiver exists
(mutable-world 7.6: failed or unheard actions still cost, or agents probe
the world for free). The spec's `range_factor` term is **deliberately
absent**: range already scales with amplitude, so a separate range factor
would price the same knob twice, and the single proportionality keeps
"louder costs more and reaches further" one evolvable quantity.

Propagation: the emission writes `amplitude * (1 - (d / range)^2)` into
every cell within `range = signal_base_range_m * amplitude` - a monotone
attenuation in distance, as the specification requires, chosen in this form
because it needs no square root in fixed point - into a
**staging field** during `apply`; contributions sum in ascending emitter
organism ID and clamp; the staging field commits into the read field at
`finalize` as `committed = decay(committed) + staged`, with
`signal_decay_q16` per tick. Sense at t+1 reads the committed field at the
receiver's cell scaled by the input binding's gain. A signal emitted at t
is therefore observable at t+1 and never earlier (cumulative-culture
9.2/10.5's simultaneous-update rule; Rule 4).

**No semantics anywhere**: no kernel code reads a signal channel and does
anything specific; channels are numbered, not named; no channel carries a
referent, a coordinate, a target id, or a truth guarantee
(social-organization 6.3; mutable-world 7.3: "explicit target pointer
transmits object identity perfectly" is the failure). Deception is
possible by construction.

**Heritable amplitude and sensitivity are the binding gains.** The spec
says `phenotype_signal_gain` and `phenotype_signal_sensitivity` are
"expressed traits"; the trait table is frozen at 14 (schema 1's layout,
load-bearing for every fixture), and a new locus kind is a genome-schema
change this phase does not need - because the evolvable per-channel scalar
already exists: every `IoBinding` carries a `gain` that point mutation
moves (D-114's operator inserts bindings; the gain target moves them).
Emission gain scales the requested amplitude; reception gain scales the
received value. Production and response are separate loci by construction,
independently recombining, which is exactly the separability the
social-organization review requires (4.4). Recorded caveat from the
cumulative-culture review (6.12): a signal system whose production and
reception are *purely* genetic is coordination, not culture - the
within-lifetime half is exactly what plasticity (rules 1-5) over the
`signal_in` channels supplies, and any transmission claim must show the
learned half doing work (its C11.3/12.10 acquisition tests).

**Corruption** is the fidelity knob (`signal_corruption_q16`, default 0):
at reception, a uniform draw in `+/- corruption` on the `Signal` stream,
keyed on `(receiver_id, channel, tick)` (social-organization 6.9 names
this exact keying; methodology 6.3 requires the named domain so arms stay
paired). Zero corruption takes no draw, so a corruption-free world
consumes nothing from the stream.

### 4. The condition gates are config, and D is first-class

Conditions A/B/C of the plan's table are realized by two sub-gates beside
the section gate: `social.perception_enabled` (off in condition C) and
`social.signal_enabled` (off in B and C). Both off-states keep the registry
width - the channels stay offered and bindable, the cues read zero and the
emissions deposit nothing - so the arms share a mutation spectrum and
differ only in their named variable, which is the D-118 lesson applied in
advance. A sub-gate moved with the section off is refused at validation,
on the probe section's precedent.

### Condition D is a first-class config gate, not an analysis trick

`social.scramble_delivery` (condition D): the emission's attenuation
profile is deposited centered on the position of a **randomly chosen other
living organism** rather than on the emitter - a draw on the `Signal`
stream keyed on `(emitter_id, tick)`, uniform over the tick-start living
population excluding the emitter, resolved against the ID-sorted
population so storage order never decides it. Cost, amplitude, attenuation
and decay are byte-identical to condition A; only the spatial-causal link
between emitter state and receiver situation is destroyed. This is the
social-organization review's own named control ("sham channels with
identical energetic cost but random receiver mapping", 11.9) and the
methodology review's strong test (3.5). The plan is explicit that C13.1
fails without the A-versus-D difference.

### 5. Rule 5 (Observational) is an instance of the existing family, gated

`RULE_COUNT` goes 5 -> 6 and `RULE_REGISTRY_VERSION` 1 -> 2, exactly as
the registry's own doc reserved. Rule 5's form is the registry doc's
sentence made precise under ADR-0022 A4 (no action labels exist, so
"perceived-action input" cannot mean an action id): **rule 1's arithmetic
with the presynaptic term replaced by the observer's own perceived
`neighbour_motion[slot 0]` value** - the committed prior-tick body motion
of the nearest conspecific, a value the organism's sense phase already
computes. It reads only what perception delivers (cumulative-culture 4.7:
pose and motion within normal sensory limits, never a
demonstrator-chose-action-N symbol; neuroevolution 6.10/13.13: observation
arrives through ordinary declared inputs, never a privileged imitation
pathway). No trace, no modulator, same clamps, same eta, same energy
pricing as every other rule; slot 0 fixed rather than a preference gene
(the spec permits either; the gene is a follow-up if a campaign motivates
it). One selected neighbour per edge per tick means there is no
per-neighbour update loop and therefore no traversal-order hazard
(methodology 10.7).

**Availability is a config gate, not a constant**:
`social.observational_enabled` (condition P true, condition S false). The
effective rule-id space follows ADR-0027's machinery - the draw modulus
and the expression reduction both see the effective count, so no rule is
preferred - and rule 5 is offered only when `social.enabled` and the gate
are both true, because in any other world its input is structurally zero
and "a rule that silently equals rule 1 with `b` disabled" is what the
registry doc refuses to admit. Consequences carried knowingly, both with
D-110 precedent: P and S arms do not share a genotype-to-phenotype map
(a stored id expresses differently under different moduli), so genomes
may not be transplanted between arms; and the S arm must be **verified by
counter, not by flag** - a `rule5_updates` counter that the S arm asserts
zero (methodology 6.4: an ablation is checked by events showing the
mechanism was actually inactive).

### 6. Determinism, numbered state, and persistence

- **RNG streams**: `Signal = 11` and `Perception = 12` are allocated - the
  two values the `RngSystem` doc left "free deliberately, as spare
  capacity" for exactly this reservation (D-028 named them). `Signal`
  carries the corruption draw and the condition-D receiver draw on
  disjoint draw indices; `Perception` is allocated and **unused** under
  the shipped policy (slot selection is deterministic slot 0; the stream
  exists so a preference gene or salience lottery later cannot renumber).
- **Checksum**: section `lifesim-social-state-v1` - the committed signal
  field, the per-organism prior-tick contact and object-delta records,
  the emission cost remainders, and the social counters - hashed only
  when enabled, appended **after** `lifesim-object-state-v1` (Rule 8 as
  amended by ADR-0028: append last, so the five fixtures never move).
  The spec names the tag `lifesim-signal-state-v1`; the section carries
  perception state as well as the field, so the broader name is used and
  the spec is corrected in the same change.
- **Persistence**: new config fields force **ALIF format 8** (D-112's
  rule), with the format-7 writer retained (`encode_snapshot_format7`),
  `FORMAT7_TO_CURRENT` registered, one new row in the format chain test
  declaring `FORMAT8_CONFIG_BYTES`, and the pre-8 writers refusing a
  state with `social.enabled` by named field (`FieldNotInFormat`). The
  signal field and per-organism records are a new snapshot section
  `SECTION_SOCIAL = 16`, guarded on `FORMAT_VERSION_8`.
  `SAVE_STATE_VERSION` stays 2 (fields added, no meaning changed).
- **Events**: schema 6 -> 7, tags append - `SignalEmitted = 24` (emitter,
  channel mask, amplitude summary, cost) and `PerceptionFault = 25`
  (non-finite neutralized, counted). Reception is deliberately not
  evented per organism per tick (unbounded); it is summarized in metrics.
  Every decoder range that names the last tag is extended and the
  round-trip sample gains both tags - trap 33 is the standing reason.
- **Fixture schema** 8 -> 9: the fixture JSON gains the social policy
  string, registry version 3, and the social counters, so the Phase 13
  verify script can refuse a vacuous trace mechanism by mechanism.

### 7. What is deliberately absent

No `imitate` action, no teach action, no signal vocabulary, no honesty
enforcement, no kinship channel, no action-class channel, no salience
prior beyond distance (mutable-world 17.7: a salience rule is an authored
attention prior), no per-organism reception events, no analysis label
reaching the kernel (ADR-0016), and no claim that any of this is language.
The observer-side machinery for C13.3/C13.7-C13.10 (communities,
traditions, genotype-matched controls) is sim-analysis work governed by
Gate E discipline: the classifiers must recover known synthetic ground
truth on scripted fixtures before they are run on evolved populations.

## Consequences

Enabling `social` starts a new replay lineage (config block hashed when
enabled; registry version 3 in the hash via the same accessor the artifact
gate uses). Disabled, the section is `None`: no perception is gathered, no
field exists, rule 5 is absent, the phase costs nothing, and every pinned
fixture reproduces - C13.12's disabled-equality clause covers the Phase 11
fixture and the re-pinned Phase 12 fixture (D-119).

Sense-phase cost scales with what evolved, not with the registry: an
organism with no social bindings gathers nothing. The benchmark section
must measure that claim rather than assume it (the plan says so).

## Evidence Required To Accept

- All five fixtures reproduce with the section disabled (tests + verify
  scripts, both hosts).
- The Phase 13 fixture trace pins every mechanism nonzero (emission,
  reception above zero field, corruption draws under a corruption arm,
  rule-5 updates under P, refusals) and its two control conditions replay
  as distinct lineages.
- Ledger exactness over 10^6 ticks with signalling costs flowing (C13.11).
- Perception causal cleanliness (C13.13) as a test that mutates mid-phase
  state and asserts invisibility.
- The S arm asserts `rule5_updates == 0` by counter.
- Mutation testing by an agent that did not write the tests.
