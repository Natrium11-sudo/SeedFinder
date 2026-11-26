//! Persisted seed collections.
//!
//! A single versioned JSON document in the Tauri app-data directory. The
//! frontend owns the list and saves it after each change; writes are atomic
//! (temp file + rename) so a crash cannot truncate the collections.
//!
//! Format 2 stores named collections; format 1 was a flat seed list and is
//! migrated into one "My collection" on load.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

const FILE_NAME: &str = "collection.json";
const FORMAT_VERSION: u32 = 2;
const LEGACY_COLLECTION_ID: &str = "meine-sammlung";

/// One saved seed with its spawn snapshot.
///
/// Snake_case to match `src/types.ts` and every other payload the UI reads.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedSeed {
    /// Decimal string, so 64-bit seeds survive the JSON round trip.
    pub seed: String,
    pub version: String,
    /// User-chosen display name, e.g. "Inselbasis".
    #[serde(default)]
    pub name: String,
    pub spawn_x: i32,
    pub spawn_z: i32,
    pub spawn_biome: String,
    #[serde(default)]
    pub note: String,
    /// Unix seconds.
    pub added_at: i64,
}

/// A named group of saved seeds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeedCollection {
    pub id: String,
    pub name: String,
    /// Unix seconds.
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub seeds: Vec<SavedSeed>,
}

#[derive(Debug, Deserialize)]
struct CollectionFile {
    version: u32,
    #[serde(default)]
    collections: Vec<SeedCollection>,
    /// Format 1 stored a flat list here.
    #[serde(default)]
    seeds: Vec<SavedSeed>,
}

#[derive(Debug, Serialize)]
struct StoredFile<'a> {
    version: u32,
    collections: &'a [SeedCollection],
}

fn path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|dir| dir.join(FILE_NAME))
        .map_err(|err| format!("App data directory is unavailable: {err}"))
}

/// Loads the collections, returning an empty list when nothing was saved yet.
pub fn load(app: &AppHandle) -> Result<Vec<SeedCollection>, String> {
    load_from(&path(app)?)
}

/// Replaces the collections on disk.
pub fn save(app: &AppHandle, collections: Vec<SeedCollection>) -> Result<(), String> {
    save_to(&path(app)?, &collections)
}

fn load_from(path: &Path) -> Result<Vec<SeedCollection>, String> {
    let raw = match std::fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(format!("Could not read collections: {err}")),
    };

    let file: CollectionFile = serde_json::from_str(&raw)
        .map_err(|err| format!("Collection file is corrupt ({}): {err}", path.display()))?;
    if file.version > FORMAT_VERSION {
        return Err(format!(
            "Collections were written by a newer version (format {}).",
            file.version
        ));
    }

    if file.version < 2 {
        if file.seeds.is_empty() {
            return Ok(Vec::new());
        }
        return Ok(vec![SeedCollection {
            id: LEGACY_COLLECTION_ID.to_owned(),
            name: "My collection".to_owned(),
            created_at: now_seconds(),
            seeds: file.seeds,
        }]);
    }

    Ok(file.collections)
}

fn save_to(path: &Path, collections: &[SeedCollection]) -> Result<(), String> {
    let dir = path
        .parent()
        .ok_or_else(|| "Invalid storage path for collections.".to_owned())?;
    std::fs::create_dir_all(dir)
        .map_err(|err| format!("Could not create the app data directory: {err}"))?;

    let json = serde_json::to_string_pretty(&StoredFile {
        version: FORMAT_VERSION,
        collections,
    })
    .map_err(|err| format!("Could not serialize collections: {err}"))?;

    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, json).map_err(|err| format!("Could not write collections: {err}"))?;
    std::fs::rename(&tmp, path).map_err(|err| format!("Could not save collections: {err}"))?;
    Ok(())
}

/// Seconds since the Unix epoch.
#[must_use]
pub fn now_seconds() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "seedfinder-store-{}-{tag}.json",
            std::process::id()
        ))
    }

    fn seed(name: &str) -> SavedSeed {
        SavedSeed {
            seed: "3055141959546".into(),
            version: "26.3".into(),
            name: name.into(),
            spawn_x: 32,
            spawn_z: 656,
            spawn_biome: "forest".into(),
            note: "Startbasis".into(),
            added_at: 1_760_000_000,
        }
    }

    #[test]
    fn collections_round_trip_with_names() {
        let path = temp_path("round-trip");
        let collections = vec![
            SeedCollection {
                id: "inseln".into(),
                name: "Inseln".into(),
                created_at: 1,
                seeds: vec![seed("Inselbasis")],
            },
            SeedCollection {
                id: "staedte".into(),
                name: "Cities".into(),
                created_at: 2,
                seeds: Vec::new(),
            },
        ];

        save_to(&path, &collections).expect("save must work");
        let loaded = load_from(&path).expect("load must work");
        let _ = std::fs::remove_file(&path);

        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].name, "Inseln");
        assert_eq!(loaded[0].seeds[0].name, "Inselbasis");
        assert_eq!(loaded[1].name, "Cities");
        assert!(loaded[1].seeds.is_empty());
    }

    #[test]
    fn legacy_flat_file_is_migrated() {
        let path = temp_path("legacy");
        std::fs::write(
            &path,
            r#"{"version":1,"seeds":[{"seed":"42","version":"1.21.4","spawn_x":1,"spawn_z":2,"spawn_biome":"plains","note":"alt","added_at":5}]}"#,
        )
        .expect("fixture must be written");

        let loaded = load_from(&path).expect("load must work");
        let _ = std::fs::remove_file(&path);

        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "My collection");
        assert_eq!(loaded[0].seeds[0].seed, "42");
        assert_eq!(loaded[0].seeds[0].name, "");
    }
}
