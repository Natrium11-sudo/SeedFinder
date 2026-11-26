import { StrictMode, useEffect, useState } from "react";
import { createRoot } from "react-dom/client";

import App from "../../src/App";
import { CollectionPanel } from "../../src/components/CollectionPanel";
import { MapPanel } from "../../src/components/MapPanel";
import type { MapLayers } from "../../src/lib/map";
import type { PreviewData, SavedSeed, SeedCollection } from "../../src/types";
import fixture from "./data.json";
import "../../src/styles.css";
import "./harness.css";

const view = new URLSearchParams(window.location.search).get("view") ?? "app";
const previewDetail = fixture.previewDetail as unknown as PreviewData;
const previewUnderground = fixture.previewUnderground as unknown as PreviewData;
const collections = fixture.collections as unknown as SeedCollection[];

function UndergroundShot() {
  const [layers, setLayers] = useState<MapLayers>({
    layer: "underground",
    depth: -16,
    structures: false,
    hiddenStructures: [],
    strongholds: false,
    slime: false,
  });

  return (
    <div
      className="app"
      style={{ width: 820, height: 900, minHeight: 0, margin: "0 auto" }}
    >
      <div className="workspace" style={{ gridTemplateColumns: "1fr" }}>
        <main
          className="stage"
          style={{ height: 900, maxHeight: 900, paddingTop: 16 }}
        >
          <MapPanel
            preview={previewUnderground}
            loading={false}
            error={null}
            version="26.3"
            zoomIndex={0}
            onZoomIndex={() => undefined}
            center={null}
            onCenter={() => undefined}
            layers={layers}
            onLayers={setLayers}
            structures={fixture.structures}
            onResetView={() => undefined}
            saved={false}
            onToggleSave={() => undefined}
          />
        </main>
      </div>
    </div>
  );
}

function UndergroundAppShot() {
  useEffect(() => {
    const button = Array.from(
      document.querySelectorAll<HTMLButtonElement>(".level-bar button"),
    ).find((entry) => entry.textContent?.trim() === "Underground");
    button?.click();

    // The 1k zoom shows cave biomes (deep dark, lush caves) at 1:4.
    const zoomTimer = window.setTimeout(() => {
      const zoom = Array.from(
        document.querySelectorAll<HTMLButtonElement>(
          ".map-controls .segmented.small button",
        ),
      ).find((entry) => entry.textContent?.trim() === "1k");
      zoom?.click();
    }, 120);

    // The depth slider only exists after the layer switch re-renders.
    const depthTimer = window.setTimeout(() => {
      const slider = document.querySelector<HTMLInputElement>(
        ".depth-field input[type=\"range\"]",
      );
      if (!slider) return;
      const setValue = Object.getOwnPropertyDescriptor(
        HTMLInputElement.prototype,
        "value",
      )?.set;
      setValue?.call(slider, "-16");
      slider.dispatchEvent(new Event("input", { bubbles: true }));
    }, 260);

    return () => {
      window.clearTimeout(zoomTimer);
      window.clearTimeout(depthTimer);
    };
  }, []);

  return <App />;
}

function CollectionsShot() {
  const [activeId, setActiveId] = useState(collections[0]?.id ?? null);

  return (
    <>
      <App />
      <CollectionPanel
        collections={collections}
        activeId={activeId}
        onActivate={setActiveId}
        onCreate={() => undefined}
        onRenameCollection={() => undefined}
        onDeleteCollection={() => undefined}
        onClose={() => undefined}
        onLoad={() => undefined}
        onRemoveSeed={() => undefined}
        onSeedChange={(_collectionId: string, _item: SavedSeed) => undefined}
      />
    </>
  );
}

const container = document.getElementById("root");
if (!container) {
  throw new Error("Screenshot harness is missing #root");
}

createRoot(container).render(
  <StrictMode>
    {view === "underground" ? (
      <UndergroundAppShot />
    ) : view === "underground-panel" ? (
      <UndergroundShot />
    ) : view === "collections" ? (
      <CollectionsShot />
    ) : (
      <App />
    )}
  </StrictMode>,
);
