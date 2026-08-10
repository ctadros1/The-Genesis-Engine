//! Logical save state: a pure, validated capture of one world at a tick
//! boundary, and the fail-closed constructor that rebuilds a world from it.
//!
//! The kernel owns only the logical representation; byte encoding,
//! compression, files, and catalogs live in `sim-persist`. Static terrain
//! is not part of the state: it is regenerated deterministically from
//! `(seed, config)` on restore and verified against the recorded terrain
//! checksum, so a save cannot silently reinterpret a different world.

use crate::climate::ClimateWorld;
use crate::config::SimConfig;
use crate::contest::{Carcass, ContestState};
use crate::genome::{Genome, MEMORY_VALUES, Phenotype};
use crate::phase2::{Phase2Counters, Phase2State};
use crate::physiology::PhysiologyState;
use crate::world::{Counters, Ledger, World};
use std::fmt;

/// Bumped whenever the logical field set or meaning changes; recorded by
/// the on-disk format alongside its own framing version.
pub const SAVE_STATE_VERSION: u16 = 1;

/// Per-organism Phase 2 logical state.
#[derive(Clone, Debug, PartialEq)]
pub struct Phase2SaveState {
    pub traits: Vec<[f32; crate::genome::TRAIT_COUNT]>,
    /// Neural genes per organism, exactly `NEURAL_COUNT` each.
    pub neural: Vec<Vec<f32>>,
    pub memory: Vec<[f32; MEMORY_VALUES]>,
    pub heading_bam: Vec<u16>,
    pub speed_milli: Vec<i64>,
    pub last_turn: Vec<f32>,
    pub parents: Vec<[u64; 2]>,
    pub depth: Vec<u32>,
    pub child_count: Vec<u32>,
    pub birth_tick: Vec<u64>,
    pub counters: Phase2Counters,
}

/// Stored Phase 6 climate state.
///
/// Only the integrator is here. Biome is derived and reclassified on load,
/// and temperature under the default policy is a pure function of
/// `(base, tick)`, so neither is stored — the same rule that keeps
/// phenotypes and genome hashes out of a save.
#[derive(Clone, Debug, PartialEq)]
pub struct ClimateSaveState {
    pub moisture_milli: Vec<i64>,
    /// Stored rather than recomputed: see `ClimateState::biome`.
    pub biome: Vec<crate::climate::Biome>,
    pub capacity_loss_milli: i128,
}

/// Stored Phase 7 contest state.
#[derive(Clone, Debug, PartialEq)]
pub struct ContestSaveState {
    pub health_milli: Vec<i64>,
    pub recent_damage_milli: Vec<i64>,
    pub carcasses: Vec<Carcass>,
    pub carcass_created_milli: i128,
    pub carcass_consumed_milli: i128,
    pub carcass_decayed_milli: i128,
    pub attacks_total: u64,
    pub damage_dealt_milli: i128,
    pub deaths_by_damage_total: u64,
    pub healed_milli: i128,
}

/// Stored Phase 8 physiology state.
#[derive(Clone, Debug, PartialEq)]
pub struct PhysiologySaveState {
    pub cumulative_hazard_q16: Vec<i64>,
    pub deaths_senescence_total: u64,
    pub deaths_extrinsic_total: u64,
    pub deaths_juvenile_total: u64,
    pub thermal_cost_milli: i128,
    pub allometric_cost_milli: i128,
}

/// Stored Phase 9 schema-2 state.
///
/// Genomes are stored in their canonical encoded form, which already carries
/// its own checksum and bounds. Activations are stored because they are
/// logical state: a recurrent organism's memory lives in the prior-state
/// buffer, and recomputing it on load would silently reset it. Compiled
/// plans are **not** stored - they are a pure function of the genome and are
/// rebuilt on load, which is the same rule terrain and phenotypes follow.
#[derive(Clone, Debug, PartialEq)]
pub struct Schema2SaveState {
    pub genomes: Vec<Vec<u8>>,
    pub activation_values: Vec<Vec<f32>>,
    pub activation_prior: Vec<Vec<f32>>,
    pub activation_faults: Vec<u32>,
    pub counters: crate::structmut::MutationCounters,
}

/// Phase 10 morphology state.
///
/// **Bodies are deliberately absent.** They are a pure function of the
/// genome, which is already here, so storing them would add a fourth
/// per-organism growth term to the snapshot for no information (ADR-0019).
/// What *is* state is the developmental counters: they are hashed into the
/// checksum, so a restore that rebuilt them from zero would produce a world
/// that no longer matched the one it was saved from - the D-077 defect, one
/// phase later.
#[derive(Clone, Debug, PartialEq)]
pub struct MorphologySaveState {
    pub counters: crate::develop::DevelopCounters,
}

/// One plastic edge's saved learned state.
///
/// The edge is named by its `homology_id` rather than by the slot it
/// occupied. A slot index is only meaningful against the plan that produced
/// it, and a plan is recompiled on load from a genome that may express its
/// edges under a different plasticity budget or - once structural mutation
/// has run - a different edge set entirely. An id survives all of that, and
/// it is what lets `from_state` refuse a save whose learned rows do not line
/// up with the network they claim to belong to instead of quietly applying
/// one edge's learning to another.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LearnedEdgeSave {
    pub edge_homology_id: u32,
    pub learned_q16: i32,
    pub trace_q16: i32,
}

/// Phase 11 learned state.
///
/// **Sparse by design, and the sparsity is a budget rather than a
/// preference.** Only plastic edges appear, in ascending `edge_homology_id`
/// order, so the cost is proportional to the plasticity that actually
/// evolved rather than to the weight count. The Phase 4 record already has
/// snapshot size dominated by per-organism genome arrays at roughly 2.8 KB
/// each with a synchronous checkpoint on the tick thread; a dense learned
/// copy of every weight would roughly double it, which is the measurement
/// C11.7 exists to make and the reason `max_plastic_edges` is provisional.
///
/// This is the only section in this file whose contents cannot be recomputed
/// from the genome. A body is regrown, a phenotype is derived, a plan is
/// recompiled; a lifetime's learning has no source but the save.
#[derive(Clone, Debug, PartialEq)]
pub struct LearnSaveState {
    /// Per organism, its plastic edges sorted by `edge_homology_id`.
    pub edges: Vec<Vec<LearnedEdgeSave>>,
    /// Per organism, non-finite deltas neutralized over its lifetime.
    pub faults: Vec<u32>,
    /// Per organism, sub-milli energy owed for plastic edges, in thousandths
    /// of a milli-EU. Dropping it on save would restart every organism's
    /// bill at zero, which over a long run is a slow, invisible refund.
    pub cost_remainder: Vec<u32>,
    pub counters: crate::plasticity::PlasticityCounters,
    pub cost_milli: i128,
}

/// Complete logical world state in stable field order.
#[derive(Clone, Debug, PartialEq)]
pub struct SaveState {
    pub config: SimConfig,
    pub tick: u64,
    pub paused: bool,
    pub extinct: bool,
    pub next_entity_id: u64,
    pub terrain_checksum: u64,

    pub ids: Vec<u64>,
    pub x_fp: Vec<i32>,
    pub y_fp: Vec<i32>,
    pub energy_milli: Vec<i64>,
    pub age_ticks: Vec<u64>,
    pub cooldown_ticks: Vec<u64>,

    pub biomass_milli: Vec<i64>,
    pub ledger: Ledger,
    pub counters: Counters,
    pub phase2: Option<Phase2SaveState>,
    /// Present exactly when the config's climate section is enabled.
    pub climate: Option<ClimateSaveState>,
    /// Present exactly when the config's contest section is enabled.
    pub contest: Option<ContestSaveState>,
    /// Present exactly when the config's physiology section is enabled.
    pub physiology: Option<PhysiologySaveState>,
    /// Present exactly when the config's genome2 section is enabled.
    pub schema2: Option<Schema2SaveState>,
    pub morphology: Option<MorphologySaveState>,
    /// Present exactly when the config's plasticity section is enabled.
    pub learn: Option<LearnSaveState>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RestoreError {
    InvalidConfig(String),
    WorldGenFailed(String),
    TerrainChecksumMismatch { recorded: u64, regenerated: u64 },
    LengthMismatch { field: &'static str },
    EntityOrder,
    InvalidGenome { index: usize },
    ClimateInvalid(String),
    StateInvalid(String),
    StateChecksumMismatch { recorded: u64, actual: u64 },
}

impl fmt::Display for RestoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for RestoreError {}

impl World {
    /// Capture the complete logical state at the current tick boundary.
    pub fn export_state(&self) -> SaveState {
        let phase2 = self.phase2_state().map(|p2: &Phase2State| Phase2SaveState {
            // Empty in a schema-2 world, whose traits live in its own
            // section as part of the diploid genome.
            traits: p2.genomes.iter().map(|genome| *genome.traits()).collect(),
            neural: p2
                .genomes
                .iter()
                .map(|genome| genome.neural().to_vec())
                .collect(),
            memory: p2.memory.clone(),
            heading_bam: p2.heading_bam.clone(),
            speed_milli: p2.speed_milli.clone(),
            last_turn: p2.last_turn.clone(),
            parents: p2.parents.clone(),
            depth: p2.depth.clone(),
            child_count: p2.child_count.clone(),
            birth_tick: p2.birth_tick.clone(),
            counters: p2.counters,
        });
        SaveState {
            config: *self.config(),
            tick: self.tick_number(),
            paused: self.is_paused(),
            extinct: self.is_extinct(),
            next_entity_id: self.next_entity_id_value(),
            terrain_checksum: self.terrain().terrain_checksum,
            ids: self.organism_ids().to_vec(),
            x_fp: self.positions_x().to_vec(),
            y_fp: self.positions_y().to_vec(),
            energy_milli: self.energies().to_vec(),
            age_ticks: self.ages().to_vec(),
            cooldown_ticks: self.cooldowns().to_vec(),
            biomass_milli: self.biomass_cells().to_vec(),
            ledger: self.ledger(),
            counters: self.counters(),
            phase2,
            climate: self
                .climate_state()
                .map(|climate: &ClimateWorld| ClimateSaveState {
                    moisture_milli: climate.state.moisture_milli.clone(),
                    biome: climate.state.biome.clone(),
                    capacity_loss_milli: climate.capacity_loss_milli,
                }),
            contest: self
                .contest_state()
                .map(|contest: &ContestState| ContestSaveState {
                    health_milli: contest.health_milli.clone(),
                    recent_damage_milli: contest.recent_damage_milli.clone(),
                    carcasses: contest.carcasses.clone(),
                    carcass_created_milli: contest.carcass_created_milli,
                    carcass_consumed_milli: contest.carcass_consumed_milli,
                    carcass_decayed_milli: contest.carcass_decayed_milli,
                    attacks_total: contest.attacks_total,
                    damage_dealt_milli: contest.damage_dealt_milli,
                    deaths_by_damage_total: contest.deaths_by_damage_total,
                    healed_milli: contest.healed_milli,
                }),
            morphology: self.morphology_state().map(|state| MorphologySaveState {
                counters: state.counters,
            }),
            schema2: self.schema2_state().map(|state| Schema2SaveState {
                genomes: state.genomes.iter().map(|genome| genome.encode()).collect(),
                activation_values: state
                    .activations
                    .iter()
                    .map(|activation| activation.values.clone())
                    .collect(),
                activation_prior: state
                    .activations
                    .iter()
                    .map(|activation| activation.prior.clone())
                    .collect(),
                activation_faults: state
                    .activations
                    .iter()
                    .map(|activation| activation.faults)
                    .collect(),
                counters: state.counters,
            }),
            // Phase 11 learned state, paired with the plans that name its
            // slots. `zip` rather than two independent maps: the edge id for
            // slot `n` is in the plan, the value is in the learned row, and a
            // section that carried one without the other would be exactly the
            // silent misalignment `from_state`'s id check exists to refuse.
            //
            // A plasticity world always has schema 2 - validation refuses the
            // section otherwise - so the `and_then` cannot drop a live
            // section; it is written this way because the two `Option`s are
            // independent types and a `zip` states the dependency instead of
            // asserting it.
            learn: self.learn_state().and_then(|learn| {
                let schema2 = self.schema2_state()?;
                let mut edges = Vec::with_capacity(learn.len());
                for index in 0..learn.len() {
                    let plan = &schema2.plans[index];
                    edges.push(
                        plan.plastic_edges
                            .iter()
                            .enumerate()
                            .map(|(slot, edge)| LearnedEdgeSave {
                                edge_homology_id: edge.homology_id,
                                learned_q16: learn.learned_q16[index][slot],
                                trace_q16: learn.trace_q16[index][slot],
                            })
                            .collect(),
                    );
                }
                Some(LearnSaveState {
                    edges,
                    faults: learn.faults.clone(),
                    cost_remainder: learn.cost_remainder.clone(),
                    counters: learn.counters,
                    cost_milli: learn.cost_milli,
                })
            }),
            physiology: self
                .physiology_state()
                .map(|physiology| PhysiologySaveState {
                    cumulative_hazard_q16: physiology.cumulative_hazard_q16.clone(),
                    deaths_senescence_total: physiology.deaths_senescence_total,
                    deaths_extrinsic_total: physiology.deaths_extrinsic_total,
                    deaths_juvenile_total: physiology.deaths_juvenile_total,
                    thermal_cost_milli: physiology.thermal_cost_milli,
                    allometric_cost_milli: physiology.allometric_cost_milli,
                }),
        }
    }

    /// Rebuild a world from logical state. Fail-closed: configuration,
    /// terrain identity, lengths, ordering, genome validity, bounds, and
    /// conservation invariants are all verified; derived state (terrain,
    /// phenotypes, genome hashes, spatial buckets) is recomputed, never
    /// trusted from the save.
    pub fn from_state(state: SaveState) -> Result<World, RestoreError> {
        state
            .config
            .validate()
            .map_err(|error| RestoreError::InvalidConfig(error.to_string()))?;

        // A fresh world regenerates terrain and derived constants.
        let mut world = World::new(state.config)
            .map_err(|error| RestoreError::WorldGenFailed(error.to_string()))?;
        if world.terrain().terrain_checksum != state.terrain_checksum {
            return Err(RestoreError::TerrainChecksumMismatch {
                recorded: state.terrain_checksum,
                regenerated: world.terrain().terrain_checksum,
            });
        }

        let population = state.ids.len();
        let same_length = |length: usize, field: &'static str| -> Result<(), RestoreError> {
            if length != population {
                return Err(RestoreError::LengthMismatch { field });
            }
            Ok(())
        };
        same_length(state.x_fp.len(), "x_fp")?;
        same_length(state.y_fp.len(), "y_fp")?;
        same_length(state.energy_milli.len(), "energy_milli")?;
        same_length(state.age_ticks.len(), "age_ticks")?;
        same_length(state.cooldown_ticks.len(), "cooldown_ticks")?;
        if state.ids.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(RestoreError::EntityOrder);
        }
        if state.biomass_milli.len() != world.terrain().cell_count() {
            return Err(RestoreError::LengthMismatch {
                field: "biomass_milli",
            });
        }

        // Schema 2 presence must match the configuration, and every genome
        // is decoded through the ordinary fail-closed path rather than
        // trusted: a save is untrusted input like any other.
        let morphology_config = world.config().morphology;
        // Rebuilt by regrowing every organism, in the phase-2 loop below.
        let mut rebuilt_morphology = morphology_config
            .enabled
            .then(crate::morphstate::MorphologyState::default);
        let rebuilt_schema2 = match (world.genome2_enabled(), state.schema2) {
            (true, Some(schema2)) => {
                same_length(schema2.genomes.len(), "schema2.genomes")?;
                same_length(schema2.activation_values.len(), "schema2.activation_values")?;
                same_length(schema2.activation_prior.len(), "schema2.activation_prior")?;
                same_length(schema2.activation_faults.len(), "schema2.activation_faults")?;
                let caps = world.config().genome2.caps;
                // The same budget the world runs, so a restored plan marks
                // exactly the edges plastic that the saved one did. A plan
                // compiled under a different budget would put an organism's
                // learned deltas in different slots.
                let budget = world.config().plasticity_budget();
                let mut rebuilt = crate::schema2::Schema2State::with_capacity(population);
                for (index, bytes) in schema2.genomes.iter().enumerate() {
                    let genome = crate::genome2::Genome2::decode(bytes, &caps)
                        .map_err(|error| RestoreError::StateInvalid(error.to_string()))?;
                    if !rebuilt.push_organism(genome, budget) {
                        return Err(RestoreError::InvalidGenome { index });
                    }
                }
                for index in 0..rebuilt.activations.len() {
                    let activation = &mut rebuilt.activations[index];
                    if schema2.activation_values[index].len() != activation.values.len()
                        || schema2.activation_prior[index].len() != activation.prior.len()
                    {
                        return Err(RestoreError::LengthMismatch {
                            field: "schema2.activation",
                        });
                    }
                    if schema2.activation_values[index]
                        .iter()
                        .chain(schema2.activation_prior[index].iter())
                        .any(|value| !value.is_finite() || !(-1.0..=1.0).contains(value))
                    {
                        return Err(RestoreError::StateInvalid(
                            "activation out of bounds".to_owned(),
                        ));
                    }
                    activation
                        .values
                        .copy_from_slice(&schema2.activation_values[index]);
                    activation
                        .prior
                        .copy_from_slice(&schema2.activation_prior[index]);
                    activation.faults = schema2.activation_faults[index];
                }
                rebuilt.counters = schema2.counters;
                Some(rebuilt)
            }
            (false, None) => None,
            _ => {
                return Err(RestoreError::StateInvalid(
                    "schema2 section presence does not match configuration".to_owned(),
                ));
            }
        };

        // Phase 2 presence must match the configuration.
        let phase2_state = match (world.phase2_enabled(), state.phase2) {
            (true, Some(phase2)) => {
                // A schema-2 world stores no flat genome, so those two
                // arrays are empty by construction rather than missing.
                if rebuilt_schema2.is_none() {
                    same_length(phase2.traits.len(), "phase2.traits")?;
                    same_length(phase2.neural.len(), "phase2.neural")?;
                } else if !phase2.traits.is_empty() || !phase2.neural.is_empty() {
                    return Err(RestoreError::StateInvalid(
                        "a schema-2 save carries flat genome arrays".to_owned(),
                    ));
                }
                same_length(phase2.memory.len(), "phase2.memory")?;
                same_length(phase2.heading_bam.len(), "phase2.heading_bam")?;
                same_length(phase2.speed_milli.len(), "phase2.speed_milli")?;
                same_length(phase2.last_turn.len(), "phase2.last_turn")?;
                same_length(phase2.parents.len(), "phase2.parents")?;
                same_length(phase2.depth.len(), "phase2.depth")?;
                same_length(phase2.child_count.len(), "phase2.child_count")?;
                same_length(phase2.birth_tick.len(), "phase2.birth_tick")?;
                Some(phase2)
            }
            (false, None) => None,
            _ => {
                return Err(RestoreError::StateInvalid(
                    "phase2 section presence does not match configuration".to_owned(),
                ));
            }
        };

        // Replace the freshly spawned population with the saved one.
        let rebuilt_phase2 = match phase2_state {
            Some(phase2) => {
                let mut rebuilt = Phase2State::with_capacity(population);
                for index in 0..population {
                    // In a schema-2 world the genome, its hash, and the
                    // phenotype all come from the diploid record; the flat
                    // path is untouched and still the only one schema 1 uses.
                    let (genome, genome_hash, phenotype) = match rebuilt_schema2.as_ref() {
                        Some(state) => {
                            let genome2 = &state.genomes[index];
                            let traits = crate::world::resolve_traits(&genome2.express_traits());
                            // **Bodies are recomputed, never read.** Nothing
                            // in the save carries one: development is a pure
                            // function of `(genome, config)`, so regrowing
                            // gives the same body bit for bit, and C10.10's
                            // "snapshot size is unaffected" is a consequence
                            // of that rather than a target to hit.
                            //
                            // This is the D-065 trap's twin, and the
                            // difference is worth naming. The biome map was
                            // documented as derived and was *not* a pure
                            // function of saved state - it was a
                            // classification cached on a cadence - so
                            // recomputing it diverged. A body is a pure
                            // function of a genome, and the genome is saved.
                            let phenotype = match rebuilt_morphology.as_mut() {
                                Some(state) => {
                                    state
                                        .push_organism(genome2, &morphology_config)
                                        .map_err(|_| RestoreError::StateInvalid(format!(
                                            "organism {index} regrew a non-viable body on restore,                                              which means development is not a pure function of the                                              genome"
                                        )))?;
                                    let derived = state.derived[state.derived.len() - 1];
                                    Phenotype::from_body(&traits, &derived, &state.reference)
                                }
                                None => Phenotype::from_traits(&traits),
                            };
                            (None, crate::checksum::fnv1a64(&genome2.encode()), phenotype)
                        }
                        None => {
                            let genome = Genome::validated(
                                phase2.traits[index],
                                phase2.neural[index].clone(),
                            )
                            .map_err(|_| RestoreError::InvalidGenome { index })?;
                            let hash = genome.stable_hash();
                            let phenotype = Phenotype::derive(&genome);
                            (Some(genome), hash, phenotype)
                        }
                    };
                    rebuilt.push_organism(
                        genome,
                        genome_hash,
                        phenotype,
                        phase2.heading_bam[index],
                        phase2.parents[index],
                        phase2.depth[index],
                        phase2.birth_tick[index],
                    );
                    rebuilt.speed_milli[index] = phase2.speed_milli[index];
                    rebuilt.last_turn[index] = phase2.last_turn[index];
                    rebuilt.memory[index] = phase2.memory[index];
                    rebuilt.child_count[index] = phase2.child_count[index];
                }
                rebuilt.counters = phase2.counters;
                Some(rebuilt)
            }
            None => None,
        };

        // Climate presence must match the configuration, and the restored
        // moisture is validated and reclassified rather than trusted.
        let rebuilt_climate = match (world.climate_enabled(), state.climate) {
            (true, Some(climate)) => {
                if climate.moisture_milli.len() != world.terrain().cell_count() {
                    return Err(RestoreError::LengthMismatch {
                        field: "climate.moisture_milli",
                    });
                }
                Some(
                    ClimateWorld::from_restored(
                        world.terrain(),
                        world.config(),
                        climate.moisture_milli,
                        climate.biome,
                        climate.capacity_loss_milli,
                        state.tick,
                    )
                    .map_err(|error| RestoreError::ClimateInvalid(error.to_string()))?,
                )
            }
            (false, None) => None,
            _ => {
                return Err(RestoreError::StateInvalid(
                    "climate section presence does not match configuration".to_owned(),
                ));
            }
        };

        // Contest presence must match the configuration, and the restored
        // arrays are length-checked against the population before use.
        let rebuilt_contest = match (world.contest_enabled(), state.contest) {
            (true, Some(contest)) => {
                same_length(contest.health_milli.len(), "contest.health_milli")?;
                same_length(
                    contest.recent_damage_milli.len(),
                    "contest.recent_damage_milli",
                )?;
                if contest
                    .carcasses
                    .windows(2)
                    .any(|pair| pair[0].id >= pair[1].id)
                {
                    return Err(RestoreError::StateInvalid(
                        "carcass table is not sorted by ID".to_owned(),
                    ));
                }
                let mut rebuilt = ContestState::with_capacity(population);
                rebuilt.health_milli = contest.health_milli;
                rebuilt.recent_damage_milli = contest.recent_damage_milli;
                rebuilt.carcasses = contest.carcasses;
                rebuilt.carcass_created_milli = contest.carcass_created_milli;
                rebuilt.carcass_consumed_milli = contest.carcass_consumed_milli;
                rebuilt.carcass_decayed_milli = contest.carcass_decayed_milli;
                rebuilt.attacks_total = contest.attacks_total;
                rebuilt.damage_dealt_milli = contest.damage_dealt_milli;
                rebuilt.deaths_by_damage_total = contest.deaths_by_damage_total;
                rebuilt.healed_milli = contest.healed_milli;
                Some(rebuilt)
            }
            (false, None) => None,
            _ => {
                return Err(RestoreError::StateInvalid(
                    "contest section presence does not match configuration".to_owned(),
                ));
            }
        };

        // Physiology presence must match the configuration too, and the
        // hazard array is length-checked against the population before use.
        let rebuilt_physiology = match (world.physiology_enabled(), state.physiology) {
            (true, Some(physiology)) => {
                same_length(
                    physiology.cumulative_hazard_q16.len(),
                    "physiology.cumulative_hazard_q16",
                )?;
                let mut rebuilt = PhysiologyState::with_capacity(population);
                rebuilt.cumulative_hazard_q16 = physiology.cumulative_hazard_q16;
                rebuilt.deaths_senescence_total = physiology.deaths_senescence_total;
                rebuilt.deaths_extrinsic_total = physiology.deaths_extrinsic_total;
                rebuilt.deaths_juvenile_total = physiology.deaths_juvenile_total;
                rebuilt.thermal_cost_milli = physiology.thermal_cost_milli;
                rebuilt.allometric_cost_milli = physiology.allometric_cost_milli;
                Some(rebuilt)
            }
            (false, None) => None,
            _ => {
                return Err(RestoreError::StateInvalid(
                    "physiology section presence does not match configuration".to_owned(),
                ));
            }
        };

        // Phase 11 learned state. Presence must match the configuration on
        // the same terms as every section above, and the contents are
        // validated against the *rebuilt plans* rather than trusted, because
        // this is the one section a wrong value in cannot be caught later:
        // an out-of-clamp delta goes straight into `effective_weight`, and a
        // row whose edges do not match the plan silently applies one edge's
        // lifetime of learning to another.
        let rebuilt_learn = match (world.config().plasticity.enabled, state.learn) {
            (true, Some(learn)) => {
                same_length(learn.edges.len(), "learn.edges")?;
                same_length(learn.faults.len(), "learn.faults")?;
                same_length(learn.cost_remainder.len(), "learn.cost_remainder")?;
                let Some(schema2) = rebuilt_schema2.as_ref() else {
                    // Unreachable through `validate`, which refuses plasticity
                    // without genome2. Fail closed anyway: without the plans
                    // there is nothing to check the stored edge ids against,
                    // and admitting the section unchecked is the one thing
                    // this block exists to prevent.
                    return Err(RestoreError::StateInvalid(
                        "a plasticity save carries no schema-2 section".to_owned(),
                    ));
                };
                let mut rebuilt = crate::learnstate::LearnState::with_capacity(population);
                for (index, row) in learn.edges.iter().enumerate() {
                    let plastic = &schema2.plans[index].plastic_edges;
                    if row.len() != plastic.len() {
                        return Err(RestoreError::StateInvalid(format!(
                            "organism {index} saved {} learned edges and its rebuilt plan \
                             has {}",
                            row.len(),
                            plastic.len()
                        )));
                    }
                    // Positional identity, not set membership. Both lists are
                    // in ascending `homology_id` - the plan by construction,
                    // the save because `export_state` walks the plan - so
                    // equality slot by slot is the check. A membership test
                    // would accept a permutation, which is precisely the
                    // silent misalignment being refused: the values would all
                    // be legal, the lengths would agree, and every organism
                    // would resume with its learning attached to the wrong
                    // synapses.
                    rebuilt.push_organism(row.len());
                    for (slot, saved) in row.iter().enumerate() {
                        let LearnedEdgeSave {
                            edge_homology_id,
                            learned_q16,
                            trace_q16,
                        } = *saved;
                        if edge_homology_id != plastic[slot].homology_id {
                            return Err(RestoreError::StateInvalid(format!(
                                "organism {index} slot {slot} saved edge {edge_homology_id} \
                                 and its rebuilt plan has {}",
                                plastic[slot].homology_id
                            )));
                        }
                        rebuilt.learned_q16[index][slot] = learned_q16;
                        rebuilt.trace_q16[index][slot] = trace_q16;
                    }
                    rebuilt.faults[index] = learn.faults[index];
                    // The remainder is a *fraction* of a milli by definition,
                    // so a value at or above 1000 is a whole milli that was
                    // never charged - refused rather than normalized, because
                    // normalizing would silently forgive it.
                    let remainder = learn.cost_remainder[index];
                    if remainder >= 1_000 {
                        return Err(RestoreError::StateInvalid(format!(
                            "organism {index} saved a plasticity cost remainder of \
                             {remainder}, which is not a fraction of a milli"
                        )));
                    }
                    rebuilt.cost_remainder[index] = remainder;
                }
                rebuilt.counters = learn.counters;
                rebuilt.cost_milli = learn.cost_milli;
                // The clamp, checked through the same predicate the running
                // world's invariant uses rather than a second copy of the
                // range. `accumulate_clamped` cannot produce a value outside
                // it, so a violation here is a corrupted or hand-built save -
                // which is exactly the input this function treats as hostile.
                if let Some(index) = rebuilt.bounds_violation() {
                    return Err(RestoreError::StateInvalid(format!(
                        "organism {index} carries a learned value outside the clamp"
                    )));
                }
                Some(rebuilt)
            }
            (false, None) => None,
            _ => {
                return Err(RestoreError::StateInvalid(
                    "learn section presence does not match configuration".to_owned(),
                ));
            }
        };

        world.replace_logical_state(
            state.tick,
            state.paused,
            state.extinct,
            state.next_entity_id,
            state.ids,
            state.x_fp,
            state.y_fp,
            state.energy_milli,
            state.age_ticks,
            state.cooldown_ticks,
            state.biomass_milli,
            state.ledger,
            state.counters,
            rebuilt_phase2,
            rebuilt_climate,
            rebuilt_contest,
            rebuilt_physiology,
            rebuilt_schema2,
            {
                // Regrowing every body incremented `bodies_grown` and its
                // siblings. The counters are world state and are hashed, so
                // they come from the save; the ones accumulated while
                // rebuilding are an artifact of the rebuild and are dropped.
                if let (Some(state), Some(saved)) =
                    (rebuilt_morphology.as_mut(), state.morphology.as_ref())
                {
                    state.counters = saved.counters;
                }
                rebuilt_morphology
            },
            rebuilt_learn,
        );

        world
            .check_invariants()
            .map_err(|violation| RestoreError::StateInvalid(violation.to_string()))?;
        Ok(world)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEED: u64 = 0x5eed_cafe_f00d_beef;

    fn run_world(phase2: bool, ticks: u64) -> World {
        let mut config = if phase2 {
            SimConfig::phase2_default(SEED)
        } else {
            SimConfig::phase1_default(SEED)
        };
        config.cells_x = 64;
        config.cells_y = 64;
        config.initial_organisms = 60;
        config.max_entities = 600;
        let mut world = World::new(config).unwrap();
        for _ in 0..ticks {
            world.step();
        }
        world
    }

    #[test]
    fn round_trip_preserves_checksum_and_trajectory() {
        for phase2 in [false, true] {
            let mut original = run_world(phase2, 400);
            let checksum = original.state_checksum();
            let state = original.export_state();
            let mut restored = World::from_state(state).unwrap();
            assert_eq!(restored.state_checksum(), checksum, "phase2={phase2}");
            restored.check_invariants().unwrap();
            // The restored world continues exactly like the original.
            for _ in 0..200 {
                original.step();
                restored.step();
            }
            assert_eq!(
                restored.state_checksum(),
                original.state_checksum(),
                "post-restore divergence (phase2={phase2})"
            );
        }
    }

    #[test]
    fn terrain_checksum_mismatch_fails_closed() {
        let world = run_world(true, 50);
        let mut state = world.export_state();
        state.terrain_checksum ^= 0xdead_beef;
        assert!(matches!(
            World::from_state(state),
            Err(RestoreError::TerrainChecksumMismatch { .. })
        ));
    }

    #[test]
    fn tampered_state_fails_invariants() {
        let world = run_world(true, 100);

        // Energy tampering breaks the exact ledger.
        let mut state = world.export_state();
        if !state.energy_milli.is_empty() {
            state.energy_milli[0] += 1_000;
        }
        assert!(matches!(
            World::from_state(state),
            Err(RestoreError::StateInvalid(_))
        ));

        // Unsorted IDs are rejected.
        let mut state = world.export_state();
        if state.ids.len() >= 2 {
            state.ids.swap(0, 1);
        }
        assert!(matches!(
            World::from_state(state),
            Err(RestoreError::EntityOrder)
        ));

        // Length desync is rejected.
        let mut state = world.export_state();
        state.x_fp.pop();
        assert!(matches!(
            World::from_state(state),
            Err(RestoreError::LengthMismatch { field: "x_fp" })
        ));

        // Invalid genome values are rejected.
        let mut state = world.export_state();
        if let Some(phase2) = state.phase2.as_mut() {
            phase2.neural[0][0] = f32::NAN;
        }
        assert!(matches!(
            World::from_state(state),
            Err(RestoreError::InvalidGenome { index: 0 })
        ));

        // Phase 2 section must match the config.
        let mut state = world.export_state();
        state.phase2 = None;
        assert!(matches!(
            World::from_state(state),
            Err(RestoreError::StateInvalid(_))
        ));
    }

    #[test]
    fn paused_and_extinct_flags_survive_round_trip() {
        let mut world = run_world(false, 20);
        world.set_paused(true);
        let state = world.export_state();
        let restored = World::from_state(state).unwrap();
        assert!(restored.is_paused());
        assert_eq!(restored.state_checksum(), world.state_checksum());
    }
}
