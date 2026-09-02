//! World origin modes: how a world begins (Phase 6, `lifesim-origin-v1`).
//!
//! Every mode is a **starting condition, never a trajectory**. Authoring
//! where the search begins is not authoring the path it takes, which is what
//! makes all of them admissible under ADR-0012
//! (`specifications/world-origin-modes.md`, ADR-0021).
//!
//! Three design decisions here are load-bearing and deliberate:
//!
//! - **The default path is the old path.** `Random` with `deme_count = 1`
//!   and default parameters draws from `Spawn` and `GenomeInit` exactly as
//!   Phase 1 and Phase 2 did, byte for byte. The `FounderSeed` stream (22)
//!   is touched only by the new paths, so adding origin modes cannot shift
//!   an existing world's founder sequence.
//!
//! - **Archetype IDs never enter world state at all.** The specification
//!   says the kernel stores an archetype ID for provenance and nothing
//!   reads it. Storing an authored label on every organism and trusting
//!   future code not to read it is a standing invitation; not storing it is
//!   strictly stronger and costs nothing, because the specification already
//!   forbids using an archetype as an explanatory category for any later
//!   observation. An archetype influences the genome it draws and then
//!   leaves no trace. This makes C6.5 true by construction rather than by
//!   discipline: there is no ID in the world to be inert.
//!
//!   Which archetypes seeded a run is recorded where it is legitimately
//!   needed — the config hash and the run manifest — so a report can still
//!   state the starting condition it is reporting on.
//!
//! - **Placement fails closed.** A founder is placed in a cell matching its
//!   archetype's biome affinity or generation fails with a typed error. A
//!   founder silently dropped into an unsuitable biome would make every
//!   later adaptation claim a claim about our placement bug.

use crate::checksum::Fnv1a64;
use crate::climate::{BIOME_COUNT, Biome};
use crate::config::{OriginConfig, Q16_ONE, SimConfig};
use crate::genome::{Genome, TRAIT_COUNT};
use crate::rng::{RngSystem, named_random};
use crate::worldgen::Terrain;
use std::fmt;

pub const ORIGIN_POLICY_VERSION: &str = "lifesim-origin-v1";

/// Bounded like every other structural cap in this project.
pub const MAX_ARCHETYPES: usize = 8;
/// Bounded number of separated founder groups.
pub const MAX_DEMES: u32 = 64;
/// Attempts to place a deme centre honouring minimum separation before
/// generation fails closed.
const DEME_PLACEMENT_ATTEMPTS: u32 = 4_096;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum OriginMode {
    /// Bounded-random founders of one body plan. The Phase 1/2 behavior.
    #[default]
    Random,
    /// Biome-matched founder archetypes: the head start.
    Seeded,
    /// No organisms: a chemistry field from which protocells may arise
    /// and, under the Phase 16 transition, individuals (ADR-0021,
    /// ADR-0032). Zero founders, `next_entity_id` starts at 1.
    Scratch,
}

impl OriginMode {
    pub fn name(self) -> &'static str {
        match self {
            OriginMode::Random => "random",
            OriginMode::Seeded => "seeded",
            OriginMode::Scratch => "scratch",
        }
    }
}

/// A named founder **distribution**, not an organism.
///
/// Two founders of the same archetype are independent draws and are not
/// identical, and neither is a designed creature. No archetype is named
/// after any real species, and none may be described as one.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Archetype {
    /// Provenance label only. Never stored on an organism, never read by any
    /// rule, input channel, mating gate, or analysis grouping.
    pub id: u16,
    /// Per-gene distribution centre, Q16 over the `[0, 1]` gene range.
    pub trait_mean_q16: [u16; TRAIT_COUNT],
    /// Half-width of the uniform draw around each centre, Q16.
    pub trait_spread_q16: u16,
    /// Half-width of the founder neural draw, Q16 of the weight limit.
    pub neural_spread_q16: u16,
    /// Bitmask over `Biome` IDs this archetype may be placed in.
    pub biome_affinity: u8,
}

impl Archetype {
    /// A neutral archetype: every gene centred, affinity to grassland.
    pub fn neutral(id: u16) -> Self {
        Self {
            id,
            trait_mean_q16: [32_768; TRAIT_COUNT],
            trait_spread_q16: 9_830, // 0.15
            neural_spread_q16: 32_768,
            biome_affinity: 1 << (Biome::Grassland as u8),
        }
    }

    pub fn accepts(&self, biome: Biome) -> bool {
        self.biome_affinity & (1 << (biome as u8)) != 0
    }

    fn hash_into(&self, hasher: &mut Fnv1a64) {
        hasher.update_u32(u32::from(self.id));
        for mean in self.trait_mean_q16 {
            hasher.update_u32(u32::from(mean));
        }
        hasher.update_u32(u32::from(self.trait_spread_q16));
        hasher.update_u32(u32::from(self.neural_spread_q16));
        hasher.update_u32(u32::from(self.biome_affinity));
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OriginError {
    /// No cell matches an archetype's biome affinity, so a founder would
    /// have to be placed somewhere it does not belong.
    NoCellMatchesAffinity {
        archetype_id: u16,
        affinity: u8,
    },
    /// Deme centres could not be placed at the configured separation.
    DemePlacementFailed {
        placed: u32,
        requested: u32,
    },
    /// A deme's radius contains no habitable cell.
    EmptyDeme {
        deme: u32,
    },
    /// `seeded` needs biomes, which means the climate section.
    SeededRequiresClimate,
    NoHabitableCells,
}

impl fmt::Display for OriginError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoCellMatchesAffinity {
                archetype_id,
                affinity,
            } => write!(
                formatter,
                "archetype {archetype_id} has biome affinity mask 0b{affinity:07b} and no \
                 generated cell matches it; widen the affinity, change the seed, or adjust \
                 the biome thresholds. A founder is never placed in an unsuitable biome"
            ),
            Self::DemePlacementFailed { placed, requested } => write!(
                formatter,
                "placed only {placed} of {requested} deme centres at the configured minimum \
                 separation; reduce deme_count, reduce deme_min_separation_m, or use a \
                 larger map"
            ),
            Self::EmptyDeme { deme } => write!(
                formatter,
                "deme {deme} has no habitable cell within its radius; increase deme_radius_m"
            ),
            Self::SeededRequiresClimate => formatter.write_str(
                "origin.mode = seeded needs biomes to match against, so the climate section \
                 must be enabled",
            ),
            Self::NoHabitableCells => {
                formatter.write_str("no habitable cell exists to place founders in")
            }
        }
    }
}

impl std::error::Error for OriginError {}

/// One generated founder, before it becomes an organism.
#[derive(Clone, Debug)]
pub struct Founder {
    pub entity_id: u64,
    pub cell: usize,
    pub genome: Genome,
}

/// Whether the origin section differs from the behavior Phase 1 and Phase 2
/// already had.
///
/// When this is false the section is excluded from the config hash and the
/// old founder code path runs, so both fixtures are preserved. This is the
/// D-014 precedent applied to a section whose "disabled" state is a set of
/// default values rather than a boolean.
pub fn is_default_origin(origin: &OriginConfig) -> bool {
    origin.mode == OriginMode::Random
        && origin.deme_count == 1
        && origin.trait_low_q16 == 16_384
        && origin.trait_span_q16 == 32_768
        && origin.neural_span_q16 == Q16_ONE
}

/// Hash the origin section. Called only when `is_default_origin` is false.
pub fn hash_origin_into(hasher: &mut Fnv1a64, origin: &OriginConfig) {
    hasher.update(b"lifesim-origin-config");
    hasher.update(ORIGIN_POLICY_VERSION.as_bytes());
    hasher.update(origin.mode.name().as_bytes());
    hasher.update_u32(origin.trait_low_q16);
    hasher.update_u32(origin.trait_span_q16);
    hasher.update_u32(origin.neural_span_q16);
    hasher.update_u32(origin.deme_count);
    hasher.update_u32(origin.deme_radius_m);
    hasher.update_u32(origin.deme_min_separation_m);
    hasher.update_u32(origin.deme_trait_spread_q16);
    hasher.update_u32(origin.archetype_count);
    // Every archetype definition is inside the config hash, so two origin
    // configurations are never the same experiment.
    for index in 0..origin.archetype_count as usize {
        origin.archetypes[index].hash_into(hasher);
    }
}

fn unit_q16(draw: u64) -> u32 {
    (draw & 0xffff) as u32
}

/// Uniform draw in `[centre - spread, centre + spread]`, clamped to the gene
/// range, all in Q16.
fn spread_draw(centre_q16: i64, spread_q16: i64, draw: u64) -> f32 {
    let offset = (i64::from(unit_q16(draw)) - 32_768) * spread_q16 * 2 / 65_536;
    let value = (centre_q16 + offset).clamp(0, i64::from(Q16_ONE));
    value as f32 / 65_536.0
}

/// Generate founders for a non-default origin configuration.
///
/// Entity IDs are allocated in ascending `(group, draw_index)` order, where
/// a group is a deme in `random` mode and an archetype in `seeded` mode.
/// The allocation is therefore a pure function of the configuration and
/// never of iteration order, which is what C6.2 checks.
pub fn generate_founders(
    config: &SimConfig,
    terrain: &Terrain,
    biome: &[Biome],
) -> Result<Vec<Founder>, OriginError> {
    let origin = &config.origin;
    let habitable: Vec<usize> = (0..terrain.cell_count())
        .filter(|&index| terrain.capacity_milli[index] > 0)
        .collect();
    if habitable.is_empty() {
        return Err(OriginError::NoHabitableCells);
    }

    match origin.mode {
        OriginMode::Seeded => {
            if biome.is_empty() {
                return Err(OriginError::SeededRequiresClimate);
            }
            generate_seeded(config, biome, &habitable)
        }
        OriginMode::Random => generate_demes(config, terrain, &habitable),
        // A scratch world has founders only in the sense that the field
        // may later make some; there is nothing to place at tick 0. The
        // habitable-cells check above still applies - a world with no
        // land is invalid whatever its origin.
        OriginMode::Scratch => Ok(Vec::new()),
    }
}

/// Founders per group, distributed as evenly as the total allows. The
/// remainder goes to the lowest-numbered groups, a fixed rule rather than a
/// rounding accident.
fn group_sizes(total: u32, groups: u32) -> Vec<u32> {
    let groups = groups.max(1);
    let base = total / groups;
    let remainder = total % groups;
    (0..groups)
        .map(|group| base + u32::from(group < remainder))
        .collect()
}

fn generate_seeded(
    config: &SimConfig,
    biome: &[Biome],
    habitable: &[usize],
) -> Result<Vec<Founder>, OriginError> {
    let origin = &config.origin;
    let seed = config.world_seed;
    let count = origin.archetype_count.max(1);
    let sizes = group_sizes(config.initial_organisms, count);
    let mut founders = Vec::with_capacity(config.initial_organisms as usize);
    let mut next_id = 1_u64;

    for (archetype_index, &size) in sizes.iter().enumerate() {
        let archetype = &origin.archetypes[archetype_index];
        // Candidate cells for this affinity, in ascending cell index so the
        // set is a function of the map and not of traversal.
        let candidates: Vec<usize> = habitable
            .iter()
            .copied()
            .filter(|&cell| archetype.accepts(biome[cell]))
            .collect();
        if candidates.is_empty() {
            return Err(OriginError::NoCellMatchesAffinity {
                archetype_id: archetype.id,
                affinity: archetype.biome_affinity,
            });
        }
        for draw_index in 0..size {
            let entity_id = next_id;
            next_id += 1;
            // Draws are keyed on the archetype's position, never on its ID,
            // so relabelling IDs cannot change a genome.
            let subject = (archetype_index as u64) << 40 | u64::from(draw_index);
            let placement = named_random(seed, 0, RngSystem::FounderSeed, subject, 0);
            let cell = candidates[(placement % candidates.len() as u64) as usize];
            let genome = archetype_genome(seed, subject, archetype);
            founders.push(Founder {
                entity_id,
                cell,
                genome,
            });
        }
    }
    Ok(founders)
}

fn archetype_genome(seed: u64, subject: u64, archetype: &Archetype) -> Genome {
    let mut traits = [0.0_f32; TRAIT_COUNT];
    for (index, gene) in traits.iter_mut().enumerate() {
        let draw = named_random(seed, 0, RngSystem::FounderSeed, subject, 16 + index as u32);
        *gene = spread_draw(
            i64::from(archetype.trait_mean_q16[index]),
            i64::from(archetype.trait_spread_q16),
            draw,
        );
    }
    let neural_span = f32::from(archetype.neural_spread_q16) / 65_536.0;
    let neural = (0..crate::genome::NEURAL_COUNT)
        .map(|index| {
            let draw = named_random(seed, 0, RngSystem::FounderSeed, subject, 64 + index as u32);
            (unit_q16(draw) as f32 / 65_536.0 - 0.5) * neural_span * 2.0
        })
        .collect();
    Genome::validated(traits, neural).expect("archetype genome is valid by construction")
}

fn generate_demes(
    config: &SimConfig,
    terrain: &Terrain,
    habitable: &[usize],
) -> Result<Vec<Founder>, OriginError> {
    let origin = &config.origin;
    let seed = config.world_seed;
    let centres = place_deme_centres(config, terrain, habitable)?;
    let sizes = group_sizes(config.initial_organisms, origin.deme_count);
    let radius_cells = (origin.deme_radius_m / config.cell_size_m.max(1)).max(1) as i64;

    let mut founders = Vec::with_capacity(config.initial_organisms as usize);
    let mut next_id = 1_u64;
    for (deme, &centre) in centres.iter().enumerate() {
        let centre_x = (centre % terrain.cells_x as usize) as i64;
        let centre_y = (centre / terrain.cells_x as usize) as i64;
        let candidates: Vec<usize> = habitable
            .iter()
            .copied()
            .filter(|&cell| {
                let cell_x = (cell % terrain.cells_x as usize) as i64;
                let cell_y = (cell / terrain.cells_x as usize) as i64;
                let dx = cell_x - centre_x;
                let dy = cell_y - centre_y;
                dx * dx + dy * dy <= radius_cells * radius_cells
            })
            .collect();
        if candidates.is_empty() {
            return Err(OriginError::EmptyDeme { deme: deme as u32 });
        }
        // Each deme gets its own trait centre, which is what makes demes
        // genetically distinct rather than merely differently sampled. Two
        // demes drawn from one distribution would have within-deme distance
        // equal to between-deme distance, and C6.3 would fail for a reason
        // that looks like a bug but is really a missing mechanism.
        let deme_subject = (deme as u64) << 48;
        let mut centre_traits = [0_i64; TRAIT_COUNT];
        for (index, value) in centre_traits.iter_mut().enumerate() {
            let draw = named_random(seed, 0, RngSystem::FounderSeed, deme_subject, index as u32);
            *value = i64::from(origin.trait_low_q16)
                + ((i64::from(unit_q16(draw)) * i64::from(origin.trait_span_q16)) >> 16);
        }

        for draw_index in 0..sizes[deme] {
            let entity_id = next_id;
            next_id += 1;
            let subject = deme_subject | u64::from(draw_index);
            let placement = named_random(seed, 0, RngSystem::FounderSeed, subject, 0);
            let cell = candidates[(placement % candidates.len() as u64) as usize];

            let mut traits = [0.0_f32; TRAIT_COUNT];
            for (index, gene) in traits.iter_mut().enumerate() {
                let draw =
                    named_random(seed, 0, RngSystem::FounderSeed, subject, 16 + index as u32);
                *gene = spread_draw(
                    centre_traits[index],
                    i64::from(origin.deme_trait_spread_q16),
                    draw,
                );
            }
            let neural_span = f64::from(origin.neural_span_q16) / 65_536.0;
            let neural = (0..crate::genome::NEURAL_COUNT)
                .map(|index| {
                    let draw =
                        named_random(seed, 0, RngSystem::FounderSeed, subject, 64 + index as u32);
                    ((f64::from(unit_q16(draw)) / 65_536.0 - 0.5) * neural_span) as f32
                })
                .collect();
            founders.push(Founder {
                entity_id,
                cell,
                genome: Genome::validated(traits, neural)
                    .expect("deme genome is valid by construction"),
            });
        }
    }
    Ok(founders)
}

/// Draw deme centres honouring minimum separation, then **sort by cell
/// index** so the assignment of founders to demes is a function of geometry
/// rather than of draw order.
fn place_deme_centres(
    config: &SimConfig,
    terrain: &Terrain,
    habitable: &[usize],
) -> Result<Vec<usize>, OriginError> {
    let origin = &config.origin;
    let seed = config.world_seed;
    let separation_cells = i64::from(origin.deme_min_separation_m / config.cell_size_m.max(1));
    let mut centres: Vec<usize> = Vec::with_capacity(origin.deme_count as usize);

    for deme in 0..origin.deme_count {
        let mut placed = false;
        for attempt in 0..DEME_PLACEMENT_ATTEMPTS {
            let draw = named_random(
                seed,
                0,
                RngSystem::FounderSeed,
                u64::from(deme) | (1 << 60),
                attempt,
            );
            let candidate = habitable[(draw % habitable.len() as u64) as usize];
            let candidate_x = (candidate % terrain.cells_x as usize) as i64;
            let candidate_y = (candidate / terrain.cells_x as usize) as i64;
            let far_enough = centres.iter().all(|&existing| {
                let existing_x = (existing % terrain.cells_x as usize) as i64;
                let existing_y = (existing / terrain.cells_x as usize) as i64;
                let dx = candidate_x - existing_x;
                let dy = candidate_y - existing_y;
                dx * dx + dy * dy >= separation_cells * separation_cells
            });
            if far_enough {
                centres.push(candidate);
                placed = true;
                break;
            }
        }
        if !placed {
            return Err(OriginError::DemePlacementFailed {
                placed: centres.len() as u32,
                requested: origin.deme_count,
            });
        }
    }
    centres.sort_unstable();
    Ok(centres)
}

/// Mean pairwise trait distance between two founder groups. Analysis only;
/// nothing in the tick consults it.
pub fn mean_trait_distance(left: &[&Genome], right: &[&Genome]) -> f64 {
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }
    let mut total = 0.0_f64;
    let mut pairs = 0_u64;
    for a in left {
        for b in right {
            if std::ptr::eq(*a, *b) {
                continue;
            }
            let mut distance = 0.0_f64;
            for gene in 0..TRAIT_COUNT {
                let delta = f64::from(a.traits()[gene]) - f64::from(b.traits()[gene]);
                distance += delta * delta;
            }
            total += distance.sqrt();
            pairs += 1;
        }
    }
    if pairs == 0 {
        0.0
    } else {
        total / pairs as f64
    }
}

/// Which biomes exist in a mask, for reports and error messages.
pub fn affinity_biomes(mask: u8) -> Vec<Biome> {
    Biome::ALL
        .into_iter()
        .filter(|biome| mask & (1 << (*biome as u8)) != 0)
        .collect()
}

/// Every biome, as an affinity mask.
pub fn all_biomes_mask() -> u8 {
    let mut mask = 0_u8;
    for index in 0..BIOME_COUNT {
        mask |= 1 << index;
    }
    mask
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_sizes_distribute_the_remainder_to_the_lowest_groups() {
        assert_eq!(group_sizes(10, 3), vec![4, 3, 3]);
        assert_eq!(group_sizes(9, 3), vec![3, 3, 3]);
        assert_eq!(group_sizes(2, 4), vec![1, 1, 0, 0]);
        assert_eq!(group_sizes(5, 1), vec![5]);
        // Every founder is allocated, never rounded away.
        for total in [0_u32, 1, 7, 100, 501] {
            for groups in 1..=8_u32 {
                assert_eq!(group_sizes(total, groups).iter().sum::<u32>(), total);
            }
        }
    }

    #[test]
    fn spread_draw_stays_inside_the_gene_range() {
        for draw in [0_u64, 1, 32_768, 65_535, u64::MAX] {
            for centre in [0_i64, 16_384, 32_768, 65_536] {
                for spread in [0_i64, 1_000, 32_768, 65_536] {
                    let value = spread_draw(centre, spread, draw);
                    assert!((0.0..=1.0).contains(&value), "{value} out of range");
                }
            }
        }
        // A zero spread reproduces the centre exactly.
        assert_eq!(spread_draw(32_768, 0, 12_345), 0.5);
    }

    #[test]
    fn affinity_masks_round_trip() {
        let archetype = Archetype::neutral(3);
        assert!(archetype.accepts(Biome::Grassland));
        assert!(!archetype.accepts(Biome::Arid));
        assert_eq!(
            affinity_biomes(archetype.biome_affinity),
            vec![Biome::Grassland]
        );
        let all = all_biomes_mask();
        for biome in Biome::ALL {
            assert!(all & (1 << (biome as u8)) != 0);
        }
    }

    #[test]
    fn default_origin_is_recognized_as_the_old_behavior() {
        let config = SimConfig::phase1_default(1);
        assert!(is_default_origin(&config.origin));
        let mut demes = config;
        demes.origin.deme_count = 4;
        assert!(!is_default_origin(&demes.origin));
        let mut seeded = config;
        seeded.origin.mode = OriginMode::Seeded;
        assert!(!is_default_origin(&seeded.origin));
    }
}
