# Proxmox Deployment Recommendation

## Status

This is a proposed deployment shape. It has not been applied and must not be applied during planning. Node capacity, storage, network addressing, monitoring ownership, backup target, and guest CPU features require a live read-only Phase 0 audit.

## Recommended Initial Placement

Use one isolated Ubuntu LTS VM on servernode3. Its described 64 GiB ECC RAM and dual-Xeon capacity provide the best starting headroom for a 16-vCPU, 16-24 GiB RAM workload and occasional independent experiments. The GTX 1660 Ti remains unused initially. Servernode1 is a secondary candidate for a lighter observer/experiment role; servernode2 is not the primary target because its mobile hardware and existing workload risk are less suitable for a continuous core service.

## VM Versus LXC

Choose a VM first. It offers stronger isolation, clearer backup/rollback boundaries, kernel/toolchain flexibility, and less coupling to existing host services. LXC can be benchmarked later for density, but it is not worth optimizing before simulation correctness and operational ownership are proven.

## Proposed VM Profile

| Resource | Proposed Initial Value | Validation Required |
|---|---|---|
| vCPU | 16 | host contention, CPU flags, tick benchmark |
| RAM | 16 GiB minimum, 24 GiB preferred | host free memory, long-run RSS |
| Disk | 100-200 GiB fast local storage | actual pool type, IOPS, backup capacity |
| GPU | None | compare CPU batching before passthrough |
| Network | private LAN/WireGuard-only | static/DHCP reservation, firewall, DNS |
| OS | supported Ubuntu LTS | current image and update policy |

## Service Topology

Run one application image/process per world service and serve compiled observer assets from the same application boundary initially. Use a minimal Compose or systemd-managed container deployment only after an implementation image exists. Add a reverse proxy only when TLS/routing needs are concrete. Keep Prometheus scraping pull-based and expose only a bounded metrics endpoint.

## Safety Constraints

- No Proxmox host change, VM creation, GPU passthrough, firewall edit, DNS edit, or monitoring edit is authorized by this document.
- Do not disrupt existing VMs/containers or reuse their storage/network settings by assumption.
- Test a guest backup and restore away from the running world before assigning an operational recovery target.
- Treat every endpoint address and existing monitoring path as live state, not embedded documentation truth.

## Rollout Gate

A deployment proposal can advance only after Phase 0 records host/guest evidence, Phase 4 proves restore, Phase 5 produces a capacity benchmark, and an explicit user approval authorizes infrastructure changes.
