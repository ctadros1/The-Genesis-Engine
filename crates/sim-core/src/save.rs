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
///
/// **Version 2, Phase 12.** The logical state gained a terrain-modification
/// section and a composed terrain checksum, which is a change of *meaning*
/// and not only of framing: version 1 said "terrain is a pure function of
/// `(seed, config)`" and version 2 says "terrain is that baseline composed
/// with a stored, verified delta". The framing version moves with it, to
/// ALIF format 4 (`crates/sim-persist/src/codec.rs`); the two numbers are
/// deliberately separate axes, because a framing change that carries the
/// same logical fields (format 3, which only split a section's two counts)
/// must not claim the logical state changed.
pub const SAVE_STATE_VERSION: u16 = 2;

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

/// Phase 11 per-organism action counts.
///
/// The second section in this file whose contents cannot be recomputed from
/// the genome, and for a different reason than the learned state's: these are
/// not a *function* of anything stored, they are an integral over the run.
/// Reconstructing them would need every tick replayed from zero.
///
/// **Dense rather than sparse**, unlike the learned state beside it, because
/// the shapes are opposite: learned rows are sparse by construction - most
/// edges are not plastic - while every living organism has an action every
/// tick, so a sparse histogram would carry an index alongside almost every
/// entry to save nothing. The cost is fixed and small: 7 columns x 4 bytes =
/// 28 bytes per organism, against roughly 1,700-1,900 measured for an
/// organism's other sections in C11.7's table, so under two percent.
#[derive(Clone, Debug, PartialEq)]
pub struct ActionCensusSaveState {
    /// One histogram per organism, in entity-ID order.
    pub counts: Vec<[u32; crate::actioncensus::ACTION_CLASS_COUNT]>,
    pub counters: crate::actioncensus::ActionCensusCounters,
}

/// Complete logical world state in stable field order.
#[derive(Clone, Debug, PartialEq)]
pub struct SaveState {
    pub config: SimConfig,
    pub tick: u64,
    pub paused: bool,
    pub extinct: bool,
    pub next_entity_id: u64,
    /// The **baseline** terrain checksum: `worldgen(seed, config)`, the
    /// number the format 1 fail-closed check has always compared against.
    /// Unchanged in meaning and unchanged in placement (it stays in the
    /// 112-byte snapshot header), because that check is the property the
    /// whole mutable-world design is built to preserve rather than replace.
    pub terrain_checksum: u64,
    /// The **composed** terrain checksum: the baseline with every stored
    /// override applied, over the terrain an observer would actually see.
    ///
    /// Equals `terrain_checksum` exactly when `worldmod` is `None` or its
    /// set is empty - `TerrainModState::composed_checksum` reuses the
    /// generator's tag and byte layout precisely so that identity holds.
    /// That is what lets the registered format 3 to format 4 migration write
    /// `composed := baseline` for a file that predates the layer, which is
    /// the only honest value such a file can be given.
    pub composed_terrain_checksum: u64,

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
    /// Phase 12 terrain modification set. Present exactly when the config's
    /// worldmod section is enabled.
    ///
    /// **The live type, not a parallel `TerrainModSaveState`.** Every other
    /// section here has a save-shaped twin because the live struct carries
    /// something derived (compiled plans, spatial buckets, a classified biome
    /// cache) that must not be trusted from a file. `TerrainModState` carries
    /// nothing derived: it is three sorted arrays, one accumulator, and nine
    /// counters, all of them logical state. A twin would add a field-by-field
    /// conversion in each direction, which is precisely the shape that dropped
    /// two counters in Phase 9 and the whole morphology config in Phase 10.
    /// Reusing the type means there is no conversion to leave a field out of.
    ///
    /// It is still untrusted input: `from_state` checks sortedness,
    /// uniqueness, and every value's domain before the set reaches a world,
    /// and then verifies the composed checksum over the result.
    pub worldmod: Option<crate::terrainmod::TerrainModState>,
    /// Phase 11 action census. Present exactly when the config's probe
    /// section enables it.
    pub action_census: Option<ActionCensusSaveState>,
    /// Phase 12 object table. Present exactly when the config's artifact
    /// section is enabled. The live logical type, on the terms `worldmod` is:
    /// `ObjectTable` carries nothing derived (the caches live on
    /// `ObjectState`, outside it), so there is no conversion to leave a
    /// field out of. Untrusted until `from_state` has run
    /// `ObjectTable::violation` over it.
    pub objects: Option<crate::artifact::ObjectTable>,
    /// Phase 13 social table. Present exactly when the config's social
    /// section is enabled. The live logical type, on the terms `objects`
    /// is: `SocialTable` carries nothing derived (the caches live on
    /// `SocialState`, outside it), so there is no conversion to leave a
    /// field out of. Untrusted until `from_state` has run
    /// `SocialTable::violation` over it.
    pub social: Option<crate::social::SocialTable>,
    /// Phase 14 ontogeny progress. Present exactly when the config's
    /// physiology section is enabled with its ontogeny gate on.
    pub ontogeny: Option<crate::ontogeny::OntogenySave>,
    /// Phase 14 mate-choice counters. Present exactly when the config's
    /// physiology section is enabled with its mate-choice gate on.
    pub matechoice: Option<crate::matechoice::MateChoiceSave>,
    /// Phase 15 chemistry field. Present exactly when the section is
    /// enabled. Stored, never recomputed (ADR-0020): the concentrations
    /// and ledger cannot be derived from anything. The save twin carries
    /// only logical state - the scratch buffer and the production weight
    /// map are caches rebuilt on load.
    pub chemistry: Option<ChemistrySave>,
    /// Phase 15 microbial field. Present exactly when the microbial gate
    /// is on (which requires chemistry). Stored, never recomputed, like
    /// the chemistry half; the mutation scratch buffer is a rebuilt cache.
    pub microbial: Option<MicrobialSave>,
    /// Phase 16 transition state. Present exactly when the transition
    /// gate is on. The persistence counters are real state (ADR-0032);
    /// the per-class eligibility table is a rebuilt cache.
    pub transition: Option<crate::transition::TransitionSave>,
}

/// The chemistry field's saved half: concentrations plus the ledger.
#[derive(Clone, Debug, PartialEq)]
pub struct ChemistrySave {
    pub concentrations: Vec<i64>,
    pub produced_milli: i128,
    pub deposited_milli: i128,
    pub seeded_out_milli: i128,
    pub abiogenesis_fired_total: u64,
}

/// The microbial field's saved half: per-cell per-class densities plus
/// the attribution counters.
#[derive(Clone, Debug, PartialEq)]
pub struct MicrobialSave {
    pub densities: Vec<i64>,
    pub grown_milli_total: i128,
    pub died_milli_total: i128,
    pub mutated_milli_total: i128,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RestoreError {
    InvalidConfig(String),
    WorldGenFailed(String),
    TerrainChecksumMismatch {
        recorded: u64,
        regenerated: u64,
    },
    /// The stored modification set, applied to a baseline that already
    /// matched, did not reproduce the recorded composed checksum.
    ///
    /// Distinct from `TerrainChecksumMismatch` on purpose. That one says
    /// "this save belongs to a different generated world"; this one says
    /// "this save belongs to *this* world and its delta has been altered".
    /// Collapsing them would report a tampered modification section as a
    /// seed/config mismatch and send a reader looking in the wrong place.
    ComposedTerrainChecksumMismatch {
        recorded: u64,
        composed: u64,
    },
    LengthMismatch {
        field: &'static str,
    },
    EntityOrder,
    InvalidGenome {
        index: usize,
    },
    ClimateInvalid(String),
    StateInvalid(String),
    StateChecksumMismatch {
        recorded: u64,
        actual: u64,
    },
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
            // A full recompute over every cell, paid once per save and never
            // in a tick; see `TerrainModState::composed_checksum` for why an
            // incremental FNV-1a does not exist. Measured at roughly 1 ms per
            // 65,536 cells, which is noise next to encoding a population's
            // genomes.
            composed_terrain_checksum: self.composed_terrain_checksum(),
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
            // Cloned wholesale rather than rebuilt field by field: the
            // section is logical state end to end, so there is no conversion
            // here for a field to fall out of.
            worldmod: self.worldmod_state().cloned(),
            // The same argument, for the same reason: `ActionCensus` carries
            // rows and counters and nothing derived, so a save-shaped twin
            // would exist only to give a field somewhere to be forgotten.
            action_census: self
                .action_census_state()
                .map(|census| ActionCensusSaveState {
                    counts: census.counts.clone(),
                    counters: census.counters,
                }),
            objects: self.object_state().map(|objects| objects.table.clone()),
            social: self.social_state().map(|social| social.table.clone()),
            ontogeny: self.ontogeny_state().map(|ontogeny| ontogeny.to_save()),
            matechoice: self
                .matechoice_state()
                .map(|matechoice| matechoice.to_save()),
            chemistry: self.chemistry_state().map(|chemistry| ChemistrySave {
                concentrations: chemistry.concentrations.clone(),
                produced_milli: chemistry.produced_milli,
                deposited_milli: chemistry.deposited_milli,
                seeded_out_milli: chemistry.seeded_out_milli,
                abiogenesis_fired_total: chemistry.abiogenesis_fired_total,
            }),
            microbial: self.microbial_state().map(|microbial| MicrobialSave {
                densities: microbial.densities.clone(),
                grown_milli_total: microbial.grown_milli_total,
                died_milli_total: microbial.died_milli_total,
                mutated_milli_total: microbial.mutated_milli_total,
            }),
            transition: self
                .transition_state()
                .map(|transition| transition.to_save()),
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
        let mut rebuilt_phase2 = match phase2_state {
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

        // Phase 12 terrain modification. Presence must match the
        // configuration on the same terms as every section above, and the
        // payload is checked before it can reach a world rather than after.
        //
        // **Sortedness and uniqueness are checked here, not only by
        // `check_invariants` afterwards.** They are what make `get` a binary
        // search, and a binary search over an unsorted array does not fail -
        // it silently finds the wrong cell or no cell at all. The invariant
        // check at the end of this function would catch the disorder, but
        // only after `composed_terrain_checksum` had already walked the set
        // and produced a number that means nothing, so the error a reader
        // would see is a composed-checksum mismatch on a save whose real
        // defect is its ordering.
        let rebuilt_worldmod = match (world.config().worldmod.enabled, state.worldmod) {
            (true, Some(worldmod)) => {
                if let Some(index) = worldmod.order_violation() {
                    return Err(RestoreError::StateInvalid(format!(
                        "terrain modification entry {index} breaks strict ascending \
                         (layer, cell) order or duplicates its predecessor"
                    )));
                }
                if let Some(index) = worldmod.bounds_violation(world.terrain().cell_count()) {
                    return Err(RestoreError::StateInvalid(format!(
                        "terrain modification entry {index} carries a layer id, cell index, \
                         or value outside its domain"
                    )));
                }
                Some(worldmod)
            }
            (false, None) => None,
            _ => {
                return Err(RestoreError::StateInvalid(
                    "worldmod section presence does not match configuration".to_owned(),
                ));
            }
        };

        // Phase 11 action census. Presence must match the configuration on
        // the same terms as every section above, and the row count must match
        // the population before it reaches a world - not merely before
        // `check_invariants` runs, because a census that is one row long in a
        // 400-organism world would be indexed by `record` on the very next
        // tick and panic there instead of failing here with a name.
        let rebuilt_census = match (
            world.config().probe.enabled && world.config().probe.action_census_enabled,
            state.action_census,
        ) {
            (true, Some(census)) => {
                same_length(census.counts.len(), "action_census.counts")?;
                let mut rebuilt = crate::actioncensus::ActionCensus::with_capacity(population);
                for row in &census.counts {
                    rebuilt.push_organism();
                    let slot = rebuilt.len() - 1;
                    rebuilt.counts[slot] = *row;
                }
                rebuilt.counters = census.counters;
                Some(rebuilt)
            }
            (false, None) => None,
            _ => {
                return Err(RestoreError::StateInvalid(
                    "action census section presence does not match configuration".to_owned(),
                ));
            }
        };

        // Phase 12 objects. Presence must match the configuration; the table
        // is checked structurally and against its own ledger *here*, by name,
        // before it reaches a world, so a reader is told "object 7's
        // composition names an absent constituent" rather than being sent to
        // the state checksum. The two caches are rebuilt from the table.
        let rebuilt_objects = match (world.config().artifact.enabled, state.objects) {
            (true, Some(table)) => {
                let max_depth = world.config().artifact.max_composition_depth.min(255) as u8;
                if let Some(violation) = table.violation(max_depth) {
                    return Err(RestoreError::StateInvalid(format!(
                        "object table: {violation:?}"
                    )));
                }
                // The per-organism observations arrive in the table and must
                // be population-long; the caches are rebuilt from it.
                same_length(table.exposure_ticks.len(), "objects.exposure_ticks")?;
                let mut objects = crate::artifact::ObjectState::from_table(table);
                objects.band_thresholds = world.capacity_band_thresholds();
                objects.held = vec![Vec::new(); population];
                objects.intents = vec![Default::default(); population];
                objects.perception = vec![[0.0; 6]; population];
                objects.rebuild_held(&state.ids);
                if !objects.held_is_consistent(&state.ids) {
                    return Err(RestoreError::StateInvalid(
                        "an object is held by an organism that is not in the save".to_owned(),
                    ));
                }
                Some(objects)
            }
            (false, None) => None,
            _ => {
                return Err(RestoreError::StateInvalid(
                    "object section presence does not match configuration".to_owned(),
                ));
            }
        };

        // Phase 13 social state. Presence must match the configuration; the
        // table is checked structurally *here*, by name, before it reaches a
        // world, so a reader is told "the field length is wrong" rather than
        // being sent to the state checksum. The caches are rebuilt from it.
        let rebuilt_social = match (world.config().social.enabled, state.social) {
            (true, Some(table)) => {
                let config = world.config();
                if let Some(violation) = table.violation(
                    (config.cells_x as usize) * (config.cells_y as usize),
                    config.social.signal_channels,
                    population,
                ) {
                    return Err(RestoreError::StateInvalid(format!(
                        "social table: {violation}"
                    )));
                }
                Some(crate::social::SocialState::from_table(table))
            }
            (false, None) => None,
            _ => {
                return Err(RestoreError::StateInvalid(
                    "social section presence does not match configuration".to_owned(),
                ));
            }
        };

        // Phase 14 ontogeny progress. Presence must match the configuration,
        // and the save is validated structurally against the *restored*
        // bodies before it reaches the world; the caches are rebuilt from
        // those bodies exactly as the bodies themselves were rebuilt from
        // genomes.
        let ontogeny_enabled = world.config().physiology.enabled
            && world.config().physiology.ontogeny_enabled;
        let rebuilt_ontogeny = match (ontogeny_enabled, state.ontogeny) {
            (true, Some(save)) => {
                let bodies = rebuilt_morphology
                    .as_ref()
                    .map(|state| state.bodies.as_slice())
                    .unwrap_or(&[]);
                Some(
                    crate::ontogeny::OntogenyState::from_save(
                        save,
                        bodies,
                        world.config().morphology.lattice,
                    )
                    .map_err(|reason| RestoreError::StateInvalid(format!("ontogeny: {reason}")))?,
                )
            }
            (false, None) => None,
            _ => {
                return Err(RestoreError::StateInvalid(
                    "ontogeny section presence does not match configuration".to_owned(),
                ));
            }
        };
        // Phase 14 mate choice. Presence must match the configuration; the
        // weights cache is expressed from the restored genomes exactly as
        // phenotypes are, and only the counters come from the save.
        let matechoice_enabled = world.config().physiology.enabled
            && world.config().physiology.mate_choice_enabled;
        let rebuilt_matechoice = match (matechoice_enabled, state.matechoice) {
            (true, Some(save)) => {
                let schema2 = rebuilt_schema2.as_ref().ok_or_else(|| {
                    RestoreError::StateInvalid(
                        "mate choice requires the schema-2 section".to_owned(),
                    )
                })?;
                let mut matechoice = crate::matechoice::MateChoiceState::with_capacity(
                    schema2.genomes.len(),
                );
                for genome in &schema2.genomes {
                    matechoice.push_organism(genome);
                }
                matechoice.choices_total = save.choices_total;
                matechoice.scrambled_choices_total = save.scrambled_choices_total;
                Some(matechoice)
            }
            (false, None) => None,
            _ => {
                return Err(RestoreError::StateInvalid(
                    "matechoice section presence does not match configuration".to_owned(),
                ));
            }
        };

        // Phase 15 chemistry. Presence must match the configuration; the
        // saved arrays are checked structurally by name, and the derived
        // caches (scratch, production weights) are rebuilt from config.
        let rebuilt_chemistry = match (world.config().chemistry.enabled, state.chemistry) {
            (true, Some(save)) => {
                let config = *world.config();
                let cells = config.cells_x as usize * config.cells_y as usize;
                if save.concentrations.len() != cells * crate::chemistry::SUBSTRATE_COUNT {
                    return Err(RestoreError::StateInvalid(format!(
                        "chemistry carries {} concentrations for {} cells",
                        save.concentrations.len(),
                        cells
                    )));
                }
                if save.concentrations.iter().any(|&value| value < 0) {
                    return Err(RestoreError::StateInvalid(
                        "chemistry concentration is negative".to_owned(),
                    ));
                }
                let mut chemistry = crate::chemistry::ChemistryState::new(
                    config.cells_x,
                    config.cells_y,
                    &config.chemistry,
                );
                chemistry.concentrations = save.concentrations;
                chemistry.produced_milli = save.produced_milli;
                chemistry.deposited_milli = save.deposited_milli;
                chemistry.seeded_out_milli = save.seeded_out_milli;
                chemistry.abiogenesis_fired_total = save.abiogenesis_fired_total;
                chemistry.rebuild_derived(config.cells_x, config.cells_y, &config.chemistry);
                Some(chemistry)
            }
            (false, None) => None,
            _ => {
                return Err(RestoreError::StateInvalid(
                    "chemistry section presence does not match configuration".to_owned(),
                ));
            }
        };

        // Phase 15 microbial. Same contract as the chemistry half: presence
        // must match the gate, the densities are validated structurally, and
        // the mutation scratch buffer is a rebuilt cache.
        let microbial_enabled =
            world.config().chemistry.enabled && world.config().chemistry.microbial_enabled;
        let rebuilt_microbial = match (microbial_enabled, state.microbial) {
            (true, Some(save)) => {
                let config = *world.config();
                let cells = config.cells_x as usize * config.cells_y as usize;
                let slots = cells * crate::microbial::class_count(&config.chemistry);
                if save.densities.len() != slots {
                    return Err(RestoreError::StateInvalid(format!(
                        "microbial carries {} densities for {} slots",
                        save.densities.len(),
                        slots
                    )));
                }
                if save.densities.iter().any(|&value| value < 0) {
                    return Err(RestoreError::StateInvalid(
                        "microbial density is negative".to_owned(),
                    ));
                }
                let mut microbial =
                    crate::microbial::MicrobialState::new(cells, &config.chemistry);
                microbial.densities = save.densities;
                microbial.grown_milli_total = save.grown_milli_total;
                microbial.died_milli_total = save.died_milli_total;
                microbial.mutated_milli_total = save.mutated_milli_total;
                microbial.rebuild_derived();
                Some(microbial)
            }
            (false, None) => None,
            _ => {
                return Err(RestoreError::StateInvalid(
                    "microbial section presence does not match configuration".to_owned(),
                ));
            }
        };

        // Phase 16 transition. Same contract again: presence must match
        // the gate, the counters are validated structurally, the
        // eligibility table is a rebuilt cache. Every field of the save
        // twin is spelled here (D-077).
        let rebuilt_transition = match (world.config().transition.enabled, state.transition) {
            (true, Some(save)) => {
                let config = *world.config();
                let cells = config.cells_x as usize * config.cells_y as usize;
                let slots = cells * crate::microbial::class_count(&config.chemistry);
                if save.persistence.len() != slots {
                    return Err(RestoreError::StateInvalid(format!(
                        "transition carries {} persistence counters for {} slots",
                        save.persistence.len(),
                        slots
                    )));
                }
                if save.materialized_milli < 0 {
                    return Err(RestoreError::StateInvalid(
                        "transition materialized_milli is negative".to_owned(),
                    ));
                }
                let crate::transition::TransitionSave {
                    persistence,
                    materialized_total,
                    events_total,
                    materialized_milli,
                    deferred_cap_total,
                    deferred_capacity_total,
                    refused_total,
                } = save;
                let mut transition = crate::transition::TransitionState::new(
                    cells,
                    &config.chemistry,
                    &config.transition,
                );
                transition.persistence = persistence;
                transition.materialized_total = materialized_total;
                transition.events_total = events_total;
                transition.materialized_milli = materialized_milli;
                transition.deferred_cap_total = deferred_cap_total;
                transition.deferred_capacity_total = deferred_capacity_total;
                transition.refused_total = refused_total;
                transition.rebuild_derived(&config.chemistry, &config.transition);
                Some(transition)
            }
            (false, None) => None,
            _ => {
                return Err(RestoreError::StateInvalid(
                    "transition section presence does not match configuration".to_owned(),
                ));
            }
        };

        // A restored organism wears its GROWN body, not its adult one. The
        // phenotypes rebuilt above came from full bodies (the only bodies
        // the morphology rebuild knows); re-applying the grown prefix here
        // is the same `apply_body` arithmetic the birth path and the growth
        // pass use, so a saved juvenile resumes with the phenotype it was
        // saved with rather than its adult's until the next activation.
        if let (Some(ontogeny), Some(morphology), Some(phase2)) = (
            rebuilt_ontogeny.as_ref(),
            rebuilt_morphology.as_ref(),
            rebuilt_phase2.as_mut(),
        ) {
            for index in 0..ontogeny.len() {
                phase2.phenotypes[index]
                    .apply_body(&ontogeny.derived_grown[index], &morphology.reference);
            }
        }

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
            rebuilt_worldmod,
            rebuilt_census,
            rebuilt_objects,
            rebuilt_social,
            rebuilt_ontogeny,
            rebuilt_matechoice,
            rebuilt_chemistry,
            rebuilt_microbial,
            rebuilt_transition,
        );

        // Step 5 of the restore order in
        // `specifications/mutable-world-state.md`: the baseline was verified
        // above, the delta has been applied, and the composed field is now
        // checked before anything else looks at it.
        //
        // **The two checks are not redundant and neither subsumes the
        // other.** The baseline check catches a save presented against a
        // different generated world and cannot see the delta at all. This one
        // catches a delta that was altered after the save was written, which
        // leaves the baseline perfectly intact. Both fail closed; together
        // they restore the format 1 guarantee - "a restore either reproduces
        // the exact recorded world or fails with a typed error" - for a world
        // whose terrain is no longer a pure function of `(seed, config)`.
        //
        // Runs for a disabled world too, where both sides are the baseline
        // checksum. That is not a wasted comparison: it is what makes a
        // composed checksum smuggled into the metadata of a world that has no
        // modification section a decode failure instead of an ignored field.
        let composed = world.composed_terrain_checksum();
        if composed != state.composed_terrain_checksum {
            return Err(RestoreError::ComposedTerrainChecksumMismatch {
                recorded: state.composed_terrain_checksum,
                composed,
            });
        }

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
