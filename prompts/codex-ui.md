# Codex Prompt: Phase 3 Observer UI

## Role

You are implementing one narrowly scoped task in Phase 3 for Artificial Life Simulation.

## Required Reading
- README.md
- AGENTS.md
- planning/phase-3-live-observer.md
- docs/10-observer-interface.md
- docs/11-api-and-streaming-protocol.md
- docs/15-security-model.md
- specifications/websocket-protocol.md

## Objective
Implement a server-authoritative private observer slice with pixel art, scientific overlay, and bounded viewport streaming.

## Scope Boundary
- One UI feature plus its complete server/protocol/test boundary.
- PixiJS WebGPU-preferred/WebGL-fallback design.
- Desktop/mobile accessibility validation.

## Explicit Exclusions
- Full-world per-tick state transfer.
- Unauthenticated admin controls.
- Game-framework rewrite or public hosting.

## Required Workflow

1. Inspect the current worktree, relevant code, tests, config, and recent benchmark records.
2. Write a concise plan before edits. Identify determinism, schema, security, and performance effects.
3. Implement the smallest complete slice; do not refactor unrelated modules.
4. Add or update focused tests before claiming completion.
5. Run the required validations and report exact results.
6. Update affected documentation and decision records in the same change.

## Validation
- Protocol golden/negative tests.
- Browser E2E select/pan/reconnect.
- Desktop/mobile viewport check.
- Tick/backpressure benchmark with client.

## Documentation Updates
- Observer/API/protocol/security docs
- metrics spec if needed
- ADR/decision log.

## Completion Report
Report client/server state ownership, protocol version effect, accessibility result, browser fallback result, and measured stream impact.

If blocked by persistent-data compatibility, experiment meaning, security, credentials, or infrastructure access, stop safely and state the exact missing decision/evidence.
