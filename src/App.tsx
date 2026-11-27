import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { UnlistenFn } from "@tauri-apps/api/event";

import { CollectionPanel } from "./components/CollectionPanel";
import { MapPanel } from "./components/MapPanel";
import { ResultsPanel } from "./components/ResultsPanel";
import { RuleBuilder } from "./components/RuleBuilder";
import {
  buildRules,
  emptyRuleState,
  type BiomeInfo,
  type KnownPortalSeed,
  type PreviewData,
  type RuleState,
  type SavedSeed,
  type SearchProgress,
  type SeedCollection,
  type SeedHit,
  type StructureInfo,
  type VersionInfo,
} from "./types";
import { formatDuration, formatNumber } from "./lib/format";
import { MAP_ZOOMS, type MapLayers } from "./lib/map";

type SaveTarget = Pick<
  SeedHit,
  "seed" | "spawn_x" | "spawn_z" | "spawn_biome"
>;

const DEFAULT_VERSION = "26.3";
const MAX_HITS = 100;
const ACTIVE_COLLECTION_KEY = "seedfinder:activeCollection";
const EMPTY_PROGRESS: SearchProgress = {
  checked: 0,
  hits: 0,
  target: 0,
  seeds_per_second: 0,
  elapsed_ms: 0,
  running: false,
};

export default function App() {
  const [versions, setVersions] = useState<VersionInfo[]>([]);
  const [version, setVersion] = useState(DEFAULT_VERSION);
  const [biomes, setBiomes] = useState<BiomeInfo[]>([]);
  const [structures, setStructures] = useState<StructureInfo[]>([]);
  const [portalSeeds, setPortalSeeds] = useState<KnownPortalSeed[]>([]);
  const [rules, setRules] = useState<RuleState>({ ...emptyRuleState });

  const [mode, setMode] = useState<"scan" | "seed">("scan");
  const [seedInput, setSeedInput] = useState("3055141959546");
  const [maxSeeds, setMaxSeeds] = useState(10_000);
  const [threads, setThreads] = useState(defaultThreads());
  const [progress, setProgress] = useState<SearchProgress>(EMPTY_PROGRESS);
  const [running, setRunning] = useState(false);
  const [hits, setHits] = useState<SeedHit[]>([]);
  const [selectedSeed, setSelectedSeed] = useState<string | null>(null);
  const [checkedSeed, setCheckedSeed] = useState<string | null>(null);

  const [preview, setPreview] = useState<PreviewData | null>(null);
  const [previewLoading, setPreviewLoading] = useState(false);
  const [previewError, setPreviewError] = useState<string | null>(null);
  const [zoomIndex, setZoomIndex] = useState(0);
  const [center, setCenter] = useState<{ x: number; z: number } | null>(null);
  const [layers, setLayers] = useState<MapLayers>({
    layer: "surface",
    depth: 0,
    structures: true,
    hiddenStructures: [],
    strongholds: false,
    slime: false,
  });

  const [collections, setCollections] = useState<SeedCollection[]>([]);
  const [activeCollectionId, setActiveCollectionId] = useState<string | null>(() =>
    localStorage.getItem(ACTIVE_COLLECTION_KEY),
  );
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  const selectedRef = useRef<string | null>(null);

  const selectSeed = useCallback((seed: string | null) => {
    selectedRef.current = seed;
    setSelectedSeed(seed);
    setCenter(null);
  }, []);

  const activeSeed = selectedSeed ?? checkedSeed;

  /** Load a verified 12-eye seed into the single-seed checker. */
  const usePortalSeed = useCallback((seed: string, presetVersion: string) => {
    setVersion(presetVersion);
    setSeedInput(seed);
    setMode("seed");
    selectSeed(null);
    setCheckedSeed(seed);
    setHits([]);
    setProgress(EMPTY_PROGRESS);
    setRules((current) => ({
      ...current,
      portal: {
        enabled: true,
        minEyes: 12,
        requireLit: true,
        maxStrongholds: 8,
        radius: 0,
      },
    }));
    setNotice(
      `Loaded verified seed ${seed} (Minecraft ${presetVersion}). Press “Check against rules” to confirm all 12 eyes.`,
    );
  }, [selectSeed]);

  /** Check the typed seed against the active rules without scanning random seeds. */
  const checkSeedRules = useCallback(async () => {
    const raw = seedInput.trim();
    if (!/^-?\d+$/.test(raw)) {
      setError("Enter a whole-number seed, e.g. 3055141959546.");
      return;
    }
    setError(null);
    setNotice(null);
    setHits([]);
    selectSeed(null);
    setCheckedSeed(raw);
    try {
      setRunning(true);
      await invoke("search_start", {
        request: {
          version,
          rules: buildRules(rules),
          maxSeeds: 1,
          threads: 1,
          seed: raw,
        },
      });
    } catch (cause) {
      setError(String(cause));
      setRunning(false);
    }
  }, [rules, seedInput, selectSeed, version]);

  useEffect(() => {
    invoke<VersionInfo[]>("version_catalog")
      .then((catalog) => {
        setVersions(catalog);
        if (!catalog.some((entry) => entry.id === DEFAULT_VERSION)) {
          const newest = catalog.at(-1);
          if (newest) setVersion(newest.id);
        }
      })
      .catch((cause: unknown) => setError(String(cause)));
  }, []);

  useEffect(() => {
    invoke<KnownPortalSeed[]>("portal_seed_presets")
      .then(setPortalSeeds)
      .catch((cause: unknown) => setError(String(cause)));
  }, []);

  useEffect(() => {
    invoke<SeedCollection[]>("collection_load")
      .then((loaded) => {
        setCollections(loaded);
        setActiveCollectionId((current) =>
          current && loaded.some((entry) => entry.id === current)
            ? current
            : (loaded[0]?.id ?? null),
        );
      })
      .catch((cause: unknown) => setError(String(cause)));
  }, []);

  useEffect(() => {
    if (activeCollectionId) {
      localStorage.setItem(ACTIVE_COLLECTION_KEY, activeCollectionId);
    } else {
      localStorage.removeItem(ACTIVE_COLLECTION_KEY);
    }
  }, [activeCollectionId]);

  useEffect(() => {
    let cancelled = false;
    setBiomes([]);
    setStructures([]);

    Promise.all([
      invoke<BiomeInfo[]>("biome_catalog", { version }),
      invoke<StructureInfo[]>("structure_catalog", { version }),
    ])
      .then(([biomeList, structureList]) => {
        if (cancelled) return;
        setBiomes(biomeList);
        setStructures(structureList);
        if (biomeList.length === 0) return;

        const names = new Set(biomeList.map((biome) => biome.name));
        const kinds = new Set(structureList.map((structure) => structure.id));
        setRules((current) => ({
          ...current,
          spawnBiomes: current.spawnBiomes.filter((name) => names.has(name)),
          area: {
            ...current.area,
            biome: names.has(current.area.biome) ? current.area.biome : "",
          },
          coord: {
            ...current.coord,
            biomes: current.coord.biomes.filter((name) => names.has(name)),
          },
          structure: {
            ...current.structure,
            kinds: current.structure.kinds.filter((kind) => kinds.has(kind)),
          },
        }));
        setLayers((current) => ({
          ...current,
          hiddenStructures: current.hiddenStructures.filter((kind) =>
            kinds.has(kind),
          ),
        }));
      })
      .catch((cause: unknown) => {
        if (!cancelled) setError(String(cause));
      });

    return () => {
      cancelled = true;
    };
  }, [version]);

  useEffect(() => {
    let active = true;
    const unlisteners: UnlistenFn[] = [];

    void (async () => {
      const fns = await Promise.all([
        listen<SearchProgress>("search:progress", (event) => {
          setProgress(event.payload);
          setRunning(event.payload.running);
        }),
        listen<SeedHit>("search:hit", (event) => {
          const hit = event.payload;
          setHits((current) => [hit, ...current].slice(0, 250));
          if (selectedRef.current === null) selectSeed(hit.seed);
        }),
        listen<SearchProgress>("search:done", (event) => {
          const final = event.payload;
          setProgress(final);
          setRunning(false);
          setNotice(
            final.hits >= MAX_HITS && final.checked < final.target
              ? `Hit limit of ${MAX_HITS} seeds reached – search stopped.`
              : `Search finished: ${formatNumber(final.checked)} seeds checked, ${formatNumber(final.hits)} hits.`,
          );
        }),
        listen<string>("search:error", (event) => {
          setError(String(event.payload));
          setRunning(false);
        }),
      ]);

      if (active) unlisteners.push(...fns);
      else fns.forEach((unlisten) => unlisten());
    })().catch((cause: unknown) => setError(String(cause)));

    return () => {
      active = false;
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, [selectSeed]);

  useEffect(() => {
    if (!activeSeed) {
      setPreview(null);
      setPreviewError(null);
      return;
    }

    const zoom = MAP_ZOOMS[zoomIndex] ?? MAP_ZOOMS[0];
    let cancelled = false;
    setPreviewLoading(true);
    invoke<PreviewData>("preview_seed", {
      request: {
        version,
        seed: activeSeed,
        cells: zoom.cells,
        scale: zoom.scale,
        centerX: center?.x,
        centerZ: center?.z,
        layer: layers.layer,
        depth: layers.depth,
        structures: layers.structures
          ? structures
              .filter((entry) => !layers.hiddenStructures.includes(entry.id))
              .map((entry) => entry.id)
          : [],
        strongholds: layers.strongholds,
        slime: layers.slime,
      },
    })
      .then((data) => {
        if (cancelled) return;
        setPreview(data);
        setPreviewError(null);
      })
      .catch((cause: unknown) => {
        if (cancelled) return;
        setPreviewError(String(cause));
        setPreview(null);
      })
      .finally(() => {
        if (!cancelled) setPreviewLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, [activeSeed, version, zoomIndex, center, layers, structures]);

  const persist = useCallback(async (next: SeedCollection[]) => {
    try {
      await invoke("collection_save", { collections: next });
    } catch (cause) {
      setError(String(cause));
    }
  }, []);

  const commit = useCallback(
    (next: SeedCollection[]) => {
      setCollections(next);
      void persist(next);
    },
    [persist],
  );

  const activeCollection = useMemo(
    () =>
      collections.find((entry) => entry.id === activeCollectionId) ??
      collections[0] ??
      null,
    [collections, activeCollectionId],
  );

  const totalSaved = useMemo(
    () => collections.reduce((sum, entry) => sum + entry.seeds.length, 0),
    [collections],
  );

  const toggleSave = (hit: SaveTarget) => {
    const now = Math.floor(Date.now() / 1000);
    const entry: SavedSeed = {
      seed: hit.seed,
      version,
      name: "",
      spawn_x: hit.spawn_x,
      spawn_z: hit.spawn_z,
      spawn_biome: hit.spawn_biome,
      note: "",
      added_at: now,
    };

    if (!activeCollection) {
      const created: SeedCollection = {
        id: newCollectionId(),
        name: "My collection",
        created_at: now,
        seeds: [entry],
      };
      setActiveCollectionId(created.id);
      setNotice(`Saved seed to “${created.name}”.`);
      commit([...collections, created]);
      return;
    }

    const exists = activeCollection.seeds.some(
      (saved) => saved.seed === hit.seed && saved.version === version,
    );
    const next = collections.map((collection) =>
      collection.id === activeCollection.id
        ? {
            ...collection,
            seeds: exists
              ? collection.seeds.filter(
                  (saved) =>
                    !(saved.seed === hit.seed && saved.version === version),
                )
              : [entry, ...collection.seeds],
          }
        : collection,
    );
    setNotice(
      exists
        ? `Removed seed from “${activeCollection.name}”.`
        : `Saved seed to “${activeCollection.name}”.`,
    );
    commit(next);
  };

  const isSaved = useCallback(
    (seed: string) =>
      activeCollection?.seeds.some(
        (saved) => saved.seed === seed && saved.version === version,
      ) ?? false,
    [activeCollection, version],
  );

  const createCollection = (name: string) => {
    const collection: SeedCollection = {
      id: newCollectionId(),
      name: name.trim() || "New collection",
      created_at: Math.floor(Date.now() / 1000),
      seeds: [],
    };
    setActiveCollectionId(collection.id);
    setNotice(`Created collection “${collection.name}”.`);
    commit([...collections, collection]);
  };

  const renameCollection = (id: string, name: string) => {
    const trimmed = name.trim();
    if (!trimmed) return;
    commit(
      collections.map((entry) =>
        entry.id === id ? { ...entry, name: trimmed } : entry,
      ),
    );
  };

  const deleteCollection = (id: string) => {
    const remaining = collections.filter((entry) => entry.id !== id);
    setActiveCollectionId((current) =>
      current === id ? (remaining[0]?.id ?? null) : current,
    );
    commit(remaining);
  };

  const updateSavedSeed = (
    collectionId: string,
    item: SavedSeed,
    patch: Partial<Pick<SavedSeed, "name" | "note">>,
  ) => {
    commit(
      collections.map((collection) =>
        collection.id === collectionId
          ? {
              ...collection,
              seeds: collection.seeds.map((saved) =>
                saved.seed === item.seed && saved.version === item.version
                  ? { ...saved, ...patch }
                  : saved,
              ),
            }
          : collection,
      ),
    );
  };

  const removeSavedSeed = (collectionId: string, item: SavedSeed) => {
    commit(
      collections.map((collection) =>
        collection.id === collectionId
          ? {
              ...collection,
              seeds: collection.seeds.filter(
                (saved) =>
                  !(saved.seed === item.seed && saved.version === item.version),
              ),
            }
          : collection,
      ),
    );
  };

  const start = async () => {
    setError(null);
    setNotice(null);

    if (mode === "seed") {
      const raw = seedInput.trim();
      if (!/^-?\d+$/.test(raw)) {
        setError("Enter a whole-number seed, e.g. 3055141959546.");
        return;
      }
      selectSeed(null);
      setCenter(null);
      setCheckedSeed(raw);
      setHits([]);
      setRunning(false);
      setProgress(EMPTY_PROGRESS);
      return;
    }

    setHits([]);
    selectSeed(null);
    setCheckedSeed(null);
    setPreview(null);
    setProgress(EMPTY_PROGRESS);

    try {
      setRunning(true);
      await invoke("search_start", {
        request: {
          version,
          rules: buildRules(rules),
          maxSeeds,
          threads,
          seed: null,
        },
      });
    } catch (cause) {
      setError(String(cause));
      setRunning(false);
    }
  };

  const stop = async () => {
    try {
      await invoke("search_stop");
    } catch (cause) {
      setError(String(cause));
    }
  };

  const versionGroups = useMemo(() => {
    const groups = new Map<string, VersionInfo[]>();
    for (const entry of [...versions].reverse()) {
      const group = groups.get(entry.group_label) ?? [];
      group.push(entry);
      groups.set(entry.group_label, group);
    }
    return [...groups.entries()];
  }, [versions]);

  const activeRules = buildRules(rules).length;
  const progressPercent =
    progress.target > 0
      ? Math.min(100, (progress.checked / progress.target) * 100)
      : 0;
  const statusLabel = running
    ? "Search running"
    : progress.target > 0
      ? "Last run finished"
      : "Ready";

  return (
    <div className="app">
      <header className="topbar">
        <div className="brand">
          <CompassMark />
          <div>
            <h1>SeedFinder</h1>
            <p>Minecraft Java · fully local</p>
          </div>
        </div>

        <div className="topbar-right">
          <label className="version-field">
            <span>Version</span>
            <select
              value={version}
              onChange={(event) => {
                setVersion(event.currentTarget.value);
                setHits([]);
                setCenter(null);
                selectSeed(null);
                setPreview(null);
              }}
            >
              {versionGroups.map(([label, entries]) => (
                <optgroup key={label} label={label}>
                  {entries.map((entry) => (
                    <option key={entry.id} value={entry.id}>
                      {entry.id}
                    </option>
                  ))}
                </optgroup>
              ))}
            </select>
          </label>

          <button
            type="button"
            className="collection-button"
            onClick={() => setDrawerOpen(true)}
          >
            <BookmarkIcon />
            Collection
            <span className="collection-count">{totalSaved}</span>
          </button>
        </div>
      </header>

      <div className="workspace">
        <aside className="controls" aria-label="Controls">
          <section className="control-block">
            <div className="segmented" role="group" aria-label="Mode">
              <button
                type="button"
                className={mode === "scan" ? "is-active" : undefined}
                onClick={() => setMode("scan")}
              >
                Search
              </button>
              <button
                type="button"
                className={mode === "seed" ? "is-active" : undefined}
                onClick={() => setMode("seed")}
              >
                Check seed
              </button>
            </div>

            {mode === "seed" ? (
              <div className="seed-check">
                <label className="field">
                  <span>Seed</span>
                  <input
                    inputMode="numeric"
                    value={seedInput}
                    onChange={(event) => setSeedInput(event.currentTarget.value)}
                    onKeyDown={(event) => {
                      if (event.key === "Enter") void start();
                    }}
                    placeholder="z. B. 3055141959546"
                  />
                </label>
                <button type="button" className="btn-primary" onClick={() => void start()}>
                  Show seed on map
                </button>
                <button
                  type="button"
                  className="btn-secondary"
                  onClick={() => void checkSeedRules()}
                >
                  Check against rules
                </button>
                <p className="rule-hint">
                  “Check against rules” runs the active rules on this single
                  seed – useful for verifying a 12-eye portal seed.
                </p>
              </div>
            ) : (
              <div className="scan-settings">
                <div className="field-row">
                  <label className="field">
                    <span>Seeds to scan</span>
                    <select
                      value={maxSeeds}
                      onChange={(event) => setMaxSeeds(Number(event.currentTarget.value))}
                      disabled={running}
                    >
                      <option value={1000}>1.000</option>
                      <option value={10_000}>10.000</option>
                      <option value={100_000}>100.000</option>
                      <option value={1_000_000}>1 Million</option>
                    </select>
                  </label>
                  <label className="field">
                    <span>Threads</span>
                    <input
                      type="number"
                      min={1}
                      max={64}
                      value={threads}
                      onChange={(event) =>
                        setThreads(
                          Math.min(
                            64,
                            Math.max(1, Number(event.currentTarget.value) || 1),
                          ),
                        )
                      }
                      disabled={running}
                    />
                  </label>
                </div>

                <button
                  type="button"
                  className="btn-primary"
                  onClick={() => void start()}
                  disabled={running}
                >
                  Start search
                </button>
                <p className="rule-hint">
                  Stops automatically after {MAX_HITS} hits.
                </p>
              </div>
            )}
          </section>

          <section className="control-block">
            <div className="block-head">
              <h2>Rules</h2>
              <span className="block-meta">
                {activeRules === 0 ? "none" : `${activeRules} active`}
              </span>
            </div>
            <RuleBuilder
              state={rules}
              onChange={setRules}
              biomes={biomes}
              structures={structures}
              portalSeeds={portalSeeds}
              onUseSeed={usePortalSeed}
              disabled={running}
            />
          </section>

          {error && (
            <p className="error" role="alert">
              {error}
            </p>
          )}
          {notice && <p className="notice">{notice}</p>}

          <p className="privacy">
            Everything is computed on this machine. No cloud, no network access
            at runtime.
          </p>
        </aside>

        <main className="stage">
          <MapPanel
            preview={preview}
            loading={previewLoading}
            error={previewError}
            version={version}
            zoomIndex={zoomIndex}
            onZoomIndex={setZoomIndex}
            center={center}
            onCenter={setCenter}
            layers={layers}
            onLayers={setLayers}
            structures={structures}
            onResetView={() => {
              setCenter(null);
              setZoomIndex(0);
            }}
            saved={preview ? isSaved(preview.seed) : false}
            onToggleSave={() => {
              if (preview) toggleSave(preview);
            }}
          />

          <footer className="statusbar">
            <div className="status-left">
              <span className={running ? "run-dot is-running" : "run-dot"} aria-hidden="true" />
              <span>{statusLabel}</span>
              {running && (
                <button type="button" className="stop-button" onClick={() => void stop()}>
                  Stop
                </button>
              )}
            </div>

            <div className="status-metrics">
              <span>
                <b>{formatNumber(progress.checked)}</b> checked
              </span>
              <span>
                <b>{formatNumber(progress.hits)}</b> hits
              </span>
              <span>
                <b>{formatNumber(progress.seeds_per_second)}</b> Seeds/s
              </span>
              <span>
                <b>{formatDuration(progress.elapsed_ms)}</b>
              </span>
            </div>

            <div className="progress-track" aria-hidden="true">
              <span style={{ width: `${progressPercent}%` }} />
            </div>
          </footer>
        </main>

        <aside className="results">
          <ResultsPanel
            hits={hits}
            biomes={biomes}
            selectedSeed={selectedSeed}
            onSelect={selectSeed}
            onSave={toggleSave}
            isSaved={isSaved}
            running={running}
          />
        </aside>
      </div>

      {drawerOpen && (
        <CollectionPanel
          collections={collections}
          activeId={activeCollectionId}
          onActivate={setActiveCollectionId}
          onCreate={createCollection}
          onRenameCollection={renameCollection}
          onDeleteCollection={deleteCollection}
          onClose={() => setDrawerOpen(false)}
          onLoad={(item) => {
            setDrawerOpen(false);
            if (item.version !== version) setVersion(item.version);
            selectSeed(null);
            setCenter(null);
            setCheckedSeed(item.seed);
            setHits([]);
          }}
          onRemoveSeed={removeSavedSeed}
          onSeedChange={updateSavedSeed}
        />
      )}
    </div>
  );
}

function newCollectionId(): string {
  return `c-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`;
}

function defaultThreads(): number {
  const cores = navigator.hardwareConcurrency ?? 8;
  return Math.max(1, Math.min(16, cores));
}

function CompassMark() {
  return (
    <svg className="brand-mark" viewBox="0 0 32 32" width="30" height="30" aria-hidden="true">
      <circle cx="16" cy="16" r="13" fill="none" stroke="currentColor" strokeWidth="1.2" />
      <circle cx="16" cy="16" r="7.5" fill="none" stroke="currentColor" strokeWidth="0.7" />
      <path d="M16 2v6M16 24v6M2 16h6M24 16h6" stroke="currentColor" strokeWidth="1.2" />
      <path d="m16 9 2.4 7L16 23l-2.4-7L16 9Z" fill="currentColor" opacity="0.85" />
    </svg>
  );
}

function BookmarkIcon() {
  return (
    <svg viewBox="0 0 20 20" width="15" height="15" aria-hidden="true">
      <path
        d="M5.5 3.5h9v13l-4.5-3.2-4.5 3.2v-13Z"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.2"
      />
    </svg>
  );
}
