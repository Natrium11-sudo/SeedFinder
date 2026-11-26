/** Colours and glyphs for the structure layer. */

const COLORS: Record<string, string> = {
  village: "#a2632c",
  outpost: "#7a5a2b",
  monument: "#2f6f8f",
  mansion: "#6b4526",
  desert_pyramid: "#c39a3f",
  jungle_temple: "#3f7d4a",
  swamp_hut: "#5c6a38",
  igloo: "#7f9aa6",
  ruined_portal: "#8a5a8f",
  ancient_city: "#3b4a5a",
  trial_chambers: "#b06a3c",
  abandoned_camp: "#8a7452",
  shipwreck: "#3d7a86",
  treasure: "#b0892f",
  stronghold: "#20261f",
};

export function structureColor(kind: string): string {
  return COLORS[kind] ?? "#4a5a4e";
}
