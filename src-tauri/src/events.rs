//! Progress and result events sent to the frontend.
//!
//! The search emits `search:progress` periodically, `search:hit` for every
//! match, `search:done` once all workers stopped and `search:error` if the
//! engine refuses to start a worker.

use serde::Serialize;

/// A structure marker belonging to a hit.
#[derive(Debug, Clone, Serialize)]
pub struct StructureHit {
    pub kind: String,
    pub label: String,
    pub x: i32,
    pub z: i32,
}

/// One seed that satisfies every active rule.
#[derive(Debug, Clone, Serialize)]
pub struct SeedHit {
    /// Decimal string, so JavaScript never loses 64-bit precision.
    pub seed: String,
    pub spawn_x: i32,
    pub spawn_z: i32,
    pub spawn_biome: String,
    pub matched: Vec<String>,
    pub structures: Vec<StructureHit>,
    /// Row-major biome ids for a small map thumbnail.
    pub thumbnail: Vec<i32>,
    pub thumbnail_size: u32,
}

/// Live search statistics.
#[derive(Debug, Clone, Serialize)]
pub struct SearchProgress {
    pub checked: u64,
    pub hits: u64,
    pub target: u64,
    pub seeds_per_second: f64,
    pub elapsed_ms: u64,
    pub running: bool,
}
