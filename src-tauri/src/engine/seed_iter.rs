//! Random 64-bit seed iterator and worker pool.
//!
//! Seeds are generated with SplitMix64 over `start + index`, partitioned across
//! workers. One controller thread emits throttled progress events and emits the
//! final `search:done`. Workers only touch atomics and their own generator, so
//! pausing is a single relaxed flag check per seed.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Emitter};

use super::generator::SeedGenerator;
use super::rules::{self, Rule};
use super::version::McVersion;
use crate::events::SearchProgress;

/// Everything a search run needs.
#[derive(Debug, Clone)]
pub struct ScanPlan {
    pub version: McVersion,
    pub rules: Vec<Rule>,
    pub max_seeds: u64,
    pub threads: usize,
    /// When set, only this one seed is checked.
    pub single_seed: Option<i64>,
}

const PROGRESS_INTERVAL: Duration = Duration::from_millis(120);
/// A scan stops after this many hits so the UI stays responsive.
const MAX_HITS: u64 = 100;
const THUMBNAIL_CELLS: u32 = 20;

/// Adds a small biome grid around the spawn to a hit for the result list.
fn attach_thumbnail(generator: &mut SeedGenerator, hit: &mut crate::events::SeedHit) {
    let origin_x = hit.spawn_x.div_euclid(4) - THUMBNAIL_CELLS as i32 / 2;
    let origin_z = hit.spawn_z.div_euclid(4) - THUMBNAIL_CELLS as i32 / 2;
    if let Ok(grid) = generator.height_grid(origin_x, origin_z, THUMBNAIL_CELLS, THUMBNAIL_CELLS) {
        hit.thumbnail = grid.biomes;
        hit.thumbnail_size = THUMBNAIL_CELLS;
    }
}

/// Starts a scan without blocking the Tauri command.
pub fn start(app: AppHandle, plan: ScanPlan, stop: Arc<AtomicBool>, running: Arc<AtomicBool>) {
    if let Some(seed) = plan.single_seed {
        run_single(app, plan, seed, running);
        return;
    }

    let target = plan.max_seeds.max(1);
    let threads = plan.threads.clamp(1, 64);
    let start_seed = random_start();
    let checked = Arc::new(AtomicU64::new(0));
    let hits = Arc::new(AtomicU64::new(0));
    let started = Instant::now();
    let mut workers = Vec::with_capacity(threads);

    for worker in 0..threads {
        let worker_app = app.clone();
        let stop = stop.clone();
        let checked = checked.clone();
        let hits = hits.clone();
        let rules = plan.rules.clone();
        let version = plan.version;

        let spawned = thread::Builder::new()
            .name(format!("seed-scan-{worker}"))
            .spawn(move || {
                let mut generator = match SeedGenerator::new(version, 0) {
                    Ok(generator) => generator,
                    Err(err) => {
                        let _ = worker_app.emit("search:error", err.to_string());
                        stop.store(true, Ordering::Relaxed);
                        return;
                    }
                };

                let mut index = worker as u64;
                let stride = threads as u64;

                loop {
                    if stop.load(Ordering::Relaxed) {
                        return;
                    }
                    let slot = checked.fetch_add(1, Ordering::Relaxed);
                    if slot >= target {
                        return;
                    }

                    let seed = mix64(start_seed.wrapping_add(index)) as i64;
                    index = index.wrapping_add(stride);

                    generator.apply_seed(seed);
                    match rules::evaluate(&generator, &rules) {
                        Ok(Some(evaluation)) => {
                            let mut hit = evaluation.into_hit();
                            attach_thumbnail(&mut generator, &mut hit);
                            let found = hits.fetch_add(1, Ordering::Relaxed) + 1;
                            let _ = worker_app.emit("search:hit", hit);
                            if found >= MAX_HITS {
                                stop.store(true, Ordering::Relaxed);
                                return;
                            }
                        }
                        Ok(None) => {}
                        Err(err) => {
                            let _ = worker_app.emit("search:error", err.to_string());
                            stop.store(true, Ordering::Relaxed);
                            return;
                        }
                    }
                }
            });

        match spawned {
            Ok(handle) => workers.push(handle),
            Err(err) => {
                let _ = app.emit("search:error", format!("Worker could not start: {err}"));
                break;
            }
        }
    }

    let controller_app = app.clone();
    let controller_running = running.clone();
    let stop_controller = stop.clone();
    let controller = thread::Builder::new()
        .name("seed-scan-progress".into())
        .spawn(move || {
            loop {
                let finished = workers.iter().all(thread::JoinHandle::is_finished);
                let elapsed = started.elapsed();
                let done_count = checked.load(Ordering::Relaxed).min(target);
                let running_now = !finished && !stop_controller.load(Ordering::Relaxed);

                let _ = controller_app.emit(
                    "search:progress",
                    progress(
                        done_count,
                        hits.load(Ordering::Relaxed),
                        target,
                        elapsed,
                        running_now,
                    ),
                );

                if finished || !running_now {
                    break;
                }
                thread::sleep(PROGRESS_INTERVAL);
            }

            for worker in workers {
                let _ = worker.join();
            }

            controller_running.store(false, Ordering::Relaxed);
            let elapsed = started.elapsed();
            let done_count = checked.load(Ordering::Relaxed).min(target);
            let final_progress = progress(
                done_count,
                hits.load(Ordering::Relaxed),
                target,
                elapsed,
                false,
            );
            let _ = controller_app.emit("search:progress", &final_progress);
            let _ = controller_app.emit("search:done", &final_progress);
        });

    if let Err(err) = controller {
        let _ = app.emit(
            "search:error",
            format!("Progress thread could not start: {err}"),
        );
        running.store(false, Ordering::Relaxed);
    }
}

fn run_single(app: AppHandle, plan: ScanPlan, seed: i64, running: Arc<AtomicBool>) {
    let started = Instant::now();
    let result = SeedGenerator::new(plan.version, seed).and_then(|generator| {
        let evaluation = rules::evaluate(&generator, &plan.rules)?;
        Ok(evaluation)
    });

    let mut hits = 0u64;
    match result {
        Ok(Some(evaluation)) => {
            hits = 1;
            let _ = app.emit("search:hit", evaluation.into_hit());
        }
        Ok(None) => {}
        Err(err) => {
            let _ = app.emit("search:error", err.to_string());
        }
    }

    let final_progress = progress(1, hits, 1, started.elapsed(), false);
    let _ = app.emit("search:progress", &final_progress);
    let _ = app.emit("search:done", &final_progress);
    running.store(false, Ordering::Relaxed);
}

fn progress(
    checked: u64,
    hits: u64,
    target: u64,
    elapsed: Duration,
    running: bool,
) -> SearchProgress {
    let seconds = elapsed.as_secs_f64().max(0.001);
    SearchProgress {
        checked,
        hits,
        target,
        seeds_per_second: checked as f64 / seconds,
        elapsed_ms: elapsed.as_millis() as u64,
        running,
    }
}

/// Deterministic starting point for a scan; purely local (time is not a
/// cryptographic seed source, it just spreads runs).
fn random_start() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0x5EED_F1DE_0000_0001, |duration| {
            duration.as_nanos() as u64 ^ 0x5EED_F1DE_0000_0001
        })
}

/// SplitMix64 finalizer: a bijective mixing function over 64-bit seeds.
#[must_use]
pub fn mix64(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn mix64_is_deterministic_and_distinct() {
        assert_eq!(mix64(42), mix64(42));
        let values: HashSet<_> = (0..1000).map(mix64).collect();
        assert_eq!(values.len(), 1000);
    }
}
