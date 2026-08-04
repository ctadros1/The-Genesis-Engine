# Phase 16: Offline Era And Tradition Detection

Status: planned, not started. Analysis versions `lifesim-era-v1`,
`lifesim-tradition-v1`. Specification:
`specifications/era-and-tradition-detection.md`.

## Problem

By this point the event log records contest, structural mutation,
plasticity, signalling, and object events across many worlds and many
conditions. Nothing reads it at the level of "did this world go through
distinguishable regimes, and did any behavior persist beyond the individuals
that performed it".

This phase builds that reader. It adds nothing to the simulation.

## The Hard Rule

Analysis observes. It never instructs.

An era is a narrative an observer detects post hoc from the event log. It is
never a state the simulation enters and no organism is ever told about one.
There is no `era` field in world state, no era input channel, and no
era-conditioned rule.

This extends the existing `lifesim-similarity-v1` precedent, where species
labels are computed offline and never feed back into mating rules, action
eligibility, or behavior. It is enforced structurally: detection lives in a
separate crate `sim-analysis` that depends on `sim-core`, and nothing in
`sim-core` depends on `sim-analysis`, so feedback is a compile error rather
than a review finding. See ADR-0016.

## Scope

- Windowed feature extraction over the event log.
- Deterministic exact change-point segmentation.
- Tradition detection with a mandatory genotype-matched control.
- Report format with full provenance and explicit negative results.
- Validation against synthetic ground truth and against ablated null
  worlds.

## Non-Goals

- **No feedback into simulation state, ever.** Not as an input channel, not
  as a config trigger, not as an intervention, not as a rendering-driven
  behavior change.
- No era naming. Segments are called segments. They are never named after a
  human historical period, and no report says "stone age" or equivalent.
- No approximate or randomized segmentation. A nondeterministic analysis
  produces irreproducible reports, which is the one thing this project does
  not tolerate.
- No machine-learned classifier over the event log in this phase. A trained
  model would need its own provenance, reproducibility, and validation story
  before it could produce a claim this project would stand behind.
- No claim that a detected segment corresponds to anything in human history.

## Prerequisites

- Phase 5's append-only event log (the deferred D-019 item).
- Event schema version 3 covering contest, structural, social, and object
  events.
- Phases 11 and 12, because a tradition claim requires transmission and
  artifacts to exist for there to be anything to detect.

## Determinism Notes

- Detection draws from no `RngSystem` stream. If sampling is needed it uses
  a separate deterministic sampler seeded from the report parameters.
- Analysis versions are recorded in reports and are deliberately **not** in
  the config hash, because an analysis version can never affect a world.
- Reports are byte-identical across clean processes for the same event log
  and parameters.

## Acceptance Criteria

- [ ] **C16.1 Zero feedback, proven.** World state checksums are
      bit-identical with detection enabled at every supported cadence and
      with detection disabled. This mirrors the existing Phase 2
      analysis-neutrality test and is the phase's most important criterion.
- [ ] **C16.2 Dependency direction enforced by the build.** `sim-core` does
      not depend on `sim-analysis`. Asserted in CI, not by convention.
- [ ] **C16.3 Reproducible reports.** The same event log and parameters
      analyzed in two clean processes produce byte-identical reports.
- [ ] **C16.4 Synthetic ground truth.** On a fixture event log with injected
      known regime changes at known ticks, the detector recovers boundaries
      within `+/- k` windows at stated precision and recall. The generator
      and its injected changes are versioned fixtures.
- [ ] **C16.5 Null control.** On event logs from runs with the relevant
      mechanisms ablated (no artifacts, no signalling, no contest), the
      detector reports no segments above threshold in at least 25 of 30
      seeds. A detector that finds eras where nothing can happen is finding
      noise, and this is the check that catches it. Without C16.5 the whole
      analysis is unfalsifiable pattern-matching.
- [ ] **C16.6 Tradition findings carry their control.** Every tradition
      finding includes its genotype-matched control statistic, the matching
      tolerance, and the cohort size. A finding constructed without one
      fails report validation. A behavior shared by close kin is a plausible
      inherited trait, not a tradition, and the report format makes that
      distinction unavoidable.
- [ ] **C16.7 Negative results are reported.** A run with no segments above
      threshold and no traditions produces a report saying exactly that.
      Silence is not a result and an empty report is not the same as no
      report.
- [ ] **C16.8 Bounded cost.** Runtime and memory on the largest supported
      event log are measured and recorded separately from tick cost,
      following the existing similarity-analysis convention.

## Test Plan

- Analysis neutrality at every cadence, automated.
- Build-level dependency assertion.
- Byte-identical report reproduction across processes.
- Synthetic ground truth precision and recall against the fixture set.
- Null control across the ablated seed set.
- Report validation: a synthetic tradition finding missing its genotype
  control is rejected.
- Bounded memory on a synthetic oversized event log; fail-closed decode of a
  corrupted log.

## Benchmark Impact

Offline only. Record analysis runtime and peak memory against event-log size
and window count, measured separately from tick cost, exactly as the Phase 2
similarity-analysis runtime is recorded today. No tick-path impact is
permitted and C16.1 proves it.

Benchmark schema unchanged; analysis timings are a separate record section.

## Documentation Updates

`docs/09-species-and-lineage.md` (era and tradition detection joins the
analysis family), `docs/16-observability.md` (analysis reports are not
metrics), `specifications/event-schema.md`,
`specifications/metrics-schema.md` (report provenance fields),
`docs/25-emergence-and-epistemic-position.md` (status),
`research/performance-notes.md`, decision log, ADR-0016.

## Risks

| Risk | Mitigation |
|---|---|
| The detector invents narrative from noise | C16.5's null control is the direct guard, and C16.4 establishes it can find real changes when they exist |
| A tradition claim is made without the genetic control and propagates into summaries | The control is a required report field; a finding without one fails validation. This is enforced by the format, not by reviewer diligence |
| Era language leaks into reports and then into how the project describes itself | Segments are called segments; no historical naming anywhere; `docs/25-emergence-and-epistemic-position.md` governs the vocabulary |
| Analysis code eventually gets a "small" hook into the kernel for convenience | The crate dependency direction makes it a compile error, which is why C16.2 is an acceptance criterion rather than a guideline |
| Event log size makes analysis impractical | Measured in C16.8; windowing and sampling bounds are config; the log format is designed for streaming reads rather than full-file loads |

## Rollback

Detection is a separate crate and a separate command. Removing it changes no
world, no save, no protocol, and no config hash. This is the least risky
phase in the plan by construction, which is exactly what the
analysis-observes-never-instructs rule buys.
