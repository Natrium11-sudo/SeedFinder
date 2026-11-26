//! SeedFinder backend: local Minecraft Java seed search.

pub mod engine;
pub mod events;
pub mod preview;
pub mod store;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};

use engine::catalog::{BiomeInfo, StructureInfo};
use engine::generator::SeedGenerator;
use engine::rules::Rule;
use engine::seed_iter::{self, ScanPlan};
use engine::version::{self, VersionInfo};
use preview::{PreviewData, PreviewRequest};
use store::SeedCollection;

/// Flags shared with a running scan.
struct SearchHandle {
    stop: Arc<AtomicBool>,
    running: Arc<AtomicBool>,
}

/// Tauri-managed state for the single active search slot.
#[derive(Default)]
struct SearchState {
    current: Mutex<Option<SearchHandle>>,
}

/// Returns the Minecraft versions this build can generate, grouped for the UI.
#[tauri::command]
fn version_catalog() -> Vec<VersionInfo> {
    version::catalog()
}

/// Overworld biomes for the rule builder, version-aware.
#[tauri::command]
fn biome_catalog(version: String) -> Result<Vec<BiomeInfo>, String> {
    let profile =
        version::by_id(&version).ok_or_else(|| format!("Unknown version: {version}"))?;
    Ok(engine::catalog::biomes(profile.mc))
}

/// Structures the selected version can generate.
#[tauri::command]
fn structure_catalog(version: String) -> Result<Vec<StructureInfo>, String> {
    let profile =
        version::by_id(&version).ok_or_else(|| format!("Unknown version: {version}"))?;
    Ok(engine::catalog::structures(profile.mc))
}

/// Spawn data for one seed, without any rule filtering.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SeedSummary {
    seed: String,
    spawn_x: i32,
    spawn_z: i32,
    spawn_biome: String,
    height: f32,
}

#[tauri::command]
fn seed_info(version: String, seed: String) -> Result<SeedSummary, String> {
    let profile =
        version::by_id(&version).ok_or_else(|| format!("Unknown version: {version}"))?;
    let parsed = seed
        .trim()
        .parse::<i64>()
        .map_err(|_| format!("Invalid seed: {seed}"))?;
    let mut generator = SeedGenerator::new(profile.mc, parsed).map_err(|err| err.to_string())?;
    let spawn = generator.spawn();
    let spawn_biome = generator
        .biome_at(
            4,
            spawn.0.div_euclid(4),
            profile.surface_y,
            spawn.1.div_euclid(4),
        )
        .ok()
        .and_then(|id| generator.biome_name(id))
        .unwrap_or_else(|| "unknown".to_owned());
    let height = generator
        .height_grid(spawn.0.div_euclid(4), spawn.1.div_euclid(4), 1, 1)
        .ok()
        .and_then(|grid| grid.heights.first().copied())
        .unwrap_or(0.0);

    Ok(SeedSummary {
        seed: parsed.to_string(),
        spawn_x: spawn.0,
        spawn_z: spawn.1,
        spawn_biome,
        height,
    })
}

/// Search request from the UI.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchRequest {
    version: String,
    rules: Vec<Rule>,
    max_seeds: u64,
    threads: u32,
    /// When set, only this seed is checked (decimal string).
    seed: Option<String>,
}

#[tauri::command]
fn search_start(
    app: AppHandle,
    state: State<'_, SearchState>,
    request: SearchRequest,
) -> Result<(), String> {
    let profile = version::by_id(&request.version)
        .ok_or_else(|| format!("Unknown version: {}", request.version))?;

    let stop = Arc::new(AtomicBool::new(false));
    let running = Arc::new(AtomicBool::new(true));

    {
        let mut current = state
            .current
            .lock()
            .map_err(|_| "Search state is locked.".to_owned())?;
        if current
            .as_ref()
            .is_some_and(|handle| handle.running.load(Ordering::Relaxed))
        {
            return Err("A search is already running.".to_owned());
        }
        *current = Some(SearchHandle {
            stop: stop.clone(),
            running: running.clone(),
        });
    }

    let single_seed = match request.seed.as_deref() {
        Some(raw) => Some(
            raw.trim()
                .parse::<i64>()
                .map_err(|_| format!("Invalid seed: {raw}"))?,
        ),
        None => None,
    };

    let plan = ScanPlan {
        version: profile.mc,
        rules: request.rules,
        max_seeds: request.max_seeds.clamp(1, 100_000_000),
        threads: request.threads.clamp(1, 64) as usize,
        single_seed,
    };

    seed_iter::start(app, plan, stop, running);
    Ok(())
}

#[tauri::command]
fn search_stop(state: State<'_, SearchState>) -> Result<(), String> {
    let current = state
        .current
        .lock()
        .map_err(|_| "Suchstatus ist blockiert.".to_owned())?;
    if let Some(handle) = current.as_ref() {
        handle.stop.store(true, Ordering::Relaxed);
    }
    Ok(())
}

#[tauri::command]
fn preview_seed(request: PreviewRequest) -> Result<PreviewData, String> {
    preview::build(&request)
}

#[tauri::command]
fn collection_load(app: AppHandle) -> Result<Vec<SeedCollection>, String> {
    store::load(&app)
}

#[tauri::command]
fn collection_save(app: AppHandle, collections: Vec<SeedCollection>) -> Result<(), String> {
    store::save(&app, collections)
}

/// App metadata for the footer.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AppInfo {
    version: &'static str,
    collection_time: i64,
}

#[tauri::command]
fn app_info() -> AppInfo {
    AppInfo {
        version: env!("CARGO_PKG_VERSION"),
        collection_time: store::now_seconds(),
    }
}

/// Starts the Tauri application.
pub fn run() {
    let result = tauri::Builder::default()
        .manage(SearchState::default())
        .invoke_handler(tauri::generate_handler![
            version_catalog,
            biome_catalog,
            structure_catalog,
            seed_info,
            search_start,
            search_stop,
            preview_seed,
            collection_load,
            collection_save,
            app_info
        ])
        .run(tauri::generate_context!());

    if let Err(err) = result {
        eprintln!("SeedFinder could not start: {err}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_request_accepts_the_ui_payload() {
        let request: SearchRequest = serde_json::from_value(serde_json::json!({
            "version": "26.3",
            "rules": [{"kind": "spawn_biome", "any_of": ["forest"]}],
            "maxSeeds": 10000,
            "threads": 20,
            "seed": null
        }))
        .expect("request must deserialize");

        assert_eq!(request.version, "26.3");
        assert_eq!(request.max_seeds, 10_000);
        assert_eq!(request.threads, 20);
        assert!(request.seed.is_none());
        assert_eq!(request.rules.len(), 1);
    }
}
