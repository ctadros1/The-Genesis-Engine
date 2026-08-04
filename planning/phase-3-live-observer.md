# Phase 3: Live Observer

## Local Implementation Status

The Phase 3 slice was implemented on 2026-08-04: `crates/sim-protocol`
(binary frame codec `ALSP` 1.0), `crates/sim-server` (threaded private
REST + WebSocket server with bearer-token roles, audited controls,
idempotency, rate limits, and latest-wins backpressure), and
`apps/observer` (TypeScript/PixiJS observer with pixel terrain, organism
sprites, pan/zoom, selection inspector, scientific overlay, population
chart, admin controls, and reconnect/resync). Evidence: 14 server/protocol
integration tests, 7 browser E2E tests, and benchmark
`phase3-local-20260804T051859Z` in `research/performance-notes.md`.
Deployment, TLS/reverse-proxy, physical devices, and WebGPU preference
evidence remain open gates; the server binds loopback only by default.

## Purpose
Expose a responsive private observation/control experience while preserving server authority and tick isolation.

## Scope
- REST metadata/control API, binary WebSocket stream, React/PixiJS observer, selection, charts, pixel art, scientific overlay, viewport culling.
- Desktop/mobile/kiosk responsive behavior and private authentication boundary.

## Non-Goals
- Public multi-user product, full analytics warehouse, distributed rendering, controller topology visualization for every organism.

## Dependencies
- Phase 2 stable worlds and event/metrics sources.
- Versioned protocol and security model.

## Deliverables
- Private observer application with pixel/scientific modes.
- Keyframe/delta stream with bounded subscriptions and resync.
- Selected organism/lineage inspector, charts, basic validated controls.
- E2E reconnect/backpressure/mobile checks.

## Technical Tasks
1. Implement server auth/role checks and request audit trail.
2. Implement protocol negotiation/keyframe/delta/error frames.
3. Implement PixiJS layers, pan/zoom/LOD/culling, selection and inspector.
4. Implement chart queries/downsampling and accessibility alternatives.
5. Test slow client, disconnect, epoch changes, and narrow mobile viewport.

## Acceptance Criteria
- [x] Browser never owns world truth or blocks ticks. All state arrives
      from the stream; mutations go through the audited REST control API;
      per-client latest-wins queues mean a stalled client collapses to a
      keyframe resync (integration-tested) while ticks continue; measured
      tick p95 moved 354.6 -> 428.8 us from zero to four observers.
- [x] Reconnect/resync returns a consistent viewport state. E2E: forced
      socket close reconnects with backoff and a fresh self-contained
      keyframe atomically replaces client state; sequence gaps and lag
      trigger server-side keyframe resync.
- [x] Sandbox actions are authorized, bounded, and logged. Pause/resume/
      speed require the admin role; observer attempts are denied and
      audited; idempotency keys replay recorded responses; speed is
      clamped to [0, 64]; the audit log records actor role, action,
      acceptance, tick, and key.
- [x] Desktop/mobile/wall modes meet documented control/visibility
      behavior for this slice: desktop shows canvas, inspector, controls,
      and chart; the mobile viewport (390x844 E2E) keeps canvas and
      compact controls usable with bottom-sheet inspection; read-only
      operation is the no-admin-token default (kiosk/wall hardware checks
      remain an open device gate).

## Test Requirements
- API authorization/validation tests.
- Binary protocol golden/negative tests.
- Browser E2E pan, select, pause/control, reconnect, mobile smoke.

## Benchmark Requirements
- Render FPS/draw calls at dense viewport.
- Per-client stream bandwidth and dropped update behavior.
- Tick p95 with zero, one, and several observers.

## Documentation Updates
- Update observer/API/protocol/security/accessibility docs and front-end ADR.

## Risks
- Sending detail data in live frames can overload clients.
- A visual implementation may hide simulation drift instead of exposing it.

## Rollback Strategy
Disable live stream/control routes behind feature flag while retaining headless world. Revert protocol version rather than silently changing frames.

## Suggested Codex Prompt
Use prompts/codex-ui.md. Preserve the server authority boundary and test both WebGPU-preferred and WebGL fallback paths.
