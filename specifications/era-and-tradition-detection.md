# Era And Tradition Detection Specification

Status: design specification, not implemented. Phase 17. Analysis versions
`lifesim-era-v1`, `lifesim-tradition-v1`. These are **not** in the config
hash, because an analysis version can never affect a world.

## The Hard Rule

Analysis observes. It never instructs.

This is the same rule `lifesim-similarity-v1` already obeys: species labels
are computed offline, recorded in reports, and never feed back into mating
rules, action eligibility, or behavior. Era and tradition detection obey it
identically. See ADR-0016.

Enforced structurally, not by convention:

- Detection lives in a separate crate, `sim-analysis`, which depends on
  `sim-core` and on the event-log reader. **Nothing in `sim-core` depends on
  `sim-analysis`.** The dependency direction makes feedback a compile error
  rather than a review finding.
- Detection consumes the append-only event log and read-only snapshot views.
  It has no mutable handle to a world.
- Detection draws from no `RngSystem` stream. If it needs sampling it uses a
  separate deterministic sampler seeded from its own report parameters.
- A required test asserts that world state checksums are bit-identical with
  detection enabled at any cadence and with detection disabled, mirroring
  the existing Phase 2 analysis-neutrality test.

An era is a narrative an observer detects post hoc. It is never a state the
simulation enters, and no organism is ever told about one. There is no
`era` field in world state, no era input channel, and no era-conditioned
rule.

## Prerequisite: The Append-Only Event Log

Detection reads the event log file. That file does not exist yet: it is the
Phase 4 item deferred under D-019, with snapshots currently carrying a zero
event-log reference. It is therefore in **Phase 5's** scope, not Phase 17's,
because multi-seed experiment analysis needs it long before era detection
does.

Detection also requires the extended event schema (version 3) covering
contest, structural mutation, signalling, and object events. See
`specifications/event-schema.md`.

## Era Detection: `lifesim-era-v1`

### Feature extraction

The event log is partitioned into fixed-width windows of `window_ticks`
(config, recorded in the report). Each window yields a fixed-length feature
vector, all values normalized and bounded:

| Feature group | Contents |
|---|---|
| Demography | Birth rate, death rate by cause, population, mean ancestry depth |
| Genetics | Structural mutation rates by operator, mean node and edge count, mean genetic diversity |
| Plasticity | Fraction of edges plastic, mean learned-delta magnitude, plasticity fault rate |
| Social | Signal emission rate, mean signal energy spent, mean perceived neighbours |
| Objects | Pick-up, place, strike, and combine rates; composite-depth distribution; mean artifact lifetime |
| Conflict | Damage event rate, within-cluster versus between-cluster damage ratio |

### Segmentation

Bounded exact dynamic-programming change-point segmentation over the window
sequence, minimizing within-segment feature variance plus a fixed
per-segment penalty. Both the penalty and the maximum segment count are
config, recorded in the report. The algorithm is exact and deterministic:
same event log gives the same segmentation, byte for byte.

An approximate or randomized segmenter is not acceptable here, not because
approximation is wrong in general but because a nondeterministic analysis
would make the reports irreproducible, which is the one thing this project
does not tolerate.

### What a segment is and is not

A segment is a statistically detected regime change in event rates. The
report calls it a segment. It does not call it an era, an age, a
revolution, or a stage, and it never names it after a human historical
period. Interpretation belongs to whoever reads the report, with the feature
deltas in front of them.

## Tradition Detection: `lifesim-tradition-v1`

The claim "a behavioral tradition exists" is much stronger than "a behavior
is common", and the difference is entirely about controlling for genetics. A
behavior shared by a local group whose members are close kin is a plausible
inherited trait, not a tradition.

A tradition claim requires all four:

1. **A behavioral variant.** A distinguishable action-distribution cluster,
   computed from event-derived per-organism action histograms over a bounded
   window, clustered by the same deterministic threshold and union-find
   method `lifesim-similarity-v1` already uses.
2. **Local concentration.** The variant's frequency inside a spatial or
   social neighbourhood exceeds its global frequency by a stated factor.
3. **Persistence beyond individuals.** The variant is present in the
   neighbourhood at tick `t` and at tick `t + L`, where `L` exceeds three
   times the median lifespan measured in that run, with no individual
   present at both endpoints.
4. **Not explained by genotype.** The decisive control. Compare the variant's
   frequency in the neighbourhood against a **genotype-matched cohort**:
   organisms elsewhere in the world whose genetic distance to the
   neighbourhood's genotype distribution is within a stated tolerance. If
   the matched cohort shows the same variant frequency, the variant is
   inherited, not transmitted, and no tradition is reported.

Criterion 4 is not optional and a report that omits it is invalid. Every
tradition finding carries its genotype-matched control statistic, its
matching tolerance, and the cohort size, so a reader can judge whether the
control had power.

## Report Contents

Every report records, following the existing similarity-report convention:

- Analysis algorithm version, analysis tick range, config hash, seed, world
  ID, save-state and event-schema versions, simulation build version.
- Window width, segmentation penalty, maximum segment count, clustering
  threshold, sampling policy and bounds.
- Segment boundaries with the feature deltas that produced each.
- Tradition findings with their genotype-matched controls.
- Explicit negative results. A run with no segments above threshold and no
  traditions reports that fact. Silence is not a result.

Reports are the analysis output format defined in
`specifications/metrics-schema.md` conventions and are versioned
independently of the world.

## Validation: The Part That Makes This Falsifiable

Without validation, "era detection" is unfalsifiable pattern-matching over
noise. Three required checks:

**Synthetic ground truth.** On a synthetic event log with injected known
regime changes at known ticks, the detector recovers boundaries within
`+/- k` windows with stated precision and recall. The synthetic generator
and its injected changes are fixtures.

**Null control.** On event logs from runs with the relevant mechanisms
ablated (no artifacts, no signalling, no contest), the detector reports no
segments above threshold in at least 25 of 30 seeds. A detector that finds
eras in a world where nothing can happen is finding noise, and this check is
what catches it.

**Determinism.** The same event log analyzed twice, in separate processes,
produces byte-identical reports.

## Performance

Detection is offline and bounded, following the existing similarity-analysis
precedent: sampling bounds, stable-ID ordering, measured runtime reported
separately from tick cost, never inside the hot loop. Runtime is a recorded
benchmark line, not an assumption.

## Test Requirements

- Analysis-neutrality: world checksums bit-identical with detection on and
  off, at every supported cadence.
- Crate dependency direction asserted by the build: `sim-core` must not
  depend on `sim-analysis`.
- Synthetic ground truth precision and recall within stated bounds.
- Null control across the ablated seed set.
- Byte-identical reports across clean processes.
- Bounded memory and runtime on the largest supported event log.
- Genotype-matched control is present in every tradition finding; a finding
  constructed without one fails validation.
