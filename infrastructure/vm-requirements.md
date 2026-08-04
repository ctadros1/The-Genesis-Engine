# VM Requirements

## Proposed Guest

One Ubuntu LTS VM on servernode3 is recommended pending Phase 0 audit. Initial target: 16 vCPUs, 16 GiB RAM minimum/24 GiB preferred, 100-200 GiB verified fast local disk, QEMU guest agent, and no GPU passthrough.

## Why

The workload is CPU/memory/cache sensitive and benefits more initially from predictable isolated resources than from an older consumer GPU. A VM protects existing host services and provides a coherent backup/rollback unit.

## Acceptance Checklist

- Verify host CPU pressure, memory headroom, storage latency/free capacity, and existing guest contention.
- Verify guest CPU flags and pinning/scheduling policy if used.
- Verify secure access path, static/reserved address, DNS policy, and time synchronization.
- Verify backup target and test an empty guest restore procedure.
- Record all actual allocations and deviations in the deployment decision record.

## New Sizing Driver (2026-08-04)

The open-ended-evolution goal changes what this guest has to be sized for.
The dominant demand is no longer one live world at 10 Hz; it is **many
independent worlds running headless as fast as possible**, because every
acceptance criterion from Phase 6 onward is a multi-seed, multi-condition
claim and the cost is run length times seeds times conditions.

Three consequences for the audit, none of which authorizes any access:

- vCPU count matters more than before, because Phase 5's scheduler runs N
  worlds concurrently and per-world throughput degrades with contention.
  Measure that degradation curve rather than assuming linear scaling.
- Disk grows faster than the current plan assumes. Each world writes
  snapshots and an append-only event log, and Phases 7, 8, and 10 each add a
  snapshot growth term to a payload already dominated by per-organism genome
  arrays. Campaign disk budget is a per-run measurement, not an estimate.
- Memory scales with concurrent worlds, not with one world.

**Whether this guest can supply enough compute for a full campaign is
unknown and unmeasured.** It is recorded as an unresolved risk in
`docs/20-risk-register.md`, not as a mitigated one. Phase 5 measures the
throughput ceiling; nothing here claims a campaign size.

## Phase 0 Status

No Proxmox host, VM, network, monitoring, storage, or backup system was accessed
during the local bootstrap. The local M3 Pro results cannot validate this guest
size or any checklist item above. The proposed allocation and ADR-0008 remain
unchanged and unaccepted pending separately approved read-only evidence.
