//! The artifact half's tick work (Phase 12, ADR-0028), as a child module of
//! `world` so it can reach the world's arrays without widening their
//! visibility.
//!
//! Four entry points, one per phase that gains work when the section is
//! enabled, every one of them a no-op that touches nothing when it is not:
//!
//! - `SpatialIndex`: [`World::rebuild_object_index`], the per-cell index of
//!   free objects, rebuilt from the table.
//! - `Sense`: [`World::sense_objects`], the six perception cues per organism.
//! - `Apply`, after `contest_phase`: [`World::artifact_phase`], the five
//!   actions and consumption, in a fixed order over organisms in ascending
//!   ID, with every contest resolved from a snapshot taken at the start of
//!   the pass.
//! - `Lifecycle`: [`World::drop_held_on_death`] before compaction,
//!   [`World::spawn_carcass_object`] from `spawn_carcass`, and
//!   [`World::decay_objects`] after; `Environment` gains
//!   [`World::regenerate_yield`].
//!
//! # Determinism, in one paragraph
//!
//! Every loop below runs over organisms in index order, which is ID order.
//! Every candidate set is materialized, sorted by `(distance_squared, id)`
//! and truncated before it is read (Rule 5). Every contest between two
//! organisms for one object is resolved by `(priority desc, distance_squared
//! asc, organism id asc)` over claims collected before any is applied, so the
//! outcome cannot depend on visit order (Rule 3). Every strike on one target
//! in a tick is summed before the fracture test (review 13.7). Every draw is
//! `named_random` on `Artifact` or `MaterialYield` keyed on the target or the
//! pair key, never on a counter that depends on how many draws preceded it.
//! New IDs are allocated at the end of the pass in the order the creations
//! were queued, which is itself a function of ID order.

use super::{EventKind, World};
use crate::artifact::{
    CAUSE_CARCASS, CAUSE_COMBINED, CAUSE_EXTRACTED, CAUSE_FRACTURED, DestroyCause,
    INTEGRITY_WHOLE_Q16, ObjectAction, ObjectRecord, ObjectState, RefuseReason,
};
use crate::config::Q16_ONE;
use crate::controller::{cos_bam_q15, sin_bam_q15};
use crate::material::{
    MATERIAL_CARCASS, MATERIAL_FIBER, MATERIAL_STONE, MATERIAL_WOOD, material, max_hardness_q16,
};
use crate::rng::{RngSystem, named_random};
use crate::terrainmod::LAYER_MATERIAL_YIELD;

/// A creation queued during the pass and allocated an ID at its end.
struct PendingObject {
    record: ObjectRecord,
    /// The counter this creation increments once it exists.
    cause: u8,
}

/// One claim on a free object this tick, by `pick_up` or `combine`.
#[derive(Clone, Copy)]
struct Claim {
    target: usize,
    organism: usize,
    priority: i32,
    distance_squared: i64,
    action: ObjectAction,
}

impl World {
    /// The material a land cell yields, by relative elevation above the
    /// coastline, or `None` for water. A physical rule with two thresholds
    /// and no table: rock where it is high, wood in the middle, fiber low.
    pub(super) fn cell_material(&self, cell: usize) -> Option<u16> {
        if !self.terrain.land[cell] {
            return None;
        }
        let threshold = u64::from(self.config.land_threshold_q16);
        let elevation = u64::from(self.terrain.elevation_q16[cell]);
        let span = (65_536_u64).saturating_sub(threshold).max(1);
        let relative = (elevation.saturating_sub(threshold) << 16) / span;
        let artifact = &self.config.artifact;
        Some(if relative >= u64::from(artifact.stone_relative_q16) {
            MATERIAL_STONE
        } else if relative >= u64::from(artifact.wood_relative_q16) {
            MATERIAL_WOOD
        } else {
            MATERIAL_FIBER
        })
    }

    /// Remaining extractable volume in a cell: the override if one is stored,
    /// else the baseline. Zero on water.
    pub(super) fn cell_yield_milli(&self, cell: usize) -> i64 {
        if self.cell_material(cell).is_none() {
            return 0;
        }
        self.worldmod
            .as_ref()
            .and_then(|state| state.get(LAYER_MATERIAL_YIELD, cell as u32))
            .unwrap_or(self.config.artifact.terrain_yield_milli)
    }

    /// Carry capacity for one organism: the body-scale ratio times the
    /// configured capacity, the shape `health_max_milli` uses (D-085).
    pub(super) fn carry_capacity_milli(&self, index: usize) -> i64 {
        let scale = self
            .phase2
            .as_ref()
            .map_or(1_000, |p2| p2.phenotypes[index].body_scale_milli);
        (self.config.artifact.carry_capacity_milli * scale / 1_000).max(1)
    }

    /// Bare strike force for one organism, before held objects.
    fn bare_strike_force_q16(&self, index: usize) -> i64 {
        let scale = self
            .phase2
            .as_ref()
            .map_or(1_000, |p2| p2.phenotypes[index].body_scale_milli);
        i64::from(self.config.artifact.strike_force_q16) * scale / 1_000
    }

    /// The `SpatialIndex` phase's contribution.
    pub(super) fn rebuild_object_index(&mut self) {
        let Some(mut objects) = self.objects.take() else {
            return;
        };
        let cell_count = self.terrain.cell_count();
        objects.rebuild_cell_index(cell_count, |x, y| self.cell_of(x, y));
        objects.intents.clear();
        objects.intents.resize(self.ids.len(), Default::default());
        self.objects = Some(objects);
    }

    /// Whether movement into `cell` is refused by an object standing in it.
    pub(super) fn cell_blocked_by_object(&self, cell: usize) -> bool {
        self.objects.as_ref().is_some_and(|objects| {
            objects.cell_is_blocked(cell, self.config.artifact.blocking_mass_milli)
        })
    }

    /// Mass held by one organism, or zero without the section.
    pub(super) fn held_mass_milli(&self, index: usize) -> i64 {
        self.objects
            .as_ref()
            .map_or(0, |objects| objects.held_mass_milli(index))
    }

    /// The `Environment` phase's contribution: yield regenerates toward its
    /// baseline on a cadence, and an override that reaches baseline is
    /// cleared. Only stored overrides are visited, so a world nobody has dug
    /// in pays nothing.
    pub(super) fn regenerate_yield(&mut self, next_tick: u64) {
        if self.objects.is_none() {
            return;
        }
        let artifact = self.config.artifact;
        if artifact.yield_regen_milli <= 0
            || !next_tick.is_multiple_of(artifact.yield_regen_interval_ticks)
        {
            return;
        }
        let Some(worldmod) = self.worldmod.as_ref() else {
            return;
        };
        let range = worldmod.layer_range(LAYER_MATERIAL_YIELD);
        let cells: Vec<u32> = worldmod.cells[range.clone()].to_vec();
        let values: Vec<i64> = worldmod.values[range].to_vec();
        let cap = self.config.worldmod.max_material_overrides;
        let baseline = artifact.terrain_yield_milli;
        if let Some(worldmod) = self.worldmod.as_mut() {
            for (cell, value) in cells.into_iter().zip(values) {
                let regrown = (value + artifact.yield_regen_milli).min(baseline);
                if regrown >= baseline {
                    worldmod.clear(LAYER_MATERIAL_YIELD, cell);
                } else {
                    worldmod.set(LAYER_MATERIAL_YIELD, cell, regrown, cap);
                }
            }
        }
    }

    /// The `Sense` phase's contribution: six cues per organism, read from the
    /// per-cell index built this tick, so perception sees the tick-start
    /// object table and nothing this tick's actions do.
    pub(super) fn sense_objects(&mut self) {
        let Some(mut objects) = self.objects.take() else {
            return;
        };
        let population = self.ids.len();
        objects.perception.clear();
        objects.perception.resize(population, [0.0; 6]);
        let Some(p2) = self.phase2.as_ref() else {
            self.objects = Some(objects);
            return;
        };
        let range_fp =
            i64::from(self.config.artifact.perception_range_m) * i64::from(crate::FP_PER_METER);
        let max_hardness = max_hardness_q16() as f32;
        for index in 0..population {
            let capacity = self.carry_capacity_milli(index);
            let carried = objects.held_mass_milli(index);
            let mut cues = [0.0_f32; 6];
            cues[5] = (carried as f32 / capacity as f32).clamp(0.0, 1.0);
            if let Some((distance_squared, target)) =
                self.nearest_free_object(&objects, index, range_fp, usize::MAX)
            {
                let range = range_fp as f32;
                let distance = (distance_squared as f32).sqrt();
                cues[0] = 1.0;
                cues[1] = (1.0 - distance / range).clamp(0.0, 1.0);
                let heading = p2.heading_bam[index];
                let heading_x = i64::from(cos_bam_q15(heading));
                let heading_y = i64::from(sin_bam_q15(heading));
                let delta_x = i64::from(objects.table.x_fp[target]) - i64::from(self.x_fp[index]);
                let delta_y = i64::from(objects.table.y_fp[target]) - i64::from(self.y_fp[index]);
                let cross = heading_x * delta_y - heading_y * delta_x;
                let norm = (delta_x.abs() + delta_y.abs()).max(1);
                cues[2] = (cross as f32 / (32768.0 * norm as f32)).clamp(-1.0, 1.0);
                cues[3] =
                    (objects.table.mass_milli[target] as f32 / capacity as f32).clamp(0.0, 1.0);
                cues[4] =
                    (objects.table.hardness_q16[target] as f32 / max_hardness).clamp(0.0, 1.0);
            }
            objects.perception[index] = cues;
        }
        self.objects = Some(objects);
    }

    /// Free objects within `range_fp` of organism `index`, as
    /// `(distance_squared, table index)`, sorted by `(distance_squared, id)`
    /// and truncated to `limit`. The materialized-sorted-truncated set of
    /// Rule 5; the cell index is scanned in a fixed rectangle and its scan
    /// order never reaches a decision.
    fn free_objects_within(
        &self,
        objects: &ObjectState,
        index: usize,
        range_fp: i64,
        limit: usize,
    ) -> Vec<(i64, usize)> {
        if !objects.cell_index_valid {
            return Vec::new();
        }
        let x = i64::from(self.x_fp[index]);
        let y = i64::from(self.y_fp[index]);
        let cell_fp = i64::from(self.config.cell_size_fp());
        let cells_x = i64::from(self.terrain.cells_x);
        let cells_y = i64::from(self.terrain.cells_y);
        let cell_x = x / cell_fp;
        let cell_y = y / cell_fp;
        let reach_cells = (range_fp + cell_fp - 1) / cell_fp;
        let mut found: Vec<(i64, u64, usize)> = Vec::new();
        for cy in (cell_y - reach_cells).max(0)..=(cell_y + reach_cells).min(cells_y - 1) {
            for cx in (cell_x - reach_cells).max(0)..=(cell_x + reach_cells).min(cells_x - 1) {
                let cell = (cy * cells_x + cx) as usize;
                let Some(bucket) = objects.cell_index.get(cell) else {
                    continue;
                };
                for &candidate in bucket {
                    let candidate = candidate as usize;
                    let dx = i64::from(objects.table.x_fp[candidate]) - x;
                    let dy = i64::from(objects.table.y_fp[candidate]) - y;
                    let distance_squared = dx * dx + dy * dy;
                    if distance_squared <= range_fp * range_fp {
                        found.push((distance_squared, objects.table.ids[candidate], candidate));
                    }
                }
            }
        }
        found.sort_unstable_by_key(|entry| (entry.0, entry.1));
        found.truncate(limit);
        found
            .into_iter()
            .map(|(distance_squared, _, candidate)| (distance_squared, candidate))
            .collect()
    }

    fn nearest_free_object(
        &self,
        objects: &ObjectState,
        index: usize,
        range_fp: i64,
        limit: usize,
    ) -> Option<(i64, usize)> {
        self.free_objects_within(objects, index, range_fp, limit.min(1).max(1))
            .into_iter()
            .next()
    }

    /// Charge one action's cost. Paid whether or not the action succeeds.
    fn charge(&mut self, index: usize, cost: i64) {
        let paid = cost.min(self.energy_milli[index]).max(0);
        self.energy_milli[index] -= paid;
        self.ledger.spent_milli += i128::from(paid);
    }

    fn refuse(
        &mut self,
        next_tick: u64,
        objects: &mut ObjectState,
        index: usize,
        action: ObjectAction,
        reason: RefuseReason,
    ) {
        objects.table.counters.refuse(reason);
        let id = self.ids[index];
        self.push_event(
            next_tick,
            EventKind::ObjectActionRefused {
                id,
                action: action.id(),
                reason: reason.id(),
            },
        );
    }

    /// The lowest-ID object organism `index` holds, as a table index.
    fn lowest_held(objects: &ObjectState, index: usize) -> Option<usize> {
        objects.held[index]
            .first()
            .and_then(|&id| objects.table.index_of(id))
    }

    /// Insert a table index into a cell's bucket, keeping the bucket sorted
    /// by table index (which is ID order within one pass).
    fn cell_index_insert(objects: &mut ObjectState, cell: usize, table_index: usize) {
        let bucket = &mut objects.cell_index[cell];
        let position = bucket
            .binary_search(&(table_index as u32))
            .unwrap_or_else(|position| position);
        bucket.insert(position, table_index as u32);
    }

    fn cell_index_remove(objects: &mut ObjectState, cell: usize, table_index: usize) {
        let bucket = &mut objects.cell_index[cell];
        if let Ok(position) = bucket.binary_search(&(table_index as u32)) {
            bucket.remove(position);
        }
    }

    /// Release the object at `table_index` from organism `index` at `(x, y)`.
    /// The caller has checked the cap. **Not inserted into the cell index**:
    /// an object released this tick is not a target for anything else this
    /// tick (review 13.7's one-tick latency, applied to releases as it is to
    /// fragments), so it becomes visible when the index is rebuilt next tick.
    /// The occupancy count for later releases into the same cell this tick
    /// comes from the pass's own `landed_in` tally.
    fn release_into_world(
        objects: &mut ObjectState,
        index: usize,
        table_index: usize,
        x_fp: i32,
        y_fp: i32,
    ) {
        let id = objects.table.ids[table_index];
        objects.table.holder_id[table_index] = 0;
        objects.table.x_fp[table_index] = x_fp;
        objects.table.y_fp[table_index] = y_fp;
        if let Ok(position) = objects.held[index].binary_search(&id) {
            objects.held[index].remove(position);
        }
    }

    /// Free objects a cell holds for the occupancy cap: what the index says
    /// plus what landed there earlier in this pass.
    fn occupancy(objects: &ObjectState, landed_in: &[(usize, usize)], cell: usize) -> usize {
        objects.free_in_cell(cell)
            + landed_in
                .iter()
                .filter(|(landed_cell, _)| *landed_cell == cell)
                .count()
    }

    /// The `Apply` phase's contribution.
    pub(super) fn artifact_phase(&mut self, next_tick: u64) {
        let Some(mut objects) = self.objects.take() else {
            return;
        };
        let artifact = self.config.artifact;
        let population = self.ids.len();
        let inert = artifact.inert;
        let reach_fp = i64::from(artifact.reach_m) * i64::from(crate::FP_PER_METER);
        let cell_fp = self.config.cell_size_fp();
        let max_candidates = artifact.max_candidates as usize;
        let mut pending: Vec<PendingObject> = Vec::new();
        let mut destroyed: Vec<bool> = vec![false; objects.table.len()];
        // Objects dropped or placed this tick, as (cell, table index): the
        // occupancy tally for later releases, and condition B's list.
        let mut landed: Vec<(usize, usize)> = Vec::new();

        // Condition C (`artifact.inert`, `lifesim-artifact-v2`): every
        // requested action is charged and counted as a success and the five
        // verbs confer nothing - no refusal, no object, no hold. That is
        // what "actions fire and pay but confer no effect" has to mean for
        // the control to measure the baseline *firing* rate rather than a
        // rate depressed by refusals that only exist because nothing can
        // ever be held. The verbs are ALL the inert arm skips: consumption
        // (section 5), exposure and carry accounting (6b) and the commit
        // pass (7) run below in both arms, and decay runs in its own phase,
        // because v1's early return here skipped carcass consumption too
        // and made the control differ from condition A in more than its
        // named variable (D-118: starvation +6%, damage deaths -42%).
        if inert {
            for index in 0..population {
                let intent = objects.intents[index];
                if intent.drop.is_some() {
                    self.charge(index, artifact.action_cost_milli);
                    objects.table.counters.dropped += 1;
                }
                if intent.place.is_some() {
                    self.charge(index, artifact.action_cost_milli);
                    objects.table.counters.placed += 1;
                }
                if intent.pick_up.is_some() {
                    self.charge(index, artifact.action_cost_milli);
                    objects.table.counters.picked_up += 1;
                }
                if intent.combine.is_some() {
                    self.charge(index, artifact.action_cost_milli);
                    objects.table.counters.combined += 1;
                }
                if intent.strike.is_some() {
                    self.charge(index, artifact.strike_cost_milli);
                    objects.table.counters.struck_terrain += 1;
                }
            }
        } else {
            // --- 1. Drops, ascending index. ---
            for index in 0..population {
                let Some(priority) = objects.intents[index].drop else {
                    continue;
                };
                let _ = priority;
                self.charge(index, artifact.action_cost_milli);
                let Some(table_index) = Self::lowest_held(&objects, index) else {
                    self.refuse(
                        next_tick,
                        &mut objects,
                        index,
                        ObjectAction::Drop,
                        RefuseReason::NothingHeld,
                    );
                    continue;
                };
                let cell = self.cell_of(self.x_fp[index], self.y_fp[index]);
                if Self::occupancy(&objects, &landed, cell)
                    >= artifact.max_objects_per_cell as usize
                {
                    self.refuse(
                        next_tick,
                        &mut objects,
                        index,
                        ObjectAction::Drop,
                        RefuseReason::OccupancyCap,
                    );
                    continue;
                }
                let (x, y) = (self.x_fp[index], self.y_fp[index]);
                Self::release_into_world(&mut objects, index, table_index, x, y);
                objects.table.counters.dropped += 1;
                landed.push((cell, table_index));
                let id = objects.table.ids[table_index];
                let holder = self.ids[index];
                self.push_event(
                    next_tick,
                    EventKind::ObjectReleased {
                        id,
                        holder,
                        placed: false,
                        cell: cell as u32,
                    },
                );
            }

            // --- 2. Places, ascending index. ---
            let extent_x = i64::from(self.config.world_extent_x_fp());
            let extent_y = i64::from(self.config.world_extent_y_fp());
            for index in 0..population {
                let Some(_priority) = objects.intents[index].place else {
                    continue;
                };
                self.charge(index, artifact.action_cost_milli);
                let Some(table_index) = Self::lowest_held(&objects, index) else {
                    self.refuse(
                        next_tick,
                        &mut objects,
                        index,
                        ObjectAction::Place,
                        RefuseReason::NothingHeld,
                    );
                    continue;
                };
                // The centre of the adjacent cell the organism faces.
                let heading = self.phase2.as_ref().map_or(0, |p2| p2.heading_bam[index]);
                let step = i64::from(cell_fp);
                let target_x =
                    i64::from(self.x_fp[index]) + step * i64::from(cos_bam_q15(heading)) / 32_768;
                let target_y =
                    i64::from(self.y_fp[index]) + step * i64::from(sin_bam_q15(heading)) / 32_768;
                if target_x < 0 || target_y < 0 || target_x >= extent_x || target_y >= extent_y {
                    self.refuse(
                        next_tick,
                        &mut objects,
                        index,
                        ObjectAction::Place,
                        RefuseReason::InvalidCell,
                    );
                    continue;
                }
                let (target_x, target_y) = (target_x as i32, target_y as i32);
                let cell = self.cell_of(target_x, target_y);
                if !self.effective_traversable(cell) {
                    self.refuse(
                        next_tick,
                        &mut objects,
                        index,
                        ObjectAction::Place,
                        RefuseReason::InvalidCell,
                    );
                    continue;
                }
                if Self::occupancy(&objects, &landed, cell)
                    >= artifact.max_objects_per_cell as usize
                {
                    self.refuse(
                        next_tick,
                        &mut objects,
                        index,
                        ObjectAction::Place,
                        RefuseReason::OccupancyCap,
                    );
                    continue;
                }
                // Snap to the cell centre so a placed object's position is a
                // function of the cell alone, never of a float heading.
                let cx = (i64::from(target_x) / step) * step + step / 2;
                let cy = (i64::from(target_y) / step) * step + step / 2;
                Self::release_into_world(&mut objects, index, table_index, cx as i32, cy as i32);
                objects.table.creator_id[table_index] = self.ids[index];
                objects.table.counters.placed += 1;
                landed.push((cell, table_index));
                let id = objects.table.ids[table_index];
                let holder = self.ids[index];
                self.push_event(
                    next_tick,
                    EventKind::ObjectReleased {
                        id,
                        holder,
                        placed: true,
                        cell: cell as u32,
                    },
                );
            }

            // --- 3. Claims: pick_up and combine, resolved jointly per target. ---
            let mut claims: Vec<Claim> = Vec::new();
            for index in 0..population {
                let intent = objects.intents[index];
                if let Some(priority) = intent.pick_up {
                    self.charge(index, artifact.action_cost_milli);
                    if objects.held[index].len() >= artifact.max_held_objects as usize {
                        self.refuse(
                            next_tick,
                            &mut objects,
                            index,
                            ObjectAction::PickUp,
                            RefuseReason::HeldCap,
                        );
                    } else {
                        let capacity =
                            self.carry_capacity_milli(index) - objects.held_mass_milli(index);
                        let candidates =
                            self.free_objects_within(&objects, index, reach_fp, max_candidates);
                        if candidates.is_empty() {
                            self.refuse(
                                next_tick,
                                &mut objects,
                                index,
                                ObjectAction::PickUp,
                                RefuseReason::NoTarget,
                            );
                        } else if let Some(&(distance_squared, target)) = candidates
                            .iter()
                            .find(|(_, candidate)| objects.table.mass_milli[*candidate] <= capacity)
                        {
                            claims.push(Claim {
                                target,
                                organism: index,
                                priority,
                                distance_squared,
                                action: ObjectAction::PickUp,
                            });
                        } else {
                            self.refuse(
                                next_tick,
                                &mut objects,
                                index,
                                ObjectAction::PickUp,
                                RefuseReason::CapacityExceeded,
                            );
                        }
                    }
                }
                if let Some(priority) = intent.combine {
                    self.charge(index, artifact.action_cost_milli);
                    if Self::lowest_held(&objects, index).is_none() {
                        self.refuse(
                            next_tick,
                            &mut objects,
                            index,
                            ObjectAction::Combine,
                            RefuseReason::NothingHeld,
                        );
                    } else {
                        let candidates =
                            self.free_objects_within(&objects, index, reach_fp, max_candidates);
                        match candidates.first() {
                            None => {
                                self.refuse(
                                    next_tick,
                                    &mut objects,
                                    index,
                                    ObjectAction::Combine,
                                    RefuseReason::NoTarget,
                                );
                            }
                            Some(&(distance_squared, target)) => claims.push(Claim {
                                target,
                                organism: index,
                                priority,
                                distance_squared,
                                action: ObjectAction::Combine,
                            }),
                        }
                    }
                }
            }
            // Resolution: sort by target, then by (priority desc, d2 asc, id asc);
            // the first claim on each target wins and every other is refused.
            claims.sort_by_key(|claim| {
                (
                    claim.target,
                    std::cmp::Reverse(claim.priority),
                    claim.distance_squared,
                    self.ids[claim.organism],
                    claim.action,
                )
            });
            let mut winners: Vec<Claim> = Vec::new();
            let mut last_target: Option<usize> = None;
            for claim in claims {
                if last_target == Some(claim.target) {
                    self.refuse(
                        next_tick,
                        &mut objects,
                        claim.organism,
                        claim.action,
                        RefuseReason::Contested,
                    );
                    continue;
                }
                last_target = Some(claim.target);
                winners.push(claim);
            }
            // Winners apply in ascending organism id: the sort above grouped by
            // target, so re-sort by organism for the application order.
            winners.sort_by_key(|claim| (self.ids[claim.organism], claim.action));
            for claim in winners {
                let index = claim.organism;
                let target = claim.target;
                match claim.action {
                    ObjectAction::PickUp => {
                        let cell =
                            self.cell_of(objects.table.x_fp[target], objects.table.y_fp[target]);
                        Self::cell_index_remove(&mut objects, cell, target);
                        let holder = self.ids[index];
                        objects.table.holder_id[target] = holder;
                        let id = objects.table.ids[target];
                        let position = objects.held[index].binary_search(&id).unwrap_or_else(|p| p);
                        objects.held[index].insert(position, id);
                        objects.table.counters.picked_up += 1;
                        self.push_event(
                            next_tick,
                            EventKind::ObjectPickedUp {
                                id,
                                holder,
                                cell: cell as u32,
                            },
                        );
                    }
                    ObjectAction::Combine => {
                        let Some(held) = Self::lowest_held(&objects, index) else {
                            continue;
                        };
                        let depth = 1 + objects.table.depth[held].max(objects.table.depth[target]);
                        if u32::from(depth) > artifact.max_composition_depth {
                            self.refuse(
                                next_tick,
                                &mut objects,
                                index,
                                ObjectAction::Combine,
                                RefuseReason::DepthCap,
                            );
                            continue;
                        }
                        if 2 > artifact.max_composition_breadth {
                            self.refuse(
                                next_tick,
                                &mut objects,
                                index,
                                ObjectAction::Combine,
                                RefuseReason::BreadthCap,
                            );
                            continue;
                        }
                        if objects.table.len() + pending.len() >= artifact.max_objects as usize {
                            self.refuse(
                                next_tick,
                                &mut objects,
                                index,
                                ObjectAction::Combine,
                                RefuseReason::ObjectCap,
                            );
                            continue;
                        }
                        let combiner = self.ids[index];
                        let target_id = objects.table.ids[target];
                        let draw = named_random(
                            self.config.world_seed,
                            next_tick,
                            RngSystem::Artifact,
                            crate::contest::pair_key(combiner, target_id),
                            0,
                        );
                        let scale = self
                            .phase2
                            .as_ref()
                            .map_or(1_000, |p2| p2.phenotypes[index].body_scale_milli);
                        let joint_q16 = ((i64::from((draw & 0xffff) as u32) * scale) / 1_000)
                            .min(i64::from(Q16_ONE));
                        if joint_q16 < i64::from(artifact.joint_floor_q16) {
                            self.refuse(
                                next_tick,
                                &mut objects,
                                index,
                                ObjectAction::Combine,
                                RefuseReason::JointFailed,
                            );
                            continue;
                        }
                        let held_id = objects.table.ids[held];
                        let (x, y) = (objects.table.x_fp[target], objects.table.y_fp[target]);
                        let cell = self.cell_of(x, y);
                        // Both constituents leave the world: the target from its
                        // cell, the held one from the holder.
                        Self::cell_index_remove(&mut objects, cell, target);
                        if let Ok(position) = objects.held[index].binary_search(&held_id) {
                            objects.held[index].remove(position);
                        }
                        let composite_id = self.next_entity_id;
                        self.next_entity_id += 1;
                        objects.table.objects_allocated_total += 1;
                        for constituent in [held, target] {
                            objects.table.holder_id[constituent] = 0;
                            objects.table.owner_id[constituent] = composite_id;
                        }
                        let mut composition = vec![held_id, target_id];
                        composition.sort_unstable();
                        let integrity = i64::from(
                            objects.table.integrity_q16[held]
                                .min(objects.table.integrity_q16[target]),
                        ) * joint_q16
                            >> 16;
                        let heavier =
                            if objects.table.mass_milli[held] >= objects.table.mass_milli[target] {
                                held
                            } else {
                                target
                            };
                        let record = ObjectRecord {
                            id: composite_id,
                            material_id: objects.table.material_id[heavier],
                            x_fp: x,
                            y_fp: y,
                            integrity_q16: integrity as i32,
                            mass_milli: objects.table.mass_milli[held]
                                + objects.table.mass_milli[target],
                            energy_milli: objects.table.energy_milli[held]
                                + objects.table.energy_milli[target],
                            hardness_q16: objects.table.hardness_q16[held]
                                .max(objects.table.hardness_q16[target]),
                            durability_q16: objects.table.durability_q16[held]
                                .min(objects.table.durability_q16[target]),
                            decay_q16: objects.table.decay_q16[held]
                                .max(objects.table.decay_q16[target]),
                            holder_id: 0,
                            owner_id: 0,
                            depth,
                            created_tick: next_tick,
                            creator_id: combiner,
                            cause: CAUSE_COMBINED,
                            parent_id: 0,
                            composition,
                        };
                        let table_index = objects.table.push(record);
                        Self::cell_index_insert(&mut objects, cell, table_index);
                        destroyed.push(false);
                        objects.table.counters.combined += 1;
                        objects.table.counters.created_combined += 1;
                        // Phase 13 cues: a successful combination is contact
                        // and an object-state change the combiner caused,
                        // sized by the composite's mass.
                        self.note_contact(index);
                        self.note_object_delta(index, objects.table.mass_milli[table_index]);
                        self.push_event(
                            next_tick,
                            EventKind::ObjectCombined {
                                composite: composite_id,
                                held: held_id,
                                target: target_id,
                                combiner,
                                depth,
                                joint_q16: joint_q16 as u32,
                            },
                        );
                    }
                    _ => {}
                }
            }

            // --- 4. Strikes: aggregated per target, then terrain. ---
            let mut force_on: Vec<(usize, i64, u64)> = Vec::new(); // (target, force, lowest striker id)
            let mut terrain_strikes: Vec<usize> = Vec::new();
            for index in 0..population {
                let Some(_priority) = objects.intents[index].strike else {
                    continue;
                };
                self.charge(index, artifact.strike_cost_milli);
                let mut force = self.bare_strike_force_q16(index);
                for &held_id in &objects.held[index] {
                    if let Some(held) = objects.table.index_of(held_id) {
                        force += i64::from(objects.table.hardness_q16[held])
                            * objects.table.mass_milli[held]
                            / artifact.strike_mass_reference_milli.max(1);
                    }
                }
                let candidates =
                    self.free_objects_within(&objects, index, reach_fp, max_candidates);
                match candidates.first() {
                    Some(&(_, target)) => {
                        // Wear on what was used to strike, whether or not the
                        // target gives.
                        for &held_id in &objects.held[index].clone() {
                            if let Some(held) = objects.table.index_of(held_id) {
                                let worn = objects.table.integrity_q16[held]
                                    - objects.table.durability_q16[held] as i32;
                                objects.table.integrity_q16[held] = worn.max(0);
                            }
                        }
                        match force_on.iter_mut().find(|entry| entry.0 == target) {
                            Some(entry) => entry.1 += force,
                            None => force_on.push((target, force, self.ids[index])),
                        }
                        objects.table.counters.struck_objects += 1;
                        // Phase 13 cue: striking an object is contact.
                        self.note_contact(index);
                        let (striker, target_id) = (self.ids[index], objects.table.ids[target]);
                        self.push_event(
                            next_tick,
                            EventKind::ObjectStruck {
                                striker,
                                target: target_id,
                                force_q16: force.min(i64::from(u32::MAX)) as u32,
                            },
                        );
                    }
                    None => terrain_strikes.push(index),
                }
            }
            // Fracture test per target, in ascending target index (= id).
            force_on.sort_by_key(|entry| entry.0);
            for (target, force, _) in force_on {
                let threshold = i64::from(objects.table.hardness_q16[target])
                    * i64::from(artifact.fracture_margin_q16)
                    >> 16;
                if force >= threshold {
                    self.fracture(
                        next_tick,
                        &mut objects,
                        &mut pending,
                        &mut destroyed,
                        target,
                        DestroyCause::Fractured,
                    );
                } else {
                    let worn = objects.table.integrity_q16[target]
                        - objects.table.durability_q16[target] as i32;
                    objects.table.integrity_q16[target] = worn.max(0);
                    if objects.table.integrity_q16[target] == 0 {
                        // Worn to nothing: a composite comes apart, a simple
                        // object is dust.
                        if objects.table.composition[target].is_empty() {
                            self.destroy_simple(
                                next_tick,
                                &mut objects,
                                &mut destroyed,
                                target,
                                DestroyCause::Fractured,
                            );
                            objects.table.counters.worn_away += 1;
                        } else {
                            self.disassemble(next_tick, &mut objects, &mut destroyed, target);
                        }
                    }
                }
            }
            // Terrain strikes, ascending organism.
            for index in terrain_strikes {
                let cell = self.cell_of(self.x_fp[index], self.y_fp[index]);
                let Some(material_id) = self.cell_material(cell) else {
                    self.refuse(
                        next_tick,
                        &mut objects,
                        index,
                        ObjectAction::Strike,
                        RefuseReason::NoYield,
                    );
                    continue;
                };
                let remaining = self.cell_yield_milli(cell);
                if remaining <= 0 {
                    self.refuse(
                        next_tick,
                        &mut objects,
                        index,
                        ObjectAction::Strike,
                        RefuseReason::Depleted,
                    );
                    continue;
                }
                if objects.table.len() + pending.len() >= artifact.max_objects as usize {
                    self.refuse(
                        next_tick,
                        &mut objects,
                        index,
                        ObjectAction::Strike,
                        RefuseReason::ObjectCap,
                    );
                    continue;
                }
                let draw = named_random(
                    self.config.world_seed,
                    next_tick,
                    RngSystem::MaterialYield,
                    cell as u64,
                    0,
                );
                let variance_q16 = 32_768 + (draw & 0x7fff) as i64;
                let volume = (artifact.extraction_milli * variance_q16 >> 16)
                    .min(remaining)
                    .max(1);
                objects.table.counters.struck_terrain += 1;
                let cap = self.config.worldmod.max_material_overrides;
                if let Some(worldmod) = self.worldmod.as_mut() {
                    let outcome =
                        worldmod.set(LAYER_MATERIAL_YIELD, cell as u32, remaining - volume, cap);
                    if outcome.is_refused() {
                        // The override cap bound: no extraction happens, and it
                        // is counted where the cap lives (the worldmod counters).
                        self.refuse(
                            next_tick,
                            &mut objects,
                            index,
                            ObjectAction::Strike,
                            RefuseReason::Depleted,
                        );
                        continue;
                    }
                }
                let def = material(material_id).expect("cell materials are registry entries");
                let record = ObjectRecord::simple(
                    0,
                    def,
                    volume,
                    self.x_fp[index],
                    self.y_fp[index],
                    next_tick,
                    CAUSE_EXTRACTED,
                    0,
                );
                pending.push(PendingObject {
                    record,
                    cause: CAUSE_EXTRACTED,
                });
                let striker = self.ids[index];
                // Phase 13 cues: extraction is contact and an object-state
                // change the striker caused, sized by the extracted volume.
                self.note_contact(index);
                self.note_object_delta(index, volume);
                self.push_event(
                    next_tick,
                    EventKind::TerrainStruck {
                        striker,
                        cell: cell as u32,
                        volume_milli: volume,
                        material_id,
                    },
                );
            }
        }

        // --- 5. Consumption of object energy, ascending organism. ---
        let consume_fp = i64::from(artifact.consume_reach_m) * i64::from(crate::FP_PER_METER);
        let assimilation = i64::from(self.config.assimilation_q16);
        let intent_eat: Vec<bool> = self
            .phase2
            .as_ref()
            .map_or_else(Vec::new, |p2| p2.intent_eat.clone());
        for index in 0..population {
            if !intent_eat.get(index).copied().unwrap_or(false) {
                continue;
            }
            let room = self.energy_capacity_of(index) - self.energy_milli[index];
            if room <= 0 {
                continue;
            }
            // Held consumables first (lowest id), then the nearest free one.
            // **Simple objects only.** A composite's energy is locked inside
            // it until it comes apart: consuming from it would move the
            // composite's stored sums away from its constituents' and break
            // the derivation `check_invariants` re-derives, and the honest
            // physical reading is the same - what is jointed is not on the
            // plate.
            let edible = |candidate: usize| {
                objects.table.energy_milli[candidate] > 0
                    && !destroyed[candidate]
                    && objects.table.composition[candidate].is_empty()
            };
            let mut target = objects.held[index]
                .iter()
                .filter_map(|&id| objects.table.index_of(id))
                .find(|&held| edible(held));
            if target.is_none() {
                target = self
                    .free_objects_within(&objects, index, consume_fp, max_candidates)
                    .into_iter()
                    .map(|(_, candidate)| candidate)
                    .find(|&candidate| edible(candidate));
            }
            let Some(target) = target else {
                continue;
            };
            let take = objects.table.energy_milli[target]
                .min(self.intake_tick)
                .max(0);
            if take <= 0 {
                continue;
            }
            let gained = ((take * assimilation) >> 16).min(room);
            if gained <= 0 {
                continue;
            }
            let raw =
                ((gained << 16) / assimilation.max(1)).min(objects.table.energy_milli[target]);
            objects.table.energy_milli[target] -= raw;
            // Mass leaves with the energy in proportion, so a carcass eaten
            // to nothing weighs nothing. Exact: the mass share is the raw
            // energy's share of what was there, remainder staying behind.
            let mass_share = if objects.table.energy_milli[target] + raw > 0 {
                objects.table.mass_milli[target] * raw / (objects.table.energy_milli[target] + raw)
            } else {
                0
            };
            objects.table.mass_milli[target] -= mass_share;
            objects.table.ledger.energy_consumed_milli += i128::from(gained);
            objects.table.ledger.energy_decayed_milli += i128::from(raw - gained);
            objects.table.ledger.mass_consumed_milli += i128::from(mass_share);
            self.energy_milli[index] += gained;
            self.ledger.assimilated_milli += i128::from(gained);
            // Phase 13 cues: consuming an object is contact, and the energy
            // taken is an object-state change the consumer caused.
            self.note_contact(index);
            self.note_object_delta(index, raw);
            objects.table.counters.consumed_events += 1;
            let (id, consumer) = (objects.table.ids[target], self.ids[index]);
            self.push_event(
                next_tick,
                EventKind::ObjectConsumed {
                    id,
                    consumer,
                    energy_milli: gained,
                },
            );
            if objects.table.energy_milli[target] == 0
                && material(objects.table.material_id[target])
                    .is_some_and(|def| def.energy_content_milli > 0)
                && objects.table.composition[target].is_empty()
            {
                self.destroy_simple(
                    next_tick,
                    &mut objects,
                    &mut destroyed,
                    target,
                    DestroyCause::Consumed,
                );
            }
        }

        // --- 6. Condition B: what landed this tick does not survive it. ---
        if artifact.ephemeral {
            for (_, table_index) in landed {
                if !destroyed[table_index] && objects.table.is_free(table_index) {
                    self.destroy_whole(
                        next_tick,
                        &mut objects,
                        &mut destroyed,
                        table_index,
                        DestroyCause::Ephemeral,
                    );
                    objects.table.counters.ephemeral_destroyed += 1;
                }
            }
        }

        // --- 6b. Observation: exposure and carrying, per organism. Read from
        // the tick-start cell index (a placed object that landed this tick
        // counts from next tick, like everything else released this tick),
        // written to counters nothing in the tick reads.
        for index in 0..population {
            let cell = self.cell_of(self.x_fp[index], self.y_fp[index]);
            let exposed = objects.cell_index.get(cell).is_some_and(|bucket| {
                bucket.iter().any(|&table_index| {
                    let table_index = table_index as usize;
                    objects.table.creator_id[table_index] != 0
                        && !destroyed[table_index]
                        && objects.table.is_free(table_index)
                })
            });
            if exposed {
                objects.table.exposure_ticks[index] += 1;
            }
            if !objects.held[index].is_empty() {
                objects.table.carry_ticks[index] += 1;
            }
        }

        // --- 7. Allocate pending creations, then compact. ---
        self.commit_pending(next_tick, &mut objects, pending, &mut destroyed);
        self.compact_objects(&mut objects, &destroyed);
        self.objects = Some(objects);
    }

    /// Queue-then-allocate: every creation queued during the pass takes its
    /// ID here, in queue order, which is a function of ID order.
    fn commit_pending(
        &mut self,
        next_tick: u64,
        objects: &mut ObjectState,
        pending: Vec<PendingObject>,
        destroyed: &mut Vec<bool>,
    ) {
        let artifact = self.config.artifact;
        for PendingObject { mut record, cause } in pending {
            if objects.table.len() >= artifact.max_objects as usize {
                // Refused by the world cap: the mass and energy that would
                // have existed go to dust so the ledger stays exact.
                objects.table.counters.refuse(RefuseReason::ObjectCap);
                objects.table.ledger.mass_dust_milli += i128::from(record.mass_milli);
                objects.table.ledger.energy_dust_milli += i128::from(record.energy_milli);
                // The source term is still booked, so dust nets it out.
                match cause {
                    CAUSE_EXTRACTED => {
                        objects.table.ledger.mass_extracted_milli += i128::from(record.mass_milli);
                        objects.table.ledger.energy_extracted_milli +=
                            i128::from(record.energy_milli);
                    }
                    CAUSE_FRACTURED => {}
                    _ => {}
                }
                continue;
            }
            record.id = self.next_entity_id;
            self.next_entity_id += 1;
            objects.table.objects_allocated_total += 1;
            match cause {
                CAUSE_EXTRACTED => {
                    objects.table.ledger.mass_extracted_milli += i128::from(record.mass_milli);
                    objects.table.ledger.energy_extracted_milli += i128::from(record.energy_milli);
                    objects.table.counters.created_extracted += 1;
                }
                CAUSE_FRACTURED => objects.table.counters.created_fractured += 1,
                CAUSE_CARCASS => {
                    objects.table.ledger.mass_carcass_milli += i128::from(record.mass_milli);
                    objects.table.ledger.energy_carcass_milli += i128::from(record.energy_milli);
                    objects.table.counters.created_carcass += 1;
                }
                _ => {}
            }
            let (id, material_id, mass_milli, energy_milli, parent_id) = (
                record.id,
                record.material_id,
                record.mass_milli,
                record.energy_milli,
                record.parent_id,
            );
            let cell = self.cell_of(record.x_fp, record.y_fp);
            let table_index = objects.table.push(record);
            destroyed.push(false);
            if objects.cell_index_valid {
                Self::cell_index_insert(objects, cell, table_index);
            }
            self.push_event(
                next_tick,
                EventKind::ObjectCreated {
                    id,
                    material_id,
                    cause,
                    mass_milli,
                    energy_milli,
                    parent_id,
                },
            );
        }
    }

    /// Compact the table with the destroyed flags; the cell index is
    /// rebuilt next tick, and `held` is already exact.
    fn compact_objects(&mut self, objects: &mut ObjectState, destroyed: &[bool]) {
        if destroyed.iter().any(|&flag| flag) {
            objects.table.retain(destroyed);
            // Table indices moved: the cell index is stale until the next
            // `SpatialIndex` phase, and nothing between here and there reads
            // it except the lifecycle sweep, which rebuilds what it needs.
            // The buckets are kept (a clear here cost a 16,384-vector
            // reallocation every tick with any destruction in it).
            objects.cell_index_valid = false;
        }
    }

    /// Destroy a simple object: its remaining mass and energy go to the sink
    /// the cause names.
    fn destroy_simple(
        &mut self,
        next_tick: u64,
        objects: &mut ObjectState,
        destroyed: &mut [bool],
        target: usize,
        cause: DestroyCause,
    ) {
        if destroyed[target] {
            return;
        }
        destroyed[target] = true;
        let mass = objects.table.mass_milli[target];
        let energy = objects.table.energy_milli[target];
        match cause {
            DestroyCause::Decayed => {
                objects.table.ledger.mass_decayed_milli += i128::from(mass);
                objects.table.ledger.energy_decayed_milli += i128::from(energy);
            }
            _ => {
                objects.table.ledger.mass_dust_milli += i128::from(mass);
                objects.table.ledger.energy_dust_milli += i128::from(energy);
            }
        }
        objects.table.mass_milli[target] = 0;
        objects.table.energy_milli[target] = 0;
        let id = objects.table.ids[target];
        // Out of the world: from the holder or the cell.
        if objects.table.holder_id[target] != 0 {
            let holder = objects.table.holder_id[target];
            if let Ok(organism) = self.ids.binary_search(&holder)
                && let Ok(position) = objects.held[organism].binary_search(&id)
            {
                objects.held[organism].remove(position);
            }
            objects.table.holder_id[target] = 0;
        } else if objects.cell_index_valid {
            let cell = self.cell_of(objects.table.x_fp[target], objects.table.y_fp[target]);
            Self::cell_index_remove(objects, cell, target);
        }
        self.push_event(
            next_tick,
            EventKind::ObjectDestroyed {
                id,
                cause: cause.id(),
            },
        );
    }

    /// Destroy an object and, if it is a composite, everything it owns, to the
    /// dust sink. Condition B's "decays to nothing": the composite's stored
    /// mass and energy are the sums the pool counted, so they are booked
    /// once at the top and the owned constituents - which the pool did not
    /// count - are simply marked destroyed with nothing booked.
    fn destroy_whole(
        &mut self,
        next_tick: u64,
        objects: &mut ObjectState,
        destroyed: &mut [bool],
        target: usize,
        cause: DestroyCause,
    ) {
        if destroyed[target] {
            return;
        }
        let composition = objects.table.composition[target].clone();
        for constituent in composition {
            if let Some(index) = objects.table.index_of(constituent) {
                self.mark_owned_destroyed(next_tick, objects, destroyed, index);
            }
        }
        objects.table.composition[target].clear();
        self.destroy_simple(next_tick, objects, destroyed, target, cause);
    }

    /// An owned constituent leaving with its composite: nothing booked (its
    /// mass was inside its composite's), nothing to remove from any index.
    fn mark_owned_destroyed(
        &mut self,
        next_tick: u64,
        objects: &mut ObjectState,
        destroyed: &mut [bool],
        index: usize,
    ) {
        if destroyed[index] {
            return;
        }
        let composition = objects.table.composition[index].clone();
        for constituent in composition {
            if let Some(inner) = objects.table.index_of(constituent) {
                self.mark_owned_destroyed(next_tick, objects, destroyed, inner);
            }
        }
        destroyed[index] = true;
        objects.table.mass_milli[index] = 0;
        objects.table.energy_milli[index] = 0;
        objects.table.owner_id[index] = 0;
        objects.table.composition[index].clear();
        let id = objects.table.ids[index];
        self.push_event(
            next_tick,
            EventKind::ObjectDestroyed {
                id,
                cause: DestroyCause::Ephemeral.id(),
            },
        );
    }

    /// A composite comes apart: its constituents return to the world at its
    /// position, unchanged, and it is destroyed. Mass- and energy-neutral by
    /// construction, since the composite's stored sums leave the pool as the
    /// constituents' stored values re-enter it.
    fn disassemble(
        &mut self,
        next_tick: u64,
        objects: &mut ObjectState,
        destroyed: &mut [bool],
        target: usize,
    ) {
        if destroyed[target] {
            return;
        }
        destroyed[target] = true;
        let (x, y) = (objects.table.x_fp[target], objects.table.y_fp[target]);
        let cell = self.cell_of(x, y);
        let id = objects.table.ids[target];
        let composition = objects.table.composition[target].clone();
        // The composite leaves the world.
        if objects.table.holder_id[target] != 0 {
            let holder = objects.table.holder_id[target];
            if let Ok(organism) = self.ids.binary_search(&holder)
                && let Ok(position) = objects.held[organism].binary_search(&id)
            {
                objects.held[organism].remove(position);
            }
            objects.table.holder_id[target] = 0;
        } else if objects.cell_index_valid {
            Self::cell_index_remove(objects, cell, target);
        }
        for constituent in composition {
            if let Some(index) = objects.table.index_of(constituent) {
                objects.table.owner_id[index] = 0;
                objects.table.holder_id[index] = 0;
                objects.table.x_fp[index] = x;
                objects.table.y_fp[index] = y;
                if objects.cell_index_valid {
                    Self::cell_index_insert(objects, cell, index);
                }
            }
        }
        objects.table.mass_milli[target] = 0;
        objects.table.energy_milli[target] = 0;
        objects.table.composition[target].clear();
        objects.table.counters.disassembled += 1;
        self.push_event(
            next_tick,
            EventKind::ObjectDestroyed {
                id,
                cause: DestroyCause::Disassembled.id(),
            },
        );
    }

    /// Fracture: a composite comes apart; a simple object becomes fragments.
    fn fracture(
        &mut self,
        next_tick: u64,
        objects: &mut ObjectState,
        pending: &mut Vec<PendingObject>,
        destroyed: &mut Vec<bool>,
        target: usize,
        cause: DestroyCause,
    ) {
        if destroyed[target] {
            return;
        }
        let artifact = self.config.artifact;
        if !objects.table.composition[target].is_empty() {
            self.disassemble(next_tick, objects, destroyed, target);
            objects.table.counters.fractured += 1;
            return;
        }
        let id = objects.table.ids[target];
        let draw = named_random(
            self.config.world_seed,
            next_tick,
            RngSystem::Artifact,
            id,
            0,
        );
        let span = u64::from(artifact.max_fragments.max(2) - 1);
        let k = 2 + (draw % span) as i64;
        let mass = objects.table.mass_milli[target];
        let energy = objects.table.energy_milli[target];
        let (x, y) = (objects.table.x_fp[target], objects.table.y_fp[target]);
        let def = *material(objects.table.material_id[target]).expect("registry material");
        let mut fragments: Vec<(i64, i64)> = Vec::with_capacity(k as usize);
        let (base_mass, rem_mass) = (mass / k, mass % k);
        let (base_energy, rem_energy) = (energy / k, energy % k);
        for slot in 0..k {
            // Remainders to the lowest new id: the first fragment.
            let m = base_mass + if slot == 0 { rem_mass } else { 0 };
            let e = base_energy + if slot == 0 { rem_energy } else { 0 };
            fragments.push((m, e));
        }
        // The parent's mass and energy are *transferred* to its fragments,
        // not created or destroyed, so nothing is booked for a fragment that
        // is created and nothing for the parent leaving; only a fragment
        // under the minimum is booked out, to dust. That is what makes
        // fracture mass- and energy-neutral to the milli with the remainder
        // on the lowest new id.
        destroyed[target] = true;
        objects.table.mass_milli[target] = 0;
        objects.table.energy_milli[target] = 0;
        if objects.cell_index_valid {
            let cell = self.cell_of(x, y);
            Self::cell_index_remove(objects, cell, target);
        }
        for (m, e) in fragments {
            if m < artifact.min_fragment_mass_milli {
                objects.table.ledger.mass_dust_milli += i128::from(m);
                objects.table.ledger.energy_dust_milli += i128::from(e);
                continue;
            }
            let mut record = ObjectRecord::simple(0, &def, 0, x, y, next_tick, CAUSE_FRACTURED, id);
            record.mass_milli = m;
            record.energy_milli = e;
            // A struck-off piece is whole in its own right; the parent's
            // wear was the parent's.
            record.integrity_q16 = INTEGRITY_WHOLE_Q16;
            pending.push(PendingObject {
                record,
                cause: CAUSE_FRACTURED,
            });
        }
        objects.table.counters.fractured += 1;
        self.push_event(
            next_tick,
            EventKind::ObjectDestroyed {
                id,
                cause: cause.id(),
            },
        );
    }

    /// The `Lifecycle` phase, before compaction: a dead organism drops what it
    /// holds at its position, ascending id, bypassing the occupancy cap.
    pub(super) fn drop_held_on_death(&mut self, next_tick: u64, dead: &[bool]) {
        let Some(mut objects) = self.objects.take() else {
            return;
        };
        for index in 0..self.ids.len() {
            if !dead[index] {
                continue;
            }
            // The organism's object history leaves with it, in the log.
            let id = self.ids[index];
            self.push_event(
                next_tick,
                EventKind::ObjectExposure {
                    id,
                    exposure_ticks: objects.table.exposure_ticks[index],
                    carry_ticks: objects.table.carry_ticks[index],
                    age_ticks: self.age_ticks[index],
                    birth_band: objects.table.birth_band[index],
                },
            );
            if objects.held[index].is_empty() {
                continue;
            }
            let held: Vec<u64> = std::mem::take(&mut objects.held[index]);
            let (x, y) = (self.x_fp[index], self.y_fp[index]);
            let holder = self.ids[index];
            for id in held {
                let Some(table_index) = objects.table.index_of(id) else {
                    continue;
                };
                objects.table.holder_id[table_index] = 0;
                objects.table.x_fp[table_index] = x;
                objects.table.y_fp[table_index] = y;
                objects.table.counters.death_drops += 1;
                let cell = self.cell_of(x, y) as u32;
                self.push_event(
                    next_tick,
                    EventKind::ObjectReleased {
                        id,
                        holder,
                        placed: false,
                        cell,
                    },
                );
            }
        }
        self.objects = Some(objects);
    }

    /// The carcass path with the section enabled: a carcass object with a
    /// fresh id, instead of a `ContestState` carcass. Called from
    /// `spawn_carcass` after its energy share is computed; returns whether it
    /// took the death.
    pub(super) fn spawn_carcass_object(
        &mut self,
        next_tick: u64,
        index: usize,
        energy: i64,
    ) -> bool {
        let Some(mut objects) = self.objects.take() else {
            return false;
        };
        let source = self.ids[index];
        let (x, y) = (self.x_fp[index], self.y_fp[index]);
        if objects.table.len() >= self.config.artifact.max_objects as usize {
            objects.table.counters.refuse(RefuseReason::ObjectCap);
            // The share leaves the organism pool with the death either way;
            // here it is booked in and straight out to dust so nothing is
            // silently dropped.
            objects.table.ledger.energy_carcass_milli += i128::from(energy);
            objects.table.ledger.energy_dust_milli += i128::from(energy);
            objects.table.ledger.mass_carcass_milli += i128::from(energy);
            objects.table.ledger.mass_dust_milli += i128::from(energy);
            self.objects = Some(objects);
            return true;
        }
        let def = material(MATERIAL_CARCASS).expect("carcass is a registry material");
        let mut record = ObjectRecord::simple(
            self.next_entity_id,
            def,
            0,
            x,
            y,
            next_tick,
            CAUSE_CARCASS,
            source,
        );
        record.mass_milli = energy;
        record.energy_milli = energy;
        self.next_entity_id += 1;
        objects.table.objects_allocated_total += 1;
        objects.table.ledger.mass_carcass_milli += i128::from(energy);
        objects.table.ledger.energy_carcass_milli += i128::from(energy);
        objects.table.counters.created_carcass += 1;
        let id = record.id;
        objects.table.push(record);
        self.push_event(
            next_tick,
            EventKind::ObjectCreated {
                id,
                material_id: MATERIAL_CARCASS,
                cause: CAUSE_CARCASS,
                mass_milli: energy,
                energy_milli: energy,
                parent_id: source,
            },
        );
        self.push_event(
            next_tick,
            EventKind::CarcassCreated {
                id,
                source,
                energy_milli: energy,
            },
        );
        self.objects = Some(objects);
        true
    }

    /// The `Lifecycle` phase, after deaths: passive decay of every unowned
    /// object, and destruction at zero.
    pub(super) fn decay_objects(&mut self, next_tick: u64) {
        let Some(mut objects) = self.objects.take() else {
            return;
        };
        let n = objects.table.len();
        let mut destroyed = vec![false; n];
        // The cell index is stale after any compaction this tick; the sweep
        // does not need it, and marking it stale makes that explicit.
        objects.cell_index_valid = false;
        for index in 0..n {
            if objects.table.owner_id[index] != 0 {
                continue;
            }
            let rate = i64::from(objects.table.decay_q16[index]);
            if rate == 0 {
                continue;
            }
            objects.table.integrity_q16[index] =
                (objects.table.integrity_q16[index] - rate as i32).max(0);
            let energy = objects.table.energy_milli[index];
            // A composite loses integrity at its fastest constituent's rate
            // and comes apart at zero; its energy and mass are its
            // constituents' and are not decayed here, or the stored sums
            // would drift from their derivation and disassembly would put
            // back what decay had already booked out.
            if energy > 0 && objects.table.composition[index].is_empty() {
                let loss = ((energy * rate) >> 16).max(1).min(energy);
                objects.table.energy_milli[index] -= loss;
                objects.table.ledger.energy_decayed_milli += i128::from(loss);
                // Mass leaves in proportion, remainder stays.
                let mass_loss = objects.table.mass_milli[index] * loss / energy;
                objects.table.mass_milli[index] -= mass_loss;
                objects.table.ledger.mass_decayed_milli += i128::from(mass_loss);
            }
            let has_energy_content = material(objects.table.material_id[index])
                .is_some_and(|def| def.energy_content_milli > 0);
            let spent = objects.table.integrity_q16[index] == 0
                || (has_energy_content
                    && objects.table.energy_milli[index] == 0
                    && objects.table.composition[index].is_empty());
            if spent {
                if objects.table.composition[index].is_empty() {
                    self.destroy_simple(
                        next_tick,
                        &mut objects,
                        &mut destroyed,
                        index,
                        DestroyCause::Decayed,
                    );
                    objects.table.counters.decayed_away += 1;
                } else {
                    self.disassemble(next_tick, &mut objects, &mut destroyed, index);
                }
            }
        }
        self.compact_objects(&mut objects, &destroyed);
        self.objects = Some(objects);
    }
}
