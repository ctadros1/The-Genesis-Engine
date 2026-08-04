# Infrastructure Documentation

This directory defines a proposed private deployment path for the artificial-life system. It is planning material only. It does not authorize or enact any Proxmox, VM, storage, network, GPU, firewall, proxy, Prometheus, Grafana, or backup modification.

Read vm-requirements.md, network-plan.md, storage-plan.md, gpu-evaluation.md, deployment-plan.md, backup-and-recovery.md, and monitoring-plan.md before deployment work. Validate every environment fact live and obtain explicit approval for each change.

## Ownership

The project owner approves infrastructure scope. The person responsible for the existing Proxmox, network, monitoring, and backup systems confirms target values and rollback procedures. Future agents must not infer ownership from a host name, a prior task, or a path in this repository.

## Evidence Gate

Before implementation rollout, record a read-only audit of the proposed guest allocation, storage pool, network route, existing monitoring conventions, and backup target. Treat addresses, free capacity, metrics paths, and dashboard state as current operational facts that may drift.

## Safe Default

Until a rollout is explicitly approved, work locally on documentation, tests, benchmark artifacts, and disposable development environments only. No infrastructure task is complete without verification and rollback evidence.
