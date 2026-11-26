/** Stub for `@tauri-apps/api/core` used by the screenshot harness only. */
import fixture from "./data.json";

type Args = { request?: { cells?: number; scale?: number; layer?: string } };

export async function invoke<T>(command: string, args?: Args): Promise<T> {
  switch (command) {
    case "version_catalog":
      return fixture.versions as unknown as T;
    case "biome_catalog":
      return fixture.biomes as unknown as T;
    case "structure_catalog":
      return fixture.structures as unknown as T;
    case "collection_load":
      return fixture.collections as unknown as T;
    case "collection_save":
      return undefined as T;
    case "preview_seed": {
      const request = args?.request ?? {};
      if (request.layer === "underground") {
        return fixture.previewUnderground as unknown as T;
      }
      const overview = (request.cells ?? 128) > 128;
      return (overview
        ? fixture.previewOverview
        : fixture.previewDetail) as unknown as T;
    }
    default:
      throw new Error(`Screenshot harness has no mock for \`${command}\``);
  }
}
