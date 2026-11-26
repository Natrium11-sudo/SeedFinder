//! End-to-end checks for the M1-M4 engine flow on the newest release.

use seedfinder_lib::engine::generator::SeedGenerator;
use seedfinder_lib::engine::version;
use seedfinder_lib::preview::{self, PreviewRequest};

const SEED: i64 = 3_055_141_959_546;

#[test]
fn newest_version_generates_map_and_structures() {
    let profile = version::by_id("26.3").expect("26.3 must be supported");
    let mut generator = SeedGenerator::new(profile.mc, SEED).expect("generator must initialize");

    let spawn = generator.spawn();
    let origin_x = spawn.0.div_euclid(4) - 16;
    let origin_z = spawn.1.div_euclid(4) - 16;
    let grid = generator
        .height_grid(origin_x, origin_z, 32, 32)
        .expect("32x32 grid must work on 26.3");

    assert_eq!(grid.biomes.len(), 32 * 32);
    assert!(grid.biomes.iter().any(|&id| id >= 0));
    let (min, max, mean) = grid.stats().expect("heights must be finite");
    assert!(min > -64.0 && max < 400.0 && mean > min && mean < max);

    let data = preview::build(&PreviewRequest {
        version: "26.3".into(),
        seed: SEED.to_string(),
        center_x: None,
        center_z: None,
        cells: Some(64),
        scale: Some(4),
        structures: vec!["village".into(), "abandoned_camp".into()],
        strongholds: None,
        slime: None,
        layer: None,
        depth: None,
    })
    .expect("preview must build");

    assert_eq!(data.width, 64);
    assert_eq!(data.height, 64);
    assert_eq!(data.spawn_x, spawn.0);
    assert!(!data.spawn_biome.is_empty());
    assert!(!data.legend.is_empty());
    assert_eq!(data.heights.len(), 64 * 64);
}

#[test]
fn every_catalog_version_builds_a_generator() {
    for profile in version::VERSIONS {
        let generator = SeedGenerator::new(profile.mc, SEED);
        assert!(
            generator.is_ok(),
            "generator for {} must initialize",
            profile.id
        );
    }
}

#[test]
fn newest_version_exposes_new_biomes() {
    use seedfinder_lib::engine::catalog;

    let newest = catalog::biomes(version::by_id("26.3").expect("26.3").mc);
    let old = catalog::biomes(version::by_id("1.21.4").expect("1.21.4").mc);
    let newest_names: Vec<&str> = newest.iter().map(|biome| biome.name.as_str()).collect();
    let old_names: Vec<&str> = old.iter().map(|biome| biome.name.as_str()).collect();

    assert!(newest_names.contains(&"dappled_forest"));
    assert!(newest_names.contains(&"sulfur_caves"));
    assert!(!old_names.contains(&"dappled_forest"));
}
