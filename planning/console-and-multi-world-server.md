# The Console And The Multi-World Server

Status: planned 2026-09-04, in progress. Decisions: ADR-0039 (this
increment), ADR-0005 (shell), ADR-0006 (protocol), ADR-0016 (analysis
observes). Not a numbered science phase: an interface and operations
increment on the roadmap's observer track.

## Problem

Connecting to the server today means pasting a token into a page that
shows one world. The owner wants to connect and be greeted like a game:
a title screen and a menu, from which worlds are spun up, watched,
paused and saved, with a world's settings edited before it starts.
The server hosts one world and cannot create another; its settings
come from command-line flags.

## Scope

**Server (`crates/sim-server`, plus a read side in
`crates/sim-experiment/src/fields.rs`).** Multi-world hosting per
ADR-0039. Routes (bearer token on everything but `/api/health`;
observer role reads, admin role mutates; every mutation audited with
its world id, keyed mutations idempotent, controls rate-limited):

| Method | Route | Role | Meaning |
|---|---|---|---|
| GET | `/api/schema` | observer | presets (`phase1`, `phase2`, description each), every field (`name`, `type` in u32/u64/i32/i64/bool/choice, `choices` for choices, `defaults` per preset), limits (`max_worlds`, `max_cells_x`, `max_cells_y`) |
| POST | `/api/schema/preview` | observer | body `{preset, seed?, settings:{name:value}}` -> `{config_hash, seed, valid, errors:[...]}`; creates nothing |
| GET | `/api/worlds` | observer | every world's summary |
| POST | `/api/worlds` | admin | body `{name, preset, seed?, settings:{}, paused?, speed?}` -> 201 summary; 400 on a bad field or a validation error (the message names the field); 409 at the bound |
| GET | `/api/worlds/{id}` | observer | summary: `world_id, name, status (running/paused/stopped), created_unix_ms, parent_world_id, world_epoch, preset, seed, config_hash, cells_x, cells_y, cell_size_m, dt_ms, tick, population, births_total, deaths_*, extinct, phase2, paused, speed_multiplier, tick_mean_us, ticks_per_second, total_biomass_milli, total_energy_milli` |
| POST | `/api/worlds/{id}/control?action=pause,resume,speed,stop` | admin | as today per world; `stop` ends the tick thread after a final checkpoint when a store exists; a stopped world stays readable and saveable |
| DELETE | `/api/worlds/{id}` | admin | removes a stopped world from the registry; 409 while running; saves stay on disk |
| GET/POST | `/api/worlds/{id}/saves` | observer/admin | rows with this world id; create as today |
| POST | `/api/worlds/{id}/saves/{save_id}/verify` | admin | as today |
| POST | `/api/worlds/{id}/branch?save_id=N&name=` | admin | 201 a new world loaded from that save (epoch 2, `parent_world_id` = id) |
| GET | `/api/worlds/{id}/organisms/{oid}`, `/api/worlds/{id}/analysis` | observer | as today per world |
| GET | `/api/audit`, `/metrics`, `/api/benchmarks/ticks?world=` | admin/observer | records and series carry `world_id` |

WebSocket: path `/worlds/{id}` selects the world (`/` is world 1); the
Hello/Welcome handshake and every frame are unchanged (`ALSP` 1.0);
Welcome reports the selected `world_id`; unknown world -> Error 404 and
close; a world stopped mid-session -> Error 410 and close.

Flags: `--max-worlds N` (default 8). World 1 still comes from the
existing flags, `--run-ticks` and `--load-save` keep their meaning for
it, and every existing integration test passes unchanged.

**Console (`apps/console`).** Plain TypeScript + Vite + PixiJS 8; no
framework; `protocol.ts` and `render.ts` copied from `apps/observer`
with a provenance header. Screens, as a stack with keyboard navigation
(arrows, enter, escape) and pointer:

1. **Title** - the name, a live pixel background (the renderer drawing
   the most recently viewed world at low zoom when connected, a
   procedural drift when not), and the menu: Continue (last world),
   Worlds, New World, Load Save, Server, About. Reduced motion honoured.
2. **Server** - profiles (name, REST base, WS base, observer token,
   admin token) kept in `localStorage`, a Test button
   (`/api/health` then `/api/worlds`), the role shown as a badge; the
   last profile auto-connects on launch.
3. **Worlds** - one card per world: name, status, tick, population,
   a sparkline from the metrics stream, seed, config hash, tick cost;
   actions View, Pause/Resume, Save, Stop, Branch, Delete (admin only,
   each confirmed in a dialog that names the world and the action).
   Refreshes every two seconds while open.
4. **New World (the builder)** - preset picker with descriptions;
   recipes (named settings sets shipped with the console, e.g. Phase
   22's chemistry-field base) applied on top of a preset; the settings
   grouped by prefix with search, typed inputs (toggle, number with the
   type's range, choice select), a seed field with Randomise; the
   server's preview shows `config_hash` and validation errors live
   (debounced); Create -> the world's live view. A summary of settings
   that differ from the preset is shown before Create.
5. **Live** - the Observer's canvas, HUD, inspector, chart, overlay,
   pause/resume/speed, plus Back and a world switcher; reconnect and
   keyframe resync as today.
6. **Saves** - per world: list, Save now, Verify, Branch into a new
   world; per the interaction rules every admin action confirms.

Style: a pixel-art surface on a dark palette, `image-rendering:
pixelated` on the canvas, large legible type, real buttons and live
regions (docs/10 accessibility), no colour-only signals.

**Developer instance.** `scripts/run-console-dev.sh` builds the server
and the console, starts `lifesim-server --rest-port 8960 --ws-port 8961
--data-dir $HOME/console-dev-data --max-worlds 8` with tokens from the
environment (generated and printed once when unset), and serves the
built console on 127.0.0.1:5280; the owner reaches both through
`ssh -L 5280:127.0.0.1:5280 -L 8960:127.0.0.1:8960 -L 8961:127.0.0.1:8961
genesis-engine`. `docs/28` gains the recipe.

## Non-Goals

- No live editing of a running world's settings, ever (ADR-0039).
- No kernel change; no protocol version change.
- No accounts: the two token roles remain.
- No production deployment by this work; the installer and Caddy are
  the owner's.
- `apps/observer` and `apps/observer-voxel-spike` are not modified.

## Acceptance Criteria

- [ ] **S1 Many worlds, one process.** Create three worlds with
      different presets and settings over REST; each ticks on its own
      thread; `GET /api/worlds` lists all three with distinct config
      hashes; pausing one does not pause the others (their ticks
      advance); stopping one ends its thread and leaves the others
      running; deleting a running world is refused (409) and a stopped
      one succeeds. Integration test.
- [ ] **S2 Settings at creation only.** `POST /api/worlds` with a bad
      field name or an out-of-range value returns 400 naming the field;
      the preview's hash equals the created world's hash; the created
      world's `differing_fields` from its preset are exactly the
      settings sent; no route changes a running world's config (asserted
      by hash before/after every control action). Integration test.
- [ ] **S3 Stream selects by path.** A socket to `/worlds/2` receives a
      Welcome with `world_id` 2 and keyframes from world 2's entities;
      `/` is world 1; `/worlds/99` gets Error 404; a world stopped
      mid-session sends Error 410 and closes. Every existing stream test
      unchanged. Integration test.
- [ ] **S4 Saves and branches per world.** A save on world 2 lists
      under world 2 only; branching it yields world 3 with
      `parent_world_id` 2, epoch 2 and the save's state checksum at its
      tick; verify still rebuilds in isolation. Integration test.
- [ ] **S5 Compatibility.** Every existing `sim-server` test passes
      unmodified; `/api/worlds/1/*` responses keep their fields; A5.1
      still equates the server's `--run-ticks` summary with the CLI
      fixture line.
- [ ] **C1 Title to live in three actions.** From a cold load with a
      saved profile: title -> New World -> Create -> live view showing
      the world, with keyboard alone and with pointer alone. Playwright
      against a real server.
- [ ] **C2 The builder is the schema.** Every field the server's schema
      returns is editable; a change updates the previewed config hash;
      an invalid value shows the server's message beside the field and
      disables Create; the differing-settings summary matches what is
      sent. Playwright.
- [ ] **C3 The worlds screen is live and safe.** Cards reflect status
      within two seconds of a control action; every admin action needs
      a confirmation naming the world; observer-role profiles see no
      admin controls. Playwright.
- [ ] **C4 Accessibility and reduced motion.** Live status region,
      focus order through every screen, no colour-only status,
      background animation off under `prefers-reduced-motion`.
      Playwright with the media feature emulated.
- [ ] **D1 Developer instance.** The script starts both, prints the
      URLs and the tokens once, and a second run reuses the data
      directory; documented in `docs/28`.

## Test Plan

Server integration tests in `crates/sim-server/tests/multiworld.rs`
using the existing spawn harness; a unit test in `sim-experiment` for
`get_field` round-tripping every registry field through `set_field`.
Console: Playwright specs in `apps/console/tests` run by
`scripts/run-console-e2e.sh` against a real server on test ports (the
Observer's pattern). Standing rule 1 applies: at least one mutant per
new server rule (a control that leaks across worlds; a path that
selects the wrong world; a settings change applied after creation).

## Documentation Updates

`docs/10` (the console's screens), `docs/11` (the routes above and the
path selection), `docs/28` (the developer instance), `docs/19` and
`planning/backlog.md` rows, ADR-0039 as built.

## Risks

| Risk | Mitigation |
|---|---|
| Sandbox worlds slow the production world | Bounded count; each world reports its tick cost; the console shows it; the operator stops worlds |
| A control leaks across worlds | Every control is keyed by world id and tested with a mutant that drops the key |
| The console's settings drift from the campaign vocabulary | The builder is generated from the server's schema, which is the campaign field registry |
| Secrets in the browser | Tokens stay in `localStorage` on the private LAN as the Observer already does; never in URLs or logs |

## Rollback

The server change is additive behind the same routes for world 1; the
console is a new directory; the developer instance is a script.
