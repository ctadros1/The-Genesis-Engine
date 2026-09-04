// Worlds screen: one card per live world with a status badge, live metrics,
// a population sparkline drawn from client-side polling history, and admin
// actions (Pause/Resume, Save, Stop, Branch, Delete) behind confirmation
// dialogs, per the plan's Worlds screen (planning/console-and-multi-world-
// server.md, screen 3).
//
// Every field read from a WorldSummary is treated as possibly absent at
// runtime even though the type says otherwise: this screen and the server's
// multi-world routes were built concurrently, so a stale or partial payload
// must render safely rather than throw.

import type { ApiResult, WorldStatus, WorldSummary } from "../api";
import type { AppContext, Screen } from "../screens";
import { button, el } from "../ui/dom";
import { confirm } from "../ui/dialog";
import { builderScreen } from "./builder";
import { liveScreen } from "./live";
import { savesScreen } from "./saves";
import "./worlds.css";

const POLL_INTERVAL_MS = 2000;
const HISTORY_LENGTH = 60;

function numOr(value: unknown, fallback: number): number {
  return typeof value === "number" && Number.isFinite(value) ? value : fallback;
}

function strOr(value: unknown, fallback: string): string {
  return typeof value === "string" && value.length > 0 ? value : fallback;
}

function statusOf(world: WorldSummary): WorldStatus {
  if (world.status === "running" || world.status === "paused" || world.status === "stopped") {
    return world.status;
  }
  return world.paused ? "paused" : "running";
}

function statusLabel(status: WorldStatus): string {
  if (status === "running") return "Running";
  if (status === "paused") return "Paused";
  return "Stopped";
}

function nameOf(world: WorldSummary): string {
  return strOr(world.name, `World ${world.world_id}`);
}

function shortHash(hash: string): string {
  return hash.length > 12 ? `${hash.slice(0, 10)}…` : hash;
}

function formatTickCost(value: unknown): string {
  const n = numOr(value, Number.NaN);
  return Number.isNaN(n) ? "n/a" : `${Math.round(n)} µs`;
}

function accentColor(): string {
  const value = getComputedStyle(document.documentElement).getPropertyValue("--accent").trim();
  return value.length > 0 ? value : "#59c3c3";
}

function drawSparkline(canvas: HTMLCanvasElement, values: number[]): void {
  const context = canvas.getContext("2d");
  if (!context) return;
  const w = canvas.width;
  const h = canvas.height;
  context.clearRect(0, 0, w, h);
  if (values.length < 2) return;
  const min = Math.min(...values);
  const max = Math.max(...values);
  const range = max - min || 1;
  const stepX = w / (values.length - 1);
  context.strokeStyle = accentColor();
  context.lineWidth = 2;
  context.beginPath();
  values.forEach((value, index) => {
    const x = index * stepX;
    const y = h - ((value - min) / range) * (h - 4) - 2;
    if (index === 0) context.moveTo(x, y);
    else context.lineTo(x, y);
  });
  context.stroke();
}

function buildSparkline(history: number[]): HTMLElement {
  const canvas = el("canvas", {
    width: 180,
    height: 40,
    class: "sparkline",
    "aria-hidden": "true",
  }) as HTMLCanvasElement;
  drawSparkline(canvas, history);
  let text: string;
  if (history.length === 0) {
    text = "Population trend: not enough samples yet.";
  } else {
    const current = history[history.length - 1] ?? 0;
    const min = Math.min(...history);
    const max = Math.max(...history);
    text = `Population trend over the last ${history.length} sample${history.length === 1 ? "" : "s"}: ranged from ${min} to ${max}, currently ${current}.`;
  }
  return el("div", { class: "sparkline-wrap" }, [canvas, el("span", { class: "visually-hidden" }, [text])]);
}

function metaRow(label: string, value: string, title?: string): HTMLElement {
  return el("div", { class: "world-meta-row" }, [
    el("dt", {}, [label]),
    el("dd", { title }, [value]),
  ]);
}

export function worldsScreen(ctx: AppContext): Screen {
  let root: HTMLElement | null = null;
  let cardsContainer: HTMLElement | null = null;
  let statusBanner: HTMLElement | null = null;
  let pollTimer: number | null = null;

  let lastWorlds: WorldSummary[] = [];
  const history = new Map<number, number[]>();
  const cardErrors = new Map<number, string>();
  const busyIds = new Set<number>();

  function currentFocusRef(): { worldId: number; buttonLabel: string | null } | null {
    const active = document.activeElement;
    if (!(active instanceof HTMLElement) || !cardsContainer?.contains(active)) return null;
    const card = active.closest<HTMLElement>(".card");
    if (!card) return null;
    const idAttr = card.getAttribute("data-world-id");
    if (!idAttr) return null;
    const worldId = Number(idAttr);
    if (active instanceof HTMLButtonElement) return { worldId, buttonLabel: active.textContent };
    return { worldId, buttonLabel: null };
  }

  function restoreFocus(ref: { worldId: number; buttonLabel: string | null } | null): void {
    if (!ref || !cardsContainer) return;
    const card = cardsContainer.querySelector<HTMLElement>(`[data-world-id="${ref.worldId}"]`);
    if (!card) return;
    if (ref.buttonLabel !== null) {
      const target = Array.from(card.querySelectorAll<HTMLButtonElement>("button")).find(
        (candidate) => candidate.textContent === ref.buttonLabel && !candidate.disabled,
      );
      if (target) {
        target.focus();
        return;
      }
    }
    card.focus();
  }

  function emptyState(): HTMLElement {
    return el("div", { class: "worlds-empty" }, [
      el("p", {}, ["No worlds are hosted right now."]),
      button("New World", () => void ctx.stack.push(builderScreen(ctx)), { variant: "primary" }),
    ]);
  }

  async function performAction(
    world: WorldSummary,
    opts: {
      label: string;
      body: string;
      danger?: boolean;
      call: () => Promise<ApiResult<unknown>>;
      successText: string;
    },
  ): Promise<void> {
    const proceed = await confirm({
      title: `${opts.label} ${nameOf(world)}?`,
      body: opts.body,
      confirmLabel: opts.label,
      danger: opts.danger,
    });
    if (!proceed) return;
    busyIds.add(world.world_id);
    paintCards();
    const result = await opts.call();
    busyIds.delete(world.world_id);
    if (!result.ok) {
      cardErrors.set(world.world_id, result.message);
      ctx.announce(`${opts.label} failed for ${nameOf(world)}: ${result.message}`);
    } else {
      cardErrors.delete(world.world_id);
      ctx.announce(opts.successText);
    }
    await refresh();
  }

  function buildCard(world: WorldSummary): HTMLElement {
    const status = statusOf(world);
    const name = nameOf(world);
    const isAdmin = ctx.session.role === "admin";
    const isBusy = busyIds.has(world.world_id);
    const hist = history.get(world.world_id) ?? [];
    const hash = strOr(world.config_hash, "—");

    const head = el("div", { class: "world-card-head" }, [
      el("h3", {}, [name]),
      el("span", { class: `status-badge status-badge--${status}` }, [statusLabel(status)]),
    ]);

    const meta = el("dl", { class: "world-meta" }, [
      metaRow("Tick", String(numOr(world.tick, 0))),
      metaRow("Population", String(numOr(world.population, 0))),
      metaRow("Seed", strOr(world.seed, "—")),
      metaRow("Config hash", shortHash(hash), hash),
      metaRow("Tick cost", formatTickCost(world.tick_mean_us)),
    ]);

    const children: (Node | string)[] = [head, meta, buildSparkline(hist)];

    const errorMessage = cardErrors.get(world.world_id);
    if (errorMessage) {
      children.push(el("p", { class: "card-error", role: "alert" }, [errorMessage]));
    }

    const actions = el("div", { class: "card-actions" }, []);
    actions.append(
      button("View", () => {
        ctx.session.lastWorldId = world.world_id;
        void ctx.stack.push(liveScreen(world.world_id, ctx));
      }),
    );

    if (isAdmin) {
      if (status === "running" || status === "paused") {
        const isRunning = status === "running";
        const label = isRunning ? "Pause" : "Resume";
        actions.append(
          button(
            label,
            () => {
              void performAction(world, {
                label,
                body: `${label} "${name}" (#${world.world_id})?`,
                call: () => ctx.api.control(world.world_id, isRunning ? "pause" : "resume"),
                successText: `${name} ${isRunning ? "paused" : "resumed"}.`,
              });
            },
            { disabled: isBusy },
          ),
        );
      }

      actions.append(
        button(
          "Save",
          () => {
            void performAction(world, {
              label: "Save",
              body: `Create a save for "${name}" (#${world.world_id}) now?`,
              call: () => ctx.api.createSave(world.world_id, `quick-${Date.now()}`),
              successText: `Save created for ${name}.`,
            });
          },
          { disabled: isBusy },
        ),
      );

      if (status !== "stopped") {
        actions.append(
          button(
            "Stop",
            () => {
              void performAction(world, {
                label: "Stop",
                danger: true,
                body: `Stop "${name}" (#${world.world_id})? Its tick thread ends after a final checkpoint; it stays readable and saveable.`,
                call: () => ctx.api.control(world.world_id, "stop"),
                successText: `${name} stopped.`,
              });
            },
            { variant: "danger", disabled: isBusy },
          ),
        );
      }

      actions.append(
        button("Branch", () => {
          ctx.session.lastWorldId = world.world_id;
          void ctx.stack.push(savesScreen(ctx));
        }),
      );

      if (status === "stopped") {
        actions.append(
          button(
            "Delete",
            () => {
              void performAction(world, {
                label: "Delete",
                danger: true,
                body: `Permanently remove "${name}" (#${world.world_id}) from the registry? Its saves stay on disk.`,
                call: () => ctx.api.deleteWorld(world.world_id),
                successText: `${name} deleted.`,
              });
            },
            { variant: "danger", disabled: isBusy },
          ),
        );
      }
    }

    children.push(actions);

    return el(
      "div",
      {
        class: "card",
        tabindex: 0,
        role: "group",
        "aria-label": `${name}, ${statusLabel(status)}`,
        "data-world-id": String(world.world_id),
      },
      children,
    );
  }

  function paintCards(): void {
    if (!cardsContainer) return;
    const focusRef = currentFocusRef();
    cardsContainer.innerHTML = "";
    if (lastWorlds.length === 0) {
      cardsContainer.append(emptyState());
      return;
    }
    for (const world of lastWorlds) {
      cardsContainer.append(buildCard(world));
    }
    restoreFocus(focusRef);
  }

  function paintBanner(message: string | null): void {
    if (!statusBanner) return;
    if (message === null) {
      statusBanner.hidden = true;
      statusBanner.textContent = "";
    } else {
      statusBanner.hidden = false;
      statusBanner.textContent = message;
    }
  }

  async function refresh(): Promise<void> {
    if (!root) return;
    const result = await ctx.api.listWorlds();
    if (!result.ok) {
      paintBanner(`Could not refresh worlds: ${result.message}`);
      return;
    }
    paintBanner(null);
    lastWorlds = result.value;
    const seen = new Set<number>();
    for (const world of lastWorlds) {
      seen.add(world.world_id);
      const pop = numOr(world.population, 0);
      const samples = history.get(world.world_id) ?? [];
      samples.push(pop);
      while (samples.length > HISTORY_LENGTH) samples.shift();
      history.set(world.world_id, samples);
    }
    for (const key of Array.from(history.keys())) {
      if (!seen.has(key)) {
        history.delete(key);
        cardErrors.delete(key);
      }
    }
    paintCards();
  }

  return {
    id: "worlds",
    title: "Worlds",

    mount(mountRoot: HTMLElement): void {
      root = mountRoot;
      const header = el("div", { class: "screen-header" }, [
        el("h1", {}, ["Worlds"]),
        button("Back", () => void ctx.stack.pop()),
      ]);
      statusBanner = el("p", { class: "worlds-banner", role: "alert" }, []);
      statusBanner.hidden = true;
      cardsContainer = el("div", { class: "card-grid" }, [el("p", {}, ["Loading worlds…"])]);
      root.append(el("div", { class: "screen" }, [header, statusBanner, cardsContainer]));

      void refresh();
      pollTimer = window.setInterval(() => void refresh(), POLL_INTERVAL_MS);
    },

    unmount(): void {
      if (pollTimer !== null) {
        window.clearInterval(pollTimer);
        pollTimer = null;
      }
      root = null;
      cardsContainer = null;
      statusBanner = null;
    },

    onKey(event: KeyboardEvent): boolean {
      if (!cardsContainer) return false;
      const active = document.activeElement;
      if (!(active instanceof HTMLElement)) return false;

      if (
        event.key === "ArrowRight" ||
        event.key === "ArrowLeft" ||
        event.key === "ArrowDown" ||
        event.key === "ArrowUp"
      ) {
        const cards = Array.from(cardsContainer.querySelectorAll<HTMLElement>(".card"));
        if (cards.length === 0) return false;
        const ownCard = active.closest<HTMLElement>(".card");
        if (!ownCard || !cardsContainer.contains(ownCard)) return false;
        const currentIndex = cards.indexOf(ownCard);
        if (currentIndex === -1) return false;
        const delta = event.key === "ArrowRight" || event.key === "ArrowDown" ? 1 : -1;
        const nextIndex = (currentIndex + delta + cards.length) % cards.length;
        cards[nextIndex]?.focus();
        return true;
      }

      if (event.key === "Enter" && active.classList.contains("card")) {
        const viewButton = active.querySelector<HTMLButtonElement>(".card-actions button");
        viewButton?.click();
        return true;
      }

      return false;
    },
  };
}
