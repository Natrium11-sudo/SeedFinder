//! Raw declarations for the cubiomes C shim (`src/ffi/shim.c`).
//!
//! These functions are only called through the safe wrapper in
//! [`crate::engine::generator`] and [`crate::engine::catalog`]. Ownership
//! rules:
//!
//! - `sf_gen_new` / `sf_noise_new` return owned pointers that must be released
//!   with their matching `*_free` function.
//! - Output buffers passed to `sf_map_approx_height` must hold `w * h` items.
//! - `sf_biome_name` returns a pointer into static cubiomes storage and must
//!   not be freed.

use std::ffi::{c_char, c_int, c_void};

extern "C" {
    pub fn sf_gen_new(mc: c_int, flags: u32) -> *mut c_void;
    pub fn sf_gen_free(generator: *mut c_void);
    pub fn sf_apply_seed(generator: *mut c_void, dim: c_int, seed: u64);
    pub fn sf_get_spawn(generator: *const c_void, out_x: *mut c_int, out_z: *mut c_int);
    pub fn sf_get_biome_at(
        generator: *const c_void,
        scale: c_int,
        x: c_int,
        y: c_int,
        z: c_int,
    ) -> c_int;

    pub fn sf_noise_new(dim: c_int, seed: u64) -> *mut c_void;
    pub fn sf_noise_free(noise: *mut c_void);
    pub fn sf_map_approx_height(
        y: *mut f32,
        ids: *mut c_int,
        generator: *const c_void,
        noise: *const c_void,
        x: c_int,
        z: c_int,
        w: c_int,
        h: c_int,
    ) -> c_int;

    pub fn sf_biome_name(mc: c_int, id: c_int) -> *const c_char;
    pub fn sf_mc_from_string(name: *const c_char) -> c_int;
    pub fn sf_biome_exists(mc: c_int, id: c_int) -> c_int;
    pub fn sf_is_overworld(mc: c_int, id: c_int) -> c_int;
    pub fn sf_is_oceanic(id: c_int) -> c_int;

    pub fn sf_biome_grid(
        ids: *mut c_int,
        generator: *const c_void,
        scale: c_int,
        x: c_int,
        z: c_int,
        w: c_int,
        h: c_int,
    ) -> c_int;

    pub fn sf_biome_grid_at(
        ids: *mut c_int,
        generator: *const c_void,
        scale: c_int,
        x: c_int,
        y: c_int,
        z: c_int,
        w: c_int,
        h: c_int,
    ) -> c_int;

    pub fn sf_slime_chunk(seed: u64, chunk_x: c_int, chunk_z: c_int) -> c_int;

    pub fn sf_structures_in_area(
        generator: *const c_void,
        structure: c_int,
        x0: c_int,
        z0: c_int,
        x1: c_int,
        z1: c_int,
        out_x: *mut c_int,
        out_z: *mut c_int,
        max: c_int,
    ) -> c_int;

    pub fn sf_strongholds(
        generator: *const c_void,
        x0: c_int,
        z0: c_int,
        x1: c_int,
        z1: c_int,
        out_x: *mut c_int,
        out_z: *mut c_int,
        max: c_int,
    ) -> c_int;

    pub fn sf_structure_supported(mc: c_int, structure: c_int) -> c_int;
    pub fn sf_structure_near(
        generator: *const c_void,
        structure: c_int,
        center_x: c_int,
        center_z: c_int,
        radius: c_int,
        out_x: *mut c_int,
        out_z: *mut c_int,
    ) -> c_int;

    /// Best end portal eye state among the first `max_strongholds`.
    pub fn sf_portal_eyes(
        generator: *const c_void,
        max_strongholds: c_int,
        out_eyes: *mut c_int,
        out_lit: *mut c_int,
        out_x: *mut c_int,
        out_z: *mut c_int,
    ) -> c_int;
}
