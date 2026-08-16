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

## Planned Successor: ALIF Format 2 (Phase 12)

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

*As built (Phase 12, 2026-08-16): the terrain split above landed with the
mutable-world half (format 4, section 12); the artifact half then took the
format to 7 - forced by two new config fields, the artifact section and
`genome2.mutation.binding_q16`, not by objects - and added section 15
(`SECTION_OBJECTS`, present when `artifact.enabled`), whose layout and
count-bounding rule are in `specifications/world-save-format.md`. Format 6
files load through `FORMAT6_TO_CURRENT`; a world with the section on cannot
be written as format 6.*

Two other storage-relevant changes:

- **The append-only event-log file, deferred under D-019, moves into Phase 5
  scope.** Every multi-seed experiment needs it long before era detection
  does, and snapshots carry a zero event-log reference until it exists.
- **Checkpointing moves off the tick thread** in Phase 5. It is currently
  synchronous, and Phases 9, 11, and 12 each add a snapshot growth term to a
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

### Asynchronous Checkpointing (Phase 5, implemented)

It did become material, it was measured, and the model changed. Phase 4 ran
steps 3 through 6 on the tick thread; Phase 5 keeps only step 2 there.

The tick thread captures `SaveState` — an owned deep copy at a tick
boundary — and hands it to a writer thread that does the encoding,
compression, atomic write, `fsync`, catalog commit, and pruning. The
durability ordering above is untouched, so an interrupted asynchronous write
leaves exactly the same evidence an interrupted synchronous one did, which
is what the extended crash-simulation test asserts.

Because the writer only ever sees an owned immutable capture, it cannot
observe a torn world: there is no path by which a snapshot is encoded from
live arrays. That is what makes the asynchronous path safe rather than
merely faster.

The queue holds **at most one** request. A checkpoint requested while
another is still being written is refused and counted, never queued and
never silently discarded: an unbounded queue under a slow disk turns a
latency problem into a memory problem, and a silent drop makes the
configured checkpoint interval a lie. Refusals are exported as
`lifesim_checkpoints_skipped_total`.

The synchronous path remains available behind `--checkpoint-mode sync` so
the Phase 4 behavior can be measured and rolled back to.

Measured effect (`phase5-local-20260804T210059Z`): the tick-thread stall
falls from 26.2 ms to 1.4 ms at the 500 tier and from 68.0 ms to 4.7 ms at
the 2,000 tier, against a 100 ms tick budget. The write itself did not get
cheaper — it peaked at 86.3 ms — it stopped happening on the tick thread.

### The Event Log (Phase 5, implemented)

The append-only event-log file deferred under D-019 exists; framing and
decode rules are specified in `specifications/event-schema.md`. Snapshots
now record a real `event_log_offset`: the byte length of the log at capture,
so a restored world knows where its recorded history stops. Snapshots taken
without a log still record zero, and that remains valid.

Measured growth: 20.7 MB per 10^6 ticks at the 2,000 tier and 0.99 MB at the
500 tier, at 48-58 bytes per event. The write cost is below this host's
run-to-run noise floor and is deliberately **not** claimed as a percentage.

## Recovery And Compatibility

On startup, scan and validate the catalog and latest checkpoints. Reject invalid/incompatible saves with actionable reason codes. Supported migrations are explicit, tested transforms from one format version to another. If no safe migration exists, preserve the old save read-only and require a compatible binary; never best-effort load an unknown schema.

## Replay

Strict replay requires compatible build/config/generator/genome versions, world seed, starting snapshot, and ordered intervention log. Replay output may define byte-exact, checksum-exact, or tolerance-exact guarantees by version/platform. State the achieved guarantee in metadata and reports.

## Backup

Back up snapshots, event logs, config, SQLite catalog, and migration tools together. VM snapshots are not a substitute for application-consistent saves. Restore testing belongs in Phase 4 and the operations runbook.
