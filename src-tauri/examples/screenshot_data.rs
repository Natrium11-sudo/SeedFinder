//! Dumps the JSON fixture the documentation screenshots are rendered with.
//!
//! Every raster and every catalog entry comes from the real engine, so the
//! screenshots show generated terrain instead of invented colours. Only the
//! Tauri IPC is stubbed by the screenshot harness.
//!
//! Regenerate with:
//! `cargo run --example screenshot_data > scripts/screenshots/data.json`

use serde_json::json;

use seedfinder_lib::engine::{catalog, version};
use seedfinder_lib::events::{SeedHit, StructureHit};
use seedfinder_lib::preview::{self, PreviewRequest};
use seedfinder_lib::store::{SavedSeed, SeedCollection};

const VERSION: &str = "26.3";
const SEED: &str = "3055141959546";

fn request(
    cells: u32,
    scale: u32,
    structures: &[&str],
    strongholds: bool,
    slime: bool,
    layer: Option<&str>,
    depth: Option<i32>,
) -> PreviewRequest {
    PreviewRequest {
        version: VERSION.to_owned(),
        seed: SEED.to_owned(),
        center_x: None,
        center_z: None,
        cells: Some(cells),
        scale: Some(scale),
        structures: structures.iter().map(|slug| (*slug).to_owned()).collect(),
        strongholds: Some(strongholds),
        slime: Some(slime),
        layer: layer.map(str::to_owned),
        depth,
    }
}

fn saved(seed: &str, name: &str, note: &str, biome: &str, x: i32, z: i32) -> SavedSeed {
    SavedSeed {
        seed: seed.to_owned(),
        version: VERSION.to_owned(),
        name: name.to_owned(),
        spawn_x: x,
        spawn_z: z,
        spawn_biome: biome.to_owned(),
        note: note.to_owned(),
        added_at: 1_764_000_000,
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let profile = version::by_id(VERSION).ok_or("version 26.3 is missing")?;
    let structure_kinds = [
        "village",
        "monument",
        "mansion",
        "outpost",
        "ruined_portal",
        "swamp_hut",
        "desert_pyramid",
        "shipwreck",
    ];

    // Zoomed-in default view (512 blocks) and the 1k overview.
    let detail = preview::build(&request(128, 4, &structure_kinds, true, true, None, None))?;
    let overview = preview::build(&request(256, 4, &structure_kinds, true, true, None, None))?;
    let underground = preview::build(&request(
        256,
        4,
        &structure_kinds,
        false,
        false,
        Some("underground"),
        Some(-16),
    ))?;

    let first_structure = overview
        .markers
        .iter()
        .find(|marker| marker.kind != "stronghold")
        .map(|marker| StructureHit {
            kind: marker.kind.clone(),
            label: marker.label.clone(),
            x: marker.x,
            z: marker.z,
        });

    let mut hits = Vec::new();
    for (index, seed) in [SEED, "42", "123456789", "-777"].iter().enumerate() {
        let data = preview::build(&PreviewRequest {
            version: VERSION.to_owned(),
            seed: (*seed).to_owned(),
            center_x: None,
            center_z: None,
            cells: Some(24),
            scale: Some(64),
            structures: Vec::new(),
            strongholds: None,
            slime: None,
            layer: None,
            depth: None,
        })?;
        hits.push(SeedHit {
            seed: data.seed.clone(),
            spawn_x: data.spawn_x,
            spawn_z: data.spawn_z,
            spawn_biome: data.spawn_biome.clone(),
            matched: vec![format!("Spawn biome: {}", data.spawn_biome)],
            structures: if index == 0 {
                first_structure.iter().cloned().collect()
            } else {
                Vec::new()
            },
            thumbnail: data.biomes,
            thumbnail_size: data.width,
        });
    }

    let collections = vec![
        SeedCollection {
            id: "islands".to_owned(),
            name: "Islands".to_owned(),
            created_at: 1_763_000_000,
            seeds: vec![
                saved(
                    SEED,
                    "Island base",
                    "Village on the west coast",
                    "plains",
                    32,
                    656,
                ),
                saved(
                    "3625507445940178159",
                    "Mushroom island",
                    "No mainland within 512 blocks",
                    "mushroom_fields",
                    -1_204,
                    2_880,
                ),
            ],
        },
        SeedCollection {
            id: "villages".to_owned(),
            name: "Villages".to_owned(),
            created_at: 1_764_500_000,
            seeds: vec![saved(
                "123456789",
                "Double village",
                "Two villages a few chunks apart",
                "forest",
                -320,
                1_104,
            )],
        },
    ];

    let payload = json!({
        "versions": version::catalog(),
        "biomes": catalog::biomes(profile.mc),
        "structures": catalog::structures(profile.mc),
        "previewDetail": detail,
        "previewOverview": overview,
        "previewUnderground": underground,
        "hits": hits,
        "collections": collections,
    });

    println!("{}", serde_json::to_string(&payload)?);
    Ok(())
}
