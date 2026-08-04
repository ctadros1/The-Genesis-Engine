# Storage Plan

## Data Classes

| Class | Retention Concept | Integrity |
|---|---|---|
| Active snapshots | recent checkpoints plus named saves | checksum and atomic write |
| Event logs | retained for replay/audit window | framed append/checkpoint linkage |
| SQLite catalog | paired with saves and backups | transactional backup/check |
| Exports | user-requested, immutable by run version | manifest/checksum |
| Benchmarks | raw outside Git, curated summaries in Git | hardware/config provenance |

## Capacity Policy

Set snapshot count, interval, compression, event retention, and export quota only after measuring world size and target pool capacity. Enforce disk-watermark behavior: reject new manual saves safely, retain last known good checkpoint, alert, and never corrupt old saves attempting to free space.

## No Assumptions

Do not assume servernode3 storage medium, free space, RAID/ZFS policy, snapshot semantics, or backup target. Phase 0 documents live evidence before a VM disk size or retention schedule is accepted.
