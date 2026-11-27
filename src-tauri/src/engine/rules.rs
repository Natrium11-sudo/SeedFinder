//! Rule tree and its cost-ordered evaluation.
//!
//! Rules are combined with AND: a seed is a hit when every active rule
//! matches. Evaluation is ordered by cost, not by insertion order, so cheap
//! biome lookups reject seeds before the expensive spawn search runs.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::catalog::{self, StructureKind};
use super::generator::{EngineError, SeedGenerator};
use crate::events::StructureHit;

/// One search rule as sent by the rule builder.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Rule {
    /// Biome at a fixed block position (does not need the spawn).
    BiomeAt { x: i32, z: i32, any_of: Vec<String> },
    /// Biome at the world spawn.
    SpawnBiome { any_of: Vec<String> },
    /// Minimum share of one biome inside a radius around the spawn.
    SpawnArea {
        biome: String,
        radius: u32,
        /// 0.0 - 1.0
        min_share: f32,
    },
    /// Any of these structure kinds within `radius` blocks of the spawn.
    StructureNear {
        structures: Vec<String>,
        radius: u32,
    },
    /// The spawn sits on a landmass that is surrounded by water within
    /// `radius` blocks (an island start).
    IslandSpawn { radius: u32 },
    /// Approximate surface category at the spawn, derived from biome and
    /// height. Cubiomes has no block ids, so this is intentionally coarse.
    SpawnSurface { category: SurfaceCategory },
    /// End portal eye count in one of the first strongholds.
    ///
    /// Twelve eyes occur roughly once per trillion seeds, so brute-forcing
    /// this is impractical; the UI offers verified seeds for that case.
    PortalEyes {
        min_eyes: u8,
        require_lit: bool,
        max_strongholds: u8,
        radius: u32,
    },
}

/// Coarse surface category at the spawn point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceCategory {
    Grass,
    Sand,
    Snow,
    Stone,
    Water,
}

impl Rule {
    /// Empty selections are inactive, so a fresh rule never filters everything.
    #[must_use]
    pub fn is_active(&self) -> bool {
        match self {
            Self::BiomeAt { any_of, .. } | Self::SpawnBiome { any_of } => !any_of.is_empty(),
            Self::SpawnArea {
                biome, min_share, ..
            } => !biome.is_empty() && *min_share > 0.0,
            Self::StructureNear { structures, .. } => !structures.is_empty(),
            Self::IslandSpawn { .. } => true,
            Self::SpawnSurface { .. } => true,
            Self::PortalEyes { min_eyes, .. } => *min_eyes > 0,
        }
    }

    const fn priority(&self) -> u8 {
        match self {
            Self::BiomeAt { .. } => 0,
            Self::SpawnBiome { .. } => 1,
            Self::SpawnSurface { .. } => 2,
            Self::IslandSpawn { .. } => 3,
            Self::SpawnArea { .. } => 4,
            Self::StructureNear { .. } => 5,
            Self::PortalEyes { .. } => 6,
        }
    }
}

/// Everything a hit needs, produced by one evaluation.
#[derive(Debug, Clone)]
pub struct Evaluation {
    pub seed: i64,
    pub spawn_x: i32,
    pub spawn_z: i32,
    pub spawn_biome: String,
    pub matched: Vec<String>,
    pub structures: Vec<StructureHit>,
}

impl Evaluation {
    #[must_use]
    pub fn into_hit(self) -> crate::events::SeedHit {
        crate::events::SeedHit {
            seed: self.seed.to_string(),
            spawn_x: self.spawn_x,
            spawn_z: self.spawn_z,
            spawn_biome: self.spawn_biome,
            matched: self.matched,
            structures: self.structures,
            thumbnail: Vec::new(),
            thumbnail_size: 0,
        }
    }
}

type Spawn = (i32, i32);

/// Rules prepared once per scan: inactive rules removed, the rest ordered by
/// cost. Avoids allocating and sorting on every candidate seed.
#[derive(Debug, Clone)]
pub struct CompiledRules {
    rules: Vec<Rule>,
}

#[must_use]
pub fn compile(rules: &[Rule]) -> CompiledRules {
    let mut rules: Vec<Rule> = rules
        .iter()
        .filter(|rule| rule.is_active())
        .cloned()
        .collect();
    rules.sort_by_key(Rule::priority);
    CompiledRules { rules }
}

/// Convenience wrapper for one-off evaluations (tests, single-seed checks).
pub fn evaluate(
    generator: &SeedGenerator,
    rules: &[Rule],
) -> Result<Option<Evaluation>, EngineError> {
    let compiled = compile(rules);
    evaluate_compiled(generator, &compiled)
}

/// Evaluates prepared rules for the generator's current seed.
///
/// Returns `Ok(None)` as soon as one rule fails (early out), otherwise the
/// collected hit data. An inactive rule list still resolves spawn and biome so
/// the UI can show what it found.
pub fn evaluate_compiled(
    generator: &SeedGenerator,
    compiled: &CompiledRules,
) -> Result<Option<Evaluation>, EngineError> {
    let mut spawn: Option<Spawn> = None;
    let mut spawn_biome: Option<String> = None;
    let mut matched = Vec::new();
    let mut structures = Vec::new();

    for rule in &compiled.rules {
        match rule {
            Rule::BiomeAt { x, z, any_of } => {
                let id = generator.biome_at(
                    4,
                    x.div_euclid(4),
                    generator.version().surface_y(),
                    z.div_euclid(4),
                )?;
                let name = biome_name(generator, id)?;
                if !any_of.iter().any(|wanted| wanted == &name) {
                    return Ok(None);
                }
                matched.push(format!("Biome at {x}/{z}: {name}"));
            }
            Rule::SpawnBiome { any_of } => {
                let (x, z) = spawn_of(generator, &mut spawn)?;
                let name = biome_name(
                    generator,
                    generator.biome_at(
                        4,
                        x.div_euclid(4),
                        generator.version().surface_y(),
                        z.div_euclid(4),
                    )?,
                )?;
                if !any_of.iter().any(|wanted| wanted == &name) {
                    return Ok(None);
                }
                spawn_biome = Some(name.clone());
                matched.push(format!("Spawn biome: {name}"));
            }
            Rule::SpawnArea {
                biome,
                radius,
                min_share,
            } => {
                let (x, z) = spawn_of(generator, &mut spawn)?;
                let share = area_share(generator, x, z, *radius, biome)?;
                if share + 0.001 < *min_share {
                    return Ok(None);
                }
                matched.push(format!(
                    "{} at {:.0} % within {radius} blocks",
                    catalog::prettify(biome),
                    share * 100.0
                ));
            }
            Rule::IslandSpawn { radius } => {
                let (x, z) = spawn_of(generator, &mut spawn)?;
                let radius = (*radius).clamp(64, 2048);
                if !island_start(generator, x, z, radius)? {
                    return Ok(None);
                }
                matched.push(format!(
                    "Island spawn: water within {radius} blocks"
                ));
            }
            Rule::SpawnSurface { category } => {
                let (x, z) = spawn_of(generator, &mut spawn)?;
                let id = generator.biome_at(
                    4,
                    x.div_euclid(4),
                    generator.version().surface_y(),
                    z.div_euclid(4),
                )?;
                let name = biome_name(generator, id)?;
                let found = surface_category(&name);
                if found != *category {
                    return Ok(None);
                }
                matched.push(format!("Spawn surface (approx.): {found:?}"));
            }
            Rule::StructureNear {
                structures: kinds,
                radius,
            } => {
                let (x, z) = spawn_of(generator, &mut spawn)?;
                let mut nearest: Option<(StructureKind, i32, i32, i64)> = None;

                for slug in kinds {
                    let Some(kind) = StructureKind::from_slug(slug) else {
                        return Err(EngineError::UnknownStructure(slug.clone()));
                    };
                    let Some((sx, sz)) = generator.structure_near(kind, x, z, *radius) else {
                        continue;
                    };
                    let dx = i64::from(sx) - i64::from(x);
                    let dz = i64::from(sz) - i64::from(z);
                    let distance = dx * dx + dz * dz;
                    if nearest.map_or(true, |(_, _, _, best)| distance < best) {
                        nearest = Some((kind, sx, sz, distance));
                    }
                    if distance == 0 {
                        break;
                    }
                }

                let Some((kind, sx, sz, _)) = nearest else {
                    return Ok(None);
                };
                matched.push(format!("{} nearby", kind.label()));
                structures.push(StructureHit {
                    kind: kind.slug().to_owned(),
                    label: kind.label().to_owned(),
                    x: sx,
                    z: sz,
                });
            }
            Rule::PortalEyes {
                min_eyes,
                require_lit,
                max_strongholds,
                radius,
            } => {
                // This is the most expensive rule; it runs last and rejects
                // early before any piece generation for the common case.
                if *min_eyes > 12 {
                    return Ok(None);
                }
                let Some(portal) = generator.portal_eyes(*max_strongholds) else {
                    return Ok(None);
                };
                if portal.eyes < *min_eyes || (*require_lit && !portal.lit) {
                    return Ok(None);
                }
                if *radius > 0 {
                    let (sx, sz) = spawn_of(generator, &mut spawn)?;
                    let distance = portal.x.abs_diff(sx) as u64 + portal.z.abs_diff(sz) as u64;
                    if distance > u64::from(*radius) {
                        return Ok(None);
                    }
                }
                matched.push(format!(
                    "End portal: {}/12 eyes{} at ({}, {})",
                    portal.eyes,
                    if portal.lit { ", active" } else { "" },
                    portal.x,
                    portal.z
                ));
            }
        }
    }

    let (spawn_x, spawn_z) = spawn_of(generator, &mut spawn)?;
    let spawn_biome = match spawn_biome {
        Some(name) => name,
        None => biome_name(
            generator,
            generator.biome_at(
                4,
                spawn_x.div_euclid(4),
                generator.version().surface_y(),
                spawn_z.div_euclid(4),
            )?,
        )?,
    };

    Ok(Some(Evaluation {
        seed: generator.seed(),
        spawn_x,
        spawn_z,
        spawn_biome,
        matched,
        structures,
    }))
}

fn spawn_of(generator: &SeedGenerator, cache: &mut Option<Spawn>) -> Result<Spawn, EngineError> {
    if let Some(spawn) = cache {
        return Ok(*spawn);
    }
    let spawn = generator.spawn();
    *cache = Some(spawn);
    Ok(spawn)
}

fn biome_name(generator: &SeedGenerator, id: i32) -> Result<String, EngineError> {
    catalog::biome_name(generator.version(), id).ok_or(EngineError::UnknownBiome { id })
}

/// Coarse category from the biome name. Cubiomes provides neither block ids
/// nor a height sampler usable from a shared reference, so this is an
/// approximation and is labelled as such in the UI.
fn surface_category(biome: &str) -> SurfaceCategory {
    let name = biome.to_ascii_lowercase();
    if name.contains("ocean") || name.contains("river") {
        return SurfaceCategory::Water;
    }
    if ["snow", "ice", "frozen", "glacier"]
        .iter()
        .any(|part| name.contains(part))
    {
        return SurfaceCategory::Snow;
    }
    if ["sand", "beach", "desert"]
        .iter()
        .any(|part| name.contains(part))
    {
        return SurfaceCategory::Sand;
    }
    if ["mountain", "peak", "hills", "stony", "crag", "gravel", "cave", "badlands"]
        .iter()
        .any(|part| name.contains(part))
    {
        return SurfaceCategory::Stone;
    }
    SurfaceCategory::Grass
}

/// True when the spawn point is land that is fully enclosed by water within
/// `radius` blocks.
///
/// The check has two stages: eight direct rays must each reach water within the
/// radius (cheap, rejects most continents), then a flood fill over a 1:16 biome
/// grid verifies that the landmass containing the spawn does not touch the
/// sampled border. Coarser than 1:4 on purpose: the rule runs for every seed of
/// a scan, and a 1:4 grid costs tens of milliseconds per seed.
fn island_start(
    generator: &SeedGenerator,
    x: i32,
    z: i32,
    radius: u32,
) -> Result<bool, EngineError> {
    let radius = radius.clamp(64, 2048);
    let y = generator.version().surface_y();
    let mut cache: HashMap<i32, bool> = HashMap::new();

    let spawn_id = generator.biome_at(4, x.div_euclid(4), y, z.div_euclid(4))?;
    if is_water(generator, spawn_id, &mut cache) {
        return Ok(false);
    }

    const RAYS: [(i32, i32); 8] = [
        (1, 0),
        (1, 1),
        (0, 1),
        (-1, 1),
        (-1, 0),
        (-1, -1),
        (0, -1),
        (1, -1),
    ];
    for (dx, dz) in RAYS {
        let mut reached_water = false;
        let mut distance = 32;
        while distance <= radius {
            let step = distance as i32;
            let sample_x = x.saturating_add(dx * step);
            let sample_z = z.saturating_add(dz * step);
            if let Ok(id) = generator.biome_at(4, sample_x.div_euclid(4), y, sample_z.div_euclid(4))
            {
                if is_water(generator, id, &mut cache) {
                    reached_water = true;
                    break;
                }
            }
            distance += 32;
        }
        if !reached_water {
            return Ok(false);
        }
    }

    let cells = (i32::try_from(radius).unwrap_or(2048) * 2 / 16).clamp(8, 256);
    let origin_x = x.div_euclid(16) - cells / 2;
    let origin_z = z.div_euclid(16) - cells / 2;
    let width = usize::try_from(cells).map_err(|_| EngineError::InvalidGridSize {
        width: cells as u32,
        height: cells as u32,
    })?;
    let grid = generator.biome_grid(16, origin_x, origin_z, cells as u32, cells as u32)?;

    let start_x = x.div_euclid(16) - origin_x;
    let start_z = z.div_euclid(16) - origin_z;
    if start_x < 0 || start_z < 0 || start_x >= cells || start_z >= cells {
        return Ok(false);
    }
    let start = start_z as usize * width + start_x as usize;
    let start_is_water = match grid.get(start).copied() {
        Some(id) => is_water(generator, id, &mut cache),
        None => true,
    };
    if start_is_water {
        return Ok(false);
    }

    let mut visited = vec![false; grid.len()];
    visited[start] = true;
    let mut stack = vec![start];
    while let Some(index) = stack.pop() {
        let cx = index % width;
        let cz = index / width;
        // Land that touches the sampled border continues beyond the radius.
        if cx == 0 || cz == 0 || cx + 1 == width || cz + 1 == width {
            return Ok(false);
        }
        for (nx, nz) in [(cx + 1, cz), (cx - 1, cz), (cx, cz + 1), (cx, cz - 1)] {
            let next = nz * width + nx;
            if visited[next] {
                continue;
            }
            visited[next] = true;
            let id = grid[next];
            if !is_water(generator, id, &mut cache) {
                stack.push(next);
            }
        }
    }

    Ok(true)
}

fn is_water(generator: &SeedGenerator, id: i32, cache: &mut HashMap<i32, bool>) -> bool {
    *cache
        .entry(id)
        .or_insert_with(|| catalog::is_water(generator.version(), id))
}

/// Share of `biome` on a 9x9 sampling grid inside `radius` blocks.
fn area_share(
    generator: &SeedGenerator,
    center_x: i32,
    center_z: i32,
    radius: u32,
    biome: &str,
) -> Result<f32, EngineError> {
    let radius = radius.clamp(16, 8192);
    let step = (radius as f32 / 4.0).max(1.0);
    let y = generator.version().surface_y();
    let mut hits = 0u32;
    let mut total = 0u32;

    for gz in -4..=4 {
        for gx in -4..=4 {
            let block_x = center_x + (gx as f32 * step) as i32;
            let block_z = center_z + (gz as f32 * step) as i32;
            // A failed sample (void/end of world) simply does not count.
            if let Ok(id) = generator.biome_at(4, block_x.div_euclid(4), y, block_z.div_euclid(4)) {
                if catalog::biome_name(generator.version(), id).as_deref() == Some(biome) {
                    hits += 1;
                }
            }
            total += 1;
        }
    }

    Ok(hits as f32 / total as f32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::version;

    fn generator(version_id: &str, seed: i64) -> SeedGenerator {
        let profile = version::by_id(version_id).expect("version must exist");
        SeedGenerator::new(profile.mc, seed).expect("generator must initialize")
    }

    #[test]
    fn no_rules_is_a_hit() {
        let gen = generator("26.3", 3055141959546);
        let evaluation = evaluate(&gen, &[]).expect("evaluation must not fail");
        let evaluation = evaluation.expect("a seed without rules is always a hit");
        assert!(!evaluation.spawn_biome.is_empty());
    }

    #[test]
    fn wrong_spawn_biome_filters() {
        let gen = generator("1.21.4", 3055141959546);
        let rules = [Rule::SpawnBiome {
            any_of: vec!["this_biome_does_not_exist".into()],
        }];
        assert!(evaluate(&gen, &rules).expect("evaluation").is_none());
    }

    #[test]
    fn matching_spawn_biome_keeps_seed() {
        let gen = generator("1.21.4", 3055141959546);
        let evaluation = evaluate(
            &gen,
            &[Rule::SpawnBiome {
                any_of: vec!["forest".into()],
            }],
        )
        .expect("evaluation");
        assert!(evaluation.is_some());
    }

    #[test]
    fn rules_use_the_documented_json_shape() {
        let value = serde_json::to_value(Rule::SpawnBiome {
            any_of: vec!["forest".into()],
        })
        .expect("rule must serialize");
        assert_eq!(value["kind"], "spawn_biome");
        assert_eq!(value["any_of"][0], "forest");

        let parsed: Rule = serde_json::from_value(serde_json::json!({
            "kind": "spawn_area",
            "biome": "plains",
            "radius": 512,
            "min_share": 0.25
        }))
        .expect("rule must deserialize");
        match parsed {
            Rule::SpawnArea {
                biome,
                radius,
                min_share,
            } => {
                assert_eq!(biome, "plains");
                assert_eq!(radius, 512);
                assert!((min_share - 0.25).abs() < f32::EPSILON);
            }
            _ => panic!("wrong rule variant"),
        }
    }

    #[test]
    fn island_rule_uses_the_documented_json_shape() {
        let value =
            serde_json::to_value(Rule::IslandSpawn { radius: 512 }).expect("rule must serialize");
        assert_eq!(value["kind"], "island_spawn");
        assert_eq!(value["radius"], 512);

        let parsed: Rule = serde_json::from_value(serde_json::json!({
            "kind": "island_spawn",
            "radius": 384
        }))
        .expect("rule must deserialize");
        match parsed {
            Rule::IslandSpawn { radius } => assert_eq!(radius, 384),
            _ => panic!("wrong rule variant"),
        }
    }

    #[test]
    fn island_rule_rejects_a_mainland_spawn() {
        let gen = generator("26.3", 1);
        let evaluation =
            evaluate(&gen, &[Rule::IslandSpawn { radius: 512 }]).expect("evaluation must not fail");
        assert!(evaluation.is_none(), "seed 1 does not start on an island");
    }

    #[test]
    fn island_rule_accepts_an_island_spawn() {
        let gen = generator("26.3", 3_625_507_445_940_178_159);
        let evaluation =
            evaluate(&gen, &[Rule::IslandSpawn { radius: 512 }]).expect("evaluation must not fail");
        let evaluation = evaluation.expect("seed starts on an island");
        assert!(
            evaluation
                .matched
                .iter()
                .any(|entry| entry.contains("Island spawn")),
            "matched rules must mention the island start"
        );
    }

    #[test]
    fn island_rule_works_on_legacy_versions() {
        let gen = generator("1.6.4", 1234);
        let _ = evaluate(&gen, &[Rule::IslandSpawn { radius: 256 }])
            .expect("legacy evaluation must not fail");
    }

    #[test]
    fn structure_rule_accepts_several_kinds() {
        let gen = generator("26.3", 3055141959546);
        let rule = Rule::StructureNear {
            structures: vec!["village".into(), "monument".into(), "mansion".into()],
            radius: 2048,
        };
        let _ = evaluate(&gen, &[rule]).expect("evaluation must not fail");
    }

    #[test]
    fn treasure_rule_finds_dense_structures() {
        let profile = version::by_id("26.3").expect("version must exist");
        let found = (0..20).any(|seed| {
            let gen = SeedGenerator::new(profile.mc, seed).expect("generator must initialize");
            evaluate(
                &gen,
                &[Rule::StructureNear {
                    structures: vec!["treasure".into()],
                    radius: 1024,
                }],
            )
            .expect("evaluation must not fail")
            .is_some()
        });
        assert!(
            found,
            "treasure uses one region per chunk and must still be searchable"
        );
    }

    #[test]
    fn village_rule_finds_a_village() {
        let gen = generator("26.3", 3055141959546);
        let evaluation = evaluate(
            &gen,
            &[Rule::StructureNear {
                structures: vec!["village".into()],
                radius: 2048,
            }],
        )
        .expect("evaluation");
        if let Some(evaluation) = evaluation {
            assert_eq!(evaluation.structures.len(), 1);
        }
    }
}
