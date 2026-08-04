//! Deterministic continent generation over raster fields.
//!
//! Generation is a pure function of `(world_seed, config)` using only named
//! random draws. The output is validated before a world can exist.

use crate::checksum::Fnv1a64;
use crate::config::{Q16_ONE, SimConfig};
use crate::rng::{RngSystem, named_random};
use std::collections::VecDeque;
use std::fmt;

/// Recorded in the config hash; bump on any generator rule change.
pub const WORLDGEN_VERSION: &str = "lifesim-worldgen-v1";

/// Lattice spacing in cells for low-frequency elevation noise.
const LATTICE_STEP: u32 = 32;

/// Static terrain fields for one generated world.
#[derive(Clone, Debug)]
pub struct Terrain {
    pub cells_x: u32,
    pub cells_y: u32,
    /// Post-falloff elevation per cell, Q16 in [0, 65536].
    pub elevation_q16: Vec<u32>,
    /// True for land cells of the single kept continent.
    pub land: Vec<bool>,
    /// Food carrying capacity K per cell, milli-biomass; 0 on water.
    pub capacity_milli: Vec<i64>,
    pub land_cells: u32,
    pub habitable_cells: u32,
    pub terrain_checksum: u64,
}

impl Terrain {
    pub fn cell_count(&self) -> usize {
        (self.cells_x as usize) * (self.cells_y as usize)
    }

    pub fn cell_index(&self, cell_x: u32, cell_y: u32) -> usize {
        (cell_y as usize) * (self.cells_x as usize) + cell_x as usize
    }

    pub fn land_fraction_q16(&self) -> u32 {
        ((u64::from(self.land_cells) << 16) / self.cell_count() as u64) as u32
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorldGenError {
    LandFractionOutOfBounds {
        fraction_q16: u32,
        min: u32,
        max: u32,
    },
    NoHabitableCells,
}

impl fmt::Display for WorldGenError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LandFractionOutOfBounds {
                fraction_q16,
                min,
                max,
            } => write!(
                formatter,
                "generated land fraction {fraction_q16}/65536 is outside [{min}, {max}]; \
                 adjust seed or land_threshold_q16"
            ),
            Self::NoHabitableCells => {
                formatter.write_str("generated world has no habitable (positive-capacity) cells")
            }
        }
    }
}

impl std::error::Error for WorldGenError {}

/// Generate and validate terrain. Pure and deterministic.
pub fn generate(config: &SimConfig) -> Result<Terrain, WorldGenError> {
    let cells_x = config.cells_x;
    let cells_y = config.cells_y;
    let cell_count = (cells_x as usize) * (cells_y as usize);

    // 1. Low-frequency lattice noise, bilinearly interpolated.
    let lattice_x = cells_x.div_ceil(LATTICE_STEP) + 2;
    let lattice_y = cells_y.div_ceil(LATTICE_STEP) + 2;
    let lattice: Vec<u32> = (0..lattice_x * lattice_y)
        .map(|index| {
            (named_random(
                config.world_seed,
                0,
                RngSystem::WorldGen,
                u64::from(index),
                0,
            ) & 0xffff) as u32
        })
        .collect();

    let mut elevation_q16 = vec![0_u32; cell_count];
    for cell_y in 0..cells_y {
        for cell_x in 0..cells_x {
            let noise = bilinear_lattice(&lattice, lattice_x, cell_x, cell_y);
            // 2. Radial falloff keeps a single continent away from the rim.
            let falloff = radial_falloff_q16(cell_x, cell_y, cells_x, cells_y);
            let value = (u64::from(noise) * u64::from(falloff)) >> 16;
            elevation_q16[(cell_y as usize) * (cells_x as usize) + cell_x as usize] = value as u32;
        }
    }

    // 3. Threshold to land, force a water rim.
    let mut land: Vec<bool> = elevation_q16
        .iter()
        .map(|&elevation| elevation >= config.land_threshold_q16)
        .collect();
    for cell_x in 0..cells_x {
        land[cell_x as usize] = false;
        land[(cells_y as usize - 1) * cells_x as usize + cell_x as usize] = false;
    }
    for cell_y in 0..cells_y {
        land[(cell_y as usize) * cells_x as usize] = false;
        land[(cell_y as usize) * cells_x as usize + cells_x as usize - 1] = false;
    }

    // 4. Keep only the largest connected landmass (deterministic BFS order).
    keep_largest_component(&mut land, cells_x, cells_y);

    // 5. Capacity from elevation suitability (peaks at mid elevation band).
    let mut capacity_milli = vec![0_i64; cell_count];
    let mut land_cells = 0_u32;
    let mut habitable_cells = 0_u32;
    for index in 0..cell_count {
        if !land[index] {
            continue;
        }
        land_cells += 1;
        let suitability = suitability_q16(elevation_q16[index], config.land_threshold_q16);
        let capacity = (config.cell_capacity_milli * i64::from(suitability)) >> 16;
        capacity_milli[index] = capacity;
        if capacity > 0 {
            habitable_cells += 1;
        }
    }

    // 6. Validate invariants and derive a reproducible checksum.
    let fraction_q16 = ((u64::from(land_cells) << 16) / cell_count as u64) as u32;
    if fraction_q16 < config.min_land_fraction_q16 || fraction_q16 > config.max_land_fraction_q16 {
        return Err(WorldGenError::LandFractionOutOfBounds {
            fraction_q16,
            min: config.min_land_fraction_q16,
            max: config.max_land_fraction_q16,
        });
    }
    if habitable_cells == 0 {
        return Err(WorldGenError::NoHabitableCells);
    }

    let mut hasher = Fnv1a64::new();
    hasher.update(b"lifesim-terrain-v1");
    hasher.update_u32(cells_x);
    hasher.update_u32(cells_y);
    for index in 0..cell_count {
        hasher.update(&[u8::from(land[index])]);
        hasher.update_u32(elevation_q16[index]);
        hasher.update_i64(capacity_milli[index]);
    }

    Ok(Terrain {
        cells_x,
        cells_y,
        elevation_q16,
        land,
        capacity_milli,
        land_cells,
        habitable_cells,
        terrain_checksum: hasher.finish(),
    })
}

fn bilinear_lattice(lattice: &[u32], lattice_x: u32, cell_x: u32, cell_y: u32) -> u32 {
    let grid_x = cell_x / LATTICE_STEP;
    let grid_y = cell_y / LATTICE_STEP;
    let frac_x = cell_x % LATTICE_STEP;
    let frac_y = cell_y % LATTICE_STEP;
    let corner = |dx: u32, dy: u32| -> u64 {
        u64::from(lattice[((grid_y + dy) * lattice_x + grid_x + dx) as usize])
    };
    let weight_x1 = u64::from(frac_x);
    let weight_x0 = u64::from(LATTICE_STEP) - weight_x1;
    let weight_y1 = u64::from(frac_y);
    let weight_y0 = u64::from(LATTICE_STEP) - weight_y1;
    let top = corner(0, 0) * weight_x0 + corner(1, 0) * weight_x1;
    let bottom = corner(0, 1) * weight_x0 + corner(1, 1) * weight_x1;
    ((top * weight_y0 + bottom * weight_y1) / (u64::from(LATTICE_STEP) * u64::from(LATTICE_STEP)))
        as u32
}

fn radial_falloff_q16(cell_x: u32, cell_y: u32, cells_x: u32, cells_y: u32) -> u32 {
    // Normalized squared distance from world center in Q16, scaled so the
    // falloff reaches zero at the rim corners.
    let half_x = i64::from(cells_x) / 2;
    let half_y = i64::from(cells_y) / 2;
    let dx = i64::from(cell_x) - half_x;
    let dy = i64::from(cell_y) - half_y;
    let dist2 = dx * dx + dy * dy;
    let max2 = half_x * half_x + half_y * half_y;
    let normalized = ((dist2 << 16) / max2.max(1)) as u64;
    let scaled = (normalized * 3) / 2;
    (u64::from(Q16_ONE)).saturating_sub(scaled) as u32
}

fn suitability_q16(elevation_q16: u32, threshold_q16: u32) -> u32 {
    // Relative elevation above the coastline, Q16 in [0, 65536].
    let span = Q16_ONE.saturating_sub(threshold_q16).max(1);
    let relative = ((u64::from(elevation_q16.saturating_sub(threshold_q16)) << 16)
        / u64::from(span))
    .min(u64::from(Q16_ONE)) as u32;
    // Triangle profile peaking at mid elevation.
    let distance_from_mid = relative.abs_diff(Q16_ONE / 2) * 2;
    Q16_ONE - distance_from_mid.min(Q16_ONE)
}

fn keep_largest_component(land: &mut [bool], cells_x: u32, cells_y: u32) {
    let cell_count = land.len();
    let mut component = vec![0_u32; cell_count];
    let mut next_component = 0_u32;
    let mut best_component = 0_u32;
    let mut best_size = 0_usize;
    let mut queue = VecDeque::new();

    for start in 0..cell_count {
        if !land[start] || component[start] != 0 {
            continue;
        }
        next_component += 1;
        let mut size = 0_usize;
        component[start] = next_component;
        queue.push_back(start);
        while let Some(index) = queue.pop_front() {
            size += 1;
            let cell_x = (index % cells_x as usize) as u32;
            let cell_y = (index / cells_x as usize) as u32;
            let mut visit = |neighbor: usize| {
                if land[neighbor] && component[neighbor] == 0 {
                    component[neighbor] = next_component;
                    queue.push_back(neighbor);
                }
            };
            if cell_x > 0 {
                visit(index - 1);
            }
            if cell_x + 1 < cells_x {
                visit(index + 1);
            }
            if cell_y > 0 {
                visit(index - cells_x as usize);
            }
            if cell_y + 1 < cells_y {
                visit(index + cells_x as usize);
            }
        }
        if size > best_size {
            best_size = size;
            best_component = next_component;
        }
    }

    for index in 0..cell_count {
        if land[index] && component[index] != best_component {
            land[index] = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_is_deterministic_for_a_seed() {
        let config = SimConfig::phase1_default(0x5eed_cafe_f00d_beef);
        let first = generate(&config).unwrap();
        let second = generate(&config).unwrap();
        assert_eq!(first.terrain_checksum, second.terrain_checksum);
        assert_eq!(first.land, second.land);
        assert_eq!(first.capacity_milli, second.capacity_milli);
    }

    #[test]
    fn different_seeds_produce_different_terrain() {
        let first = generate(&SimConfig::phase1_default(1)).unwrap();
        let second = generate(&SimConfig::phase1_default(2)).unwrap();
        assert_ne!(first.terrain_checksum, second.terrain_checksum);
    }

    #[test]
    fn boundary_ring_is_water_and_land_is_connected() {
        let config = SimConfig::phase1_default(0x5eed_cafe_f00d_beef);
        let terrain = generate(&config).unwrap();
        for cell_x in 0..terrain.cells_x {
            assert!(!terrain.land[terrain.cell_index(cell_x, 0)]);
            assert!(!terrain.land[terrain.cell_index(cell_x, terrain.cells_y - 1)]);
        }
        for cell_y in 0..terrain.cells_y {
            assert!(!terrain.land[terrain.cell_index(0, cell_y)]);
            assert!(!terrain.land[terrain.cell_index(terrain.cells_x - 1, cell_y)]);
        }
        assert!(terrain.habitable_cells > 0);
        // Water cells never carry capacity.
        for index in 0..terrain.cell_count() {
            if !terrain.land[index] {
                assert_eq!(terrain.capacity_milli[index], 0);
            } else {
                assert!(terrain.capacity_milli[index] <= config.cell_capacity_milli);
            }
        }
    }

    #[test]
    fn suitability_is_bounded_and_peaks_mid_band() {
        let threshold = 17_000;
        assert_eq!(suitability_q16(threshold, threshold), 0);
        assert_eq!(suitability_q16(Q16_ONE, threshold), 0);
        let mid = threshold + (Q16_ONE - threshold) / 2;
        assert!(suitability_q16(mid, threshold) > Q16_ONE / 2);
    }
}
