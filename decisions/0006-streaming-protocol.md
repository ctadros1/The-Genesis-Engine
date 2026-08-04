# ADR-0006: Observer Streaming Protocol

Status: Proposed
Date: 2026-08-03
Author: Project planning

## Context
Live views must not transfer the full world every tick or allow slow clients to stall the server.

## Options Considered
- REST polling.
- JSON WebSocket full state.
- Binary WebSocket viewport keyframe/delta plus REST metadata.
- Native client protocol.

## Proposed Decision
Propose REST for commands/details and versioned binary WebSocket viewport keyframes/deltas.

## Consequences
Adds protocol/schema test burden but controls bandwidth and supports resync.

## Performance Implications
Measure state frame size, drop/resync behavior, and tick impact at observer counts.

## Operational Implications
Needs protocol version rollout policy and private access controls.

## Revisit Conditions
Protocol spike fails debugging/compatibility objectives or a simpler framed format proves sufficient.

## Evidence Required To Accept

- Phase-specific tests and benchmark evidence.
- Compatibility and rollback impact.
- Explicit review/approval when production infrastructure is affected.

## Phase 3 Local Evidence

Protocol `ALSP` 1.0 is implemented (`crates/sim-protocol`) and served by
`crates/sim-server`: REST for metadata/detail/controls, binary WebSocket
keyframe/delta viewport streaming with acks, sequence tracking, and
latest-wins backpressure that collapses slow clients to a keyframe resync.
Golden/negative/corruption codec tests, 14 integration tests, and 7
browser E2E tests pass. Measured locally (benchmark
`phase3-local-20260804T051859Z`): ~190 KB/s per full-world 20 Hz client,
zero drops for healthy clients, and tick p95 354.6 -> 428.8 us from zero
to four observers. Deployment-network behavior, TLS, and multi-device
evidence remain open. Status remains Proposed.
