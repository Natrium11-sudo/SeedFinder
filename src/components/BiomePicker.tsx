import { useEffect, useMemo, useRef, useState } from "react";
import type { BiomeInfo } from "../types";
import { prettify } from "../lib/format";

interface BiomePickerProps {
  label: string;
  options: BiomeInfo[];
  value: string[];
  onChange: (next: string[]) => void;
  placeholder: string;
  single?: boolean;
  disabled?: boolean;
}

export function BiomePicker({
  label,
  options,
  value,
  onChange,
  placeholder,
  single = false,
  disabled = false,
}: BiomePickerProps) {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const rootRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;

    const onPointerDown = (event: MouseEvent) => {
      if (!rootRef.current?.contains(event.target as Node)) setOpen(false);
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") setOpen(false);
    };

    document.addEventListener("mousedown", onPointerDown);
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("mousedown", onPointerDown);
      document.removeEventListener("keydown", onKeyDown);
    };
  }, [open]);

  const labels = useMemo(() => {
    const map = new Map<string, string>();
    for (const biome of options) map.set(biome.name, biome.label);
    return map;
  }, [options]);

  const filtered = useMemo(() => {
    const needle = query.trim().toLowerCase();
    if (!needle) return options;
    return options.filter(
      (biome) =>
        biome.label.toLowerCase().includes(needle) ||
        biome.name.toLowerCase().includes(needle),
    );
  }, [options, query]);

  const summary = single
    ? value.length > 0
      ? (labels.get(value[0]) ?? prettify(value[0]))
      : placeholder
    : value.length === 0
      ? placeholder
      : value.length === 1
        ? (labels.get(value[0]) ?? prettify(value[0]))
        : `${value.length} biomes selected`;

  const toggle = (name: string) => {
    if (single) {
      onChange([name]);
      setOpen(false);
      setQuery("");
      return;
    }
    onChange(
      value.includes(name)
        ? value.filter((entry) => entry !== name)
        : [...value, name],
    );
  };

  return (
    <div className="picker" ref={rootRef}>
      <button
        type="button"
        className="picker-trigger"
        onClick={() => setOpen((current) => !current)}
        aria-expanded={open}
        aria-label={label}
        disabled={disabled}
      >
        <span className={value.length > 0 ? "picker-value" : "picker-value is-empty"}>
          {summary}
        </span>
        <span className="picker-caret" aria-hidden="true">
          ▾
        </span>
      </button>

      {!single && value.length > 1 && (
        <div className="chips">
          {value.map((name) => (
            <button
              key={name}
              type="button"
              className="chip"
              onClick={() => onChange(value.filter((entry) => entry !== name))}
              title={`Remove ${labels.get(name) ?? prettify(name)}`}
            >
              {labels.get(name) ?? prettify(name)}
              <span aria-hidden="true">×</span>
            </button>
          ))}
        </div>
      )}

      {open && (
        <div className="popover" role="dialog" aria-label={label}>
          <input
            className="popover-search"
            placeholder="Filter …"
            value={query}
            onChange={(event) => setQuery(event.currentTarget.value)}
            autoFocus
          />
          <div className="popover-list">
            {filtered.map((biome) => {
              const active = value.includes(biome.name);
              return (
                <button
                  key={biome.name}
                  type="button"
                  className={active ? "option is-active" : "option"}
                  aria-pressed={active}
                  onClick={() => toggle(biome.name)}
                >
                  <span>{biome.label}</span>
                  {active && (
                    <span className="option-check" aria-hidden="true">
                      ✓
                    </span>
                  )}
                </button>
              );
            })}
            {filtered.length === 0 && (
              <p className="popover-empty">No biome found.</p>
            )}
          </div>
          <div className="popover-footer">
            {!single && (
              <button type="button" className="link" onClick={() => onChange([])}>
                Clear selection
              </button>
            )}
            <button type="button" className="link" onClick={() => setOpen(false)}>
              Done
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
