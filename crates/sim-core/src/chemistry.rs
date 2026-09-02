//! The chemistry half of the field regime (Phase 15, ADR-0031,
//! `lifesim-chemistry-v1`).
//!
//! Four abstract substrates per raster cell, everything i64 milli fixed
//! point (Rule 7 - the field integrates over horizons far longer than an
//! organism lifetime). Conservation is BY CONSTRUCTION, never by
//! tolerance: diffusion moves exactly what it subtracts, reactions convert
//! 1:1 in mass, production and deposits are counted in the chemistry
//! ledger, and the C15.1 identity
//!
//!   produced + deposited - seeded_out == current total
//!
//! holds to the milli-unit at every step (the field starts empty, so
//! there is no initial term; `seeded_out` is what abiogenesis moves into
//! the microbial field, Phase 15 increment 2 - zero until then).
//!
//! Every pass reads a committed buffer and writes a scratch, so storage
//! order cannot influence results - C15.7's permutation clause is a
//! property of the structure, not a discipline.

use crate::checksum::Fnv1a64;
use crate::config::ChemistryConfig;

pub const CHEMISTRY_POLICY_VERSION: &str = "lifesim-chemistry-v1";
/// Bumped whenever a substrate is added or a reaction changes meaning;
/// enters the config hash.
pub const SUBSTRATE_REGISTRY_VERSION: u16 = 1;

/// Substrate ids are **permanent**, like RNG streams and module types.
pub const S_PRIMORDIAL: usize = 0;
pub const S_MONOMER: usize = 1;
pub const S_POLYMER: usize = 2;
pub const S_WASTE: usize = 3;
pub const SUBSTRATE_COUNT: usize = 4;

/// Per-world chemistry state. `None` exactly when the section is
/// disabled, so a disabled world takes the existing code paths and every
/// prior fixture reproduces.
#[derive(Clone, Debug, PartialEq)]
pub struct ChemistryState {
    /// Concentrations, milli, flattened `cell * SUBSTRATE_COUNT + s`.
    pub concentrations: Vec<i64>,
    /// Scratch buffer for the double-buffered passes. Same layout; never
    /// saved or hashed (a work area, not state).
    scratch: Vec<i64>,
    /// Production weight per cell, Q16, summing to exactly
    /// `cells * Q16_ONE` by construction (the scaffold redistributes,
    /// never adds). Derived from config and dimensions; never saved.
    production_weight_q16: Vec<u64>,

    /// The chemistry ledger, i128 like the world's.
    pub produced_milli: i128,
    pub deposited_milli: i128,
    pub seeded_out_milli: i128,
    /// Abiogenesis draws that fired (increment 2 consumes this; counted
    /// from the start so the counter's meaning never moves).
    pub abiogenesis_fired_total: u64,
}

impl ChemistryState {
    pub fn new(cells_x: u32, cells_y: u32, config: &ChemistryConfig) -> Self {
        let cells = cells_x as usize * cells_y as usize;
        Self {
            concentrations: vec![0; cells * SUBSTRATE_COUNT],
            scratch: vec![0; cells * SUBSTRATE_COUNT],
            production_weight_q16: production_weights(cells_x, cells_y, config),
            produced_milli: 0,
            deposited_milli: 0,
            seeded_out_milli: 0,
            abiogenesis_fired_total: 0,
        }
    }

    /// Rebuild the derived members after a restore installed the saved
    /// arrays (the scratch and weights are caches, like bodies).
    pub fn rebuild_derived(&mut self, cells_x: u32, cells_y: u32, config: &ChemistryConfig) {
        self.scratch = vec![0; self.concentrations.len()];
        self.production_weight_q16 = production_weights(cells_x, cells_y, config);
    }

    pub fn total_milli(&self) -> i128 {
        self.concentrations
            .iter()
            .map(|&value| i128::from(value))
            .sum()
    }

    /// The C15.1 identity's defect: exactly zero in a correct world.
    pub fn conservation_defect_milli(&self) -> i128 {
        self.produced_milli + self.deposited_milli - self.seeded_out_milli - self.total_milli()
    }

    /// One field step: diffusion, then the abiotic reactions, then
    /// production. (The microbial passes join between reactions and
    /// production in increment 2.)
    pub fn step(&mut self, cells_x: u32, cells_y: u32, config: &ChemistryConfig) {
        self.diffuse(cells_x, cells_y, config);
        self.react(config);
        self.produce(config);
    }

    /// Von Neumann diffusion, conserving by construction: each cell sends
    /// `concentration * rate >> 16` to each in-map neighbour, and what the
    /// sources lose is exactly what the destinations gain - the truncation
    /// remainder stays home. Reads committed, writes scratch, swaps.
    fn diffuse(&mut self, cells_x: u32, cells_y: u32, config: &ChemistryConfig) {
        let width = cells_x as usize;
        let height = cells_y as usize;
        let rate = i64::from(config.diffusion_q16);
        self.scratch.copy_from_slice(&self.concentrations);
        for cell in 0..width * height {
            let x = cell % width;
            let y = cell / width;
            for substrate in 0..SUBSTRATE_COUNT {
                let concentration = self.concentrations[cell * SUBSTRATE_COUNT + substrate];
                if concentration <= 0 {
                    continue;
                }
                let per_neighbour = (concentration * rate) >> 16;
                if per_neighbour == 0 {
                    continue;
                }
                let neighbours = [
                    (x > 0).then(|| cell - 1),
                    (x + 1 < width).then(|| cell + 1),
                    (y > 0).then(|| cell - width),
                    (y + 1 < height).then(|| cell + width),
                ];
                for neighbour in neighbours.into_iter().flatten() {
                    self.scratch[cell * SUBSTRATE_COUNT + substrate] -= per_neighbour;
                    self.scratch[neighbour * SUBSTRATE_COUNT + substrate] += per_neighbour;
                }
            }
        }
        std::mem::swap(&mut self.concentrations, &mut self.scratch);
    }

    /// The two abiotic reactions, 1:1 in mass: what leaves one substrate
    /// arrives in the other, in the same cell, in the same step.
    fn react(&mut self, config: &ChemistryConfig) {
        let monomer_rate = i64::from(config.reaction_monomer_q16);
        let recycle_rate = i64::from(config.reaction_recycle_q16);
        for cell in 0..self.concentrations.len() / SUBSTRATE_COUNT {
            let base = cell * SUBSTRATE_COUNT;
            let converted = (self.concentrations[base + S_PRIMORDIAL] * monomer_rate) >> 16;
            self.concentrations[base + S_PRIMORDIAL] -= converted;
            self.concentrations[base + S_MONOMER] += converted;
            let recycled = (self.concentrations[base + S_WASTE] * recycle_rate) >> 16;
            self.concentrations[base + S_WASTE] -= recycled;
            self.concentrations[base + S_PRIMORDIAL] += recycled;
        }
    }

    /// Abiotic S_PRIMORDIAL input, weighted by the scaffold map, every
    /// deposited milli counted as production.
    ///
    /// Distributed by the telescoping cumulative method: cell i receives
    /// `(cumulative_i >> 16) - (cumulative_{i-1} >> 16)`, so the total per
    /// step is EXACTLY `production * cells` whatever the weights - per-cell
    /// truncation cannot lose a milli, it only shifts which cell of a run
    /// gets the odd one (deterministically, in ascending cell index). The
    /// scaffold test caught the naive per-cell truncation losing mass on
    /// fractional weights; this is the by-construction fix.
    fn produce(&mut self, config: &ChemistryConfig) {
        if config.production_milli_per_step <= 0 {
            return;
        }
        let mut cumulative: i64 = 0;
        let mut emitted: i64 = 0;
        for cell in 0..self.production_weight_q16.len() {
            cumulative += config.production_milli_per_step * self.production_weight_q16[cell] as i64;
            let target = cumulative >> 16;
            let amount = target - emitted;
            emitted = target;
            if amount > 0 {
                self.concentrations[cell * SUBSTRATE_COUNT + S_PRIMORDIAL] += amount;
                self.produced_milli += i128::from(amount);
            }
        }
    }

    /// An organism-side deposit (excretion, remains), counted.
    pub fn deposit(&mut self, cell: usize, substrate: usize, amount_milli: i64) {
        if amount_milli <= 0 {
            return;
        }
        self.concentrations[cell * SUBSTRATE_COUNT + substrate] += amount_milli;
        self.deposited_milli += i128::from(amount_milli);
    }

    /// Only logical state enters the checksum; the scratch and weight
    /// caches do not, on the terms bodies do not.
    pub fn hash_into(&self, hasher: &mut Fnv1a64) {
        hasher.update(b"lifesim-chemistry-state-v1");
        for &value in &self.concentrations {
            hasher.update_i64(value);
        }
        hasher.update_i128(self.produced_milli);
        hasher.update_i128(self.deposited_milli);
        hasher.update_i128(self.seeded_out_milli);
        hasher.update_u64(self.abiogenesis_fired_total);
    }
}

/// The scaffold's production weight map: Q16 weights whose SUM is exactly
/// `cells * Q16_ONE`, so the map redistributes and never adds. Radius 0
/// (or contrast one) is uniform - the N arm. Otherwise patch centres sit
/// on a regular grid with spacing four radii; cells within Chebyshev
/// `radius` of a grid line intersection weigh `contrast`, outside cells
/// share the exact remainder, and the integer remainder of that division
/// lands on the lowest eligible cell index - counted placement, following
/// the lowest-id remainder convention.
fn production_weights(cells_x: u32, cells_y: u32, config: &ChemistryConfig) -> Vec<u64> {
    let width = cells_x as usize;
    let height = cells_y as usize;
    let cells = width * height;
    let one = u64::from(crate::config::Q16_ONE);
    if config.scaffold_patch_radius_cells == 0
        || config.scaffold_patch_contrast_q16 == crate::config::Q16_ONE
    {
        return vec![one; cells];
    }
    let radius = config.scaffold_patch_radius_cells as usize;
    let spacing = radius * 4;
    let mut inside = vec![false; cells];
    let mut inside_count = 0_usize;
    for y in 0..height {
        for x in 0..width {
            let near_centre = (x % spacing).min(spacing - x % spacing) <= radius
                && (y % spacing).min(spacing - y % spacing) <= radius;
            if near_centre {
                inside[y * width + x] = true;
                inside_count += 1;
            }
        }
    }
    if inside_count == 0 || inside_count == cells {
        return vec![one; cells];
    }
    let contrast = u64::from(config.scaffold_patch_contrast_q16);
    let total: u64 = one * cells as u64;
    let inside_total = contrast * inside_count as u64;
    // If the requested contrast would consume more than the whole budget,
    // clamp by giving outside cells zero and inside cells the exact share.
    let (inside_weight, outside_weight, remainder) = if inside_total >= total {
        (total / inside_count as u64, 0, total % inside_count as u64)
    } else {
        let outside_count = (cells - inside_count) as u64;
        let outside_total = total - inside_total;
        (
            contrast,
            outside_total / outside_count,
            outside_total % outside_count,
        )
    };
    let mut weights = vec![0_u64; cells];
    let mut remainder_target: Option<usize> = None;
    for cell in 0..cells {
        weights[cell] = if inside[cell] {
            inside_weight
        } else {
            outside_weight
        };
        if remainder_target.is_none() {
            let takes_remainder = if inside_total >= total {
                inside[cell]
            } else {
                !inside[cell]
            };
            if takes_remainder {
                remainder_target = Some(cell);
            }
        }
    }
    if let Some(cell) = remainder_target {
        weights[cell] += remainder;
    }
    weights
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> ChemistryConfig {
        let mut config = ChemistryConfig::chemistry_default();
        config.enabled = true;
        config
    }

    #[test]
    fn diffusion_conserves_exactly_under_an_adversarial_gradient() {
        let mut state = ChemistryState::new(8, 8, &config());
        // Everything in one corner, nothing anywhere else - the steepest
        // gradient the map can hold - plus awkward odd values that force
        // truncation remainders everywhere.
        state.concentrations[S_PRIMORDIAL] = 999_999_999;
        state.concentrations[S_WASTE] = 1_234_567;
        state.produced_milli = 999_999_999 + 1_234_567;
        for _ in 0..500 {
            state.diffuse(8, 8, &config());
            assert_eq!(state.conservation_defect_milli(), 0);
        }
    }

    #[test]
    fn reactions_convert_mass_one_to_one() {
        let mut state = ChemistryState::new(2, 2, &config());
        state.concentrations[S_PRIMORDIAL] = 100_000;
        state.concentrations[S_WASTE] = 50_001;
        state.produced_milli = 150_001;
        for _ in 0..200 {
            state.react(&config());
            assert_eq!(state.conservation_defect_milli(), 0);
        }
        // The cycle really ran: monomer accumulated and waste recycled.
        assert!(state.concentrations[S_MONOMER] > 0);
        assert!(state.concentrations[S_WASTE] < 50_001);
    }

    #[test]
    fn production_is_counted_and_the_identity_holds_through_full_steps() {
        let mut state = ChemistryState::new(16, 16, &config());
        for _ in 0..300 {
            state.step(16, 16, &config());
            assert_eq!(state.conservation_defect_milli(), 0);
        }
        assert!(state.produced_milli > 0);
        assert_eq!(state.total_milli(), state.produced_milli);
    }

    #[test]
    fn the_scaffold_map_redistributes_and_never_adds() {
        let mut scaffolded = config();
        scaffolded.scaffold_patch_radius_cells = 2;
        scaffolded.scaffold_patch_contrast_q16 = 4 * crate::config::Q16_ONE;
        let weights = production_weights(32, 32, &scaffolded);
        let total: u64 = weights.iter().sum();
        assert_eq!(
            total,
            u64::from(crate::config::Q16_ONE) * 32 * 32,
            "the scaffold must hold the production total exactly constant"
        );
        let max = *weights.iter().max().unwrap();
        let min = *weights.iter().min().unwrap();
        assert!(max > min, "a contrast above one must actually concentrate");
    }

    #[test]
    fn deposits_are_counted_into_the_identity() {
        let mut state = ChemistryState::new(4, 4, &config());
        state.deposit(5, S_WASTE, 777);
        state.deposit(3, S_PRIMORDIAL, 223);
        assert_eq!(state.conservation_defect_milli(), 0);
        assert_eq!(state.deposited_milli, 1_000);
    }
}
