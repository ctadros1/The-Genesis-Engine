//! The social channel's tick work (Phase 13, ADR-0029), as a child module of
//! `world` beside `artifact_tick`, for the same reason: it reaches the
//! world's arrays without widening their visibility.
//!
//! Four entry points, one per phase that gains work when the section is
//! enabled, every one a no-op that touches nothing when it is not:
//!
//! - `Sense`: [`World::sense_social`], the perception cue vector and the
//!   committed signal field per organism, read entirely from tick-start
//!   committed state (Rule 4).
//! - `Controllers`: the gather closure in `controllers_phase2` reads the cue
//!   vector; emission requests are captured there into the social scratch.
//! - `Apply`: [`World::social_emit_phase`], charging emission and stamping
//!   the staging field in ascending emitter ID order.
//! - `Finalize`: [`World::commit_social`], decaying and committing the
//!   field, and committing this tick's contact and object-delta records
//!   into the one-tick cue memory next tick's sense reads.
//!
//! # Determinism, in one paragraph
//!
//! Perception candidates are materialized, sorted by `(distance_squared,
//! organism id)` and truncated to K before any value is read (Rule 5).
//! Emission accumulates into a staging field in ascending emitter index -
//! which is ID order - and commits once at `Finalize`, so a signal emitted
//! at tick t is observable at t+1 and never earlier, and emission order
//! cannot matter (two-phase commit; the attenuation stamp is pure addition,
//! clamped only at commit). Every draw is `named_random` on the `Signal`
//! stream: reception corruption keyed on the receiver with draw index =
//! channel, the condition-D receiver keyed on the emitter at draw index 16.
//! No value anywhere names a meaning: signal channels are numbered, and no
//! kernel code reads one and does anything specific with it.

use super::{EventKind, World};
use crate::config::Q16_ONE;
use crate::rng::{RngSystem, named_random};
use crate::social::{SOCIAL_CUE_COUNT, SOCIAL_DELTA_REFERENCE_MILLI};

/// Draw index for the condition-D receiver draw, disjoint from the
/// per-channel corruption indices (0..=3) because one organism can be both
/// an emitter and a receiver in the same tick.
const SCRAMBLE_DRAW_INDEX: u32 = 16;

impl World {
    /// Record that organism `index` was in contact this tick (fed, consumed
    /// object energy, dealt or took damage). Free when the section is off.
    pub(super) fn note_contact(&mut self, index: usize) {
        if let Some(social) = self.social.as_mut()
            && let Some(flag) = social.contact_now.get_mut(index)
        {
            *flag = true;
        }
    }

    /// Record `milli` of object-state change caused by organism `index`
    /// this tick (extraction volume, fracture, combination, consumed
    /// energy). Accumulates; normalized at `Finalize`. Free when off.
    pub(super) fn note_object_delta(&mut self, index: usize, milli: i64) {
        if let Some(social) = self.social.as_mut()
            && let Some(slot) = social.object_delta_now_milli.get_mut(index)
        {
            *slot += milli.max(0);
        }
    }

    /// Conspecifics within `range_fp` of organism `index`, as
    /// `(distance_squared, index)`, sorted by `(distance_squared, id)` and
    /// truncated to `k`: the materialized-sorted-truncated set of Rule 5.
    /// The bucket rectangle is computed from the range, so a radius wider
    /// than one bucket is still complete.
    pub(crate) fn conspecifics_within(
        &self,
        index: usize,
        range_fp: i64,
        k: usize,
    ) -> Vec<(i64, usize)> {
        let x = i64::from(self.x_fp[index]);
        let y = i64::from(self.y_fp[index]);
        let range_squared = range_fp * range_fp;
        let bucket_fp = i64::from(self.bucket_size_fp);
        let reach = ((range_fp + bucket_fp - 1) / bucket_fp).max(1) as i32;
        let bucket_x = (self.x_fp[index] / self.bucket_size_fp).min(self.buckets_x as i32 - 1);
        let bucket_y = (self.y_fp[index] / self.bucket_size_fp).min(self.buckets_y as i32 - 1);
        let mut found: Vec<(i64, u64, usize)> = Vec::new();
        for by in (bucket_y - reach).max(0)..=(bucket_y + reach).min(self.buckets_y as i32 - 1) {
            for bx in (bucket_x - reach).max(0)..=(bucket_x + reach).min(self.buckets_x as i32 - 1)
            {
                let bucket = (by as usize) * self.buckets_x as usize + bx as usize;
                for &candidate in &self.buckets[bucket] {
                    let candidate = candidate as usize;
                    if candidate == index {
                        continue;
                    }
                    let dx = i64::from(self.x_fp[candidate]) - x;
                    let dy = i64::from(self.y_fp[candidate]) - y;
                    let distance_squared = dx * dx + dy * dy;
                    if distance_squared <= range_squared {
                        found.push((distance_squared, self.ids[candidate], candidate));
                    }
                }
            }
        }
        found.sort_unstable_by_key(|entry| (entry.0, entry.1));
        found.truncate(k);
        found
            .into_iter()
            .map(|(distance_squared, _, candidate)| (distance_squared, candidate))
            .collect()
    }

    /// The `Sense` phase's contribution: the cue vector per organism, read
    /// from tick-start committed state - positions and headings are last
    /// tick's outcome at sense time, the contact and object-delta cues read
    /// the records committed at last tick's `Finalize`, and the signal
    /// values read the committed field. Nothing this tick does is visible.
    pub(super) fn sense_social(&mut self) {
        let Some(mut social) = self.social.take() else {
            return;
        };
        let population = self.ids.len();
        social.perception.clear();
        social
            .perception
            .resize(population, [0.0; SOCIAL_CUE_COUNT]);
        social.emission_q16.clear();
        social
            .emission_q16
            .resize(population, [0; crate::social::SIGNAL_CHANNELS_MAX as usize]);
        let Some(p2) = self.phase2.as_ref() else {
            self.social = Some(social);
            return;
        };
        let config = self.config.social;
        let range_fp = i64::from(config.perception_radius_m) * i64::from(crate::FP_PER_METER);
        let k = config.perception_k as usize;
        let channels = config.signal_channels as usize;
        let corruption = config.signal_corruption_q16;
        let tick = self.tick;
        let seed = self.config.world_seed;
        for index in 0..population {
            let mut cues = [0.0_f32; SOCIAL_CUE_COUNT];
            if config.perception_enabled {
                let neighbours = self.conspecifics_within(index, range_fp, k);
                let heading = p2.heading_bam[index];
                let heading_x = i64::from(crate::controller::cos_bam_q15(heading));
                let heading_y = i64::from(crate::controller::sin_bam_q15(heading));
                for (slot, &(distance_squared, other)) in neighbours.iter().enumerate() {
                    // The nine formulas live in `World::conspecific_cues` -
                    // one source shared with mate choice (ADR-0030), so the
                    // two cannot drift. The Phase 13 fixture's byte identity
                    // is what proves the extraction preserved this path's
                    // arithmetic exactly. `health_max` was a per-population
                    // precomputation of the same pure function the shared
                    // path now calls per neighbour; the values are
                    // identical.
                    let base = slot * 9;
                    let row = self.conspecific_cues(
                        p2,
                        index,
                        other,
                        distance_squared,
                        range_fp,
                        heading_x,
                        heading_y,
                        Some(&social.table),
                    );
                    cues[base..base + 9].copy_from_slice(&row);
                }
            }
            if config.signal_enabled {
                let cell = self.cell_of(self.x_fp[index], self.y_fp[index]);
                let receiver = self.ids[index];
                for channel in 0..channels {
                    let committed = social.table.committed_field_q16[cell * channels + channel]
                        as f32
                        / Q16_ONE as f32;
                    let received = if corruption > 0 {
                        // A uniform draw in +/- corruption, taken for every
                        // organism and channel whatever its bindings, so the
                        // draw pattern cannot depend on what evolved.
                        let draw =
                            named_random(seed, tick, RngSystem::Signal, receiver, channel as u32);
                        social.table.counters.corruption_draws_total += 1;
                        let unit = (draw >> 11) as f32 / (1_u64 << 53) as f32;
                        let noise = (unit * 2.0 - 1.0) * (corruption as f32 / Q16_ONE as f32);
                        (committed + noise).clamp(0.0, 1.0)
                    } else {
                        committed
                    };
                    cues[36 + channel] = received;
                }
            }
            social.perception[index] = cues;
        }
        self.social = Some(social);
    }

    /// The `Apply` phase's contribution: charge every emission and stamp the
    /// staging field, in ascending emitter index (ID order). The cost is
    /// paid whether or not any receiver exists; the stamp is pure addition
    /// into `i64` staging, clamped only at commit, so simultaneous emitters
    /// cannot depend on order.
    pub(super) fn social_emit_phase(&mut self, next_tick: u64) {
        let Some(mut social) = self.social.take() else {
            return;
        };
        let config = self.config.social;
        if !config.signal_enabled {
            self.social = Some(social);
            return;
        }
        let channels = config.signal_channels as usize;
        let base_range_fp = i64::from(config.signal_base_range_m) * i64::from(crate::FP_PER_METER);
        let cell_fp = i64::from(self.config.cell_size_fp());
        let cells_x = i64::from(self.terrain.cells_x);
        let cells_y = i64::from(self.terrain.cells_y);
        let seed = self.config.world_seed;
        let population = self.ids.len();
        for index in 0..population {
            let amplitudes = social.emission_q16[index];
            let total_amplitude: i64 = amplitudes
                .iter()
                .take(channels)
                .map(|&amplitude| i64::from(amplitude))
                .sum();
            if total_amplitude == 0 {
                continue;
            }
            // The cost, exact to the bit (D-094): `cost_milli` per whole
            // unit of amplitude per tick, the sub-milli part carried per
            // organism in Q16 fractional milli.
            let scaled = config.signal_cost_milli * total_amplitude
                + social.table.emission_remainder_milli[index];
            let charge = scaled >> 16;
            social.table.emission_remainder_milli[index] = scaled & 0xFFFF;
            let paid = charge.min(self.energy_milli[index]).max(0);
            self.energy_milli[index] -= paid;
            self.ledger.spent_milli += i128::from(paid);
            social.table.counters.signal_cost_milli_total += paid as u64;
            social.table.counters.signals_emitted_total += 1;

            // The stamp centre: the emitter, or under condition D a randomly
            // drawn other living organism - cost, attenuation and decay
            // identical, only the spatial-causal link destroyed. The draw
            // resolves against the ID-sorted organism arrays, so storage
            // order never decides it.
            let (centre_x, centre_y) = if config.scramble_delivery && population > 1 {
                let draw = named_random(
                    seed,
                    next_tick,
                    RngSystem::Signal,
                    self.ids[index],
                    SCRAMBLE_DRAW_INDEX,
                );
                let mut other = (draw % (population as u64 - 1)) as usize;
                if other >= index {
                    other += 1;
                }
                social.table.counters.scrambled_deliveries_total += 1;
                (self.x_fp[other], self.y_fp[other])
            } else {
                (self.x_fp[index], self.y_fp[index])
            };

            let mut mask = 0_u8;
            let mut peak = 0_i32;
            for (channel, &amplitude) in amplitudes.iter().enumerate().take(channels) {
                if amplitude == 0 {
                    continue;
                }
                mask |= 1 << channel;
                peak = peak.max(amplitude);
                // Range scales with amplitude; attenuation is
                // `1 - (d/range)^2`, a monotone function of distance that
                // needs no square root in fixed point.
                let range_fp = base_range_fp * i64::from(amplitude) >> 16;
                if range_fp <= 0 {
                    continue;
                }
                let range_squared = range_fp * range_fp;
                let cell_x = i64::from(centre_x) / cell_fp;
                let cell_y = i64::from(centre_y) / cell_fp;
                let reach = (range_fp + cell_fp - 1) / cell_fp;
                for cy in (cell_y - reach).max(0)..=(cell_y + reach).min(cells_y - 1) {
                    for cx in (cell_x - reach).max(0)..=(cell_x + reach).min(cells_x - 1) {
                        let centre_cell_x = cx * cell_fp + cell_fp / 2;
                        let centre_cell_y = cy * cell_fp + cell_fp / 2;
                        let dx = centre_cell_x - i64::from(centre_x);
                        let dy = centre_cell_y - i64::from(centre_y);
                        let distance_squared = dx * dx + dy * dy;
                        if distance_squared > range_squared {
                            continue;
                        }
                        let attenuation_q16 = i64::from(amplitude)
                            * (range_squared - distance_squared)
                            / range_squared;
                        let cell = (cy * cells_x + cx) as usize;
                        social.staged_field[cell * channels + channel] += attenuation_q16;
                    }
                }
            }
            let id = self.ids[index];
            self.push_event(
                next_tick,
                EventKind::SignalEmitted {
                    id,
                    channel_mask: mask,
                    peak_amplitude_q16: peak as u32,
                    cost_milli: paid,
                },
            );
        }
        self.social = Some(social);
    }

    /// The `Finalize` phase's contribution: decay-then-add commit of the
    /// signal field, and the one-tick cue records next tick's sense reads.
    pub(super) fn commit_social(&mut self) {
        let Some(mut social) = self.social.take() else {
            return;
        };
        let retain = i64::from(self.config.social.signal_retain_q16);
        let q16 = i64::from(Q16_ONE);
        for (slot, committed) in social.table.committed_field_q16.iter_mut().enumerate() {
            let decayed = i64::from(*committed) * retain >> 16;
            let next = (decayed + social.staged_field[slot]).clamp(0, q16);
            *committed = next as i32;
        }
        social.staged_field.iter_mut().for_each(|slot| *slot = 0);
        // Holding an object counts as contact, read from committed tick-end
        // state so it needs no hook in the artifact pass.
        if let Some(objects) = self.objects.as_ref() {
            for index in 0..self.ids.len() {
                if !objects.held[index].is_empty() {
                    social.contact_now[index] = true;
                }
            }
        }
        for index in 0..self.ids.len() {
            social.table.prior_contact[index] = social.contact_now[index];
            social.contact_now[index] = false;
            let delta_q16 = (social.object_delta_now_milli[index] * q16
                / SOCIAL_DELTA_REFERENCE_MILLI)
                .clamp(0, q16) as i32;
            social.table.prior_object_delta_q16[index] = delta_q16;
            social.object_delta_now_milli[index] = 0;
        }
        self.social = Some(social);
    }
}
