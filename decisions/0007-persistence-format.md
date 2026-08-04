# ADR-0007: Persistence Format

Status: Proposed
Date: 2026-08-03
Author: Project planning

## Context
Worlds need durable checkpoints, named saves, replay provenance, and safe migration without database overreach.

## Options Considered
- Versioned binary snapshots plus SQLite catalog.
- MessagePack-only files.
- Protocol Buffers/FlatBuffers.
- PostgreSQL-first.
- Raw runtime serialization.

## Proposed Decision
Propose framed custom logical binary snapshot compressed with zstd, append-only events, and SQLite metadata catalog.

## Consequences
Requires explicit schema/migration code but keeps deployment simple and files portable.

## Performance Implications
Measure encode/decode/size and copy-on-write impact at supported tiers.

## Operational Implications
Back up snapshot/catalog/config as one recovery set; no database server initially.

## Revisit Conditions
Phase 4 evidence shows query/concurrency/analytics needs exceed SQLite/file model.

## Evidence Required To Accept

- Phase-specific tests and benchmark evidence.
- Compatibility and rollback impact.
- Explicit review/approval when production infrastructure is affected.

## Phase 0 Local Evidence

The spike implements only a versioned little-endian frame, bounded lengths,
entity section, payload CRC32, state checksum, and malformed-input rejection.
At 2,000 organisms the uncompressed snapshot was 32,072 bytes with p95 encode
274.709 us and decode 266.417 us on the local M3 Pro. Compression, atomic file
writes, migration, SQLite, and restore are not implemented or validated.
Status remains Proposed.

## Phase 4 Local Evidence

The full proposed stack now exists and is tested: framed ALIF format 1
snapshots with per-section checksums, zstd compression, atomic
temp+fsync+rename writes with catalog-after-durability ordering, SQLite
catalog, recovery scan, isolated restore verification, and a fail-closed
migration registry. The explicitly bounded compressed-codec comparison the
Phase 0 gate required is recorded in `research/performance-notes.md`
(benchmark `phase4-local-20260804T141013Z`): zstd-1/-3 cut snapshot size
30-48 percent at equal-or-better encode/decode time versus the
uncompressed codec, supporting the zstd choice (server default level 3).
Crash-simulation, corruption-sweep, and restore-from-backup tests all
pass. Remaining before acceptance: deployment-shaped storage evidence,
the event-log segment, retention/backup-destination policy (infrastructure
approval), and explicit user approval. Status remains Proposed.
