//! M0 tech spike: spawn, biome sampling and approximated height raster for the
//! canonical cubiomes example seed on 1.21.4 and 1.16.5.
//!
//! cubiomes' README uses seed `3055141959546` for its `getSpawn()` example (on
//! 1.18), so it is a natural smoke-test seed. There is no published reference
//! spawn for 1.16.5/1.21.4, so the assertions below check plausibility and the
//! cross-version differences are documented in the README.

use seedfinder_lib::engine::generator::SeedGenerator;
use seedfinder_lib::engine::version;

const SEED: i64 = 3_055_141_959_546;
const GRID_CELLS: u32 = 32;
/// Half the 1024-block preview area: 1024/2/4 = 128 cells.
const GRID_RADIUS_CELLS: i32 = 16;

fn shade(height: f32, min: f32, max: f32) -> char {
    const RAMP: &[u8] = b" .:-=+*#%@";
    if !height.is_finite() || max <= min {
        return '?';
    }
    let normalized = ((height - min) / (max - min)).clamp(0.0, 1.0);
    let index = (normalized * (RAMP.len() - 1) as f32).round() as usize;
    char::from(RAMP[index])
}

#[test]
fn m0_spawn_biome_and_height_grid() {
    let mut summaries = Vec::new();

    for version_id in ["1.21.4", "1.16.5"] {
        let profile =
            version::by_id(version_id).unwrap_or_else(|| panic!("{version_id} must be supported"));

        let mut generator = SeedGenerator::new(profile.mc, SEED)
            .unwrap_or_else(|err| panic!("{version_id}: generator init failed: {err}"));
        let (spawn_x, spawn_z) = generator.spawn();

        // Plausibility: cubiomes searches around the origin and returns a
        // location within a few hundred blocks in practice.
        assert!(
            spawn_x.abs() < 5_000 && spawn_z.abs() < 5_000,
            "{version_id}: implausible spawn ({spawn_x}, {spawn_z})"
        );

        // scale 4 takes 1:4 cell coordinates, not block coordinates.
        let spawn_biome = generator
            .biome_at(
                4,
                spawn_x.div_euclid(4),
                profile.surface_y,
                spawn_z.div_euclid(4),
            )
            .unwrap_or_else(|err| panic!("{version_id}: spawn biome failed: {err}"));
        let spawn_biome_name = generator
            .biome_name(spawn_biome)
            .unwrap_or_else(|| format!("id {spawn_biome}"));

        // 1024x1024 block area around the spawn, at 1:4 scale.
        let origin_cell_x = spawn_x.div_euclid(4) - GRID_RADIUS_CELLS;
        let origin_cell_z = spawn_z.div_euclid(4) - GRID_RADIUS_CELLS;
        let grid = generator
            .height_grid(origin_cell_x, origin_cell_z, GRID_CELLS, GRID_CELLS)
            .unwrap_or_else(|err| panic!("{version_id}: height grid failed: {err}"));

        assert_eq!(grid.heights.len(), (GRID_CELLS * GRID_CELLS) as usize);
        assert_eq!(grid.biomes.len(), (GRID_CELLS * GRID_CELLS) as usize);
        assert!(
            grid.biomes.iter().all(|&id| id >= 0),
            "{version_id}: negative biome id in grid"
        );

        let (min, max, mean) = grid.stats().expect("grid is non-empty");
        assert!(
            (-80.0..400.0).contains(&min) && (-80.0..400.0).contains(&max),
            "{version_id}: implausible height range {min}..{max}"
        );

        // The 32x32 raster as an ASCII height map. Run with `--nocapture`.
        println!(
            "\n=== {} (mc={:?}, seed {SEED}) ===",
            profile.id, profile.mc
        );
        println!("Spawn: ({spawn_x}, {spawn_z})");
        println!(
            "Biom am Spawn (scale 4, y={}): {spawn_biome} ({spawn_biome_name})",
            profile.surface_y
        );
        println!(
            "Höhenraster: {GRID_CELLS}x{GRID_CELLS} Zellen @ ({origin_cell_x}, {origin_cell_z}) \
             = 1024x1024 Blöcke, ca. {min:.1} .. {max:.1} (Mittel {mean:.1})"
        );
        for z in 0..grid.height {
            let mut row = String::with_capacity(grid.width as usize);
            for x in 0..grid.width {
                let (height, _) = grid.at(x, z).expect("cell in bounds");
                row.push(shade(height, min, max));
            }
            println!("{row}");
        }

        let mut biome_counts = std::collections::BTreeMap::new();
        for &id in &grid.biomes {
            *biome_counts.entry(id).or_insert(0usize) += 1;
        }
        let mut biomes: Vec<_> = biome_counts
            .into_iter()
            .map(|(id, count)| (count, id))
            .collect();
        biomes.sort_unstable_by(|a, b| b.cmp(a));
        let legend: Vec<String> = biomes
            .iter()
            .map(|(count, id)| {
                let name = generator
                    .biome_name(*id)
                    .unwrap_or_else(|| format!("id {id}"));
                format!("{name} ×{count}")
            })
            .collect();
        println!("Biome im Raster: {}", legend.join(", "));

        summaries.push((profile.id, spawn_x, spawn_z, spawn_biome, min, max, mean));
    }

    // Cross-version sanity: 1.16.5 and 1.21.4 generate different terrain, so at
    // least one of spawn position or spawn biome must differ.
    let (_, sx1, sz1, b1, ..) = &summaries[0];
    let (_, sx2, sz2, b2, ..) = &summaries[1];
    assert_ne!(
        (&sx1, &sz1, &b1),
        (&sx2, &sz2, &b2),
        "1.16.5 and 1.21.4 unexpectedly produced identical spawn data"
    );
}
