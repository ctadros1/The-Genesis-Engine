# Claude Session Guide

This repository is in planning phase. Treat the documentation as the source of truth until an implementation phase is explicitly approved.

## Required Session Start

1. Read README.md and AGENTS.md.
2. Identify the active item in planning/.
3. Read relevant docs/ and specifications/ files.
4. Check docs/22-decision-log.md and unresolved items in docs/21-open-questions.md.
5. Inspect existing code and run the current focused tests before edits.

## Non-Negotiables

- Keep sim-core independent from UI, transport, and deployment concerns, and
  independent from sim-analysis. Analysis observes; it never instructs.
- Author physics, never progress. No technology tree, research graph, era
  state, recipe, or civilization mechanic. If you can name the outcome a
  mechanism makes more likely, it is authored progress.
- Preserve deterministic behavior and version all behavior/schema changes.
  `specifications/determinism-extensions.md` is normative for everything
  from Phase 5 onward.
- Treat rules as tunable experiment policies, not permanent design laws.
- Do not claim scale, cross-platform determinism, GPU value, or emergent
  behavior without measured evidence. Every behavioral claim needs a stated
  control or ablation; a measured null is a result.
- Do not touch homelab infrastructure without explicit approval.
- Do not use an LLM as the decision engine for organisms, and do not deliver
  an authored reward to any network.

## Orientation After The 2026-08-04 Goal Change

Read `docs/25-emergence-and-epistemic-position.md` before making any claim
about what the project does or expects. It separates what the simulation
makes possible, what we hope to observe, and what we actually predict, and
the honest prior is not favorable. Read
`docs/26-biological-realism-policy.md` before adding any biological
mechanism.

## Completion Report

State: scope completed, files changed, tests/benchmarks run and results, documentation/ADR updates, compatibility effects, and any remaining risks. If blocked, state the precise missing fact or decision and stop safely.
