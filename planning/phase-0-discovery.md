# Phase 0: Discovery And Benchmarks

## Local Bootstrap Status

The approved local-only slice completed on 2026-08-03 EDT (benchmark ID
`phase0-local-20260804T030100Z`). It produced the deterministic tick, bounded
snapshot, and PixiJS renderer spikes plus raw local benchmark records. No host,
VM, network, monitoring, backup, or production service was accessed or changed.

The infrastructure audit, deployment-shaped VM run, physical mobile/kiosk
browser checks, compressed snapshot comparison, and any production decision
remain explicitly unresolved. Completion of this local slice does not authorize
Phase 1 or deployment work.

## Purpose
Validate the recommended technical and deployment baseline before committing production implementation structure or infrastructure changes.

## Scope
- Read-only audit of approved development/homelab facts if separately authorized.
- Minimal Rust kernel, save-format, renderer, and streaming spikes.
- Benchmark harness, hardware profile capture, and decision evidence.
- Architecture and VM recommendation finalization.

## Non-Goals
- No production world, public service, Proxmox change, GPU passthrough, or full application.
- No claim of target population capacity.

## Dependencies
- README.md, AGENTS.md, docs/01-user-requirements.md, docs/03-system-architecture.md.
- Explicit approval before any external host access or configuration change.

## Deliverables
- Benchmark harness with provenance schema.
- Small deterministic tick/snapshot prototype and rendering spike.
- Live-audit evidence or explicitly unresolved infrastructure facts.
- Accepted/revised proposed ADRs with benchmark evidence.

## Technical Tasks
1. Create a minimal pure tick microbenchmark with reproducible seed/config.
2. Compare Rust prototype to only one credible alternative if uncertainty remains.
3. Measure PixiJS WebGPU/WebGL fallback/culling on intended browsers.
4. Measure snapshot encode/decode/checksum for representative synthetic worlds.
5. Record candidate VM CPU/RAM/storage/network evidence read-only.
6. Decide whether Rust/PixiJS/CPU-first baselines advance.

## Acceptance Criteria
- [x] Every technical proposal exercised by this local slice has evidence or remains explicitly proposed.
- [x] Benchmark records include toolchain, hardware, config, seed, and raw-result location.
- [x] No infrastructure was changed; the run was local-only.
- [x] The narrowest Phase 1 proposal is bounded to a 500-organism deterministic headless world, subject to separate approval.

## Local Evidence

- Rust 1.97.1 unit, negative-decode, formatting, and Clippy checks passed.
- Two clean processes produced the same 500-organism/500-tick fixture checksum.
- Rust release benchmarks recorded 500 and 2,000-organism tick phases, RSS,
  snapshot encode/decode time, and snapshot size.
- PixiJS 8.19.0 WebGL smoke tests passed at desktop and mobile viewports.
- Local Chrome 150 completed recorded WebGL and WebGPU runs with viewport
  culling at desktop and mobile-sized viewports.
- Summaries and limitations are in `research/performance-notes.md`; raw files are
  under `benchmarks/raw/phase0-local-20260804T030100Z/` and intentionally ignored.

## Test Requirements
- Deterministic micro-tick equality fixture.
- Malformed snapshot decoding negative tests.
- Renderer smoke test on desktop and mobile viewport.

## Benchmark Requirements
- Tick phase timing and RSS at 500/2,000 synthetic organisms.
- Snapshot size/duration and browser draw/update cost.
- CPU-first versus any proposed alternative only if comparison is fair.

## Documentation Updates
- Update decision log, ADRs, VM requirements, performance strategy, and open questions.
- Add benchmark summaries under research/performance-notes.md.

## Risks
- Benchmark prototype may accidentally become production architecture.
- Observed host facts may drift before deployment.

## Rollback Strategy
Discard experimental code or isolate it behind a documented spike boundary. Do not apply changes to any host; remove only local disposable artifacts.

## Suggested Codex Prompt
Use prompts/codex-bootstrap.md with Phase 0 scope. Do not begin Phase 1 until every acceptance criterion is evidenced or explicitly deferred.
