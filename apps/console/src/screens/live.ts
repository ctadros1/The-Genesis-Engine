// The Live screen: the Observer's canvas, HUD, inspector, chart, overlay and
// pause/resume/speed controls ported onto the screen stack, plus Back and a
// world switcher. Connection, subscription, input, and rendering are the
// same protocol as apps/observer/src/main.ts (protocol.ts and render.ts are
// copied unchanged into this app) adapted to the multi-world routes and the
// active profile's tokens instead of pasted-in query params.
//
// The browser never owns world truth here either: every mutation goes
// through ctx.api's authenticated control calls, and all live state arrives
// from the WebSocket stream.

import { Application } from "pixi.js";
import type { OrganismDetail, WorldSummary } from "../api";
import type { Profile } from "../profiles";
import {
  LAYER_METRICS,
  LAYER_ORGANISMS,
  LAYER_TERRAIN,
  decodeFrame,
  encodeAck,
  encodeHello,
  encodeSubscribe,
  type MetricsSample,
  type ServerFrame,
} from "../protocol";
import { FP_PER_METER, WorldRenderer } from "../render";
import type { AppContext, Screen } from "../screens";
import { button, el, field } from "../ui/dom";
import "./live.css";
import { savesScreen } from "./saves";

const POPULATION_HISTORY_LIMIT = 130;
const META_POLL_MS = 2_000;
const RECONNECT_INITIAL_MS = 500;
const RECONNECT_MAX_MS = 5_000;

/// The token that gives the fullest access the active profile has: admin
/// when present, else observer. Mirrors ApiClient's private authToken()
/// (not exported), needed here because Hello is a raw WS frame, not a REST
/// call the client wraps. Exported so worlds.ts's per-card metrics sockets
/// (also raw WS frames) don't duplicate it.
export function bestToken(profile: Profile | null): string {
  if (!profile) return "";
  return profile.adminToken && profile.adminToken.length > 0 ? profile.adminToken : profile.observerToken;
}

function isAdminProfile(profile: Profile | null): boolean {
  return !!profile?.adminToken && profile.adminToken.length > 0;
}

/// Exported alongside bestToken for the same reason.
export function activeProfile(ctx: AppContext): Profile | null {
  return ctx.session.profile ?? ctx.profiles.active();
}

export function liveScreen(worldId: number, ctx: AppContext): Screen {
  let root: HTMLElement | null = null;
  let unmounted = false;

  // --- connection state --------------------------------------------------
  let socket: WebSocket | null = null;
  let intentionalClose = false;
  let reconnectDelayMs = RECONNECT_INITIAL_MS;
  let reconnectTimer: number | null = null;
  let reconnectAttempts = 0;

  // --- render state --------------------------------------------------------
  let app: Application | null = null;
  let renderer: WorldRenderer | null = null;
  let resizeHandler: (() => void) | null = null;

  // --- simulation-facing state ---------------------------------------------
  let tick = 0n;
  let population = 0;
  let paused = false;
  let selectedId: string | null = null;
  let selectedSensorRangeM: number | null = null;
  let populationHistory: number[] = [];

  // --- world meta polling ----------------------------------------------------
  let metaPollTimer: number | null = null;
  let knownWorldIds: number[] = [];

  const profile = activeProfile(ctx);
  const admin = isAdminProfile(profile);

  // --- DOM (populated in mount) ---------------------------------------------
  let connectionStatusEl: HTMLElement;
  let worldNameEl: HTMLElement;
  let statusBadgeEl: HTMLElement;
  let worldSwitcher: HTMLSelectElement;
  let stageWrap: HTMLElement;
  let viewportEl: HTMLElement;
  let inspectorEl: HTMLElement;
  let inspectIdEl: HTMLElement;
  let inspectBodyEl: HTMLElement;
  let chartCanvas: HTMLCanvasElement;
  let chartTextEl: HTMLElement;
  let overlayButton: HTMLButtonElement;
  let pauseButton: HTMLButtonElement;
  let resumeButton: HTMLButtonElement;
  let speedSelect: HTMLSelectElement;
  let errorPanel: HTMLElement;
  let errorMessageEl: HTMLElement;

  function setConnectionStatus(text: string): void {
    connectionStatusEl.textContent = text;
  }

  function updateHud(): void {
    const pausedSuffix = paused ? " | PAUSED" : "";
    const selectedSuffix = selectedId ? ` | selected ${selectedId}` : "";
    setConnectionStatus(`Connected | tick ${tick} | ${population} organisms${pausedSuffix}${selectedSuffix}`);
  }

  function showFatalError(code: number, message: string): void {
    stageWrap.hidden = true;
    errorMessageEl.textContent = `World ${worldId}: server error ${code} — ${message}`;
    errorPanel.hidden = false;
    ctx.announce(`World ${worldId} unavailable: ${message}`);
  }

  // --- connection ------------------------------------------------------------

  function connect(): void {
    if (unmounted || !profile) return;
    intentionalClose = false;
    setConnectionStatus("Connecting...");
    const ws = new WebSocket(ctx.api.wsUrl(worldId));
    ws.binaryType = "arraybuffer";
    socket = ws;

    ws.onopen = () => {
      ws.send(encodeHello(bestToken(profile)));
    };
    ws.onmessage = (event: MessageEvent<ArrayBuffer>) => {
      if (unmounted) return;
      const frame = decodeFrame(event.data);
      if (frame) handleFrame(frame, ws);
    };
    ws.onclose = () => {
      if (unmounted || intentionalClose) return;
      reconnectAttempts += 1;
      setConnectionStatus(`Reconnecting (attempt ${reconnectAttempts})...`);
      const delay = reconnectDelayMs;
      reconnectDelayMs = Math.min(reconnectDelayMs * 2, RECONNECT_MAX_MS);
      reconnectTimer = window.setTimeout(() => connect(), delay);
    };
    ws.onerror = () => ws.close();
  }

  function subscribeAll(ws: WebSocket): void {
    const worldFp = 1_048_576; // server clamps to actual extent
    ws.send(encodeSubscribe(0, 0, worldFp - 1, worldFp - 1, LAYER_TERRAIN | LAYER_ORGANISMS | LAYER_METRICS, 20));
  }

  function handleFrame(frame: ServerFrame, ws: WebSocket): void {
    switch (frame.kind) {
      case "welcome": {
        if (!renderer) {
          void initialiseStage({
            cellsX: frame.cellsX,
            cellsY: frame.cellsY,
            cellSizeM: frame.cellSizeM,
          }).then(() => subscribeAll(ws));
        } else {
          // Reconnect to the same world: resubscribe for a fresh keyframe.
          subscribeAll(ws);
        }
        break;
      }
      case "subscribed":
        reconnectDelayMs = RECONNECT_INITIAL_MS;
        reconnectAttempts = 0;
        setConnectionStatus("Connected");
        ctx.announce(`Connected to world ${worldId}`);
        updateHud();
        break;
      case "keyframe": {
        if (!renderer) break;
        tick = frame.tick;
        if (frame.tiles) renderer.applyTiles(frame.tiles);
        renderer.replaceEntities(frame.entities);
        ws.send(encodeAck(frame.sequence));
        updateHud();
        break;
      }
      case "delta": {
        if (!renderer) break;
        tick = frame.tick;
        for (const id of frame.removed) renderer.remove(id);
        for (const record of frame.upserts) renderer.upsert(record);
        if (frame.tiles) renderer.applyTiles(frame.tiles);
        ws.send(encodeAck(frame.sequence));
        updateHud();
        break;
      }
      case "metrics":
        onMetrics(frame);
        break;
      case "error":
        setConnectionStatus(`Server error ${frame.code}: ${frame.message}`);
        if (frame.code === 404 || frame.code === 410) {
          intentionalClose = true;
          showFatalError(frame.code, frame.message);
        } else if (frame.code === 401) {
          intentionalClose = true;
          ctx.announce(`Authentication rejected for world ${worldId}`);
        }
        break;
      default:
        break;
    }
  }

  // --- stage / input -----------------------------------------------------------

  async function initialiseStage(world: { cellsX: number; cellsY: number; cellSizeM: number }): Promise<void> {
    if (unmounted) return;
    stageWrap.hidden = false;
    const pixi = new Application();
    await pixi.init({
      width: viewportEl.clientWidth,
      height: viewportEl.clientHeight,
      preference: "webgl",
      background: 0x10182a,
      resolution: Math.min(window.devicePixelRatio, 2),
      autoDensity: true,
    });
    if (unmounted) {
      pixi.destroy(true, { children: true, texture: true });
      return;
    }
    app = pixi;
    viewportEl.replaceChildren(pixi.canvas);
    const worldRenderer = new WorldRenderer(pixi, world);
    renderer = worldRenderer;
    installInput(pixi, worldRenderer);
    resizeHandler = () => {
      app?.renderer.resize(viewportEl.clientWidth, viewportEl.clientHeight);
    };
    window.addEventListener("resize", resizeHandler);
  }

  /// pixi's camera container is positioned in the canvas's own pixel space
  /// (origin at the canvas's top-left), but pointer/wheel events report
  /// clientX/Y in page space. The two only coincide when the canvas sits at
  /// the page origin (true of the Observer's fullscreen layout, not true
  /// here where the canvas sits inside a bordered, padded viewport) — so
  /// every conversion from a DOM event to world space must subtract the
  /// canvas's own offset first, or zoom pivots and picks drift by that
  /// offset (compounding on repeated zooms).
  function canvasPoint(canvas: HTMLCanvasElement, clientX: number, clientY: number): { x: number; y: number } {
    const rect = canvas.getBoundingClientRect();
    return { x: clientX - rect.left, y: clientY - rect.top };
  }

  function installInput(application: Application, worldRenderer: WorldRenderer): void {
    const canvas = application.canvas;
    const pointers = new Map<number, { x: number; y: number }>();
    let pinchDistance = 0;
    let dragged = false;

    canvas.addEventListener("pointerdown", (event) => {
      pointers.set(event.pointerId, { x: event.clientX, y: event.clientY });
      dragged = false;
      canvas.setPointerCapture(event.pointerId);
    });
    canvas.addEventListener("pointermove", (event) => {
      const previous = pointers.get(event.pointerId);
      if (!previous) return;
      if (pointers.size === 1) {
        // A same-frame delta between two clientX/Y readings is offset-
        // invariant (the canvas's own position cancels out), so panning
        // needs no canvasPoint() conversion.
        const dx = event.clientX - previous.x;
        const dy = event.clientY - previous.y;
        if (Math.abs(dx) + Math.abs(dy) > 2) dragged = true;
        worldRenderer.camera.x += dx;
        worldRenderer.camera.y += dy;
      } else if (pointers.size === 2) {
        const entries = [...pointers.values()];
        const other = entries.find((entry) => entry !== previous);
        if (other) {
          const distance = Math.hypot(event.clientX - other.x, event.clientY - other.y);
          if (pinchDistance > 0) {
            const pivot = canvasPoint(canvas, event.clientX, event.clientY);
            applyZoom(worldRenderer, distance / pinchDistance, pivot.x, pivot.y);
          }
          pinchDistance = distance;
        }
        dragged = true;
      }
      pointers.set(event.pointerId, { x: event.clientX, y: event.clientY });
    });
    canvas.addEventListener("pointerup", (event) => {
      pointers.delete(event.pointerId);
      pinchDistance = 0;
      if (!dragged) {
        const point = canvasPoint(canvas, event.clientX, event.clientY);
        void handleSelect(point.x, point.y, worldRenderer);
      }
    });
    canvas.addEventListener("pointercancel", (event) => {
      pointers.delete(event.pointerId);
      pinchDistance = 0;
    });
    canvas.addEventListener(
      "wheel",
      (event) => {
        event.preventDefault();
        const pivot = canvasPoint(canvas, event.clientX, event.clientY);
        applyZoom(worldRenderer, event.deltaY < 0 ? 1.15 : 1 / 1.15, pivot.x, pivot.y);
      },
      { passive: false },
    );
  }

  function applyZoom(worldRenderer: WorldRenderer, factor: number, pivotX: number, pivotY: number): void {
    const camera = worldRenderer.camera;
    const oldScale = camera.scale.x;
    const newScale = Math.min(Math.max(oldScale * factor, worldRenderer.baseScale * 0.5), worldRenderer.baseScale * 60);
    const worldX = (pivotX - camera.x) / oldScale;
    const worldY = (pivotY - camera.y) / oldScale;
    camera.scale.set(newScale);
    camera.x = pivotX - worldX * newScale;
    camera.y = pivotY - worldY * newScale;
  }

  async function handleSelect(canvasX: number, canvasY: number, worldRenderer: WorldRenderer): Promise<void> {
    const camera = worldRenderer.camera;
    const worldX = (canvasX - camera.x) / camera.scale.x;
    const worldY = (canvasY - camera.y) / camera.scale.y;
    const pickRadius = Math.max(2, 8 / camera.scale.x);
    const id = worldRenderer.pick(worldX, worldY, pickRadius);
    worldRenderer.select(id);
    selectedId = id === null ? null : id.toString();
    if (id === null) {
      inspectorEl.hidden = true;
      selectedSensorRangeM = null;
      worldRenderer.redrawOverlay(null);
      updateHud();
      return;
    }
    const result = await ctx.api.organism(worldId, id.toString());
    if (unmounted) return;
    if (!result.ok) {
      inspectorEl.hidden = true;
      ctx.announce(`Organism ${id} unavailable`);
      updateHud();
      return;
    }
    renderInspector(result.value);
    updateHud();
  }

  function renderInspector(detail: OrganismDetail): void {
    inspectIdEl.textContent = String(detail.id);
    const rows: [string, string][] = [
      ["Energy", `${((detail.energy_milli as number) / 1000).toFixed(2)} EU`],
      ["Age", `${detail.age_ticks} ticks`],
      [
        "Position",
        `${((detail.x_fp as number) / FP_PER_METER).toFixed(1)}, ${((detail.y_fp as number) / FP_PER_METER).toFixed(1)} m`,
      ],
    ];
    const phenotype = detail.phenotype as Record<string, number> | undefined;
    if (phenotype) {
      rows.push(
        ["Parents", (detail.parents as number[]).join(", ") || "founders"],
        ["Generation", String(detail.ancestry_depth)],
        ["Offspring", String(detail.child_count)],
        ["Max speed", `${(phenotype.max_speed_milli ?? 0) / 1000} m/s`],
        ["Sensor range", `${(phenotype.sensor_range_milli ?? 0) / 1000} m`],
        ["Genome", String(detail.genome_hash)],
      );
      selectedSensorRangeM = (phenotype.sensor_range_milli ?? 0) / 1000;
    } else {
      selectedSensorRangeM = null;
    }
    inspectBodyEl.replaceChildren(
      ...rows.flatMap(([term, value]) => {
        const dt = el("dt", {}, [term]);
        const dd = el("dd", {}, [value]);
        return [dt, dd];
      }),
    );
    inspectorEl.hidden = false;
    ctx.announce(`Organism ${detail.id} selected`);
    renderer?.redrawOverlay(selectedSensorRangeM);
  }

  // --- HUD / chart -------------------------------------------------------------

  function onMetrics(sample: MetricsSample): void {
    population = sample.population;
    populationHistory.push(sample.population);
    if (populationHistory.length > POPULATION_HISTORY_LIMIT) populationHistory.shift();
    const context = chartCanvas.getContext("2d");
    if (context) {
      context.clearRect(0, 0, chartCanvas.width, chartCanvas.height);
      if (!ctx.reducedMotion()) {
        context.strokeStyle = "#59c3c3";
        context.lineWidth = 2;
        const max = Math.max(...populationHistory, 1);
        context.beginPath();
        populationHistory.forEach((value, index) => {
          const x = (index / Math.max(populationHistory.length - 1, 1)) * chartCanvas.width;
          const y = chartCanvas.height - (value / max) * (chartCanvas.height - 6) - 3;
          if (index === 0) context.moveTo(x, y);
          else context.lineTo(x, y);
        });
        context.stroke();
      } else {
        // Reduced motion: draw a static final bar instead of an animated line.
        const max = Math.max(...populationHistory, 1);
        const barHeight = (sample.population / max) * (chartCanvas.height - 6);
        context.fillStyle = "#59c3c3";
        context.fillRect(0, chartCanvas.height - barHeight, chartCanvas.width, barHeight);
      }
    }
    chartTextEl.textContent =
      `Population ${sample.population}, births ${sample.birthsTotal}, ` +
      `deaths ${sample.deathsStarvation + sample.deathsOldAge}, generation depth ${sample.maxAncestryDepth}`;
    updateHud();
  }

  // --- controls (admin only) ----------------------------------------------------

  async function runControl(action: "pause" | "resume" | "speed", multiplier?: number): Promise<void> {
    const result = await ctx.api.control(worldId, action, multiplier);
    if (unmounted) return;
    if (result.ok) {
      paused = result.value.paused;
      updateHud();
      ctx.announce(action === "speed" ? `Speed set to ${multiplier}x` : `World ${paused ? "paused" : "resumed"}`);
    } else {
      setConnectionStatus(`Control rejected (${result.status})`);
      ctx.announce(`Control rejected: ${result.message}`);
    }
  }

  // --- world meta polling (name/status badge + switcher) -------------------------

  function statusBadgeClass(status: WorldSummary["status"]): string {
    return `badge badge-${status}`;
  }

  /// summary.status/.name are typed as required by the multi-world schema,
  /// but a pre-multi-world server (this screen's transitional test target)
  /// omits both; fall back rather than throw out of a poll callback.
  function applyWorldSummary(summary: WorldSummary): void {
    worldNameEl.textContent = summary.name ?? `World ${worldId}`;
    const status: WorldSummary["status"] = summary.status ?? (summary.paused ? "paused" : "running");
    statusBadgeEl.textContent = status.toUpperCase();
    statusBadgeEl.className = statusBadgeClass(status);
    paused = summary.paused;
  }

  function rebuildSwitcher(worlds: WorldSummary[]): void {
    const ids = worlds.map((world) => world.world_id);
    const changed =
      ids.length !== knownWorldIds.length || ids.some((id, index) => id !== knownWorldIds[index]);
    if (!changed && worldSwitcher.options.length > 0) return;
    knownWorldIds = ids;
    const options = worlds.map((world) =>
      el("option", { value: String(world.world_id), selected: world.world_id === worldId }, [
        `${world.name ?? `World ${world.world_id}`} (#${world.world_id})`,
      ]),
    );
    if (!worlds.some((world) => world.world_id === worldId)) {
      options.unshift(el("option", { value: String(worldId), selected: true }, [`World ${worldId}`]));
    }
    worldSwitcher.replaceChildren(...options);
    worldSwitcher.value = String(worldId);
  }

  async function pollMeta(): Promise<void> {
    const [worldResult, listResult] = await Promise.all([ctx.api.getWorld(worldId), ctx.api.listWorlds()]);
    if (unmounted) return;
    if (worldResult.ok) applyWorldSummary(worldResult.value);
    if (listResult.ok) rebuildSwitcher(listResult.value);
  }

  // --- mount / unmount -----------------------------------------------------------

  function buildDom(): HTMLElement {
    connectionStatusEl = el("p", { class: "live-connection-status", role: "status", "aria-live": "polite" }, [
      "Disconnected",
    ]);
    worldNameEl = el("span", { class: "live-world-name" }, [`World ${worldId}`]);
    statusBadgeEl = el("span", { class: "badge" }, ["UNKNOWN"]);
    worldSwitcher = el("select", { "aria-label": "Switch world" }, [
      el("option", { value: String(worldId), selected: true }, [`World ${worldId}`]),
    ]);
    worldSwitcher.addEventListener("change", () => {
      const next = Number(worldSwitcher.value);
      if (Number.isFinite(next) && next !== worldId) {
        ctx.rememberWorld(next);
        void ctx.stack.replace(liveScreen(next, ctx));
      }
    });

    const savesButton = button("Saves", () => {
      ctx.rememberWorld(worldId);
      void ctx.stack.push(savesScreen(ctx));
    });

    const topbar = el("div", { class: "screen-header live-topbar" }, [
      el("div", { class: "live-topbar-left" }, [
        button("Back", () => void ctx.stack.pop()),
        el("h1", {}, [worldNameEl]),
        statusBadgeEl,
      ]),
      el("div", { class: "live-topbar-right" }, [field("Switch world", worldSwitcher), savesButton]),
    ]);

    viewportEl = el("div", { class: "live-viewport" });

    pauseButton = button(
      "Pause",
      () => void runControl("pause"),
      { disabled: !admin, title: admin ? undefined : "Requires an admin token on this profile" },
    );
    resumeButton = button(
      "Resume",
      () => void runControl("resume"),
      { disabled: !admin, title: admin ? undefined : "Requires an admin token on this profile" },
    );
    speedSelect = el("select", { disabled: !admin, title: admin ? undefined : "Requires an admin token on this profile" }, [
      el("option", { value: "1" }, ["1x"]),
      el("option", { value: "4" }, ["4x"]),
      el("option", { value: "16" }, ["16x"]),
    ]);
    speedSelect.addEventListener("change", () => void runControl("speed", Number(speedSelect.value)));

    overlayButton = button("Scientific overlay", () => {
      if (!renderer) return;
      renderer.overlayEnabled = !renderer.overlayEnabled;
      overlayButton.setAttribute("aria-pressed", String(renderer.overlayEnabled));
      ctx.announce(`Scientific overlay ${renderer.overlayEnabled ? "on" : "off"}`);
      renderer.redrawOverlay(selectedSensorRangeM);
    });
    overlayButton.setAttribute("aria-pressed", "false");

    const controlsGroup: (Node | string)[] = admin
      ? [pauseButton, resumeButton, field("Speed", speedSelect), overlayButton]
      : [overlayButton];
    const controls = el("div", { class: "live-controls", "aria-label": "Simulation controls" }, controlsGroup);

    chartCanvas = el("canvas", { class: "live-chart-canvas", width: "260", height: "80" });
    chartTextEl = el("p", { class: "visually-hidden", "aria-live": "off" });
    const chart = el("div", { class: "live-chart", "aria-label": "Population chart" }, [chartCanvas, chartTextEl]);

    const hud = el("div", { class: "live-hud" }, [controls, chart]);

    inspectIdEl = el("span", {});
    inspectBodyEl = el("dl", { class: "live-inspect-body" });
    const inspectClose = button("Close", () => {
      inspectorEl.hidden = true;
      renderer?.select(null);
      selectedId = null;
      updateHud();
    });
    inspectorEl = el("aside", { class: "live-inspector", "aria-label": "Organism inspector", hidden: true }, [
      el("h2", {}, ["Organism ", inspectIdEl]),
      inspectClose,
      inspectBodyEl,
    ]);

    stageWrap = el("section", { class: "live-stage-wrap", "aria-label": "World view", hidden: true }, [
      viewportEl,
      hud,
      inspectorEl,
    ]);

    errorMessageEl = el("p", {});
    errorPanel = el("div", { class: "live-error-panel", hidden: true }, [
      errorMessageEl,
      button("Back", () => void ctx.stack.pop()),
    ]);

    return el("div", { class: "screen live-screen" }, [topbar, connectionStatusEl, stageWrap, errorPanel]);
  }

  /// Left/Right nav order, per the plan: Back, world switcher, Saves (the
  /// top bar), then Pause, Resume, Speed, Overlay (the HUD) — exactly the
  /// DOM order buildDom() authors them in, so one selector in document
  /// order gives the right sequence without hand-listing elements.
  /// Disabled controls (Pause/Resume/Speed on an observer profile) are
  /// excluded by :not([disabled]) so focus never lands somewhere a screen
  /// reader or a click could never activate.
  function collectNavTargets(): HTMLElement[] {
    if (!root) return [];
    const selector =
      ".live-topbar button:not([disabled]), .live-topbar select:not([disabled]), " +
      ".live-controls button:not([disabled]), .live-controls select:not([disabled])";
    return Array.from(root.querySelectorAll<HTMLElement>(selector));
  }

  return {
    id: "live",
    title: `World ${worldId}`,

    mount(mountRoot: HTMLElement): void {
      root = mountRoot;
      unmounted = false;
      root.append(buildDom());

      if (!profile) {
        showFatalError(0, "No server profile is configured.");
        return;
      }

      void pollMeta();
      metaPollTimer = window.setInterval(() => void pollMeta(), META_POLL_MS);
      connect();
    },

    unmount(): void {
      unmounted = true;
      intentionalClose = true;
      if (reconnectTimer !== null) {
        window.clearTimeout(reconnectTimer);
        reconnectTimer = null;
      }
      if (metaPollTimer !== null) {
        window.clearInterval(metaPollTimer);
        metaPollTimer = null;
      }
      if (resizeHandler) {
        window.removeEventListener("resize", resizeHandler);
        resizeHandler = null;
      }
      socket?.close();
      socket = null;
      if (app) {
        app.destroy(true, { children: true, texture: true });
        app = null;
      }
      renderer = null;
      root = null;
    },

    onKey(event: KeyboardEvent): boolean {
      if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return false;
      const targets = collectNavTargets();
      if (targets.length === 0) return false;
      const currentIndex = targets.indexOf(document.activeElement as HTMLElement);
      if (currentIndex === -1) return false;
      const delta = event.key === "ArrowRight" ? 1 : -1;
      const nextIndex = (currentIndex + delta + targets.length) % targets.length;
      targets[nextIndex]!.focus();
      return true;
    },
  };
}
