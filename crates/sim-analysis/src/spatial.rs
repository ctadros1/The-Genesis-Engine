//! World-level spatial structure indices (`lifesim-spatial-index-v1`).
//!
//! Phase 7's primary endpoint C7.1 asks whether contest changes "a
//! world-level index of spatial aggregation and of encounter avoidance".
//! Two properties decide what that index has to be, and both are forced by
//! the data rather than chosen for convenience.
//!
//! **It must not be a population measure in disguise.** The Phase 7
//! campaign found median populations of 48, 30, and 15 across conditions.
//! Any statistic whose expectation moves with N would separate the
//! conditions whether or not their spatial structure differed at all, which
//! is precisely the class of result this project exists not to produce. The
//! Morisita index of dispersion is the standard ecological answer: it is
//! the probability that two randomly chosen individuals fall in the same
//! quadrat, divided by that probability under random placement, and its
//! expectation is 1 under complete spatial randomness for any N.
//!
//! **Aggregation and encounter avoidance must be separable.** They are not
//! independent -- organisms that spread out score lower on both -- so the
//! encounter measure is computed *conditional on* the coarse-scale
//! distribution. Take Morisita at two nested scales. If organisms are
//! clustered among coarse quadrats but uniformly spread within them, then
//!
//!   I_fine = Q_fine * P(same fine quadrat)
//!          = (Q_coarse * k) * P(same coarse) / k
//!          = I_coarse
//!
//! for k fine quadrats per coarse one. So the ratio `R = I_fine / I_coarse`
//! is exactly 1 whenever fine-scale spacing is random given coarse-scale
//! position, below 1 when organisms hold each other at arm's length inside
//! a shared region, and above 1 when they clump more tightly than their
//! region alone explains. That ratio is the encounter-avoidance index.
//!
//! Three further properties fall out of the construction rather than
//! needing to be engineered:
//!
//! - **The denominators cancel.** `R = (Q_f * P_f) / (Q_c * P_c)`, where P
//!   is a pooled count of same-quadrat ordered pairs. R therefore does not
//!   depend on the total pair count at all.
//! - **It is exact.** Everything is integer counts; the indices are
//!   reported in milli-units computed with `i128`. No float appears
//!   anywhere, so a report reproduces bit-for-bit like everything else here.
//! - **No sample is ever filtered.** Samples with fewer than two organisms
//!   contribute zero to both numerator and denominator. A minimum-population
//!   filter would have been an exclusion rule correlated with the treatment
//!   -- the same defect `preflight` exists to prevent one level up -- and
//!   pooling removes the need for one.
//!
//! Quadrats are counted over **habitable** area only: a quadrat enters `Q`
//! when it contains at least one land cell. The kernel's own invariant
//! check requires every organism to stand on land, so the null this
//! normalizes against is "distributed at random over the land available",
//! which is the only null that means anything on a continent map.

use sim_core::{FP_PER_METER, Terrain};
use sim_persist::SpatialSample;

pub const SPATIAL_INDEX_VERSION: &str = "lifesim-spatial-index-v1";

/// Prespecified analysis scales, in terrain cells.
///
/// At the default `cell_size_m = 4` these are 8 m and 64 m. The fine scale
/// is chosen to sit inside the band over which organisms actually interact
/// -- attack range 3 m, pairing range 4 m, crowding radius 6 m, sensor
/// range at most 12 m -- so "same fine quadrat" means "close enough to
/// matter". The coarse scale is 8 times larger, giving 64 nested fine
/// quadrats per coarse one.
pub const FINE_SCALE_CELLS: u32 = 2;
pub const COARSE_SCALE_CELLS: u32 = 16;

/// Why a world produced no index.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IndexRefusal {
    /// No sample after burn-in held two organisms at once, so no pair ever
    /// existed to be counted.
    NoPairsObserved,
    /// Pairs existed but none ever shared a coarse quadrat, leaving the
    /// encounter ratio's denominator zero.
    NoCoarsePairsObserved,
    /// The sample series was empty after burn-in.
    NoSamples,
    /// The scales do not nest, so the ratio has no conditional reading.
    ScalesDoNotNest { fine: u32, coarse: u32 },
}

impl std::fmt::Display for IndexRefusal {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

/// A quadrat grid over the terrain at one scale.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QuadratGrid {
    pub scale_cells: u32,
    pub quadrats_x: u32,
    pub quadrats_y: u32,
    /// Quadrats containing at least one land cell. This is Morisita's `Q`.
    pub habitable_quadrats: u64,
    habitable: Vec<bool>,
}

impl QuadratGrid {
    pub fn new(terrain: &Terrain, scale_cells: u32) -> Self {
        let scale = scale_cells.max(1);
        let quadrats_x = terrain.cells_x.div_ceil(scale);
        let quadrats_y = terrain.cells_y.div_ceil(scale);
        let mut habitable = vec![false; (quadrats_x as usize) * (quadrats_y as usize)];
        for cell_y in 0..terrain.cells_y {
            for cell_x in 0..terrain.cells_x {
                if terrain.land[terrain.cell_index(cell_x, cell_y)] {
                    let quadrat =
                        (cell_y / scale) as usize * quadrats_x as usize + (cell_x / scale) as usize;
                    habitable[quadrat] = true;
                }
            }
        }
        let habitable_quadrats = habitable.iter().filter(|value| **value).count() as u64;
        Self {
            scale_cells: scale,
            quadrats_x,
            quadrats_y,
            habitable_quadrats,
            habitable,
        }
    }

    pub fn len(&self) -> usize {
        self.habitable.len()
    }

    pub fn is_empty(&self) -> bool {
        self.habitable.is_empty()
    }
}

/// Pooled index for one world.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorldIndex {
    /// Samples that entered the pool (after burn-in).
    pub samples_used: u64,
    /// Total organism-observations pooled, for reporting precision.
    pub observations: u64,
    /// Ordered pairs pooled: `sum_t N_t (N_t - 1)`.
    pub ordered_pairs: u128,
    /// Ordered pairs sharing a coarse quadrat.
    pub coarse_pairs: u128,
    /// Ordered pairs sharing a fine quadrat.
    pub fine_pairs: u128,
    pub habitable_coarse_quadrats: u64,
    pub habitable_fine_quadrats: u64,
    /// Morisita index at the coarse scale, milli-units. 1000 == random.
    pub aggregation_milli: i64,
    /// Morisita index at the fine scale, milli-units.
    pub fine_milli: i64,
    /// `fine / coarse`, milli-units. 1000 == fine-scale spacing is random
    /// given coarse-scale position; below 1000 is encounter avoidance.
    pub encounter_milli: i64,
}

/// Compute both indices for one world's sample series.
///
/// `burn_in_ticks` drops the opening transient, during which the layout
/// still reflects how founders were placed rather than anything organisms
/// did. Samples exactly at `burn_in_ticks` are dropped; later ones are kept.
pub fn world_index(
    terrain: &Terrain,
    samples: &[SpatialSample],
    burn_in_ticks: u64,
    cell_size_m: u32,
    fine_scale_cells: u32,
    coarse_scale_cells: u32,
) -> Result<WorldIndex, IndexRefusal> {
    if coarse_scale_cells == 0
        || fine_scale_cells == 0
        || !coarse_scale_cells.is_multiple_of(fine_scale_cells)
        || coarse_scale_cells < fine_scale_cells
    {
        return Err(IndexRefusal::ScalesDoNotNest {
            fine: fine_scale_cells,
            coarse: coarse_scale_cells,
        });
    }
    let fine = QuadratGrid::new(terrain, fine_scale_cells);
    let coarse = QuadratGrid::new(terrain, coarse_scale_cells);

    let mut fine_counts = vec![0_u32; fine.len()];
    let mut coarse_counts = vec![0_u32; coarse.len()];
    let mut touched_fine: Vec<usize> = Vec::new();
    let mut touched_coarse: Vec<usize> = Vec::new();

    let mut samples_used = 0_u64;
    let mut observations = 0_u64;
    let mut ordered_pairs = 0_u128;
    let mut fine_pairs = 0_u128;
    let mut coarse_pairs = 0_u128;

    // Fixed-point sub-units per terrain cell. Positions are `i32` in units
    // of 1/1024 m, so this is exact.
    let fp_per_cell = i64::from(cell_size_m) * i64::from(FP_PER_METER);

    for sample in samples {
        if sample.tick <= burn_in_ticks {
            continue;
        }
        samples_used += 1;
        let population = sample.positions.len() as u128;
        observations += sample.positions.len() as u64;
        if population < 2 {
            // Contributes nothing to either side of the ratio, which is
            // why no population filter is needed.
            continue;
        }
        ordered_pairs += population * (population - 1);

        for &(x_fp, y_fp) in &sample.positions {
            let cell_x = ((i64::from(x_fp) / fp_per_cell).max(0) as u32)
                .min(terrain.cells_x.saturating_sub(1));
            let cell_y = ((i64::from(y_fp) / fp_per_cell).max(0) as u32)
                .min(terrain.cells_y.saturating_sub(1));

            let fine_index = (cell_y / fine.scale_cells) as usize * fine.quadrats_x as usize
                + (cell_x / fine.scale_cells) as usize;
            if fine_counts[fine_index] == 0 {
                touched_fine.push(fine_index);
            }
            fine_counts[fine_index] += 1;

            let coarse_index = (cell_y / coarse.scale_cells) as usize * coarse.quadrats_x as usize
                + (cell_x / coarse.scale_cells) as usize;
            if coarse_counts[coarse_index] == 0 {
                touched_coarse.push(coarse_index);
            }
            coarse_counts[coarse_index] += 1;
        }

        // Only touched quadrats are visited and cleared, so cost is O(N)
        // per sample rather than O(quadrats).
        for &index in &touched_fine {
            let count = u128::from(fine_counts[index]);
            fine_pairs += count * (count - 1);
            fine_counts[index] = 0;
        }
        touched_fine.clear();
        for &index in &touched_coarse {
            let count = u128::from(coarse_counts[index]);
            coarse_pairs += count * (count - 1);
            coarse_counts[index] = 0;
        }
        touched_coarse.clear();
    }

    if samples_used == 0 {
        return Err(IndexRefusal::NoSamples);
    }
    if ordered_pairs == 0 {
        return Err(IndexRefusal::NoPairsObserved);
    }
    if coarse_pairs == 0 {
        return Err(IndexRefusal::NoCoarsePairsObserved);
    }

    let aggregation_milli =
        ((u128::from(coarse.habitable_quadrats) * coarse_pairs * 1_000) / ordered_pairs) as i64;
    let fine_milli =
        ((u128::from(fine.habitable_quadrats) * fine_pairs * 1_000) / ordered_pairs) as i64;
    // The pooled pair total cancels out of the ratio entirely.
    let encounter_milli = ((u128::from(fine.habitable_quadrats) * fine_pairs * 1_000)
        / (u128::from(coarse.habitable_quadrats) * coarse_pairs)) as i64;

    Ok(WorldIndex {
        samples_used,
        observations,
        ordered_pairs,
        coarse_pairs,
        fine_pairs,
        habitable_coarse_quadrats: coarse.habitable_quadrats,
        habitable_fine_quadrats: fine.habitable_quadrats,
        aggregation_milli,
        fine_milli,
        encounter_milli,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim_core::SimConfig;

    /// An all-land square world, so `Q` is every quadrat and the expected
    /// values are exactly computable by hand.
    fn flat_terrain(cells: u32) -> Terrain {
        Terrain {
            cells_x: cells,
            cells_y: cells,
            elevation_q16: vec![40_000; (cells * cells) as usize],
            land: vec![true; (cells * cells) as usize],
            capacity_milli: vec![30_000; (cells * cells) as usize],
            land_cells: cells * cells,
            habitable_cells: cells * cells,
            terrain_checksum: 0,
        }
    }

    fn sample(tick: u64, positions: Vec<(i32, i32)>) -> SpatialSample {
        SpatialSample { tick, positions }
    }

    /// Position at the centre of cell `(x, y)` for `cell_size_m = 4`.
    fn at_cell(x: u32, y: u32) -> (i32, i32) {
        let fp_per_cell = 4 * FP_PER_METER;
        (
            (x as i32) * fp_per_cell + fp_per_cell / 2,
            (y as i32) * fp_per_cell + fp_per_cell / 2,
        )
    }

    fn index_of(terrain: &Terrain, samples: &[SpatialSample]) -> WorldIndex {
        world_index(terrain, samples, 0, 4, FINE_SCALE_CELLS, COARSE_SCALE_CELLS)
            .expect("index is defined")
    }

    #[test]
    fn perfectly_spread_organisms_score_below_random_at_both_scales() {
        // One organism per coarse quadrat: no two ever share a quadrat, so
        // both same-quadrat pair counts are zero and both indices are zero.
        let terrain = flat_terrain(64);
        let positions: Vec<(i32, i32)> = (0..4)
            .flat_map(|qy| (0..4).map(move |qx| at_cell(qx * 16 + 8, qy * 16 + 8)))
            .collect();
        assert_eq!(positions.len(), 16);
        let index = world_index(
            &terrain,
            &[sample(10, positions)],
            0,
            4,
            FINE_SCALE_CELLS,
            COARSE_SCALE_CELLS,
        );
        // No coarse pair exists at all, which the index refuses rather than
        // reporting a ratio with a zero denominator.
        assert_eq!(index, Err(IndexRefusal::NoCoarsePairsObserved));
    }

    #[test]
    fn one_tight_clump_scores_far_above_random() {
        // Everyone in a single fine quadrat: every pair shares both
        // quadrats, so I = Q at each scale.
        let terrain = flat_terrain(64);
        let positions: Vec<(i32, i32)> = (0..8).map(|_| at_cell(1, 1)).collect();
        let index = index_of(&terrain, &[sample(10, positions)]);
        assert_eq!(index.ordered_pairs, 8 * 7);
        assert_eq!(index.fine_pairs, 8 * 7);
        assert_eq!(index.coarse_pairs, 8 * 7);
        // 64x64 cells: 32x32 fine quadrats, 4x4 coarse quadrats.
        assert_eq!(index.habitable_fine_quadrats, 1_024);
        assert_eq!(index.habitable_coarse_quadrats, 16);
        assert_eq!(index.aggregation_milli, 16_000);
        assert_eq!(index.fine_milli, 1_024_000);
        assert_eq!(index.encounter_milli, 64_000);
    }

    #[test]
    fn holding_each_other_at_arms_length_inside_a_region_scores_below_one() {
        // Organisms confined to one coarse quadrat, one per fine quadrat
        // inside it: every pair shares a coarse quadrat and none shares a
        // fine one. That is spacing *below* random given the region, so
        // the encounter ratio must sit below 1000 -- here at its floor.
        let terrain = flat_terrain(64);
        let spread: Vec<(i32, i32)> = (0..8).map(|k| at_cell((k % 4) * 2, (k / 4) * 2)).collect();
        let spread_index = index_of(&terrain, &[sample(10, spread)]);
        assert_eq!(spread_index.fine_pairs, 0);
        assert_eq!(spread_index.encounter_milli, 0);

        // The same coarse confinement with organisms doubled up in fine
        // quadrats: fine-scale co-occurrence rises, and the coarse-scale
        // aggregation is unchanged because the region is the same.
        let paired: Vec<(i32, i32)> = (0..8)
            .map(|k| at_cell((k / 2 % 4) * 2, (k / 8) * 2))
            .collect();
        let paired_index = index_of(&terrain, &[sample(10, paired)]);
        assert!(paired_index.encounter_milli > spread_index.encounter_milli);
        assert_eq!(
            paired_index.aggregation_milli,
            spread_index.aggregation_milli
        );
    }

    /// Place `count` organisms drawn from a fixed clustered intensity: a
    /// fraction land uniformly inside one 16x16-cell region, the rest
    /// uniformly over the whole map. The *process* is fixed; only how many
    /// draws come from it changes.
    fn draw_from_fixed_process(
        seed: u64,
        sample_index: u64,
        count: usize,
        clustered_permille: u64,
    ) -> Vec<(i32, i32)> {
        (0..count)
            .map(|draw| {
                let pick = sim_core::named_random(
                    seed,
                    sample_index,
                    sim_core::RngSystem::Analysis,
                    draw as u64,
                    0,
                );
                let where_to = sim_core::named_random(
                    seed,
                    sample_index,
                    sim_core::RngSystem::Analysis,
                    draw as u64,
                    1,
                ) % 1_000;
                if where_to < clustered_permille {
                    at_cell((pick % 16) as u32, ((pick >> 8) % 16) as u32)
                } else {
                    at_cell((pick % 64) as u32, ((pick >> 8) % 64) as u32)
                }
            })
            .collect()
    }

    #[test]
    fn the_index_does_not_move_with_population_alone() {
        // The confound this whole construction exists to defeat. The Phase
        // 7 conditions differ threefold in median population, so an index
        // whose expectation tracked N would separate them whether or not
        // their spatial structure differed at all.
        //
        // Two worlds draw from the *same* clustered process, one at four
        // times the population of the other. Both indices must agree to
        // within a few percent.
        let terrain = flat_terrain(64);
        let samples_small: Vec<SpatialSample> = (0..400)
            .map(|t| sample(t + 1, draw_from_fixed_process(11, t, 40, 700)))
            .collect();
        let samples_large: Vec<SpatialSample> = (0..400)
            .map(|t| sample(t + 1, draw_from_fixed_process(11, t, 160, 700)))
            .collect();

        let small = index_of(&terrain, &samples_small);
        let large = index_of(&terrain, &samples_large);
        assert_eq!(large.observations, small.observations * 4);
        // The process really is clustered, or agreeing on 1000 would prove
        // nothing.
        assert!(small.aggregation_milli > 2_000, "{small:?}");

        for (name, a, b) in [
            (
                "aggregation",
                small.aggregation_milli,
                large.aggregation_milli,
            ),
            ("encounter", small.encounter_milli, large.encounter_milli),
        ] {
            let drift = (a - b).abs() * 1_000 / a.max(1);
            assert!(
                drift < 50,
                "{name} moved {drift} permille with population alone: {a} vs {b}"
            );
        }
    }

    #[test]
    fn random_placement_scores_one_at_both_scales() {
        // The calibration the whole index rests on: organisms scattered at
        // random over the habitable area must score 1000 (== random), and
        // the encounter ratio must also be 1000 because fine-scale spacing
        // is random given coarse-scale position.
        let terrain = flat_terrain(64);
        let samples: Vec<SpatialSample> = (0..400)
            .map(|t| sample(t + 1, draw_from_fixed_process(23, t, 80, 0)))
            .collect();
        let index = index_of(&terrain, &samples);
        for (name, value) in [
            ("aggregation", index.aggregation_milli),
            ("fine", index.fine_milli),
            ("encounter", index.encounter_milli),
        ] {
            assert!(
                (value - 1_000).abs() < 60,
                "{name} index is {value}, expected about 1000 under random placement"
            );
        }
    }

    #[test]
    fn samples_below_two_organisms_are_pooled_not_filtered() {
        let terrain = flat_terrain(64);
        let with_singletons = [
            sample(10, vec![at_cell(1, 1), at_cell(1, 1)]),
            sample(20, vec![at_cell(5, 5)]),
            sample(30, Vec::new()),
        ];
        let index = index_of(&terrain, &with_singletons);
        // All three samples are counted as used; only the first could
        // contribute pairs.
        assert_eq!(index.samples_used, 3);
        assert_eq!(index.observations, 3);
        assert_eq!(index.ordered_pairs, 2);
    }

    #[test]
    fn burn_in_drops_the_opening_transient_and_nothing_later() {
        let terrain = flat_terrain(64);
        let samples = [
            sample(100, vec![at_cell(1, 1), at_cell(1, 1)]),
            sample(200, vec![at_cell(1, 1), at_cell(1, 1)]),
            sample(300, vec![at_cell(1, 1), at_cell(1, 1)]),
        ];
        let all = world_index(&terrain, &samples, 0, 4, 2, 16).expect("index");
        let cut = world_index(&terrain, &samples, 200, 4, 2, 16).expect("index");
        assert_eq!(all.samples_used, 3);
        assert_eq!(cut.samples_used, 1);
    }

    #[test]
    fn refusals_are_typed_rather_than_defaulted() {
        let terrain = flat_terrain(64);
        assert_eq!(
            world_index(&terrain, &[], 0, 4, 2, 16),
            Err(IndexRefusal::NoSamples)
        );
        assert_eq!(
            world_index(&terrain, &[sample(10, vec![at_cell(1, 1)])], 0, 4, 2, 16),
            Err(IndexRefusal::NoPairsObserved)
        );
        // 25 is not a multiple of 2, so the coarse grid is not a union of
        // whole fine quadrats and the ratio loses its conditional reading.
        assert_eq!(
            world_index(&terrain, &[sample(10, Vec::new())], 0, 4, 2, 25),
            Err(IndexRefusal::ScalesDoNotNest {
                fine: 2,
                coarse: 25
            })
        );
    }

    #[test]
    fn water_quadrats_are_excluded_from_the_normalizer() {
        // Half the map flooded must halve `Q` at both scales, because the
        // null is "at random over the land available" and not "at random
        // over the bounding rectangle".
        let mut terrain = flat_terrain(64);
        for cell_y in 0..64 {
            for cell_x in 32..64 {
                let index = terrain.cell_index(cell_x, cell_y);
                terrain.land[index] = false;
            }
        }
        let fine = QuadratGrid::new(&terrain, 2);
        let coarse = QuadratGrid::new(&terrain, 16);
        assert_eq!(fine.habitable_quadrats, 32 * 16);
        assert_eq!(coarse.habitable_quadrats, 4 * 2);
    }

    #[test]
    fn a_generated_world_has_a_plausible_habitable_quadrat_count() {
        // Guards the cell-to-quadrat mapping against an off-by-one that a
        // hand-built flat map cannot expose.
        let config = SimConfig::phase7_default(1);
        let terrain = sim_core::generate_terrain(&config).expect("terrain");
        let coarse = QuadratGrid::new(&terrain, COARSE_SCALE_CELLS);
        let fine = QuadratGrid::new(&terrain, FINE_SCALE_CELLS);
        assert_eq!(coarse.quadrats_x, 16);
        assert_eq!(fine.quadrats_x, 128);
        assert!(coarse.habitable_quadrats > 0 && coarse.habitable_quadrats <= 256);
        assert!(fine.habitable_quadrats > 0 && fine.habitable_quadrats <= 16_384);
        // A coarse quadrat holds 64 fine ones, so the land-containing fine
        // count can never exceed 64x the coarse count.
        assert!(fine.habitable_quadrats <= coarse.habitable_quadrats * 64);
    }
}
