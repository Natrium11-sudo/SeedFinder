/** Terrain palette for the biome map. Unknown ids fall back to a stable hash. */

const COLORS: Record<string, string> = {
  ocean: "#3d6b86",
  deep_ocean: "#2b5170",
  warm_ocean: "#3f7f9e",
  lukewarm_ocean: "#447d97",
  cold_ocean: "#3a5f7d",
  deep_cold_ocean: "#2f4f68",
  deep_frozen_ocean: "#3b5666",
  frozen_ocean: "#4a6878",
  river: "#4e81a1",
  frozen_river: "#7093a8",
  beach: "#d8cba0",
  snowy_beach: "#e2e3da",
  stony_shore: "#8b8d84",
  plains: "#8fae62",
  sunflower_plains: "#a9ba5e",
  forest: "#5e8d4e",
  flower_forest: "#7ba85c",
  birch_forest: "#8dab71",
  old_growth_birch_forest: "#7fa063",
  dark_forest: "#3f6039",
  taiga: "#50795b",
  snowy_taiga: "#7d998d",
  old_growth_pine_taiga: "#4b6b52",
  old_growth_spruce_taiga: "#55705a",
  windswept_hills: "#7c8474",
  windswept_forest: "#6e8468",
  windswept_gravelly_hills: "#8b8d84",
  windswept_savanna: "#a7995f",
  jungle: "#2f7a3f",
  sparse_jungle: "#4f8a46",
  bamboo_jungle: "#4c8f3d",
  swamp: "#566b3f",
  mangrove_swamp: "#4a6b4e",
  desert: "#d8c58c",
  savanna: "#b5a55f",
  savanna_plateau: "#a99b5a",
  badlands: "#a9774e",
  eroded_badlands: "#b77b53",
  wooded_badlands: "#9a6b4a",
  snowy_plains: "#e4e7e2",
  ice_spikes: "#d7e4e8",
  mushroom_fields: "#9c8a9e",
  stony_peaks: "#9aa1a0",
  jagged_peaks: "#b9bfbd",
  frozen_peaks: "#d5dedc",
  snowy_slopes: "#dde5e3",
  grove: "#78907e",
  meadow: "#8fb26b",
  cherry_grove: "#e2b7c4",
  pale_garden: "#7c8b82",
  deep_dark: "#2a3b3a",
  dripstone_caves: "#8a7b63",
  lush_caves: "#4e7c4a",
  sulfur_caves: "#b9a94e",
  dappled_forest: "#6e8c55",
  the_void: "#101010",
};

const FALLBACKS = [
  "#6f8f63",
  "#8a9c6a",
  "#5d7f74",
  "#a08a62",
  "#7b6f8c",
  "#8c7d6c",
];

export function biomeColor(name: string | undefined): string {
  if (!name) return "#b9bfb2";
  const known = COLORS[name];
  if (known) return known;

  let hash = 0;
  for (let i = 0; i < name.length; i += 1) {
    hash = (hash * 31 + name.charCodeAt(i)) | 0;
  }
  return FALLBACKS[Math.abs(hash) % FALLBACKS.length];
}

export function shadeColor(hex: string, factor: number): string {
  const value = Number.parseInt(hex.slice(1), 16);
  const r = clamp(Math.round(((value >> 16) & 255) * factor));
  const g = clamp(Math.round(((value >> 8) & 255) * factor));
  const b = clamp(Math.round((value & 255) * factor));
  return `rgb(${r} ${g} ${b})`;
}

function clamp(value: number): number {
  return Math.min(255, Math.max(0, value));
}
