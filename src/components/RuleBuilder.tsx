import type {
  BiomeInfo,
  KnownPortalSeed,
  RuleState,
  StructureInfo,
  SurfaceCategory,
} from "../types";
import { structureIconUrl } from "../lib/structureIcons";
import { BiomePicker } from "./BiomePicker";

interface RuleBuilderProps {
  state: RuleState;
  onChange: (next: RuleState) => void;
  biomes: BiomeInfo[];
  structures: StructureInfo[];
  portalSeeds: KnownPortalSeed[];
  onUseSeed: (seed: string, version: string) => void;
  disabled?: boolean;
}

export function RuleBuilder({
  state,
  onChange,
  biomes,
  structures,
  portalSeeds,
  onUseSeed,
  disabled = false,
}: RuleBuilderProps) {
  const patch = (part: Partial<RuleState>) => onChange({ ...state, ...part });
  const allKinds = structures.map((structure) => structure.id);
  const chosenKinds = state.structure.kinds;

  const toggleKind = (id: string) => {
    const kinds = chosenKinds.includes(id)
      ? chosenKinds.filter((kind) => kind !== id)
      : [...chosenKinds, id];
    patch({ structure: { ...state.structure, enabled: kinds.length > 0, kinds } });
  };

  return (
    <div className="rules">
      <section className={chosenKinds.length > 0 ? "rule is-on" : "rule"}>
        <div className="rule-head">
          <h3>Structures</h3>
          <span className="rule-note">
            {chosenKinds.length} of {structures.length} types
          </span>
        </div>
        <p className="rule-hint">
          Search directly for seeds whose spawn is near one or more selected
          structures.
        </p>

        <div className="structure-head">
          <span className="rule-note">
            {chosenKinds.length === 0
              ? "Nothing selected – structures do not filter"
              : "At least one selected type must be present"}
          </span>
          <span className="structure-actions">
            <button
              type="button"
              className="link"
              onClick={() =>
                patch({
                  structure: { ...state.structure, enabled: true, kinds: allKinds },
                })
              }
              disabled={disabled || chosenKinds.length === allKinds.length}
            >
              All
            </button>
            <button
              type="button"
              className="link"
              onClick={() =>
                patch({ structure: { ...state.structure, enabled: false, kinds: [] } })
              }
              disabled={disabled || chosenKinds.length === 0}
            >
              None
            </button>
          </span>
        </div>

        <div className="structure-grid" role="group" aria-label="Structures">
          {structures.map((structure) => {
            const checked = chosenKinds.includes(structure.id);
            const icon = structureIconUrl(structure.id);
            return (
              <label
                key={structure.id}
                className={checked ? "structure-choice is-on" : "structure-choice"}
              >
                <input
                  type="checkbox"
                  checked={checked}
                  onChange={() => toggleKind(structure.id)}
                  disabled={disabled}
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

        <label className="field">
          <span>Radius around spawn</span>
          <span className="input-with-unit">
            <input
              type="number"
              min={128}
              max={8192}
              step={128}
              value={state.structure.radius}
              onChange={(event) =>
                patch({
                  structure: {
                    ...state.structure,
                    radius: clampInt(
                      event.currentTarget.value,
                      128,
                      8192,
                      state.structure.radius,
                    ),
                  },
                })
              }
              disabled={disabled}
            />
            <em>blocks</em>
          </span>
        </label>
        <p className="rule-hint">
          An empty selection does not filter. The structure found is listed
          under each hit.
        </p>
      </section>

      <section className="rule">
        <div className="rule-head">
          <h3>Biome at spawn</h3>
          <span className="rule-note">at least one</span>
        </div>
        <BiomePicker
          label="Biome at spawn"
          options={biomes}
          value={state.spawnBiomes}
          onChange={(spawnBiomes) => patch({ spawnBiomes })}
          placeholder="Allow all biomes"
          disabled={disabled}
        />
        <p className="rule-hint">Leave empty if the spawn does not matter.</p>
      </section>

      <section className={state.island.enabled ? "rule is-on" : "rule"}>
        <label className="switch">
          <input
            type="checkbox"
            checked={state.island.enabled}
            onChange={(event) =>
              patch({
                island: { ...state.island, enabled: event.currentTarget.checked },
              })
            }
            disabled={disabled}
          />
          <span>Island spawn</span>
        </label>

        {state.island.enabled && (
          <div className="rule-body">
            <label className="field">
              <span>Water within</span>
              <span className="input-with-unit">
                <input
                  type="number"
                  min={128}
                  max={2048}
                  step={128}
                  value={state.island.radius}
                  onChange={(event) =>
                    patch({
                      island: {
                        ...state.island,
                        radius: clampInt(
                          event.currentTarget.value,
                          128,
                          2048,
                          state.island.radius,
                        ),
                      },
                    })
                  }
                  disabled={disabled}
                />
                <em>blocks</em>
              </span>
            </label>
            <p className="rule-hint">
              The spawn must be on land that reaches water in all eight
              directions within this radius.
            </p>
          </div>
        )}
      </section>

      <section className={state.area.enabled ? "rule is-on" : "rule"}>
        <label className="switch">
          <input
            type="checkbox"
            checked={state.area.enabled}
            onChange={(event) =>
              patch({
                area: { ...state.area, enabled: event.currentTarget.checked },
              })
            }
            disabled={disabled}
          />
          <span>Biome in radius</span>
        </label>

        {state.area.enabled && (
          <div className="rule-body">
            <BiomePicker
              label="Biome in radius"
              options={biomes}
              value={state.area.biome ? [state.area.biome] : []}
              onChange={(next) =>
                patch({ area: { ...state.area, biome: next[0] ?? "" } })
              }
              placeholder="Choose a biome"
              single
              disabled={disabled}
            />
            <div className="field-row">
              <label className="field">
                <span>Radius</span>
                <span className="input-with-unit">
                  <input
                    type="number"
                    min={64}
                    max={8192}
                    step={64}
                    value={state.area.radius}
                    onChange={(event) =>
                      patch({
                        area: {
                          ...state.area,
                          radius: clampInt(event.currentTarget.value, 64, 8192, state.area.radius),
                        },
                      })
                    }
                    disabled={disabled}
                  />
                  <em>blocks</em>
                </span>
              </label>
              <label className="field">
                <span>Share: {state.area.share} %</span>
                <input
                  type="range"
                  min={5}
                  max={100}
                  step={5}
                  value={state.area.share}
                  onChange={(event) =>
                    patch({
                      area: {
                        ...state.area,
                        share: Number(event.currentTarget.value),
                      },
                    })
                  }
                  disabled={disabled}
                />
              </label>
            </div>
          </div>
        )}
      </section>

      <section className={state.coord.enabled ? "rule is-on" : "rule"}>
        <label className="switch">
          <input
            type="checkbox"
            checked={state.coord.enabled}
            onChange={(event) =>
              patch({
                coord: { ...state.coord, enabled: event.currentTarget.checked },
              })
            }
            disabled={disabled}
          />
          <span>Biome at coordinate</span>
        </label>

        {state.coord.enabled && (
          <div className="rule-body">
            <div className="field-row">
              <label className="field">
                <span>X</span>
                <input
                  type="number"
                  step={16}
                  value={state.coord.x}
                  onChange={(event) =>
                    patch({
                      coord: {
                        ...state.coord,
                        x: Number(event.currentTarget.value) || 0,
                      },
                    })
                  }
                  disabled={disabled}
                />
              </label>
              <label className="field">
                <span>Z</span>
                <input
                  type="number"
                  step={16}
                  value={state.coord.z}
                  onChange={(event) =>
                    patch({
                      coord: {
                        ...state.coord,
                        z: Number(event.currentTarget.value) || 0,
                      },
                    })
                  }
                  disabled={disabled}
                />
              </label>
            </div>
            <BiomePicker
              label="Biome at coordinate"
              options={biomes}
              value={state.coord.biomes}
              onChange={(coordBiomes) =>
                patch({ coord: { ...state.coord, biomes: coordBiomes } })
              }
              placeholder="Choose a biome"
              disabled={disabled}
            />
          </div>
        )}
      </section>

      <section className={state.spawnSurface !== "any" ? "rule is-on" : "rule"}>
        <label className="field">
          <span>Spawn surface (approximate)</span>
          <select
            value={state.spawnSurface}
            onChange={(event) =>
              patch({ spawnSurface: event.currentTarget.value as SurfaceCategory | "any" })
            }
            disabled={disabled}
          >
            <option value="any">Any surface</option>
            <option value="grass">Grass-like</option>
            <option value="sand">Sand / desert</option>
            <option value="snow">Snow / ice</option>
            <option value="stone">Stone / mountains</option>
            <option value="water">Water / ocean</option>
          </select>
        </label>
        <p className="rule-hint">
          Approximate only: derived from the biome at spawn. Cubiomes does not
          expose individual block ids, so this cannot identify an exact block.
        </p>
      </section>

      <section className={state.portal.enabled ? "rule is-on" : "rule"}>
        <label className="switch">
          <input
            type="checkbox"
            checked={state.portal.enabled}
            onChange={(event) =>
              patch({
                portal: { ...state.portal, enabled: event.currentTarget.checked },
              })
            }
            disabled={disabled}
          />
          <span>End portal eyes</span>
        </label>

        {state.portal.enabled && (
          <div className="rule-body">
            <label className="field">
              <span>Minimum eyes: {state.portal.minEyes} / 12</span>
              <input
                type="range"
                min={1}
                max={12}
                step={1}
                value={state.portal.minEyes}
                onChange={(event) =>
                  patch({
                    portal: {
                      ...state.portal,
                      minEyes: Number(event.currentTarget.value),
                    },
                  })
                }
                disabled={disabled}
              />
            </label>
            <div className="field-row">
              <label className="switch">
                <input
                  type="checkbox"
                  checked={state.portal.requireLit}
                  onChange={(event) =>
                    patch({
                      portal: {
                        ...state.portal,
                        requireLit: event.currentTarget.checked,
                      },
                    })
                  }
                  disabled={disabled}
                />
                <span>Portal must be active</span>
              </label>
              <label className="field">
                <span>Strongholds to check</span>
                <input
                  type="number"
                  min={1}
                  max={8}
                  value={state.portal.maxStrongholds}
                  onChange={(event) =>
                    patch({
                      portal: {
                        ...state.portal,
                        maxStrongholds: clampInt(
                          event.currentTarget.value,
                          1,
                          8,
                          state.portal.maxStrongholds,
                        ),
                      },
                    })
                  }
                  disabled={disabled}
                />
              </label>
            </div>
            <p className="rule-hint">
              Rare: 4+ eyes ≈ 1 in 47 seeds, 5+ ≈ 1 in 231, 6+ ≈ 1 in 1,848.
              This rule runs last because each stronghold is expensive.
            </p>
            {state.portal.minEyes >= 12 && (
              <div className="rule-hint portal-warning">
                12 eyes occur about once per trillion seeds – a random scan will
                not find one. Use a verified seed below, or lower the threshold
                to search for a partial portal.
              </div>
            )}
            <div className="portal-seeds">
              {portalSeeds.map((preset) => (
                <button
                  key={`${preset.version}:${preset.seed}`}
                  type="button"
                  className="link"
                  onClick={() => onUseSeed(preset.seed, preset.version)}
                  disabled={disabled}
                  title={`${preset.label} (Minecraft ${preset.version})`}
                >
                  {preset.seed} · {preset.version}
                </button>
              ))}
            </div>
          </div>
        )}
      </section>

      <button
        type="button"
        className="link reset"
        onClick={() =>
          onChange({
            spawnBiomes: [],
            area: { ...state.area, enabled: false, biome: "" },
            coord: { ...state.coord, enabled: false, biomes: [] },
            structure: { ...state.structure, enabled: false, kinds: [] },
            island: { ...state.island, enabled: false },
            spawnSurface: "any",
            portal: { ...state.portal, enabled: false },
          })
        }
        disabled={disabled}
      >
        Reset all rules
      </button>
    </div>
  );
}

function clampInt(raw: string, min: number, max: number, fallback: number): number {
  const value = Number.parseInt(raw, 10);
  if (Number.isNaN(value)) return fallback;
  return Math.min(max, Math.max(min, value));
}
