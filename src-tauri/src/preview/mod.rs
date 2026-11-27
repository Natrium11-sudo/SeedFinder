//! Preview pipeline: one biome (+ optional height) grid per seed.
//!
//! The UI uses three sampling scales: 1:4 for the detail view (with hillshade),
//! 1:16 and 1:64 for zooming out. Coarse views skip height generation so a
//! 16k-block overview stays fast. Structures, strongholds and slime chunks are
//! collected for the visible rectangle.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::engine::catalog::{self, BiomeInfo, StructureKind};
use crate::engine::generator::SeedGenerator;
use crate::engine::version::{self, McVersion};

/// Request from the map view.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewRequest {
    pub version: String,
    /// Decimal seed string (64-bit safe through JSON).
    pub seed: String,
    pub center_x: Option<i32>,
    pub center_z: Option<i32>,
    /// Grid edge length in cells.
    pub cells: Option<u32>,
    /// Blocks per cell: 4 (detail), 16 or 64 (overview).
    pub scale: Option<u32>,
    /// Structure slugs to mark on the map.
    #[serde(default)]
    pub structures: Vec<String>,
    /// Show stronghold markers.
    pub strongholds: Option<bool>,
    /// Show slime chunk markers.
    pub slime: Option<bool>,
    /// Map layer: `surface` (top-down, no caves) or `underground` (cave biomes).
    #[serde(default)]
    pub layer: Option<String>,
    /// Block height sampled by the underground layer.
    #[serde(default)]
    pub depth: Option<i32>,
}

/// A structure marker inside the preview area.
#[derive(Debug, Clone, Serialize)]
pub struct PreviewMarker {
    pub kind: String,
    pub label: String,
    pub x: i32,
    pub z: i32,
}

/// One slime chunk (chunk coordinates).
#[derive(Debug, Clone, Serialize)]
pub struct SlimeChunk {
    pub x: i32,
    pub z: i32,
}

/// Serializable map payload.
///
/// Field names stay snake_case: they mirror `src/types.ts` and the other
/// payloads (`SeedHit`, `BiomeInfo`). A mismatch here leaves the map blank.
#[derive(Debug, Clone, Serialize)]
pub struct PreviewData {
    pub seed: String,
    pub spawn_x: i32,
    pub spawn_z: i32,
    pub spawn_biome: String,
    /// Block coordinate of the grid's north-west corner.
    pub origin_block_x: i32,
    pub origin_block_z: i32,
    pub cell_scale: u32,
    pub width: u32,
    pub height: u32,
    /// Row-major approximated surface heights in blocks; empty for coarse maps.
    pub heights: Vec<f32>,
    /// Row-major biome ids.
    pub biomes: Vec<i32>,
    pub legend: Vec<BiomeInfo>,
    pub markers: Vec<PreviewMarker>,
    pub slime_chunks: Vec<SlimeChunk>,
    pub min_height: f32,
    pub max_height: f32,
    /// `surface` or `underground`, matching the request.
    pub layer: String,
    /// Block height the grid was sampled at.
    pub sample_y: i32,
    pub generated_ms: u64,
}

/// Which height the biome grid is sampled at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MapLayer {
    /// Top-down surface: the world as seen from above, without cave biomes.
    Surface,
    /// Fixed block height below the surface, where cave biomes exist.
    Underground,
}

impl MapLayer {
    const fn slug(self) -> &'static str {
        match self {
            Self::Surface => "surface",
            Self::Underground => "underground",
        }
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "surface" => Ok(Self::Surface),
            "underground" => Ok(Self::Underground),
            other => Err(format!(
                "Unknown map layer: {other} (allowed: surface, underground)."
            )),
        }
    }
}

/// Overworld build limits; the underground layer may sample any of them.
const DEPTH_MIN: i32 = -64;
const DEPTH_MAX: i32 = 320;

/// Caps how many markers of one kind are drawn, evenly thinned over the scan.
const MARKERS_PER_KIND: usize = 60;
const MARKERS_TOTAL: usize = 420;
const SLIME_TOTAL: usize = 5000;

/// Builds the preview grid for a seed.
pub fn build(request: &PreviewRequest) -> Result<PreviewData, String> {
    let profile = version::by_id(&request.version)
        .ok_or_else(|| format!("Unknown Minecraft version: {}", request.version))?;

    let seed = request
        .seed
        .trim()
        .parse::<i64>()
        .map_err(|_| format!("Invalid seed: {}", request.seed))?;
    let cells = request.cells.unwrap_or(128).clamp(24, 256);
    let scale = request.scale.unwrap_or(4);
    if !matches!(scale, 4 | 16 | 64) {
        return Err(format!("Invalid map scale {scale} (allowed: 4, 16 or 64)."));
    }
    let layer = MapLayer::parse(request.layer.as_deref().unwrap_or("surface"))?;
    let depth = request.depth.unwrap_or(0).clamp(DEPTH_MIN, DEPTH_MAX);
    let mut generator = SeedGenerator::new(profile.mc, seed).map_err(|err| err.to_string())?;

    let spawn = generator.spawn();
    let center_x = request.center_x.unwrap_or(spawn.0);
    let center_z = request.center_z.unwrap_or(spawn.1);

    let started = Instant::now();
    let cell_scale = scale as i32;
    let span = cells as i32 * cell_scale;
    let origin_block_x = (center_x - span / 2).div_euclid(cell_scale) * cell_scale;
    let origin_block_z = (center_z - span / 2).div_euclid(cell_scale) * cell_scale;

    let (heights, biomes, min_height, max_height, sample_y) = match (layer, scale) {
        // Detail view of the surface: heights plus the biome at that height.
        (MapLayer::Surface, 4) => {
            let mut grid = generator
                .height_grid(
                    origin_block_x.div_euclid(4),
                    origin_block_z.div_euclid(4),
                    cells,
                    cells,
                )
                .map_err(|err| err.to_string())?;
            // `mapApproxHeight()` samples its biome ids at the approximated
            // surface height (around Y 60-100), where 1.18+ cave biomes live.
            // The surface layer must show the top-down biome instead, just
            // like the coarse layers do.
            grid.biomes = generator
                .biome_grid(
                    4,
                    origin_block_x.div_euclid(4),
                    origin_block_z.div_euclid(4),
                    cells,
                    cells,
                )
                .map_err(|err| err.to_string())?;
            let stats = grid.stats();
            (
                grid.heights,
                grid.biomes,
                stats.map_or(0.0, |(min, _, _)| min),
                stats.map_or(0.0, |(_, max, _)| max),
                profile.surface_y,
            )
        }
        // Coarse surface view: top of the world, hillshade skipped.
        // `genBiomes()` at 1:64 can return cave biomes on 1.18+, so sample
        // each cell centre at the build limit instead.
        (MapLayer::Surface, _) => {
            let grid = generator
                .biome_grid_top(scale, origin_block_x, origin_block_z, cells, cells)
                .map_err(|err| err.to_string())?;
            (Vec::new(), grid, 0.0, 0.0, profile.surface_y)
        }
        // Underground: one fixed height, flat shading, cave biomes included.
        (MapLayer::Underground, _) => {
            let grid = generator
                .biome_grid_at(
                    scale,
                    depth,
                    origin_block_x.div_euclid(cell_scale),
                    origin_block_z.div_euclid(cell_scale),
                    cells,
                    cells,
                )
                .map_err(|err| err.to_string())?;
            (Vec::new(), grid, 0.0, 0.0, depth)
        }
    };

    let spawn_biome = generator
        .biome_at(
            4,
            spawn.0.div_euclid(4),
            profile.surface_y,
            spawn.1.div_euclid(4),
        )
        .ok()
        .and_then(|id| catalog::biome_name(profile.mc, id))
        .unwrap_or_else(|| "unbekannt".to_owned());

    let mut legend = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for &id in &biomes {
        if id >= 0 && seen.insert(id) {
            if let Some(name) = catalog::biome_name(profile.mc, id) {
                legend.push(BiomeInfo {
                    id,
                    label: catalog::prettify(&name),
                    name,
                });
            }
        }
    }
    legend.sort_by(|a, b| a.label.cmp(&b.label));

    let x1 = origin_block_x + span - 1;
    let z1 = origin_block_z + span - 1;
    let mut markers = Vec::new();
    for slug in &request.structures {
        if markers.len() >= MARKERS_TOTAL {
            break;
        }
        let Some(kind) = StructureKind::from_slug(slug) else {
            continue;
        };
        let found = generator.structure_area(
            kind,
            origin_block_x,
            origin_block_z,
            x1,
            z1,
            MARKERS_PER_KIND * 8,
        );
        for (x, z) in thin(found, MARKERS_PER_KIND) {
            markers.push(PreviewMarker {
                kind: kind.slug().to_owned(),
                label: kind.label().to_owned(),
                x,
                z,
            });
            if markers.len() >= MARKERS_TOTAL {
                break;
            }
        }
    }

    if request.strongholds.unwrap_or(false) && markers.len() < MARKERS_TOTAL {
        let found = strongholds_for(profile.mc, seed)
            .into_iter()
            .filter(|&(x, z)| x >= origin_block_x && x <= x1 && z >= origin_block_z && z <= z1)
            .take(MARKERS_TOTAL - markers.len());
        for (x, z) in found {
            markers.push(PreviewMarker {
                kind: "stronghold".to_owned(),
                label: "Festung".to_owned(),
                x,
                z,
            });
        }
    }

    let mut slime_chunks = Vec::new();
    if request.slime.unwrap_or(false) && scale <= 16 {
        let cx0 = origin_block_x.div_euclid(16);
        let cz0 = origin_block_z.div_euclid(16);
        let chunks = (span / 16).clamp(1, 1024);
        'scan: for dz in 0..chunks {
            for dx in 0..chunks {
                if generator.is_slime_chunk(cx0 + dx, cz0 + dz) {
                    slime_chunks.push(SlimeChunk {
                        x: cx0 + dx,
                        z: cz0 + dz,
                    });
                    if slime_chunks.len() >= SLIME_TOTAL {
                        break 'scan;
                    }
                }
            }
        }
    }

    Ok(PreviewData {
        seed: seed.to_string(),
        spawn_x: spawn.0,
        spawn_z: spawn.1,
        spawn_biome,
        origin_block_x,
        origin_block_z,
        cell_scale: scale,
        width: cells,
        height: cells,
        heights,
        biomes,
        legend,
        markers,
        slime_chunks,
        min_height,
        max_height,
        layer: layer.slug().to_owned(),
        sample_y,
        generated_ms: started.elapsed().as_millis() as u64,
    })
}

/// All strongholds of a seed, computed once and reused while panning.
///
/// A full ring iteration costs roughly half a second because every stronghold
/// needs a biome check, so the map must not repeat it for each viewport.
fn strongholds_for(mc: McVersion, seed: i64) -> Vec<(i32, i32)> {
    type Cache = Mutex<HashMap<(i32, i64), Vec<(i32, i32)>>>;
    static CACHE: OnceLock<Cache> = OnceLock::new();

    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let key = (mc.cubiomes_id(), seed);
    if let Some(found) = cache
        .lock()
        .ok()
        .and_then(|entries| entries.get(&key).cloned())
    {
        return found;
    }

    let Ok(generator) = SeedGenerator::new(mc, seed) else {
        return Vec::new();
    };
    let found = generator.strongholds(-30_000_000, -30_000_000, 30_000_000, 30_000_000, 128);

    if let Ok(mut entries) = cache.lock() {
        if entries.len() >= 32 {
            entries.clear();
        }
        entries.insert(key, found.clone());
    }
    found
}

/// Evenly reduces a scan result so a crowded kind does not cluster in one band.
fn thin(points: Vec<(i32, i32)>, limit: usize) -> Vec<(i32, i32)> {
    if points.len() <= limit {
        return points;
    }
    let step = points.len() as f64 / limit as f64;
    (0..limit)
        .filter_map(|index| points.get((index as f64 * step) as usize).copied())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(cells: u32, scale: u32) -> PreviewRequest {
        PreviewRequest {
            version: "26.3".into(),
            seed: "3055141959546".into(),
            center_x: None,
            center_z: None,
            cells: Some(cells),
            scale: Some(scale),
            structures: Vec::new(),
            strongholds: None,
            slime: None,
            layer: None,
            depth: None,
        }
    }

    #[test]
    fn preview_request_accepts_the_ui_payload() {
        let parsed: PreviewRequest = serde_json::from_value(serde_json::json!({
            "version": "26.3",
            "seed": "3055141959546",
            "cells": 64,
            "scale": 16,
            "structures": ["village"],
            "strongholds": true,
            "slime": false
        }))
        .expect("preview request must deserialize");

        assert_eq!(parsed.seed, "3055141959546");
        assert_eq!(parsed.cells, Some(64));
        assert_eq!(parsed.scale, Some(16));
        assert!(parsed.center_x.is_none());
        assert_eq!(parsed.structures, vec!["village"]);
        assert_eq!(parsed.strongholds, Some(true));
    }

    #[test]
    fn preview_payload_matches_the_ui_contract() {
        let data = build(&request(32, 4)).expect("preview must generate");
        let value = serde_json::to_value(&data).expect("preview must serialize");

        for key in [
            "seed",
            "spawn_x",
            "spawn_z",
            "spawn_biome",
            "origin_block_x",
            "origin_block_z",
            "cell_scale",
            "width",
            "height",
            "heights",
            "biomes",
            "legend",
            "markers",
            "slime_chunks",
            "min_height",
            "max_height",
            "layer",
            "sample_y",
            "generated_ms",
        ] {
            assert!(
                value.get(key).is_some(),
                "the UI reads `{key}`, but the payload does not contain it"
            );
        }
        assert!(
            value.get("spawnX").is_none(),
            "payload must not be camelCase"
        );
        assert!(value
            .get("spawn_biome")
            .is_some_and(serde_json::Value::is_string));
        assert!(value
            .get("biomes")
            .is_some_and(|grid| grid.as_array().is_some_and(|cells| cells.len() == 32 * 32)));
    }

    #[test]
    fn coarse_map_uses_biomes_only_and_covers_the_requested_span() {
        let data = build(&request(64, 64)).expect("coarse preview must generate");

        assert_eq!(data.cell_scale, 64);
        assert_eq!(data.width, 64);
        assert_eq!(data.biomes.len(), 64 * 64);
        assert!(data.heights.is_empty(), "coarse maps skip hillshade");
        assert_eq!(data.origin_block_x.rem_euclid(64), 0);
        assert!(data.biomes.iter().any(|&id| id >= 0));
        assert!(!data.legend.is_empty());
    }

    #[test]
    fn structures_and_slime_stay_inside_the_area() {
        let mut req = request(64, 4);
        req.structures = vec!["village".into()];
        req.slime = Some(true);
        req.strongholds = Some(true);
        let data = build(&req).expect("layers must generate");

        let span = 64 * 4;
        for marker in &data.markers {
            assert!(marker.x >= data.origin_block_x);
            assert!(marker.x < data.origin_block_x + span);
            assert!(marker.z >= data.origin_block_z);
            assert!(marker.z < data.origin_block_z + span);
        }
        let chunk_span = span / 16;
        for chunk in &data.slime_chunks {
            assert!(chunk.x >= data.origin_block_x.div_euclid(16));
            assert!(chunk.x < data.origin_block_x.div_euclid(16) + chunk_span);
        }
    }

    #[test]
    fn coarse_maps_work_for_legacy_layers() {
        let mut req = request(48, 64);
        req.version = "1.12.2".into();
        let data = build(&req).expect("legacy coarse preview must generate");

        assert_eq!(data.cell_scale, 64);
        assert!(data.biomes.iter().any(|&id| id >= 0));
        assert!(!data.legend.is_empty());
    }

    #[test]
    fn surface_is_the_default_layer() {
        let data = build(&request(48, 16)).expect("surface preview must generate");

        assert_eq!(data.layer, "surface");
        assert_eq!(data.sample_y, 300);
        assert!(data.heights.is_empty());
    }

    #[test]
    fn surface_detail_hides_cave_biomes() {
        let data = build(&request(128, 4)).expect("surface preview must generate");
        assert_eq!(data.sample_y, 300);
        assert!(!data.heights.is_empty(), "surface keeps hillshade heights");
        let names: Vec<&str> = data
            .legend
            .iter()
            .map(|entry| entry.name.as_str())
            .collect();
        assert!(
            !names
                .iter()
                .any(|name| name.contains("caves") || *name == "deep_dark"),
            "surface layer leaked cave biomes: {names:?}"
        );
    }

    #[test]
    fn coarse_surface_hides_cave_biomes() {
        for scale in [16, 64] {
            let data = build(&request(128, scale)).expect("coarse surface preview");
            let names: Vec<&str> = data
                .legend
                .iter()
                .map(|entry| entry.name.as_str())
                .collect();
            assert!(
                !names
                    .iter()
                    .any(|name| name.contains("caves") || *name == "deep_dark"),
                "1:{scale} surface layer leaked cave biomes: {names:?}"
            );
        }
    }

    #[test]
    fn underground_layer_shows_cave_biomes() {
        let mut req = request(128, 16);
        req.layer = Some("underground".into());
        req.depth = Some(-16);
        let data = build(&req).expect("underground preview must generate");

        assert_eq!(data.layer, "underground");
        assert_eq!(data.sample_y, -16);
        assert!(
            data.heights.is_empty(),
            "the underground layer has no surface hillshade"
        );
        assert!(data.biomes.iter().any(|&id| id >= 0));
        let names: Vec<&str> = data
            .legend
            .iter()
            .map(|entry| entry.name.as_str())
            .collect();
        assert!(
            names
                .iter()
                .any(|name| name.contains("caves") || *name == "deep_dark"),
            "expected a cave biome in a 2048-block area, got {names:?}"
        );
    }

    #[test]
    fn underground_detail_view_skips_heights() {
        let mut req = request(32, 4);
        req.layer = Some("underground".into());
        req.depth = Some(0);
        let data = build(&req).expect("underground detail preview must generate");

        assert!(data.heights.is_empty());
        assert_eq!(data.biomes.len(), 32 * 32);
    }

    #[test]
    fn unknown_layer_is_rejected() {
        let mut req = request(32, 16);
        req.layer = Some("nether".into());
        assert!(build(&req).is_err());
    }

    #[test]
    fn unknown_scale_is_rejected() {
        assert!(build(&request(32, 8)).is_err());
    }
}
