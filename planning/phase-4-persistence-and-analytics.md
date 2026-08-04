# Phase 4: Persistence And Analytics

## Local Implementation Status

The Phase 4 slice was implemented on 2026-08-04: `sim_core::SaveState`
(logical capture and fail-closed restore with terrain regeneration and
full invariant verification), `crates/sim-persist` (ALIF format 1 codec
with per-section checksums and zstd, atomic temp+fsync+rename writes,
SQLite catalog with durable-file-first ordering, recovery scan, isolated
restore verifier, explicit fail-closed migration registry), CLI
save/load/verify/CSV-export/compare commands, and server checkpoint
scheduling with audited save/list/verify endpoints and `--load-save`
branching (new world epoch). Benchmark `phase4-local-20260804T141013Z`
records save/restore cost and the zstd-versus-uncompressed comparison.
Deliberately deferred within Phase 4 scope (documented in the decision
log): the append-only event-log file (snapshots carry a zero event-log
reference until it exists), Parquet export, and any live
Prometheus/Grafana integration (proposal only; no monitoring access is
approved).

## Purpose
Make worlds safely durable, branchable, exportable, and recoverable with provenance.

## Scope
- Manual/automatic saves, atomic checkpoints, catalog, recovery, versioned migration, export, experiment metadata, Grafana integration planning.

## Non-Goals
- PostgreSQL by default, untested automatic migration, production backup changes without explicit approval.

## Dependencies
- Stable world state model, schema versions, Phase 3 controls if UI exposure is desired.
- Approved backup/monitoring owners for any integration.

## Deliverables
- Framed snapshot codec, catalog, checkpoint scheduler, migration registry.
- Restore verifier and isolated restore runbook.
- CSV export and experiment comparison metadata.
- Prometheus/Grafana integration proposal backed by live validation when approved.

## Technical Tasks
1. Implement atomic writes/checksums and catalog transaction ordering.
2. Implement load-time bounds/version validation and explicit migration/rejection paths.
3. Implement named save/branch/restore workflow with audit trail.
4. Implement export schema/version manifest and basic experiment comparer.
5. Exercise backup and restore in an isolated target.

## Acceptance Criteria
- [x] Interrupted save never replaces last valid checkpoint. Files become
      durable (temp + fsync + atomic rename + directory sync) before any
      catalog row exists; the crash-simulation test leaves a partial temp
      file and a corrupted committed file, and recovery removes the temp,
      marks the corrupt row broken, and keeps the prior checkpoint
      authoritative and verifiable.
- [x] Valid restore preserves documented state/provenance. Restores compare
      world ID, tick, seed, config hash, terrain checksum, and the recorded
      state checksum; a restored world's checksum is bit-identical and its
      subsequent trajectory matches the original exactly (tested in-kernel,
      via the store, and across a copied backup set).
- [x] Invalid/incompatible save fails safely and clearly. Typed errors for
      magic/version/header/length/checksum/section faults; bounds cap
      allocation and decompression; a 2,000-case corruption sweep rejects
      >99.5 percent with zero panics; unknown format versions fail closed
      through the migration registry with actionable messages.
- [x] Recovery and restore have executed evidence. The
      restore-from-backup test copies a complete recovery set (snapshots +
      catalog) to an isolated target, validates it, verifies provenance,
      branches a world that continues identically, and proves the source
      set unmodified; server tests exercise live checkpoints, pruning,
      audited manual saves, isolated verify, and `--load-save` branching.

## Test Requirements
- Golden snapshot, corruption, truncation, oversized payload, and migration tests.
- Crash/interruption simulation around write stages.
- Restore-from-backup integration test.

## Benchmark Requirements
- Save duration/size at supported tiers.
- Restore duration and event-log/catalog growth.
- Export resource use and impact on tick p95.

## Documentation Updates
- Update storage/save/event/metrics specs, backup runbook, monitoring plan, decision log.

## Risks
- Schemas drift without migration discipline.
- Snapshots become too slow or large for continuous operation.

## Rollback Strategy
Keep prior reader/migrator compatible until a tested retirement decision. Restore from last validated set; do not hand-edit binary snapshots.

## Suggested Codex Prompt
Use prompts/codex-persistence.md. Treat every decoder as hostile-input code and prove restore before UI polish.
