import { useEffect, useRef, useState } from "react";
import type { BiomeInfo, SeedHit } from "../types";
import { formatNumber, prettify } from "../lib/format";
import { drawThumbnail } from "../lib/map";
import { copyText } from "../lib/clipboard";

interface ResultsPanelProps {
  hits: SeedHit[];
  biomes: BiomeInfo[];
  selectedSeed: string | null;
  onSelect: (seed: string) => void;
  onSave: (hit: SeedHit) => void;
  isSaved: (seed: string) => boolean;
  running: boolean;
}

export function ResultsPanel({
  hits,
  biomes,
  selectedSeed,
  onSelect,
  onSave,
  isSaved,
  running,
}: ResultsPanelProps) {
  const names = useRef<Map<number, string>>(new Map());
  names.current = new Map(biomes.map((biome) => [biome.id, biome.name]));

  return (
    <section className="results-panel" aria-label="Hits">
      <header className="results-head">
        <h2>Hits</h2>
        <span className="count">{formatNumber(hits.length)}</span>
      </header>

      {hits.length === 0 && (
        <div className="results-empty">
          {running ? (
            <p>The search is running. Hits appear here as soon as a seed matches every rule.</p>
          ) : (
            <p>
              No hits yet. Start a search on the left – tighter rules mean fewer,
              but better-matching seeds.
            </p>
          )}
        </div>
      )}

      <div className="hit-list">
        {hits.map((hit) => {
          const selected = hit.seed === selectedSeed;
          return (
            <article
              key={hit.seed}
              className={selected ? "hit is-selected" : "hit"}
            >
              <MiniMap
                ids={hit.thumbnail}
                gridSize={hit.thumbnail_size}
                names={names.current}
              />

              <button
                type="button"
                className="hit-main"
                onClick={() => onSelect(hit.seed)}
                aria-pressed={selected}
              >
                <span className="hit-seed">{hit.seed}</span>
                <span className="hit-meta">
                  {prettify(hit.spawn_biome)} · {formatNumber(hit.spawn_x)} /{" "}
                  {formatNumber(hit.spawn_z)}
                </span>
                {hit.structures.length > 0 && (
                  <span className="hit-structures">
                    {hit.structures.map((structure) => (
                      <span className="hit-structure" key={`${structure.kind}-${structure.x}`}>
                        <i aria-hidden="true" />
                        {structure.label}
                      </span>
                    ))}
                  </span>
                )}
              </button>

              <div className="hit-actions">
                <CopyButton seed={hit.seed} />
                <button
                  type="button"
                  className={isSaved(hit.seed) ? "icon-btn is-saved" : "icon-btn"}
                  title={
                    isSaved(hit.seed)
                      ? "Remove from collection"
                      : "Add to collection"
                  }
                  aria-label={
                    isSaved(hit.seed)
                      ? "Remove from collection"
                      : "Add to collection"
                  }
                  aria-pressed={isSaved(hit.seed)}
                  onClick={() => onSave(hit)}
                >
                  <BookmarkIcon filled={isSaved(hit.seed)} />
                </button>
              </div>
            </article>
          );
        })}
      </div>
    </section>
  );
}

function MiniMap({
  ids,
  gridSize,
  names,
}: {
  ids: number[];
  gridSize: number;
  names: Map<number, string>;
}) {
  const ref = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    if (ref.current) drawThumbnail(ref.current, ids, gridSize, names);
  }, [ids, gridSize, names]);

  return <canvas ref={ref} className="thumb" aria-hidden="true" />;
}

function CopyButton({ seed }: { seed: string }) {
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    if (!copied) return;
    const timeout = window.setTimeout(() => setCopied(false), 1400);
    return () => window.clearTimeout(timeout);
  }, [copied]);

  return (
    <button
      type="button"
      className="icon-btn"
      title="Copy seed"
      aria-label="Copy seed"
      onClick={async () => {
        const ok = await copyText(seed);
        if (ok) setCopied(true);
      }}
    >
      {copied ? <CheckIcon /> : <CopyIcon />}
    </button>
  );
}

function CopyIcon() {
  return (
    <svg viewBox="0 0 20 20" width="15" height="15" aria-hidden="true">
      <rect x="6.5" y="6.5" width="9" height="10" rx="1.5" fill="none" stroke="currentColor" />
      <path d="M4.5 13V3.5h8" fill="none" stroke="currentColor" />
    </svg>
  );
}

function CheckIcon() {
  return (
    <svg viewBox="0 0 20 20" width="15" height="15" aria-hidden="true">
      <path d="M4 10.5 8 14.5 16 5.5" fill="none" stroke="currentColor" strokeWidth="1.8" />
    </svg>
  );
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
