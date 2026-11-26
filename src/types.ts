export type VersionGroup =
  | "legacy"
  | "village_and_pillage"
  | "caves_and_cliffs"
  | "modern"
  | "calendar";

export type SpawnRule = "legacy" | "pre118" | "post118" | "post1212";

export interface VersionInfo {
  id: string;
  group: VersionGroup;
  group_label: string;
  surface_y: number;
  spawn_rule: SpawnRule;
  height_approximate: boolean;
}

export interface BiomeInfo {
  id: number;
  name: string;
  label: string;
}

export interface StructureInfo {
  id: string;
  label: string;
}

export type Rule =
  | { kind: "biome_at"; x: number; z: number; any_of: string[] }
  | { kind: "spawn_biome"; any_of: string[] }
  | { kind: "spawn_area"; biome: string; radius: number; min_share: number }
  | { kind: "structure_near"; structures: string[]; radius: number }
  | { kind: "island_spawn"; radius: number };

export interface StructureHit {
  kind: string;
  label: string;
  x: number;
  z: number;
}

export interface SeedHit {
  seed: string;
  spawn_x: number;
  spawn_z: number;
  spawn_biome: string;
  matched: string[];
  structures: StructureHit[];
  thumbnail: number[];
  thumbnail_size: number;
}

export interface SearchProgress {
  checked: number;
  hits: number;
  target: number;
  seeds_per_second: number;
  elapsed_ms: number;
  running: boolean;
}

export interface PreviewMarker {
  kind: string;
  label: string;
  x: number;
  z: number;
}

export interface SlimeChunk {
  x: number;
  z: number;
}

export interface PreviewData {
  seed: string;
  spawn_x: number;
  spawn_z: number;
  spawn_biome: string;
  origin_block_x: number;
  origin_block_z: number;
  cell_scale: number;
  width: number;
  height: number;
  /** Empty for coarse (1:16 / 1:64) maps, which skip hillshade. */
  heights: number[];
  biomes: number[];
  legend: BiomeInfo[];
  markers: PreviewMarker[];
  slime_chunks: SlimeChunk[];
  min_height: number;
  max_height: number;
  /** `surface` (top-down, no caves) or `underground` (cave biomes). */
  layer: "surface" | "underground";
  /** Block height the biome grid was sampled at. */
  sample_y: number;
  generated_ms: number;
}

export interface SavedSeed {
  seed: string;
  version: string;
  name: string;
  spawn_x: number;
  spawn_z: number;
  spawn_biome: string;
  note: string;
  added_at: number;
}

export interface SeedCollection {
  id: string;
  name: string;
  created_at: number;
  seeds: SavedSeed[];
}

/** Editable rule builder state. */
export interface RuleState {
  spawnBiomes: string[];
  area: {
    enabled: boolean;
    biome: string;
    radius: number;
    /** Percent, 1 - 100. */
    share: number;
  };
  coord: {
    enabled: boolean;
    x: number;
    z: number;
    biomes: string[];
  };
  structure: {
    enabled: boolean;
    /** Empty means the rule is inactive. */
    kinds: string[];
    radius: number;
  };
  island: {
    enabled: boolean;
    radius: number;
  };
}

export function buildRules(state: RuleState): Rule[] {
  const rules: Rule[] = [];

  if (state.spawnBiomes.length > 0) {
    rules.push({ kind: "spawn_biome", any_of: state.spawnBiomes });
  }
  if (state.area.enabled && state.area.biome) {
    rules.push({
      kind: "spawn_area",
      biome: state.area.biome,
      radius: state.area.radius,
      min_share: state.area.share / 100,
    });
  }
  if (state.coord.enabled && state.coord.biomes.length > 0) {
    rules.push({
      kind: "biome_at",
      x: state.coord.x,
      z: state.coord.z,
      any_of: state.coord.biomes,
    });
  }
  if (state.structure.enabled && state.structure.kinds.length > 0) {
    rules.push({
      kind: "structure_near",
      structures: state.structure.kinds,
      radius: state.structure.radius,
    });
  }
  if (state.island.enabled) {
    rules.push({ kind: "island_spawn", radius: state.island.radius });
  }

  return rules;
}

export const emptyRuleState: RuleState = {
  spawnBiomes: [],
  area: { enabled: false, biome: "", radius: 512, share: 25 },
  coord: { enabled: false, x: 0, z: 0, biomes: [] },
  structure: { enabled: false, kinds: [], radius: 1024 },
  island: { enabled: false, radius: 512 },
};
