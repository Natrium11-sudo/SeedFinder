//! Contract tests for the JSON the frontend sends and receives.

use seedfinder_lib::events::{SeedHit, StructureHit};
use seedfinder_lib::store::{SavedSeed, SeedCollection};

#[test]
fn seed_hit_keeps_snake_case_fields_and_string_seed() {
    let hit = SeedHit {
        seed: "3055141959546".into(),
        spawn_x: 32,
        spawn_z: 656,
        spawn_biome: "forest".into(),
        matched: vec!["Spawnbiom: forest".into()],
        structures: vec![StructureHit {
            kind: "village".into(),
            label: "Dorf".into(),
            x: 12,
            z: 34,
        }],
        thumbnail: vec![1, 2, 3],
        thumbnail_size: 1,
    };

    let value = serde_json::to_value(&hit).expect("hit must serialize");
    assert_eq!(value["seed"], "3055141959546");
    assert_eq!(value["spawn_x"], 32);
    assert_eq!(value["thumbnail_size"], 1);
    assert_eq!(value["structures"][0]["label"], "Dorf");
}

#[test]
fn saved_seed_keeps_snake_case_fields() {
    let saved = SavedSeed {
        seed: "3055141959546".into(),
        version: "26.3".into(),
        name: "Inselbasis".into(),
        spawn_x: 32,
        spawn_z: 656,
        spawn_biome: "forest".into(),
        note: "Startbasis".into(),
        added_at: 1_760_000_000,
    };

    let value = serde_json::to_value(&saved).expect("saved seed must serialize");
    assert_eq!(value["spawn_x"], 32);
    assert_eq!(value["spawn_z"], 656);
    assert_eq!(value["spawn_biome"], "forest");
    assert_eq!(value["name"], "Inselbasis");
    assert_eq!(value["added_at"], 1_760_000_000_i64);
    assert!(
        value.get("spawnX").is_none(),
        "the collection UI reads `spawn_x`, not `spawnX`"
    );

    let restored: SavedSeed = serde_json::from_value(value).expect("round trip must work");
    assert_eq!(restored.spawn_biome, "forest");
    assert_eq!(restored.name, "Inselbasis");
    assert_eq!(restored.note, "Startbasis");
}

#[test]
fn collection_payload_from_the_ui_deserializes() {
    let value = serde_json::json!({
        "collections": [{
            "id": "c-inseln",
            "name": "Inseln",
            "created_at": 1_760_000_000_i64,
            "seeds": [{
                "seed": "3055141959546",
                "version": "26.3",
                "name": "Inselbasis",
                "spawn_x": 32,
                "spawn_z": 656,
                "spawn_biome": "forest",
                "note": "",
                "added_at": 1_760_000_000_i64
            }]
        }]
    });

    let collections: Vec<SeedCollection> =
        serde_json::from_value(value["collections"].clone()).expect("UI payload must deserialize");
    assert_eq!(collections.len(), 1);
    assert_eq!(collections[0].name, "Inseln");
    assert_eq!(collections[0].seeds[0].name, "Inselbasis");
    assert_eq!(collections[0].seeds[0].spawn_x, 32);
}
