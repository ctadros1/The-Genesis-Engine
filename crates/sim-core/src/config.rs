//! Versioned experiment configuration for the Phase 1 minimum simulation.
//!
//! Every field is part of the canonical config hash. Changing a default or a
//! formula constant here creates a new experiment lineage; never silently
//! reinterpret an old run.

use crate::checksum::Fnv1a64;
use std::fmt;

/// Bumped whenever the meaning or set of config fields changes.
pub const CONFIG_SCHEMA_VERSION: u32 = 1;

/// Version tag for the Phase 1 behavioral rule set (movement, feeding,
/// crowding, reproduction, death). Included in the config hash.
pub const BEHAVIOR_POLICY_VERSION: &str = "phase1-behavior-v1";

/// Version tag for the Phase 2 behavioral rule set (inherited controllers,
/// heading/throttle movement, paired-parent reproduction, ancestry).
/// Included in the config hash only when `phase2.enabled` is true; enabling
/// Phase 2 always starts a new replay lineage.
pub const PHASE2_BEHAVIOR_POLICY_VERSION: &str = "phase2-behavior-v1";

/// Q16 fixed-point one (65536 == 1.0).
pub const Q16_ONE: u32 = 65536;

/// Hard process-safety ceiling independent of configuration.
pub const ABSOLUTE_MAX_ENTITIES: u32 = 200_000;

const MAX_CELLS_PER_AXIS: u32 = 4_096;

/// Complete Phase 1 configuration. All values are experimental policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SimConfig {
    pub world_seed: u64,
    /// Raster cells along x/y. World meters = cells * cell_size_m.
    pub cells_x: u32,
    pub cells_y: u32,
    pub cell_size_m: u32,
    pub initial_organisms: u32,
    /// Process safety ceiling; births beyond it are rejected, never culled.
    pub max_entities: u32,
    /// Fixed tick length in simulated milliseconds.
    pub dt_ms: u32,

    // Resource policy (logistic regrowth).
    /// Logistic growth rate r per simulated second, Q16.
    pub growth_rate_q16_per_s: u32,
    /// Carrying capacity K of the most suitable cell, milli-biomass.
    pub cell_capacity_milli: i64,
    /// Initial biomass as a Q16 fraction of each cell's capacity.
    pub initial_biomass_q16: u32,

    // Organism energetics.
    pub energy_max_milli: i64,
    pub initial_energy_milli: i64,
    pub basal_cost_milli_per_s: i64,
    /// Movement cost per simulated second while moving at cruise speed.
    pub move_cost_milli_per_s: i64,
    pub intake_rate_milli_per_s: i64,
    /// Assimilation efficiency, Q16 fraction in (0, 1].
    pub assimilation_q16: u32,
    /// Cruise speed in meters per simulated second, Q16.
    pub speed_mps_q16: u32,

    // Crowding (competition proxy; exercises the spatial index).
    pub crowding_radius_m: u32,
    pub crowding_threshold: u32,
    pub crowding_cost_milli_per_s: i64,

    // Lifecycle.
    pub maturity_age_ticks: u64,
    pub max_age_ticks: u64,
    /// Asexual reproduction gate; disabled when `reproduction_enabled` is 0.
    pub reproduction_enabled: bool,
    pub repro_threshold_milli: i64,
    pub offspring_energy_milli: i64,
    pub repro_overhead_milli: i64,
    pub repro_cooldown_ticks: u64,

    // World generation.
    /// Elevation at or above this Q16 threshold is land before masking.
    pub land_threshold_q16: u32,
    pub min_land_fraction_q16: u32,
    pub max_land_fraction_q16: u32,

    /// Phase 2 policy section. When `phase2.enabled` is false, the world
    /// behaves bit-identically to Phase 1 and this section is excluded from
    /// the config hash, preserving Phase 1 fixtures and replay lineages.
    pub phase2: Phase2Config,

    /// Phase 6 climate and biome section, disabled by default. Follows the
    /// same D-014 rule: excluded from the config hash and behaviorally inert
    /// when disabled, so a disabled world takes the exact Phase 1/2 code
    /// paths and reproduces both fixtures.
    pub climate: ClimateConfig,

    /// Phase 6 origin section. Excluded from the config hash while it holds
    /// its documented defaults, which are exactly the Phase 1/2 founder
    /// behavior, so both fixtures are preserved.
    pub origin: OriginConfig,

    /// Phase 7 contest section, disabled by default. Same D-014 rule: a
    /// disabled section is behaviorally inert, excluded from the config
    /// hash, and appends nothing to the checksum, so both fixtures survive.
    pub contest: ContestConfig,
    /// Phase 8 demography section, disabled by default. Same D-014 rule: a
    /// disabled section is behaviorally inert, excluded from the config
    /// hash, and appends nothing to the checksum, so every earlier fixture
    /// survives.
    pub physiology: PhysiologyConfig,
    /// Phase 9 genome schema 2 section, disabled by default. A world is
    /// schema 1 or schema 2 by config; there is no mixed-schema world and no
    /// migration between them.
    pub genome2: Genome2Config,
    pub morphology: MorphologyConfig,
    /// Phase 11 plasticity section, disabled by default. Same D-014 rule.
    ///
    /// This section is what `specifications/determinism-extensions.md` Rule 0
    /// demands of Phase 11 by name: `PlasticityGenes` and
    /// `EDGE_FLAG_PLASTIC` are already on every schema-2 edge, so an
    /// implementation that acted on them **without its own gate** would move
    /// the Phase 9 fixture while every section that was disabled stayed
    /// disabled. Disabled, no edge compiles plastic, the learn phase writes
    /// nothing, no learned state exists, and the checksum appends nothing.
    pub plasticity: PlasticityConfig,
    /// Phase 12 mutable-world section, disabled by default. Same D-014 rule,
    /// and here it carries the heaviest obligation any section has carried:
    /// C12.8 requires **four** fixtures to reproduce exactly with this
    /// disabled - Phase 1, Phase 2, Phase 9, and Phase 11. Disabled, no
    /// modification state exists, both composed accessors on `World` return
    /// the raw terrain value through the pre-Phase-12 code path, nothing is
    /// appended to the config hash or the state checksum, and the relocation
    /// schedule never runs.
    pub worldmod: WorldModConfig,
    /// Phase 11 measurement section, disabled by default. Same D-014 rule as
    /// every section above, and it carries the same obligation Phase 12's
    /// does: **five** fixtures must reproduce with this disabled - Phase 1,
    /// Phase 2, Phase 5, Phase 9, and Phase 11. Disabled, no census array
    /// exists, no marker locus is written into a founder, nothing is appended
    /// to the config hash or the state checksum, and no locus type that did
    /// not exist before appears in any genome.
    pub probe: ProbeConfig,
    /// Phase 12 artifact section, disabled by default. Same D-014 rule and
    /// the same four-fixture obligation `worldmod` carries. Disabled, no
    /// object table exists, no object channel is offered, the world's channel
    /// registry version is 1, nothing is appended to the config hash or the
    /// state checksum, and `spawn_carcass` takes the Phase 7 path.
    pub artifact: ArtifactConfig,
    /// Phase 13 social section, disabled by default. Same D-014 rule, and
    /// it carries the heaviest fixture obligation yet: **five** fixtures
    /// must reproduce with this disabled - Phase 1, Phase 2, Phase 9,
    /// Phase 11, and Phase 12 (as re-pinned by D-119). Disabled, no
    /// perception is gathered, no signal field exists, no social channel is
    /// offered, rule 5 is absent from the effective rule space, and nothing
    /// is appended to the config hash or the state checksum.
    pub social: SocialConfig,
    /// Phase 15 chemistry field (ADR-0031). Inert when disabled; the
    /// Phase 13 fixture reproduces exactly.
    pub chemistry: ChemistryConfig,
    /// Phase 16 field-to-individual transition (ADR-0032). Inert when
    /// disabled; the Phase 15 fixture reproduces exactly.
    pub transition: TransitionConfig,
}

/// Versioned Phase 12 artifact policy (`lifesim-artifact-v2`, ADR-0028;
/// v1 -> v2 is the D-118 inert-arm fix, D-119).
///
/// Enabling this adds objects to the world, eleven channels to the registry
/// the organisms of this world may bind, one pass to `Apply` and one to
/// `Lifecycle`. It changes no rule any earlier phase runs: movement gains one
/// entry check, the movement cost gains one multiplier that is exactly one
/// for an organism holding nothing, and consumption of an object's energy
/// runs beside biomass feeding on the same arithmetic.
///
/// # There is no recipe here
///
/// Nothing in this struct names a combination, a material pair, a tool, or a
/// structure. Every field is a cap, a cost, a threshold on a physical
/// quantity, or a rate. The three fields that realise the campaign's control
/// arms (`inert`, `ephemeral`, and `max_composition_depth = 0`) remove
/// physics; none adds any.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArtifactConfig {
    pub enabled: bool,
    /// Condition C: the five verbs resolve, cost, count and event, and
    /// confer nothing. Separates "actions fire" from "actions pay" (the
    /// plan's four-condition design). Consumption, decay, exposure and
    /// carry accounting still run (`lifesim-artifact-v2`, D-118): the
    /// control removes exactly the verbs, not the object ecology.
    pub inert: bool,
    /// Condition B: an object dropped or placed this tick is destroyed at the
    /// end of it, ledgered to dust. Persistence removed, nothing else.
    pub ephemeral: bool,

    // Caps (C12.7). Each rejects deterministically, counts, and events.
    pub max_objects: u32,
    pub max_objects_per_cell: u32,
    /// Depth of a composite; 0 is condition D, simple objects only.
    pub max_composition_depth: u32,
    pub max_composition_breadth: u32,
    pub max_held_objects: u32,
    /// Truncation of every sorted candidate set (Rule 5). Enters the hash
    /// because changing it is a replay-lineage change.
    pub max_candidates: u32,

    // Carrying.
    /// Capacity at body scale 1,000; scales with `body_scale_milli` (D-085).
    pub carry_capacity_milli: i64,
    /// Movement-cost multiplier per unit of carried / capacity, Q16.
    pub carry_move_cost_q16: u32,
    /// Per-second hold cost at full load, scaled by carried / capacity.
    pub hold_cost_milli_per_s: i64,

    // Costs, charged on every attempt whether or not it succeeds.
    pub action_cost_milli: i64,
    pub strike_cost_milli: i64,
    /// Request value, Q16, above which a bound action channel fires.
    pub action_threshold_q16: i32,

    // Reach.
    pub reach_m: u32,
    pub consume_reach_m: u32,
    pub perception_range_m: u32,

    // Striking and fracture.
    /// Bare force at body scale 1,000, in hardness units (Q16).
    pub strike_force_q16: u32,
    /// Held mass that adds one full hardness to the force.
    pub strike_mass_reference_milli: i64,
    /// Multiplier on hardness the summed force must reach, Q16.
    pub fracture_margin_q16: u32,
    /// Fragment count upper bound; the lower bound is 2.
    pub max_fragments: u32,
    /// A fragment lighter than this is dust rather than an object.
    pub min_fragment_mass_milli: i64,

    // Combination.
    /// Joint draws below this fail the attempt, Q16.
    pub joint_floor_q16: u32,

    // Occupancy.
    /// A free object at or above this mass blocks entry to its cell.
    pub blocking_mass_milli: i64,

    // Terrain yield (the material-yield layer's producer and consumer).
    pub terrain_yield_milli: i64,
    /// Volume per terrain strike before variance and density.
    pub extraction_milli: i64,
    pub yield_regen_milli: i64,
    pub yield_regen_interval_ticks: u64,
    /// Relative elevation above the coastline, Q16, at or above which a cell
    /// yields stone; wood at or above `wood_relative_q16`; fiber below.
    pub stone_relative_q16: u32,
    pub wood_relative_q16: u32,
}

/// Largest composition depth a config may ask for: `depth` is a `u8` in the
/// object table and the checked identity is `depth <= cap`.
pub const MAX_COMPOSITION_DEPTH: u32 = 16;
/// Largest fragment fan-out a config may ask for.
pub const MAX_FRAGMENTS: u32 = 16;

impl ArtifactConfig {
    /// Documented Phase 12 defaults (disabled by default). Provisional, as
    /// every cap and rate in this repo is; the specification records the
    /// reasoning beside each.
    pub fn artifact_default() -> Self {
        Self {
            enabled: false,
            inert: false,
            ephemeral: false,
            max_objects: 4_096,
            max_objects_per_cell: 8,
            max_composition_depth: 4,
            max_composition_breadth: 8,
            max_held_objects: 1,
            max_candidates: 8,
            carry_capacity_milli: 4_000,
            carry_move_cost_q16: Q16_ONE,
            hold_cost_milli_per_s: 20,
            action_cost_milli: 60,
            strike_cost_milli: 120,
            action_threshold_q16: (Q16_ONE / 2) as i32,
            reach_m: 2,
            consume_reach_m: 2,
            perception_range_m: 8,
            strike_force_q16: 4 * Q16_ONE,
            strike_mass_reference_milli: 2_000,
            fracture_margin_q16: Q16_ONE,
            max_fragments: 4,
            // 100, not the 400 first written: a pilot on a disjoint seed
            // (0x9999, 20,000 ticks) showed extracted wood at ~385 milli-mass
            // and fiber at ~240, so under 400 no fragment of either could
            // ever be an object and every strike on them was dust. The rule
            // that set it: below half the lightest material's smallest
            // extraction, so fracture can yield fragments for every material
            // (fiber 800 * 0.5 volume * 300 density / 1000 = 120; half is
            // 60; 100 leaves three-way fiber splits as dust and two-way as
            // objects). A rule about the mechanism running, not about any
            // outcome.
            min_fragment_mass_milli: 100,
            joint_floor_q16: Q16_ONE / 4,
            blocking_mass_milli: 3_000,
            terrain_yield_milli: 6_000,
            extraction_milli: 800,
            yield_regen_milli: 400,
            yield_regen_interval_ticks: 600,
            stone_relative_q16: 39_322,
            wood_relative_q16: 16_384,
        }
    }
}

/// Versioned Phase 13 social policy (`lifesim-social-v1`, ADR-0029).
///
/// Enabling this adds forty-four channels to the registry this world's
/// organisms may bind (nine cues per neighbour slot, the signal-field
/// inputs, and the emission outputs), one gather to `Sense`, one pass to
/// `Apply` (emission into the staging field) and one to `Finalize` (decay
/// and commit). Nothing here names a meaning: signal channels are numbered,
/// not named, no kernel code reads one and does anything specific with it,
/// and no cue is a label (ADR-0022 A3/A4).
///
/// # The four condition gates
///
/// `perception_enabled` and `signal_enabled` split conditions A/B/C of the
/// plan's design; `scramble_delivery` is condition D (emission deposited at
/// a randomly drawn other organism, identical cost, the spatial-causal link
/// destroyed); `observational_enabled` is condition P against S (rule 5
/// offered or absent). Each removes physics; none adds any.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SocialConfig {
    pub enabled: bool,
    /// Perception of the K nearest conspecifics. Off, the cue channels read
    /// zero for bound genomes and the sense phase gathers nothing (condition
    /// C keeps the registry width so the mutation spectrum matches A's).
    pub perception_enabled: bool,
    /// The signal field. Off, emission requests are charged nothing, deposit
    /// nothing, and the field does not exist (condition B).
    pub signal_enabled: bool,
    /// Condition D: every emission lands centered on a randomly drawn other
    /// living organism instead of the emitter. Cost, attenuation and decay
    /// identical to condition A.
    pub scramble_delivery: bool,
    /// Condition P against S: whether rule 5 (Observational) is in the
    /// effective plasticity rule space. Verified by counter, not by flag
    /// (ADR-0029 section 5).
    pub observational_enabled: bool,
    /// Neighbour slots gathered, `1..=PERCEPTION_K_MAX`.
    pub perception_k: u32,
    /// Candidate radius for the K-nearest set, metres.
    pub perception_radius_m: u32,
    /// Live signal channels, `1..=SIGNAL_CHANNELS_MAX`.
    pub signal_channels: u32,
    /// Range of a full-amplitude emission, metres; actual range scales with
    /// amplitude.
    pub signal_base_range_m: u32,
    /// Cost per whole unit of emitted amplitude per tick, milli-EU, charged
    /// with a carried remainder (D-094: a per-tick truncation on a small
    /// number lands on zero).
    pub signal_cost_milli: i64,
    /// Fraction of the committed field retained per tick, Q16, strictly
    /// below one whole: a signal is a transient local event, and a
    /// non-decaying field would be a permanent world marking, which is what
    /// artifacts are for.
    pub signal_retain_q16: u32,
    /// Reception noise half-width, Q16 of the channel range; zero draws
    /// nothing. The fidelity knob of the corruption sweep.
    pub signal_corruption_q16: u32,
}

impl SocialConfig {
    /// Documented Phase 13 defaults (disabled by default). Provisional, as
    /// every rate in this repo is; ADR-0029 records the reasoning.
    pub fn social_default() -> Self {
        Self {
            enabled: false,
            perception_enabled: true,
            signal_enabled: true,
            scramble_delivery: false,
            observational_enabled: false,
            perception_k: 4,
            perception_radius_m: 8,
            signal_channels: 4,
            signal_base_range_m: 4,
            signal_cost_milli: 20,
            signal_retain_q16: 49_152,
            signal_corruption_q16: 0,
        }
    }
}

/// Versioned Phase 11 measurement policy (`lifesim-probe-v1`).
///
/// Two measurement instruments, each separately gated, both of them
/// **observation** in the ADR-0016 sense: nothing here describes a world an
/// organism should reach, and nothing here can be read by a tick.
///
/// # Why a section of its own rather than fields on `genome2` and `plasticity`
///
/// A field appended to either of those is hashed by every world that already
/// enables them, which is the Phase 9 and Phase 11 fixtures - so a
/// measurement nobody switched on would have moved two fixtures. A section
/// appended last and hashed only when enabled cannot.
///
/// # The marker locus is not free of consequence, and the report says so
///
/// A marker locus is never expressed, but it **is** a locus, so point
/// mutation can land on it. Adding one to a genome of `n` loci therefore
/// lowers every other locus's share of the mutational input to `n/(n+1)`.
/// That is not a defect - it is what "mutates at the same rate as the genes
/// it controls for" *means* - but it makes a marker world a different
/// experiment from a marker-free one, which is exactly why the gate exists
/// and why C11.2's arms all carry the marker.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProbeConfig {
    /// Section gate. Off, the world builds no probe state at all and takes
    /// the pre-probe code paths.
    pub enabled: bool,

    /// Per-organism action counting (C11.1's measurement substrate).
    ///
    /// Gated separately from `enabled` for the reason `worldmod.patch_enabled`
    /// is: a run can want the neutral marker without paying a histogram per
    /// organism in its snapshot, and vice versa.
    pub action_census_enabled: bool,

    /// The neutral marker locus (C11.2's drift control).
    ///
    /// On, every founder haplotype carries one inert marker locus, and point
    /// mutation reaches its two alleles on exactly the terms it reaches an
    /// edge's `eta` and plastic flag.
    pub marker_locus_enabled: bool,
}

impl ProbeConfig {
    pub fn probe_default() -> Self {
        Self {
            enabled: false,
            action_census_enabled: false,
            marker_locus_enabled: false,
        }
    }
}

/// Versioned Phase 12 mutable-world policy (`lifesim-worldmod-v1`).
///
/// The seam is as narrow as Phase 9's, 10's, and 11's, and for the same
/// reason (D-072): enabling this changes exactly **what a cell's
/// traversability and carrying capacity are** - the baseline value composed
/// with a stored override instead of the baseline value alone - and adds one
/// declarative schedule in the `Commands` phase. Sensing, movement, feeding,
/// pairing, contest, and physiology all read the composed accessors and
/// cannot tell whether an override was involved.
///
/// # There is no reward here either
///
/// Nothing in this struct describes a world an organism should build. The
/// relocating patch is a property of the *environment*, drawn from
/// `(world_seed, tick)` and blind to every organism: it moves on a schedule
/// whether the population is thriving or extinct. A field that moved the
/// patch toward or away from organisms would be an authored objective
/// delivered through the terrain, which is the prohibited thing rather than
/// a refinement of it (`docs/02-scope-and-non-goals.md`, ADR-0014).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WorldModConfig {
    pub enabled: bool,

    /// Q16 fraction of a layer's cell count above which the **dense**
    /// representation is chosen for that layer in the save.
    ///
    /// Persistence policy, read by the encoder in stage 2 and by nothing in
    /// the tick: the two representations restore to identical worlds by
    /// construction, so this trades snapshot bytes against nothing else. It
    /// is versioned config rather than a magic number because
    /// `specifications/mutable-world-state.md` requires the chosen
    /// representation to be recorded so a reader never guesses, and a
    /// threshold that lived in the encoder would be a silent format
    /// parameter.
    pub dense_threshold_q16: u32,

    /// Per-layer caps on the number of stored overrides. Separate rather
    /// than one shared budget because the layers have unrelated cost models:
    /// a traversability override is one bit of meaning and a material yield
    /// is a depleting integrator, and a shared cap would let a heavily-dug
    /// world stop being able to block a cell.
    pub max_traversable_overrides: u32,
    pub max_capacity_overrides: u32,
    pub max_material_overrides: u32,

    /// The relocating resource patch: a periodic, declarative move of a
    /// high- (or equal-) capacity region.
    ///
    /// **This is Phase 11's C11.1 dependency**, which is why the mutable
    /// world half is built before the artifact half. A world whose resources
    /// never move gives lifetime learning nothing to learn: the optimal
    /// policy is fixed at birth and an evolved constant beats any learner
    /// that pays for plasticity.
    ///
    /// Gated separately from `enabled` because a world can have the section
    /// on for organism-driven modification with no environmental schedule at
    /// all, and because the phase's control arm needs the schedule *on*.
    pub patch_enabled: bool,
    /// Ticks between relocations. The patch centre is a pure function of
    /// `(world_seed, epoch)` where `epoch = tick / relocate_interval_ticks`,
    /// so the schedule needs no save section - only the override set it
    /// produces does.
    pub relocate_interval_ticks: u64,
    /// Patch half-width in cells; the footprint is `(2r+1)^2` cells before
    /// the habitable filter.
    pub patch_radius_cells: u32,
    /// Q16 multiplier applied to the carrying capacity of every habitable
    /// cell in the patch.
    ///
    /// **`Q16_ONE` is the control arm and it is not a disabled arm.** A
    /// schedule-free world is the wrong control: relocating a patch trims
    /// biomass into the loss sink every time it leaves a cell, so a
    /// treatment arm carries a lower standing biomass than a schedule-free
    /// arm for a reason that has nothing to do with what is being measured.
    /// At 1.0 the schedule runs identically - same draws, same override set,
    /// same entry count, same code path - and composes to exactly the
    /// baseline capacity, so the two arms are matched on everything but the
    /// magnitude of the move. `worldmod_capacity_loss_milli` is zero in the
    /// control and nonzero in the treatment, and a test asserts it.
    pub patch_capacity_scale_q16: u32,
}

/// Largest patch half-width a config may ask for. The footprint is
/// quadratic, so this is the bound that keeps one relocation's write list -
/// and therefore one tick's worst case - bounded by something other than the
/// map size.
pub const MAX_PATCH_RADIUS_CELLS: u32 = 64;

/// Largest capacity scale a config may ask for, matching the stored value
/// domain in `terrainmod::value_in_domain`. Two places state it because one
/// is config validation and the other is decode validation of an untrusted
/// payload; they are checked against each other in a test.
pub const MAX_CAPACITY_SCALE_Q16: u32 = 256 * Q16_ONE;

impl WorldModConfig {
    /// Documented conservative Phase 12 defaults (disabled by default).
    pub fn worldmod_default() -> Self {
        Self {
            enabled: false,
            // Half the cells of a layer: past that a dense field of i64 is
            // smaller than a sparse list of (u8, u32, i64) triples.
            dense_threshold_q16: Q16_ONE / 2,
            // 4,096 of a 65,536-cell default map, per layer. Provisional in
            // the sense every cap in this repo is: the number that will
            // replace it comes from the snapshot-size measurement Phase 12's
            // benchmark section demands, not from taste.
            max_traversable_overrides: 4_096,
            max_capacity_overrides: 4_096,
            max_material_overrides: 4_096,
            patch_enabled: false,
            // 2,000 ticks is 200 simulated seconds at the default 100 ms
            // tick: long enough that an organism can reach a patch and feed,
            // short enough that several relocations happen inside one
            // lifetime (`max_age_ticks` is 36,000). Both halves matter - a
            // schedule slower than a lifetime is a constant world with extra
            // steps, and one faster than a crossing is noise.
            relocate_interval_ticks: 2_000,
            // 15 cells at the default 4 m cell is a 124 m square patch, 961
            // cells before the habitable filter, comfortably under the cap.
            patch_radius_cells: 15,
            // 2.0. The treatment magnitude; the control sets 65_536.
            patch_capacity_scale_q16: 2 * Q16_ONE,
        }
    }
}

/// Versioned Phase 9 genome schema 2 policy.
///
/// Enabling this replaces the genome and the controller and nothing else.
/// Every other subsystem - movement, feeding, pairing, contest, physiology -
/// runs the same code, which is what makes a schema-1 world a usable
/// baseline rather than a different simulation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Genome2Config {
    pub enabled: bool,
    pub caps: crate::genome2::GenomeCaps,
    pub meiosis: crate::meiosis::MeiosisConfig,
    pub mutation: crate::structmut::MutationConfig,
}

impl Genome2Config {
    pub fn genome2_default() -> Self {
        Self {
            enabled: false,
            caps: crate::genome2::GenomeCaps::provisional(),
            meiosis: crate::meiosis::MeiosisConfig::default(),
            mutation: crate::structmut::MutationConfig::default(),
        }
    }
}

/// Versioned Phase 10 morphology policy (`lifesim-morphology-v1`,
/// `lifesim-develop-v1`).
///
/// The seam is as narrow as Phase 9's and for the same reason (D-072):
/// enabling morphology replaces exactly **how the phenotype is computed** -
/// from a grown body rather than from trait genes - and nothing else.
/// Movement, feeding, pairing, contest, and physiology all read `Phenotype`
/// and cannot tell which produced it. If morphology also rewrote the tick,
/// C10.3's "morphological change has consequence" and C10.6's "the ecology
/// is still stable" would be comparisons between two different simulations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MorphologyConfig {
    pub enabled: bool,
    pub lattice: crate::morphology::LatticeKind,
    pub caps: crate::morphology::MorphologyCaps,
    /// Controller nodes available before any neural module is grown.
    ///
    /// Non-zero on purpose. A unicell with no neural tissue still responds
    /// to its surroundings - chemotaxis needs no neurons - and a floor of
    /// zero would make every founder non-viable, since the minimal founder
    /// network has three nodes and no body starts with a brain. Neural
    /// modules buy budget *above* this floor, which is where C10.7's
    /// coupling lives.
    pub base_node_budget: u32,
}

impl MorphologyConfig {
    pub fn morphology_default() -> Self {
        Self {
            enabled: false,
            lattice: crate::morphology::LatticeKind::Square,
            caps: crate::morphology::MorphologyCaps::provisional(),
            base_node_budget: 4,
        }
    }
}

/// Versioned Phase 11 plasticity policy (`lifesim-plasticity-v2`).
///
/// The seam is as narrow as Phase 9's and Phase 10's, and for the same reason
/// (D-072): enabling plasticity changes exactly **what an edge's weight is
/// during evaluation** - the genome weight plus a per-organism learned delta
/// instead of the genome weight alone - and adds one tick phase and one
/// energy cost. Sensing, movement, feeding, pairing, contest, and physiology
/// all run the same code and cannot tell which weight produced the intent.
///
/// # There is no reward here either
///
/// Nothing in this struct describes what an organism should learn, and
/// nothing in it can. `plastic_edge_cost_milli_per_s` is a price, not a
/// signal: it is charged for every plastic edge whatever that edge does, so
/// it cannot reward an outcome. What gates learning is the organism's own
/// modulatory node (`plasticity.rs`), and a config field that measured how
/// well an organism was doing would be the prohibited thing rather than a
/// refinement of it (`docs/02-scope-and-non-goals.md`, ADR-0014).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlasticityConfig {
    pub enabled: bool,
    /// Energy charged per plastic edge per simulated second.
    ///
    /// Nonzero on purpose. Without a cost every edge becomes plastic by
    /// drift, the trait stops being informative, and C11.2's "is plasticity
    /// under selection" has no direction it could answer in. With a cost the
    /// plastic-edge count is itself under selection and "how much plasticity
    /// does this environment pay for" becomes a measurable result.
    pub plastic_edge_cost_milli_per_s: i64,
    /// Hard ceiling on how many of one organism's expressed edges may be
    /// plastic. Edges beyond it, in ascending `homology_id` order, compile
    /// as ordinary fixed edges and are counted (`plastic_over_cap`).
    ///
    /// **Provisional.** C11.7 sets this from measurement: learned state is
    /// the per-organism snapshot term this cap bounds, and the Phase 4 record
    /// already has snapshots dominated by per-organism genome arrays with a
    /// synchronous checkpoint on the tick thread. 32 is a placeholder chosen
    /// to sit at a fifth of `GenomeCaps::provisional().max_edges` (160), not
    /// a measured budget, and it must be restated once C11.7 has numbers -
    /// exactly as the genome caps were restated once by C9.8.
    pub max_plastic_edges: u32,
    /// Q16 fraction of a parent's learned delta a child inherits.
    ///
    /// **Zero, and zero is not a tuning default.** Reset at birth is the
    /// invariant that keeps Phase 13's question meaningful: if learned state
    /// were inherited, a discovery would become a heritable trait and
    /// transmission would be indistinguishable from inheritance. A nonzero
    /// value is an explicit experimental condition that **must be reported in
    /// every result derived from such a run**, never a default and never
    /// silently enabled. The `PlasticityInit` RNG stream is reserved for the
    /// policy that would implement it, so adopting one later cannot renumber
    /// a stream.
    pub lamarckian_fraction_q16: u32,
    /// Whether rule id 0 names a live learning rule instead of a no-op.
    ///
    /// **Declared and encoded here; it does not reach the kernel yet.**
    /// This is D-107's adopted option A3 - remove the dead value from the
    /// rule id space so a point mutation on target 2 can no longer be spent
    /// on nothing - carried as a runtime setting rather than a compile-time
    /// registry change, because the 2x2 it exists for must run both arms
    /// against the same seeds on one build, and a constant cannot be an arm.
    ///
    /// It is a field on the config, and therefore in the snapshot, **before**
    /// it is a behaviour, and that ordering is forced rather than chosen:
    /// `encode_config` is positional, so the byte has to be reserved by a
    /// format bump (ALIF 5) before anything can depend on it, and the 120
    /// format-4 campaign artifacts still being read for re-analysis must not
    /// stop decoding on the way. Until the kernel half lands, `validate`
    /// **refuses** `true` rather than accepting it - an accepted flag that
    /// changed nothing would produce runs that look like the treatment arm
    /// and are the control, which is the exact false null the 2x2 exists to
    /// avoid. The same refusal, for the same reason, guards
    /// `lamarckian_fraction_q16`.
    pub live_rule_zero: bool,
    /// Charge the per-edge cost only for edges whose learned state moved.
    ///
    /// **The moat half of D-107's 2x2.** Today every flagged edge pays
    /// whatever it does, and the measured consequence is that ~95 percent of
    /// the confirmatory campaign's 221,410,876 milli-EU bought rule-0
    /// no-ops - so the flagged half of the path to plasticity is deleterious
    /// while its interior is exactly neutral. A plateau with a moat.
    ///
    /// **The basis is state movement, not `StepKind::Applied`, and the
    /// difference is load-bearing.** `Applied` means "the rule form ran and
    /// the learned state was rewritten, possibly to the same value"; the only
    /// non-`Applied` kinds are the rule-0 early return and an unreachable
    /// refusal. Since `live_rule_zero` removes rule 0, pricing on `Applied`
    /// would charge every edge in exactly the arms where the chain is on -
    /// making the moat a no-op there and collapsing the 2x2's fourth arm into
    /// its second. D-107 anticipated this in writing; the campaign
    /// pre-registration records the correction.
    ///
    /// Held separate from `plastic_edge_cost_milli_per_s` rather than folded
    /// into it as a second rate, because the 2x2 needs the price *basis* to
    /// be an arm and a rate of zero is a different experiment - it removes
    /// the cost rather than repricing it.
    pub price_moved_edges_only: bool,
}

impl PlasticityConfig {
    /// Documented conservative Phase 11 defaults (disabled by default).
    pub fn plasticity_default() -> Self {
        Self {
            enabled: false,
            // 2 milli-EU per plastic edge per second against a basal cost of
            // 100: a fully capped 32-edge organism pays 64, about two thirds
            // of basal. Provisional in the same sense the cap is - the pair
            // sets the selective price of plasticity and C11.2 reads the
            // answer off it, so it is a policy value to be reported, not a
            // constant to be trusted.
            plastic_edge_cost_milli_per_s: 2,
            max_plastic_edges: 32,
            lamarckian_fraction_q16: 0,
            // False, and false is what every world that has ever run had.
            // Rule 0 has been a no-op for the whole of Phase 11, so this
            // default is not a policy preference - it is the value that makes
            // a format-4 file and a format-5 file describe the same world.
            live_rule_zero: false,
            // False, and false is what every world that has run had: the
            // shipped price is per flagged edge, and D-098's finding is about
            // that price rather than about a bug in it.
            price_moved_edges_only: false,
        }
    }
}

/// Versioned Phase 8 demography policy (`lifesim-demography-v1`).
///
/// Six independently gated mechanisms rather than one switch, because the
/// phase's acceptance criteria compare them against each other and a
/// campaign has to be able to turn exactly one on. All disabled is
/// behaviorally identical to Phase 7.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PhysiologyConfig {
    pub enabled: bool,

    /// Basal cost scales as body mass to `basal_exponent_quarters / 4`.
    /// The default 3 is Kleiber's 0.75. Quarters rather than a Q16
    /// fraction because a quarter-power is exactly two integer square
    /// roots and needs no transcendental (see `physiology.rs`).
    pub allometry_enabled: bool,
    pub basal_exponent_quarters: u32,

    /// Thermal preference becomes live against the Phase 6 temperature
    /// field. Inert without climate, which is a documented precondition
    /// rather than an error: a world with no temperature field has nothing
    /// for the gene to be preferred against.
    pub thermoregulation_enabled: bool,
    /// Temperatures that thermal preference 0 and 1000 map to,
    /// milli-degrees.
    pub thermal_pref_low_milli: i32,
    pub thermal_pref_high_milli: i32,
    /// Deviation tolerated for free, milli-degrees.
    pub thermal_neutral_band_milli: i32,
    /// Milli-EU per second per milli-degree of excess deviation.
    pub thermal_cost_milli_per_s_per_degree: i64,

    /// Age-dependent hazard replacing the hard `max_age_ticks` cutoff.
    pub senescence_enabled: bool,
    pub senescence_onset_ticks: u64,
    /// Age scale over which the hazard reaches its base rate.
    pub senescence_scale_ticks: u64,
    /// Weibull shape; 1..=4.
    pub senescence_power: u32,
    pub senescence_hazard_q16_per_s: u32,

    /// Non-food hazard, the mechanism that lets a population sit below its
    /// food ceiling. Zero is off.
    pub extrinsic_hazard_q16_per_s: u32,

    /// Hazard multiplier applied before maturity. `Q16_ONE` is no penalty.
    pub juvenile_hazard_multiplier_q16: u32,

    /// Ontogeny (Phase 14, ADR-0030): the developed body is revealed one
    /// module at a time in canonical BFS order from the origin module,
    /// each activation paid through the ledger, and every juvenile
    /// constraint is a consequence of the partially grown body's own
    /// derived attributes. Requires morphology - there is no body to grow
    /// otherwise. Founders start fully grown (they seed the population,
    /// as in every phase before this one); children start at
    /// `birth_modules_min`.
    pub ontogeny_enabled: bool,
    /// Modules already grown at birth. At least 1: a zero-module organism
    /// is the `Empty` viability failure, not a newborn.
    pub birth_modules_min: u32,
    /// Milli-EU paid per milli-unit of module mass to activate it.
    pub growth_cost_milli_per_mass_milli: i64,
    /// Ledger flow cap into growth, milli-EU per second, so growth is a
    /// metered expense over juvenile life rather than a lump sum at birth.
    pub growth_rate_milli_per_s: i64,

    /// Mate choice (Phase 14, ADR-0030 decision 2): pairing selects the
    /// candidate with the highest evolved-preference score over its
    /// perceived cue values rather than the nearest, with the existing
    /// `(distance^2, id)` key as the tie-break - so an all-neutral
    /// preference reproduces proximity pairing exactly. Requires phase2
    /// (there is no pairing otherwise) and genome2 (the preference band is
    /// schema-2 trait loci).
    pub mate_choice_enabled: bool,
    /// The P-scramble arm: candidate cue vectors are permuted among the
    /// candidates actually under consideration before scoring, preserving
    /// eligibility, distance and cost while destroying which cues belong
    /// to whom. Checked, never merely configured: every permuted choice
    /// counts in `scrambled_choices_total`.
    pub mate_choice_scramble: bool,
}

impl PhysiologyConfig {
    /// Documented conservative Phase 8 defaults (disabled by default).
    pub fn physiology_default() -> Self {
        Self {
            enabled: false,
            allometry_enabled: true,
            basal_exponent_quarters: 3, // 0.75, Kleiber
            thermoregulation_enabled: true,
            thermal_pref_low_milli: 0,
            thermal_pref_high_milli: 40_000,
            thermal_neutral_band_milli: 6_000,
            thermal_cost_milli_per_s_per_degree: 4,
            senescence_enabled: true,
            senescence_onset_ticks: 6_000,
            senescence_scale_ticks: 12_000,
            senescence_power: 2,
            senescence_hazard_q16_per_s: 655, // 0.01 per second at scale
            extrinsic_hazard_q16_per_s: 13,   // ~0.0002 per second
            juvenile_hazard_multiplier_q16: 2 * Q16_ONE,
            ontogeny_enabled: false,
            birth_modules_min: 1,
            growth_cost_milli_per_mass_milli: 100,
            growth_rate_milli_per_s: 50,
            mate_choice_enabled: false,
            mate_choice_scramble: false,
        }
    }
}

/// Versioned Phase 15 chemistry-field policy (`lifesim-chemistry-v1`,
/// ADR-0031). The field regime's chemistry half: four abstract substrates
/// on the raster, a closed abiotic mass cycle, and the abiogenesis rate
/// function. Everything fixed point; every rate is per FIELD step, and
/// `field_steps_per_tick` of those run per world tick.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChemistryConfig {
    pub enabled: bool,
    /// Field steps per world tick; at least 1. Versioned config, hashed -
    /// an abstraction, not a timescale claim (ADR-0020).
    pub field_steps_per_tick: u32,
    /// Per-neighbour outflow rate, Q16 per field step. Four neighbours,
    /// so validation bounds it at a quarter of one to keep outflow below
    /// the cell's own content.
    pub diffusion_q16: u32,
    /// S_PRIMORDIAL -> S_MONOMER abiotic rate, Q16 per field step.
    pub reaction_monomer_q16: u32,
    /// S_WASTE -> S_PRIMORDIAL recycling rate, Q16 per field step.
    pub reaction_recycle_q16: u32,
    /// Abiotic S_PRIMORDIAL input, milli per cell-average per field step;
    /// counted in the chemistry ledger as production, so C15.1's identity
    /// stays exact. The scaffold redistributes this same total.
    pub production_milli_per_step: i64,
    /// ADR-0018 scaffold: 0 radius = uniform production (the N arm);
    /// otherwise production concentrates into patches on a regular grid
    /// of centres (spacing four radii), total held constant.
    pub scaffold_patch_radius_cells: u32,
    /// Production multiplier inside patches, Q16; outside cells share the
    /// remainder so the total is unchanged. Q16_ONE means no contrast.
    pub scaffold_patch_contrast_q16: u32,
    /// The abiogenesis rate function's gate and parameters.
    pub abiogenesis_enabled: bool,
    /// Q16 weights over the rate inputs (primordial, monomer, polymer
    /// surface term), applied to milli concentrations; the sum, capped,
    /// becomes the per-cell firing probability in Q16 per field step.
    pub abiogenesis_weight_primordial_q16: u32,
    pub abiogenesis_weight_monomer_q16: u32,
    pub abiogenesis_weight_polymer_q16: u32,
    /// Probability cap, Q16 per field step.
    pub abiogenesis_cap_q16: u32,
    /// Density seeded into the founder class on a firing, milli; the same
    /// mass is debited from S_PRIMORDIAL, so genesis conserves. A firing
    /// with less S_PRIMORDIAL present than this seeds nothing.
    pub abiogenesis_seed_milli: i64,

    /// The microbial half of the field regime (ADR-0031): per-cell
    /// densities over the genotype-class registry. Requires the chemistry
    /// half - classes eat substrates.
    pub microbial_enabled: bool,
    /// Class-axis sizes. Substrate preference is fixed at two (primordial,
    /// monomer); these two are config-swept per the plan. Class count =
    /// 2 * replication_axis * aggregation_axis, bounded by validation.
    pub replication_axis: u32,
    pub aggregation_axis: u32,
    /// Growth rate at the lowest and highest replication-axis position,
    /// Q16 per field step; intermediate positions interpolate linearly.
    pub growth_rate_low_q16: u32,
    pub growth_rate_high_q16: u32,
    /// Fraction of consumed substrate that becomes density; the remainder
    /// is metabolic loss deposited as S_WASTE in the same step.
    pub growth_yield_q16: u32,
    /// Death rate, Q16 per field step; the died mass splits between
    /// S_WASTE and S_PRIMORDIAL by `death_waste_fraction_q16`.
    pub death_q16: u32,
    pub death_waste_fraction_q16: u32,
    /// Mutation flow to each single-axis-step neighbour class, Q16 per
    /// field step.
    pub mutation_q16: u32,
    /// Coupling v1 (ADR-0031): fraction of each organism's basal metabolic
    /// payment deposited as S_WASTE in its cell, per tick, through the
    /// field ledger's `deposited` term. Zero = no coupling.
    pub excretion_fraction_q16: u32,
    /// Coupling v1: fraction of the energy removed at death deposited as
    /// S_PRIMORDIAL in the death cell. Zero = no coupling.
    pub remains_fraction_q16: u32,
}

impl ChemistryConfig {
    pub fn chemistry_default() -> Self {
        Self {
            enabled: false,
            field_steps_per_tick: 1,
            diffusion_q16: 3_277, // 0.05 per neighbour per step
            reaction_monomer_q16: 655, // 0.01
            reaction_recycle_q16: 1_311, // 0.02
            production_milli_per_step: 2,
            scaffold_patch_radius_cells: 0,
            scaffold_patch_contrast_q16: Q16_ONE,
            abiogenesis_enabled: false,
            abiogenesis_weight_primordial_q16: 66,
            abiogenesis_weight_monomer_q16: 131,
            abiogenesis_weight_polymer_q16: 262,
            abiogenesis_cap_q16: 655, // 0.01 per cell per step at the cap
            abiogenesis_seed_milli: 1_000,
            microbial_enabled: false,
            replication_axis: 2,
            aggregation_axis: 2,
            growth_rate_low_q16: 1_311,  // 0.02 per step
            growth_rate_high_q16: 3_932, // 0.06
            growth_yield_q16: 39_322,    // 0.6 of consumed substrate
            death_q16: 655,              // 0.01
            death_waste_fraction_q16: 32_768, // half to waste, half recycled
            mutation_q16: 66,            // 0.001 per neighbour
            excretion_fraction_q16: 0,
            remains_fraction_q16: 0,
        }
    }
}

/// Versioned Phase 16 transition policy (`lifesim-transition-v1`,
/// ADR-0032): when microbial density becomes individual organisms. A
/// physical condition with a memory, not a detector of anything: a slot
/// that has held at least `density_floor_milli` for `persistence_checks`
/// consecutive checks, in a class at or above `aggregation_step_min`, in a
/// cell an organism can stand on, converts `organism_energy_milli` of
/// density per organism into one-module organisms. Nothing here reads a
/// module count or grants anything for crossing anything.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TransitionConfig {
    pub enabled: bool,
    /// The trigger is evaluated every this many world ticks; at least 1.
    pub check_interval_ticks: u64,
    /// A slot must hold at least this density (milli) at a check for the
    /// check to count toward persistence. At least `organism_energy_milli`.
    pub density_floor_milli: i64,
    /// Consecutive checks at or above the floor before a slot triggers.
    pub persistence_checks: u32,
    /// Only classes at or above this aggregation-axis position trigger;
    /// below `chemistry.aggregation_axis`.
    pub aggregation_step_min: u32,
    /// Energy credited per materialized organism, debited 1:1 from the
    /// slot's density. Bounded above by the unicell body's energy capacity
    /// at world construction, where the body exists.
    pub organism_energy_milli: i64,
    /// Organisms one `(cell, class)` trigger may produce; at least 1.
    pub max_organisms_per_event: u32,
    /// Organisms admitted per world tick across all triggers; the rest
    /// defer whole to the next check, counted. At least 1.
    pub max_materializations_per_tick: u32,
}

impl TransitionConfig {
    pub fn transition_default() -> Self {
        Self {
            enabled: false,
            check_interval_ticks: 100,
            // Twenty seedings: reachable by growth (the Phase 15 campaign's
            // standing densities are ~53x seeded), unreachable by a lone
            // abiogenesis firing.
            density_floor_milli: 20_000,
            persistence_checks: 5,
            // The top step of the default two-position aggregation axis.
            aggregation_step_min: 1,
            // `offspring_energy_milli`'s value: a materialized organism
            // starts where a born one does.
            organism_energy_milli: 4_000,
            max_organisms_per_event: 4,
            max_materializations_per_tick: 64,
        }
    }
}

/// Versioned Phase 7 contest policy (`contest-behavior-v1`). Every value is
/// experimental policy, hashed only when `enabled` is true.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContestConfig {
    pub enabled: bool,
    /// Health of a body-scale-1.0 organism, milli-units.
    pub base_health_milli: i64,
    /// Damage one landed attack does before body-scale scaling and variance.
    /// Zero is condition C of the phase design: the action fires and costs
    /// energy without consequence.
    pub damage_base_milli: i64,
    /// Half-width of the damage draw, Q16.
    pub damage_variance_q16: u32,
    /// Energy an attack costs its attacker, whether or not it lands.
    pub attack_cost_milli: i64,
    /// Maximum distance at which an attack can land, meters.
    pub attack_range_m: u32,
    /// Controller output above which the attack intent fires, Q16 signed.
    pub attack_threshold_q16: i32,
    /// Ticks an organism must wait between attacks.
    pub attack_cooldown_ticks: u64,

    /// Health restored per second while healing.
    pub heal_milli_per_s: i64,
    /// Energy spent per milli-unit of health restored, Q16.
    pub heal_energy_cost_q16: u32,
    /// Healing only happens above this energy fraction, Q16.
    pub heal_energy_floor_q16: u32,
    /// Q16 fraction of accumulated recent damage that decays each second.
    pub damage_decay_q16_per_s: u32,

    /// Q16 fraction of a dead organism's energy that becomes a carcass.
    pub carcass_energy_q16: u32,
    /// Q16 fraction of a carcass's energy that decays each second.
    pub carcass_decay_q16_per_s: u32,
    /// Maximum distance at which a carcass can be eaten, meters.
    pub carcass_reach_m: u32,
    /// Maximum carcasses retained; the oldest are dropped beyond it, with
    /// the loss ledgered as decay rather than silently discarded.
    pub max_carcasses: u32,
    /// Extra biomass a feeding organism removes from its own cell, sharpening
    /// local resource contention so a patch is worth defending.
    pub local_depletion_milli: i64,
}

impl ContestConfig {
    /// Documented conservative Phase 7 defaults (disabled by default).
    pub fn contest_default() -> Self {
        Self {
            enabled: false,
            base_health_milli: 10_000,
            damage_base_milli: 1_200,
            damage_variance_q16: 16_384, // +/- 0.25
            attack_cost_milli: 120,
            attack_range_m: 3,
            attack_threshold_q16: 32_768, // 0.5
            attack_cooldown_ticks: 10,
            heal_milli_per_s: 60,
            heal_energy_cost_q16: 131_072,  // 2.0 energy per health
            heal_energy_floor_q16: 32_768,  // heal only above half energy
            damage_decay_q16_per_s: 6_554,  // 0.10 per second
            carcass_energy_q16: 45_875,     // 0.70 of remaining energy
            carcass_decay_q16_per_s: 3_277, // 0.05 per second
            carcass_reach_m: 2,
            max_carcasses: 4_096,
            local_depletion_milli: 0,
        }
    }
}

/// Versioned Phase 6 origin policy (`lifesim-origin-v1`).
///
/// The `random` defaults below are the constants Phase 1 and Phase 2 had
/// hard-coded, lifted into config without changing them: founder traits land
/// in `0.25 + u * 0.5` and founder neural weights in `+/- 0.5`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OriginConfig {
    pub mode: crate::origin::OriginMode,
    pub trait_low_q16: u32,
    pub trait_span_q16: u32,
    pub neural_span_q16: u32,
    /// Number of separated founder groups. `1` is the Phase 1/2 behavior.
    pub deme_count: u32,
    pub deme_radius_m: u32,
    pub deme_min_separation_m: u32,
    /// Half-width of a founder's draw around its deme's trait centre. Small
    /// relative to `trait_span_q16`, which is what makes within-deme genetic
    /// distance smaller than between-deme distance.
    pub deme_trait_spread_q16: u32,
    pub archetype_count: u32,
    pub archetypes: [crate::origin::Archetype; crate::origin::MAX_ARCHETYPES],
}

impl OriginConfig {
    /// Defaults reproducing the Phase 1/2 founder behavior exactly.
    pub fn origin_default() -> Self {
        Self {
            mode: crate::origin::OriginMode::Random,
            trait_low_q16: 16_384,  // 0.25
            trait_span_q16: 32_768, // 0.5
            neural_span_q16: Q16_ONE,
            deme_count: 1,
            deme_radius_m: 128,
            deme_min_separation_m: 192,
            deme_trait_spread_q16: 6_554, // 0.10
            archetype_count: 0,
            archetypes: [crate::origin::Archetype::neutral(0); crate::origin::MAX_ARCHETYPES],
        }
    }
}

/// Which world generator produced (and will regenerate) a world's terrain.
///
/// The selected version is folded into the config hash in place of a
/// constant, so a v1 world keeps its terrain checksum and both fixtures
/// **forever**: adding v2 cannot change what v1 hashes to. v1 stays in the
/// build permanently and v1 worlds never see v2.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum WorldgenVersion {
    #[default]
    V1,
    /// Adds moisture, temperature, and biome fields (Phase 6).
    V2,
}

impl WorldgenVersion {
    pub fn tag(self) -> &'static str {
        match self {
            WorldgenVersion::V1 => "lifesim-worldgen-v1",
            WorldgenVersion::V2 => "lifesim-worldgen-v2",
        }
    }
}

/// Versioned Phase 6 climate, biome, and generator policy. Every value is
/// experimental policy, hashed only when `enabled` is true.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClimateConfig {
    pub enabled: bool,
    /// Generator selection. `enabled` implies v2; a disabled section leaves
    /// this at v1 and validation enforces the pairing.
    pub worldgen_version: WorldgenVersion,

    // Static base temperature, milli-degrees.
    pub base_temperature_milli: i32,
    /// Temperature drop across the full Q16 elevation range.
    pub lapse_milli_per_full_elevation: i32,
    /// Pole-to-equator temperature difference.
    pub latitude_amplitude_milli: i32,

    // Stateless time terms.
    pub season_period_ticks: u64,
    pub season_amplitude_milli: i32,
    /// Three incommensurate drift periods, all far longer than the season.
    pub drift_period_ticks: [u64; 3],
    pub drift_amplitude_milli: [i32; 3],

    pub temperature_min_milli: i32,
    pub temperature_max_milli: i32,

    // Moisture, milli-units. The field is conserved: these values set the
    // initial distribution and how it redistributes, never a source or sink.
    pub initial_moisture_milli: i64,
    pub coastal_moisture_bonus_milli: i64,
    pub moisture_max_milli: i64,
    /// Absolute ceiling any cell may reach; validation, not a clamp.
    pub moisture_ceiling_milli: i64,
    /// How much of the initial moisture blend comes from sea proximity
    /// rather than low ground, Q16. Zero makes moisture a pure function of
    /// elevation, which collapses the biome map onto elevation bands.
    pub sea_proximity_weight_q16: u32,
    /// Fraction of a cell's moisture that leaves it each step, Q16.
    pub moisture_diffusion_q16: u32,
    /// Extra share a downhill neighbour receives (drainage).
    pub moisture_drain_weight: u32,

    // Biome thresholds (`lifesim-biome-v1`).
    pub highland_elevation_q16: u32,
    pub wetland_moisture_milli: i64,
    pub arid_moisture_milli: i64,
    pub forest_moisture_milli: i64,
    pub forest_min_temperature_milli: i32,
    /// Per-biome carrying-capacity multiplier, Q16, indexed by biome ID.
    pub biome_capacity_q16: [u32; crate::climate::BIOME_COUNT],
    /// Ticks between reclassifications. Climate moves on timescales far
    /// longer than a tick, so reclassifying every tick would be pure cost;
    /// the cadence is versioned policy like any other formula constant.
    pub reclassify_interval_ticks: u64,
}

impl ClimateConfig {
    /// Documented conservative Phase 6 defaults (disabled by default).
    pub fn climate_default() -> Self {
        Self {
            enabled: false,
            worldgen_version: WorldgenVersion::V1,
            base_temperature_milli: 22_000,
            lapse_milli_per_full_elevation: 30_000,
            latitude_amplitude_milli: 28_000,
            season_period_ticks: 36_000,
            season_amplitude_milli: 6_000,
            // Pairwise-coprime primes: the sum is quasi-periodic rather than
            // repeating on any short cycle.
            drift_period_ticks: [1_000_003, 410_009, 173_021],
            drift_amplitude_milli: [7_000, 3_000, 1_500],
            temperature_min_milli: -60_000,
            temperature_max_milli: 60_000,
            initial_moisture_milli: 120_000,
            coastal_moisture_bonus_milli: 40_000,
            moisture_max_milli: 200_000,
            moisture_ceiling_milli: 4_000_000,
            sea_proximity_weight_q16: 39_322, // 0.60 sea proximity, 0.40 relief
            moisture_diffusion_q16: 3_277,    // 0.05 of a cell per step
            moisture_drain_weight: 2,
            // Calibrated against measured field distributions rather than
            // guessed: across seven seeds at 96x96, land elevation spans
            // roughly p0 17,000 to p100 35,500-53,300, and inland moisture
            // spans p0 18,500-37,000 to p100 111,500-115,600. Each threshold
            // sits inside the range every seed reaches, so no biome is
            // unreachable by construction.
            highland_elevation_q16: 30_000,
            wetland_moisture_milli: 105_000,
            arid_moisture_milli: 55_000,
            forest_moisture_milli: 90_000,
            forest_min_temperature_milli: 4_000,
            // Water contributes nothing; wetland and forest are the most
            // productive; arid and highland the least.
            biome_capacity_q16: [
                0,      // water
                72_000, // coast
                26_214, // highland  (0.40)
                98_304, // wetland   (1.50)
                19_661, // arid      (0.30)
                85_197, // forest    (1.30)
                65_536, // grassland (1.00)
            ],
            reclassify_interval_ticks: 100,
        }
    }
}

/// Versioned Phase 2 policy: inherited controllers, paired-parent
/// reproduction, and offline similarity analysis. All values are
/// experimental simulation policy, hashed only when `enabled` is true.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Phase2Config {
    pub enabled: bool,
    /// Per-gene variation probability, Q16.
    pub variation_probability_q16: u32,
    /// Maximum trait-gene delta, Q16 gene units.
    pub variation_trait_sigma_q16: u32,
    /// Maximum neural-gene delta as a Q16 fraction of the weight limit.
    pub variation_neural_sigma_q16: u32,
    /// Maximum pairing distance in meters.
    pub pairing_range_m: u32,
    /// Maximum normalized trait distance for a compatible pair, Q16.
    pub compatibility_threshold_q16: u32,
    /// Energy readiness gate for pairing intent, milli-EU.
    pub pairing_energy_threshold_milli: i64,
    /// Non-transferred energy each parent spends per accepted pairing.
    pub pairing_overhead_milli: i64,
    /// Controller output thresholds, Q16 signed (output range [-1, 1]).
    pub eat_threshold_q16: i32,
    pub mate_threshold_q16: i32,
    pub rest_threshold_q16: i32,
    /// Maximum heading change per tick, BAM units (65536 per full turn).
    pub max_turn_per_tick_bam: u32,
    /// Offline similarity analysis: cluster threshold and sampling bound.
    pub cluster_threshold_q16: u32,
    pub cluster_sample_max: u32,
    /// Q16 weight of controller-parameter distance in similarity analysis
    /// (0 omits controller parameters, the documented starting policy).
    pub cluster_neural_weight_q16: u32,
}

impl Phase2Config {
    /// Documented conservative Phase 2 defaults (disabled by default).
    pub fn phase2_default() -> Self {
        Self {
            enabled: false,
            variation_probability_q16: 1_311,  // 0.02
            variation_trait_sigma_q16: 3_277,  // 0.05 gene units
            variation_neural_sigma_q16: 3_277, // 0.05 * 8.0 = 0.4 weight units
            pairing_range_m: 4,
            compatibility_threshold_q16: 32_768, // 0.5 normalized distance
            pairing_energy_threshold_milli: 7_000,
            pairing_overhead_milli: 500,
            // Permissive defaults keep founder populations viable while the
            // gates stay evolvable: an output below the threshold still
            // suppresses the action.
            eat_threshold_q16: -32_768,    // eat unless output < -0.5
            mate_threshold_q16: -16_384,   // mate intent unless output < -0.25
            rest_threshold_q16: 32_768,    // 0.5
            max_turn_per_tick_bam: 2_048,  // 1/32 turn per tick
            cluster_threshold_q16: 13_107, // 0.2
            cluster_sample_max: 2_048,
            cluster_neural_weight_q16: 0,
        }
    }
}

impl SimConfig {
    /// Documented Phase 1 defaults: a 1,024 m square continent world tuned
    /// for the 500-organism baseline. These are experimental values.
    pub fn phase1_default(world_seed: u64) -> Self {
        Self {
            world_seed,
            cells_x: 256,
            cells_y: 256,
            cell_size_m: 4,
            initial_organisms: 500,
            max_entities: 5_000,
            dt_ms: 100,
            growth_rate_q16_per_s: 3_277, // 0.05 per second
            cell_capacity_milli: 30_000,
            initial_biomass_q16: 32_768, // 0.5
            energy_max_milli: 12_000,
            initial_energy_milli: 8_000,
            basal_cost_milli_per_s: 100,
            move_cost_milli_per_s: 500,
            intake_rate_milli_per_s: 2_000,
            assimilation_q16: 45_875, // 0.7
            speed_mps_q16: 131_072,   // 2.0 m/s
            crowding_radius_m: 6,
            crowding_threshold: 4,
            crowding_cost_milli_per_s: 200,
            maturity_age_ticks: 600,
            max_age_ticks: 36_000,
            reproduction_enabled: true,
            repro_threshold_milli: 10_000,
            offspring_energy_milli: 4_000,
            repro_overhead_milli: 1_000,
            repro_cooldown_ticks: 300,
            land_threshold_q16: 17_000,
            min_land_fraction_q16: 6_554,  // 0.10
            max_land_fraction_q16: 58_982, // 0.90
            phase2: Phase2Config::phase2_default(),
            climate: ClimateConfig::climate_default(),
            origin: OriginConfig::origin_default(),
            contest: ContestConfig::contest_default(),
            physiology: PhysiologyConfig::physiology_default(),
            genome2: Genome2Config::genome2_default(),
            morphology: MorphologyConfig::morphology_default(),
            plasticity: PlasticityConfig::plasticity_default(),
            worldmod: WorldModConfig::worldmod_default(),
            probe: ProbeConfig::probe_default(),
            artifact: ArtifactConfig::artifact_default(),
            social: SocialConfig::social_default(),
            chemistry: ChemistryConfig::chemistry_default(),
            transition: TransitionConfig::transition_default(),
        }
    }

    /// Phase 11 defaults: the schema-2 world with plasticity live.
    ///
    /// Both flags, because either alone is a condition rather than the
    /// treatment. `plasticity.enabled` without
    /// `genome2.mutation.plasticity_enabled` is a world where plasticity runs
    /// but no organism's plasticity genes can ever change - which is exactly
    /// condition B, not condition A - and the mutation flag without the
    /// section is genes that mutate and never act.
    pub fn phase11_default(world_seed: u64) -> Self {
        let mut config = Self::phase2_default(world_seed);
        config.genome2.enabled = true;
        config.genome2.mutation.plasticity_enabled = true;
        config.plasticity.enabled = true;
        config
    }

    /// Phase 12 mutable-world defaults: the Phase 11 world with the terrain
    /// modification section and the relocating resource patch live.
    ///
    /// Built on `phase11_default` rather than on `phase2_default` because the
    /// patch exists to serve C11.1: a world whose resources move is what
    /// makes lifetime learning able to beat an evolved constant, and a
    /// relocating patch in a world with no learner is a schedule with nothing
    /// to measure. Nothing stops a Phase 1 world from enabling the section -
    /// the validation above requires no other section - and the kernel tests
    /// use exactly that, because it isolates the terrain arithmetic from
    /// every genome-driven source of variance.
    pub fn phase12_default(world_seed: u64) -> Self {
        let mut config = Self::phase11_default(world_seed);
        config.worldmod.enabled = true;
        config.worldmod.patch_enabled = true;
        config
    }

    /// Phase 7 defaults: the Phase 2 world with contest enabled.
    pub fn phase7_default(world_seed: u64) -> Self {
        let mut config = Self::phase2_default(world_seed);
        config.contest.enabled = true;
        config
    }

    /// Phase 6 defaults: the Phase 2 world with climate, biomes, and the
    /// `lifesim-worldgen-v2` generator enabled.
    pub fn phase6_default(world_seed: u64) -> Self {
        let mut config = Self::phase2_default(world_seed);
        config.climate.enabled = true;
        config.climate.worldgen_version = WorldgenVersion::V2;
        config
    }

    /// Phase 2 defaults: the Phase 1 world with inherited controllers and
    /// paired-parent reproduction enabled (`phase2-behavior-v1`).
    pub fn phase2_default(world_seed: u64) -> Self {
        let mut config = Self::phase1_default(world_seed);
        config.phase2.enabled = true;
        config
    }

    /// Validate ranges and cross-field consistency. All construction paths
    /// must call this before building a world.
    pub fn validate(&self) -> Result<(), ConfigError> {
        let fraction_fields = [
            ("initial_biomass_q16", self.initial_biomass_q16),
            ("assimilation_q16", self.assimilation_q16),
            ("land_threshold_q16", self.land_threshold_q16),
            ("min_land_fraction_q16", self.min_land_fraction_q16),
            ("max_land_fraction_q16", self.max_land_fraction_q16),
        ];
        for (name, value) in fraction_fields {
            if value > Q16_ONE {
                return Err(ConfigError::FractionOutOfRange(name, value));
            }
        }
        if self.cells_x < 8
            || self.cells_y < 8
            || self.cells_x > MAX_CELLS_PER_AXIS
            || self.cells_y > MAX_CELLS_PER_AXIS
        {
            return Err(ConfigError::WorldDimensions(self.cells_x, self.cells_y));
        }
        if self.cell_size_m == 0 || self.cell_size_m > 64 {
            return Err(ConfigError::CellSize(self.cell_size_m));
        }
        // World extent must fit i32 fixed-point.
        let extent_fp = i64::from(self.cells_x.max(self.cells_y))
            * i64::from(self.cell_size_m)
            * i64::from(crate::FP_PER_METER);
        if extent_fp > i64::from(i32::MAX) {
            return Err(ConfigError::WorldDimensions(self.cells_x, self.cells_y));
        }
        if self.max_entities == 0 || self.max_entities > ABSOLUTE_MAX_ENTITIES {
            return Err(ConfigError::MaxEntities(self.max_entities));
        }
        // Phase 16 (ADR-0032): a scratch world begins with no organisms
        // and is the only origin that may; every other mode keeps the
        // Phase 1 rule exactly.
        let scratch = self.origin.mode == crate::origin::OriginMode::Scratch;
        if (self.initial_organisms == 0) != scratch || self.initial_organisms > self.max_entities
        {
            return Err(ConfigError::InitialOrganisms(self.initial_organisms));
        }
        if self.dt_ms == 0 || self.dt_ms > 10_000 {
            return Err(ConfigError::TickLength(self.dt_ms));
        }
        if self.assimilation_q16 == 0 {
            return Err(ConfigError::FractionOutOfRange(
                "assimilation_q16",
                self.assimilation_q16,
            ));
        }
        if self.cell_capacity_milli <= 0 {
            return Err(ConfigError::NonPositive("cell_capacity_milli"));
        }
        if self.energy_max_milli <= 0 {
            return Err(ConfigError::NonPositive("energy_max_milli"));
        }
        if self.initial_energy_milli <= 0 || self.initial_energy_milli > self.energy_max_milli {
            return Err(ConfigError::InitialEnergy(self.initial_energy_milli));
        }
        let non_negative = [
            ("basal_cost_milli_per_s", self.basal_cost_milli_per_s),
            ("move_cost_milli_per_s", self.move_cost_milli_per_s),
            ("intake_rate_milli_per_s", self.intake_rate_milli_per_s),
            ("crowding_cost_milli_per_s", self.crowding_cost_milli_per_s),
        ];
        for (name, value) in non_negative {
            if value < 0 {
                return Err(ConfigError::Negative(name));
            }
        }
        if self.speed_mps_q16 == 0 || self.speed_mps_q16 > 20 * Q16_ONE {
            return Err(ConfigError::Speed(self.speed_mps_q16));
        }
        if self.crowding_radius_m == 0 || self.crowding_radius_m > 64 {
            return Err(ConfigError::CrowdingRadius(self.crowding_radius_m));
        }
        if self.max_age_ticks == 0 || self.max_age_ticks <= self.maturity_age_ticks {
            return Err(ConfigError::AgePolicy {
                maturity: self.maturity_age_ticks,
                max_age: self.max_age_ticks,
            });
        }
        if self.reproduction_enabled {
            if self.offspring_energy_milli <= 0 || self.repro_overhead_milli < 0 {
                return Err(ConfigError::NonPositive("offspring_energy_milli"));
            }
            let total_cost = self
                .offspring_energy_milli
                .saturating_add(self.repro_overhead_milli);
            if self.repro_threshold_milli < total_cost {
                return Err(ConfigError::ReproductionEnergy {
                    threshold: self.repro_threshold_milli,
                    total_cost,
                });
            }
            if self.repro_threshold_milli > self.energy_max_milli {
                return Err(ConfigError::ReproductionEnergy {
                    threshold: self.repro_threshold_milli,
                    total_cost,
                });
            }
        }
        if self.min_land_fraction_q16 >= self.max_land_fraction_q16 {
            return Err(ConfigError::LandFractionBounds {
                min: self.min_land_fraction_q16,
                max: self.max_land_fraction_q16,
            });
        }
        if self.phase2.enabled {
            self.validate_phase2()?;
        }
        self.validate_climate()?;
        self.validate_origin()?;
        self.validate_contest()?;
        self.validate_subsystems()?;
        Ok(())
    }

    /// Validation for the subsystems that are not contest.
    ///
    /// **These checks used to live inside `validate_contest`**, which
    /// early-returns when the contest section is disabled - so every
    /// genome2 and physiology cap check was silently skipped in any world
    /// without contest, which is most of them. The blocks were appended to
    /// the wrong function over successive phases and nothing failed,
    /// because a skipped validation looks exactly like a passing one.
    fn validate_subsystems(&self) -> Result<(), ConfigError> {
        if self.genome2.enabled {
            if !self.phase2.enabled {
                return Err(ConfigError::PhysiologyRange("genome2 requires phase2", 0));
            }
            let caps = &self.genome2.caps;
            if caps.max_chromosomes == 0
                || caps.max_loci_per_chromosome == 0
                || caps.max_nodes == 0
                || caps.max_edges_per_node == 0
                || caps.max_genome_bytes == 0
                || caps.min_nodes == 0
            {
                return Err(ConfigError::PhysiologyRange("genome2 cap is zero", 0));
            }
        }
        if self.morphology.enabled {
            // Morphology is expressed from schema-2 loci, so it cannot run
            // without the genome that carries them. Refused rather than
            // silently ignored: a config that asks for bodies and gets none
            // would report morphology metrics of zero and read as a null.
            if !self.genome2.enabled {
                return Err(ConfigError::PhysiologyRange(
                    "morphology requires genome2",
                    0,
                ));
            }
            let caps = &self.morphology.caps;
            if caps.max_modules == 0 || caps.lattice_radius <= 0 || caps.max_growth_steps == 0 {
                return Err(ConfigError::PhysiologyRange("morphology cap is zero", 0));
            }
        }
        // Phase 11. **Inside `validate_subsystems`, never appended to
        // `validate_contest`** - that function early-returns on a disabled
        // contest section, and D-084 records what appending checks there
        // cost: three phases of cap validation that never ran in any world
        // without contest, which is most of them.
        let plasticity = &self.plasticity;
        if plasticity.enabled {
            // Plastic edges live on schema-2 edge loci and are gated by
            // schema-2 modulatory nodes, so there is nothing for the section
            // to act on without genome2. Refused rather than silently
            // ignored, for the reason morphology is: a config that asks for
            // learning and gets none would report plasticity metrics of zero
            // and read as C11.1's null.
            if !self.genome2.enabled {
                return Err(ConfigError::PhysiologyRange(
                    "plasticity requires genome2",
                    0,
                ));
            }
            if plasticity.plastic_edge_cost_milli_per_s < 0 {
                return Err(ConfigError::Negative("plastic_edge_cost_milli_per_s"));
            }
            if plasticity.max_plastic_edges == 0 {
                return Err(ConfigError::PhysiologyRange("max_plastic_edges is zero", 0));
            }
            // A plastic-edge cap above the structural edge cap could never
            // bind, which would make the field look enforced while being
            // decorative - and C11.7 is going to set this number from a
            // measurement of what it actually bounds.
            if plasticity.max_plastic_edges > self.genome2.caps.max_edges {
                return Err(ConfigError::PhysiologyRange(
                    "max_plastic_edges exceeds genome2.caps.max_edges",
                    i64::from(plasticity.max_plastic_edges),
                ));
            }
            if plasticity.lamarckian_fraction_q16 > Q16_ONE {
                return Err(ConfigError::FractionOutOfRange(
                    "lamarckian_fraction_q16",
                    plasticity.lamarckian_fraction_q16,
                ));
            }
            // Nonzero Lamarckian inheritance is a declared experimental
            // condition with a reporting obligation attached, and **no
            // policy implements it yet**. Accepting the value silently would
            // produce runs that look like the condition and are not it,
            // which is worse than refusing them.
            if plasticity.lamarckian_fraction_q16 != 0 {
                return Err(ConfigError::PhysiologyRange(
                    "lamarckian_fraction_q16 is nonzero but no inheritance policy is implemented; \
                     see specifications/plasticity-and-learning.md",
                    i64::from(plasticity.lamarckian_fraction_q16),
                ));
            }
        }
        // Phase 12. **Inside `validate_subsystems`, never appended to
        // `validate_contest`** - D-084 records that appending checks there
        // cost three phases of cap validation that never ran in any world
        // without contest, which is most of them. Every check below is
        // reachable from a world with nothing else enabled, which is the
        // configuration the relocating patch is meant to be studied in.
        let worldmod = &self.worldmod;
        if worldmod.enabled {
            if worldmod.dense_threshold_q16 > Q16_ONE {
                return Err(ConfigError::FractionOutOfRange(
                    "worldmod.dense_threshold_q16",
                    worldmod.dense_threshold_q16,
                ));
            }
            if worldmod.max_traversable_overrides == 0
                || worldmod.max_capacity_overrides == 0
                || worldmod.max_material_overrides == 0
            {
                return Err(ConfigError::PhysiologyRange("worldmod cap is zero", 0));
            }
            if worldmod.patch_enabled {
                if worldmod.relocate_interval_ticks == 0 {
                    return Err(ConfigError::NonPositive("relocate_interval_ticks"));
                }
                if worldmod.patch_radius_cells == 0
                    || worldmod.patch_radius_cells > MAX_PATCH_RADIUS_CELLS
                {
                    return Err(ConfigError::PhysiologyRange(
                        "patch_radius_cells",
                        i64::from(worldmod.patch_radius_cells),
                    ));
                }
                if worldmod.patch_capacity_scale_q16 == 0
                    || worldmod.patch_capacity_scale_q16 > MAX_CAPACITY_SCALE_Q16
                {
                    return Err(ConfigError::PhysiologyRange(
                        "patch_capacity_scale_q16",
                        i64::from(worldmod.patch_capacity_scale_q16),
                    ));
                }
                // A patch wider than the map would wrap its own clamp and
                // stop being a patch. Refused rather than clamped: a config
                // that asks for a patch covering everything is asking for a
                // different experiment than the one it would get.
                let span = 2 * worldmod.patch_radius_cells + 1;
                if span >= self.cells_x || span >= self.cells_y {
                    return Err(ConfigError::PhysiologyRange(
                        "patch_radius_cells spans the map",
                        i64::from(worldmod.patch_radius_cells),
                    ));
                }
                // The footprint has to fit under the layer's cap, or the
                // schedule would silently run against a full layer from its
                // first relocation - a run pressed against a cap that looks
                // exactly like a run that chose a smaller patch (C12.7).
                if u64::from(span) * u64::from(span) > u64::from(worldmod.max_capacity_overrides) {
                    return Err(ConfigError::PhysiologyRange(
                        "patch footprint exceeds max_capacity_overrides",
                        i64::from(span) * i64::from(span),
                    ));
                }
                // **A raising override is inert in a climate world, and this
                // refuses the combination rather than letting it read as a
                // null.**
                //
                // Measured, not suspected. `ClimateWorld::step` trims every
                // cell's biomass down to the **biome** capacity on the
                // reclassification cadence (100 ticks by default) and ledgers
                // the excess into its own sink. It derives that capacity
                // itself and knows nothing about this section, so a cell whose
                // composed capacity is 4x its biome capacity is cut back to 1x
                // every hundred ticks: the patch's headroom is harvested as
                // fast as `grow_food` fills it. A 4x patch measured over 4,900
                // ticks filled to 1.000 of its composed capacity without
                // climate and to 0.256 - which is 1/4, the biome ceiling -
                // with it.
                //
                // The fix belongs in `climate.rs`: `ClimateWorld::step` has to
                // trim against the composed capacity rather than derive its
                // own, which changes a signature in a file this phase was
                // scoped out of. Until it lands, a campaign that enabled both
                // would report a treatment arm indistinguishable from its
                // control and read it as "the relocating patch had no effect"
                // - the exact shape of null the morphology and plasticity
                // gates above exist to refuse. **Lowering** overrides are
                // unaffected and stay legal: a composed capacity below the
                // biome capacity is never reached by the climate trim.
                if self.climate.enabled && worldmod.patch_capacity_scale_q16 > Q16_ONE {
                    return Err(ConfigError::PhysiologyRange(
                        "a patch_capacity_scale_q16 above 1.0 is inert while the climate section \
                         is enabled: ClimateWorld::step trims biomass to the biome capacity and \
                         does not compose terrain overrides",
                        i64::from(worldmod.patch_capacity_scale_q16),
                    ));
                }
            }
        }
        // Phase 11 measurement section, validated on the same terms as every
        // section above and in the same function, for the same D-084 reason.
        let probe = &self.probe;
        if probe.enabled {
            // The marker locus is a schema-2 locus, so there is nothing for
            // it to live in without genome2. Refused rather than ignored: a
            // campaign that declared a drift control and silently got none
            // would compute C11.2's comparison against an empty denominator
            // and report a shift that had no control at all.
            if probe.marker_locus_enabled && !self.genome2.enabled {
                return Err(ConfigError::PhysiologyRange(
                    "probe.marker_locus_enabled requires genome2",
                    0,
                ));
            }
            // The action census counts Phase 2 intents. Without Phase 2 there
            // are no intents to classify and every row would be all-zero,
            // which is C11.1's null produced by the configuration rather than
            // by the world.
            if probe.action_census_enabled && !self.phase2.enabled {
                return Err(ConfigError::PhysiologyRange(
                    "probe.action_census_enabled requires phase2",
                    0,
                ));
            }
        } else if probe.action_census_enabled || probe.marker_locus_enabled {
            // A sub-gate on with the section off is refused rather than
            // treated as off. Silently ignoring it is how a campaign runs a
            // condition it did not get: `worldmod.patch_enabled` has the same
            // shape, and the same reasoning applies with more force here,
            // because a probe that quietly does nothing produces a *file* -
            // an empty one - rather than an obvious absence.
            return Err(ConfigError::PhysiologyRange(
                "a probe feature is enabled while probe.enabled is false",
                0,
            ));
        }
        // Phase 12 artifact section, validated on the same terms and in the
        // same function (D-084).
        let artifact = &self.artifact;
        if artifact.enabled {
            // Schema 1's fixed topology cannot bind a channel it was not
            // built with, so a schema-1 artifact world would have objects
            // nobody could ever touch: C12.1's null produced by the
            // configuration. Refused rather than allowed to run quietly.
            if !self.genome2.enabled {
                return Err(ConfigError::PhysiologyRange(
                    "artifact.enabled requires genome2",
                    0,
                ));
            }
            // The material-yield layer is mutable-world state; a terrain
            // strike writes it.
            if !self.worldmod.enabled {
                return Err(ConfigError::PhysiologyRange(
                    "artifact.enabled requires worldmod",
                    0,
                ));
            }
            for (name, value) in [
                ("artifact.max_objects", u64::from(artifact.max_objects)),
                (
                    "artifact.max_objects_per_cell",
                    u64::from(artifact.max_objects_per_cell),
                ),
                (
                    "artifact.max_composition_breadth",
                    u64::from(artifact.max_composition_breadth),
                ),
                (
                    "artifact.max_held_objects",
                    u64::from(artifact.max_held_objects),
                ),
                (
                    "artifact.max_candidates",
                    u64::from(artifact.max_candidates),
                ),
                ("artifact.reach_m", u64::from(artifact.reach_m)),
                (
                    "artifact.consume_reach_m",
                    u64::from(artifact.consume_reach_m),
                ),
                (
                    "artifact.perception_range_m",
                    u64::from(artifact.perception_range_m),
                ),
                (
                    "artifact.strike_force_q16",
                    u64::from(artifact.strike_force_q16),
                ),
                (
                    "artifact.fracture_margin_q16",
                    u64::from(artifact.fracture_margin_q16),
                ),
                (
                    "artifact.yield_regen_interval_ticks",
                    artifact.yield_regen_interval_ticks,
                ),
            ] {
                if value == 0 {
                    return Err(ConfigError::PhysiologyRange(name, 0));
                }
            }
            for (name, value) in [
                (
                    "artifact.carry_capacity_milli",
                    artifact.carry_capacity_milli,
                ),
                (
                    "artifact.strike_mass_reference_milli",
                    artifact.strike_mass_reference_milli,
                ),
                (
                    "artifact.min_fragment_mass_milli",
                    artifact.min_fragment_mass_milli,
                ),
                ("artifact.blocking_mass_milli", artifact.blocking_mass_milli),
                ("artifact.extraction_milli", artifact.extraction_milli),
            ] {
                if value <= 0 {
                    return Err(ConfigError::PhysiologyRange(name, value));
                }
            }
            for (name, value) in [
                (
                    "artifact.hold_cost_milli_per_s",
                    artifact.hold_cost_milli_per_s,
                ),
                ("artifact.action_cost_milli", artifact.action_cost_milli),
                ("artifact.strike_cost_milli", artifact.strike_cost_milli),
                ("artifact.terrain_yield_milli", artifact.terrain_yield_milli),
                ("artifact.yield_regen_milli", artifact.yield_regen_milli),
            ] {
                if value < 0 {
                    return Err(ConfigError::PhysiologyRange(name, value));
                }
            }
            if artifact.max_composition_depth > MAX_COMPOSITION_DEPTH {
                return Err(ConfigError::PhysiologyRange(
                    "artifact.max_composition_depth",
                    i64::from(artifact.max_composition_depth),
                ));
            }
            if artifact.max_composition_breadth < 2 {
                return Err(ConfigError::PhysiologyRange(
                    "artifact.max_composition_breadth",
                    i64::from(artifact.max_composition_breadth),
                ));
            }
            if artifact.max_fragments < 2 || artifact.max_fragments > MAX_FRAGMENTS {
                return Err(ConfigError::PhysiologyRange(
                    "artifact.max_fragments",
                    i64::from(artifact.max_fragments),
                ));
            }
            if artifact.joint_floor_q16 > Q16_ONE {
                return Err(ConfigError::FractionOutOfRange(
                    "artifact.joint_floor_q16",
                    artifact.joint_floor_q16,
                ));
            }
            if artifact.action_threshold_q16 < -(Q16_ONE as i32)
                || artifact.action_threshold_q16 > Q16_ONE as i32
            {
                return Err(ConfigError::ControllerThreshold(
                    "artifact.action_threshold_q16",
                    artifact.action_threshold_q16,
                ));
            }
            if artifact.stone_relative_q16 > Q16_ONE
                || artifact.wood_relative_q16 > artifact.stone_relative_q16
            {
                return Err(ConfigError::FractionOutOfRange(
                    "artifact.stone_relative_q16",
                    artifact.stone_relative_q16,
                ));
            }
            // A per-cell cap above the world cap can never bind, and a
            // held cap above the world cap likewise; neither is a defect,
            // but a fragment fan-out that cannot fit under the world cap
            // means every fracture is partly dust by construction. Refused
            // so the campaign that hits it is told rather than left to read
            // it as physics.
            if artifact.max_fragments > artifact.max_objects {
                return Err(ConfigError::PhysiologyRange(
                    "artifact.max_fragments exceeds max_objects",
                    i64::from(artifact.max_fragments),
                ));
            }
        } else if artifact.inert || artifact.ephemeral {
            // A condition arm switched on with the section off is refused
            // rather than treated as off, exactly as a probe feature is: a
            // campaign that asked for condition C and silently got a
            // section-less world would report a null it never measured.
            return Err(ConfigError::PhysiologyRange(
                "an artifact condition is enabled while artifact.enabled is false",
                0,
            ));
        }
        // Phase 13. Inside `validate_subsystems` for D-084's reason.
        let social = &self.social;
        if social.enabled {
            // The registry version scheme is a total order (ADR-0029
            // section 1): a social world offers versions 1..=3, and offering
            // the artifact channels without the artifact section would admit
            // a genome bound to `pick_up` in a world with no objects.
            if !self.artifact.enabled {
                return Err(ConfigError::PhysiologyRange(
                    "social.enabled requires artifact",
                    0,
                ));
            }
            if social.perception_k == 0 || social.perception_k > crate::social::PERCEPTION_K_MAX {
                return Err(ConfigError::PhysiologyRange(
                    "social.perception_k",
                    i64::from(social.perception_k),
                ));
            }
            if social.signal_channels == 0
                || social.signal_channels > crate::social::SIGNAL_CHANNELS_MAX
            {
                return Err(ConfigError::PhysiologyRange(
                    "social.signal_channels",
                    i64::from(social.signal_channels),
                ));
            }
            for (name, value) in [
                (
                    "social.perception_radius_m",
                    u64::from(social.perception_radius_m),
                ),
                (
                    "social.signal_base_range_m",
                    u64::from(social.signal_base_range_m),
                ),
            ] {
                if value == 0 {
                    return Err(ConfigError::PhysiologyRange(name, 0));
                }
            }
            if social.signal_cost_milli < 0 {
                return Err(ConfigError::Negative("social.signal_cost_milli"));
            }
            // A field retained whole never decays, and a permanent marking
            // is what artifacts are for; the boundary between the two
            // channels is load-bearing for the Phase 12/13 comparison.
            if social.signal_retain_q16 >= Q16_ONE {
                return Err(ConfigError::FractionOutOfRange(
                    "social.signal_retain_q16",
                    social.signal_retain_q16,
                ));
            }
            if social.signal_corruption_q16 > Q16_ONE {
                return Err(ConfigError::FractionOutOfRange(
                    "social.signal_corruption_q16",
                    social.signal_corruption_q16,
                ));
            }
            // Rule 5 is a plasticity rule; a world that offers it with no
            // learn phase would report a null it never measured.
            if social.observational_enabled && !self.plasticity.enabled {
                return Err(ConfigError::PhysiologyRange(
                    "social.observational_enabled requires plasticity",
                    0,
                ));
            }
            // Condition D preserves the emission and its cost and scrambles
            // only delivery; with the signal half off there is nothing to
            // scramble and the arm would silently be condition B.
            if social.scramble_delivery && !social.signal_enabled {
                return Err(ConfigError::PhysiologyRange(
                    "social.scramble_delivery requires signal_enabled",
                    0,
                ));
            }
        } else if social.perception_enabled != SocialConfig::social_default().perception_enabled
            || social.signal_enabled != SocialConfig::social_default().signal_enabled
            || social.scramble_delivery
            || social.observational_enabled
        {
            // A condition arm switched on (or a default sub-gate moved) with
            // the section off is refused rather than treated as off, exactly
            // as an artifact condition is.
            return Err(ConfigError::PhysiologyRange(
                "a social condition is set while social.enabled is false",
                0,
            ));
        }
        let physiology = &self.physiology;
        if physiology.enabled {
            if !(1..=6).contains(&physiology.basal_exponent_quarters) {
                return Err(ConfigError::PhysiologyRange(
                    "basal_exponent_quarters",
                    i64::from(physiology.basal_exponent_quarters),
                ));
            }
            if !(1..=4).contains(&physiology.senescence_power) {
                return Err(ConfigError::PhysiologyRange(
                    "senescence_power",
                    i64::from(physiology.senescence_power),
                ));
            }
            if physiology.senescence_scale_ticks == 0 {
                return Err(ConfigError::PhysiologyRange("senescence_scale_ticks", 0));
            }
            if physiology.thermal_pref_high_milli <= physiology.thermal_pref_low_milli {
                return Err(ConfigError::PhysiologyRange(
                    "thermal_pref_high_milli",
                    i64::from(physiology.thermal_pref_high_milli),
                ));
            }
            if physiology.thermal_neutral_band_milli < 0
                || physiology.thermal_cost_milli_per_s_per_degree < 0
            {
                return Err(ConfigError::PhysiologyRange(
                    "thermal_neutral_band_milli",
                    i64::from(physiology.thermal_neutral_band_milli),
                ));
            }
            // Phase 14 ontogeny (ADR-0030). Refused rather than inert when
            // its preconditions are missing: a config that asks for growth
            // and gets none would report adult-shaped juveniles and read as
            // C14.1's null.
            if physiology.ontogeny_enabled {
                if !self.morphology.enabled {
                    return Err(ConfigError::PhysiologyRange(
                        "ontogeny requires morphology",
                        0,
                    ));
                }
                if physiology.birth_modules_min == 0 {
                    return Err(ConfigError::PhysiologyRange("birth_modules_min is zero", 0));
                }
                if physiology.growth_cost_milli_per_mass_milli < 0 {
                    return Err(ConfigError::Negative("growth_cost_milli_per_mass_milli"));
                }
                if physiology.growth_rate_milli_per_s <= 0 {
                    return Err(ConfigError::NonPositive("growth_rate_milli_per_s"));
                }
                // The growth pass budgets `rate * dt_ms / 1000` whole milli
                // per tick; a rate that truncates to zero at this dt asks
                // for growth and silently gets none - refused rather than
                // inert, like every other gate in this section (found by
                // the Phase 14 benchmark's growing arm, whose juveniles
                // scanned forever and never paid a milli).
                if physiology.growth_rate_milli_per_s * i64::from(self.dt_ms) < 1_000 {
                    return Err(ConfigError::PhysiologyRange(
                        "growth_rate_milli_per_s floors to zero at this dt_ms",
                        physiology.growth_rate_milli_per_s,
                    ));
                }
            }
            // Phase 14 mate choice (ADR-0030). Refused rather than inert
            // when its preconditions are missing, for the reason ontogeny
            // is: a config that asks for choice and gets none would pair by
            // proximity and read as C14.2's null.
            if physiology.mate_choice_enabled {
                if !self.phase2.enabled {
                    return Err(ConfigError::PhysiologyRange(
                        "mate choice requires phase2",
                        0,
                    ));
                }
                if !self.genome2.enabled {
                    return Err(ConfigError::PhysiologyRange(
                        "mate choice requires genome2",
                        0,
                    ));
                }
            }
            if physiology.mate_choice_scramble && !physiology.mate_choice_enabled {
                return Err(ConfigError::PhysiologyRange(
                    "mate_choice_scramble is set while mate_choice_enabled is false",
                    0,
                ));
            }
        }
        // Phase 15 chemistry field (ADR-0031). Refused rather than inert
        // when a value asks for something the update cannot deliver.
        let chemistry = &self.chemistry;
        if chemistry.enabled {
            if chemistry.field_steps_per_tick == 0 {
                return Err(ConfigError::NonPositive("field_steps_per_tick"));
            }
            // Four neighbours each taking `diffusion_q16` must leave the
            // cell non-negative: bound the per-neighbour rate strictly
            // below a quarter.
            if chemistry.diffusion_q16 >= Q16_ONE / 4 {
                return Err(ConfigError::FractionOutOfRange(
                    "chemistry.diffusion_q16",
                    chemistry.diffusion_q16,
                ));
            }
            if chemistry.reaction_monomer_q16 > Q16_ONE
                || chemistry.reaction_recycle_q16 > Q16_ONE
                || chemistry.abiogenesis_cap_q16 > Q16_ONE
                || chemistry.scaffold_patch_contrast_q16 == 0
            {
                return Err(ConfigError::PhysiologyRange(
                    "chemistry rate outside its range",
                    0,
                ));
            }
            if chemistry.production_milli_per_step < 0 {
                return Err(ConfigError::Negative("production_milli_per_step"));
            }
            if chemistry.abiogenesis_enabled && chemistry.abiogenesis_seed_milli <= 0 {
                return Err(ConfigError::NonPositive("abiogenesis_seed_milli"));
            }
            if chemistry.microbial_enabled {
                if chemistry.replication_axis == 0
                    || chemistry.aggregation_axis == 0
                    || 2 * chemistry.replication_axis * chemistry.aggregation_axis > 64
                {
                    return Err(ConfigError::PhysiologyRange(
                        "microbial class axes outside 1..=64 classes",
                        i64::from(2 * chemistry.replication_axis * chemistry.aggregation_axis),
                    ));
                }
                if chemistry.growth_rate_low_q16 > chemistry.growth_rate_high_q16
                    || chemistry.growth_rate_high_q16 > Q16_ONE
                    || chemistry.growth_yield_q16 > Q16_ONE
                    || chemistry.death_q16 > Q16_ONE
                    || chemistry.death_waste_fraction_q16 > Q16_ONE
                    || chemistry.mutation_q16 > Q16_ONE / 8
                {
                    return Err(ConfigError::PhysiologyRange(
                        "microbial rate outside its range",
                        0,
                    ));
                }
            } else if chemistry.abiogenesis_enabled {
                // Abiogenesis seeds a CLASS density; with no microbial half
                // there is nothing to seed - refused rather than inert.
                return Err(ConfigError::PhysiologyRange(
                    "abiogenesis_enabled requires microbial_enabled",
                    0,
                ));
            }
            if chemistry.scaffold_patch_radius_cells > 0 {
                let span = 4 * chemistry.scaffold_patch_radius_cells;
                if span >= self.cells_x || span >= self.cells_y {
                    return Err(ConfigError::PhysiologyRange(
                        "scaffold_patch_radius_cells spans the map",
                        i64::from(chemistry.scaffold_patch_radius_cells),
                    ));
                }
            }
            if chemistry.excretion_fraction_q16 > Q16_ONE
                || chemistry.remains_fraction_q16 > Q16_ONE
            {
                return Err(ConfigError::PhysiologyRange(
                    "coupling fraction outside its range",
                    0,
                ));
            }
        } else if chemistry.abiogenesis_enabled {
            // A condition arm switched on with the section off is refused
            // rather than treated as off, exactly as a social condition is.
            return Err(ConfigError::PhysiologyRange(
                "abiogenesis_enabled is set while chemistry.enabled is false",
                0,
            ));
        } else if chemistry.excretion_fraction_q16 > 0 || chemistry.remains_fraction_q16 > 0 {
            // A coupling with no field to deposit into is refused rather
            // than inert, on the same terms.
            return Err(ConfigError::PhysiologyRange(
                "a coupling fraction is set while chemistry.enabled is false",
                0,
            ));
        }
        // Phase 16 transition (ADR-0032). Refused rather than inert on the
        // chemistry section's terms: the organism it produces is a schema-2
        // genome with a developed body, and nothing less can be admitted.
        let transition = &self.transition;
        if transition.enabled {
            if !(chemistry.enabled && chemistry.microbial_enabled) {
                return Err(ConfigError::TransitionRequires("chemistry.microbial_enabled"));
            }
            if !self.phase2.enabled {
                return Err(ConfigError::TransitionRequires("phase2.enabled"));
            }
            if !self.genome2.enabled {
                return Err(ConfigError::TransitionRequires("genome2.enabled"));
            }
            if !self.morphology.enabled {
                return Err(ConfigError::TransitionRequires("morphology.enabled"));
            }
            if transition.check_interval_ticks == 0 {
                return Err(ConfigError::PhysiologyRange(
                    "transition.check_interval_ticks is zero",
                    0,
                ));
            }
            if transition.persistence_checks == 0 {
                return Err(ConfigError::PhysiologyRange(
                    "transition.persistence_checks is zero",
                    0,
                ));
            }
            if transition.organism_energy_milli <= 0 {
                return Err(ConfigError::PhysiologyRange(
                    "transition.organism_energy_milli must be positive",
                    transition.organism_energy_milli,
                ));
            }
            if transition.density_floor_milli < transition.organism_energy_milli {
                return Err(ConfigError::PhysiologyRange(
                    "transition.density_floor_milli is below one organism's energy",
                    transition.density_floor_milli,
                ));
            }
            if transition.aggregation_step_min >= chemistry.aggregation_axis {
                return Err(ConfigError::PhysiologyRange(
                    "transition.aggregation_step_min is outside the aggregation axis",
                    i64::from(transition.aggregation_step_min),
                ));
            }
            if transition.max_organisms_per_event == 0
                || transition.max_materializations_per_tick == 0
            {
                return Err(ConfigError::PhysiologyRange(
                    "a transition cap is zero",
                    0,
                ));
            }
        }
        Ok(())
    }

    fn validate_contest(&self) -> Result<(), ConfigError> {
        let contest = &self.contest;
        if !contest.enabled {
            return Ok(());
        }
        // Contest is a Phase 2 mechanism: it wires reserved controller
        // channels, which only exist when a controller does.
        if !self.phase2.enabled {
            return Err(ConfigError::ContestRequiresPhase2);
        }
        if contest.base_health_milli <= 0 {
            return Err(ConfigError::NonPositive("base_health_milli"));
        }
        if contest.damage_base_milli < 0 {
            return Err(ConfigError::Negative("damage_base_milli"));
        }
        if contest.attack_cost_milli < 0 {
            return Err(ConfigError::Negative("attack_cost_milli"));
        }
        if contest.local_depletion_milli < 0 {
            return Err(ConfigError::Negative("local_depletion_milli"));
        }
        if contest.attack_range_m == 0 || contest.attack_range_m > 32 {
            return Err(ConfigError::AttackRange(contest.attack_range_m));
        }
        if contest.carcass_reach_m == 0 || contest.carcass_reach_m > 32 {
            return Err(ConfigError::AttackRange(contest.carcass_reach_m));
        }
        if contest.max_carcasses == 0 || contest.max_carcasses > 200_000 {
            return Err(ConfigError::MaxCarcasses(contest.max_carcasses));
        }
        for (name, value) in [
            ("damage_variance_q16", contest.damage_variance_q16),
            ("heal_energy_floor_q16", contest.heal_energy_floor_q16),
            ("damage_decay_q16_per_s", contest.damage_decay_q16_per_s),
            ("carcass_energy_q16", contest.carcass_energy_q16),
            ("carcass_decay_q16_per_s", contest.carcass_decay_q16_per_s),
        ] {
            if value > Q16_ONE {
                return Err(ConfigError::FractionOutOfRange(name, value));
            }
        }
        if !(-(Q16_ONE as i32)..=Q16_ONE as i32).contains(&contest.attack_threshold_q16) {
            return Err(ConfigError::ControllerThreshold(
                "attack_threshold_q16",
                contest.attack_threshold_q16,
            ));
        }
        Ok(())
    }

    fn validate_origin(&self) -> Result<(), ConfigError> {
        let origin = &self.origin;
        if origin.deme_count == 0 || origin.deme_count > crate::origin::MAX_DEMES {
            return Err(ConfigError::DemeCount(origin.deme_count));
        }
        if origin.archetype_count as usize > crate::origin::MAX_ARCHETYPES {
            return Err(ConfigError::ArchetypeCount(origin.archetype_count));
        }
        for field in [
            ("trait_low_q16", origin.trait_low_q16),
            ("trait_span_q16", origin.trait_span_q16),
            ("neural_span_q16", origin.neural_span_q16),
            ("deme_trait_spread_q16", origin.deme_trait_spread_q16),
        ] {
            if field.1 > Q16_ONE {
                return Err(ConfigError::FractionOutOfRange(field.0, field.1));
            }
        }
        // Phase 16 (ADR-0032): a scratch world needs a source of density
        // to have anything to become. The permanently empty world is
        // reached by disabling the *transition*, never by a scratch world
        // with no abiogenesis - that is refused rather than silently
        // empty.
        if origin.mode == crate::origin::OriginMode::Scratch
            && !(self.chemistry.enabled
                && self.chemistry.microbial_enabled
                && self.chemistry.abiogenesis_enabled)
        {
            return Err(ConfigError::ScratchRequiresAbiogenesis);
        }
        if origin.mode == crate::origin::OriginMode::Seeded {
            if origin.archetype_count == 0 {
                return Err(ConfigError::ArchetypeCount(0));
            }
            // Seeded placement matches against biomes, which the climate
            // section owns.
            if !self.climate.enabled {
                return Err(ConfigError::SeededRequiresClimate);
            }
            // Archetypes are sorted by ascending ID so that "allocate
            // founder IDs in ascending (archetype_id, draw_index) order" and
            // "key draws on array position" are the same ordering.
            for index in 1..origin.archetype_count as usize {
                if origin.archetypes[index].id <= origin.archetypes[index - 1].id {
                    return Err(ConfigError::ArchetypeOrder {
                        index,
                        id: origin.archetypes[index].id,
                    });
                }
            }
            for index in 0..origin.archetype_count as usize {
                if origin.archetypes[index].biome_affinity == 0 {
                    return Err(ConfigError::EmptyArchetypeAffinity {
                        id: origin.archetypes[index].id,
                    });
                }
            }
        }
        Ok(())
    }

    fn validate_climate(&self) -> Result<(), ConfigError> {
        let climate = &self.climate;
        // The generator and the climate section move together. A v2 world
        // without climate fields, or climate fields on a v1 world, would
        // both be worlds whose terrain and whose rules disagree.
        if climate.enabled != (climate.worldgen_version == WorldgenVersion::V2) {
            return Err(ConfigError::ClimateGeneratorMismatch {
                enabled: climate.enabled,
                generator: climate.worldgen_version.tag(),
            });
        }
        if !climate.enabled {
            return Ok(());
        }
        if climate.temperature_min_milli >= climate.temperature_max_milli {
            return Err(ConfigError::TemperatureBounds {
                min: climate.temperature_min_milli,
                max: climate.temperature_max_milli,
            });
        }
        for period in climate.drift_period_ticks {
            if period == 0 {
                return Err(ConfigError::NonPositive("drift_period_ticks"));
            }
        }
        if climate.season_period_ticks == 0 {
            return Err(ConfigError::NonPositive("season_period_ticks"));
        }
        // Drift must be slow relative to the season, or it is a second
        // season rather than a long-timescale term.
        for period in climate.drift_period_ticks {
            if period <= climate.season_period_ticks {
                return Err(ConfigError::DriftPeriodTooShort {
                    period,
                    season: climate.season_period_ticks,
                });
            }
        }
        if climate.sea_proximity_weight_q16 > Q16_ONE {
            return Err(ConfigError::FractionOutOfRange(
                "sea_proximity_weight_q16",
                climate.sea_proximity_weight_q16,
            ));
        }
        if climate.moisture_diffusion_q16 == 0 || climate.moisture_diffusion_q16 > Q16_ONE / 2 {
            return Err(ConfigError::FractionOutOfRange(
                "moisture_diffusion_q16",
                climate.moisture_diffusion_q16,
            ));
        }
        if climate.initial_moisture_milli <= 0 || climate.moisture_max_milli <= 0 {
            return Err(ConfigError::NonPositive("initial_moisture_milli"));
        }
        if climate.moisture_ceiling_milli < climate.moisture_max_milli {
            return Err(ConfigError::NonPositive("moisture_ceiling_milli"));
        }
        if climate.coastal_moisture_bonus_milli < 0 {
            return Err(ConfigError::Negative("coastal_moisture_bonus_milli"));
        }
        // Biome thresholds must be ordered, or a biome is unreachable by
        // construction and C6.7 would fail for a reason config caused.
        if !(climate.arid_moisture_milli < climate.forest_moisture_milli
            && climate.forest_moisture_milli < climate.wetland_moisture_milli)
        {
            return Err(ConfigError::BiomeThresholdOrder {
                arid: climate.arid_moisture_milli,
                forest: climate.forest_moisture_milli,
                wetland: climate.wetland_moisture_milli,
            });
        }
        if climate.highland_elevation_q16 > Q16_ONE {
            return Err(ConfigError::FractionOutOfRange(
                "highland_elevation_q16",
                climate.highland_elevation_q16,
            ));
        }
        if climate.reclassify_interval_ticks == 0 {
            return Err(ConfigError::NonPositive("reclassify_interval_ticks"));
        }
        if climate.biome_capacity_q16[crate::climate::Biome::Water as usize] != 0 {
            return Err(ConfigError::NonPositive("biome_capacity_q16[water]"));
        }
        Ok(())
    }

    fn validate_phase2(&self) -> Result<(), ConfigError> {
        let phase2 = &self.phase2;
        let q16_fractions = [
            (
                "variation_probability_q16",
                phase2.variation_probability_q16,
            ),
            (
                "variation_trait_sigma_q16",
                phase2.variation_trait_sigma_q16,
            ),
            (
                "variation_neural_sigma_q16",
                phase2.variation_neural_sigma_q16,
            ),
            (
                "compatibility_threshold_q16",
                phase2.compatibility_threshold_q16,
            ),
            ("cluster_threshold_q16", phase2.cluster_threshold_q16),
            (
                "cluster_neural_weight_q16",
                phase2.cluster_neural_weight_q16,
            ),
        ];
        for (name, value) in q16_fractions {
            if value > Q16_ONE {
                return Err(ConfigError::FractionOutOfRange(name, value));
            }
        }
        if phase2.pairing_range_m == 0 || phase2.pairing_range_m > 32 {
            return Err(ConfigError::PairingRange(phase2.pairing_range_m));
        }
        if phase2.pairing_energy_threshold_milli <= 0
            || phase2.pairing_energy_threshold_milli > self.energy_max_milli
        {
            return Err(ConfigError::PairingEnergy(
                phase2.pairing_energy_threshold_milli,
            ));
        }
        if phase2.pairing_overhead_milli < 0 {
            return Err(ConfigError::Negative("pairing_overhead_milli"));
        }
        let thresholds = [
            ("eat_threshold_q16", phase2.eat_threshold_q16),
            ("mate_threshold_q16", phase2.mate_threshold_q16),
            ("rest_threshold_q16", phase2.rest_threshold_q16),
        ];
        for (name, value) in thresholds {
            if !(-(Q16_ONE as i32)..=Q16_ONE as i32).contains(&value) {
                return Err(ConfigError::ControllerThreshold(name, value));
            }
        }
        if phase2.max_turn_per_tick_bam == 0 || phase2.max_turn_per_tick_bam > 16_384 {
            return Err(ConfigError::TurnRate(phase2.max_turn_per_tick_bam));
        }
        if phase2.cluster_sample_max == 0 || phase2.cluster_sample_max > 65_536 {
            return Err(ConfigError::ClusterSample(phase2.cluster_sample_max));
        }
        Ok(())
    }

    /// Canonical config hash over the schema version, policy versions, and
    /// every field in declaration order.
    /// The channel registry version this world offers its organisms: 2 when
    /// the artifact section is enabled, else 1. What the genome2 config block
    /// hashes, what `bind` draws from, and what a genome's bindings are
    /// validated against at construction and restore (ADR-0028 section 7).
    pub fn channel_registry_version(&self) -> u16 {
        if self.social.enabled {
            crate::registry::CHANNEL_REGISTRY_VERSION_SOCIAL
        } else if self.artifact.enabled {
            crate::registry::CHANNEL_REGISTRY_VERSION_ARTIFACT
        } else {
            crate::registry::CHANNEL_REGISTRY_VERSION
        }
    }

    pub fn stable_hash(&self) -> u64 {
        let mut hasher = Fnv1a64::new();
        hasher.update(b"lifesim-config");
        hasher.update_u32(CONFIG_SCHEMA_VERSION);
        hasher.update(BEHAVIOR_POLICY_VERSION.as_bytes());
        hasher.update(crate::rng::RNG_ALGORITHM_VERSION.as_bytes());
        // The *selected* generator, not a constant. With the default V1 this
        // hashes exactly the byte string it always did, so adding V2 cannot
        // move any existing world's config hash.
        hasher.update(self.climate.worldgen_version.tag().as_bytes());
        hasher.update_u64(self.world_seed);
        hasher.update_u32(self.cells_x);
        hasher.update_u32(self.cells_y);
        hasher.update_u32(self.cell_size_m);
        hasher.update_u32(self.initial_organisms);
        hasher.update_u32(self.max_entities);
        hasher.update_u32(self.dt_ms);
        hasher.update_u32(self.growth_rate_q16_per_s);
        hasher.update_i64(self.cell_capacity_milli);
        hasher.update_u32(self.initial_biomass_q16);
        hasher.update_i64(self.energy_max_milli);
        hasher.update_i64(self.initial_energy_milli);
        hasher.update_i64(self.basal_cost_milli_per_s);
        hasher.update_i64(self.move_cost_milli_per_s);
        hasher.update_i64(self.intake_rate_milli_per_s);
        hasher.update_u32(self.assimilation_q16);
        hasher.update_u32(self.speed_mps_q16);
        hasher.update_u32(self.crowding_radius_m);
        hasher.update_u32(self.crowding_threshold);
        hasher.update_i64(self.crowding_cost_milli_per_s);
        hasher.update_u64(self.maturity_age_ticks);
        hasher.update_u64(self.max_age_ticks);
        hasher.update_u32(u32::from(self.reproduction_enabled));
        hasher.update_i64(self.repro_threshold_milli);
        hasher.update_i64(self.offspring_energy_milli);
        hasher.update_i64(self.repro_overhead_milli);
        hasher.update_u64(self.repro_cooldown_ticks);
        hasher.update_u32(self.land_threshold_q16);
        hasher.update_u32(self.min_land_fraction_q16);
        hasher.update_u32(self.max_land_fraction_q16);
        // The Phase 2 section participates in the hash only when enabled so
        // that phase2-disabled configs hash identically to Phase 1 configs.
        // A disabled section is behaviorally inert by construction (the
        // world takes the exact Phase 1 code paths), so this equality is
        // semantic, not cosmetic. Enabling Phase 2 changes the hash and
        // starts a new replay lineage.
        if self.phase2.enabled {
            hasher.update(b"lifesim-phase2-config");
            hasher.update(PHASE2_BEHAVIOR_POLICY_VERSION.as_bytes());
            hasher.update(crate::genome::GENOME_POLICY_VERSION.as_bytes());
            hasher.update(crate::controller::CONTROLLER_POLICY_VERSION.as_bytes());
            hasher.update_u32(u32::from(crate::genome::GENOME_SCHEMA_VERSION));
            hasher.update_u32(u32::from(crate::genome::TOPOLOGY_ID));
            hasher.update_u32(self.phase2.variation_probability_q16);
            hasher.update_u32(self.phase2.variation_trait_sigma_q16);
            hasher.update_u32(self.phase2.variation_neural_sigma_q16);
            hasher.update_u32(self.phase2.pairing_range_m);
            hasher.update_u32(self.phase2.compatibility_threshold_q16);
            hasher.update_i64(self.phase2.pairing_energy_threshold_milli);
            hasher.update_i64(self.phase2.pairing_overhead_milli);
            hasher.update_i32(self.phase2.eat_threshold_q16);
            hasher.update_i32(self.phase2.mate_threshold_q16);
            hasher.update_i32(self.phase2.rest_threshold_q16);
            hasher.update_u32(self.phase2.max_turn_per_tick_bam);
            hasher.update_u32(self.phase2.cluster_threshold_q16);
            hasher.update_u32(self.phase2.cluster_sample_max);
            hasher.update_u32(self.phase2.cluster_neural_weight_q16);
        }
        // The Phase 6 section participates only when enabled, so a
        // climate-disabled config hashes exactly as it did before Phase 6
        // existed and both fixtures are preserved.
        if self.climate.enabled {
            hasher.update(b"lifesim-climate-config");
            hasher.update(crate::climate::BIOME_POLICY_VERSION.as_bytes());
            hasher.update(crate::climate::CLIMATE_POLICY_VERSION.as_bytes());
            hasher.update_i32(self.climate.base_temperature_milli);
            hasher.update_i32(self.climate.lapse_milli_per_full_elevation);
            hasher.update_i32(self.climate.latitude_amplitude_milli);
            hasher.update_u64(self.climate.season_period_ticks);
            hasher.update_i32(self.climate.season_amplitude_milli);
            for period in self.climate.drift_period_ticks {
                hasher.update_u64(period);
            }
            for amplitude in self.climate.drift_amplitude_milli {
                hasher.update_i32(amplitude);
            }
            hasher.update_i32(self.climate.temperature_min_milli);
            hasher.update_i32(self.climate.temperature_max_milli);
            hasher.update_i64(self.climate.initial_moisture_milli);
            hasher.update_i64(self.climate.coastal_moisture_bonus_milli);
            hasher.update_i64(self.climate.moisture_max_milli);
            hasher.update_i64(self.climate.moisture_ceiling_milli);
            hasher.update_u32(self.climate.sea_proximity_weight_q16);
            hasher.update_u32(self.climate.moisture_diffusion_q16);
            hasher.update_u32(self.climate.moisture_drain_weight);
            hasher.update_u32(self.climate.highland_elevation_q16);
            hasher.update_i64(self.climate.wetland_moisture_milli);
            hasher.update_i64(self.climate.arid_moisture_milli);
            hasher.update_i64(self.climate.forest_moisture_milli);
            hasher.update_i32(self.climate.forest_min_temperature_milli);
            for multiplier in self.climate.biome_capacity_q16 {
                hasher.update_u32(multiplier);
            }
            hasher.update_u64(self.climate.reclassify_interval_ticks);
        }
        // The origin section participates only when it differs from the
        // Phase 1/2 founder behavior, so a default-origin config hashes
        // exactly as it did before Phase 6 existed.
        if !crate::origin::is_default_origin(&self.origin) {
            crate::origin::hash_origin_into(&mut hasher, &self.origin);
        }
        // Phase 7 section: hashed only when enabled, so a contest-disabled
        // config hashes exactly as it did before Phase 7 existed.
        if self.contest.enabled {
            hasher.update(b"lifesim-contest-config");
            hasher.update(crate::contest::CONTEST_POLICY_VERSION.as_bytes());
            hasher.update(crate::contest::PAIR_KEY_POLICY_VERSION.as_bytes());
            hasher.update_i64(self.contest.base_health_milli);
            hasher.update_i64(self.contest.damage_base_milli);
            hasher.update_u32(self.contest.damage_variance_q16);
            hasher.update_i64(self.contest.attack_cost_milli);
            hasher.update_u32(self.contest.attack_range_m);
            hasher.update_i32(self.contest.attack_threshold_q16);
            hasher.update_u64(self.contest.attack_cooldown_ticks);
            hasher.update_i64(self.contest.heal_milli_per_s);
            hasher.update_u32(self.contest.heal_energy_cost_q16);
            hasher.update_u32(self.contest.heal_energy_floor_q16);
            hasher.update_u32(self.contest.damage_decay_q16_per_s);
            hasher.update_u32(self.contest.carcass_energy_q16);
            hasher.update_u32(self.contest.carcass_decay_q16_per_s);
            hasher.update_u32(self.contest.carcass_reach_m);
            hasher.update_u32(self.contest.max_carcasses);
            hasher.update_i64(self.contest.local_depletion_milli);
        }
        // Phase 8 section: hashed only when enabled, so a
        // demography-disabled config hashes exactly as it did before Phase
        // 8 existed and every earlier fixture is preserved.
        if self.physiology.enabled {
            hasher.update(b"lifesim-physiology-config");
            hasher.update(crate::physiology::PHYSIOLOGY_POLICY_VERSION.as_bytes());
            hasher.update_u32(u32::from(self.physiology.allometry_enabled));
            hasher.update_u32(self.physiology.basal_exponent_quarters);
            hasher.update_u32(u32::from(self.physiology.thermoregulation_enabled));
            hasher.update_i32(self.physiology.thermal_pref_low_milli);
            hasher.update_i32(self.physiology.thermal_pref_high_milli);
            hasher.update_i32(self.physiology.thermal_neutral_band_milli);
            hasher.update_i64(self.physiology.thermal_cost_milli_per_s_per_degree);
            hasher.update_u32(u32::from(self.physiology.senescence_enabled));
            hasher.update_u64(self.physiology.senescence_onset_ticks);
            hasher.update_u64(self.physiology.senescence_scale_ticks);
            hasher.update_u32(self.physiology.senescence_power);
            hasher.update_u32(self.physiology.senescence_hazard_q16_per_s);
            hasher.update_u32(self.physiology.extrinsic_hazard_q16_per_s);
            hasher.update_u32(self.physiology.juvenile_hazard_multiplier_q16);
            // Phase 14 ontogeny: hashed only when its own gate is on, so
            // every physiology-enabled hash issued before ontogeny existed
            // is unchanged - the same nesting discipline the artifact and
            // social sections use (smallest covering version).
            if self.physiology.ontogeny_enabled {
                hasher.update(b"lifesim-physiology-v2-ontogeny");
                hasher.update_u32(self.physiology.birth_modules_min);
                hasher.update_i64(self.physiology.growth_cost_milli_per_mass_milli);
                hasher.update_i64(self.physiology.growth_rate_milli_per_s);
            }
            if self.physiology.mate_choice_enabled {
                hasher.update(b"lifesim-physiology-v2-mate-choice");
                hasher.update_u32(u32::from(self.physiology.mate_choice_scramble));
            }
        }
        // Phase 15 section: hashed only when enabled, so every hash issued
        // before the chemistry field existed is unchanged.
        if self.chemistry.enabled {
            hasher.update(b"lifesim-chemistry-config");
            hasher.update(crate::chemistry::CHEMISTRY_POLICY_VERSION.as_bytes());
            hasher.update_u32(u32::from(crate::chemistry::SUBSTRATE_REGISTRY_VERSION));
            hasher.update_u32(self.chemistry.field_steps_per_tick);
            hasher.update_u32(self.chemistry.diffusion_q16);
            hasher.update_u32(self.chemistry.reaction_monomer_q16);
            hasher.update_u32(self.chemistry.reaction_recycle_q16);
            hasher.update_i64(self.chemistry.production_milli_per_step);
            hasher.update_u32(self.chemistry.scaffold_patch_radius_cells);
            hasher.update_u32(self.chemistry.scaffold_patch_contrast_q16);
            hasher.update_u32(u32::from(self.chemistry.abiogenesis_enabled));
            hasher.update_u32(self.chemistry.abiogenesis_weight_primordial_q16);
            hasher.update_u32(self.chemistry.abiogenesis_weight_monomer_q16);
            hasher.update_u32(self.chemistry.abiogenesis_weight_polymer_q16);
            hasher.update_u32(self.chemistry.abiogenesis_cap_q16);
            hasher.update_i64(self.chemistry.abiogenesis_seed_milli);
            if self.chemistry.microbial_enabled {
                hasher.update(b"lifesim-microbial-config");
                hasher.update(crate::microbial::MICROBIAL_POLICY_VERSION.as_bytes());
                hasher.update_u32(u32::from(crate::microbial::CLASS_REGISTRY_VERSION));
                hasher.update_u32(self.chemistry.replication_axis);
                hasher.update_u32(self.chemistry.aggregation_axis);
                hasher.update_u32(self.chemistry.growth_rate_low_q16);
                hasher.update_u32(self.chemistry.growth_rate_high_q16);
                hasher.update_u32(self.chemistry.growth_yield_q16);
                hasher.update_u32(self.chemistry.death_q16);
                hasher.update_u32(self.chemistry.death_waste_fraction_q16);
                hasher.update_u32(self.chemistry.mutation_q16);
            }
            // Coupling v1: hashed only when a fraction is nonzero, so every
            // hash issued before the coupling existed is unchanged.
            if self.chemistry.excretion_fraction_q16 > 0 || self.chemistry.remains_fraction_q16 > 0
            {
                hasher.update(b"lifesim-chemistry-coupling");
                hasher.update_u32(self.chemistry.excretion_fraction_q16);
                hasher.update_u32(self.chemistry.remains_fraction_q16);
            }
        }
        // Phase 16 section: hashed only when enabled, so every hash issued
        // before the transition existed is unchanged. The map version is
        // part of what a materialized organism means, so it enters here.
        if self.transition.enabled {
            hasher.update(b"lifesim-transition-config");
            hasher.update(crate::transition::TRANSITION_POLICY_VERSION.as_bytes());
            hasher.update_u32(u32::from(crate::transition::GENOME_MAP_VERSION));
            hasher.update_u64(self.transition.check_interval_ticks);
            hasher.update_i64(self.transition.density_floor_milli);
            hasher.update_u32(self.transition.persistence_checks);
            hasher.update_u32(self.transition.aggregation_step_min);
            hasher.update_i64(self.transition.organism_energy_milli);
            hasher.update_u32(self.transition.max_organisms_per_event);
            hasher.update_u32(self.transition.max_materializations_per_tick);
        }
        // Phase 9 section: hashed only when enabled, so a schema-1 config
        // hashes exactly as it did before schema 2 existed and every earlier
        // fixture is preserved.
        if self.genome2.enabled {
            hasher.update(b"lifesim-genome2-config");
            hasher.update(crate::genome2::GENOME2_POLICY_VERSION.as_bytes());
            hasher.update(crate::meiosis::MEIOSIS_POLICY_VERSION.as_bytes());
            hasher.update(crate::structmut::STRUCTMUT_POLICY_VERSION.as_bytes());
            hasher.update(crate::controller2::CONTROLLER2_POLICY_VERSION.as_bytes());
            // The registries are part of what a genome means, so their
            // versions enter the hash: the same loci under a different
            // channel registry describe a different organism. **The channel
            // registry version is the one this world offers**, which is 1
            // for every world without the artifact section - so every hash
            // issued before the artifact half existed is unchanged - and 2
            // for a world with it (ADR-0028 section 7).
            let (_, activations) = crate::genome2::registry_versions();
            hasher.update_u32(u32::from(self.channel_registry_version()));
            hasher.update_u32(u32::from(activations));
            let caps = &self.genome2.caps;
            hasher.update_u32(u32::from(caps.max_chromosomes));
            hasher.update_u32(caps.max_loci_per_chromosome);
            hasher.update_u32(caps.max_nodes);
            hasher.update_u32(caps.max_edges);
            hasher.update_u32(caps.max_edges_per_node);
            hasher.update_u32(caps.max_genome_bytes);
            hasher.update_u32(caps.min_nodes);
            hasher.update_u32(u32::from(self.genome2.meiosis.mode.id()));
            hasher.update_u32(self.genome2.meiosis.max_extra_crossovers);
            let mutation = &self.genome2.mutation;
            hasher.update_u32(mutation.point_q16);
            hasher.update_u32(mutation.duplication_q16);
            hasher.update_u32(mutation.deletion_q16);
            hasher.update_u32(mutation.insertion_q16);
            hasher.update_u32(mutation.transposition_q16);
            hasher.update_u32(mutation.max_run);
            hasher.update_u32(mutation.point_delta_q16);
            hasher.update_u32(u32::from(mutation.regulatory_enabled));
            // Phase 11's plasticity-mutation gate: **hashed only when it is
            // true**, and appended after every field that existed before it.
            //
            // Both obvious choices are wrong. Omitting it would let two
            // behaviorally different worlds - one whose plasticity genes
            // evolve, one whose do not - share a config hash, which is a
            // real defect and the thing this hand-maintained list exists to
            // prevent. Hashing it unconditionally would fold a Phase 11
            // field into every schema-2 world that already exists and move
            // the Phase 9 fixture (config `0x9abc0cd47914127f`), which was
            // pinned before the field existed.
            //
            // This is D-014's "a section is folded in only when enabled"
            // applied at **field** granularity rather than section
            // granularity, which is what the situation actually needs: the
            // genome2 section *is* enabled in those worlds, and the field
            // is not. Its own tag keeps it self-describing, so a later
            // Phase 11 config block appending here cannot alias it.
            if mutation.plasticity_enabled {
                hasher.update(b"lifesim-plasticity-mutation-v1");
                hasher.update_u32(1);
            }
        }
        // Phase 10 section, on the same terms: hashed only when enabled, so
        // every earlier fixture survives untouched (D-014).
        if self.morphology.enabled {
            hasher.update(b"lifesim-morphology-config");
            hasher.update(crate::morphology::MORPHOLOGY_POLICY_VERSION.as_bytes());
            hasher.update(crate::develop::DEVELOP_POLICY_VERSION.as_bytes());
            // The module registry is part of what a body means: the same
            // modules under different coefficients describe a different
            // organism.
            hasher.update_u32(u32::from(crate::morphology::MODULE_REGISTRY_VERSION));
            hasher.update_u32(u32::from(self.morphology.lattice.id()));
            let caps = &self.morphology.caps;
            hasher.update_u32(u32::from(caps.max_modules));
            hasher.update_u32(caps.lattice_radius as u32);
            hasher.update_u32(u32::from(caps.max_growth_steps));
            hasher.update_u32(u32::from(caps.required_types_mask));
            hasher.update_u32(self.morphology.base_node_budget);
        }
        // Phase 11 section, **appended last and hashed only when enabled**.
        // Appended rather than slotted next to the genome2 block it depends
        // on: the order of this function is the definition of every existing
        // config hash, and inserting a section anywhere but the end would
        // move worlds that do not have it. Enabling plasticity changes the
        // hash and starts a new replay lineage, which is correct - a world
        // whose weights change within a lifetime is not the same experiment.
        if self.plasticity.enabled {
            hasher.update(b"lifesim-plasticity-config");
            hasher.update(crate::plasticity::PLASTICITY_POLICY_VERSION.as_bytes());
            // The rule registry is part of what a plasticity gene means: the
            // same `rule_id` under a different registry is a different rule,
            // exactly as the same locus under a different channel registry
            // describes a different organism.
            // The version this world *offers*, not the constant: 1 for every
            // world without the gated observational rule, so the Phase 11
            // fixture hashes the byte it always hashed (the channel
            // registry's precedent, ADR-0028 section 7).
            hasher.update_u32(u32::from(self.plasticity_rule_registry_version()));
            hasher.update_i64(self.plasticity.plastic_edge_cost_milli_per_s);
            hasher.update_u32(self.plasticity.max_plastic_edges);
            hasher.update_u32(self.plasticity.lamarckian_fraction_q16);
            // ADR-0027, hashed **only when set** and appended after the
            // fields that were already here. A world with a live rule 0 is a
            // different experiment - the same allele names a different rule,
            // and the mutation draw has a different range - so it is a new
            // replay lineage and must be. A world with the flag clear hashes
            // byte-identically to one from before the flag existed, which is
            // what keeps the Phase 11 fixture where it is.
            if self.plasticity.live_rule_zero {
                hasher.update(b"lifesim-live-rule-zero");
                hasher.update_u32(u32::from(crate::plasticity::LIVE_RULE_COUNT));
            }
            // Appended after the chain's word, hashed only when set, for the
            // reason every section before it was: this function's order is
            // the definition of every hash already issued. Repricing plastic
            // edges changes what selection sees and is a new replay lineage.
            if self.plasticity.price_moved_edges_only {
                hasher.update(b"lifesim-plasticity-moat");
            }
        }
        // Phase 12 section, **appended after Phase 11's and hashed only when
        // enabled**. Appended for the reason every section before it was:
        // the order of this function is the definition of every existing
        // config hash, and inserting a section anywhere but the end would
        // move worlds that do not have it - here, four of them. Enabling the
        // mutable world changes the hash and starts a new replay lineage,
        // which is correct: a world whose terrain organisms can edit is not
        // the same experiment as one whose terrain is a function of its seed.
        if self.worldmod.enabled {
            hasher.update(b"lifesim-worldmod-config");
            hasher.update(crate::terrainmod::WORLDMOD_POLICY_VERSION.as_bytes());
            // The layer registry is part of what an override means: the same
            // `(layer_id, value)` under a different layer assignment
            // describes a different world, exactly as the same locus under a
            // different channel registry describes a different organism.
            hasher.update_u32(u32::from(crate::terrainmod::LAYER_COUNT));
            hasher.update_u32(self.worldmod.dense_threshold_q16);
            hasher.update_u32(self.worldmod.max_traversable_overrides);
            hasher.update_u32(self.worldmod.max_capacity_overrides);
            hasher.update_u32(self.worldmod.max_material_overrides);
            hasher.update_u32(u32::from(self.worldmod.patch_enabled));
            hasher.update_u64(self.worldmod.relocate_interval_ticks);
            hasher.update_u32(self.worldmod.patch_radius_cells);
            hasher.update_u32(self.worldmod.patch_capacity_scale_q16);
        }
        // Phase 11 measurement section, **appended after Phase 12's and
        // hashed only when enabled**, for the reason every section before it
        // was appended: this function's order is the definition of every
        // existing config hash, and inserting anywhere but the end would move
        // worlds that do not have the section - here, five of them.
        //
        // Enabling it changes the hash and starts a new replay lineage, which
        // is correct in both halves. The marker locus changes what a genome
        // contains and therefore what point mutation can land on; the action
        // census changes what is stored and checksummed. Neither is the same
        // experiment as the world without it, and a config hash that said
        // otherwise would let a campaign compare them as if they were.
        if self.probe.enabled {
            hasher.update(b"lifesim-probe-config");
            hasher.update(crate::actioncensus::ACTION_CENSUS_POLICY_VERSION.as_bytes());
            // The class set is part of what a recorded histogram means, on
            // the same terms the rule registry is part of what a plasticity
            // gene means.
            hasher.update_u32(crate::actioncensus::ACTION_CLASS_COUNT as u32);
            hasher.update_i32(crate::actioncensus::TURN_BAND_MILLI);
            hasher.update_u32(u32::from(self.probe.action_census_enabled));
            hasher.update_u32(u32::from(self.probe.marker_locus_enabled));
        }
        // Phase 12 artifact section, **appended after every section before it
        // and hashed only when enabled**, for the reason every one of them
        // was: this function's order is the definition of every existing
        // config hash. Enabling objects changes the hash and starts a new
        // replay lineage, which is correct - a world with objects in it is not
        // the same experiment as one without - and the material registry and
        // the channel set this world offers are part of what the section
        // means, on the terms the channel registry is part of what a genome
        // means.
        if self.artifact.enabled {
            let artifact = &self.artifact;
            hasher.update(b"lifesim-artifact-config");
            hasher.update(crate::artifact::ARTIFACT_POLICY_VERSION.as_bytes());
            crate::material::hash_registry_into(&mut hasher);
            hasher.update_u32(u32::from(
                crate::registry::CHANNEL_REGISTRY_VERSION_ARTIFACT,
            ));
            hasher.update_u32(u32::from(artifact.inert));
            hasher.update_u32(u32::from(artifact.ephemeral));
            hasher.update_u32(artifact.max_objects);
            hasher.update_u32(artifact.max_objects_per_cell);
            hasher.update_u32(artifact.max_composition_depth);
            hasher.update_u32(artifact.max_composition_breadth);
            hasher.update_u32(artifact.max_held_objects);
            hasher.update_u32(artifact.max_candidates);
            hasher.update_i64(artifact.carry_capacity_milli);
            hasher.update_u32(artifact.carry_move_cost_q16);
            hasher.update_i64(artifact.hold_cost_milli_per_s);
            hasher.update_i64(artifact.action_cost_milli);
            hasher.update_i64(artifact.strike_cost_milli);
            hasher.update_i32(artifact.action_threshold_q16);
            hasher.update_u32(artifact.reach_m);
            hasher.update_u32(artifact.consume_reach_m);
            hasher.update_u32(artifact.perception_range_m);
            hasher.update_u32(artifact.strike_force_q16);
            hasher.update_i64(artifact.strike_mass_reference_milli);
            hasher.update_u32(artifact.fracture_margin_q16);
            hasher.update_u32(artifact.max_fragments);
            hasher.update_i64(artifact.min_fragment_mass_milli);
            hasher.update_u32(artifact.joint_floor_q16);
            hasher.update_i64(artifact.blocking_mass_milli);
            hasher.update_i64(artifact.terrain_yield_milli);
            hasher.update_i64(artifact.extraction_milli);
            hasher.update_i64(artifact.yield_regen_milli);
            hasher.update_u64(artifact.yield_regen_interval_ticks);
            hasher.update_u32(artifact.stone_relative_q16);
            hasher.update_u32(artifact.wood_relative_q16);
        }
        // Phase 13 social section, **appended after every section before it
        // and hashed only when enabled**, for the reason every one of them
        // was: this function's order is the definition of every existing
        // config hash. Enabling the section changes the hash and starts a
        // new replay lineage, which is correct - a world whose organisms can
        // perceive one another and signal is not the same experiment as one
        // whose organisms cannot - and the channel set this world offers is
        // part of what the section means.
        if self.social.enabled {
            let social = &self.social;
            hasher.update(b"lifesim-social-config");
            hasher.update(crate::social::SOCIAL_POLICY_VERSION.as_bytes());
            hasher.update_u32(u32::from(crate::registry::CHANNEL_REGISTRY_VERSION_SOCIAL));
            hasher.update_u32(u32::from(social.perception_enabled));
            hasher.update_u32(u32::from(social.signal_enabled));
            hasher.update_u32(u32::from(social.scramble_delivery));
            hasher.update_u32(u32::from(social.observational_enabled));
            hasher.update_u32(social.perception_k);
            hasher.update_u32(social.perception_radius_m);
            hasher.update_u32(social.signal_channels);
            hasher.update_u32(social.signal_base_range_m);
            hasher.update_i64(social.signal_cost_milli);
            hasher.update_u32(social.signal_retain_q16);
            hasher.update_u32(social.signal_corruption_q16);
        }
        hasher.finish()
    }

    /// How many plastic edges a network compiled for this world may carry.
    ///
    /// `None` when the plasticity section is disabled, which is **not** the
    /// same as a budget of zero: with `None` no edge is compiled plastic at
    /// all and nothing is counted as refused, so the compiled plan is
    /// byte-identical to the one this world produced before Phase 11.
    /// `live_rule_zero` is carried here too, and it is gated on `enabled` for
    /// the same reason the cap is: a disabled section compiles exactly the
    /// plan it compiled before Phase 11, and a remap applied to a world with
    /// no plastic edges would be a difference with nothing to differ about.
    pub fn plasticity_budget(&self) -> crate::controller2::PlasticityBudget {
        if !self.plasticity.enabled {
            return crate::controller2::PlasticityBudget::disabled();
        }
        let budget = crate::controller2::PlasticityBudget::edges(self.plasticity.max_plastic_edges);
        let budget = if self.plasticity.live_rule_zero {
            budget.with_live_rule_zero()
        } else {
            budget
        };
        if self.observational_offered() {
            budget.with_observational()
        } else {
            budget
        }
    }

    /// Whether rule 5 (Observational) is in this world's effective rule
    /// space: the social section on and its gate set (condition P).
    /// Validation guarantees the gate implies the plasticity section, so
    /// the rule always has a learn phase to run in.
    pub fn observational_offered(&self) -> bool {
        self.social.enabled && self.social.observational_enabled
    }

    /// The plasticity rule-registry version this world offers: 2 when the
    /// observational rule is offered, else 1.
    pub fn plasticity_rule_registry_version(&self) -> u16 {
        if self.observational_offered() {
            crate::plasticity::RULE_REGISTRY_VERSION_OBSERVATIONAL
        } else {
            crate::plasticity::RULE_REGISTRY_VERSION
        }
    }

    /// The rule count `structmut`'s fresh-rule draw ranges over.
    ///
    /// ADR-0027: with the flag set the draw is uniform over the four live
    /// rules rather than over five values one of which is dead. This is the
    /// **only** place the flag changes what a mutation produces; everything
    /// else about a `rule_id` allele - storage, crossover, the mod-5
    /// reduction in `PlasticityGenes::normalized` - is untouched, because
    /// this draw is the only place a fresh id is ever born.
    pub fn plasticity_rule_draw_count(&self) -> u8 {
        let base = if self.plasticity.enabled && self.plasticity.live_rule_zero {
            crate::plasticity::LIVE_RULE_COUNT
        } else {
            crate::genome2::PLASTICITY_RULE_COUNT
        };
        // Condition P widens the draw by one so rule 5 is reachable by the
        // same redraw that reaches every other rule (ADR-0029 section 5).
        base + u8::from(self.observational_offered())
    }

    pub fn world_extent_x_fp(&self) -> i32 {
        (self.cells_x * self.cell_size_m) as i32 * crate::FP_PER_METER
    }

    pub fn world_extent_y_fp(&self) -> i32 {
        (self.cells_y * self.cell_size_m) as i32 * crate::FP_PER_METER
    }

    pub fn cell_size_fp(&self) -> i32 {
        self.cell_size_m as i32 * crate::FP_PER_METER
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfigError {
    /// A Phase 8 physiology parameter outside its documented range.
    PhysiologyRange(&'static str, i64),
    FractionOutOfRange(&'static str, u32),
    WorldDimensions(u32, u32),
    CellSize(u32),
    MaxEntities(u32),
    InitialOrganisms(u32),
    TickLength(u32),
    NonPositive(&'static str),
    Negative(&'static str),
    InitialEnergy(i64),
    Speed(u32),
    CrowdingRadius(u32),
    AgePolicy {
        maturity: u64,
        max_age: u64,
    },
    ReproductionEnergy {
        threshold: i64,
        total_cost: i64,
    },
    LandFractionBounds {
        min: u32,
        max: u32,
    },
    PairingRange(u32),
    PairingEnergy(i64),
    ControllerThreshold(&'static str, i32),
    TurnRate(u32),
    ClusterSample(u32),
    ClimateGeneratorMismatch {
        enabled: bool,
        generator: &'static str,
    },
    TemperatureBounds {
        min: i32,
        max: i32,
    },
    DriftPeriodTooShort {
        period: u64,
        season: u64,
    },
    BiomeThresholdOrder {
        arid: i64,
        forest: i64,
        wetland: i64,
    },
    DemeCount(u32),
    ArchetypeCount(u32),
    ArchetypeOrder {
        index: usize,
        id: u16,
    },
    EmptyArchetypeAffinity {
        id: u16,
    },
    SeededRequiresClimate,
    /// Phase 16: `origin.mode = scratch` without the field stack that
    /// could ever populate it.
    ScratchRequiresAbiogenesis,
    /// Phase 16: the transition section enabled without a gate it needs.
    TransitionRequires(&'static str),
    ContestRequiresPhase2,
    AttackRange(u32),
    MaxCarcasses(u32),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PhysiologyRange(name, value) => {
                write!(formatter, "{name} is outside its supported range: {value}")
            }
            Self::FractionOutOfRange(name, value) => {
                write!(formatter, "{name} must be a Q16 fraction, got {value}")
            }
            Self::WorldDimensions(x, y) => {
                write!(formatter, "unsupported world dimensions {x}x{y} cells")
            }
            Self::CellSize(size) => write!(formatter, "unsupported cell size {size} m"),
            Self::MaxEntities(value) => write!(formatter, "invalid max_entities {value}"),
            Self::InitialOrganisms(value) => {
                write!(formatter, "invalid initial organism count {value}")
            }
            Self::TickLength(value) => write!(formatter, "invalid dt_ms {value}"),
            Self::NonPositive(name) => write!(formatter, "{name} must be positive"),
            Self::Negative(name) => write!(formatter, "{name} must be non-negative"),
            Self::InitialEnergy(value) => write!(formatter, "invalid initial energy {value}"),
            Self::Speed(value) => write!(formatter, "invalid speed_mps_q16 {value}"),
            Self::CrowdingRadius(value) => write!(formatter, "invalid crowding radius {value} m"),
            Self::AgePolicy { maturity, max_age } => write!(
                formatter,
                "max_age_ticks {max_age} must exceed maturity_age_ticks {maturity}"
            ),
            Self::ReproductionEnergy {
                threshold,
                total_cost,
            } => write!(
                formatter,
                "repro_threshold_milli {threshold} must cover offspring cost {total_cost} and fit energy_max"
            ),
            Self::LandFractionBounds { min, max } => write!(
                formatter,
                "min_land_fraction {min} must be below max_land_fraction {max}"
            ),
            Self::PairingRange(value) => write!(formatter, "invalid pairing_range_m {value}"),
            Self::PairingEnergy(value) => {
                write!(formatter, "invalid pairing_energy_threshold_milli {value}")
            }
            Self::ControllerThreshold(name, value) => {
                write!(
                    formatter,
                    "{name} must be a signed Q16 fraction, got {value}"
                )
            }
            Self::TurnRate(value) => write!(formatter, "invalid max_turn_per_tick_bam {value}"),
            Self::ClusterSample(value) => write!(formatter, "invalid cluster_sample_max {value}"),
            Self::ClimateGeneratorMismatch { enabled, generator } => write!(
                formatter,
                "climate.enabled is {enabled} but the generator is {generator}; the climate \
                 section and lifesim-worldgen-v2 must be enabled together"
            ),
            Self::TemperatureBounds { min, max } => write!(
                formatter,
                "temperature_min_milli {min} must be below temperature_max_milli {max}"
            ),
            Self::DriftPeriodTooShort { period, season } => write!(
                formatter,
                "drift period {period} must exceed the season period {season}; a drift term \
                 on a seasonal timescale is a second season, not a long-timescale term"
            ),
            Self::BiomeThresholdOrder {
                arid,
                forest,
                wetland,
            } => write!(
                formatter,
                "biome moisture thresholds must ascend: arid {arid} < forest {forest} < \
                 wetland {wetland}, or a biome is unreachable by construction"
            ),
            Self::DemeCount(value) => write!(formatter, "invalid deme_count {value}"),
            Self::ArchetypeCount(value) => write!(
                formatter,
                "invalid archetype_count {value}; seeded origin needs at least one"
            ),
            Self::ArchetypeOrder { index, id } => write!(
                formatter,
                "archetype {index} has id {id}, which does not ascend; archetypes are sorted \
                 by id so founder allocation order is a function of the set, not the array"
            ),
            Self::EmptyArchetypeAffinity { id } => write!(
                formatter,
                "archetype {id} has an empty biome affinity, so no cell could ever match it"
            ),
            Self::SeededRequiresClimate => formatter.write_str(
                "origin.mode = seeded needs biomes to match against, so the climate section \
                 must be enabled",
            ),
            Self::ScratchRequiresAbiogenesis => formatter.write_str(
                "origin.mode = scratch begins with no organisms, so chemistry, its microbial \
                 half and abiogenesis must all be enabled for anything to arise",
            ),
            Self::TransitionRequires(gate) => write!(
                formatter,
                "the transition section materializes schema-2 organisms with developed bodies, \
                 so {gate} must be true"
            ),
            Self::ContestRequiresPhase2 => formatter.write_str(
                "the contest section wires reserved controller channels, so phase2 must be \
                 enabled",
            ),
            Self::AttackRange(value) => write!(formatter, "invalid reach {value} m"),
            Self::MaxCarcasses(value) => write!(formatter, "invalid max_carcasses {value}"),
        }
    }
}

impl std::error::Error for ConfigError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid_and_hash_is_stable_within_build() {
        let config = SimConfig::phase1_default(42);
        config.validate().unwrap();
        assert_eq!(config.stable_hash(), config.stable_hash());
        let other_seed = SimConfig::phase1_default(43);
        assert_ne!(config.stable_hash(), other_seed.stable_hash());
    }

    #[test]
    fn every_field_affects_the_hash() {
        let base = SimConfig::phase1_default(42);
        let mut changed = base;
        changed.crowding_threshold += 1;
        assert_ne!(base.stable_hash(), changed.stable_hash());
        let mut changed = base;
        changed.reproduction_enabled = false;
        assert_ne!(base.stable_hash(), changed.stable_hash());
    }

    #[test]
    fn the_plasticity_section_is_inert_when_disabled_and_hashed_when_enabled() {
        // D-014's rule at the config layer, and the two halves are separate
        // claims. Disabled, the section must not touch the hash at all - the
        // Phase 9 fixture depends on it. Enabled, every field must reach the
        // hash, or two behaviorally different worlds would share a lineage.
        let base = SimConfig::phase2_default(42);
        let mut with_defaults = base;
        with_defaults.plasticity = PlasticityConfig::plasticity_default();
        assert_eq!(
            base.stable_hash(),
            with_defaults.stable_hash(),
            "a disabled plasticity section reached the config hash"
        );
        // ...and it stays out even when its fields are moved, which is the
        // assertion a `plasticity.enabled` check alone would not make.
        let mut moved = base;
        moved.plasticity.plastic_edge_cost_milli_per_s = 999;
        moved.plasticity.max_plastic_edges = 7;
        // **Both flags, and their absence here was a real defect.** This list
        // held only the two numeric fields, so hoisting either flag's hash
        // block out of the `enabled` gate passed the whole workspace. A
        // mutation run found it, and the failure it hides is the one this
        // test exists for: `validate` gates every plasticity check on
        // `enabled`, so `enabled: false` with a flag set is an *accepted*
        // config that `fields.rs` exposes to campaign sweeps - and under the
        // defect it hashes differently from the same world before the flag
        // existed, which is a fixture moving for a setting nobody switched on.
        //
        // A field added to `PlasticityConfig` belongs on this list.
        moved.plasticity.live_rule_zero = true;
        moved.plasticity.price_moved_edges_only = true;
        assert_eq!(
            base.stable_hash(),
            moved.stable_hash(),
            "a disabled plasticity section reached the config hash through one \
             of its fields; check that every hashed block is inside the \
             `plasticity.enabled` gate"
        );

        let enabled = SimConfig::phase11_default(42);
        enabled.validate().expect("phase11 defaults are valid");
        let reference = enabled.stable_hash();
        assert_ne!(reference, base.stable_hash());
        let mutators: [fn(&mut SimConfig); 4] = [
            |config| config.plasticity.plastic_edge_cost_milli_per_s += 1,
            |config| config.plasticity.max_plastic_edges += 1,
            // The other half of the same claim: enabled, every field must
            // reach the hash, or two behaviourally different worlds share a
            // replay lineage. These two are arms of D-107's 2x2, so a shared
            // lineage between them would be the 2x2 comparing a world to
            // itself.
            |config| config.plasticity.live_rule_zero = true,
            |config| config.plasticity.price_moved_edges_only = true,
        ];
        for (index, mutate) in mutators.into_iter().enumerate() {
            let mut changed = enabled;
            mutate(&mut changed);
            assert_ne!(changed.stable_hash(), reference, "field {index}");
        }
    }

    #[test]
    fn the_plasticity_section_is_validated_where_validation_actually_runs() {
        // D-084: these checks live in `validate_subsystems`, not in
        // `validate_contest`, which early-returns on a disabled contest
        // section. **The contest section is disabled in every config below**,
        // so a check appended to the wrong function would make every
        // assertion here pass vacuously.
        let mut config = SimConfig::phase2_default(1);
        assert!(!config.contest.enabled);
        config.plasticity.enabled = true;
        assert_eq!(
            config.validate(),
            Err(ConfigError::PhysiologyRange(
                "plasticity requires genome2",
                0
            ))
        );

        let mut config = SimConfig::phase11_default(1);
        config.plasticity.max_plastic_edges = 0;
        assert!(matches!(
            config.validate(),
            Err(ConfigError::PhysiologyRange("max_plastic_edges is zero", 0))
        ));

        let mut config = SimConfig::phase11_default(1);
        config.plasticity.max_plastic_edges = config.genome2.caps.max_edges + 1;
        assert!(matches!(
            config.validate(),
            Err(ConfigError::PhysiologyRange(
                "max_plastic_edges exceeds genome2.caps.max_edges",
                _
            ))
        ));

        let mut config = SimConfig::phase11_default(1);
        config.plasticity.plastic_edge_cost_milli_per_s = -1;
        assert_eq!(
            config.validate(),
            Err(ConfigError::Negative("plastic_edge_cost_milli_per_s"))
        );

        // Nonzero Lamarckian inheritance is refused rather than silently
        // accepted, because no policy implements it: a run that looked like
        // the declared experimental condition and was not it is worse than a
        // refused config.
        let mut config = SimConfig::phase11_default(1);
        config.plasticity.lamarckian_fraction_q16 = 1;
        assert!(matches!(
            config.validate(),
            Err(ConfigError::PhysiologyRange(_, 1))
        ));
        let mut config = SimConfig::phase11_default(1);
        config.plasticity.lamarckian_fraction_q16 = Q16_ONE + 1;
        assert!(matches!(
            config.validate(),
            Err(ConfigError::FractionOutOfRange(
                "lamarckian_fraction_q16",
                _
            ))
        ));

        // The budget is `None` when the section is off, which is not the same
        // as a budget of zero: `None` compiles no plastic edge at all.
        assert_eq!(
            SimConfig::phase2_default(1).plasticity_budget(),
            crate::controller2::PlasticityBudget::disabled()
        );
        assert_eq!(
            SimConfig::phase11_default(1).plasticity_budget(),
            crate::controller2::PlasticityBudget::edges(32)
        );
    }

    /// The two accessors that connect ADR-0027's flag to the engine.
    ///
    /// **Both of these survived a mutation run**, and for the same reason:
    /// every other ADR-0027 test builds a `PlasticityBudget` by hand or is
    /// handed the draw count as a literal, so the flag's route from
    /// `SimConfig` into the kernel was pinned nowhere. The arithmetic on
    /// either side was pinned to death; the wiring between them was not.
    ///
    /// What each defect does, stated so a future reader knows what these
    /// assertions are worth:
    ///
    /// - `plasticity_budget()` dropping the flag leaves a configured world
    ///   compiling every plastic edge onto the dead rule, while the config
    ///   hash still moves - so the arm starts its own replay lineage and is
    ///   behaviourally identical to its control. That is the failure the
    ///   increment-A refusal existed to prevent, arriving through the wiring.
    /// - `plasticity_rule_draw_count()` returning 5 while the remap divides
    ///   by 4 sends draw values 0 and 4 both to rule 1, giving plain Hebbian
    ///   40 percent against 20 each for the rest. That is ADR-0027's rejected
    ///   option (b) reconstructed by accident.
    #[test]
    fn the_flag_reaches_the_budget_and_the_draw_count_or_it_reaches_nothing() {
        let mut on = SimConfig::phase11_default(1);
        on.plasticity.live_rule_zero = true;
        let off = SimConfig::phase11_default(1);
        assert!(!off.plasticity.live_rule_zero);

        assert_eq!(
            on.plasticity_budget(),
            crate::controller2::PlasticityBudget::edges(32).with_live_rule_zero()
        );
        assert!(on.plasticity_budget().live_rule_zero);
        assert!(!off.plasticity_budget().live_rule_zero);

        assert_eq!(
            on.plasticity_rule_draw_count(),
            crate::plasticity::LIVE_RULE_COUNT
        );
        assert_eq!(
            off.plasticity_rule_draw_count(),
            crate::genome2::PLASTICITY_RULE_COUNT
        );
        assert_ne!(
            crate::plasticity::LIVE_RULE_COUNT,
            crate::genome2::PLASTICITY_RULE_COUNT,
            "the two counts are equal, so the assertions above cannot tell them apart"
        );

        // Both accessors are gated on the section, not only on the flag. A
        // world with plasticity disabled compiles no plastic edge for the
        // remap to act on, and its draw must stay where it was or the flag
        // would move a world that has no plasticity at all.
        let mut disabled = SimConfig::phase2_default(1);
        disabled.plasticity.live_rule_zero = true;
        assert!(!disabled.plasticity.enabled);
        assert_eq!(
            disabled.plasticity_budget(),
            crate::controller2::PlasticityBudget::disabled()
        );
        assert_eq!(
            disabled.plasticity_rule_draw_count(),
            crate::genome2::PLASTICITY_RULE_COUNT
        );
    }

    #[test]
    fn the_worldmod_section_is_inert_when_disabled_and_hashed_when_enabled() {
        // D-014 at the config layer, and the disabled half is what four
        // fixtures depend on: Phase 1, Phase 2, Phase 9, and Phase 11 were
        // all pinned before this section existed.
        let base = SimConfig::phase1_default(42);
        let mut with_defaults = base;
        with_defaults.worldmod = WorldModConfig::worldmod_default();
        assert_eq!(
            base.stable_hash(),
            with_defaults.stable_hash(),
            "a disabled worldmod section reached the config hash"
        );
        // ...and it stays out even when every field is moved, which is the
        // assertion a `worldmod.enabled` check alone would not make.
        let mut moved = base;
        moved.worldmod.dense_threshold_q16 = 1;
        moved.worldmod.max_capacity_overrides = 9;
        moved.worldmod.patch_enabled = true;
        moved.worldmod.relocate_interval_ticks = 3;
        moved.worldmod.patch_radius_cells = 2;
        moved.worldmod.patch_capacity_scale_q16 = 7;
        assert_eq!(base.stable_hash(), moved.stable_hash());

        let mut enabled = base;
        enabled.worldmod.enabled = true;
        enabled.worldmod.patch_enabled = true;
        enabled.validate().expect("worldmod defaults are valid");
        let reference = enabled.stable_hash();
        assert_ne!(reference, base.stable_hash());
        // Every settable field, one at a time. The control arm differs from
        // the treatment arm in `patch_capacity_scale_q16` alone, so a field
        // that missed the hash here would give the two arms one config hash
        // and one replay lineage - the exact defect the hand-maintained list
        // exists to prevent.
        let mutators: [fn(&mut SimConfig); 8] = [
            |config| config.worldmod.dense_threshold_q16 -= 1,
            |config| config.worldmod.max_traversable_overrides -= 1,
            |config| config.worldmod.max_capacity_overrides -= 1,
            |config| config.worldmod.max_material_overrides -= 1,
            |config| config.worldmod.patch_enabled = false,
            |config| config.worldmod.relocate_interval_ticks += 1,
            |config| config.worldmod.patch_radius_cells -= 1,
            |config| config.worldmod.patch_capacity_scale_q16 = Q16_ONE,
        ];
        for (index, mutate) in mutators.into_iter().enumerate() {
            let mut changed = enabled;
            mutate(&mut changed);
            assert_ne!(changed.stable_hash(), reference, "field {index}");
        }
    }

    #[test]
    fn the_worldmod_section_is_validated_where_validation_actually_runs() {
        // D-084 again: **the contest section is disabled in every config
        // below**, so a check appended to `validate_contest` would make every
        // assertion here pass vacuously. `phase1_default` also leaves phase2,
        // genome2, morphology and plasticity off, so this is the narrowest
        // world the section can be validated in.
        let enabled = || {
            let mut config = SimConfig::phase1_default(1);
            assert!(!config.contest.enabled);
            config.worldmod.enabled = true;
            config.worldmod.patch_enabled = true;
            config
        };
        enabled().validate().expect("valid by default");

        let mut config = enabled();
        config.worldmod.dense_threshold_q16 = Q16_ONE + 1;
        assert_eq!(
            config.validate(),
            Err(ConfigError::FractionOutOfRange(
                "worldmod.dense_threshold_q16",
                Q16_ONE + 1
            ))
        );

        let mut config = enabled();
        config.worldmod.max_material_overrides = 0;
        assert_eq!(
            config.validate(),
            Err(ConfigError::PhysiologyRange("worldmod cap is zero", 0))
        );

        let mut config = enabled();
        config.worldmod.relocate_interval_ticks = 0;
        assert_eq!(
            config.validate(),
            Err(ConfigError::NonPositive("relocate_interval_ticks"))
        );

        let mut config = enabled();
        config.worldmod.patch_radius_cells = MAX_PATCH_RADIUS_CELLS + 1;
        assert!(matches!(
            config.validate(),
            Err(ConfigError::PhysiologyRange("patch_radius_cells", _))
        ));

        let mut config = enabled();
        config.worldmod.patch_capacity_scale_q16 = 0;
        assert!(matches!(
            config.validate(),
            Err(ConfigError::PhysiologyRange("patch_capacity_scale_q16", _))
        ));
        let mut config = enabled();
        config.worldmod.patch_capacity_scale_q16 = MAX_CAPACITY_SCALE_Q16 + 1;
        assert!(matches!(
            config.validate(),
            Err(ConfigError::PhysiologyRange("patch_capacity_scale_q16", _))
        ));

        // A patch that cannot fit under its own layer cap would press
        // against it from the first relocation and look like a smaller
        // patch, which is exactly the invisible-cap failure C12.7 names.
        let mut config = enabled();
        config.worldmod.max_capacity_overrides = 100;
        assert!(matches!(
            config.validate(),
            Err(ConfigError::PhysiologyRange(
                "patch footprint exceeds max_capacity_overrides",
                _
            ))
        ));

        // A patch as wide as the map is a different experiment from the one
        // its author wrote down.
        let mut config = enabled();
        config.cells_x = 16;
        config.cells_y = 16;
        config.worldmod.patch_radius_cells = 8;
        config.worldmod.max_capacity_overrides = 4_096;
        assert!(matches!(
            config.validate(),
            Err(ConfigError::PhysiologyRange(
                "patch_radius_cells spans the map",
                _
            ))
        ));

        // The schedule's own gate: with `patch_enabled` false none of the
        // patch fields is validated, because none of them is read.
        let mut config = enabled();
        config.worldmod.patch_enabled = false;
        config.worldmod.relocate_interval_ticks = 0;
        config.worldmod.patch_radius_cells = 0;
        config.validate().expect("an unread field is not validated");
    }

    #[test]
    fn a_raising_patch_is_refused_while_climate_is_enabled() {
        // The measured interaction defect, made fail-closed. `ClimateWorld::
        // step` trims biomass to the biome capacity on its reclassification
        // cadence and composes no terrain override, so a patch above 1.0 is
        // harvested as fast as it grows: 4x filled to 1.000 of composed
        // capacity without climate and 0.256 with it, over 4,900 ticks.
        let mut config = SimConfig::phase6_default(7);
        config.worldmod.enabled = true;
        config.worldmod.patch_enabled = true;
        config.worldmod.patch_capacity_scale_q16 = 2 * Q16_ONE;
        assert!(matches!(
            config.validate(),
            Err(ConfigError::PhysiologyRange(message, _))
                if message.contains("inert while the climate section")
        ));

        // The zero-magnitude control and every lowering scale stay legal: a
        // composed capacity at or below the biome capacity is never reached
        // by the climate trim, so the interaction cannot arise.
        config.worldmod.patch_capacity_scale_q16 = Q16_ONE;
        config.validate().expect("a 1.0 control is unaffected");
        config.worldmod.patch_capacity_scale_q16 = Q16_ONE / 2;
        config.validate().expect("a lowering patch is unaffected");

        // ...and without climate the raising patch is the ordinary treatment.
        let mut plain = SimConfig::phase1_default(7);
        plain.worldmod.enabled = true;
        plain.worldmod.patch_enabled = true;
        plain.worldmod.patch_capacity_scale_q16 = 2 * Q16_ONE;
        plain.validate().expect("no climate, no interaction");
    }

    #[test]
    fn the_config_scale_ceiling_matches_the_stored_value_domain() {
        // Two statements of the same bound: config validation refuses a
        // scale above `MAX_CAPACITY_SCALE_Q16`, and `value_in_domain`
        // refuses a decoded override above the same number. If they drifted
        // apart, either a legal config would produce an illegal world or a
        // decoded payload could exceed what the arithmetic was checked for.
        assert!(crate::terrainmod::value_in_domain(
            crate::terrainmod::LAYER_CAPACITY_SCALE,
            i64::from(MAX_CAPACITY_SCALE_Q16)
        ));
        assert!(!crate::terrainmod::value_in_domain(
            crate::terrainmod::LAYER_CAPACITY_SCALE,
            i64::from(MAX_CAPACITY_SCALE_Q16) + 1
        ));
    }

    #[test]
    fn invalid_configs_are_rejected() {
        let mut config = SimConfig::phase1_default(1);
        config.cells_x = 4;
        assert_eq!(config.validate(), Err(ConfigError::WorldDimensions(4, 256)));

        let mut config = SimConfig::phase1_default(1);
        config.initial_organisms = config.max_entities + 1;
        assert!(matches!(
            config.validate(),
            Err(ConfigError::InitialOrganisms(_))
        ));

        let mut config = SimConfig::phase1_default(1);
        config.assimilation_q16 = 0;
        assert!(matches!(
            config.validate(),
            Err(ConfigError::FractionOutOfRange("assimilation_q16", 0))
        ));

        let mut config = SimConfig::phase1_default(1);
        config.repro_threshold_milli = 100;
        assert!(matches!(
            config.validate(),
            Err(ConfigError::ReproductionEnergy { .. })
        ));

        let mut config = SimConfig::phase1_default(1);
        config.max_age_ticks = config.maturity_age_ticks;
        assert!(matches!(
            config.validate(),
            Err(ConfigError::AgePolicy { .. })
        ));

        let mut config = SimConfig::phase1_default(1);
        config.dt_ms = 0;
        assert_eq!(config.validate(), Err(ConfigError::TickLength(0)));
    }
}
