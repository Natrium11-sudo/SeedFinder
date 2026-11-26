//! Throughput benchmark for the M0 engine path.
//!
//! Measures `applySeed -> getSpawn -> getBiomeAt(spawn)` per seed across a
//! worker pool, which is the cost baseline for the search engine.
//!
//! Usage:
//!   cargo run --release --example bench_seed_scan -- [version] [seeds] [threads] [mode]
//!
//! `mode` is `spawn_biome` (default), `spawn` or `biome`.

use std::thread;
use std::time::Instant;

use seedfinder_lib::engine::generator::SeedGenerator;
use seedfinder_lib::engine::version;

/// SplitMix64: deterministic, fast, good enough for independent 64-bit seeds.
fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let version_id = args.first().map_or("1.21.4", String::as_str);
    let total: u64 = args
        .get(1)
        .and_then(|value| value.parse().ok())
        .unwrap_or(100_000);
    let threads: usize = args
        .get(2)
        .and_then(|value| value.parse().ok())
        .filter(|&n| n > 0)
        .unwrap_or_else(|| thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get));
    let mode = args.get(3).map_or("spawn_biome", String::as_str);

    let profile = version::by_id(version_id)
        .unwrap_or_else(|| panic!("unsupported Minecraft version: {version_id}"));

    let per_thread = total / threads as u64;
    let total = per_thread * threads as u64;

    println!(
        "bench: version={} seeds={} threads={} mode={}",
        profile.id, total, threads, mode
    );

    let started = Instant::now();
    let handles: Vec<_> = (0..threads)
        .map(|worker| {
            let mc = profile.mc;
            let surface_y = profile.surface_y;
            let mode = mode.to_owned();
            thread::spawn(move || {
                let mut generator = SeedGenerator::new(mc, 0).expect("generator allocation failed");
                let mut state =
                    0xA076_1D64_78BD_642F_u64 ^ (worker as u64).wrapping_mul(0xD6E8_FEB8_6659_FD93);
                let mut checksum = 0i64;

                for _ in 0..per_thread {
                    let seed = splitmix64(&mut state) as i64;
                    generator.apply_seed(seed);

                    match mode.as_str() {
                        "biome" => {
                            let biome = generator
                                .biome_at(4, 0, surface_y, 0)
                                .expect("biome lookup must succeed");
                            checksum = checksum.wrapping_add(i64::from(biome));
                        }
                        "spawn" => {
                            let (x, z) = generator.spawn();
                            checksum = checksum
                                .wrapping_add(i64::from(x))
                                .wrapping_add(i64::from(z));
                        }
                        "spawn_biome" => {
                            let (x, z) = generator.spawn();
                            // scale 4 uses 1:4 cell coordinates.
                            let biome = generator
                                .biome_at(4, x.div_euclid(4), surface_y, z.div_euclid(4))
                                .expect("biome lookup must succeed");
                            checksum = checksum
                                .wrapping_add(i64::from(x))
                                .wrapping_add(i64::from(z))
                                .wrapping_add(i64::from(biome));
                        }
                        other => panic!("unknown mode {other} (spawn_biome|spawn|biome)"),
                    }
                }
                checksum
            })
        })
        .collect();

    let mut checksum = 0i64;
    for handle in handles {
        checksum = checksum.wrapping_add(handle.join().expect("worker panicked"));
    }

    let elapsed = started.elapsed();
    let seconds = elapsed.as_secs_f64();
    let rate = total as f64 / seconds.max(f64::EPSILON);
    let micros = seconds * 1e6 / total as f64;

    println!(
        "result: {total} seeds in {seconds:.3}s = {rate:.0} seeds/s ({micros:.1} µs/seed), checksum {checksum}"
    );
}
