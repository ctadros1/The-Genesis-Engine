> **Superseded 2026-08-04.** This plan predates the open-ended-evolution goal
> recorded in `docs/25-emergence-and-epistemic-position.md` and ADR-0012. It is
> preserved unmodified below for history. Do not execute it as written.
>
> Successors: see `docs/19-implementation-roadmap.md` for the current phase
> order. Performance work is now a standing discipline carried by every phase's
> Benchmark Impact section plus `planning/backlog.md`; the headless-throughput
> and independent-world parts moved into
> `planning/phase-5-headless-scale-and-experiments.md`. The advanced-ecosystem
> slices are now named phases 6 through 12.

# Phase 6: Advanced Ecosystems

## Purpose
Add richer ecological variation only after the base world is explainable, durable, observable, and benchmarked.

## Scope
- Climate/biome depth, predation refinement, disease, communication, sexual-selection depth, disasters, richer species models, optional structures.

## Non-Goals
- Assuming society or intelligence will emerge.
- Combining many unmeasured ecosystem rules in one release.

## Dependencies
- Phase 5 capacity evidence, Phase 4 durable branching, Phase 3 observability/control, research design review.

## Deliverables
- One bounded advanced-system slice at a time with versioned config, scenarios, metrics, and user-observable explanation.

## Technical Tasks
1. Select one research question/feature and define expected observable outcomes.
2. Model it as a bounded field/state machine/action extension.
3. Add config/schema/event/metric/overlay support.
4. Run controlled A/B seeded experiments and long-run stability tests.
5. Decide retain/revise/remove based on evidence.

## Acceptance Criteria
- [ ] The feature has explicit non-goals and does not violate kernel boundaries.
- [ ] Historic worlds remain interpretable through config/schema versions.
- [ ] User can observe and explain the feature's state/effects.

## Test Requirements
- Feature units, invalid input, deterministic scenario, long-run regression, UI overlay test.

## Benchmark Requirements
- Incremental phase cost, memory, stream payload, and save impact.

## Documentation Updates
- Update all affected model/spec/risk/decision documents and add an ADR.

## Risks
- Feature creep and causal opacity.
- False scientific framing.

## Rollback Strategy
Disable the feature by config/version for new worlds; retain readers/visualizers for old worlds. Preserve experiment evidence.

## Suggested Codex Prompt
Use prompts/codex-review.md first to assess scope. Implement exactly one advanced-system experiment, not a bundle.
