import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import type { PreviewData, StructureInfo } from "../types";
import { biomeColor } from "../lib/biomes";
import { formatNumber, prettify } from "../lib/format";
import { copyText } from "../lib/clipboard";
import { drawPreview, MAP_ZOOMS, type MapLayers } from "../lib/map";
import { structureColor } from "../lib/structures";
import { onStructureIconsLoaded, structureIconUrl } from "../lib/structureIcons";

interface MapPanelProps {
  preview: PreviewData | null;
  loading: boolean;
  error: string | null;
  version: string;
  zoomIndex: number;
  onZoomIndex: (index: number) => void;
  center: { x: number; z: number } | null;
  onCenter: (center: { x: number; z: number }) => void;
  layers: MapLayers;
  onLayers: (layers: MapLayers) => void;
  structures: StructureInfo[];
  onResetView: () => void;
  saved: boolean;
  onToggleSave: () => void;
}

export function MapPanel({
  preview,
  loading,
  error,
  version,
  zoomIndex,
  onZoomIndex,
  center,
  onCenter,
  layers,
  onLayers,
  structures,
  onResetView,
  saved,
  onToggleSave,
}: MapPanelProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const frameRef = useRef<HTMLDivElement>(null);
  const dragRef = useRef<{ pointerId: number; x: number; y: number } | null>(null);

  const [size, setSize] = useState(0);
  const [hover, setHover] = useState<{ x: number; z: number } | null>(null);
  const [dragOffset, setDragOffset] = useState({ x: 0, y: 0 });
  const [dragging, setDragging] = useState(false);
  const [copied, setCopied] = useState(false);
  const [copiedCoords, setCopiedCoords] = useState<string | null>(null);
  const [iconsTick, setIconsTick] = useState(0);
  const [structurePickerOpen, setStructurePickerOpen] = useState(true);
  const [lastReadout, setLastReadout] = useState<{
    x: number;
    z: number;
    biome: string;
  } | null>(null);

  const span = preview ? preview.width * preview.cell_scale : 0;
  const caveBiomesAvailable = supportsCaveBiomes(version);
  const visibleStructureCount = structures.filter(
    (entry) => !layers.hiddenStructures.includes(entry.id),
  ).length;

  useEffect(() => {
    const frame = frameRef.current;
    if (!frame) return;

    const measure = () => {
      const box = frame.getBoundingClientRect();
      setSize(Math.max(180, Math.min(box.width - 16, box.height - 16)));
    };
    measure();

    const observer = new ResizeObserver(measure);
    observer.observe(frame);
    return () => observer.disconnect();
  }, []);

  // Layout effect: the raster must be on the canvas before the frame paints,
  // otherwise every new preview shows one blank frame first.
  useLayoutEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || !preview || size <= 0) return;
    drawPreview(canvas, preview, size, layers);
  }, [preview, size, layers, iconsTick]);

  useEffect(
    () => onStructureIconsLoaded(() => setIconsTick((tick) => tick + 1)),
    [],
  );

  useEffect(() => {
    if (!copied) return;
    const timeout = window.setTimeout(() => setCopied(false), 1400);
    return () => window.clearTimeout(timeout);
  }, [copied]);

  useEffect(() => {
    if (copiedCoords === null) return;
    const timeout = window.setTimeout(() => setCopiedCoords(null), 1400);
    return () => window.clearTimeout(timeout);
  }, [copiedCoords]);

  useEffect(() => {
    const frame = frameRef.current;
    if (!frame) return;

    const onWheel = (event: WheelEvent) => {
      if (!preview) return;
      event.preventDefault();
      const next = event.deltaY < 0 ? zoomIndex - 1 : zoomIndex + 1;
      if (next >= 0 && next < MAP_ZOOMS.length) onZoomIndex(next);
    };

    frame.addEventListener("wheel", onWheel, { passive: false });
    return () => frame.removeEventListener("wheel", onWheel);
  }, [preview, zoomIndex, onZoomIndex]);

  const hoverInfo = useMemo(() => {
    if (!preview || !hover) return null;
    const id = preview.biomes[hover.z * preview.width + hover.x] ?? -1;
    const biome = preview.legend.find((entry) => entry.id === id)?.label ?? "Unbekannt";
    return {
      x: preview.origin_block_x + hover.x * preview.cell_scale,
      z: preview.origin_block_z + hover.z * preview.cell_scale,
      biome,
    };
  }, [preview, hover]);

  // The pointer leaves the map before a click can land on the readout button,
  // so keep the last sampled position for copying.
  useEffect(() => {
    if (hoverInfo) setLastReadout(hoverInfo);
  }, [hoverInfo]);

  const readout = hoverInfo ?? lastReadout;

  const copyCoords = async (key: string, x: number, z: number) => {
    if (await copyText(`${x} ${z}`)) setCopiedCoords(key);
  };

  const markerLegend = useMemo(() => {
    if (!preview) return [] as [string, string][];
    const seen = new Map<string, string>();
    for (const marker of preview.markers) {
      const visible =
        marker.kind === "stronghold" ? layers.strongholds : layers.structures;
      if (visible && !seen.has(marker.kind)) seen.set(marker.kind, marker.label);
    }
    return [...seen.entries()];
  }, [preview, layers]);

  const trackHover = (event: React.PointerEvent<HTMLDivElement>) => {
    const canvas = canvasRef.current;
    if (!canvas || !preview) return;

    const rect = canvas.getBoundingClientRect();
    const cellX = ((event.clientX - rect.left) / rect.width) * preview.width;
    const cellZ = ((event.clientY - rect.top) / rect.height) * preview.height;
    if (cellX < 0 || cellZ < 0 || cellX >= preview.width || cellZ >= preview.height) {
      setHover(null);
      return;
    }

    const next = { x: Math.floor(cellX), z: Math.floor(cellZ) };
    setHover((current) =>
      current && current.x === next.x && current.z === next.z ? current : next,
    );
  };

  const onPointerDown = (event: React.PointerEvent<HTMLDivElement>) => {
    if (!preview || event.button !== 0) return;
    dragRef.current = { pointerId: event.pointerId, x: event.clientX, y: event.clientY };
    event.currentTarget.setPointerCapture(event.pointerId);
    setDragging(true);
  };

  const onPointerMove = (event: React.PointerEvent<HTMLDivElement>) => {
    trackHover(event);
    const start = dragRef.current;
    if (!start) return;
    setDragOffset({ x: event.clientX - start.x, y: event.clientY - start.y });
  };

  const onPointerUp = (event: React.PointerEvent<HTMLDivElement>) => {
    const start = dragRef.current;
    dragRef.current = null;
    setDragging(false);
    setDragOffset({ x: 0, y: 0 });
    if (!start || !preview) return;

    const dx = event.clientX - start.x;
    const dy = event.clientY - start.y;
    if (Math.abs(dx) < 3 && Math.abs(dy) < 3) return;

    const perPixel = span / Math.max(1, size);
    onCenter({
      x: Math.round(preview.origin_block_x + span / 2 - dx * perPixel),
      z: Math.round(preview.origin_block_z + span / 2 - dy * perPixel),
    });
  };

  const zoomIn = () => {
    if (zoomIndex > 0) onZoomIndex(zoomIndex - 1);
  };
  const zoomOut = () => {
    if (zoomIndex < MAP_ZOOMS.length - 1) onZoomIndex(zoomIndex + 1);
  };

  return (
    <section className="map-panel" aria-label="Map preview">
      <header className="map-head">
        <div>
          <h2>Map</h2>
          <p className="map-sub">
            {preview
              ? `Seed ${preview.seed}`
              : "Pick a hit, check a seed, or drag the map"}
          </p>
        </div>

        <div className="map-controls">
          <button
            type="button"
            className="icon-btn"
            onClick={zoomIn}
            disabled={zoomIndex === 0}
            title="Zoom in"
            aria-label="Zoom in"
          >
            −
          </button>
          <div className="segmented small" role="group" aria-label="Map extent">
            {MAP_ZOOMS.map((zoom, index) => (
              <button
                key={zoom.label}
                type="button"
                className={index === zoomIndex ? "is-active" : undefined}
                onClick={() => onZoomIndex(index)}
                title={`${zoom.label} blocks`}
              >
                {zoom.label}
              </button>
            ))}
          </div>
          <button
            type="button"
            className="icon-btn"
            onClick={zoomOut}
            disabled={zoomIndex === MAP_ZOOMS.length - 1}
            title="Zoom out"
            aria-label="Zoom out"
          >
            +
          </button>
          <button type="button" className="link reset" onClick={onResetView}>
            {center ? "Go to spawn" : "Reset view"}
          </button>
        </div>
      </header>

      {preview && (
        <div className="seed-plate">
          <div className="seed-plate-top">
            <span className="seed-plate-label">Seed · {version} · complete</span>
            <div className="seed-plate-actions">
              <button
                type="button"
                className="icon-btn"
                title="Copy seed"
                aria-label="Copy seed"
                onClick={async () => {
                  if (await copyText(preview.seed)) setCopied(true);
                }}
              >
                {copied ? "✓" : "⧉"}
              </button>
              <button
                type="button"
                className={saved ? "icon-btn is-saved" : "icon-btn"}
                title={
                  saved
                    ? "Remove from collection"
                    : "Bookmark this seed"
                }
                aria-label={
                  saved
                    ? "Remove from collection"
                    : "Bookmark this seed"
                }
                aria-pressed={saved}
                onClick={onToggleSave}
              >
                <BookmarkIcon filled={saved} />
              </button>
            </div>
          </div>
          <code className="seed-plate-value">{preview.seed}</code>
          <div className="seed-plate-meta">
            <button
              type="button"
              className="coord-copy"
              title="Copy spawn coordinates"
              aria-label="Copy spawn coordinates"
              onClick={() =>
                void copyCoords("spawn", preview.spawn_x, preview.spawn_z)
              }
            >
              Spawn {formatNumber(preview.spawn_x)} / {formatNumber(preview.spawn_z)}
              <span aria-hidden="true">{copiedCoords === "spawn" ? "✓" : "⧉"}</span>
            </button>
            <span>
              · {prettify(preview.spawn_biome)} · {formatNumber(span)} ×{" "}
              {formatNumber(span)} blocks
            </span>
          </div>
        </div>
      )}

      <div
        className="map-frame"
        ref={frameRef}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        onPointerCancel={onPointerUp}
        onPointerLeave={() => setHover(null)}
        style={{ cursor: preview ? (dragging ? "grabbing" : "grab") : "default" }}
      >
        {preview && (
          <div
            className="map-stage"
            style={{
              width: size || undefined,
              height: size || undefined,
              transform: `translate(${dragOffset.x}px, ${dragOffset.y}px)`,
            }}
          >
            <canvas
              ref={canvasRef}
              className="map-canvas"
              style={{ width: size || undefined, height: size || undefined }}
              aria-label="Biome map"
            />
            {hover && size > 0 && (
              <span
                className="hover-cell"
                aria-hidden="true"
                style={{
                  left: (hover.x * size) / preview.width,
                  top: (hover.z * size) / preview.height,
                  width: size / preview.width,
                  height: size / preview.height,
                }}
              />
            )}
          </div>
        )}

        {!preview && !loading && (
          <div className="map-empty">
            <MapGlyph />
            <p>No map yet</p>
            <span>
              Start a search or enter a seed on the left. Drag to pan, scroll to
              zoom from 512 to 16,384 blocks.
            </span>
          </div>
        )}

        {loading && (
          <div className="map-loading" role="status">
            <span className="spinner" aria-hidden="true" />
            Rendering map …
          </div>
        )}

        {error && <div className="map-error">{error}</div>}

        {preview && !loading && (
          <>
            <span className="map-tag top-left">
              {formatNumber(span)} × {formatNumber(span)} blocks
            </span>
            {preview.layer === "underground" ? (
              <span className="map-tag top-right">
                Underground · Y {preview.sample_y}
              </span>
            ) : (
              preview.heights.length > 0 && (
                <span className="map-tag top-right">
                  Height {Math.round(preview.min_height)}–{Math.round(preview.max_height)}
                </span>
              )
            )}
            <span className="map-tag bottom-left">
              Spawn {formatNumber(preview.spawn_x)} / {formatNumber(preview.spawn_z)}
            </span>
            <span className="map-tag bottom-right">
              1 cell = {preview.cell_scale} blocks
            </span>
          </>
        )}
      </div>

      <div className="level-bar">
        <div className="segmented small" role="group" aria-label="Map layer">
          <button
            type="button"
            className={layers.layer === "surface" ? "is-active" : undefined}
            onClick={() => onLayers({ ...layers, layer: "surface" })}
            title="Whole world from above, without caves"
          >
            Surface
          </button>
          <button
            type="button"
            className={layers.layer === "underground" ? "is-active" : undefined}
            onClick={() => onLayers({ ...layers, layer: "underground" })}
            title="Underground with cave biomes"
          >
            Underground
          </button>
        </div>

        {layers.layer === "underground" && (
          <label className="depth-field">
            <span>Y</span>
            <input
              type="range"
              min={-64}
              max={320}
              step={8}
              value={layers.depth}
              onChange={(event) =>
                onLayers({ ...layers, depth: Number(event.currentTarget.value) })
              }
              aria-label="Underground layer height in blocks"
            />
            <b>{layers.depth}</b>
          </label>
        )}

        <span className="level-note">
          {layers.layer === "surface"
            ? "Top-down, without cave biomes."
            : caveBiomesAvailable
              ? "Cave biomes at a fixed height; coarse scales round Y."
              : "Cave biomes start in 1.18 – older versions show the surface here."}
        </span>
      </div>

      <div className="layer-toggles" role="group" aria-label="Map layers">
        <label className="toggle">
          <input
            type="checkbox"
            checked={layers.structures}
            onChange={(event) =>
              onLayers({ ...layers, structures: event.currentTarget.checked })
            }
          />
          Structures
        </label>
        <label className="toggle">
          <input
            type="checkbox"
            checked={layers.strongholds}
            onChange={(event) =>
              onLayers({ ...layers, strongholds: event.currentTarget.checked })
            }
          />
          Strongholds
        </label>
        <label className="toggle">
          <input
            type="checkbox"
            checked={layers.slime}
            onChange={(event) => onLayers({ ...layers, slime: event.currentTarget.checked })}
          />
          Slime chunks
        </label>
        {readout ? (
          <button
            type="button"
            className="map-readout coord-copy"
            title="Copy coordinates (X Z)"
            aria-label="Copy coordinates"
            onClick={() => void copyCoords("hover", readout.x, readout.z)}
          >
            {formatNumber(readout.x)} / {formatNumber(readout.z)} · {readout.biome}
            <span aria-hidden="true">{copiedCoords === "hover" ? "✓" : "⧉"}</span>
          </button>
        ) : (
          <span className="map-readout">
            Hover the map for coordinates and biome
          </span>
        )}
      </div>

      {structures.length > 0 && (
        <section className="structure-layer">
          <button
            type="button"
            className="structure-summary"
            aria-expanded={structurePickerOpen}
            onClick={() => setStructurePickerOpen((open) => !open)}
          >
            <span>Show or hide structure types</span>
            <span className="structure-layer-count">
              {layers.structures
                ? `${visibleStructureCount} of ${structures.length} visible`
                : "Layer off"}
            </span>
          </button>
          {structurePickerOpen && (
          <div className="structure-layer-body">
            <div className="structure-head">
              <span className="rule-note">
                Like Chunkbase: the checkbox controls which type appears on the
                map.
              </span>
              <span className="structure-actions">
                <button
                  type="button"
                  className="link"
                  onClick={() =>
                    onLayers({ ...layers, structures: true, hiddenStructures: [] })
                  }
                >
                  All
                </button>
                <button
                  type="button"
                  className="link"
                  onClick={() =>
                    onLayers({
                      ...layers,
                      structures: true,
                      hiddenStructures: structures.map((entry) => entry.id),
                    })
                  }
                >
                  None
                </button>
              </span>
            </div>

            <div className="structure-grid" role="group" aria-label="Structures on the map">
              {structures.map((structure) => {
                const shown =
                  layers.structures &&
                  !layers.hiddenStructures.includes(structure.id);
                const icon = structureIconUrl(structure.id);
                return (
                  <label
                    key={structure.id}
                    className={shown ? "structure-choice is-on" : "structure-choice"}
                  >
                    <input
                      type="checkbox"
                      checked={shown}
                      onChange={() => {
                        const hidden = layers.hiddenStructures.includes(structure.id)
                          ? layers.hiddenStructures.filter((kind) => kind !== structure.id)
                          : [...layers.hiddenStructures, structure.id];
                        onLayers({ ...layers, structures: true, hiddenStructures: hidden });
                      }}
                    />
                    {icon && (
                      <img
                        className="structure-icon"
                        src={icon}
                        alt=""
                        aria-hidden="true"
                      />
                    )}
                    <span>{structure.label}</span>
                  </label>
                );
              })}
            </div>
          </div>
          )}
        </section>
      )}

      <div className="map-foot">
        <div className="legend" aria-label="Legend">
          {(preview?.legend ?? []).slice(0, 10).map((biome) => (
            <span className="legend-item" key={`${biome.id}-${biome.name}`}>
              <i
                className="swatch"
                style={{ background: biomeColor(biome.name) }}
                aria-hidden="true"
              />
              {biome.label}
            </span>
          ))}
          {markerLegend.slice(0, 12).map(([kind, label]) => {
            const icon = structureIconUrl(kind);
            return (
              <span className="legend-item" key={kind}>
                {icon ? (
                  <img className="legend-icon" src={icon} alt="" aria-hidden="true" />
                ) : (
                  <i
                    className="swatch diamond"
                    style={{ background: structureColor(kind) }}
                    aria-hidden="true"
                  />
                )}
                {label}
              </span>
            );
          })}
          {preview && preview.legend.length === 0 && (
            <span className="legend-item">No biome data</span>
          )}
        </div>

        {preview && (
          <p className="map-facts">
            {preview.markers.length} markers · {preview.slime_chunks.length} slime ·{" "}
            {preview.generated_ms} ms
          </p>
        )}
      </div>
    </section>
  );
}

/** Cave biomes exist from 1.18 on, where the biome noise became 3D. */
function supportsCaveBiomes(version: string): boolean {
  const [major, minor] = version.split(".").map(Number);
  if (!Number.isFinite(major)) return false;
  if (major >= 26) return true;
  return major === 1 && Number.isFinite(minor) && minor >= 18;
}

function BookmarkIcon({ filled }: { filled: boolean }) {
  return (
    <svg viewBox="0 0 20 20" width="15" height="15" aria-hidden="true">
      <path
        d="M5.5 3.5h9v13l-4.5-3.2-4.5 3.2v-13Z"
        fill={filled ? "currentColor" : "none"}
        stroke="currentColor"
        strokeWidth="1.2"
      />
    </svg>
  );
}

function MapGlyph() {
  return (
    <svg viewBox="0 0 64 64" width="64" height="64" aria-hidden="true">
      <circle cx="32" cy="32" r="25" fill="none" stroke="currentColor" strokeWidth="1" />
      <circle cx="32" cy="32" r="16" fill="none" stroke="currentColor" strokeWidth="0.75" />
      <circle cx="32" cy="32" r="8" fill="none" stroke="currentColor" strokeWidth="0.5" />
      <path d="M32 3v58M3 32h58" stroke="currentColor" strokeWidth="0.75" />
      <circle cx="32" cy="32" r="2.5" fill="currentColor" />
    </svg>
  );
}
