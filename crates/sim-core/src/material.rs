//! The material registry (Phase 12 artifact half, `lifesim-material-v1`).
//!
//! A bounded, versioned table of physical property records. **Material IDs
//! select parameter records; physical equations consume the parameters; no
//! rule anywhere branches on a material ID** (review 5.7, ADR-0028 section
//! 5). That sentence is the whole of the no-recipe boundary at this layer,
//! and `rule_coverage_moves_every_outcome_continuously` in the tests is what
//! asserts it: perturb a property and every outcome that reads it moves;
//! swap the ID with the properties held fixed and nothing moves.
//!
//! There is deliberately no `affordances` field. The 2026-08-04 spec carried
//! a six-bit capability tag per material; ADR-0028 replaced every bit with a
//! predicate over physical quantities evaluated at the moment of the action
//! (`carryable` is mass against the asker's capacity, `consumable` is energy
//! greater than zero, `blocks_movement` is a mass threshold). A per-material
//! capability tag is the review's "recipe hidden as physics" in its smallest
//! form: it says what a material is *for*.
//!
//! The registry version enters the config hash inside the artifact section,
//! so adding a material is a config change with a new lineage, not a feature
//! toggle. Materials are world physics, not content.

use crate::checksum::Fnv1a64;

/// Recorded in the config hash under the artifact section.
pub const MATERIAL_POLICY_VERSION: &str = "lifesim-material-v1";
/// Bumped when an entry is added. IDs are permanent and never reused; 0 is
/// invalid, so a zeroed record can never decode as stone.
pub const MATERIAL_REGISTRY_VERSION: u16 = 1;

pub const MATERIAL_STONE: u16 = 1;
pub const MATERIAL_WOOD: u16 = 2;
pub const MATERIAL_FIBER: u16 = 3;
pub const MATERIAL_CARCASS: u16 = 4;
/// One past the highest material id.
pub const MATERIAL_COUNT: u16 = 5;

/// One material's physical property record. All fixed point (Rule 7).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MaterialDef {
    pub id: u16,
    pub name: &'static str,
    /// The force a strike must reach to fracture an object of this material.
    pub hardness_q16: u32,
    /// Mass per 1,000 units of extracted volume, milli-mass.
    pub density_milli: i64,
    /// Integrity lost per strike, by the struck object and by a held object
    /// used to strike.
    pub durability_q16: u32,
    /// Passive integrity loss per tick (linear), and for a material with
    /// energy content the per-tick fraction of remaining energy lost
    /// (exponential, the Phase 7 carcass rule).
    pub decay_per_tick_q16: u32,
    /// Assimilable energy per 1,000 milli-mass; zero for stone and wood.
    pub energy_content_milli: i64,
}

/// Registry version 1. Provisional values, recorded as such in the
/// specification: review 19.7 calls the decay regime relative to organism
/// lifespan an unswept axis, and C12.2 sits on it.
///
/// Carcass decay: 328 Q16 per tick is `0.05 / s * 100 ms` of
/// `contest.carcass_decay_q16_per_s = 3_277`, so a carcass object's energy
/// half-life (~139 ticks) is the one Phase 7 shipped. Its integrity runs out
/// after 200 ticks, in the same regime.
pub const MATERIALS: [MaterialDef; 4] = [
    MaterialDef {
        id: MATERIAL_STONE,
        name: "stone",
        hardness_q16: 8 << 16,
        density_milli: 2_500,
        durability_q16: 32,
        decay_per_tick_q16: 0,
        energy_content_milli: 0,
    },
    MaterialDef {
        id: MATERIAL_WOOD,
        name: "wood",
        hardness_q16: 2 << 16,
        density_milli: 700,
        durability_q16: 512,
        decay_per_tick_q16: 1,
        energy_content_milli: 0,
    },
    MaterialDef {
        id: MATERIAL_FIBER,
        name: "fiber",
        hardness_q16: 1 << 15,
        density_milli: 300,
        durability_q16: 2_048,
        decay_per_tick_q16: 8,
        energy_content_milli: 500,
    },
    MaterialDef {
        id: MATERIAL_CARCASS,
        name: "carcass",
        hardness_q16: 1 << 14,
        density_milli: 1_000,
        durability_q16: 4_096,
        decay_per_tick_q16: 328,
        energy_content_milli: 1_000,
    },
];

/// The record for a material id, or `None` for an id this build does not
/// know. Fails closed: a decoded object naming an unknown material is a
/// restore error, never a default.
pub fn material(id: u16) -> Option<&'static MaterialDef> {
    MATERIALS.iter().find(|entry| entry.id == id)
}

pub fn material_exists(id: u16) -> bool {
    material(id).is_some()
}

/// The largest hardness in the registry: the denominator of the
/// `object_hardness` perception cue.
pub fn max_hardness_q16() -> u32 {
    MATERIALS
        .iter()
        .map(|entry| entry.hardness_q16)
        .max()
        .unwrap_or(1)
        .max(1)
}

/// Fold the registry into the config hash: version, count, then every field
/// of every entry in id order. The same loci under a different registry
/// describe a different world, exactly as the channel registry does.
pub fn hash_registry_into(hasher: &mut Fnv1a64) {
    hasher.update(MATERIAL_POLICY_VERSION.as_bytes());
    hasher.update_u32(u32::from(MATERIAL_REGISTRY_VERSION));
    hasher.update_u32(MATERIALS.len() as u32);
    for entry in MATERIALS.iter() {
        hasher.update_u32(u32::from(entry.id));
        hasher.update_u32(entry.hardness_q16);
        hasher.update_i64(entry.density_milli);
        hasher.update_u32(entry.durability_q16);
        hasher.update_u32(entry.decay_per_tick_q16);
        hasher.update_i64(entry.energy_content_milli);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_dense_from_one_and_zero_is_invalid() {
        for (index, entry) in MATERIALS.iter().enumerate() {
            assert_eq!(usize::from(entry.id), index + 1, "{}", entry.name);
            assert!(material_exists(entry.id));
        }
        assert_eq!(MATERIAL_COUNT, MATERIALS.len() as u16 + 1);
        assert!(!material_exists(0));
        assert!(!material_exists(MATERIAL_COUNT));
        assert!(!material_exists(u16::MAX));
    }

    #[test]
    fn every_property_is_in_its_documented_domain() {
        for entry in MATERIALS.iter() {
            assert!(entry.hardness_q16 > 0, "{}", entry.name);
            assert!(entry.density_milli > 0, "{}", entry.name);
            assert!(entry.durability_q16 > 0, "{}", entry.name);
            assert!(entry.decay_per_tick_q16 <= 65_536, "{}", entry.name);
            assert!(entry.energy_content_milli >= 0, "{}", entry.name);
        }
        assert_eq!(max_hardness_q16(), 8 << 16);
    }

    #[test]
    fn stone_is_hardest_and_carcass_softest_which_the_fracture_rule_reads() {
        // The ordering the perception cue and the fracture rule see. Not a
        // recipe: no rule reads the *name*; but the ordering is what makes
        // "strike stone with stone" a physical fact rather than a table row.
        let hardness = |id: u16| material(id).unwrap().hardness_q16;
        assert!(hardness(MATERIAL_STONE) > hardness(MATERIAL_WOOD));
        assert!(hardness(MATERIAL_WOOD) > hardness(MATERIAL_FIBER));
        assert!(hardness(MATERIAL_FIBER) > hardness(MATERIAL_CARCASS));
    }

    #[test]
    fn the_registry_hash_moves_when_a_property_moves() {
        let mut baseline = Fnv1a64::new();
        hash_registry_into(&mut baseline);
        let baseline = baseline.finish();
        let mut again = Fnv1a64::new();
        hash_registry_into(&mut again);
        assert_eq!(baseline, again.finish(), "the hash is a pure function");
        // A registry with the same shape and one property moved must hash
        // differently, or the config hash cannot tell two physics apart.
        let mut moved = Fnv1a64::new();
        moved.update(MATERIAL_POLICY_VERSION.as_bytes());
        moved.update_u32(u32::from(MATERIAL_REGISTRY_VERSION));
        moved.update_u32(MATERIALS.len() as u32);
        for (index, entry) in MATERIALS.iter().enumerate() {
            moved.update_u32(u32::from(entry.id));
            moved.update_u32(entry.hardness_q16 + u32::from(index == 0));
            moved.update_i64(entry.density_milli);
            moved.update_u32(entry.durability_q16);
            moved.update_u32(entry.decay_per_tick_q16);
            moved.update_i64(entry.energy_content_milli);
        }
        assert_ne!(baseline, moved.finish());
    }
}
