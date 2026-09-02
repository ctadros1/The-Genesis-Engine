# Phase 13: Social Channel

Status: mechanism BUILT (2026-09-01, ADR-0029, D-122); behavioral criteria
MEASURED (2026-09-02, D-126, campaign `0x5a0ee0ccb8d48df7`, findings at
`experiments/results/phase13-social-confirmatory-findings.txt`). **C13.1,
C13.3, C13.7, C13.8 and C13.9 are UNMET as the pre-registered expected
nulls, with controls - and the censuses make C13.1 a transmission null,
not a reachability null** (every decision-arm world carries hearers,
speakers and a naive cohort at the horizon). C13.4 landed on its third outcome
(both A and S fail; scaffolding controlled). C13.11 PASSED. The campaign's
three unplanned findings: speaker depletion under emission billing (the
named lever for the next iteration), the fidelity exposure delta present
in full in no-channel arms (subtract a no-channel baseline in any reuse),
and the tradition detector's control arm firing 4/12 against a bar of
zero (the era-and-tradition control needs sharpening). The ladder
(`planning/backlog.md`) recorded the null expectation in advance.
Fixture: `scripts/verify-phase13-determinism.sh` (config
`0x252199db7099e9a5`, state `0x5861f0fc8ab02957`). Policy version
`lifesim-social-v1`. Specification: `specifications/social-signal-channel.md`
(as-built notes at its end); every deviation is recorded in ADR-0029.

## Problem

There is no transmission path between organisms: no signalling channel, no
perception of another organism's actions, no imitation. Cumulative culture
requires a transmission path with high enough fidelity that improvements
accumulate rather than decay. This phase builds the path and measures
whether anything travels along it.

This is the phase the whole plan turns on, and it is the most likely to
return a null result.

## Scope

- Perception of the K nearest organisms through **cues, not labels**:
  presence, distance, bearing, body motion, contact events, object-state
  change, carried-object class, and visible phenotype. No action-class
  channel and no genotype-distance channel (ADR-0022 A3, A4). All read from
  the previous tick's committed state.
- A bounded continuous signal channel set, emitted at an energy cost, with
  heritable amplitude and sensitivity, attenuating over distance and
  decaying per tick.
- Plasticity rule form 5 (Observational), available under one experimental
  condition and withheld under another.
- Signal corruption as a configurable fidelity parameter, because fidelity
  is the variable the accumulation question turns on.

## Non-Goals

- **No authored signal semantics.** No kernel code reads a signal channel
  and does anything specific with it. Signals have no meaning in the
  simulation. Meaning, if it arises, is a correlation between emission and
  world state that receivers came to exploit.
- **No `imitate` action.** Imitation is not an action an organism can
  request.
- **No language.** A four-channel local signal field is not a language and
  no document, report, or metric will call it one.
- No enforced signal honesty. Deception is possible by construction and that
  is the correct authored physics.
- No claim that observed transmission is analogous to human culture.

## Prerequisites

- Phase 11 (plasticity: without within-lifetime change there is nothing to
  transmit).
- Phase 7 (contest: threat is the information most likely to be worth
  signalling, and it is the reason contest was moved earlier in the order).
- Phase 9 (channel registry).

## Determinism Notes

- New streams: `Signal` (11), `Perception` (12).
- Emission accumulates during `apply` and commits at `finalize`; perception
  reads the committed prior-tick field. Two-phase commit makes emission and
  perception order irrelevant.
- Multiple emitters into one cell sum in ascending emitter object-ID order,
  then clamp.
- Candidate neighbours are sorted by `(distance_squared, object_id)` and
  truncated before any selection; bucket scan order never reaches a
  decision (Rule 5).
- If A learns from B and B from A in the same tick, both read frozen prior
  state, so the exchange is symmetric and order-free (Rule 4).
- Checksum section `lifesim-signal-state-v1`, present only when enabled.

## The Experimental Design

This is the most carefully controlled phase in the plan, because the naive
comparison (signalling on versus off) confounds three different things: the
existence of the behavior, its energy cost, and the information it carries.

Conditions, matched on seeds (30), config, and run length:

| Condition | Perception of actions | Signal channel | Signal delivery | Observational rule |
|---|---|---|---|---|
| **A** | on | on | to neighbours in range | available |
| **B** | on | off | - | available |
| **C** | off | off | - | available |
| **D** | on | on | **to a randomly chosen unrelated receiver elsewhere in the world** | available |
| **S** | on | on | to neighbours in range | **removed from the registry** |

**Condition D is the load-bearing control.** It preserves the emission
behavior, the energy cost, and the receipt of signals, and destroys only the
spatial and causal link between emitter state and receiver situation. If A
outperforms C but not D, then what was measured was the cost or the
stimulation, not the information. Almost every claim of "communication
evolved" in a simulation fails to run this control, and running it is the
difference between a measurement and an anecdote.

**Condition S** is the philosophical control described in
`specifications/social-signal-channel.md`: it removes the observational
plasticity rule and requires imitation to be discovered from generic
plasticity plus perception alone. Running both S and A means the project
does not have to resolve by assertion how much scaffolding observational
learning needs.

Condition D's random receiver is drawn deterministically from the `Signal`
stream keyed on the emitter and tick, and the resulting delivery set is
sorted by receiver object ID before application, so D is as deterministic as
every other condition.

## Acceptance Criteria

**Primary endpoint: C13.1**, and specifically the A-versus-D comparison.
Secondary criteria do not rescue a failed primary (ADR-0022 A7). The world
is the replicate; per-individual quantities are aggregated to a world-level
statistic before analysis (ADR-0022 A5). Seed counts are floors set by
pilot-driven power analysis, and 50 rather than 30 applies to C13.9 and
C13.10 because the outcomes are rare and heavy-tailed.


- [ ] **C13.1 Transmission occurs.** Naive individuals (born after tick t and
      never personally present at resource patch P) reach P measurably
      faster under A than under C, in at least 20 of 30 seeds, with the
      stated effect size. The same difference must hold for **A versus D**.
      An A-versus-C difference without an A-versus-D difference is not
      transmission and is reported as a negative result.
      **UNMET, measured, 2026-09-02: A-vs-C 0 of 30, A-vs-D 0 of 30**
      (bar 20 of 30 on both; campaign `0x5a0ee0ccb8d48df7`, D-126). The
      pre-registered expected null, and a transmission null: hearers,
      speakers and naive cohorts present in every world's own census.
- [ ] **C13.2 Fidelity clears the accumulation threshold.** Measure
      transmission fidelity F as the correlation between a demonstrator's
      action policy and an observer's post-exposure policy, controlled for
      their genetic similarity. Report F against the threshold at which a
      variant's expected persistence across transmission events exceeds one,
      which is the condition under which improvements accumulate rather than
      decay. Report F across the corruption sweep, so the result is a curve
      rather than a single number.
      **Measured, 2026-09-02: the curve is FLAT (+15/+13/+16/+14 milli
      at corruption 0/8192/16384/32768) and the no-channel arms sit on
      it (B +19, C +17)** - the exposure delta is not transmission, and
      transmission-attributable F is indistinguishable from zero at
      every corruption level (D-126).
- [ ] **C13.3 Traditions outlive individuals.** A behavioral variant present
      in a local neighbourhood at tick t is present at tick t + L, where L
      exceeds three times that run's median lifespan and no individual is
      present at both endpoints, in at least 15 of 30 seeds under A and 0 of
      12 under C. **The finding is invalid without the genotype-matched
      control**: the variant's frequency in the neighbourhood must exceed
      its frequency in a cohort of organisms elsewhere matched on genetic
      distance to the neighbourhood's genotype distribution. Without that
      control, an inherited trait is indistinguishable from a tradition. See
      `specifications/era-and-tradition-detection.md`.
      **UNMET, measured, 2026-09-02: A 8 of 30 (bar 15) - and C fired
      4 of 12 against a bar of ZERO**, so the detector's positives are
      false positives by construction and the spec's control needs
      sharpening before any verdict here means anything (D-126).
- [ ] **C13.4 Scaffolding requirement measured.** Report C13.1 and C13.3
      separately for conditions A and S. Three outcomes, all publishable:
      S succeeds (the observational rule was unnecessary and should be
      dropped); S fails and A succeeds (a specific, quantified statement
      about how much scaffolding observational learning needs in this
      world); both fail (a clean null with the scaffolding question already
      controlled for).
      **Measured, 2026-09-02: the third outcome - both fail** (A-vs-S
      arrival 0 of 30, mean -1 milli, CI [-2,0]; traditions S 5/30 vs
      A 8/30; D-126). The S ablation itself verified at scale: zero
      expressed rule-5 organisms in all 30 S worlds.
- [ ] **C13.5 Payoff-sensitive variant competition.** When two arbitrary
      behavioral variants with different payoffs are both present, the
      higher-payoff variant increases in relative frequency faster under A
      than under C, in at least 20 of 30 worlds. This is the rung between
      "something spreads" and "something useful spreads", and the plan
      previously skipped it (ADR-0022 A12).
      **Reported 2026-09-02: undefined by its own wording** - no two
      identified variants exist (C13.3's positives do not survive their
      control arm; D-126).
- [ ] **C13.6 Retention of one socially acquired improvement.** At least one
      performance improvement is acquired socially by an individual that did
      not discover it, and is retained through that individual's remaining
      lifetime. Measured per individual, aggregated to a world-level rate,
      with the world as the replicate. This is the minimum unit of
      accumulation and must pass before any cumulative claim in Phase 12 or
      16 is interpretable.
      **Measured 2026-09-02: rate 0 in 240 of 240 worlds** - no
      transmission was detected upstream to retain (D-126).

### Organized conflict, relocated from Phase 7

These moved here because they need recognition, memory, and transmission,
none of which exist at Phase 7 (ADR-0022 A1). They remain the plan's most
demanding behavioral criteria.

- [ ] **C13.7 Recognition from cues.** Organisms discriminate between
      conspecifics in a way that correlates with phenotype similarity, with
      no genotype-distance channel available. Under a cue-scrambled control
      the discrimination disappears. Absent this, nothing below is
      interpretable as group-structured.
      **UNMET, measured, 2026-09-02: A 2 of 30 clear the
      scale-permutation null (bar 20), C 1 of 30 (the control holds)** -
      the addendum's expected null on a median 112k usable attacks per
      world (campaign `0x33099bb271e99cde`, schema 8; the C-as-ablation
      deviation is recorded in the addendum; D-126).
- [ ] **C13.8 Persistent interaction communities.** Repeated-association
      networks built from the event log show communities that persist beyond
      the stated window, and are not explained by spatial proximity alone
      (tested against a proximity-matched null).
      **UNMET, measured, 2026-09-02: A 10 of 30 above the null p95 (bar
      20), C 10 of 30 - exactly arm-symmetric, chains median 0** (D-126;
      addendum rules, commit f66b0e2).
- [ ] **C13.9 Directed rather than indiscriminate aggression.** The rate of
      damage between communities exceeds the rate within them by at least
      factor f, in at least 20 of 30 worlds, with communities defined
      offline from the interaction network and never entering the
      simulation. Under the cue-scrambled control the rates do not differ.
      **UNMET, measured, 2026-09-02: 6 of 30 (bar 20), p = 1.000, mean
      -234 milli** - the addendum's expected null; the co-present
      between/within factor is boundary structure present in BOTH arms,
      C's descriptively higher (D-126).
- [ ] **C13.10 Coalition and asymmetry, or a measured null.** Report whether
      aggression rate depends on local numerical advantage, and whether
      multiple aggressors act on one target more often than chance. **Both
      are expected to return null** and are stated so in advance; a null
      here is a measurement about what this physics supports.
      **Measured 2026-09-02: the expected null, arms agreeing to within
      rounding** (advantage ratios 1.85:1 A vs 1.81:1 C; coalition rates
      48 vs 47 milli - spatial, not social; D-126).

- [x] **C13.11 Energy accounting.** The ledger stays exact to the milli-unit
      with signalling costs flowing through it over a 10^6-tick run.
      **PASSED, 2026-09-01/02**: seed 13201, 10^6 ticks, continuous
      check interval, exit 0, ledger exact throughout; archived at
      `runs/phase13-c1311-ledger-soak-0x2323c6b96b9d0b35`.
- [x] **C13.12 Determinism.** Storage-permutation equality over N ticks;
      committed signal field identical under permutation; clean-process
      fixture replay; social-disabled configs reproduce the Phase 11 fixture
      exactly.
      **PASSED, carried by the committed suite** (fixture replay, storage
      permutation over the social arrays, verify scripts green on both
      hosts; D-122/D-123/D-126).
- [x] **C13.13 Perception is causally clean.** A test that mutates an
      organism's state mid-phase must not be observable by any other
      organism in the same tick.
      **PASSED, carried by the committed suite** (the mid-phase mutation
      test; D-122/D-123).

## Test Plan

- Determinism: field commit ordering under permutation; attenuation exact in
  fixed point with no drift over a long run; clean-process fixture.
- Unit: attenuation function at boundaries; amplitude and range clamping;
  cost formula; candidate sorting and truncation.
- Integration: mutual simultaneous learning; a full-perception-radius crowd
  producing bounded work; signal field decay to exactly zero.
- Behavioral: the naive-individual probe as a scripted deterministic
  scenario across all five conditions.
- Fault handling: non-finite neutralization on every perception and emission
  path, counted and evented, no panic.
- Save round trip with a nonzero committed signal field.

## Benchmark Impact

Perception extends `sense` with K-nearest gathering and sorting; signalling
extends `apply` and `finalize` with field accumulation and decay. Record
both per-phase deltas at both tiers, and record how sense cost scales with
`perception_k` and with local density, because crowding makes the candidate
sets large exactly when the tick is already busiest.

Note the cost-saving property worth verifying: unbound channels are not
gathered, so an organism whose genome has no binding to
`neighbour_action[2]` pays nothing for it. Measure whether that holds in
practice or whether the branch cost dominates.

Benchmark schema 6.

## Documentation Updates

`docs/04-simulation-model.md`, `docs/06-organism-model.md`,
`docs/02-scope-and-non-goals.md` (communication moves from deferred to
this phase), `docs/21-open-questions.md`,
`specifications/simulation-tick.md`, `specifications/event-schema.md`,
`specifications/metrics-schema.md`, decision log, risk register.

## Risks

| Risk | Mitigation |
|---|---|
| **Null result: no transmission in any seed under any condition.** The most likely single outcome of this phase | The design makes the null informative. Conditions B, C, D, and S plus the fidelity curve mean a null answers *why*, not just *whether*. This is the intended value of the phase even if C13.1 fails |
| Signals evolve into noise because they are costly and uninformative | Expected under many parameter settings; the corruption sweep and the contest-derived threat information are the conditions under which it is least likely |
| A-versus-C difference is claimed as transmission without running D | D is an acceptance criterion, not an optional extra. C13.1 explicitly fails without it |
| A tradition claim is made without the genotype control | C13.3 is invalid without it, and the report format in `specifications/era-and-tradition-detection.md` requires the control statistic in every finding |
| Condition S is unfalsifiable in practice: the search never finds it regardless of run length | Real and unresolved. The seed count and run length that would make an S null meaningful may exceed the compute budget. If so, the phase reports S as underpowered rather than as negative. That distinction must survive into every downstream summary |
| Perception cost dominates the tick at high density | Bounded K, sorted truncation, and a measured density sweep |

## Rollback

One config section. Disabled, no perception channels are gathered, no signal
field exists, rule form 5 is absent from the registry, and the Phase 11
fixture reproduces exactly.
