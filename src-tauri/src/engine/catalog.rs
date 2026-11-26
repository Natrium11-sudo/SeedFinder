//! Biome and structure catalogs for the rule builder.
//!
//! Everything here is version-aware: the lists only contain entries that the
//! vendored cubiomes build knows for the selected Minecraft version.

use std::ffi::CStr;

use serde::Serialize;

use super::ffi;
use super::version::McVersion;

/// One selectable biome.
#[derive(Debug, Clone, Serialize)]
pub struct BiomeInfo {
    pub id: i32,
    /// Minecraft's biome id, e.g. `plains`.
    pub name: String,
    /// Display form, e.g. `Plains`.
    pub label: String,
}

/// One selectable structure type.
#[derive(Debug, Clone, Serialize)]
pub struct StructureInfo {
    pub id: &'static str,
    pub label: &'static str,
}

/// Structures that can be located with `getStructurePos()`.
///
/// The numeric ids mirror `enum StructureType` in `vendor/cubiomes/finders.h`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructureKind {
    Village = 5,
    Shipwreck = 7,
    Monument = 8,
    Mansion = 9,
    Outpost = 10,
    RuinedPortal = 11,
    AncientCity = 13,
    Treasure = 14,
    DesertPyramid = 1,
    JungleTemple = 2,
    SwampHut = 3,
    Igloo = 4,
    TrialChambers = 25,
    AbandonedCamp = 26,
}

impl StructureKind {
    pub const ALL: &'static [Self] = &[
        Self::Village,
        Self::Outpost,
        Self::Monument,
        Self::Mansion,
        Self::DesertPyramid,
        Self::JungleTemple,
        Self::SwampHut,
        Self::Igloo,
        Self::RuinedPortal,
        Self::AncientCity,
        Self::TrialChambers,
        Self::AbandonedCamp,
        Self::Shipwreck,
        Self::Treasure,
    ];

    #[must_use]
    pub const fn id(self) -> i32 {
        self as i32
    }

    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Village => "village",
            Self::Outpost => "outpost",
            Self::Monument => "monument",
            Self::Mansion => "mansion",
            Self::DesertPyramid => "desert_pyramid",
            Self::JungleTemple => "jungle_temple",
            Self::SwampHut => "swamp_hut",
            Self::Igloo => "igloo",
            Self::RuinedPortal => "ruined_portal",
            Self::AncientCity => "ancient_city",
            Self::TrialChambers => "trial_chambers",
            Self::AbandonedCamp => "abandoned_camp",
            Self::Shipwreck => "shipwreck",
            Self::Treasure => "treasure",
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Village => "Village",
            Self::Outpost => "Pillager Outpost",
            Self::Monument => "Ocean Monument",
            Self::Mansion => "Woodland Mansion",
            Self::DesertPyramid => "Desert Pyramid",
            Self::JungleTemple => "Jungle Temple",
            Self::SwampHut => "Witch Hut",
            Self::Igloo => "Igloo",
            Self::RuinedPortal => "Ruined Portal",
            Self::AncientCity => "Ancient City",
            Self::TrialChambers => "Trial Chambers",
            Self::AbandonedCamp => "Abandoned Camp",
            Self::Shipwreck => "Shipwreck",
            Self::Treasure => "Buried Treasure",
        }
    }

    #[must_use]
    pub fn from_slug(slug: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|kind| kind.slug() == slug)
    }
}

/// Name of a biome id for `mc`, if cubiomes knows it.
#[must_use]
pub fn biome_name(mc: McVersion, id: i32) -> Option<String> {
    // SAFETY: the returned pointer is static cubiomes storage; only read here.
    let name = unsafe { ffi::sf_biome_name(mc.cubiomes_id(), id) };
    if name.is_null() {
        return None;
    }
    // SAFETY: non-null and NUL-terminated by cubiomes.
    unsafe { CStr::from_ptr(name) }
        .to_str()
        .ok()
        .map(str::to_owned)
}

/// All overworld biomes that exist in `mc`, sorted by display label.
#[must_use]
pub fn biomes(mc: McVersion) -> Vec<BiomeInfo> {
    let mc_id = mc.cubiomes_id();
    let mut out = Vec::new();

    for id in 0..=255 {
        // SAFETY: pure lookup functions without shared mutable state.
        let exists = unsafe { ffi::sf_biome_exists(mc_id, id) } != 0;
        let overworld = unsafe { ffi::sf_is_overworld(mc_id, id) } != 0;
        if !exists || !overworld {
            continue;
        }
        let Some(name) = biome_name(mc, id) else {
            continue;
        };
        out.push(BiomeInfo {
            id,
            label: prettify(&name),
            name,
        });
    }

    out.sort_by(|a, b| a.label.cmp(&b.label));
    out.dedup_by(|a, b| a.name == b.name);
    out
}

/// True for ocean, deep-ocean and river biomes.
///
/// Used by the island rule: a landmass only counts as an island when it is
/// surrounded by water of one of these kinds.
#[must_use]
pub fn is_water(mc: McVersion, id: i32) -> bool {
    // SAFETY: isOceanic is a pure lookup over static biome data.
    if unsafe { ffi::sf_is_oceanic(id) } != 0 {
        return true;
    }
    biome_name(mc, id).is_some_and(|name| name.contains("river"))
}

/// Structures that the selected version actually generates.
#[must_use]
pub fn structures(mc: McVersion) -> Vec<StructureInfo> {
    StructureKind::ALL
        .iter()
        .filter(|kind| {
            // SAFETY: getStructureConfig only reads static tables.
            (unsafe { ffi::sf_structure_supported(mc.cubiomes_id(), kind.id()) }) != 0
        })
        .map(|kind| StructureInfo {
            id: kind.slug(),
            label: kind.label(),
        })
        .collect()
}

/// `old_growth_pine_taiga` -> `Old Growth Pine Taiga`.
#[must_use]
pub fn prettify(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut capitalize = true;
    for ch in name.chars() {
        if ch == '_' {
            out.push(' ');
            capitalize = true;
        } else if capitalize {
            out.extend(ch.to_uppercase());
            capitalize = false;
        } else {
            out.push(ch);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modern_biomes_include_new_drops() {
        let names: Vec<_> = biomes(McVersion::V26_3)
            .into_iter()
            .map(|biome| biome.name)
            .collect();
        assert!(names.iter().any(|name| name == "pale_garden"));
        assert!(names.iter().any(|name| name == "sulfur_caves"));
        assert!(names.iter().any(|name| name == "dappled_forest"));
    }

    #[test]
    fn structures_are_version_aware() {
        let old = structures(McVersion::V1_12_2);
        assert!(old.iter().all(|entry| entry.id != "trial_chambers"));

        let newest = structures(McVersion::V26_3);
        assert!(newest.iter().any(|entry| entry.id == "trial_chambers"));
        assert!(newest.iter().any(|entry| entry.id == "abandoned_camp"));
    }

    #[test]
    fn prettify_capitalizes_words() {
        assert_eq!(prettify("old_growth_pine_taiga"), "Old Growth Pine Taiga");
        assert_eq!(prettify("plains"), "Plains");
    }
}
