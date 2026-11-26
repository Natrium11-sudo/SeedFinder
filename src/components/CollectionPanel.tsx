import { useEffect, useState } from "react";
import type { SavedSeed, SeedCollection } from "../types";
import { formatNumber, prettify } from "../lib/format";

interface CollectionPanelProps {
  collections: SeedCollection[];
  activeId: string | null;
  onActivate: (id: string) => void;
  onCreate: (name: string) => void;
  onRenameCollection: (id: string, name: string) => void;
  onDeleteCollection: (id: string) => void;
  onClose: () => void;
  onLoad: (item: SavedSeed) => void;
  onRemoveSeed: (collectionId: string, item: SavedSeed) => void;
  onSeedChange: (
    collectionId: string,
    item: SavedSeed,
    patch: { name?: string; note?: string },
  ) => void;
}

export function CollectionPanel({
  collections,
  activeId,
  onActivate,
  onCreate,
  onRenameCollection,
  onDeleteCollection,
  onClose,
  onLoad,
  onRemoveSeed,
  onSeedChange,
}: CollectionPanelProps) {
  const [draftName, setDraftName] = useState("");
  const [confirmDeleteId, setConfirmDeleteId] = useState<string | null>(null);
  const active =
    collections.find((entry) => entry.id === activeId) ?? collections[0] ?? null;
  const total = collections.reduce((sum, entry) => sum + entry.seeds.length, 0);

  useEffect(() => setConfirmDeleteId(null), [active?.id]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, [onClose]);

  const create = () => {
    onCreate(draftName.trim() || "New collection");
    setDraftName("");
  };

  return (
    <>
      <div className="drawer-backdrop" onClick={onClose} />
      <aside className="drawer" role="dialog" aria-modal="true" aria-label="Collections">
        <header className="drawer-head">
          <div>
            <h2>Collections</h2>
            <p>
              {formatNumber(collections.length)} collections ·{" "}
              {formatNumber(total)} seeds
            </p>
          </div>
          <button type="button" className="icon-btn" onClick={onClose} aria-label="Close collections">
            <svg viewBox="0 0 20 20" width="16" height="16" aria-hidden="true">
              <path d="M5 5l10 10M15 5 5 15" stroke="currentColor" strokeWidth="1.5" />
            </svg>
          </button>
        </header>

        <div className="drawer-body">
          <div className="collection-new">
            <input
              value={draftName}
              onChange={(event) => setDraftName(event.currentTarget.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter") create();
              }}
              placeholder="New collection, e.g. Islands"
              aria-label="Name of the new collection"
            />
            <button type="button" className="btn-compact" onClick={create}>
              Create
            </button>
          </div>

          {collections.length === 0 ? (
            <p className="drawer-empty">
              No collections yet. Create one above or bookmark a hit – that
              creates “My collection” automatically.
            </p>
          ) : (
            <>
              <div className="collection-tabs" role="tablist" aria-label="Collections">
                {collections.map((collection) => (
                  <button
                    key={collection.id}
                    type="button"
                    role="tab"
                    aria-selected={collection.id === active?.id}
                    className={
                      collection.id === active?.id
                        ? "collection-tab is-active"
                        : "collection-tab"
                    }
                    onClick={() => onActivate(collection.id)}
                  >
                    <span>{collection.name}</span>
                    <em>{collection.seeds.length}</em>
                  </button>
                ))}
              </div>

              {active && (
                <section className="collection-active">
                  <div className="collection-head">
                    <TextField
                      value={active.name}
                      placeholder="Collection name"
                      ariaLabel="Collection name"
                      onSave={(name) => onRenameCollection(active.id, name)}
                    />
                    <button
                      type="button"
                      className="link danger"
                      onClick={() => {
                        if (confirmDeleteId === active.id) {
                          onDeleteCollection(active.id);
                          setConfirmDeleteId(null);
                        } else {
                          setConfirmDeleteId(active.id);
                        }
                      }}
                    >
                      {confirmDeleteId === active.id
                        ? "Really delete?"
                        : "Delete"}
                    </button>
                  </div>

                  {active.seeds.length === 0 && (
                    <p className="drawer-empty">
                      This collection is empty. Hits appear here once you
                      bookmark them.
                    </p>
                  )}

                  {active.seeds.map((item) => (
                    <article className="saved" key={`${item.version}-${item.seed}`}>
                      <TextField
                        value={item.name}
                        placeholder="Name, e.g. Island base"
                        ariaLabel={`Name of seed ${item.seed}`}
                        onSave={(name) => onSeedChange(active.id, item, { name })}
                      />
                      <div className="saved-head">
                        <code>{item.seed}</code>
                        <span className="saved-version">{item.version}</span>
                      </div>
                      <p className="saved-meta">
                        {prettify(item.spawn_biome)} · {formatNumber(item.spawn_x)} /{" "}
                        {formatNumber(item.spawn_z)}
                      </p>
                      <TextField
                        value={item.note}
                        placeholder="Note, e.g. Village by the river"
                        ariaLabel={`Note for seed ${item.seed}`}
                        multiline
                        onSave={(note) => onSeedChange(active.id, item, { note })}
                      />
                      <div className="saved-actions">
                        <button type="button" className="link" onClick={() => onLoad(item)}>
                          Open on map
                        </button>
                        <button
                          type="button"
                          className="link danger"
                          onClick={() => onRemoveSeed(active.id, item)}
                        >
                          Remove
                        </button>
                      </div>
                    </article>
                  ))}
                </section>
              )}
            </>
          )}
        </div>
      </aside>
    </>
  );
}

function TextField({
  value,
  onSave,
  placeholder,
  ariaLabel,
  multiline = false,
}: {
  value: string;
  onSave: (value: string) => void;
  placeholder: string;
  ariaLabel: string;
  multiline?: boolean;
}) {
  const [draft, setDraft] = useState(value);

  useEffect(() => setDraft(value), [value]);

  const commit = () => {
    const next = draft.trim();
    if (next === value) return;
    if (!next) {
      setDraft(value);
      return;
    }
    onSave(next);
  };

  return (
    <label className={multiline ? "note" : "name-field"}>
      <span>{multiline ? "Note" : "Name"}</span>
      {multiline ? (
        <textarea
          rows={2}
          value={draft}
          placeholder={placeholder}
          aria-label={ariaLabel}
          onChange={(event) => setDraft(event.currentTarget.value)}
          onBlur={commit}
        />
      ) : (
        <input
          value={draft}
          placeholder={placeholder}
          aria-label={ariaLabel}
          onChange={(event) => setDraft(event.currentTarget.value)}
          onBlur={commit}
        />
      )}
    </label>
  );
}
