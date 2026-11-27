//! End portal and spawn surface rule tests.
//!
//! A twelve-eye portal occurs roughly once per trillion seeds, so the search
//! for them uses verified seeds instead of brute force. These tests keep the
//! detection honest when the vendored cubiomes fork changes.

use seedfinder_lib::engine::generator::SeedGenerator;
use seedfinder_lib::engine::rules::{compile, evaluate, evaluate_compiled, Rule, SurfaceCategory};
use seedfinder_lib::engine::version;

const KNOWN_121: i64 = -2_627_568_906_264_457_629;
const KNOWN_116: i64 = -4_530_634_556_500_121_041;

fn generator(version_id: &str, seed: i64) -> SeedGenerator {
    let profile = version::by_id(version_id).expect("version must exist");
    SeedGenerator::new(profile.mc, seed).expect("generator must initialize")
}

#[test]
fn known_twelve_eye_seeds_are_detected() {
    for (version_id, seed) in [("1.21.4", KNOWN_121), ("1.16.5", KNOWN_116)] {
        let gen = generator(version_id, seed);
        let portal = gen.portal_eyes(8).expect("a stronghold must exist");
        assert_eq!(portal.eyes, 12, "{version_id}: expected all twelve eyes");
        assert!(portal.lit, "{version_id}: portal should be active");
    }
}

#[test]
fn twelve_eye_rule_accepts_known_seed_and_rejects_others() {
    let rule = [Rule::PortalEyes {
        min_eyes: 12,
        require_lit: true,
        max_strongholds: 8,
        radius: 0,
    }];

    let hit = evaluate(&generator("1.21.4", KNOWN_121), &rule)
        .expect("evaluation")
        .expect("known seed must match");
    assert!(hit.matched.iter().any(|line| line.contains("12/12 eyes")));

    let miss = evaluate(&generator("1.21.4", 0), &rule).expect("evaluation");
    assert!(miss.is_none(), "a random seed must not have twelve eyes");
}

#[test]
fn lower_eye_thresholds_find_seeds() {
    let rule = [Rule::PortalEyes {
        min_eyes: 4,
        require_lit: false,
        max_strongholds: 1,
        radius: 0,
    }];
    let mut hits = 0;
    for seed in 0..200 {
        if evaluate(&generator("1.21.4", seed), &rule)
            .expect("evaluation")
            .is_some()
        {
            hits += 1;
            if hits >= 3 {
                break;
            }
        }
    }
    assert!(hits >= 3, "about 1 in 47 seeds has four or more eyes");
}

#[test]
fn compiled_rules_match_direct_evaluation() {
    let rules = [
        Rule::SpawnBiome {
            any_of: vec!["plains".into(), "forest".into()],
        },
        Rule::PortalEyes {
            min_eyes: 4,
            require_lit: false,
            max_strongholds: 1,
            radius: 0,
        },
        Rule::SpawnSurface {
            category: SurfaceCategory::Grass,
        },
        Rule::IslandSpawn { radius: 512 },
    ];
    let compiled = compile(&rules);
    for seed in 0..50 {
        let gen = generator("1.21.4", seed);
        let direct = evaluate(&gen, &rules).expect("direct evaluation");
        let prepared = evaluate_compiled(&gen, &compiled).expect("compiled evaluation");
        assert_eq!(direct.is_some(), prepared.is_some(), "seed {seed}");
    }
}

#[test]
fn spawn_surface_category_is_mutually_exclusive() {
    let gen = generator("26.3", 3_055_141_959_546);
    let all = [
        SurfaceCategory::Grass,
        SurfaceCategory::Sand,
        SurfaceCategory::Snow,
        SurfaceCategory::Stone,
        SurfaceCategory::Water,
    ];
    let matched = all
        .iter()
        .filter(|category| {
            let rules = [Rule::SpawnSurface { category: **category }];
            evaluate(&gen, &rules)
                .expect("evaluation")
                .is_some()
        })
        .count();
    assert_eq!(matched, 1, "exactly one category must match a spawn");
}
