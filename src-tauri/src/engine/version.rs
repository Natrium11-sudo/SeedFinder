//! Versions supported by the vendored cubiomes build.
//!
//! The vendored C library is the actively maintained `xpple/cubiomes` fork
//! (MIT, upstream: `Cubitect/cubiomes`). It implements biome generation from
//! 1.6.4 up to Minecraft **26.3**. Patch releases with unchanged world
//! generation share the enum of their release line (e.g. 1.21.7/1.21.8 use
//! `MC_1_21_6`), which is exactly how cubiomes models patches.
//!
//! This catalog is the single place where a Minecraft version string is mapped
//! to a cubiomes enum and to version-specific generation details (surface
//! sampling height, spawn algorithm family).

use std::ffi::CString;

use serde::Serialize;

use super::ffi;

/// Cubiomes' `MCVersion` values, mirrored from `vendor/cubiomes/biomes.h`.
///
/// The numeric values are checked against the C header by the unit tests, so a
/// cubiomes update cannot silently break this mapping.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum McVersion {
    V1_6_4 = 9,
    V1_7_10 = 10,
    V1_8_9 = 11,
    V1_9_4 = 12,
    V1_10_2 = 13,
    V1_11_2 = 14,
    V1_12_2 = 15,
    V1_13_2 = 16,
    V1_14_4 = 17,
    V1_15_2 = 18,
    V1_16_1 = 19,
    V1_16_5 = 20,
    V1_17_1 = 21,
    V1_18_2 = 22,
    V1_19_2 = 23,
    V1_19_4 = 24,
    V1_20_6 = 25,
    V1_21_1 = 26,
    V1_21_3 = 27,
    /// The 1.21 Winter Drop, released as Minecraft 1.21.4.
    V1_21_4 = 28,
    V1_21_5 = 29,
    V1_21_6 = 30,
    V1_21_9 = 31,
    V1_21_11 = 32,
    V26_1 = 33,
    V26_2 = 34,
    V26_3 = 35,
}

impl McVersion {
    #[must_use]
    pub const fn cubiomes_id(self) -> i32 {
        self as i32
    }

    /// Maps a cubiomes enum value back to a known version.
    #[must_use]
    pub const fn from_cubiomes_id(id: i32) -> Option<Self> {
        match id {
            9 => Some(Self::V1_6_4),
            10 => Some(Self::V1_7_10),
            11 => Some(Self::V1_8_9),
            12 => Some(Self::V1_9_4),
            13 => Some(Self::V1_10_2),
            14 => Some(Self::V1_11_2),
            15 => Some(Self::V1_12_2),
            16 => Some(Self::V1_13_2),
            17 => Some(Self::V1_14_4),
            18 => Some(Self::V1_15_2),
            19 => Some(Self::V1_16_1),
            20 => Some(Self::V1_16_5),
            21 => Some(Self::V1_17_1),
            22 => Some(Self::V1_18_2),
            23 => Some(Self::V1_19_2),
            24 => Some(Self::V1_19_4),
            25 => Some(Self::V1_20_6),
            26 => Some(Self::V1_21_1),
            27 => Some(Self::V1_21_3),
            28 => Some(Self::V1_21_4),
            29 => Some(Self::V1_21_5),
            30 => Some(Self::V1_21_6),
            31 => Some(Self::V1_21_9),
            32 => Some(Self::V1_21_11),
            33 => Some(Self::V26_1),
            34 => Some(Self::V26_2),
            35 => Some(Self::V26_3),
            _ => None,
        }
    }

    /// Y coordinate used when sampling the surface biome.
    ///
    /// Pre-1.18 worlds are 256 blocks tall, so 255 is the top layer; 1.18+
    /// samples at 300. That is above the terrain but below the build limit, so
    /// the map shows the naturally generated surface without cave layers.
    #[must_use]
    pub const fn surface_y(self) -> i32 {
        if (self as i32) >= Self::V1_18_2 as i32 {
            300
        } else {
            255
        }
    }
}

/// Resolves a version with cubiomes' own `str2mc()` parser.
///
/// The parser only knows cubiomes' own names (e.g. `"1.21.4"` or
/// `"1.21 WD"`); the catalog lookup in [`by_id`] is authoritative for the UI.
#[must_use]
pub fn mc_from_string(name: &str) -> Option<McVersion> {
    let name = CString::new(name).ok()?;
    // SAFETY: `name` is a valid NUL-terminated string for the call duration.
    let id = unsafe { ffi::sf_mc_from_string(name.as_ptr()) };
    McVersion::from_cubiomes_id(id)
}

/// UI grouping of the supported releases.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VersionGroup {
    /// 1.6 - 1.12: pre-"Update Aquatic" biome generation.
    Legacy,
    /// 1.13 - 1.17: village & pillage era, still 256 blocks tall.
    VillageAndPillage,
    /// 1.18 - 1.21.4: caves & cliffs terrain, surface sampling at Y 300.
    CavesAndCliffs,
    /// 1.21.5 - 1.21.11: the last 1.x drops.
    Modern,
    /// 26.x calendar versions.
    Calendar,
}

impl VersionGroup {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Legacy => "1.6 – 1.12",
            Self::VillageAndPillage => "1.13 – 1.17",
            Self::CavesAndCliffs => "1.18 – 1.21.4",
            Self::Modern => "1.21.5 – 1.21.11",
            Self::Calendar => "26.x",
        }
    }
}

/// Which spawn algorithm generation a version uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SpawnRule {
    /// 1.6 - 1.12 spawn search.
    Legacy,
    /// 1.13 - 1.17 spawn search (256-block world height).
    Pre118,
    /// 1.18 - 1.21.1 terrain-aware spawn search.
    Post118,
    /// 1.21.2+ spawn search (reworked again in the 1.21.2 drop).
    Post1212,
}

/// Everything the engine needs to know about one selectable version.
#[derive(Debug, Clone, Copy)]
pub struct VersionProfile {
    /// User-facing Minecraft version, e.g. `"1.16.5"`.
    pub id: &'static str,
    /// Cubiomes enum used for generation.
    pub mc: McVersion,
    pub group: VersionGroup,
    /// Y coordinate used when sampling the surface biome at 1:4 scale.
    pub surface_y: i32,
    pub spawn_rule: SpawnRule,
    /// `mapApproxHeight()` is an approximation for every version, marked in the UI.
    pub height_approximate: bool,
}

/// Serializable version record sent to the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct VersionInfo {
    pub id: &'static str,
    pub group: VersionGroup,
    pub group_label: &'static str,
    pub surface_y: i32,
    pub spawn_rule: SpawnRule,
    pub height_approximate: bool,
}

impl VersionProfile {
    #[must_use]
    pub fn info(&self) -> VersionInfo {
        VersionInfo {
            id: self.id,
            group: self.group,
            group_label: self.group.label(),
            surface_y: self.surface_y,
            spawn_rule: self.spawn_rule,
            height_approximate: self.height_approximate,
        }
    }
}

const fn legacy(id: &'static str, mc: McVersion) -> VersionProfile {
    VersionProfile {
        id,
        mc,
        group: VersionGroup::Legacy,
        surface_y: 255,
        spawn_rule: SpawnRule::Legacy,
        height_approximate: true,
    }
}

const fn village(id: &'static str, mc: McVersion) -> VersionProfile {
    VersionProfile {
        id,
        mc,
        group: VersionGroup::VillageAndPillage,
        surface_y: 255,
        spawn_rule: SpawnRule::Pre118,
        height_approximate: true,
    }
}

const fn caves(id: &'static str, mc: McVersion, spawn_rule: SpawnRule) -> VersionProfile {
    VersionProfile {
        id,
        mc,
        group: VersionGroup::CavesAndCliffs,
        surface_y: 300,
        spawn_rule,
        height_approximate: true,
    }
}

const fn modern(id: &'static str, mc: McVersion) -> VersionProfile {
    VersionProfile {
        id,
        mc,
        group: VersionGroup::Modern,
        surface_y: 300,
        spawn_rule: SpawnRule::Post1212,
        height_approximate: true,
    }
}

const fn calendar(id: &'static str, mc: McVersion) -> VersionProfile {
    VersionProfile {
        id,
        mc,
        group: VersionGroup::Calendar,
        surface_y: 300,
        spawn_rule: SpawnRule::Post1212,
        height_approximate: true,
    }
}

/// All supported releases, oldest first.
pub static VERSIONS: &[VersionProfile] = &[
    legacy("1.6.4", McVersion::V1_6_4),
    legacy("1.7.10", McVersion::V1_7_10),
    legacy("1.8.9", McVersion::V1_8_9),
    legacy("1.9.4", McVersion::V1_9_4),
    legacy("1.10.2", McVersion::V1_10_2),
    legacy("1.11.2", McVersion::V1_11_2),
    legacy("1.12.2", McVersion::V1_12_2),
    village("1.13.2", McVersion::V1_13_2),
    village("1.14.4", McVersion::V1_14_4),
    village("1.15.2", McVersion::V1_15_2),
    village("1.16.1", McVersion::V1_16_1),
    village("1.16.5", McVersion::V1_16_5),
    village("1.17.1", McVersion::V1_17_1),
    caves("1.18.2", McVersion::V1_18_2, SpawnRule::Post118),
    caves("1.19.2", McVersion::V1_19_2, SpawnRule::Post118),
    caves("1.19.4", McVersion::V1_19_4, SpawnRule::Post118),
    caves("1.20.6", McVersion::V1_20_6, SpawnRule::Post118),
    caves("1.21.1", McVersion::V1_21_1, SpawnRule::Post118),
    caves("1.21.3", McVersion::V1_21_3, SpawnRule::Post1212),
    // The 1.21 "Winter Drop" shipped as 1.21.4.
    caves("1.21.4", McVersion::V1_21_4, SpawnRule::Post1212),
    modern("1.21.5", McVersion::V1_21_5),
    // 1.21.7 and 1.21.8 did not change world generation.
    modern("1.21.6", McVersion::V1_21_6),
    modern("1.21.7", McVersion::V1_21_6),
    modern("1.21.8", McVersion::V1_21_6),
    modern("1.21.9", McVersion::V1_21_9),
    // 1.21.10 did not change world generation.
    modern("1.21.10", McVersion::V1_21_9),
    modern("1.21.11", McVersion::V1_21_11),
    calendar("26.1", McVersion::V26_1),
    calendar("26.1.1", McVersion::V26_1),
    calendar("26.1.2", McVersion::V26_1),
    calendar("26.2", McVersion::V26_2),
    calendar("26.3", McVersion::V26_3),
];

/// Looks up a version by its user-facing id.
#[must_use]
pub fn by_id(id: &str) -> Option<&'static VersionProfile> {
    VERSIONS.iter().find(|profile| profile.id == id)
}

/// Returns the serializable catalog for the UI.
#[must_use]
pub fn catalog() -> Vec<VersionInfo> {
    VERSIONS.iter().map(VersionProfile::info).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn ids_are_unique() {
        let mut seen = HashSet::new();
        for profile in VERSIONS {
            assert!(
                seen.insert(profile.id),
                "duplicate version id {}",
                profile.id
            );
        }
    }

    #[test]
    fn winter_drop_is_1214() {
        let profile = by_id("1.21.4").expect("1.21.4 must be supported");
        assert_eq!(profile.mc, McVersion::V1_21_4);
        assert_eq!(profile.surface_y, 300);
        assert_eq!(profile.spawn_rule, SpawnRule::Post1212);
    }

    #[test]
    fn newest_versions_are_supported() {
        for id in ["1.21.5", "1.21.9", "1.21.11", "26.1.2", "26.2", "26.3"] {
            let profile = by_id(id).unwrap_or_else(|| panic!("{id} must be supported"));
            assert_eq!(profile.surface_y, 300);
            assert_eq!(profile.spawn_rule, SpawnRule::Post1212);
        }
        assert_eq!(by_id("26.3").map(|p| p.mc), Some(McVersion::V26_3));
        assert_eq!(by_id("1.21.8").map(|p| p.mc), Some(McVersion::V1_21_6));
    }

    #[test]
    fn mirrored_enum_matches_cubiomes_header() {
        assert_eq!(mc_from_string("1.6.4"), Some(McVersion::V1_6_4));
        assert_eq!(mc_from_string("1.16.5"), Some(McVersion::V1_16_5));
        assert_eq!(mc_from_string("1.21.3"), Some(McVersion::V1_21_3));
        assert_eq!(mc_from_string("26.3"), Some(McVersion::V26_3));
    }

    #[test]
    fn older_versions_sample_below_256() {
        for id in ["1.12.2", "1.16.5", "1.17.1"] {
            let profile = by_id(id).expect("version must exist");
            assert_eq!(
                profile.surface_y, 255,
                "{id} uses the 256-block world height"
            );
        }
    }

    #[test]
    fn unsupported_versions_are_rejected() {
        assert!(by_id("26.4").is_none());
        assert!(by_id("1.5.2").is_none());
        assert!(by_id("").is_none());
    }

    #[test]
    fn catalog_covers_all_groups() {
        let catalog = catalog();
        assert_eq!(catalog.len(), VERSIONS.len());
        for group in [
            VersionGroup::Legacy,
            VersionGroup::VillageAndPillage,
            VersionGroup::CavesAndCliffs,
            VersionGroup::Modern,
            VersionGroup::Calendar,
        ] {
            assert!(catalog.iter().any(|info| info.group == group));
        }
    }
}
