// The title screen's full-bleed background: a low-rate live preview of the
// last-viewed world when the active profile is connected and one exists,
// otherwise a cheap procedural pixel drift. Purely decorative — it never
// mutates the world, never blocks the menu, and cleans up completely on
// unmount (closes its socket, destroys its PixiJS Application, clears its
// timers) so navigating away and back never leaks a connection or a timer.

import { Application } from "pixi.js";
import type { AppContext } from "../../screens";
import type { Profile } from "../../profiles";
import {
  LAYER_ORGANISMS,
  LAYER_TERRAIN,
  decodeFrame,
  encodeAck,
  encodeHello,
  encodeSubscribe,
  type ServerFrame,
} from "../../protocol";
import { WorldRenderer, type WorldInfo } from "../../render";

// Server clamps the subscribe rectangle to the world's actual extent, so a
// generous fixed-point bound (mirrors apps/observer's subscribeAll) always
// covers the whole map regardless of its real size.
const WORLD_FP_EXTENT = 1_048_576;
const BACKGROUND_RATE_HZ = 4;

const DRIFT_COLS = 80;
const DRIFT_ROWS = 45;
const DRIFT_STEP_MS = 160;

function clamp01(value: number): number {
  return Math.min(1, Math.max(0, value));
}

export class TitleBackground {
  private ctx: AppContext;
  private container: HTMLElement | null = null;
  private disposed = false;

  // Live-world mode.
  private socket: WebSocket | null = null;
  private intentionalClose = false;
  private app: Application | null = null;
  private renderer: WorldRenderer | null = null;
  private resizeHandler: (() => void) | null = null;
  private panRaf: number | null = null;
  private panStartMs = 0;

  // Drift mode.
  private driftCanvas: HTMLCanvasElement | null = null;
  private driftCtx2d: CanvasRenderingContext2D | null = null;
  private driftCells: Float32Array = new Float32Array(DRIFT_COLS * DRIFT_ROWS);
  private driftTimer: number | null = null;

  constructor(ctx: AppContext) {
    this.ctx = ctx;
  }

  mount(container: HTMLElement): void {
    this.container = container;
    this.disposed = false;
    const profile = this.ctx.session.profile;
    const worldId = this.ctx.session.lastWorldId;
    const connected = this.ctx.session.role !== undefined;
    if (connected && profile && worldId !== undefined) {
      this.mountLiveWorld(profile, worldId);
    } else {
      this.mountDrift();
    }
  }

  unmount(): void {
    this.disposed = true;
    this.unmountLiveWorld();
    this.unmountDrift();
    this.container = null;
  }

  // --- live world preview --------------------------------------------------

  private mountLiveWorld(profile: Profile, worldId: number): void {
    this.intentionalClose = false;
    const socket = new WebSocket(this.ctx.api.wsUrl(worldId));
    socket.binaryType = "arraybuffer";
    this.socket = socket;

    socket.onopen = () => {
      socket.send(encodeHello(profile.observerToken));
    };
    socket.onmessage = (event: MessageEvent<ArrayBuffer>) => {
      if (this.disposed) return;
      const frame = decodeFrame(event.data);
      if (frame) this.handleFrame(frame, socket);
    };
    socket.onerror = () => socket.close();
    socket.onclose = () => {
      if (this.disposed || this.intentionalClose) return;
      // Decorative only: on any drop (auth failure, a stopped world, a
      // network hiccup) fall back to the drift pattern rather than
      // reconnecting forever behind the menu.
      this.unmountLiveWorld();
      this.mountDrift();
    };
  }

  private handleFrame(frame: ServerFrame, socket: WebSocket): void {
    if (this.disposed) return;
    switch (frame.kind) {
      case "welcome":
        if (!this.renderer) {
          void this.initialiseStage({
            cellsX: frame.cellsX,
            cellsY: frame.cellsY,
            cellSizeM: frame.cellSizeM,
          }).then(() => {
            if (!this.disposed) this.subscribe(socket);
          });
        } else {
          this.subscribe(socket);
        }
        break;
      case "keyframe":
        if (!this.renderer) break;
        if (frame.tiles) this.renderer.applyTiles(frame.tiles);
        this.renderer.replaceEntities(frame.entities);
        socket.send(encodeAck(frame.sequence));
        break;
      case "delta":
        if (!this.renderer) break;
        for (const id of frame.removed) this.renderer.remove(id);
        for (const record of frame.upserts) this.renderer.upsert(record);
        if (frame.tiles) this.renderer.applyTiles(frame.tiles);
        socket.send(encodeAck(frame.sequence));
        break;
      case "error":
        // Unknown (404) or stopped (410) world: stop trying, drift instead.
        this.intentionalClose = true;
        this.unmountLiveWorld();
        this.mountDrift();
        break;
      default:
        break;
    }
  }

  private subscribe(socket: WebSocket): void {
    socket.send(
      encodeSubscribe(
        0,
        0,
        WORLD_FP_EXTENT - 1,
        WORLD_FP_EXTENT - 1,
        LAYER_TERRAIN | LAYER_ORGANISMS,
        BACKGROUND_RATE_HZ,
      ),
    );
  }

  private async initialiseStage(world: WorldInfo): Promise<void> {
    if (this.disposed || !this.container) return;
    const app = new Application();
    await app.init({
      width: Math.max(this.container.clientWidth, 1),
      height: Math.max(this.container.clientHeight, 1),
      preference: "webgl",
      background: 0x0b0f14,
      resolution: Math.min(window.devicePixelRatio, 2),
      autoDensity: true,
    });
    if (this.disposed || !this.container) {
      app.destroy(true, true);
      return;
    }
    this.container.replaceChildren(app.canvas);
    this.app = app;
    this.renderer = new WorldRenderer(app, world);
    this.resizeHandler = () => {
      if (!this.container || !this.app) return;
      this.app.renderer.resize(this.container.clientWidth, this.container.clientHeight);
      this.refit();
    };
    window.addEventListener("resize", this.resizeHandler);
    if (!this.ctx.reducedMotion()) this.startPan();
  }

  private refit(): void {
    if (!this.app || !this.renderer) return;
    const worldMeters = this.renderer.world.cellsX * this.renderer.world.cellSizeM;
    if (worldMeters <= 0) return;
    const fit = Math.min(this.app.renderer.width / worldMeters, this.app.renderer.height / worldMeters);
    this.renderer.baseScale = fit;
    this.renderer.camera.scale.set(fit);
  }

  private startPan(): void {
    this.panStartMs = performance.now();
    const step = (now: number) => {
      if (this.disposed || !this.renderer) return;
      const elapsedS = (now - this.panStartMs) / 1000;
      // Small, slow drift so the menu stays legible over it.
      this.renderer.camera.x = Math.sin(elapsedS * 0.05) * 26;
      this.renderer.camera.y = Math.cos(elapsedS * 0.037) * 18;
      this.panRaf = requestAnimationFrame(step);
    };
    this.panRaf = requestAnimationFrame(step);
  }

  private unmountLiveWorld(): void {
    this.intentionalClose = true;
    if (this.panRaf !== null) {
      cancelAnimationFrame(this.panRaf);
      this.panRaf = null;
    }
    if (this.resizeHandler) {
      window.removeEventListener("resize", this.resizeHandler);
      this.resizeHandler = null;
    }
    if (this.socket) {
      this.socket.onopen = null;
      this.socket.onmessage = null;
      this.socket.onclose = null;
      this.socket.onerror = null;
      this.socket.close();
      this.socket = null;
    }
    if (this.app) {
      this.app.destroy(true, true);
      this.app = null;
    }
    this.renderer = null;
  }

  // --- procedural pixel drift ----------------------------------------------

  private mountDrift(): void {
    if (this.disposed || !this.container) return;
    const canvas = document.createElement("canvas");
    canvas.width = DRIFT_COLS;
    canvas.height = DRIFT_ROWS;
    canvas.className = "title-drift-canvas";
    canvas.setAttribute("aria-hidden", "true");
    const context = canvas.getContext("2d");
    if (!context) return;
    this.driftCanvas = canvas;
    this.driftCtx2d = context;
    this.container.replaceChildren(canvas);

    this.seedDrift();
    this.drawDrift();
    if (!this.ctx.reducedMotion()) {
      this.driftTimer = window.setInterval(() => {
        this.stepDrift();
        this.drawDrift();
      }, DRIFT_STEP_MS);
    }
  }

  private seedDrift(): void {
    for (let index = 0; index < this.driftCells.length; index += 1) {
      this.driftCells[index] = Math.random();
    }
  }

  private stepDrift(): void {
    const next = new Float32Array(DRIFT_COLS * DRIFT_ROWS);
    for (let y = 0; y < DRIFT_ROWS; y += 1) {
      for (let x = 0; x < DRIFT_COLS; x += 1) {
        let sum = 0;
        for (let dy = -1; dy <= 1; dy += 1) {
          for (let dx = -1; dx <= 1; dx += 1) {
            if (dx === 0 && dy === 0) continue;
            const nx = (x + dx + DRIFT_COLS) % DRIFT_COLS;
            const ny = (y + dy + DRIFT_ROWS) % DRIFT_ROWS;
            sum += this.driftCells[ny * DRIFT_COLS + nx] ?? 0;
          }
        }
        const average = sum / 8;
        const noise = (Math.random() - 0.5) * 0.05;
        next[y * DRIFT_COLS + x] = clamp01(average * 0.94 + noise + 0.03);
      }
    }
    this.driftCells = next;
  }

  private drawDrift(): void {
    if (!this.driftCtx2d) return;
    const image = this.driftCtx2d.createImageData(DRIFT_COLS, DRIFT_ROWS);
    for (let index = 0; index < this.driftCells.length; index += 1) {
      const value = this.driftCells[index] ?? 0;
      const offset = index * 4;
      if (value < 0.42) {
        // Water: matches WorldRenderer.applyTiles' water fill.
        image.data[offset] = 22;
        image.data[offset + 1] = 40 + value * 60;
        image.data[offset + 2] = 84 + value * 40;
      } else {
        // Land: darker with depleted "food", greener with abundant "food".
        const food = (value - 0.42) / 0.58;
        image.data[offset] = 46 + (110 - 46) * (1 - food);
        image.data[offset + 1] = 90 + (190 - 90) * food;
        image.data[offset + 2] = 46;
      }
      image.data[offset + 3] = 255;
    }
    this.driftCtx2d.putImageData(image, 0, 0);
  }

  private unmountDrift(): void {
    if (this.driftTimer !== null) {
      window.clearInterval(this.driftTimer);
      this.driftTimer = null;
    }
    this.driftCanvas?.remove();
    this.driftCanvas = null;
    this.driftCtx2d = null;
  }
}
