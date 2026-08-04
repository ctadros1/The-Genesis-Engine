# Data Storage And Saves

## Phase 4 Implementation Status

Implemented: ALIF format 1 snapshots (zstd by default at level 3;
uncompressed supported), the SQLite catalog with durable-file-first
commit ordering, automatic checkpoints with retention pruning
(`--checkpoint-interval-secs`, `--checkpoint-keep`), audited manual saves
and isolated verify via the admin REST API, startup recovery scanning,
`--load-save` branching with a new world epoch, `lifesim verify-save`,
versioned CSV metrics export (`lifesim run --csv-out`), and the
`lifesim compare` experiment comparer (same-lineage detection by config
hash/seed/policy). The append-only event-log file and Parquet export are
explicitly deferred (decision log D-019); snapshots carry a zero
event-log reference until it exists. Restore-from-backup evidence lives
in the `sim-persist` test suite per the backup runbook.

## Planned Successor: ALIF Format 2 (Phase 10)

Design in `specifications/mutable-world-state.md` and
`specifications/world-save-format.md`; decision in ADR-0015.

The format 1 property that terrain is regenerated rather than stored does
not survive organism-modified terrain. It is split rather than abandoned:
the **baseline** still regenerates from `(seed, config)` and is verified
against its recorded checksum exactly as today, and a separately checksummed
modification delta is stored and applied in a fixed order, with a second
checksum over the composed result. Both are verified before a world exists,
so the fail-closed restore property is preserved rather than weakened.

Format 1 is never reinterpreted in place. It loads through a registered
migration whose result must be byte-identical to a format 1 load, and format
1 readers and tests stay in the build permanently. This is the largest
operational risk in the plan and the byte-identity test is its guard.

Two other storage-relevant changes:

- **The append-only event-log file, deferred under D-019, moves into Phase 5
  scope.** Every multi-seed experiment needs it long before era detection
  does, and snapshots carry a zero event-log reference until it exists.
- **Checkpointing moves off the tick thread** in Phase 5. It is currently
  synchronous, and Phases 7, 8, and 10 each add a snapshot growth term to a
  payload already dominated by per-organism genome arrays at roughly 2.8 KB
  each. The checkpoint budget is re-verified in each of those phases rather
  than assumed to carry forward.

Backup sets keep their shape: snapshots, catalog, config, and now the event
log, backed up and restored together. The restore runbook gains the
composed-terrain verification step.

## Recommended Initial Storage

Use versioned compressed binary world snapshots plus an append-only event log and a small SQLite catalog. This avoids PostgreSQL operational overhead while supporting named worlds, metadata query, manual/automatic saves, lineage/event lookup, and exports.

| Data | Initial Format | Rationale |
|---|---|---|
| World snapshot | Framed custom binary, zstd-compressed | Compact, deterministic field ordering, explicit schema version |
| Metadata/catalog | SQLite | Simple local query/index/transaction support |
| Events | Framed append-only binary or JSONL export | Replay/audit and resilient append path |
| Analytics export | CSV initially; Parquet in Phase 4 if analysis needs it | Simple first, columnar when justified |
| Config | Versioned human-readable TOML/JSON | Inspectable experiment definition |

## Snapshot Contract

A snapshot header includes magic bytes, format version, simulation build version, config hash, generator/genome/protocol versions, world ID, parent world ID, tick, seed, compression flag, payload length, and checksum. The payload contains only documented logical state in stable field order. Never serialize pointers, runtime task state, browser state, or unvalidated raw memory.

## Save Procedure

1. Reach a safe tick boundary.
2. Capture a consistent immutable state view.
3. Encode, compress, checksum, and write to a temporary file in the destination filesystem.
4. Flush file and directory metadata as supported, then atomically rename.
5. Commit catalog metadata only after the file is durable.
6. Emit save duration/size/success metrics and retain the last known good checkpoint.

Save work must not block the hot tick indefinitely. If copy-on-write/serialization cost becomes material, measure it before changing the model.

## Recovery And Compatibility

On startup, scan and validate the catalog and latest checkpoints. Reject invalid/incompatible saves with actionable reason codes. Supported migrations are explicit, tested transforms from one format version to another. If no safe migration exists, preserve the old save read-only and require a compatible binary; never best-effort load an unknown schema.

## Replay

Strict replay requires compatible build/config/generator/genome versions, world seed, starting snapshot, and ordered intervention log. Replay output may define byte-exact, checksum-exact, or tolerance-exact guarantees by version/platform. State the achieved guarantee in metadata and reports.

## Backup

Back up snapshots, event logs, config, SQLite catalog, and migration tools together. VM snapshots are not a substitute for application-consistent saves. Restore testing belongs in Phase 4 and the operations runbook.
