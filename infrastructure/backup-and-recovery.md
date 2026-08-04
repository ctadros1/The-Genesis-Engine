# Backup And Recovery Runbook

## Phase 4 Local Evidence

The application-side half of this runbook is implemented and tested
locally: a recovery set is the data directory (ALIF snapshots plus the
SQLite catalog); the `sim-persist` restore-from-backup test packages one,
transfers it to an isolated target, validates checksums and provenance
(world ID, tick, seed, config hash, state checksum), branches a world that
continues bit-identically, and proves the source unmodified. Interrupted
saves are removed at recovery and never displace the last valid
checkpoint. The approved backup destination, transfer mechanism,
retention policy, and any VM-level coordination remain unimplemented and
require separate infrastructure approval.

## Purpose

Protect world snapshots, event logs, configs, SQLite catalog, and deployment metadata so a world can be restored with known provenance. VM-level backups supplement rather than replace application-consistent saves.

## Prerequisites

- Approved backup destination and retention policy.
- A completed, validated application checkpoint.
- Read-only knowledge of current VM/application version and config hash.
- An isolated restore target for test recovery.

## Procedure

1. Confirm current checkpoint integrity and catalog consistency.
2. Package snapshot, event segment, config, SQLite catalog, checksum manifest, and migration tooling as one recovery set.
3. Transfer using the approved backup mechanism; record immutable backup ID.
4. Restore to an isolated path/guest first.
5. Run snapshot validation and a read-only/world-paused load check.
6. Compare world ID, tick, config hash, checksums, and catalog metadata.
7. Record success/failure and retention status.

## Verification

- A restored world loads without schema/checksum warning.
- It reports expected world ID/tick/config/build provenance.
- The original live world was not modified during test restore.

## Troubleshooting

| Symptom | Likely Cause | Safe Response |
|---|---|---|
| Checksum mismatch | partial/corrupt transfer | reject restore; retain source; recopy verified set |
| Unsupported format | missing migration/binary | use compatible release or preserve read-only |
| Catalog points to missing file | incomplete backup set | restore paired catalog/snapshot or rebuild only with verified manifest |

## Escalation

Escalate before replacing any live world, changing VM snapshots, or altering backup retention. A restore that has not passed isolated verification is not eligible for production use.
