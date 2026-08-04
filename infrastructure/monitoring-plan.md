# Monitoring Plan

## Phase 4 Integration Proposal (documentation only)

The application side is ready: `lifesim-server` exposes `/metrics` in
Prometheus text format behind the observer bearer token (loopback only),
covering world, stream, and save metric families
(`specifications/metrics-schema.md`). The proposed integration, pending
the separately approved live audit, is one scrape target for the server's
REST port with the token supplied via the scrape config's authorization
header, an operations dashboard (tick rate, observers, stream bytes,
dropped updates, save duration/failures) and a world dashboard
(population, births/deaths, biomass, ancestry depth). No Prometheus or
Grafana configuration has been touched, and none will be without explicit
approval and the verification steps below.

## Integration

Expose a private Prometheus metrics endpoint from the application. Add a scrape target and Grafana dashboards only after a live audit confirms the existing monitoring repository, owner, alert policy, port policy, and rollback/export procedure. Do not modify any current monitoring configuration during this planning phase.

## Required Verification

Before dashboard work, validate syntax with the native monitoring tooling, confirm the scrape reports success, query the new metric family directly, and then import/create dashboard panels. A Grafana panel is not proof that the source scrape works.

## Dashboard Separation

Use operations panels for availability/performance and separate world-analysis panels for ecosystem outcomes. Keep high-cardinality scientific detail in event/export systems, not Prometheus labels.

## Runbook References

See docs/16-observability.md for metric intent and infrastructure/backup-and-recovery.md for persistence health. Record the actual scrape target and owner only after verification, not in generic source files.
