/** Bundled Minecraft sprites used for the structure layer. */

import village from "../assets/structures/village.png";
import outpost from "../assets/structures/outpost.png";
import monument from "../assets/structures/monument.png";
import mansion from "../assets/structures/mansion.png";
import desertPyramid from "../assets/structures/desert_pyramid.png";
import jungleTemple from "../assets/structures/jungle_temple.png";
import swampHut from "../assets/structures/swamp_hut.png";
import igloo from "../assets/structures/igloo.png";
import ruinedPortal from "../assets/structures/ruined_portal.png";
import ancientCity from "../assets/structures/ancient_city.png";
import trialChambers from "../assets/structures/trial_chambers.png";
import abandonedCamp from "../assets/structures/abandoned_camp.png";
import shipwreck from "../assets/structures/shipwreck.png";
import treasure from "../assets/structures/treasure.png";
import stronghold from "../assets/structures/stronghold.png";

const URLS: Record<string, string> = {
  village,
  outpost,
  monument,
  mansion,
  desert_pyramid: desertPyramid,
  jungle_temple: jungleTemple,
  swamp_hut: swampHut,
  igloo,
  ruined_portal: ruinedPortal,
  ancient_city: ancientCity,
  trial_chambers: trialChambers,
  abandoned_camp: abandonedCamp,
  shipwreck,
  treasure,
  stronghold,
};

const images = new Map<string, HTMLImageElement>();
const listeners = new Set<() => void>();

function load(kind: string): HTMLImageElement | null {
  const url = URLS[kind];
  if (!url) return null;

  let image = images.get(kind);
  if (!image) {
    image = new Image();
    image.addEventListener("load", () => {
      for (const listener of listeners) listener();
    });
    image.src = url;
    images.set(kind, image);
  }
  return image;
}

/** The ready-to-draw sprite, or null while it is still loading. */
export function structureIcon(kind: string): HTMLImageElement | null {
  const image = load(kind);
  if (!image || !image.complete || image.naturalWidth === 0) return null;
  return image;
}

export function structureIconUrl(kind: string): string | null {
  return URLS[kind] ?? null;
}

/** Starts loading every sprite and calls back whenever one becomes ready. */
export function onStructureIconsLoaded(listener: () => void): () => void {
  listeners.add(listener);
  for (const kind of Object.keys(URLS)) load(kind);
  return () => {
    listeners.delete(listener);
  };
}
