//! Thread-local cubiomes generator wrapper.
//!
//! Each search worker owns one [`SeedGenerator`]. Cubiomes is only thread-safe
//! when every thread has its own `Generator` and `SurfaceNoise` state, which
//! this type enforces by construction.
//!
//! The C structs live behind [`crate::engine::ffi`], so this module never
//! duplicates cubiomes layouts. See `vendor/cubiomes/README.md` and
//! `src/ffi/shim.c` for the C side.

use std::ffi::{c_void, CStr};
use std::fmt;
use std::ptr::NonNull;

use super::catalog::StructureKind;
use super::ffi;
use super::version::McVersion;

/// Overworld dimension id (`DIM_OVERWORLD` in cubiomes).
const DIM_OVERWORLD: i32 = 0;
/// Generator flag bits: no large biomes, no forced ocean variants.
const NO_FLAGS: u32 = 0;

/// State of the best end portal found in a seed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortalInfo {
    /// Number of eyes already placed (0-12) in the portal room.
    pub eyes: u8,
    /// Whether the portal is already active (cubiomes bit 12).
    pub lit: bool,
    /// Stronghold position (block coordinates).
    pub x: i32,
    pub z: i32,
}

/// Errors surfaced by the generator wrapper.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineError {
    /// Cubiomes only accepts scale 1 (block) and 4 (biome cell).
    InvalidScale(u32),
    /// Map grids support 1:4, 1:16 and 1:64 sampling.
    InvalidGridScale(u32),
    /// `genBiomes()` refused the requested area.
    BiomeGridFailed(i32),
    /// `getBiomeAt()` returned -1 for this position.
    BiomeAtFailed { scale: u32, x: i32, y: i32, z: i32 },
    /// `mapApproxHeight()` reported an error code.
    HeightGridFailed(i32),
    /// Requested grid is empty or too large to address.
    InvalidGridSize { width: u32, height: u32 },
    /// Cubiomes state could not be allocated.
    AllocationFailed,
    /// A biome id had no name for this version.
    UnknownBiome { id: i32 },
    /// The rule builder sent a structure slug the engine does not know.
    UnknownStructure(String),
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidScale(scale) => {
                write!(f, "invalid biome scale {scale} (allowed: 1 or 4)")
            }
            Self::InvalidGridScale(scale) => {
                write!(f, "invalid map scale {scale} (allowed: 4, 16 or 64)")
            }
            Self::BiomeGridFailed(code) => {
                write!(f, "biome grid could not be generated (code {code})")
            }
            Self::BiomeAtFailed { scale, x, y, z } => write!(
                f,
                "biome at scale {scale}, ({x}, {y}, {z}) could not be generated"
            ),
            Self::HeightGridFailed(code) => {
                write!(f, "height grid could not be generated (code {code})")
            }
            Self::InvalidGridSize { width, height } => {
                write!(f, "invalid grid size {width}×{height}")
            }
            Self::AllocationFailed => {
                write!(f, "cubiomes memory could not be allocated")
            }
            Self::UnknownBiome { id } => write!(f, "unknown biome with id {id}"),
            Self::UnknownStructure(name) => write!(f, "unknown structure: {name}"),
        }
    }
}

impl std::error::Error for EngineError {}

/// A 1:4 height/biome raster produced by [`SeedGenerator::height_grid`].
///
/// `heights` are cubiomes' *approximated* surface heights in blocks. They are
/// suitable for hillshading and rough height rules, never for exact claims.
#[derive(Debug, Clone, PartialEq)]
pub struct HeightGrid {
    /// X origin in 1:4 cells (multiply by 4 for block coordinates).
    pub origin_cell_x: i32,
    /// Z origin in 1:4 cells.
    pub origin_cell_z: i32,
    pub width: u32,
    pub height: u32,
    /// Row-major heights, indexed `z * width + x`.
    pub heights: Vec<f32>,
    /// Row-major biome ids at the same cells.
    pub biomes: Vec<i32>,
}

impl HeightGrid {
    /// Returns `(approximate height, biome id)` for a cell inside the grid.
    #[must_use]
    pub fn at(&self, x: u32, z: u32) -> Option<(f32, i32)> {
        if x >= self.width || z >= self.height {
            return None;
        }
        let index = z as usize * self.width as usize + x as usize;
        Some((*self.heights.get(index)?, *self.biomes.get(index)?))
    }

    /// Lowest, highest and mean approximated height in the grid.
    #[must_use]
    pub fn stats(&self) -> Option<(f32, f32, f32)> {
        if self.heights.is_empty() {
            return None;
        }
        let mut min = f32::INFINITY;
        let mut max = f32::NEG_INFINITY;
        let mut sum = 0.0f64;
        for &height in &self.heights {
            if !height.is_finite() {
                return None;
            }
            min = min.min(height);
            max = max.max(height);
            sum += f64::from(height);
        }
        Some((min, max, (sum / self.heights.len() as f64) as f32))
    }
}

/// Owned cubiomes surface noise for one seed.
struct SurfaceNoise {
    ptr: NonNull<c_void>,
}

impl Drop for SurfaceNoise {
    fn drop(&mut self) {
        // SAFETY: `ptr` came from sf_noise_new and is freed exactly once here.
        unsafe { ffi::sf_noise_free(self.ptr.as_ptr()) };
    }
}

// SAFETY: the noise state is exclusively owned; moving it moves ownership.
unsafe impl Send for SurfaceNoise {}

/// One cubiomes generator plus its matching surface noise, owned per thread.
pub struct SeedGenerator {
    generator: NonNull<c_void>,
    noise: Option<SurfaceNoise>,
    version: McVersion,
    seed: i64,
}

// SAFETY: `SeedGenerator` exclusively owns its cubiomes state. There is no
// shared pointer to it, and moving it to another thread moves that ownership.
unsafe impl Send for SeedGenerator {}

impl fmt::Debug for SeedGenerator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SeedGenerator")
            .field("version", &self.version)
            .field("seed", &self.seed)
            .field("noise_initialized", &self.noise.is_some())
            .finish()
    }
}

impl Drop for SeedGenerator {
    fn drop(&mut self) {
        // SAFETY: `generator` came from sf_gen_new and is freed exactly once.
        unsafe { ffi::sf_gen_free(self.generator.as_ptr()) };
    }
}

impl SeedGenerator {
    /// Creates a generator for `version` with `seed` applied (overworld).
    ///
    /// The seed is the signed `i64` shown in Minecraft; cubiomes stores it as
    /// the bit-identical `u64`, so negative seeds work without special casing.
    pub fn new(version: McVersion, seed: i64) -> Result<Self, EngineError> {
        // SAFETY: sf_gen_new allocates and fully initializes a Generator.
        let raw = unsafe { ffi::sf_gen_new(version.cubiomes_id(), NO_FLAGS) };
        let generator = NonNull::new(raw).ok_or(EngineError::AllocationFailed)?;

        let mut this = Self {
            generator,
            noise: None,
            version,
            seed: 0,
        };
        this.apply_seed(seed);
        Ok(this)
    }

    /// Applies a new seed, invalidating cached surface noise.
    pub fn apply_seed(&mut self, seed: i64) {
        self.seed = seed;
        self.noise = None;
        // SAFETY: the generator is initialized and exclusively borrowed. The
        // seed is passed as the bit-identical underlying u64.
        unsafe {
            ffi::sf_apply_seed(self.generator.as_ptr(), DIM_OVERWORLD, seed as u64);
        }
    }

    #[must_use]
    pub const fn seed(&self) -> i64 {
        self.seed
    }

    #[must_use]
    pub const fn version(&self) -> McVersion {
        self.version
    }

    /// Finds the world spawn using cubiomes' block-aware spawn search.
    ///
    /// This is the expensive part of a spawn-biome check (tens to hundreds of
    /// microseconds). Cubiomes documents it as approximate because the real
    /// spawn depends on grass blocks; it is the best offline estimate available.
    #[must_use]
    pub fn spawn(&self) -> (i32, i32) {
        let mut x = 0;
        let mut z = 0;
        // SAFETY: generator is initialized; both out-pointers are valid.
        unsafe { ffi::sf_get_spawn(self.generator.as_ptr(), &mut x, &mut z) };
        (x, z)
    }

    /// Samples a biome id at `scale` (1 = block, 4 = biome cell).
    pub fn biome_at(&self, scale: u32, x: i32, y: i32, z: i32) -> Result<i32, EngineError> {
        if scale != 1 && scale != 4 {
            return Err(EngineError::InvalidScale(scale));
        }

        // SAFETY: generator is initialized; scale was validated against
        // cubiomes' documented values.
        let biome = unsafe { ffi::sf_get_biome_at(self.generator.as_ptr(), scale as i32, x, y, z) };
        if biome < 0 {
            return Err(EngineError::BiomeAtFailed { scale, x, y, z });
        }
        Ok(biome)
    }

    /// Returns the version-specific biome name, if cubiomes knows the id.
    #[must_use]
    pub fn biome_name(&self, biome_id: i32) -> Option<String> {
        // SAFETY: returned pointer is static cubiomes storage; it is only read.
        let name = unsafe { ffi::sf_biome_name(self.version.cubiomes_id(), biome_id) };
        if name.is_null() {
            return None;
        }
        // SAFETY: non-null and NUL-terminated by cubiomes.
        unsafe { CStr::from_ptr(name) }
            .to_str()
            .ok()
            .map(str::to_owned)
    }

    /// Generates a 1:4 height and biome raster.
    ///
    /// `origin_cell_x`/`origin_cell_z` are biome-cell coordinates (block / 4),
    /// matching cubiomes' `mapApproxHeight()` convention. Heights are
    /// approximated surface heights in blocks.
    pub fn height_grid(
        &mut self,
        origin_cell_x: i32,
        origin_cell_z: i32,
        width: u32,
        height: u32,
    ) -> Result<HeightGrid, EngineError> {
        let cells = (width as usize)
            .checked_mul(height as usize)
            .filter(|&cells| cells > 0)
            .ok_or(EngineError::InvalidGridSize { width, height })?;

        if width > i32::MAX as u32 || height > i32::MAX as u32 {
            return Err(EngineError::InvalidGridSize { width, height });
        }

        let mut heights = vec![0.0f32; cells];
        let mut biomes = vec![-1i32; cells];
        let noise = self.ensure_surface_noise()?;

        // SAFETY: generator and noise are initialized for the same seed and
        // overworld dimension. Both output buffers hold exactly width*height
        // elements, which is what mapApproxHeight writes.
        let result = unsafe {
            ffi::sf_map_approx_height(
                heights.as_mut_ptr(),
                biomes.as_mut_ptr(),
                self.generator.as_ptr(),
                noise,
                origin_cell_x,
                origin_cell_z,
                width as i32,
                height as i32,
            )
        };

        if result != 0 {
            return Err(EngineError::HeightGridFailed(result));
        }

        Ok(HeightGrid {
            origin_cell_x,
            origin_cell_z,
            width,
            height,
            heights,
            biomes,
        })
    }

    /// Finds the nearest structure of `kind` within `radius` blocks of
    /// `(center_x, center_z)`. Returns its block position when one exists and
    /// passes the structure's own biome check.
    #[must_use]
    pub fn structure_near(
        &self,
        kind: StructureKind,
        center_x: i32,
        center_z: i32,
        radius: u32,
    ) -> Option<(i32, i32)> {
        let mut x = 0;
        let mut z = 0;
        // SAFETY: generator is initialized and owns the applied seed; both
        // out-pointers are valid for the call.
        let found = unsafe {
            ffi::sf_structure_near(
                self.generator.as_ptr(),
                kind.id(),
                center_x,
                center_z,
                radius.min(32_768) as i32,
                &mut x,
                &mut z,
            )
        };
        (found == 1).then_some((x, z))
    }

    /// Best end portal among the first `max_strongholds` strongholds.
    ///
    /// Generating stronghold pieces is expensive (~0.1-1 ms per stronghold),
    /// so callers should keep `max_strongholds` small and run this check last.
    #[must_use]
    pub fn portal_eyes(&self, max_strongholds: u8) -> Option<PortalInfo> {
        let mut eyes = 0;
        let mut lit = 0;
        let mut x = 0;
        let mut z = 0;
        // SAFETY: generator is initialized; all out-pointers are valid.
        let found = unsafe {
            ffi::sf_portal_eyes(
                self.generator.as_ptr(),
                i32::from(max_strongholds.clamp(1, 16)),
                &mut eyes,
                &mut lit,
                &mut x,
                &mut z,
            )
        };
        (found == 1).then_some(PortalInfo {
            eyes: eyes as u8,
            lit: lit != 0,
            x,
            z,
        })
    }

    /// Samples a biome grid at a native cubiomes scale (4, 16 or 64).
    ///
    /// `origin_x`/`origin_z` are coordinates in that scale, so a 1:16 origin of
    /// block 2048 is 128. Coarse grids skip height generation entirely and are
    /// used for the zoomed-out map.
    pub fn biome_grid(
        &self,
        scale: u32,
        origin_x: i32,
        origin_z: i32,
        width: u32,
        height: u32,
    ) -> Result<Vec<i32>, EngineError> {
        if !matches!(scale, 4 | 16 | 64) {
            return Err(EngineError::InvalidGridScale(scale));
        }
        let cells = (width as usize)
            .checked_mul(height as usize)
            .filter(|&cells| cells > 0)
            .ok_or(EngineError::InvalidGridSize { width, height })?;

        let mut ids = vec![-1i32; cells];
        // SAFETY: the generator is initialized and owns the seed; the output
        // buffer holds exactly width*height ids, which is what genBiomes writes.
        let result = unsafe {
            ffi::sf_biome_grid(
                ids.as_mut_ptr(),
                self.generator.as_ptr(),
                scale as i32,
                origin_x,
                origin_z,
                width as i32,
                height as i32,
            )
        };
        if result != 0 {
            return Err(EngineError::BiomeGridFailed(result));
        }
        Ok(ids)
    }

    /// Top-down biome grid for coarse overview maps.
    ///
    /// `cubiomes::genBiomes()` at scale 1:64 ignores the vertical coordinate
    /// on 1.18+ and can return cave biomes. This samples every cell centre at
    /// the build limit instead, so surface views never show caves.
    pub fn biome_grid_top(
        &self,
        step_blocks: u32,
        origin_block_x: i32,
        origin_block_z: i32,
        width: u32,
        height: u32,
    ) -> Result<Vec<i32>, EngineError> {
        if !matches!(step_blocks, 16 | 64) {
            return Err(EngineError::InvalidGridScale(step_blocks));
        }
        let cells = (width as usize)
            .checked_mul(height as usize)
            .filter(|&cells| cells > 0)
            .ok_or(EngineError::InvalidGridSize { width, height })?;
        let mut ids = vec![-1i32; cells];
        // SAFETY: generator is initialized; the output buffer holds w*h ids.
        let result = unsafe {
            ffi::sf_biome_grid_top(
                ids.as_mut_ptr(),
                self.generator.as_ptr(),
                step_blocks as i32,
                origin_block_x,
                origin_block_z,
                width as i32,
                height as i32,
            )
        };
        if result != 0 {
            return Err(EngineError::BiomeGridFailed(result));
        }
        Ok(ids)
    }

    /// Samples a biome grid at a native cubiomes scale and a fixed block `y`.
    ///
    /// The underground map layer uses this to reach cave biomes: their 3D
    /// biome noise only exists from 1.18 on, so sampling the top of the world
    /// (see [`Self::biome_grid`]) can never show them. For older versions the
    /// height is ignored by cubiomes and the 2D biome map is returned.
    pub fn biome_grid_at(
        &self,
        scale: u32,
        y: i32,
        origin_x: i32,
        origin_z: i32,
        width: u32,
        height: u32,
    ) -> Result<Vec<i32>, EngineError> {
        if !matches!(scale, 4 | 16 | 64) {
            return Err(EngineError::InvalidGridScale(scale));
        }
        let cells = (width as usize)
            .checked_mul(height as usize)
            .filter(|&cells| cells > 0)
            .ok_or(EngineError::InvalidGridSize { width, height })?;

        let mut ids = vec![-1i32; cells];
        // SAFETY: the generator is initialized and owns the seed; the output
        // buffer holds exactly width*height ids.
        let result = unsafe {
            ffi::sf_biome_grid_at(
                ids.as_mut_ptr(),
                self.generator.as_ptr(),
                scale as i32,
                origin_x,
                y,
                origin_z,
                width as i32,
                height as i32,
            )
        };
        if result != 0 {
            return Err(EngineError::BiomeGridFailed(result));
        }
        Ok(ids)
    }

    /// All viable structures of `kind` inside the block rectangle.
    ///
    /// `max` bounds the scan so a wide map view cannot stall the preview; dense
    /// kinds (shipwreck, treasure) are skipped by the C side when a view would
    /// need thousands of regions.
    #[must_use]
    pub fn structure_area(
        &self,
        kind: StructureKind,
        x0: i32,
        z0: i32,
        x1: i32,
        z1: i32,
        max: usize,
    ) -> Vec<(i32, i32)> {
        let mut xs = vec![0i32; max];
        let mut zs = vec![0i32; max];
        // SAFETY: generator is initialized; both buffers hold `max` ints.
        let count = unsafe {
            ffi::sf_structures_in_area(
                self.generator.as_ptr(),
                kind.id(),
                x0,
                z0,
                x1,
                z1,
                xs.as_mut_ptr(),
                zs.as_mut_ptr(),
                max as i32,
            )
        };
        let count = count.max(0) as usize;
        xs.truncate(count);
        zs.truncate(count);
        xs.into_iter().zip(zs).collect()
    }

    /// Strongholds inside the block rectangle, in ring order.
    #[must_use]
    pub fn strongholds(&self, x0: i32, z0: i32, x1: i32, z1: i32, max: usize) -> Vec<(i32, i32)> {
        let mut xs = vec![0i32; max];
        let mut zs = vec![0i32; max];
        // SAFETY: generator is initialized; both buffers hold `max` ints.
        let count = unsafe {
            ffi::sf_strongholds(
                self.generator.as_ptr(),
                x0,
                z0,
                x1,
                z1,
                xs.as_mut_ptr(),
                zs.as_mut_ptr(),
                max as i32,
            )
        };
        let count = count.max(0) as usize;
        xs.truncate(count);
        zs.truncate(count);
        xs.into_iter().zip(zs).collect()
    }

    /// Slime chunk test for the current seed (16x16 block chunks).
    #[must_use]
    pub fn is_slime_chunk(&self, chunk_x: i32, chunk_z: i32) -> bool {
        // SAFETY: pure function over the seed; no generator state is touched.
        unsafe { ffi::sf_slime_chunk(self.seed as u64, chunk_x, chunk_z) != 0 }
    }

    /// Lazily initializes the surface noise for the current seed.
    fn ensure_surface_noise(&mut self) -> Result<*const c_void, EngineError> {
        if self.noise.is_none() {
            // SAFETY: sf_noise_new allocates and initializes the noise.
            let raw = unsafe { ffi::sf_noise_new(DIM_OVERWORLD, self.seed as u64) };
            let noise = NonNull::new(raw).ok_or(EngineError::AllocationFailed)?;
            self.noise = Some(SurfaceNoise { ptr: noise });
        }

        Ok(self.noise.as_ref().map_or(std::ptr::null(), |noise| {
            noise.ptr.as_ptr() as *const c_void
        }))
    }
}
