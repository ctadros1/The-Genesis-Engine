// PixiJS rendering: terrain texture layer, pooled organism sprites,
// pan/zoom camera, selection ring, and scientific overlay.
//
// The observer renders and interpolates only; it never simulates. Render
// cadence is decoupled from server tick cadence.

import {
  Application,
  Container,
  Graphics,
  Rectangle,
  Sprite,
  Texture,
} from "pixi.js";
import type { EntityRecord, TileBlock } from "./protocol";

export const FP_PER_METER = 1024;

export interface WorldInfo {
  cellsX: number;
  cellsY: number;
  cellSizeM: number;
}

export class WorldRenderer {
  app: Application;
  camera: Container;
  terrainSprite: Sprite;
  organismLayer: Container;
  overlayLayer: Graphics;
  selectionRing: Graphics;
  terrainCanvas: HTMLCanvasElement;
  terrainContext: CanvasRenderingContext2D;
  terrainTexture: Texture;
  sprites = new Map<bigint, Sprite>();
  records = new Map<bigint, EntityRecord>();
  world: WorldInfo;
  selectedId: bigint | null = null;
  overlayEnabled = false;
  reducedMotion: boolean;
  /// Pixels per meter at zoom 1.
  baseScale = 1;

  constructor(app: Application, world: WorldInfo) {
    this.app = app;
    this.world = world;
    this.reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;

    this.camera = new Container();
    app.stage.addChild(this.camera);

    this.terrainCanvas = document.createElement("canvas");
    this.terrainCanvas.width = world.cellsX;
    this.terrainCanvas.height = world.cellsY;
    const context = this.terrainCanvas.getContext("2d");
    if (!context) throw new Error("no 2d context for terrain");
    this.terrainContext = context;
    this.terrainTexture = Texture.from(this.terrainCanvas);
    this.terrainTexture.source.scaleMode = "nearest";
    this.terrainSprite = new Sprite(this.terrainTexture);
    const worldMeters = world.cellsX * world.cellSizeM;
    this.terrainSprite.width = worldMeters;
    this.terrainSprite.height = world.cellsY * world.cellSizeM;
    this.camera.addChild(this.terrainSprite);

    this.organismLayer = new Container();
    this.organismLayer.cullableChildren = true;
    this.camera.addChild(this.organismLayer);

    this.overlayLayer = new Graphics();
    this.overlayLayer.visible = false;
    this.camera.addChild(this.overlayLayer);

    this.selectionRing = new Graphics();
    this.selectionRing.visible = false;
    this.camera.addChild(this.selectionRing);

    // Fit the world into the view initially.
    const fit = Math.min(
      app.renderer.width / worldMeters,
      app.renderer.height / worldMeters,
    );
    this.baseScale = fit;
    this.camera.scale.set(fit);
  }

  applyTiles(tiles: TileBlock): void {
    const image = this.terrainContext.createImageData(tiles.width, tiles.height);
    for (let index = 0; index < tiles.width * tiles.height; index += 1) {
      const flags = tiles.cells[index * 2] ?? 0;
      const food = tiles.cells[index * 2 + 1] ?? 0;
      const offset = index * 4;
      if (flags & 1) {
        // Land: darker with depleted food, greener with abundant food.
        image.data[offset] = 46 + ((110 - 46) * (255 - food)) / 512;
        image.data[offset + 1] = 90 + ((190 - 90) * food) / 255;
        image.data[offset + 2] = 46;
      } else {
        image.data[offset] = 22;
        image.data[offset + 1] = 40;
        image.data[offset + 2] = 84;
      }
      image.data[offset + 3] = 255;
    }
    this.terrainContext.putImageData(image, tiles.x0, tiles.y0);
    this.terrainTexture.source.update();
  }

  replaceEntities(entities: EntityRecord[]): void {
    const seen = new Set<bigint>();
    for (const record of entities) {
      seen.add(record.id);
      this.upsert(record);
    }
    for (const id of [...this.sprites.keys()]) {
      if (!seen.has(id)) this.remove(id);
    }
  }

  upsert(record: EntityRecord): void {
    this.records.set(record.id, record);
    let sprite = this.sprites.get(record.id);
    if (!sprite) {
      sprite = new Sprite(Texture.WHITE);
      sprite.anchor.set(0.5);
      sprite.cullable = true;
      sprite.cullArea = new Rectangle(-1, -1, 2, 2);
      this.organismLayer.addChild(sprite);
      this.sprites.set(record.id, sprite);
    }
    const scale = 0.6 + (record.bodyScale / 255) * 1.0; // meters across
    sprite.width = scale;
    sprite.height = scale;
    sprite.position.set(record.xFp / FP_PER_METER, record.yFp / FP_PER_METER);
    sprite.rotation = this.reducedMotion
      ? 0
      : (record.headingBam / 65536) * Math.PI * 2;
    sprite.tint = hueToRgb(record.hue / 255, 0.35 + (record.energyFrac / 255) * 0.5);
    if (this.selectedId === record.id) this.drawSelection(record);
  }

  remove(id: bigint): void {
    const sprite = this.sprites.get(id);
    if (sprite) {
      sprite.destroy();
      this.sprites.delete(id);
    }
    this.records.delete(id);
    if (this.selectedId === id) {
      this.selectedId = null;
      this.selectionRing.visible = false;
    }
  }

  select(id: bigint | null): void {
    this.selectedId = id;
    if (id === null) {
      this.selectionRing.visible = false;
      return;
    }
    const record = this.records.get(id);
    if (record) this.drawSelection(record);
  }

  private drawSelection(record: EntityRecord): void {
    // Selection uses a ring plus the inspector text panel, never color
    // alone.
    this.selectionRing.clear();
    this.selectionRing
      .circle(record.xFp / FP_PER_METER, record.yFp / FP_PER_METER, 2.2)
      .stroke({ width: 0.3, color: 0xffffff })
      .circle(record.xFp / FP_PER_METER, record.yFp / FP_PER_METER, 2.8)
      .stroke({ width: 0.15, color: 0x101820 });
    this.selectionRing.visible = true;
  }

  redrawOverlay(sensorRangeM: number | null): void {
    this.overlayLayer.clear();
    this.overlayLayer.visible = this.overlayEnabled;
    if (!this.overlayEnabled) return;
    const worldMeters = this.world.cellsX * this.world.cellSizeM;
    // Cell grid at coarse spacing.
    const step = this.world.cellSizeM * 16;
    for (let x = 0; x <= worldMeters; x += step) {
      this.overlayLayer.moveTo(x, 0).lineTo(x, worldMeters);
    }
    for (let y = 0; y <= worldMeters; y += step) {
      this.overlayLayer.moveTo(0, y).lineTo(worldMeters, y);
    }
    this.overlayLayer.stroke({ width: 0.12, color: 0xffffff, alpha: 0.25 });
    // Selected organism sensor radius.
    if (this.selectedId !== null && sensorRangeM !== null) {
      const record = this.records.get(this.selectedId);
      if (record) {
        this.overlayLayer
          .circle(record.xFp / FP_PER_METER, record.yFp / FP_PER_METER, sensorRangeM)
          .stroke({ width: 0.2, color: 0x59c3c3, alpha: 0.9 });
      }
    }
  }

  /// Nearest organism to a world-space point within a pick radius.
  pick(worldX: number, worldY: number, radiusM: number): bigint | null {
    let best: bigint | null = null;
    let bestDistance = radiusM * radiusM;
    for (const [id, record] of this.records) {
      const dx = record.xFp / FP_PER_METER - worldX;
      const dy = record.yFp / FP_PER_METER - worldY;
      const distance = dx * dx + dy * dy;
      if (distance < bestDistance) {
        bestDistance = distance;
        best = id;
      }
    }
    return best;
  }
}

function hueToRgb(hue: number, lightness: number): number {
  const chroma = (1 - Math.abs(2 * lightness - 1)) * 0.85;
  const section = (hue * 6) % 6;
  const x = chroma * (1 - Math.abs((section % 2) - 1));
  let rgb: [number, number, number];
  if (section < 1) rgb = [chroma, x, 0];
  else if (section < 2) rgb = [x, chroma, 0];
  else if (section < 3) rgb = [0, chroma, x];
  else if (section < 4) rgb = [0, x, chroma];
  else if (section < 5) rgb = [x, 0, chroma];
  else rgb = [chroma, 0, x];
  const m = lightness - chroma / 2;
  const to255 = (value: number) => Math.round((value + m) * 255);
  return (to255(rgb[0]) << 16) | (to255(rgb[1]) << 8) | to255(rgb[2]);
}
