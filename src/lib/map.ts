import { biomeColor, shadeColor } from "./biomes";
import { structureColor } from "./structures";
import { structureIcon } from "./structureIcons";
import type { PreviewData } from "../types";

/** One selectable map extent; `cells * scale` is the visible span in blocks. */
export interface MapZoom {
  label: string;
  cells: number;
  scale: 4 | 16 | 64;
}

/** Zoom steps from detail view to a 16k-block overview. */
export const MAP_ZOOMS: readonly MapZoom[] = [
  { label: "512", cells: 128, scale: 4 },
  { label: "1k", cells: 256, scale: 4 },
  { label: "2k", cells: 128, scale: 16 },
  { label: "4k", cells: 256, scale: 16 },
  { label: "8k", cells: 128, scale: 64 },
  { label: "16k", cells: 256, scale: 64 },
];

export interface MapLayers {
  /** Top-down surface, or a fixed underground height for cave biomes. */
  layer: "surface" | "underground";
  /** Block height sampled by the underground layer. */
  depth: number;
  /** Master switch for the structure layer. */
  structures: boolean;
  /** Structure slugs hidden with the per-kind map picker. */
  hiddenStructures: string[];
  strongholds: boolean;
  slime: boolean;
}

export type MapView = MapLayers;

/** Paints the biome / height raster with the optional marker layers. */
export function drawPreview(
  canvas: HTMLCanvasElement,
  data: PreviewData,
  size: number,
  view: MapView,
): void {
  const context = canvas.getContext("2d");
  if (!context) return;

  const dpr = window.devicePixelRatio || 1;
  const pixels = Math.max(1, Math.floor(size));
  canvas.width = Math.floor(pixels * dpr);
  canvas.height = Math.floor(pixels * dpr);
  context.setTransform(dpr, 0, 0, dpr, 0, 0);
  context.clearRect(0, 0, pixels, pixels);

  const names = new Map<number, string>();
  for (const biome of data.legend) names.set(biome.id, biome.name);

  const { width, height } = data;
  const cellWidth = pixels / width;
  const cellHeight = pixels / height;
  const hasHeights = data.heights.length === width * height;

  for (let z = 0; z < height; z += 1) {
    for (let x = 0; x < width; x += 1) {
      const index = z * width + x;
      const name = names.get(data.biomes[index] ?? -1);
      let color = biomeColor(name);

      if (hasHeights) {
        const h = data.heights[index] ?? 0;
        const left = data.heights[index - 1] ?? h;
        const right = data.heights[index + 1] ?? h;
        const up = data.heights[index - width] ?? h;
        const down = data.heights[index + width] ?? h;
        const light = clamp(1 + ((left - right) + (up - down)) * 0.014, 0.7, 1.3);
        color = shadeColor(color, light);
      }

      context.fillStyle = color;
      context.fillRect(
        x * cellWidth,
        z * cellHeight,
        Math.ceil(cellWidth) + 0.5,
        Math.ceil(cellHeight) + 0.5,
      );
    }
  }

  const toCanvas = (blockX: number, blockZ: number): [number, number] => [
    ((blockX - data.origin_block_x) / data.cell_scale) * cellWidth,
    ((blockZ - data.origin_block_z) / data.cell_scale) * cellHeight,
  ];

  if (view.slime && data.slime_chunks.length > 0) {
    const chunkCells = 16 / data.cell_scale;
    context.fillStyle = "rgba(126, 200, 80, 0.42)";
    for (const chunk of data.slime_chunks) {
      const [x, y] = toCanvas(chunk.x * 16, chunk.z * 16);
      const side = chunkCells * cellWidth;
      if (x < -side || y < -side || x > pixels || y > pixels) continue;
      context.fillRect(x, y, side + 0.5, side + 0.5);
    }
  }

  if (data.cell_scale <= 4) {
    context.strokeStyle = "rgba(20, 26, 21, 0.09)";
    context.lineWidth = 1;
    for (let x = 4; x < width; x += 4) {
      context.beginPath();
      context.moveTo(x * cellWidth, 0);
      context.lineTo(x * cellWidth, pixels);
      context.stroke();
    }
    for (let z = 4; z < height; z += 4) {
      context.beginPath();
      context.moveTo(0, z * cellHeight);
      context.lineTo(pixels, z * cellHeight);
      context.stroke();
    }
  }

  const radius = pixels < 420 ? 3.5 : 4.5;
  for (const marker of data.markers) {
    if (marker.kind === "stronghold") {
      if (!view.strongholds) continue;
    } else if (
      !view.structures ||
      view.hiddenStructures.includes(marker.kind)
    ) {
      continue;
    }
    const [x, y] = toCanvas(marker.x, marker.z);
    if (x < -10 || y < -10 || x > pixels + 10 || y > pixels + 10) continue;

    const color = structureColor(marker.kind);
    context.save();
    context.translate(x, y);

    const sprite = structureIcon(marker.kind);
    if (sprite) {
      const plate = pixels < 420 ? 7 : 8.5;
      context.beginPath();
      context.arc(0, 0, plate, 0, Math.PI * 2);
      context.fillStyle = "rgba(250, 246, 236, 0.96)";
      context.fill();
      context.lineWidth = marker.kind === "stronghold" ? 2 : 1.3;
      context.strokeStyle = color;
      context.stroke();

      const box = plate * 1.55;
      const scale = Math.min(box / sprite.naturalWidth, box / sprite.naturalHeight);
      const width = sprite.naturalWidth * scale;
      const height = sprite.naturalHeight * scale;
      context.drawImage(sprite, -width / 2, -height / 2, width, height);
      context.restore();
      continue;
    }

    context.lineWidth = 1.4;
    context.strokeStyle = "rgba(255, 250, 240, 0.9)";

    if (marker.kind === "stronghold") {
      context.beginPath();
      context.arc(0, 0, radius + 1.5, 0, Math.PI * 2);
      context.fillStyle = "#f4f2e9";
      context.fill();
      context.strokeStyle = color;
      context.lineWidth = 2;
      context.stroke();
      context.fillStyle = color;
      context.beginPath();
      context.arc(0, 0, 1.6, 0, Math.PI * 2);
      context.fill();
    } else {
      context.rotate(Math.PI / 4);
      context.fillStyle = color;
      context.fillRect(-radius, -radius, radius * 2, radius * 2);
      context.strokeRect(-radius, -radius, radius * 2, radius * 2);
    }
    context.restore();
  }

  // Spawn crosshair.
  const [spawnX, spawnY] = toCanvas(data.spawn_x, data.spawn_z);
  if (spawnX > -12 && spawnY > -12 && spawnX < pixels + 12 && spawnY < pixels + 12) {
    context.strokeStyle = "rgba(20, 25, 21, 0.85)";
    context.lineWidth = 1.25;
    context.beginPath();
    context.moveTo(spawnX - 9, spawnY);
    context.lineTo(spawnX + 9, spawnY);
    context.moveTo(spawnX, spawnY - 9);
    context.lineTo(spawnX, spawnY + 9);
    context.stroke();

    context.beginPath();
    context.arc(spawnX, spawnY, 4.5, 0, Math.PI * 2);
    context.fillStyle = "#f4f2e9";
    context.fill();
    context.strokeStyle = "#bd4726";
    context.lineWidth = 2;
    context.stroke();
  }
}

/** Flat biome thumbnail used in the result list. */
export function drawThumbnail(
  canvas: HTMLCanvasElement,
  ids: number[],
  gridSize: number,
  names: Map<number, string>,
): void {
  const context = canvas.getContext("2d");
  if (!context) return;
  if (gridSize <= 0 || ids.length < gridSize * gridSize) {
    context.clearRect(0, 0, canvas.width, canvas.height);
    return;
  }

  const dpr = window.devicePixelRatio || 1;
  const cssSize = canvas.clientWidth || 48;
  const size = Math.max(1, Math.floor(cssSize));
  canvas.width = Math.floor(size * dpr);
  canvas.height = Math.floor(size * dpr);
  context.setTransform(dpr, 0, 0, dpr, 0, 0);

  const cell = size / gridSize;
  for (let z = 0; z < gridSize; z += 1) {
    for (let x = 0; x < gridSize; x += 1) {
      const id = ids[z * gridSize + x] ?? -1;
      context.fillStyle = biomeColor(names.get(id));
      context.fillRect(x * cell, z * cell, Math.ceil(cell) + 0.5, Math.ceil(cell) + 0.5);
    }
  }

  context.fillStyle = "#f4f2e9";
  context.beginPath();
  context.arc(size / 2, size / 2, 2.5, 0, Math.PI * 2);
  context.fill();
  context.strokeStyle = "#bd4726";
  context.lineWidth = 1.5;
  context.stroke();
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}
