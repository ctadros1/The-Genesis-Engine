//! Terrain modification state (Phase 12, `lifesim-worldmod-v1`).
//!
//! **This is the state that breaks the invariant ALIF format 1 was built
//! around.** Terrain used to be a pure function of `(seed, config)`,
//! regenerated on load and verified against `terrain_checksum`, which is why
//! a snapshot could never be silently reinterpreted against a different
//! world. Once organisms can edit terrain that stops being true, so the
//! invariant is split rather than abandoned: the *baseline* is still
//! regenerated and still checked byte for byte by the existing fail-closed
//! check, and this file is the verified delta laid over it
//! (`specifications/mutable-world-state.md`).
//!
//! # Why an `Option<TerrainModState>` on `World` and not fields on `Terrain`
//!
//! The same reason `LearnState` is an option rather than two more vectors on
//! `Schema2State` (see `learnstate.rs`): C12.8 requires **four** existing
//! fixtures - Phase 1 `0x1e3158a26afd3b39`, Phase 2 `0xff9dfcff5dffbf42`,
//! Phase 9 `0x5f0c4e95e4f5170f`, Phase 11 `0x53b354bd94e82bcf` - to reproduce
//! exactly with the section disabled. A field hung off `Terrain` exists in
//! every world that has terrain, which is all of them, so it would hash and
//! cost in worlds pinned before this phase existed. An `Option` gated on
//! `worldmod.enabled` is absent in those worlds: nothing is appended to the
//! checksum, and the composed accessors on `World` return the raw terrain
//! value through a `None` arm that is the pre-Phase-12 code path unchanged.
//!
//! # The three layers, and which of them has a producer today
//!
//! | id | layer | producer today | consumer today |
//! |---|---|---|---|
//! | 0 | traversability override | none | `World::effective_traversable` |
//! | 1 | food capacity override (Q16 scale) | the relocating resource patch | `World::effective_capacity_milli` |
//! | 2 | material yield | **none** | **none** |
//!
//! All three are reserved, composed, and persisted now so that the save
//! format does not change again when the artifact half lands. **Layer 2 is
//! deliberately inert until then**: `strike` is what depletes material yield
//! and `strike` does not exist yet, so there is nothing to write it and
//! nothing to read it. It is carried rather than added later because adding
//! a layer means a format version, and a format version means a migration
//! (`specifications/world-save-format.md`); carrying an inert layer id costs
//! one match arm and zero bytes in a world that never writes it.
//!
//! Layer 0 has a consumer but no producer for the same reason: blocking
//! objects and digging are artifact-half actions. Its **safety policy** is
//! resolved here rather than discovered later, and is enforced by
//! `World::apply_terrain_modification`, and stated on
//! `ModOutcome::RefusedOccupied`.
//!
//! # The capacity override is a scale, not a delta
//!
//! Q16 multiplicative, matching the climate precedent
//! (`ClimateState::capacity_milli` scales by `biome_capacity_q16`). A delta
//! composes badly with the `[0, ...]` clamp every capacity consumer already
//! assumes - a negative delta on a low-capacity cell has to be clamped
//! somewhere, and every consumer would have to agree where. A scale of
//! `Q16_ONE` is exactly the identity on a non-negative capacity, which is
//! what makes a zero-magnitude control arm possible: the control runs the
//! identical relocation schedule at scale 1.0, writes the identical override
//! set, and changes no cell's capacity by one milli.

use crate::checksum::Fnv1a64;
use crate::worldgen::Terrain;

/// Recorded in the config hash and in the state checksum tag.
pub const WORLDMOD_POLICY_VERSION: &str = "lifesim-worldmod-v1";

/// Blocks or permits movement through a cell. Value 0 blocks, nonzero
/// permits; absent means "whatever the baseline says", which is
/// `Terrain::land`.
pub const LAYER_TRAVERSABLE: u8 = 0;
/// Q16 multiplier on a cell's carrying capacity. Absent means `Q16_ONE`.
pub const LAYER_CAPACITY_SCALE: u8 = 1;
/// Remaining extractable material, milli-units. **No producer or consumer
/// until the artifact half lands**; `strike` is what depletes it.
pub const LAYER_MATERIAL_YIELD: u8 = 2;
/// One past the highest reserved layer id. A value outside `0..LAYER_COUNT`
/// is refused at the write and at `check_invariants`, so a future layer is a
/// deliberate addition rather than an accident that silently persists.
pub const LAYER_COUNT: u8 = 3;

/// Outcome of one terrain-modification write. Every refusal is a distinct
/// value because C12.7 requires caps to "reject deterministically, count, and
/// event" - a single boolean is how a cap becomes invisible in a report
/// (D-074).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModOutcome {
    /// A new override was inserted.
    Inserted,
    /// An existing override at this key was overwritten.
    Replaced,
    /// An existing override was removed; the cell returns to baseline.
    Cleared,
    /// The write asked for exactly what was already there, or asked to clear
    /// a key that was already absent. Not an error, and counted separately
    /// so "the schedule ran and changed nothing" is visible.
    NoChange,
    /// The layer's override cap would have been exceeded.
    RefusedCap,
    /// The write would have made a cell non-traversable with an organism
    /// standing on it.
    ///
    /// **The safety property the specification asks this phase to resolve
    /// explicitly rather than discover.** A cell can be made non-traversable
    /// while an organism stands on it, and `check_invariants` refuses an
    /// organism on a non-traversable cell. Three resolutions were available
    /// and only one keeps the invariant as strong as it is today:
    ///
    /// 1. *Evict* the organism to a neighbouring cell. Rejected: it invents a
    ///    teleport with no energy cost, and a chain of evictions has an
    ///    ordering that would have to be specified and tested.
    /// 2. *Weaken the invariant* to "traversability gates entry, not
    ///    residency". Rejected: the invariant would then no longer catch an
    ///    organism that walked into water, which is what it exists for, and
    ///    the permit direction of this layer (a water cell made traversable)
    ///    means the check cannot simply keep reading `Terrain::land` either.
    /// 3. **Refuse the write.** Adopted. Making an occupied cell
    ///    non-traversable is refused and counted; the modification simply does
    ///    not happen. An organism can therefore never be standing somewhere it
    ///    could not be, and `check_invariants` composes the two layers and
    ///    stays exact.
    ///
    /// The rule is symmetric, and the asymmetric version is the bug:
    /// *removing* a permit override from a water cell an organism is standing
    /// on strands it exactly as blocking a land cell would, so a clear is
    /// refused on the same terms as a set.
    ///
    /// The cost of the choice is that a blocking action can fail for a reason
    /// the actor did not choose, which is why it is a distinct counted
    /// outcome rather than a silent no-op. It is not *evented* yet, and that
    /// is a stage-2 item rather than an omission: `EventKind` is matched
    /// exhaustively by `sim-persist`'s event-log codec, so a new variant needs
    /// a tag, an encoder, a decoder, and an `EVENT_SCHEMA_VERSION` bump in a
    /// crate this stage does not own.
    RefusedOccupied,
    /// Layer id outside `0..LAYER_COUNT`, cell index outside the map, or a
    /// value outside the layer's domain.
    RefusedInvalid,
}

impl ModOutcome {
    /// Whether the write was refused. The three refusal reasons stay distinct
    /// in the counters (D-074); this is for a caller that only needs to know
    /// its modification did not happen.
    pub fn is_refused(self) -> bool {
        matches!(
            self,
            Self::RefusedCap | Self::RefusedOccupied | Self::RefusedInvalid
        )
    }
}

/// Counters for the modification path. On this struct rather than on
/// `world::Counters` because `Counters` is hashed field by field into every
/// world's state checksum, so a field added there would move four fixtures;
/// here it is hashed under the section tag and only when the section exists,
/// exactly as `PlasticityCounters` is.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TerrainModCounters {
    pub writes_inserted: u64,
    pub writes_replaced: u64,
    pub writes_cleared: u64,
    pub writes_no_change: u64,
    pub refused_cap: u64,
    pub refused_occupied: u64,
    pub refused_invalid: u64,
    /// Relocations of the resource patch that actually ran.
    pub relocations: u64,
    /// Cells whose standing biomass had to be trimmed because a write
    /// lowered their capacity below it. The count that goes with
    /// `capacity_loss_milli`: a large loss spread over few cells and a small
    /// loss spread over many are different worlds.
    pub cells_trimmed: u64,
}

impl TerrainModCounters {
    /// Every counter, split into the disposition half and the refusal half.
    ///
    /// **Destructured with no `..`** (D-077), for the reason
    /// `PlasticityCounters::partitioned` is: a counter added to the struct
    /// fails to compile here until it is put in one bucket or the other,
    /// which is what stops it from being reported while sitting outside the
    /// checksum.
    ///
    /// The concatenation order is the declaration order and is
    /// **permanent**: it is the byte order `hash_into` feeds the hasher.
    /// Append, never reorder.
    fn partitioned(&self) -> ([u64; 6], [u64; 3]) {
        let Self {
            writes_inserted,
            writes_replaced,
            writes_cleared,
            writes_no_change,
            refused_cap,
            refused_occupied,
            refused_invalid,
            relocations,
            cells_trimmed,
        } = *self;
        (
            [
                writes_inserted,
                writes_replaced,
                writes_cleared,
                writes_no_change,
                relocations,
                cells_trimmed,
            ],
            [refused_cap, refused_occupied, refused_invalid],
        )
    }

    /// Writes that were refused for any reason. C12.7's "a run silently
    /// pressed against a cap must be visible in its report".
    pub fn refusals(&self) -> u64 {
        let (_, refusals) = self.partitioned();
        refusals.iter().sum()
    }

    pub fn hash_into(&self, hasher: &mut Fnv1a64) {
        let (disposition, refusals) = self.partitioned();
        for value in disposition.iter().chain(refusals.iter()) {
            hasher.update_u64(*value);
        }
    }
}

/// The sparse modification set: a sorted, unique list of
/// `(layer_id, cell_index, value)` overrides plus the biomass sink that
/// lowering a capacity opens.
///
/// Storage is struct-of-arrays in ascending `(layer_id, cell_index)` order.
/// **Sortedness and uniqueness are invariants, not conveniences.** They are
/// what make application "a simple ordered scan, so it is trivially
/// deterministic", what make the encoding of a logical modification set
/// unique (a golden-snapshot test over a set with two legal encodings tests
/// nothing), and what let a lookup be a binary search instead of a scan.
/// `check_invariants` verifies both rather than trusting them, because a
/// restore decodes an untrusted payload straight into these arrays.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TerrainModState {
    /// Ascending by `(layers[i], cells[i])`, strictly - equal keys are a
    /// violation, not a duplicate to be tolerated.
    pub layers: Vec<u8>,
    pub cells: Vec<u32>,
    pub values: Vec<i64>,
    /// Biomass removed because a modification lowered a cell's carrying
    /// capacity below its standing biomass.
    ///
    /// A genuine sink, ledgered rather than discarded, exactly as
    /// `ClimateWorld::capacity_loss_milli` is and for the same reason:
    /// `check_invariants` refuses biomass above capacity with no tolerance,
    /// and the biomass conservation identity would break by exactly this
    /// amount if the trimmed biomass simply vanished. Rule 7 fixed point:
    /// this accumulates for the life of the world.
    pub capacity_loss_milli: i128,
    pub counters: TerrainModCounters,
}

/// Sort key for one override. `layer` in the high bits so the ordering is
/// lexicographic on `(layer_id, cell_index)`, which is the order the
/// specification requires modifications to be applied in.
fn key(layer: u8, cell: u32) -> u64 {
    (u64::from(layer) << 32) | u64::from(cell)
}

impl TerrainModState {
    pub fn len(&self) -> usize {
        self.layers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.layers.is_empty()
    }

    /// Overrides on one layer. Used for the per-layer cap and by the
    /// full-map composition walk.
    pub fn layer_range(&self, layer: u8) -> std::ops::Range<usize> {
        let start = self.lower_bound(key(layer, 0));
        let end = self.lower_bound(key(layer, u32::MAX).wrapping_add(1));
        start..end
    }

    pub fn layer_len(&self, layer: u8) -> usize {
        self.layer_range(layer).len()
    }

    /// First index whose key is `>= target`.
    fn lower_bound(&self, target: u64) -> usize {
        let (mut low, mut high) = (0_usize, self.layers.len());
        while low < high {
            let mid = low + (high - low) / 2;
            if key(self.layers[mid], self.cells[mid]) < target {
                low = mid + 1;
            } else {
                high = mid;
            }
        }
        low
    }

    /// `Ok(index)` when the key is present, `Err(insertion point)` when not.
    fn search(&self, layer: u8, cell: u32) -> Result<usize, usize> {
        let target = key(layer, cell);
        let index = self.lower_bound(target);
        if index < self.layers.len() && key(self.layers[index], self.cells[index]) == target {
            Ok(index)
        } else {
            Err(index)
        }
    }

    /// The override at `(layer, cell)`, or `None` for "baseline".
    pub fn get(&self, layer: u8, cell: u32) -> Option<i64> {
        self.search(layer, cell)
            .ok()
            .map(|index| self.values[index])
    }

    /// Insert or overwrite one override, keeping the arrays sorted.
    ///
    /// `cap` is the per-layer override cap; an insert that would exceed it is
    /// refused and counted. Note the cap binds **inserts only**: overwriting
    /// an existing key adds no storage, so refusing it would make a world
    /// that reached the cap unable to *lower* an override, which is the wrong
    /// direction for a cap that exists to bound memory and snapshot size.
    pub fn set(&mut self, layer: u8, cell: u32, value: i64, cap: u32) -> ModOutcome {
        match self.search(layer, cell) {
            Ok(index) => {
                if self.values[index] == value {
                    self.counters.writes_no_change += 1;
                    return ModOutcome::NoChange;
                }
                self.values[index] = value;
                self.counters.writes_replaced += 1;
                ModOutcome::Replaced
            }
            Err(index) => {
                if self.layer_len(layer) >= cap as usize {
                    self.counters.refused_cap += 1;
                    return ModOutcome::RefusedCap;
                }
                self.layers.insert(index, layer);
                self.cells.insert(index, cell);
                self.values.insert(index, value);
                self.counters.writes_inserted += 1;
                ModOutcome::Inserted
            }
        }
    }

    /// Remove one override; the cell returns to its baseline value.
    pub fn clear(&mut self, layer: u8, cell: u32) -> ModOutcome {
        match self.search(layer, cell) {
            Ok(index) => {
                self.layers.remove(index);
                self.cells.remove(index);
                self.values.remove(index);
                self.counters.writes_cleared += 1;
                ModOutcome::Cleared
            }
            Err(_) => {
                self.counters.writes_no_change += 1;
                ModOutcome::NoChange
            }
        }
    }

    /// `Some(index)` naming the first entry that breaks strict ascending
    /// `(layer_id, cell_index)` order, or that duplicates its predecessor.
    ///
    /// Checked rather than assumed: `set` and `clear` cannot produce an
    /// unsorted array, so the path this defends is the one that does not go
    /// through them - a restore decoding an untrusted payload.
    pub fn order_violation(&self) -> Option<usize> {
        if self.cells.len() != self.layers.len() || self.values.len() != self.layers.len() {
            return Some(0);
        }
        (1..self.layers.len()).find(|&index| {
            key(self.layers[index - 1], self.cells[index - 1])
                >= key(self.layers[index], self.cells[index])
        })
    }

    /// `Some(index)` naming the first entry whose layer id, cell index, or
    /// value is outside its documented domain.
    ///
    /// The value domains are per layer and are what stop a decoded payload
    /// from putting, say, a negative capacity scale into
    /// `effective_capacity_milli` and a negative capacity into the biomass
    /// bounds check.
    pub fn bounds_violation(&self, cell_count: usize) -> Option<usize> {
        (0..self.layers.len()).find(|&index| {
            let layer = self.layers[index];
            let value = self.values[index];
            layer >= LAYER_COUNT
                || self.cells[index] as usize >= cell_count
                || !value_in_domain(layer, value)
        })
    }

    /// Hash every field under the section tag.
    ///
    /// **Destructured with no `..` (D-077).** A field added to this struct
    /// fails to compile here until it is either hashed or given an explicit
    /// reason not to be. The byte order below is permanent: it is the
    /// definition of a mutable world's checksum. Append, never reorder.
    pub fn hash_into(&self, hasher: &mut Fnv1a64) {
        let Self {
            layers,
            cells,
            values,
            capacity_loss_milli,
            counters,
        } = self;
        hasher.update(b"lifesim-terrainmod-state-v1");
        // The entry count is hashed too, so a truncated set is a checksum
        // difference rather than a prefix that could coincidentally re-align
        // with a shorter one.
        hasher.update_u64(layers.len() as u64);
        for index in 0..layers.len() {
            hasher.update_u32(u32::from(layers[index]));
            hasher.update_u32(cells[index]);
            hasher.update_i64(values[index]);
        }
        counters.hash_into(hasher);
        hasher.update_i128(*capacity_loss_milli);
    }

    /// The composed terrain checksum: FNV-1a over the terrain an observer
    /// would actually see, baseline plus every override applied.
    ///
    /// # This is a full recompute, and the specification asked for
    /// incremental
    ///
    /// `specifications/mutable-world-state.md` says the composed checksum "is
    /// recomputed incrementally as modifications are applied, not recomputed
    /// over the full field every tick", with a periodic full recompute as a
    /// cross-check. **That is not implementable exactly with FNV-1a and it is
    /// not implemented here.** FNV-1a is a multiply-and-xor chain over a byte
    /// stream: changing a byte in the middle changes every subsequent state,
    /// so there is no update that folds one cell's new value into a finished
    /// hash. The honest options were a different, genuinely incremental
    /// digest - which would be a second checksum algorithm in a codebase
    /// whose determinism story rests on having exactly one - or a full
    /// recompute. The full recompute is what is here, and the incremental
    /// clause of the specification is a **documentation change owed**: it
    /// describes an algorithm the chosen primitive cannot provide.
    ///
    /// The cost is paid where it is needed and nowhere else. **No tick calls
    /// this.** It is called at save time (stage 2 records it in
    /// `SECTION_WORLD_META`), on restore to verify the recorded value, and by
    /// tests. `World::state_checksum` deliberately does *not* include it: it
    /// is a pure function of `terrain_checksum` and the modification set, and
    /// both of those are already hashed, so it would add no discriminating
    /// power at a cost of one pass over every cell - the same argument that
    /// keeps developed bodies out of the checksum in Phase 10.
    ///
    /// # Why an empty set reproduces `terrain_checksum` exactly
    ///
    /// The tag, field order, and byte layout below are copied from
    /// `worldgen::generate` deliberately. With no overrides, every composed
    /// value equals its baseline and this returns exactly
    /// `terrain.terrain_checksum`. That equality is load-bearing: it is what
    /// makes the registered format 3 to format 4 migration expressible as
    /// "composed checksum := baseline checksum", which is the only honest
    /// thing a migration can write for a file that predates the layer.
    ///
    /// Layer 2 is appended after the field, not merged into it, because it
    /// has no baseline field to override. Appended only when nonempty, so
    /// the empty-set equality above survives the layer's existence.
    pub fn composed_checksum(&self, terrain: &Terrain) -> u64 {
        let mut hasher = Fnv1a64::new();
        hasher.update(b"lifesim-terrain-v1");
        hasher.update_u32(terrain.cells_x);
        hasher.update_u32(terrain.cells_y);
        let traversable = self.layer_range(LAYER_TRAVERSABLE);
        let capacity = self.layer_range(LAYER_CAPACITY_SCALE);
        // Two cursors rather than two binary searches per cell: the arrays
        // are sorted by cell within a layer and this walks cells ascending,
        // so the whole pass is O(cells + overrides).
        let mut traversable_cursor = traversable.start;
        let mut capacity_cursor = capacity.start;
        for index in 0..terrain.cell_count() {
            let cell = index as u32;
            while traversable_cursor < traversable.end && self.cells[traversable_cursor] < cell {
                traversable_cursor += 1;
            }
            let land =
                if traversable_cursor < traversable.end && self.cells[traversable_cursor] == cell {
                    self.values[traversable_cursor] != 0
                } else {
                    terrain.land[index]
                };
            while capacity_cursor < capacity.end && self.cells[capacity_cursor] < cell {
                capacity_cursor += 1;
            }
            let capacity_milli =
                if capacity_cursor < capacity.end && self.cells[capacity_cursor] == cell {
                    scale_capacity(terrain.capacity_milli[index], self.values[capacity_cursor])
                } else {
                    terrain.capacity_milli[index]
                };
            hasher.update(&[u8::from(land)]);
            hasher.update_u32(terrain.elevation_q16[index]);
            hasher.update_i64(capacity_milli);
        }
        let material = self.layer_range(LAYER_MATERIAL_YIELD);
        if !material.is_empty() {
            hasher.update(b"lifesim-material-yield-v1");
            for index in material {
                hasher.update_u32(self.cells[index]);
                hasher.update_i64(self.values[index]);
            }
        }
        hasher.finish()
    }

    // --- Observation only (ADR-0016) -------------------------------------

    /// Overrides per layer, for the report. Nothing in the tick reads it.
    pub fn layer_counts(&self) -> [u64; LAYER_COUNT as usize] {
        let mut counts = [0_u64; LAYER_COUNT as usize];
        for &layer in &self.layers {
            if layer < LAYER_COUNT {
                counts[layer as usize] += 1;
            }
        }
        counts
    }
}

/// Whether a stored value is inside its layer's domain.
pub fn value_in_domain(layer: u8, value: i64) -> bool {
    match layer {
        LAYER_TRAVERSABLE => (0..=1).contains(&value),
        // A capacity scale is a non-negative Q16 multiplier. The ceiling is
        // what stops `base * scale` from overflowing i64 for any capacity a
        // config can express, and it is generous: 256x.
        LAYER_CAPACITY_SCALE => (0..=(256 * 65_536)).contains(&value),
        LAYER_MATERIAL_YIELD => value >= 0,
        _ => false,
    }
}

/// Compose one cell's capacity with a Q16 scale.
///
/// The single definition of the composition, called both by the random-access
/// accessor on `World` and by the full-map walk in `composed_checksum`. Two
/// copies of this formula is exactly how the two would come to disagree, and
/// a composed checksum that disagrees with the live world is a restore that
/// fails for no reason a reader could find.
///
/// `base` is non-negative for every terrain the generator produces (water is
/// 0, land is `cell_capacity_milli * suitability >> 16`) and the scale is
/// non-negative by `value_in_domain`, so the product is non-negative and the
/// shift is an exact truncating divide. Widened to i128 first: the domain cap
/// allows 256x, and a large `cell_capacity_milli` at 256x would overflow i64
/// before the shift brought it back.
pub fn scale_capacity(base: i64, scale_q16: i64) -> i64 {
    let scaled = (i128::from(base.max(0)) * i128::from(scale_q16.max(0))) >> 16;
    scaled.min(i128::from(i64::MAX)) as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SimConfig;

    const CAP: u32 = 64;

    fn hash(state: &TerrainModState) -> u64 {
        let mut hasher = Fnv1a64::new();
        state.hash_into(&mut hasher);
        hasher.finish()
    }

    /// Deliberately inserted out of order and across all three layers, so a
    /// sort that only worked for a single layer, or only for ascending
    /// input, is visible.
    fn populated() -> TerrainModState {
        let mut state = TerrainModState::default();
        for (layer, cell, value) in [
            (LAYER_CAPACITY_SCALE, 900_u32, 131_072_i64),
            (LAYER_TRAVERSABLE, 12, 0),
            (LAYER_MATERIAL_YIELD, 4, 7_000),
            (LAYER_CAPACITY_SCALE, 5, 32_768),
            (LAYER_TRAVERSABLE, 3, 1),
        ] {
            assert_eq!(state.set(layer, cell, value, CAP), ModOutcome::Inserted);
        }
        state
    }

    #[test]
    fn inserts_land_sorted_and_unique_whatever_order_they_arrive_in() {
        let state = populated();
        assert_eq!(state.len(), 5);
        assert_eq!(state.order_violation(), None);
        assert_eq!(state.layers, vec![0, 0, 1, 1, 2]);
        assert_eq!(state.cells, vec![3, 12, 5, 900, 4]);
        // Uniqueness: the same key twice is a replace, never a second entry.
        let mut again = state.clone();
        assert_eq!(
            again.set(LAYER_CAPACITY_SCALE, 5, 40_000, CAP),
            ModOutcome::Replaced
        );
        assert_eq!(again.len(), 5);
        assert_eq!(again.get(LAYER_CAPACITY_SCALE, 5), Some(40_000));
        assert_eq!(again.order_violation(), None);
        // ...and writing the value it already holds changes nothing.
        assert_eq!(
            again.set(LAYER_CAPACITY_SCALE, 5, 40_000, CAP),
            ModOutcome::NoChange
        );
    }

    #[test]
    fn lookup_finds_only_its_own_layer_and_cell() {
        let state = populated();
        assert_eq!(state.get(LAYER_TRAVERSABLE, 3), Some(1));
        assert_eq!(state.get(LAYER_CAPACITY_SCALE, 900), Some(131_072));
        assert_eq!(state.get(LAYER_MATERIAL_YIELD, 4), Some(7_000));
        // Cell 4 exists on layer 2 and cell 3 on layer 0; a lookup that
        // ignored the layer would find them here.
        assert_eq!(state.get(LAYER_TRAVERSABLE, 4), None);
        assert_eq!(state.get(LAYER_CAPACITY_SCALE, 3), None);
        assert_eq!(state.get(LAYER_MATERIAL_YIELD, 900), None);
        assert_eq!(state.layer_len(LAYER_TRAVERSABLE), 2);
        assert_eq!(state.layer_len(LAYER_CAPACITY_SCALE), 2);
        assert_eq!(state.layer_len(LAYER_MATERIAL_YIELD), 1);
        assert_eq!(state.layer_counts(), [2, 2, 1]);
    }

    #[test]
    fn clearing_removes_exactly_one_entry_and_keeps_the_order() {
        let mut state = populated();
        assert_eq!(state.clear(LAYER_CAPACITY_SCALE, 5), ModOutcome::Cleared);
        assert_eq!(state.len(), 4);
        assert_eq!(state.get(LAYER_CAPACITY_SCALE, 5), None);
        assert_eq!(state.get(LAYER_CAPACITY_SCALE, 900), Some(131_072));
        assert_eq!(state.order_violation(), None);
        assert_eq!(state.clear(LAYER_CAPACITY_SCALE, 5), ModOutcome::NoChange);
        assert_eq!(state.len(), 4);
    }

    #[test]
    fn the_per_layer_cap_refuses_inserts_and_never_replacements() {
        let mut state = TerrainModState::default();
        for cell in 0..3_u32 {
            assert_eq!(
                state.set(LAYER_CAPACITY_SCALE, cell, 65_536, 3),
                ModOutcome::Inserted
            );
        }
        assert_eq!(
            state.set(LAYER_CAPACITY_SCALE, 99, 65_536, 3),
            ModOutcome::RefusedCap
        );
        assert_eq!(state.counters.refused_cap, 1);
        assert_eq!(state.counters.refusals(), 1);
        // The cap is per layer, so another layer is unaffected...
        assert_eq!(state.set(LAYER_TRAVERSABLE, 99, 0, 3), ModOutcome::Inserted);
        // ...and a replacement at the cap still works, because it adds no
        // storage. A cap that blocked it would leave a full layer unable to
        // lower its own overrides.
        assert_eq!(
            state.set(LAYER_CAPACITY_SCALE, 1, 32_768, 3),
            ModOutcome::Replaced
        );
        assert_eq!(state.get(LAYER_CAPACITY_SCALE, 1), Some(32_768));
    }

    #[test]
    fn the_order_check_finds_a_payload_a_restore_could_hand_it() {
        let mut state = populated();
        assert_eq!(state.order_violation(), None);
        // Two entries swapped: what a corrupted or hostile section looks
        // like, and what `set` can never produce.
        state.cells.swap(0, 1);
        assert_eq!(state.order_violation(), Some(1));
        state.cells.swap(0, 1);
        // A duplicated key is equally illegal: application order would still
        // be defined, but the encoding of a logical set would not be unique.
        state.layers[1] = 0;
        state.cells[1] = 3;
        assert_eq!(state.order_violation(), Some(1));
        state.layers[1] = 0;
        state.cells[1] = 12;
        assert_eq!(state.order_violation(), None);
        // Ragged arrays are a violation at index 0 rather than a panic.
        state.values.pop();
        assert_eq!(state.order_violation(), Some(0));
    }

    #[test]
    fn the_bounds_check_finds_every_out_of_domain_value() {
        let mut state = populated();
        assert_eq!(state.bounds_violation(1_024), None);
        // Cell index past the end of the map.
        assert_eq!(state.bounds_violation(64), Some(3));
        // Traversability is a flag; 2 is not one.
        state.values[0] = 2;
        assert_eq!(state.bounds_violation(1_024), Some(0));
        state.values[0] = 1;
        // A negative capacity scale would produce a negative capacity and
        // make the biomass bounds check unsatisfiable.
        state.values[2] = -1;
        assert_eq!(state.bounds_violation(1_024), Some(2));
        state.values[2] = 32_768;
        // An unknown layer id is refused rather than ignored.
        state.layers[4] = LAYER_COUNT;
        assert_eq!(state.bounds_violation(1_024), Some(4));
    }

    #[test]
    fn every_field_reaches_the_checksum() {
        let base = populated();
        let reference = hash(&base);
        let mutators: [fn(&mut TerrainModState); 5] = [
            |state| state.layers[4] = LAYER_CAPACITY_SCALE,
            |state| state.cells[0] += 1,
            |state| state.values[0] += 1,
            |state| state.capacity_loss_milli += 1,
            |state| state.counters.cells_trimmed += 1,
        ];
        for (index, mutate) in mutators.into_iter().enumerate() {
            let mut moved = base.clone();
            mutate(&mut moved);
            assert_ne!(hash(&moved), reference, "field {index} missed the hash");
        }
        // Every counter, not just the one above: a counter that is reported
        // but not hashed is how two restored checksums came to differ in
        // Phase 9.
        let counter_mutators: [fn(&mut TerrainModCounters); 9] = [
            |counters| counters.writes_inserted += 1,
            |counters| counters.writes_replaced += 1,
            |counters| counters.writes_cleared += 1,
            |counters| counters.writes_no_change += 1,
            |counters| counters.refused_cap += 1,
            |counters| counters.refused_occupied += 1,
            |counters| counters.refused_invalid += 1,
            |counters| counters.relocations += 1,
            |counters| counters.cells_trimmed += 1,
        ];
        for (index, mutate) in counter_mutators.into_iter().enumerate() {
            let mut moved = base.clone();
            mutate(&mut moved.counters);
            assert_ne!(hash(&moved), reference, "counter {index} missed the hash");
        }
        // Truncation must move the hash even though the remaining prefix is
        // byte-identical, which is what the leading length is for.
        let mut shorter = base.clone();
        shorter.layers.pop();
        shorter.cells.pop();
        shorter.values.pop();
        assert_ne!(hash(&shorter), reference);
    }

    #[test]
    fn an_empty_modification_set_composes_to_the_baseline_checksum() {
        // The identity the format 3 to format 4 migration is built on: a
        // file that predates the layer has no modification set, so the only
        // honest composed checksum it can be given is its baseline one.
        let terrain = crate::worldgen::generate(&SimConfig::phase1_default(7)).unwrap();
        let empty = TerrainModState::default();
        assert_eq!(empty.composed_checksum(&terrain), terrain.terrain_checksum);
    }

    #[test]
    fn a_single_override_moves_the_composed_checksum_and_a_no_op_does_not() {
        let terrain = crate::worldgen::generate(&SimConfig::phase1_default(7)).unwrap();
        let baseline = terrain.terrain_checksum;
        let habitable = (0..terrain.cell_count())
            .find(|&cell| terrain.capacity_milli[cell] > 0)
            .expect("a habitable cell");

        // Scale 1.0 is exactly the identity on a non-negative capacity, so a
        // control arm's override set changes no cell's value. This is what
        // makes "identical schedule, zero magnitude" a real control rather
        // than a differently-shaped run.
        let mut control = TerrainModState::default();
        control.set(LAYER_CAPACITY_SCALE, habitable as u32, 65_536, CAP);
        assert_eq!(control.composed_checksum(&terrain), baseline);

        let mut treatment = TerrainModState::default();
        treatment.set(LAYER_CAPACITY_SCALE, habitable as u32, 131_072, CAP);
        assert_ne!(treatment.composed_checksum(&terrain), baseline);

        // Traversability composes on the same terms and in the other
        // direction: permitting a water cell must move the checksum too.
        let water = (0..terrain.cell_count())
            .find(|&cell| !terrain.land[cell])
            .expect("a water cell");
        let mut permitted = TerrainModState::default();
        permitted.set(LAYER_TRAVERSABLE, water as u32, 1, CAP);
        assert_ne!(permitted.composed_checksum(&terrain), baseline);
        // ...and an override that asserts what the baseline already says is
        // a no-op on the composed field.
        let mut redundant = TerrainModState::default();
        redundant.set(LAYER_TRAVERSABLE, water as u32, 0, CAP);
        assert_eq!(redundant.composed_checksum(&terrain), baseline);
    }

    #[test]
    fn material_yield_reaches_the_composed_checksum_without_a_terrain_field() {
        // Layer 2 has no baseline field to override, so it is appended
        // rather than composed. It still has to be covered: an inert layer
        // that silently dropped out of the checksum would be discovered by
        // the artifact half, not by this phase.
        let terrain = crate::worldgen::generate(&SimConfig::phase1_default(7)).unwrap();
        let mut state = TerrainModState::default();
        state.set(LAYER_MATERIAL_YIELD, 11, 5_000, CAP);
        let first = state.composed_checksum(&terrain);
        assert_ne!(first, terrain.terrain_checksum);
        state.set(LAYER_MATERIAL_YIELD, 11, 5_001, CAP);
        assert_ne!(state.composed_checksum(&terrain), first);
    }

    #[test]
    fn the_composed_walk_agrees_with_per_cell_composition() {
        // `composed_checksum` walks the arrays with cursors for speed; this
        // is the same field built by the naive per-cell lookup. A cursor
        // that skipped an entry, or advanced past one it should have
        // matched, is invisible in the hash alone.
        let terrain = crate::worldgen::generate(&SimConfig::phase1_default(7)).unwrap();
        let mut state = TerrainModState::default();
        // Deliberately sparse and non-contiguous, spanning the first and last
        // cell so a cursor that starts or ends wrong is caught, and
        // **including cells whose capacity is positive**.
        //
        // The first cut of this list was `[0, 1, 2, 1000, 4097, 4098, 65535]`,
        // chosen by eye to be spread out. Every one of them is on or beside
        // the generator's forced water rim, so every one has capacity zero,
        // so `scale_capacity(0, anything)` is zero and **the capacity half of
        // this test asserted nothing**. A cursor mutation that dropped every
        // capacity override passed it. That is what the guard below exists
        // for, and it is why the habitable cells are found rather than
        // guessed.
        let habitable: Vec<u32> = (0..terrain.cell_count())
            .filter(|cell| terrain.capacity_milli[*cell] > 0)
            .map(|cell| cell as u32)
            .collect();
        assert!(habitable.len() > 8, "the test world has no interior");
        let step = habitable.len() / 6;
        let mut cells: Vec<u32> = (0..6).map(|index| habitable[index * step]).collect();
        cells.push(0);
        cells.push(terrain.cell_count() as u32 - 1);
        cells.sort_unstable();
        cells.dedup();
        assert!(
            cells
                .iter()
                .filter(|cell| terrain.capacity_milli[**cell as usize] > 0)
                .count()
                >= 6,
            "the capacity half of this test would be vacuous"
        );
        for (index, cell) in cells.iter().enumerate() {
            state.set(LAYER_CAPACITY_SCALE, *cell, 20_000 + index as i64, 4_096);
            state.set(LAYER_TRAVERSABLE, *cell, (index % 2) as i64, 4_096);
        }
        let mut expected = Fnv1a64::new();
        expected.update(b"lifesim-terrain-v1");
        expected.update_u32(terrain.cells_x);
        expected.update_u32(terrain.cells_y);
        for index in 0..terrain.cell_count() {
            let cell = index as u32;
            let land = match state.get(LAYER_TRAVERSABLE, cell) {
                Some(value) => value != 0,
                None => terrain.land[index],
            };
            let capacity = match state.get(LAYER_CAPACITY_SCALE, cell) {
                Some(scale) => scale_capacity(terrain.capacity_milli[index], scale),
                None => terrain.capacity_milli[index],
            };
            expected.update(&[u8::from(land)]);
            expected.update_u32(terrain.elevation_q16[index]);
            expected.update_i64(capacity);
        }
        assert_eq!(state.composed_checksum(&terrain), expected.finish());
    }

    #[test]
    fn scaling_is_exact_at_one_and_saturating_at_the_ceiling() {
        assert_eq!(scale_capacity(30_000, 65_536), 30_000);
        assert_eq!(scale_capacity(30_000, 0), 0);
        assert_eq!(scale_capacity(30_000, 32_768), 15_000);
        assert_eq!(scale_capacity(0, 256 * 65_536), 0);
        // The widening is what stops the domain ceiling from overflowing.
        assert_eq!(scale_capacity(i64::MAX, 256 * 65_536), i64::MAX);
        // Identity holds for every capacity the generator can produce.
        for base in [1_i64, 7, 29_999, 30_000, i64::MAX] {
            assert_eq!(scale_capacity(base, 65_536), base);
        }
    }
}
