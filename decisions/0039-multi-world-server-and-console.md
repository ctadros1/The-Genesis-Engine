# ADR-0039: Multi-World Server And The Console

Status: accepted 2026-09-04. Design authority:
`planning/console-and-multi-world-server.md`. Supersedes nothing;
extends ADR-0005 (TypeScript + PixiJS shell), ADR-0006 (the streaming
protocol, unchanged at `ALSP` 1.0) and the Phase 3 server. Where this
record and the plan disagree, the disagreement is a defect in this
record.

## Context

`lifesim-server` hosts exactly one authoritative world, created from
its command-line flags; its only controls are pause, resume and speed;
the Observer is a single-page canvas behind a token paste. The owner
asked for a console that greets a connection like a game's title
screen and lets a user spin up worlds, watch the ones running, and
edit a world's settings. Four decisions were put to the owner and
answered: settings are edited **at creation only** (a running world's
settings are hashed into every result it produces; a live edit would
fork its identity); the server becomes **multi-world**; the console is
a **fresh app that reuses the Observer's renderer and protocol code**;
it runs first as a **developer instance on the VM** behind the
bearer-token model, and production deployment stays with the root-only
installer.

## Decision

1. **The server hosts many worlds in one process.** A `Hub` owns the
   tokens, the audit log, the idempotency cache, the snapshot store and
   a registry of `WorldRuntime`s; each runtime owns one `World`, its
   control state, its subscribers, its tick thread and its counters.
   World ids are process-assigned integers; the world built from the
   command-line flags is world 1, so every existing route, test and the
   production deployment keep their meaning. The bound is
   `--max-worlds` (default 8).
2. **Worlds are created from a preset plus named settings**, the same
   vocabulary campaigns use (`sim-experiment::fields`: the field
   registry, `set_field`, typed values), validated by `SimConfig::validate`,
   and reported with their config hash before and after creation
   (`POST /api/schema/preview`, `POST /api/worlds`). The seed is set at
   creation and never a "setting". No setting of a running world can be
   changed; the control surface stays pause, resume, speed and stop.
3. **Saves stay in the one snapshot store**, which already carries a
   `world_id` column; a world's saves are the rows with its id, and
   **branching** a save makes a new world (new epoch, parent recorded),
   which is how a world outlives a server restart in this increment.
4. **The stream selects a world by WebSocket path** (`/worlds/{id}`,
   with `/` meaning world 1). `ALSP` stays at 1.0: no frame changes, the
   Welcome's `world_id` field reports the selection, an unknown or
   stopped world answers Error 404/410 and closes.
5. **The console is `apps/console`**: plain TypeScript, Vite, PixiJS 8,
   a screen stack (title, connect, worlds, builder, live, saves), server
   profiles remembered in the browser, and the Observer's `protocol.ts`
   and `render.ts` copied in with their provenance noted. Admin actions
   confirm before they send and every one is audited server-side, as
   `docs/10` requires. `apps/observer` is untouched and keeps working
   against world 1.
6. **Developer deployment**: a second server instance on the VM on
   developer ports with its own data directory and tokens from the
   environment, reached through the SSH alias's local port-forward;
   `scripts/run-console-dev.sh` starts it. Production (Caddy at
   `genesisengine.local`) changes only through the installer and is the
   owner's call.

## What this is not

- Not a kernel change: no field, hash, fixture or format moves; every
  verify script asserts it.
- Not live editing: there is no route that mutates a world's config.
- Not a scheduler: worlds are independent tick threads; no fairness,
  priority or resource accounting beyond the count bound and the
  measured tick cost each world reports.
- Not durable registry state: a server restart boots world 1 only; the
  others are saves to branch from. Recorded as the first revisit.

## Consequences

- One process serves the production world and any number of sandbox
  worlds up to the bound; the tick threads share the VM's cores with
  the production world, so the console shows each world's measured
  tick cost and the operator decides.
- Every mutation stays admin-only, idempotent where keyed, rate-limited
  and audited with its world id.
- The field registry gains a read side (`get_field`, field types) so
  the schema endpoint can report defaults per preset; it is the same
  table campaigns validate against, so the console cannot name a
  setting a campaign could not.

## Revisit

When a world must survive a restart without a manual branch (a durable
registry); when a third role is needed (per-world ownership); when
`ALSP` 1.1 lands (objects and modified terrain reach the console
through the same path selection); if the tick threads contend with the
production world measurably (a scheduler or a second process).
