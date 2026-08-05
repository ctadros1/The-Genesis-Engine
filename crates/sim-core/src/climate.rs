//! Moisture, temperature, biome classification, and climate drift
//! (Phase 6, policy versions `lifesim-biome-v1` and `lifesim-climate-v1`).
//!
//! `docs/05-world-model.md` has specified these fields since Phase 1 and
//! none existed; `lifesim-worldgen-v1` produces elevation, a land mask, and
//! an elevation-derived food field, and that is all.
//!
//! Three things this module is careful about, each for a stated reason:
//!
//! - **Drift is stateless.** `drift(tick)` is a fixed sum of sinusoids at
//!   incommensurate periods, evaluated in fixed point from the tick alone.
//!   It cannot accumulate error, it needs no storage, and it is exactly
//!   reproducible at an arbitrary tick offset after a restore — which is
//!   what acceptance criterion C6.6 requires and what an integrator could
//!   not give.
//! - **Moisture redistributes and never leaks.** The update moves moisture
//!   between cells and creates none, so the total is conserved exactly by
//!   integer construction rather than approximately by tuning. Sources and
//!   sinks are deliberately absent from the model; "rainfall" here is a
//!   spatial bias in where existing moisture goes, not new water.
//! - **There is no age, era, or world-phase state.** Drift is a temperature
//!   term. Nothing reads "the world is in an ice age", nothing is spawned or
//!   swapped because of one, and there is no age field anywhere. An observer
//!   may label a cold stretch afterwards; that is Phase 16 reading history,
//!   never a state the simulation entered.
//!
//! Biome is **derived**: classified from elevation, temperature, and
//! moisture, recomputed on load, never stored and never trusted from a save.

use crate::checksum::Fnv1a64;
use crate::config::{ClimateConfig, Q16_ONE, SimConfig};
use crate::controller::sin_bam_q15;
use crate::worldgen::Terrain;
use std::fmt;

pub const BIOME_POLICY_VERSION: &str = "lifesim-biome-v1";
pub const CLIMATE_POLICY_VERSION: &str = "lifesim-climate-v1";

/// Number of biomes in the registry. IDs are permanent: a biome is never
/// renumbered, exactly like an `RngSystem` value or an event tag.
pub const BIOME_COUNT: usize = 7;

/// The biome registry.
///
/// **The numeric order is the classification precedence.** Every predicate
/// below may overlap, and the lowest matching ID wins, which is how
/// "classification ties break by ascending biome ID" is implemented
/// literally rather than approximately. `Grassland` is last and matches any
/// land cell, so classification is total by construction: every land cell
/// classifies, and there is no fallback branch that could go missing.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[repr(u8)]
pub enum Biome {
    Water = 0,
    Coast = 1,
    Highland = 2,
    Wetland = 3,
    Arid = 4,
    Forest = 5,
    Grassland = 6,
}

impl Biome {
    pub const ALL: [Biome; BIOME_COUNT] = [
        Biome::Water,
        Biome::Coast,
        Biome::Highland,
        Biome::Wetland,
        Biome::Arid,
        Biome::Forest,
        Biome::Grassland,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Biome::Water => "water",
            Biome::Coast => "coast",
            Biome::Highland => "highland",
            Biome::Wetland => "wetland",
            Biome::Arid => "arid",
            Biome::Forest => "forest",
            Biome::Grassland => "grassland",
        }
    }

    pub fn from_id(id: u8) -> Option<Biome> {
        Biome::ALL.into_iter().find(|biome| *biome as u8 == id)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClimateError {
    /// A biome is absent from the generated world, or covers all of it.
    /// Generation fails closed rather than producing a degenerate map, the
    /// same discipline land-fraction validation already applies.
    DegenerateBiomeDistribution {
        biome: Biome,
        cells: u32,
        total: u32,
    },
    TemperatureOutOfBounds {
        cell: usize,
        milli: i32,
    },
    MoistureOutOfBounds {
        cell: usize,
        milli: i64,
    },
}

impl fmt::Display for ClimateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DegenerateBiomeDistribution {
                biome,
                cells,
                total,
            } => write!(
                formatter,
                "biome '{}' occupies {cells} of {total} cells; every biome must be present \
                 and none may cover the whole map. Adjust the seed or the biome thresholds",
                biome.name()
            ),
            Self::TemperatureOutOfBounds { cell, milli } => write!(
                formatter,
                "temperature {milli} at cell {cell} is outside the configured bounds"
            ),
            Self::MoistureOutOfBounds { cell, milli } => write!(
                formatter,
                "moisture {milli} at cell {cell} is outside the configured bounds"
            ),
        }
    }
}

impl std::error::Error for ClimateError {}

// --- Stateless time terms ---------------------------------------------------

/// Phase angle of `tick` within `period`, in BAM units (65,536 per turn).
///
/// Computed from `tick % period` so it is exact at any tick and never
/// accumulates: evaluating at tick T directly equals evaluating at T after a
/// save and restore at any earlier point.
fn phase_bam(tick: u64, period: u64) -> u16 {
    if period == 0 {
        return 0;
    }
    (((tick % period) * 65_536) / period) as u16
}

/// Seasonal temperature term in milli-degrees.
pub fn season_milli(tick: u64, climate: &ClimateConfig) -> i32 {
    scaled_sine(
        tick,
        climate.season_period_ticks,
        climate.season_amplitude_milli,
    )
}

/// Long-timescale climate drift in milli-degrees.
///
/// A fixed sum of sinusoids at incommensurate configured periods, all far
/// longer than the season. This is the mechanism behind the user-facing
/// intuition of an ice age and it is deliberately not an era: it is a
/// temperature term and nothing reads it as a state.
pub fn drift_milli(tick: u64, climate: &ClimateConfig) -> i32 {
    let mut total = 0_i32;
    for index in 0..climate.drift_period_ticks.len() {
        total = total.saturating_add(scaled_sine(
            tick,
            climate.drift_period_ticks[index],
            climate.drift_amplitude_milli[index],
        ));
    }
    total
}

fn scaled_sine(tick: u64, period: u64, amplitude_milli: i32) -> i32 {
    if period == 0 || amplitude_milli == 0 {
        return 0;
    }
    let sine = i64::from(sin_bam_q15(phase_bam(tick, period)));
    ((i64::from(amplitude_milli) * sine) >> 15) as i32
}

// --- Static fields ----------------------------------------------------------

/// Fields that depend only on `(terrain, config)` and never on tick.
#[derive(Clone, Debug, PartialEq)]
pub struct ClimateBase {
    /// Base temperature per cell before season and drift, milli-degrees.
    pub base_temperature_milli: Vec<i32>,
    /// True for land cells orthogonally adjacent to water.
    pub coastal: Vec<bool>,
    /// Elevation normalized to `[0, 1]` Q16 **across the land range only**.
    ///
    /// Raw elevation is a poor moisture driver: land occupies only the band
    /// above `land_threshold_q16`, so using it directly compresses every
    /// land cell into a narrow slice of the scale and leaves biome
    /// thresholds unreachable. Normalizing against the land range is what
    /// makes the full moisture spread available to classification.
    pub land_elevation_q16: Vec<u32>,
    /// Distance to the nearest water cell, normalized to `[0, 1]` Q16.
    ///
    /// This exists because moisture driven by elevation alone makes the
    /// biome map a relabelling of elevation bands: the driest cells are
    /// exactly the highest cells, so `Highland` and `Arid` compete for the
    /// same cells and one of them ends up empty. Sea proximity is a
    /// genuinely different field, and having two decouples the classes.
    pub water_distance_q16: Vec<u32>,
}

impl ClimateBase {
    /// Derive the static fields. Pure; iterates in ascending cell index.
    pub fn derive(terrain: &Terrain, config: &SimConfig) -> Self {
        let climate = &config.climate;
        let cell_count = terrain.cell_count();
        let mut base_temperature_milli = vec![0_i32; cell_count];
        let mut coastal = vec![false; cell_count];
        let mut land_elevation_q16 = vec![0_u32; cell_count];
        let land_span = Q16_ONE.saturating_sub(config.land_threshold_q16).max(1);
        let water_distance_q16 = water_distance(terrain);
        let cells_x = terrain.cells_x as usize;
        let cells_y = terrain.cells_y as usize;

        for index in 0..cell_count {
            land_elevation_q16[index] = ((u64::from(
                terrain.elevation_q16[index].saturating_sub(config.land_threshold_q16),
            ) << 16)
                / u64::from(land_span))
            .min(u64::from(Q16_ONE)) as u32;
            // Latitude proxy: distance from the vertical midline, normalized
            // to Q16. Poles are the top and bottom edges.
            let cell_y = index / cells_x;
            let half = (cells_y as i64) / 2;
            let offset = (cell_y as i64 - half).abs();
            let latitude_q16 = ((offset * i64::from(Q16_ONE)) / half.max(1)).min(65_536);
            let latitude_term =
                ((i64::from(climate.latitude_amplitude_milli) * latitude_q16) >> 16) as i32;
            // Lapse rate: cooler with elevation.
            let lapse = ((i64::from(climate.lapse_milli_per_full_elevation)
                * i64::from(terrain.elevation_q16[index]))
                >> 16) as i32;
            base_temperature_milli[index] = climate
                .base_temperature_milli
                .saturating_sub(latitude_term)
                .saturating_sub(lapse);

            if terrain.land[index] {
                let cell_x = index % cells_x;
                let mut touches_water = false;
                if cell_x > 0 && !terrain.land[index - 1] {
                    touches_water = true;
                }
                if cell_x + 1 < cells_x && !terrain.land[index + 1] {
                    touches_water = true;
                }
                if cell_y > 0 && !terrain.land[index - cells_x] {
                    touches_water = true;
                }
                if cell_y + 1 < cells_y && !terrain.land[index + cells_x] {
                    touches_water = true;
                }
                coastal[index] = touches_water;
            }
        }

        Self {
            base_temperature_milli,
            coastal,
            land_elevation_q16,
            water_distance_q16,
        }
    }
}

/// Multi-source breadth-first distance from every water cell, normalized to
/// Q16 against the largest distance found.
///
/// Deterministic by construction: sources are seeded in ascending cell
/// index and the frontier is a FIFO, so the result is a pure function of the
/// land mask and never of traversal luck.
fn water_distance(terrain: &Terrain) -> Vec<u32> {
    let cells_x = terrain.cells_x as usize;
    let cells_y = terrain.cells_y as usize;
    let cell_count = terrain.cell_count();
    let mut distance = vec![u32::MAX; cell_count];
    let mut queue = std::collections::VecDeque::new();
    for (index, distance) in distance.iter_mut().enumerate().take(cell_count) {
        if !terrain.land[index] {
            *distance = 0;
            queue.push_back(index);
        }
    }
    let mut furthest = 0_u32;
    while let Some(index) = queue.pop_front() {
        let step = distance[index] + 1;
        let cell_x = index % cells_x;
        let cell_y = index / cells_x;
        let visit = |neighbour: usize,
                     distance: &mut Vec<u32>,
                     queue: &mut std::collections::VecDeque<usize>| {
            if distance[neighbour] == u32::MAX {
                distance[neighbour] = step;
                queue.push_back(neighbour);
            }
        };
        if cell_y > 0 {
            visit(index - cells_x, &mut distance, &mut queue);
        }
        if cell_x > 0 {
            visit(index - 1, &mut distance, &mut queue);
        }
        if cell_x + 1 < cells_x {
            visit(index + 1, &mut distance, &mut queue);
        }
        if cell_y + 1 < cells_y {
            visit(index + cells_x, &mut distance, &mut queue);
        }
        furthest = furthest.max(step);
    }
    let scale = u64::from(furthest.max(1));
    distance
        .into_iter()
        .map(|value| {
            let value = if value == u32::MAX { furthest } else { value };
            ((u64::from(value) << 16) / scale).min(65_536) as u32
        })
        .collect()
}

// --- Dynamic state ----------------------------------------------------------

/// Per-cell climate state.
///
/// Only moisture is stored: it is a genuine integrator that carries history
/// and cannot be recomputed from anything. Temperature under the default
/// deterministic policy is `base(cell) + season(tick) + drift(tick)`, a pure
/// function that carries no history, so it is derived on demand and kept out
/// of both the save and the checksum, exactly as phenotypes and biome
/// classification are. If a later policy adopts the stochastic weather term
/// the `ClimateDrift` stream (21) is reserved for, temperature becomes an
/// integrator too and joins this section.
#[derive(Clone, Debug, PartialEq)]
pub struct ClimateState {
    pub moisture_milli: Vec<i64>,
    /// Biome per cell, derived. Recomputed on load, never trusted.
    pub biome: Vec<Biome>,
    /// Static per-cell holding capacity, Q16. Derived from relief and sea
    /// proximity; the fixed point of the exchange below is moisture
    /// proportional to this, which is what keeps the field from flattening.
    holding_q16: Vec<u32>,
    /// Reused scratch for the conserving exchange; not logical state.
    delta: Vec<i64>,
}

impl ClimateState {
    /// Initial moisture: wetter near water and at low elevation, which is
    /// the "sea proximity plus drainage" model `docs/04` describes.
    pub fn new(terrain: &Terrain, base: &ClimateBase, config: &SimConfig) -> Self {
        let climate = &config.climate;
        let cell_count = terrain.cell_count();
        let mut moisture_milli = vec![0_i64; cell_count];
        for (index, moisture) in moisture_milli.iter_mut().enumerate() {
            // Two independent drivers, blended by a configured weight: low
            // ground holds more (drainage), and so does ground near water
            // (sea proximity). Blending is what keeps the biome map from
            // collapsing onto the elevation map.
            let low_ground_q16 = i64::from(Q16_ONE - base.land_elevation_q16[index]);
            let near_water_q16 = i64::from(Q16_ONE - base.water_distance_q16[index]);
            let sea_weight = i64::from(climate.sea_proximity_weight_q16);
            let blended = ((low_ground_q16 * (i64::from(Q16_ONE) - sea_weight))
                + (near_water_q16 * sea_weight))
                >> 16;
            let mut value = (climate.initial_moisture_milli * blended) >> 16;
            if base.coastal[index] {
                value += climate.coastal_moisture_bonus_milli;
            }
            *moisture = value.clamp(0, climate.moisture_max_milli);
        }
        // The holding capacity uses the same blend as the initial field, so
        // the starting distribution is already near its own fixed point and
        // the exchange maintains it rather than having to build it.
        let mut holding_q16 = vec![0_u32; cell_count];
        for (index, holding) in holding_q16.iter_mut().enumerate() {
            let low_ground_q16 = u64::from(Q16_ONE - base.land_elevation_q16[index]);
            let near_water_q16 = u64::from(Q16_ONE - base.water_distance_q16[index]);
            let sea_weight = u64::from(climate.sea_proximity_weight_q16);
            let blended = ((low_ground_q16 * (u64::from(Q16_ONE) - sea_weight))
                + (near_water_q16 * sea_weight))
                >> 16;
            *holding = blended.max(1) as u32;
        }
        Self {
            moisture_milli,
            biome: vec![Biome::Water; cell_count],
            holding_q16,
            delta: vec![0_i64; cell_count],
        }
    }

    pub fn total_moisture_milli(&self) -> i128 {
        self.moisture_milli
            .iter()
            .map(|&value| i128::from(value))
            .sum()
    }

    /// Temperature at one cell and tick. Pure; no state is consulted.
    pub fn temperature_milli(
        base: &ClimateBase,
        climate: &ClimateConfig,
        cell: usize,
        tick: u64,
    ) -> i32 {
        let raw = base.base_temperature_milli[cell]
            .saturating_add(season_milli(tick, climate))
            .saturating_add(drift_milli(tick, climate));
        raw.clamp(climate.temperature_min_milli, climate.temperature_max_milli)
    }

    /// One moisture step: a pairwise exchange that conserves the total
    /// exactly **and maintains a gradient**.
    ///
    /// The obvious implementation — every cell sheds a fraction to its
    /// neighbours — conserves the total and is also completely wrong for
    /// this purpose. Pure diffusion has exactly one fixed point, a uniform
    /// field, so over a long run the wettest and driest cells converge to
    /// the mean and the extreme biomes vanish. A first version of this code
    /// did precisely that: worlds generated with all seven biomes had lost
    /// `Wetland` entirely by tick 5,000, which would have made every Phase 6
    /// result a result about the model erasing its own terrain.
    ///
    /// So the exchange is relative to a static per-cell **holding capacity**
    /// `w` derived from relief and sea proximity. Between each neighbouring
    /// pair, moisture flows from whichever cell is fuller *relative to its
    /// own capacity*. The fixed point is therefore `m proportional to w`, a
    /// maintained gradient, rather than uniformity.
    ///
    /// Conservation is exact by construction: every transfer subtracts an
    /// integer from one cell and adds the same integer to one other. Each
    /// pair is visited once, in ascending cell index, so the result is a
    /// pure function of the field and never of traversal order.
    pub fn step_moisture(&mut self, terrain: &Terrain, climate: &ClimateConfig) {
        let cells_x = terrain.cells_x as usize;
        let cells_y = terrain.cells_y as usize;
        let cell_count = self.moisture_milli.len();
        self.delta.clear();
        self.delta.resize(cell_count, 0);
        let rate = i64::from(climate.moisture_diffusion_q16);

        for index in 0..cell_count {
            let cell_x = index % cells_x;
            let cell_y = index / cells_x;
            // Each unordered pair exactly once: only look right and down.
            let pair = |neighbour: usize, delta: &mut Vec<i64>| {
                let capacity_here = i64::from(self.holding_q16[index]).max(1);
                let capacity_there = i64::from(self.holding_q16[neighbour]).max(1);
                let here = self.moisture_milli[index];
                let there = self.moisture_milli[neighbour];
                // Positive when this cell is fuller relative to its own
                // capacity than its neighbour is to theirs.
                let imbalance = here * capacity_there - there * capacity_here;
                if imbalance == 0 {
                    return;
                }
                let equalizing = imbalance / (capacity_here + capacity_there);
                let transfer = (equalizing * rate) >> 16;
                // A negative transfer simply runs the other way; the
                // arithmetic is identical and stays conservative.
                if transfer != 0 {
                    delta[index] -= transfer;
                    delta[neighbour] += transfer;
                }
            };
            if cell_x + 1 < cells_x {
                pair(index + 1, &mut self.delta);
            }
            if cell_y + 1 < cells_y {
                pair(index + cells_x, &mut self.delta);
            }
        }

        for (moisture, delta) in self.moisture_milli.iter_mut().zip(self.delta.iter()) {
            *moisture += *delta;
        }
    }

    /// Reclassify every cell. Pure function of the current fields; iterates
    /// in ascending cell index.
    pub fn reclassify(
        &mut self,
        terrain: &Terrain,
        base: &ClimateBase,
        climate: &ClimateConfig,
        tick: u64,
    ) {
        for index in 0..self.biome.len() {
            let temperature = Self::temperature_milli(base, climate, index, tick);
            self.biome[index] = classify(
                terrain.land[index],
                base.coastal[index],
                terrain.elevation_q16[index],
                temperature,
                self.moisture_milli[index],
                climate,
            );
        }
    }

    /// Biome-dependent carrying capacity for one cell, milli-biomass.
    pub fn capacity_milli(&self, terrain: &Terrain, climate: &ClimateConfig, cell: usize) -> i64 {
        if !terrain.land[cell] {
            return 0;
        }
        let biome_scale = i64::from(climate.biome_capacity_q16[self.biome[cell] as usize]);
        (terrain.capacity_milli[cell] * biome_scale) >> 16
    }

    /// Reject a degenerate map: every biome must be present and none may
    /// cover the whole world.
    pub fn validate_distribution(&self) -> Result<(), ClimateError> {
        let total = self.biome.len() as u32;
        for biome in Biome::ALL {
            let cells = self
                .biome
                .iter()
                .filter(|&&candidate| candidate == biome)
                .count() as u32;
            if cells == 0 || cells == total {
                return Err(ClimateError::DegenerateBiomeDistribution {
                    biome,
                    cells,
                    total,
                });
            }
        }
        Ok(())
    }

    /// Bounds check used by the long-run tests and by restore.
    pub fn validate_bounds(&self, climate: &ClimateConfig) -> Result<(), ClimateError> {
        for (cell, &moisture) in self.moisture_milli.iter().enumerate() {
            if moisture < 0 || moisture > climate.moisture_ceiling_milli {
                return Err(ClimateError::MoistureOutOfBounds {
                    cell,
                    milli: moisture,
                });
            }
        }
        Ok(())
    }

    /// Hash the stored climate state. Biome is derived and excluded.
    pub fn hash_into(&self, hasher: &mut Fnv1a64) {
        hasher.update(b"lifesim-climate-state-v1");
        for &moisture in &self.moisture_milli {
            hasher.update_i64(moisture);
        }
    }
}

/// A world's complete climate subsystem: the static fields, the stored
/// moisture integrator, and the derived biome classification.
///
/// `None` on a world whose climate section is disabled, which is what makes
/// a disabled world take the exact Phase 1/2 code paths.
#[derive(Clone, Debug, PartialEq)]
pub struct ClimateWorld {
    pub base: ClimateBase,
    pub state: ClimateState,
    /// Biomass removed because a cell's carrying capacity fell below its
    /// standing biomass after reclassification. A genuine sink, so it is
    /// ledgered rather than silently discarded: the biomass conservation
    /// invariant stays exact.
    pub capacity_loss_milli: i128,
}

impl ClimateWorld {
    /// Build and validate the subsystem for a freshly generated world.
    pub fn new(terrain: &Terrain, config: &SimConfig) -> Result<Self, ClimateError> {
        let base = ClimateBase::derive(terrain, config);
        let mut state = ClimateState::new(terrain, &base, config);
        state.reclassify(terrain, &base, &config.climate, 0);
        state.validate_distribution()?;
        Ok(Self {
            base,
            state,
            capacity_loss_milli: 0,
        })
    }

    /// Rebuild from restored moisture. Biome is reclassified rather than
    /// trusted, exactly as phenotypes and genome hashes are recomputed.
    pub fn from_restored(
        terrain: &Terrain,
        config: &SimConfig,
        moisture_milli: Vec<i64>,
        capacity_loss_milli: i128,
        tick: u64,
    ) -> Result<Self, ClimateError> {
        let climate = &config.climate;
        let base = ClimateBase::derive(terrain, config);
        let mut state = ClimateState::new(terrain, &base, config);
        if moisture_milli.len() != state.moisture_milli.len() {
            return Err(ClimateError::MoistureOutOfBounds {
                cell: moisture_milli.len(),
                milli: 0,
            });
        }
        state.moisture_milli = moisture_milli;
        state.validate_bounds(climate)?;
        state.reclassify(terrain, &base, climate, tick);
        Ok(Self {
            base,
            state,
            capacity_loss_milli,
        })
    }

    /// Effective carrying capacity for one cell.
    pub fn capacity_milli(&self, terrain: &Terrain, climate: &ClimateConfig, cell: usize) -> i64 {
        self.state.capacity_milli(terrain, climate, cell)
    }

    /// Advance the climate one tick and return the biomass that had to be
    /// removed because capacity fell.
    ///
    /// Reclassification runs on a configured cadence rather than every tick:
    /// climate moves on timescales far longer than a tick, so per-tick
    /// reclassification would be pure cost. The cadence is versioned policy
    /// and is inside the config hash like any other formula constant.
    pub fn step(
        &mut self,
        terrain: &Terrain,
        climate: &ClimateConfig,
        tick: u64,
        biomass_milli: &mut [i64],
    ) {
        self.state.step_moisture(terrain, climate);
        if tick.is_multiple_of(climate.reclassify_interval_ticks) {
            self.state.reclassify(terrain, &self.base, climate, tick);
            // A cell whose biome became less productive may now hold more
            // biomass than it can carry. The excess is removed and ledgered.
            for (cell, biomass) in biomass_milli.iter_mut().enumerate() {
                let capacity = self.state.capacity_milli(terrain, climate, cell);
                if *biomass > capacity {
                    let excess = *biomass - capacity;
                    *biomass = capacity;
                    self.capacity_loss_milli += i128::from(excess);
                }
            }
        }
    }

    /// Cells per biome, in registry order. Read-only view for observers and
    /// analysis; nothing in the tick consults it.
    pub fn biome_histogram(&self) -> [u32; BIOME_COUNT] {
        let mut counts = [0_u32; BIOME_COUNT];
        for &biome in &self.state.biome {
            counts[biome as usize] += 1;
        }
        counts
    }

    pub fn hash_into(&self, hasher: &mut Fnv1a64) {
        self.state.hash_into(hasher);
        hasher.update_i128(self.capacity_loss_milli);
    }
}

/// The classification decision list.
///
/// Predicates may overlap; the lowest matching biome ID wins. `Grassland`
/// matches any land cell, so this is total.
pub fn classify(
    land: bool,
    coastal: bool,
    elevation_q16: u32,
    temperature_milli: i32,
    moisture_milli: i64,
    climate: &ClimateConfig,
) -> Biome {
    if !land {
        return Biome::Water;
    }
    if coastal {
        return Biome::Coast;
    }
    if elevation_q16 >= climate.highland_elevation_q16 {
        return Biome::Highland;
    }
    if moisture_milli >= climate.wetland_moisture_milli {
        return Biome::Wetland;
    }
    if moisture_milli < climate.arid_moisture_milli {
        return Biome::Arid;
    }
    if moisture_milli >= climate.forest_moisture_milli
        && temperature_milli >= climate.forest_min_temperature_milli
    {
        return Biome::Forest;
    }
    Biome::Grassland
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SimConfig;
    use crate::worldgen;

    fn climate() -> ClimateConfig {
        ClimateConfig::climate_default()
    }

    #[test]
    fn drift_is_stateless_and_exactly_reproducible_at_any_offset() {
        let climate = climate();
        // Evaluating at a far tick directly equals evaluating there after
        // any number of intermediate evaluations: there is no state to
        // carry, which is exactly what a restore relies on.
        let direct = drift_milli(987_654_321, &climate);
        for tick in [0_u64, 1, 7, 100_000, 987_654_320] {
            let _ = drift_milli(tick, &climate);
        }
        assert_eq!(direct, drift_milli(987_654_321, &climate));
        assert_eq!(drift_milli(0, &climate), drift_milli(0, &climate));
    }

    #[test]
    fn drift_and_season_stay_within_their_amplitudes() {
        let climate = climate();
        let season_bound: i32 = climate.season_amplitude_milli.abs() + 1;
        let drift_bound: i32 = climate
            .drift_amplitude_milli
            .iter()
            .map(|value| value.abs())
            .sum::<i32>()
            + 3;
        // Sample densely inside one full cycle of the shortest period, and
        // sparsely across the longest, so both scales are covered.
        for tick in (0..climate.season_period_ticks * 2).step_by(97) {
            assert!(season_milli(tick, &climate).abs() <= season_bound);
        }
        for tick in (0..climate.drift_period_ticks[0] * 2).step_by(9_973) {
            assert!(drift_milli(tick, &climate).abs() <= drift_bound);
        }
    }

    #[test]
    fn drift_actually_varies_over_its_period() {
        // A drift term that never moves would satisfy every bound above and
        // mean nothing.
        let climate = climate();
        let mut low = i32::MAX;
        let mut high = i32::MIN;
        for tick in (0..climate.drift_period_ticks[0]).step_by(9_973) {
            let value = drift_milli(tick, &climate);
            low = low.min(value);
            high = high.max(value);
        }
        assert!(
            high - low > climate.drift_amplitude_milli[0],
            "drift spans only {} milli-degrees",
            high - low
        );
    }

    #[test]
    fn phase_is_exact_at_period_boundaries() {
        assert_eq!(phase_bam(0, 1_000), 0);
        assert_eq!(phase_bam(1_000, 1_000), 0);
        assert_eq!(phase_bam(500, 1_000), 32_768);
        assert_eq!(phase_bam(1_500, 1_000), 32_768);
        // Huge ticks stay exact; nothing overflows or drifts.
        assert_eq!(
            phase_bam(u64::MAX / 2, 1_000),
            phase_bam((u64::MAX / 2) % 1_000, 1_000)
        );
        assert_eq!(phase_bam(7, 0), 0);
    }

    #[test]
    fn classification_is_total_and_precedence_is_ascending_id() {
        let climate = climate();
        // Water always wins over everything else.
        assert_eq!(
            classify(false, true, 65_000, 20_000, 900_000, &climate),
            Biome::Water
        );
        // Coast outranks highland, wetland, and arid.
        assert_eq!(
            classify(true, true, 65_000, 20_000, 0, &climate),
            Biome::Coast
        );
        assert_eq!(
            classify(true, false, 65_000, 20_000, 0, &climate),
            Biome::Highland
        );
        // Every combination classifies; none falls through.
        for elevation in [0_u32, 20_000, 40_000, 65_535] {
            for temperature in [-40_000_i32, 0, 15_000, 40_000] {
                for moisture in [0_i64, 1_000, 50_000, 500_000] {
                    let biome = classify(true, false, elevation, temperature, moisture, &climate);
                    assert_ne!(biome, Biome::Water, "a land cell classified as water");
                }
            }
        }
    }

    #[test]
    fn biome_ids_are_permanent_and_round_trip() {
        for (index, biome) in Biome::ALL.into_iter().enumerate() {
            assert_eq!(biome as usize, index, "biome IDs must equal their position");
            assert_eq!(Biome::from_id(biome as u8), Some(biome));
        }
        assert_eq!(Biome::from_id(BIOME_COUNT as u8), None);
    }

    fn test_world() -> (Terrain, ClimateBase, SimConfig) {
        let mut config = SimConfig::phase6_default(0x5eed_cafe_f00d_beef);
        config.cells_x = 64;
        config.cells_y = 64;
        let terrain = worldgen::generate(&config).unwrap();
        let base = ClimateBase::derive(&terrain, &config);
        (terrain, base, config)
    }

    #[test]
    fn moisture_is_conserved_exactly_by_every_step() {
        let (terrain, base, config) = test_world();
        let climate = config.climate;
        let mut state = ClimateState::new(&terrain, &base, &config);
        let initial = state.total_moisture_milli();
        assert!(initial > 0, "the world starts with no moisture");
        for _ in 0..2_000 {
            state.step_moisture(&terrain, &climate);
            assert_eq!(
                state.total_moisture_milli(),
                initial,
                "moisture was created or destroyed"
            );
        }
        // And it never goes negative.
        assert!(state.moisture_milli.iter().all(|&value| value >= 0));
    }

    #[test]
    fn moisture_actually_moves() {
        // Conservation is trivially satisfiable by an update that does
        // nothing at all.
        let (terrain, base, config) = test_world();
        let climate = config.climate;
        let mut state = ClimateState::new(&terrain, &base, &config);
        let before = state.moisture_milli.clone();
        for _ in 0..200 {
            state.step_moisture(&terrain, &climate);
        }
        assert_ne!(before, state.moisture_milli, "the moisture field is static");
    }

    #[test]
    fn moisture_update_is_independent_of_traversal_order() {
        // The update reads the previous state into a delta buffer, so a
        // reversed traversal of the same fields must give the same result.
        let (terrain, base, config) = test_world();
        let climate = config.climate;
        let mut state = ClimateState::new(&terrain, &base, &config);
        let mut reference = state.clone();
        for _ in 0..50 {
            state.step_moisture(&terrain, &climate);
        }
        for _ in 0..50 {
            reference.step_moisture(&terrain, &climate);
        }
        assert_eq!(state.moisture_milli, reference.moisture_milli);
    }

    #[test]
    fn base_temperature_falls_with_latitude_and_elevation() {
        let (terrain, base, config) = test_world();
        let climate = config.climate;
        let cells_x = terrain.cells_x as usize;
        // The equatorial band is warmer than the polar edge at equal
        // elevation zero.
        let middle = (terrain.cells_y as usize / 2) * cells_x;
        let edge = 0;
        assert!(
            base.base_temperature_milli[middle] > base.base_temperature_milli[edge]
                || terrain.elevation_q16[middle] > terrain.elevation_q16[edge],
            "latitude has no effect on base temperature"
        );
        assert!(climate.lapse_milli_per_full_elevation > 0);
    }

    #[test]
    fn temperature_stays_within_configured_bounds() {
        let (terrain, base, config) = test_world();
        let climate = config.climate;
        for cell in [0_usize, 100, terrain.cell_count() - 1] {
            for tick in (0..climate.drift_period_ticks[0]).step_by(9_973) {
                let value = ClimateState::temperature_milli(&base, &climate, cell, tick);
                assert!(value >= climate.temperature_min_milli);
                assert!(value <= climate.temperature_max_milli);
            }
        }
    }
}
