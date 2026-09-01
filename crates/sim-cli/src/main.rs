//! Headless Phase 1 runner.
//!
//! All wall-clock, filesystem, and process concerns live here; `sim-core`
//! stays pure. Subcommands:
//!
//! - `run`       advance a world, verify invariants, report a summary
//! - `fixture`   single-line deterministic fixture for clean-process checks
//! - `inspect`   world-generation summary at tick zero
//! - `benchmark` per-phase timing, allocation, and RSS record with provenance
//! - `batch`     run a multi-seed, multi-condition campaign (Phase 5)
//! - `report`    compare conditions in a campaign manifest, or refuse
//! - `fields`    list config fields a campaign may set
//! - `verify-events` replay an event log and report what it contains

use sim_core::{
    BEHAVIOR_POLICY_VERSION, CONTROLLER_POLICY_VERSION, CONTROLLER2_POLICY_VERSION,
    GENOME_POLICY_VERSION, GENOME_SCHEMA_VERSION, GENOME2_POLICY_VERSION, GENOME2_SCHEMA_VERSION,
    InheritanceMode, LearnedEdgeSave, LocusKind, MEIOSIS_POLICY_VERSION,
    PHASE2_BEHAVIOR_POLICY_VERSION, PLASTICITY_POLICY_VERSION, PlasticityGenes,
    RNG_ALGORITHM_VERSION, RULE_HEBBIAN, RULE_REGISTRY_VERSION, STRUCTMUT_POLICY_VERSION,
    SimConfig, TOPOLOGY_ID, TickObserver, TickPhase, WORLDGEN_VERSION, World, analyze,
    registry_versions,
};
use std::alloc::{GlobalAlloc, Layout, System};
use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const DEFAULT_SEED: u64 = 0x5eed_cafe_f00d_beef;

// --- Allocation counting (benchmark evidence) ----------------------------

static ALLOCATION_COUNT: AtomicU64 = AtomicU64::new(0);

struct CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATION_COUNT.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCATION_COUNT.fetch_add(1, Ordering::Relaxed);
        unsafe { System.realloc(pointer, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL_ALLOCATOR: CountingAllocator = CountingAllocator;

// --- Entry ----------------------------------------------------------------

fn main() {
    if let Err(error) = run() {
        eprintln!("lifesim: {error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("run") => command_run(parse_options(args.collect())?),
        Some("fixture") => command_fixture(parse_options(args.collect())?),
        Some("inspect") => command_inspect(parse_options(args.collect())?),
        Some("benchmark") => command_benchmark(parse_options(args.collect())?),
        Some("analyze") => command_analyze(parse_options(args.collect())?),
        Some("verify-save") => command_verify_save(args.collect()),
        Some("compare") => command_compare(args.collect()),
        Some("batch") => command_batch(parse_options(args.collect())?),
        Some("report") => command_report(parse_options(args.collect())?),
        Some("spatial") => command_spatial(parse_options(args.collect())?),
        Some("demography") => command_demography(parse_options(args.collect())?),
        Some("structure") => command_structure(parse_options(args.collect())?),
        Some("morph") => command_morph(parse_options(args.collect())?),
        Some("plasticity") => command_plasticity(parse_options(args.collect())?),
        Some("conjunction") => command_conjunction(parse_options(args.collect())?),
        Some("artifact") => command_artifact(parse_options(args.collect())?),
        Some("social") => command_social(parse_options(args.collect())?),
        Some("social-contrast") => command_social_contrast(parse_options(args.collect())?),
        Some("fields") => command_fields(),
        Some("verify-events") => command_verify_events(args.collect()),
        _ => Err(usage()),
    }
}

fn usage() -> String {
    concat!(
        "usage: lifesim run --ticks N [config flags] [--pause-at T --pause-ticks M] [--metrics-out PATH|-] [--check-interval N] [--save-path P [--compress L]] [--load-save P] [--csv-out P [--csv-interval N]]\n",
        "       lifesim fixture --ticks N [config flags]\n",
        "       lifesim inspect [config flags]\n",
        "       lifesim benchmark --benchmark-id ID --output DIR [config flags] [--warmup N --samples N --ticks-per-sample N]\n",
        "       lifesim analyze --ticks N [config flags]   (requires --phase2)\n",
        "       lifesim verify-save PATH\n",
        "       lifesim compare SUMMARY_A SUMMARY_B\n",
        "       lifesim batch --campaign FILE --output DIR [--workers N] [--no-preflight]\n",
        "       lifesim report --manifest FILE [--baseline CONDITION]\n",
        "       lifesim spatial --manifest FILE --baseline CONDITION [--burn-in N] [--sesoi N] [--analysis-seed HEX] [--power]\n",
        "       lifesim demography --manifest FILE\n",
        "       lifesim structure --manifest FILE --baseline CONDITION\n",
        "       lifesim morph --manifest FILE --baseline CONDITION\n",
        "       lifesim plasticity --manifest FILE --treatment CONDITION --baseline CONDITION [--burn-in N] [--sesoi N] [--analysis-seed HEX]\n",
        "       lifesim conjunction --manifest FILE   (descriptive census; no threshold, no verdict)\n",
        "       lifesim artifact --manifest FILE --treatment A --control C --disabled D   (C12.1-C12.3 against the pre-registered bars)\n",
        "       lifesim social --manifest FILE --epoch N   (C13.1 arrival census + reachability census; no verdict)\n",
        "       lifesim social-contrast --manifest FILE --treatment A --baseline C --epochs N,N,.. --sesoi N [--analysis-seed HEX]   (C13.1 seed-paired contrast)\n",
        "       lifesim fields\n",
        "       lifesim verify-events LOG [--expect-events]\n",
        "config flags: --seed HEX|N --organisms N --max-entities N --cells-x N --cells-y N --dt-ms N --no-reproduction --phase2 --genome2 --plasticity --artifact [--artifact-inert] [--artifact-ephemeral]\n",
        "  --genome2 selects the Phase 9 schema-2 genome and controller; it requires --phase2, and it\n",
        "  pins the structural caps, meiosis mode, and mutation rates literally rather than inheriting\n",
        "  today's defaults, so the Phase 9 fixture cannot move when a default is revised\n",
        "  --plasticity builds the Phase 11 numeric-safety trace: one immortal, sterile organism whose\n",
        "  edges are plastic, for C11.5's 10^6-tick fixed-point trace. It requires --phase2 --genome2,\n",
        "  it pins the entire configuration literally, and it ignores the other config flags on purpose\n",
        "  --artifact builds the Phase 12 artifact trace: a pinned schema-2 world with the artifact section\n",
        "  on and founders scripted to strike, pick up, place, drop and combine, so every object mechanism\n",
        "  runs from tick one. It requires --phase2 --genome2 and pins the whole configuration literally;\n",
        "  --artifact-inert and --artifact-ephemeral select the plan's conditions C and B on top of it\n",
        "  --social builds the Phase 13 social trace on top of the artifact trace: perception, the\n",
        "  signal field, and a scripted rule-5 plastic edge under condition P; --social-scramble is\n",
        "  condition D, --social-strict is condition S (rule 5 withheld), --social-corrupt turns on\n",
        "  the pinned reception corruption\n",
        "run also accepts: --event-log PATH"
    )
    .to_owned()
}

#[derive(Default)]
struct Options {
    seed: Option<u64>,
    organisms: Option<u32>,
    max_entities: Option<u32>,
    cells_x: Option<u32>,
    cells_y: Option<u32>,
    dt_ms: Option<u32>,
    no_reproduction: bool,
    no_preflight: bool,
    phase2: bool,
    genome2: bool,
    plasticity: bool,
    artifact: bool,
    artifact_inert: bool,
    artifact_ephemeral: bool,
    social: bool,
    social_scramble: bool,
    social_strict: bool,
    social_corrupt: bool,
    ticks: Option<u64>,
    pause_at: Option<u64>,
    pause_ticks: Option<u64>,
    check_interval: Option<u64>,
    metrics_out: Option<String>,
    warmup: Option<u64>,
    samples: Option<usize>,
    ticks_per_sample: Option<u64>,
    benchmark_id: Option<String>,
    output: Option<PathBuf>,
    save_path: Option<PathBuf>,
    load_save: Option<PathBuf>,
    compress: Option<i32>,
    csv_out: Option<PathBuf>,
    csv_interval: Option<u64>,
    event_log: Option<PathBuf>,
    campaign: Option<PathBuf>,
    manifest: Option<PathBuf>,
    baseline: Option<String>,
    treatment: Option<String>,
    disabled: Option<String>,
    workers: Option<usize>,
    burn_in: Option<u64>,
    sesoi: Option<i64>,
    analysis_seed: Option<u64>,
    power: bool,
    epoch: Option<u64>,
    epochs: Option<String>,
}

fn parse_options(args: Vec<String>) -> Result<Options, String> {
    let mut options = Options::default();
    let mut index = 0;
    while index < args.len() {
        let name = args[index].as_str();
        if name == "--no-reproduction" {
            options.no_reproduction = true;
            index += 1;
            continue;
        }
        if name == "--phase2" {
            options.phase2 = true;
            index += 1;
            continue;
        }
        if name == "--genome2" {
            options.genome2 = true;
            index += 1;
            continue;
        }
        if name == "--plasticity" {
            options.plasticity = true;
            index += 1;
            continue;
        }
        if name == "--artifact" {
            options.artifact = true;
            index += 1;
            continue;
        }
        if name == "--artifact-inert" {
            options.artifact_inert = true;
            index += 1;
            continue;
        }
        if name == "--artifact-ephemeral" {
            options.artifact_ephemeral = true;
            index += 1;
            continue;
        }
        if name == "--social" {
            options.social = true;
            index += 1;
            continue;
        }
        if name == "--social-scramble" {
            options.social_scramble = true;
            index += 1;
            continue;
        }
        if name == "--social-strict" {
            options.social_strict = true;
            index += 1;
            continue;
        }
        if name == "--social-corrupt" {
            options.social_corrupt = true;
            index += 1;
            continue;
        }
        if name == "--no-preflight" {
            options.no_preflight = true;
            index += 1;
            continue;
        }
        if name == "--power" {
            options.power = true;
            index += 1;
            continue;
        }
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("missing value for {name}\n{}", usage()))?;
        match name {
            "--seed" => options.seed = Some(parse_seed(value)?),
            "--organisms" => options.organisms = Some(parse_number(name, value)?),
            "--max-entities" => options.max_entities = Some(parse_number(name, value)?),
            "--cells-x" => options.cells_x = Some(parse_number(name, value)?),
            "--cells-y" => options.cells_y = Some(parse_number(name, value)?),
            "--dt-ms" => options.dt_ms = Some(parse_number(name, value)?),
            "--ticks" => options.ticks = Some(parse_number(name, value)?),
            "--pause-at" => options.pause_at = Some(parse_number(name, value)?),
            "--pause-ticks" => options.pause_ticks = Some(parse_number(name, value)?),
            "--check-interval" => options.check_interval = Some(parse_number(name, value)?),
            "--metrics-out" => options.metrics_out = Some(value.clone()),
            "--warmup" => options.warmup = Some(parse_number(name, value)?),
            "--samples" => options.samples = Some(parse_number(name, value)?),
            "--ticks-per-sample" => options.ticks_per_sample = Some(parse_number(name, value)?),
            "--benchmark-id" => options.benchmark_id = Some(value.clone()),
            "--output" => options.output = Some(PathBuf::from(value)),
            "--save-path" => options.save_path = Some(PathBuf::from(value)),
            "--load-save" => options.load_save = Some(PathBuf::from(value)),
            "--compress" => options.compress = Some(parse_number(name, value)?),
            "--csv-out" => options.csv_out = Some(PathBuf::from(value)),
            "--csv-interval" => options.csv_interval = Some(parse_number(name, value)?),
            "--event-log" => options.event_log = Some(PathBuf::from(value)),
            "--campaign" => options.campaign = Some(PathBuf::from(value)),
            "--manifest" => options.manifest = Some(PathBuf::from(value)),
            "--baseline" => options.baseline = Some(value.clone()),
            "--treatment" => options.treatment = Some(value.clone()),
            "--disabled" => options.disabled = Some(value.clone()),
            "--workers" => options.workers = Some(parse_number(name, value)?),
            "--burn-in" => options.burn_in = Some(parse_number(name, value)?),
            "--sesoi" => options.sesoi = Some(parse_number::<i64>(name, value)?),
            "--analysis-seed" => options.analysis_seed = Some(parse_seed(value)?),
            "--epoch" => options.epoch = Some(parse_number(name, value)?),
            "--epochs" => options.epochs = Some(value.clone()),
            _ => return Err(format!("unknown option {name}\n{}", usage())),
        }
        index += 2;
    }
    Ok(options)
}

fn parse_number<T: std::str::FromStr>(name: &str, value: &str) -> Result<T, String> {
    value
        .parse()
        .map_err(|_| format!("invalid value for {name}: {value}"))
}

fn parse_seed(value: &str) -> Result<u64, String> {
    value
        .strip_prefix("0x")
        .map_or_else(|| value.parse(), |hex| u64::from_str_radix(hex, 16))
        .map_err(|_| format!("invalid seed: {value}"))
}

fn build_config(options: &Options) -> Result<SimConfig, String> {
    let mut config = SimConfig::phase1_default(options.seed.unwrap_or(DEFAULT_SEED));
    if let Some(organisms) = options.organisms {
        config.initial_organisms = organisms;
    }
    if let Some(max_entities) = options.max_entities {
        config.max_entities = max_entities;
    }
    if let Some(cells_x) = options.cells_x {
        config.cells_x = cells_x;
    }
    if let Some(cells_y) = options.cells_y {
        config.cells_y = cells_y;
    }
    if let Some(dt_ms) = options.dt_ms {
        config.dt_ms = dt_ms;
    }
    if options.no_reproduction {
        config.reproduction_enabled = false;
    }
    if options.phase2 {
        config.phase2.enabled = true;
    }
    if options.genome2 {
        // `validate_subsystems` refuses genome2 without phase2, and its
        // message names a config field rather than the flag that was typed.
        // Say it in the caller's vocabulary instead of letting a validator
        // explain a flag it has never heard of.
        if !options.phase2 {
            return Err(format!("--genome2 requires --phase2\n{}", usage()));
        }
        config.genome2.enabled = true;
        apply_pinned_genome2_policy(&mut config);
    }
    config.validate().map_err(|error| error.to_string())?;
    Ok(config)
}

/// The Phase 9 fixture's genome-2 policy, written out literally.
///
/// **Pinned rather than inherited from `Genome2Config::genome2_default()`,
/// for the reason `experiments/phase9-c91-confirmatory.campaign` states in
/// its own caps block (D-078).** `SimConfig::stable_hash` folds the whole
/// genome2 section in when it is enabled - four policy strings, both
/// registry versions, all seven `GenomeCaps` fields, the meiosis mode and
/// crossover bound, and all eight `MutationConfig` fields. A fixture built
/// from defaults therefore breaks the moment any one of those defaults is
/// revised, and the break would look like a determinism failure rather than
/// what it is. The caps were already restated once, by C9.8's measurement,
/// which is the event this guards against repeating.
///
/// Every value below is today's default. Pinning changes nothing about what
/// the fixture *is*; it changes who decides when it moves.
fn apply_pinned_genome2_policy(config: &mut SimConfig) {
    let caps = &mut config.genome2.caps;
    caps.max_chromosomes = 4;
    caps.max_loci_per_chromosome = 160;
    caps.max_nodes = 160;
    caps.max_edges = 160;
    caps.max_edges_per_node = 32;
    caps.max_genome_bytes = 16_384;
    caps.min_nodes = 2;

    config.genome2.meiosis.mode = InheritanceMode::Meiotic;
    config.genome2.meiosis.max_extra_crossovers = 2;

    let mutation = &mut config.genome2.mutation;
    mutation.point_q16 = 6_554; // 0.10 per birth
    mutation.duplication_q16 = 655; // 0.01
    mutation.deletion_q16 = 655; // 0.01
    mutation.insertion_q16 = 0; // duplication-only is the ADR-0013 baseline
    mutation.transposition_q16 = 328; // 0.005; inert on single-chromosome founders (D-074)
    mutation.regulatory_enabled = true;
    mutation.max_run = 3;
    mutation.point_delta_q16 = 3_277; // 0.05
    // Phase 11, pinned false on both flags for the same D-078 reason. These
    // are today's defaults; pinning them means a future decision to enable
    // plasticity by default cannot move `0x9abc0cd47914127f` by accident.
    // `mutation.plasticity_enabled` is folded into the genome2 block of the
    // config hash when true, and the plasticity section appends its own block
    // when enabled, so leaving either to a default is leaving the fixture to
    // one.
    mutation.plasticity_enabled = false;
    config.plasticity.enabled = false;
}

fn build_world(options: &Options) -> Result<World, String> {
    if options.plasticity {
        return plasticity_trace_world(options);
    }
    if options.social {
        return social_trace_world(options);
    }
    if options.social_scramble || options.social_strict || options.social_corrupt {
        return Err(format!(
            "--social-scramble, --social-strict and --social-corrupt require --social\n{}",
            usage()
        ));
    }
    if options.artifact {
        return artifact_trace_world(options);
    }
    if options.artifact_inert || options.artifact_ephemeral {
        return Err(format!(
            "--artifact-inert and --artifact-ephemeral require --artifact\n{}",
            usage()
        ));
    }
    let config = build_config(options)?;
    World::new(config).map_err(|error| error.to_string())
}

// --- the Phase 12 artifact trace -------------------------------------------

/// The Phase 12 artifact fixture's configuration, pinned literally.
///
/// A population, not a single organism, because what the artifact half adds
/// is interaction: contested pick-ups, objects outliving their makers,
/// carcasses eaten by others. The Phase 9 fixture's genome-2 policy is
/// reused so this world is that lineage plus the section; on top of it the
/// worldmod and contest sections (the yield layer and carcasses live there)
/// and **every `ArtifactConfig` field written out literally**, on D-078's
/// terms: `SimConfig::stable_hash` folds the whole section in when it is
/// enabled, and the section's caps and rates are provisional by the
/// specification's own account, so a fixture built from
/// `ArtifactConfig::artifact_default()` would move the day one is restated
/// from measurement. `binding_q16` is pinned nonzero so the operator D-114
/// added is exercised, and it is hashed only because it is nonzero.
fn artifact_trace_config(seed: u64, inert: bool, ephemeral: bool) -> SimConfig {
    let mut config = SimConfig::phase1_default(seed);
    config.phase2.enabled = true;
    config.genome2.enabled = true;
    apply_pinned_genome2_policy(&mut config);
    config.cells_x = 64;
    config.cells_y = 64;
    config.initial_organisms = 200;
    config.max_entities = 2_000;
    config.genome2.mutation.binding_q16 = 32_768; // 0.5 per birth

    // Physiology, pinned literally (D-078), for one reason: an extrinsic
    // hazard is the only death in an 8,000-tick horizon that leaves energy
    // behind, and a carcass object needs energy to be made of. Starvation
    // leaves none, old age is past the horizon, and nobody attacks unless
    // evolution binds it. Values are today's defaults except the hazard,
    // which is the Phase 11 confirmatory campaign's 13.
    let physiology = &mut config.physiology;
    physiology.enabled = true;
    physiology.allometry_enabled = true;
    physiology.basal_exponent_quarters = 3;
    physiology.thermoregulation_enabled = true;
    physiology.thermal_pref_low_milli = 0;
    physiology.thermal_pref_high_milli = 40_000;
    physiology.thermal_neutral_band_milli = 6_000;
    physiology.thermal_cost_milli_per_s_per_degree = 4;
    physiology.senescence_enabled = true;
    physiology.senescence_onset_ticks = 6_000;
    physiology.senescence_scale_ticks = 12_000;
    physiology.senescence_power = 2;
    physiology.senescence_hazard_q16_per_s = 655;
    physiology.extrinsic_hazard_q16_per_s = 13;
    physiology.juvenile_hazard_multiplier_q16 = 131_072;

    config.contest.enabled = true;
    config.worldmod.enabled = true;
    config.worldmod.patch_enabled = false;
    config.worldmod.dense_threshold_q16 = 32_768;
    config.worldmod.max_traversable_overrides = 4_096;
    config.worldmod.max_capacity_overrides = 4_096;
    config.worldmod.max_material_overrides = 4_096;

    let artifact = &mut config.artifact;
    artifact.enabled = true;
    artifact.inert = inert;
    artifact.ephemeral = ephemeral;
    artifact.max_objects = 4_096;
    artifact.max_objects_per_cell = 8;
    artifact.max_composition_depth = 4;
    artifact.max_composition_breadth = 8;
    artifact.max_held_objects = 1;
    artifact.max_candidates = 8;
    artifact.carry_capacity_milli = 4_000;
    artifact.carry_move_cost_q16 = 65_536;
    artifact.hold_cost_milli_per_s = 20;
    // Costs are a tenth of the shipped defaults, and that is the one place
    // this trace departs from them: its founders fire an action every tick
    // by script, and at 60 and 120 milli per attempt against a basal 10 per
    // tick every one of them starves inside the horizon and the last
    // three-quarters of the run measures an empty world - a fixture that is
    // a control (evidence trap 1). The campaign runs the shipped costs; the
    // fixture pins the arithmetic, and the arithmetic does not care what
    // the price is.
    artifact.action_cost_milli = 6;
    artifact.strike_cost_milli = 12;
    artifact.action_threshold_q16 = 32_768;
    artifact.reach_m = 2;
    artifact.consume_reach_m = 2;
    artifact.perception_range_m = 8;
    artifact.strike_force_q16 = 262_144;
    artifact.strike_mass_reference_milli = 2_000;
    artifact.fracture_margin_q16 = 65_536;
    artifact.max_fragments = 4;
    artifact.min_fragment_mass_milli = 100;
    artifact.joint_floor_q16 = 16_384;
    artifact.blocking_mass_milli = 3_000;
    artifact.terrain_yield_milli = 6_000;
    artifact.extraction_milli = 800;
    artifact.yield_regen_milli = 400;
    artifact.yield_regen_interval_ticks = 600;
    artifact.stone_relative_q16 = 39_322;
    artifact.wood_relative_q16 = 16_384;
    config
}

/// The founders' networks, scripted through the public save path so every
/// object mechanism runs from tick one, on the same terms
/// `plasticity_trace_world` authors the plastic flag: confined to this
/// harness, reachable from no campaign and no default.
///
/// Founder `i` gets one of four scripts by `i % 4` - strike; strike, pick up,
/// place; strike, pick up, combine; pick up, drop - each an always-on output
/// node (bias 8) bound to the channel. Movement stays evolved (`turn` from
/// `energy_fraction`, throttle at its unbound half), so strikers meet their
/// own extractions, pickers meet strikers' objects, and placement and
/// combination happen where the traffic is. Births carry the scripts on to
/// children through the ordinary meiosis, and `bind` adds to them.
fn artifact_trace_world(options: &Options) -> Result<World, String> {
    if !options.genome2 || !options.phase2 {
        return Err(format!(
            "--artifact requires --phase2 --genome2\n{}",
            usage()
        ));
    }
    let config = artifact_trace_config(
        options.seed.unwrap_or(DEFAULT_SEED),
        options.artifact_inert,
        options.artifact_ephemeral,
    );
    config.validate().map_err(|error| error.to_string())?;
    let world = World::new(config).map_err(|error| error.to_string())?;
    let mut state = world.export_state();
    let caps = state.config.genome2.caps;
    let schema2 = state
        .schema2
        .as_mut()
        .ok_or_else(|| "the trace world is not schema 2".to_owned())?;
    const SCRIPTS: [&[u16]; 4] = [
        &[sim_core::CHANNEL_STRIKE],
        &[
            sim_core::CHANNEL_STRIKE,
            sim_core::CHANNEL_PICK_UP,
            sim_core::CHANNEL_PLACE,
        ],
        &[
            sim_core::CHANNEL_STRIKE,
            sim_core::CHANNEL_PICK_UP,
            sim_core::CHANNEL_COMBINE,
        ],
        &[sim_core::CHANNEL_PICK_UP, sim_core::CHANNEL_DROP],
    ];
    for (index, encoded) in schema2.genomes.iter_mut().enumerate() {
        let mut genome = sim_core::Genome2::decode(encoded, &caps)
            .map_err(|error| format!("founder genome does not decode: {error}"))?;
        let script = SCRIPTS[index % SCRIPTS.len()];
        for (salt, &channel) in script.iter().enumerate() {
            let node_id = sim_core::STRUCTURAL_HOMOLOGY_BASE + 50_000 + salt as u32 * 10;
            for haplotype in &mut genome.haplotypes {
                let chromosome = &mut haplotype.chromosomes[0];
                chromosome.push(sim_core::Locus {
                    homology_id: node_id,
                    gene_lineage_id: u64::from(node_id),
                    mutation_event_id: 0,
                    kind: LocusKind::Node {
                        role: sim_core::NodeRole::Output,
                        activation_id: sim_core::Activation::TanhApprox.id(),
                        bias: 8.0,
                        time_constant: 0,
                    },
                });
                chromosome.push(sim_core::Locus {
                    homology_id: node_id + 1,
                    gene_lineage_id: u64::from(node_id + 1),
                    mutation_event_id: 0,
                    kind: LocusKind::IoBinding {
                        node: node_id,
                        channel_id: channel,
                        gain: 1.0,
                    },
                });
                chromosome.sort_unstable_by_key(|locus| locus.homology_id);
            }
        }
        genome
            .validate_structure(&caps)
            .map_err(|error| format!("scripted founder does not validate: {error}"))?;
        *encoded = genome.encode();
        for _ in 0..script.len() {
            schema2.activation_values[index].push(0.0);
            schema2.activation_prior[index].push(0.0);
        }
    }
    World::from_state(state).map_err(|error| format!("scripted founders do not restore: {error}"))
}

// --- the Phase 13 social trace ----------------------------------------------

/// The Phase 13 social fixture's configuration, pinned literally: the Phase
/// 12 artifact trace's world (so every object mechanism keeps running) plus
/// the social section and a plasticity section for the scripted rule-5 edge.
///
/// The chain (`live_rule_zero`) is deliberately **off**: under chain-and-
/// observational the live rule space renumbers (a stored 4 names rule 5),
/// and a fixture should pin the arithmetic in the base id space where the
/// scripted allele's stored value and its expressed rule coincide. The
/// campaign pairs the chain with the phase's arms; the fixture pins the
/// mechanism.
fn social_trace_config(seed: u64, scramble: bool, strict: bool, corrupt: bool) -> SimConfig {
    let mut config = artifact_trace_config(seed, false, false);
    let social = &mut config.social;
    social.enabled = true;
    social.perception_enabled = true;
    social.signal_enabled = true;
    social.scramble_delivery = scramble;
    social.observational_enabled = !strict;
    social.perception_k = 4;
    social.perception_radius_m = 8;
    social.signal_channels = 4;
    social.signal_base_range_m = 4;
    // A tenth-scale cost for the same reason the artifact trace's action
    // costs are: scripted founders emit every tick, and a fixture whose
    // population starves into an empty world is a control (trap 1).
    social.signal_cost_milli = 2;
    social.signal_retain_q16 = 49_152;
    social.signal_corruption_q16 = if corrupt { 8_192 } else { 0 };
    let plasticity = &mut config.plasticity;
    plasticity.enabled = true;
    plasticity.max_plastic_edges = 8;
    plasticity.plastic_edge_cost_milli_per_s = 0;
    config
}

/// The artifact trace's scripted founders, plus one always-on signal
/// emission per founder and one plastic rule-5 edge feeding a hidden node
/// nothing reads - so emission, reception, the field, and the observational
/// rule all run from tick one, and the artifact scripts keep running
/// underneath.
fn social_trace_world(options: &Options) -> Result<World, String> {
    if !options.genome2 || !options.phase2 {
        return Err(format!("--social requires --phase2 --genome2\n{}", usage()));
    }
    let config = social_trace_config(
        options.seed.unwrap_or(DEFAULT_SEED),
        options.social_scramble,
        options.social_strict,
        options.social_corrupt,
    );
    config.validate().map_err(|error| error.to_string())?;
    let world = World::new(config).map_err(|error| error.to_string())?;
    let mut state = world.export_state();
    let caps = state.config.genome2.caps;
    let schema2 = state
        .schema2
        .as_mut()
        .ok_or_else(|| "the trace world is not schema 2".to_owned())?;
    const SCRIPTS: [&[u16]; 4] = [
        &[sim_core::CHANNEL_STRIKE],
        &[
            sim_core::CHANNEL_STRIKE,
            sim_core::CHANNEL_PICK_UP,
            sim_core::CHANNEL_PLACE,
        ],
        &[
            sim_core::CHANNEL_STRIKE,
            sim_core::CHANNEL_PICK_UP,
            sim_core::CHANNEL_COMBINE,
        ],
        &[sim_core::CHANNEL_PICK_UP, sim_core::CHANNEL_DROP],
    ];
    const RULE5_NODE: u32 = sim_core::STRUCTURAL_HOMOLOGY_BASE + 8_000;
    const RULE5_EDGE: u32 = sim_core::STRUCTURAL_HOMOLOGY_BASE + 9_000;
    const FOUNDER_INPUT: u32 = sim_core::STRUCTURAL_HOMOLOGY_BASE + 1_000;
    let mut learn_rows = Vec::new();
    for (index, encoded) in schema2.genomes.iter_mut().enumerate() {
        let mut genome = sim_core::Genome2::decode(encoded, &caps)
            .map_err(|error| format!("founder genome does not decode: {error}"))?;
        let script = SCRIPTS[index % SCRIPTS.len()];
        let mut extra_nodes = 0_usize;
        for (salt, &channel) in script
            .iter()
            .chain(std::iter::once(&sim_core::CHANNEL_SIGNAL_EMIT_BASE))
            .enumerate()
        {
            let node_id = sim_core::STRUCTURAL_HOMOLOGY_BASE + 50_000 + salt as u32 * 10;
            for haplotype in &mut genome.haplotypes {
                let chromosome = &mut haplotype.chromosomes[0];
                chromosome.push(sim_core::Locus {
                    homology_id: node_id,
                    gene_lineage_id: u64::from(node_id),
                    mutation_event_id: 0,
                    kind: LocusKind::Node {
                        role: sim_core::NodeRole::Output,
                        activation_id: sim_core::Activation::TanhApprox.id(),
                        bias: 8.0,
                        time_constant: 0,
                    },
                });
                chromosome.push(sim_core::Locus {
                    homology_id: node_id + 1,
                    gene_lineage_id: u64::from(node_id + 1),
                    mutation_event_id: 0,
                    kind: LocusKind::IoBinding {
                        node: node_id,
                        channel_id: channel,
                        gain: 1.0,
                    },
                });
                chromosome.sort_unstable_by_key(|locus| locus.homology_id);
            }
            extra_nodes += 1;
        }
        // The rule-5 edge: plastic, observational, feeding a hidden node
        // nothing reads, so the rule runs without touching behaviour.
        for haplotype in &mut genome.haplotypes {
            let chromosome = &mut haplotype.chromosomes[0];
            chromosome.push(sim_core::Locus {
                homology_id: RULE5_NODE,
                gene_lineage_id: u64::from(RULE5_NODE),
                mutation_event_id: 0,
                kind: LocusKind::Node {
                    role: sim_core::NodeRole::Hidden,
                    activation_id: sim_core::Activation::TanhApprox.id(),
                    bias: 0.0,
                    time_constant: 0,
                },
            });
            chromosome.push(sim_core::Locus {
                homology_id: RULE5_EDGE,
                gene_lineage_id: u64::from(RULE5_EDGE),
                mutation_event_id: 0,
                kind: LocusKind::Edge {
                    source: FOUNDER_INPUT,
                    target: RULE5_NODE,
                    weight: 1.0,
                    flags: sim_core::EDGE_FLAG_PLASTIC,
                    plasticity: sim_core::PlasticityGenes {
                        rule_id: sim_core::RULE_OBSERVATIONAL,
                        eta: 0.5,
                        // c*y + d beside the social terms, so the edge
                        // learns whenever the rule runs and the P-versus-S
                        // difference is the gate itself.
                        coefficients: [1.0, 1.0, 1.0, 0.1],
                        decay: 0.0,
                        modulator_node: 0,
                    },
                },
            });
            chromosome.sort_unstable_by_key(|locus| locus.homology_id);
        }
        extra_nodes += 1;
        genome
            .validate_structure(&caps)
            .map_err(|error| format!("scripted founder does not validate: {error}"))?;
        *encoded = genome.encode();
        learn_rows.push(vec![sim_core::LearnedEdgeSave {
            edge_homology_id: RULE5_EDGE,
            learned_q16: 0,
            trace_q16: 0,
        }]);
        for _ in 0..extra_nodes {
            schema2.activation_values[index].push(0.0);
            schema2.activation_prior[index].push(0.0);
        }
    }
    if let Some(learn) = state.learn.as_mut() {
        learn.edges = learn_rows;
    }
    World::from_state(state).map_err(|error| format!("scripted founders do not restore: {error}"))
}

// --- the Phase 11 numeric-safety trace --------------------------------------

/// The Phase 11 plasticity trace's configuration, pinned literally.
///
/// **One organism that cannot die and cannot reproduce.** C11.5 asks for a
/// 10^6-tick single-organism plasticity trace that reproduces bit-identically
/// across clean processes, and every word of that is load-bearing:
///
/// - *Single organism*, so what is being pinned is the fixed-point update
///   arithmetic accumulating over a lifetime rather than an ecology. A
///   population fixture pins the same arithmetic diluted through births,
///   deaths and compaction, and when it moves nobody can say which.
/// - *Cannot reproduce*, so the network never changes: structural mutation
///   only runs at birth, so the same plastic edges accumulate for the whole
///   run and the trace is a trace of one individual rather than of a lineage.
/// - *Cannot die*, and this is the artificial part, stated rather than
///   hidden. Basal, movement and crowding costs are zero and the plastic-edge
///   cost is zero, so energy never falls and no starvation is possible. With
///   the shipped costs a lone organism starves within a few thousand ticks
///   and the remaining 99% of the run measures an empty world - a fixture
///   that is a control and does not say so. The cost path is not what this
///   fixture is for: C11.6 measures it, exactly, against a disabled control.
///
/// `max_age_ticks` is set past the horizon for the same reason.
///
/// Everything is written out literally rather than inherited from a default,
/// on D-078's terms: `SimConfig::stable_hash` folds the whole plasticity
/// section in when it is enabled, so a fixture built from
/// `PlasticityConfig::plasticity_default()` moves the moment C11.7 restates
/// `max_plastic_edges` from measurement - which it is under an explicit
/// obligation to do.
fn plasticity_trace_config(seed: u64) -> SimConfig {
    let mut config = SimConfig::phase1_default(seed);
    config.phase2.enabled = true;
    config.genome2.enabled = true;
    apply_pinned_genome2_policy(&mut config);

    config.cells_x = 16;
    config.cells_y = 16;
    config.initial_organisms = 1;
    config.max_entities = 1;
    config.reproduction_enabled = false;
    config.max_age_ticks = 4_000_000;
    config.basal_cost_milli_per_s = 0;
    config.move_cost_milli_per_s = 0;
    config.crowding_cost_milli_per_s = 0;

    config.genome2.mutation.plasticity_enabled = true;
    config.plasticity.enabled = true;
    config.plasticity.plastic_edge_cost_milli_per_s = 0;
    config.plasticity.max_plastic_edges = 32;
    config.plasticity.lamarckian_fraction_q16 = 0;
    config
}

/// The founder's own edges, made plastic, through the public save path.
///
/// **Nothing in the engine writes `EDGE_FLAG_PLASTIC`**: only point mutation
/// can, over generations, which is the correct design - whether an edge is
/// plastic is evolved, not authored. A fixture that waited for that would be
/// a fixture of the mutation operator. So this authors the flag directly, and
/// it is confined to this one determinism harness: no campaign, no default,
/// and no other subcommand can reach it.
///
/// The genes are rule 1 with a nonzero `decay`, and the decay is the point:
/// it is the step that was specified as an arithmetic shift and corrected to
/// a truncation on 2026-08-10, because a shift floors and therefore decays
/// negative learned weights faster than positive ones - and it is integer
/// arithmetic executed two million times here. `eta` is small enough that the
/// value settles well inside the clamp instead of pinning against it, because
/// a trace that saturates in the first hundred ticks spends the remaining
/// 999,900 proving that a constant stays constant.
///
/// # The input binding is moved from `energy_fraction` to `age_fraction`
///
/// **Measured, not assumed.** The first version of this fixture left the
/// founder's binding on channel 1. With every energy cost zeroed, energy pins
/// at capacity, so that input is the constant 1.0, the network reaches a
/// fixed point, and the learned value reaches an equilibrium where decay
/// exactly cancels the delta: `mean_abs_learned_milli` read 964 at 10,000
/// ticks and 964 at 1,000,000. The arithmetic still ran two million times and
/// the checksum was still sensitive to the rounding rule, but the last 99% of
/// the run repeated one step, so a fault that only shows up as slow
/// accumulation would have been invisible.
///
/// `age_fraction` is `age / max_age_ticks`, so over this horizon it sweeps 0
/// to 0.25 monotonically and never repeats a value. The equilibrium therefore
/// moves for the whole run instead of being reached and held. The `d`
/// coefficient is nonzero for the same reason at the other end: with `d = 0`
/// and an input near zero the delta rounds to zero in Q16 and the first tens
/// of thousands of ticks learn nothing at all.
fn plasticity_trace_world(options: &Options) -> Result<World, String> {
    if !options.genome2 || !options.phase2 {
        return Err(format!(
            "--plasticity requires --phase2 --genome2\n{}",
            usage()
        ));
    }
    let config = plasticity_trace_config(options.seed.unwrap_or(DEFAULT_SEED));
    config.validate().map_err(|error| error.to_string())?;
    let world = World::new(config).map_err(|error| error.to_string())?;
    let mut state = world.export_state();
    let caps = state.config.genome2.caps;
    let budget = state.config.plasticity_budget();
    let mut rows: Vec<Vec<LearnedEdgeSave>> = Vec::new();
    let schema2 = state
        .schema2
        .as_mut()
        .ok_or_else(|| "the trace world is not schema 2".to_owned())?;
    for encoded in schema2.genomes.iter_mut() {
        let mut genome = sim_core::Genome2::decode(encoded, &caps)
            .map_err(|error| format!("founder genome does not decode: {error}"))?;
        for haplotype in &mut genome.haplotypes {
            for chromosome in &mut haplotype.chromosomes {
                for locus in chromosome.iter_mut() {
                    match &mut locus.kind {
                        LocusKind::Edge {
                            flags, plasticity, ..
                        } => {
                            *flags |= sim_core::EDGE_FLAG_PLASTIC;
                            *plasticity = PlasticityGenes {
                                rule_id: RULE_HEBBIAN,
                                eta: 0.01,
                                coefficients: [1.0, 0.25, -0.25, 0.1],
                                decay: 0.01,
                                modulator_node: 0,
                            };
                        }
                        // Channel 1 is `energy_fraction` and is constant in a
                        // world with no energy costs; channel 3 is
                        // `age_fraction`, which sweeps monotonically and
                        // never repeats. See the doc comment.
                        LocusKind::IoBinding { channel_id, .. } if *channel_id == 1 => {
                            *channel_id = 3;
                        }
                        _ => {}
                    }
                }
            }
        }
        rows.push(
            sim_core::compile_network_with_budget(&genome.express_network(), budget)
                .map_err(|error| format!("the rewritten founder does not compile: {error:?}"))?
                .plastic_edges
                .iter()
                .map(|edge| LearnedEdgeSave {
                    edge_homology_id: edge.homology_id,
                    learned_q16: 0,
                    trace_q16: 0,
                })
                .collect(),
        );
        *encoded = genome.encode();
    }
    // The learn section has to describe the rewritten plans or `from_state`
    // refuses it, which is the edge-id check doing exactly its job.
    state
        .learn
        .as_mut()
        .ok_or_else(|| "the trace world has no learn section".to_owned())?
        .edges = rows;
    World::from_state(state).map_err(|error| error.to_string())
}

// --- run -------------------------------------------------------------------

fn command_run(options: Options) -> Result<(), String> {
    let ticks = options
        .ticks
        .ok_or_else(|| format!("run requires --ticks\n{}", usage()))?;
    let check_interval = options.check_interval.unwrap_or(1_000).max(1);
    let mut world = match &options.load_save {
        Some(path) => {
            // Branch from a validated snapshot; config flags are ignored in
            // favor of the recorded configuration.
            let (info, world) =
                sim_persist::SnapshotStore::load_world(path).map_err(|error| error.to_string())?;
            eprintln!(
                "loaded save: world {} tick {} config 0x{:016x} build {}",
                info.world_id, info.tick, info.config_hash, info.build_version
            );
            world
        }
        None => build_world(&options)?,
    };

    let mut csv: Option<std::io::BufWriter<File>> = match &options.csv_out {
        Some(path) => {
            let mut writer =
                std::io::BufWriter::new(File::create(path).map_err(|error| error.to_string())?);
            // Versioned export manifest header.
            writeln!(writer, "# lifesim-csv-v1").map_err(|error| error.to_string())?;
            writeln!(
                writer,
                "# seed=0x{:016x} config_hash=0x{:016x} policy={}",
                world.config().world_seed,
                world.config_hash(),
                active_policy(&world)
            )
            .map_err(|error| error.to_string())?;
            writeln!(
                writer,
                "tick,population,births_total,deaths_starvation_total,deaths_old_age_total,paired_births_total,total_biomass_milli,total_energy_milli,max_ancestry_depth"
            )
            .map_err(|error| error.to_string())?;
            Some(writer)
        }
        None => None,
    };
    let csv_interval = options.csv_interval.unwrap_or(100).max(1);

    // Append-only event log (Phase 5, D-019). Absent by default, so a run
    // without `--event-log` takes the exact Phase 4 path.
    let mut recorder = match &options.event_log {
        Some(path) => {
            let writer = sim_persist::EventLogWriter::create(
                path,
                &sim_persist::EventLogInfo {
                    format_version: sim_persist::EVENT_LOG_FORMAT_VERSION,
                    world_id: 1,
                    seed: world.config().world_seed,
                    config_hash: world.config_hash(),
                    event_schema_version: sim_core::EVENT_SCHEMA_VERSION,
                    max_events_per_tick: sim_core::MAX_EVENTS_PER_TICK as u32,
                    start_tick: world.tick_number(),
                    build_version: sim_persist::BUILD_VERSION.to_owned(),
                },
            )
            .map_err(|error| error.to_string())?;
            Some(sim_persist::EventLogRecorder::new(writer))
        }
        None => None,
    };

    let mut paused_ticks_verified = 0_u64;
    let mut tick_durations = Vec::with_capacity(ticks.min(1_000_000) as usize);
    let wall_started = Instant::now();

    for tick in 0..ticks {
        if options.pause_at == Some(tick) {
            let pause_ticks = options.pause_ticks.unwrap_or(10);
            world.set_paused(true);
            let checksum_before = world.state_checksum();
            let tick_before = world.tick_number();
            for _ in 0..pause_ticks {
                world.step();
            }
            if world.tick_number() != tick_before || world.state_checksum() != checksum_before {
                return Err("paused world advanced state; determinism violation".to_owned());
            }
            paused_ticks_verified = pause_ticks;
            world.set_paused(false);
        }
        let started = Instant::now();
        world.step();
        tick_durations.push(started.elapsed());
        // Recorded immediately: the next step clears the buffer, and the
        // kernel's own state never depends on whether anyone read it.
        if let Some(recorder) = recorder.as_mut() {
            recorder.record(&world).map_err(|error| error.to_string())?;
        }
        if (tick + 1) % check_interval == 0 {
            world.check_invariants().map_err(|violation| {
                format!("invariant violation at tick {}: {violation}", tick + 1)
            })?;
        }
        if let Some(writer) = csv.as_mut()
            && (tick + 1) % csv_interval == 0
        {
            let sample = world.metrics();
            writeln!(
                writer,
                "{},{},{},{},{},{},{},{},{}",
                sample.tick,
                sample.population,
                sample.births_total,
                sample.deaths_starvation_total,
                sample.deaths_old_age_total,
                sample.paired_births_total,
                sample.total_biomass_milli,
                sample.total_energy_milli,
                sample.max_ancestry_depth
            )
            .map_err(|error| error.to_string())?;
        }
    }
    if let Some(mut writer) = csv.take() {
        writer.flush().map_err(|error| error.to_string())?;
    }
    let event_log_offset = match recorder.as_mut() {
        Some(recorder) => {
            recorder
                .writer_mut()
                .sync()
                .map_err(|error| error.to_string())?;
            let writer = recorder.writer();
            eprintln!(
                "event log: {} bytes, {} segments, {} events, {} dropped",
                writer.offset(),
                writer.segments(),
                writer.events(),
                writer.dropped()
            );
            writer.offset()
        }
        None => 0,
    };
    let wall_elapsed = wall_started.elapsed();
    world
        .check_invariants()
        .map_err(|violation| format!("final invariant violation: {violation}"))?;

    if let Some(path) = &options.save_path {
        let bytes = sim_persist::encode_snapshot(
            &world.export_state(),
            1,
            0,
            world.state_checksum(),
            sim_persist::BUILD_VERSION,
            event_log_offset,
            Some(options.compress.unwrap_or(3)),
        )
        .map_err(|error| error.to_string())?;
        fs::write(path, &bytes).map_err(|error| error.to_string())?;
        eprintln!(
            "saved {} bytes to {} (state 0x{:016x})",
            bytes.len(),
            path.display(),
            world.state_checksum()
        );
    }

    if let Some(target) = options.metrics_out.as_deref() {
        let text = prometheus_text(&world, &tick_durations, wall_elapsed);
        if target == "-" {
            print!("{text}");
        } else {
            fs::write(target, text).map_err(|error| error.to_string())?;
        }
    }

    let metrics = world.metrics();
    println!(
        concat!(
            "{{\"run_schema_version\":2,",
            "\"behavior_policy\":\"{}\",\"rng_algorithm\":\"{}\",\"worldgen_version\":\"{}\",",
            "\"seed\":\"0x{:016x}\",\"config_hash\":\"0x{:016x}\",",
            "\"ticks_requested\":{},\"final_tick\":{},",
            "\"population\":{},\"births_total\":{},",
            "\"deaths_starvation_total\":{},\"deaths_old_age_total\":{},",
            "\"capacity_rejections_total\":{},\"dropped_events_total\":{},",
            "\"total_energy_milli\":{},\"total_biomass_milli\":{},",
            "\"extinct\":{},\"paused_ticks_verified\":{},",
            "\"phase2_enabled\":{},\"paired_births_total\":{},",
            "\"pair_rejected_capacity_total\":{},\"pair_rejected_placement_total\":{},",
            "\"pair_rejected_energy_total\":{},\"controller_faults_total\":{},",
            "\"max_ancestry_depth\":{},",
            "\"terrain_checksum\":\"0x{:016x}\",\"state_checksum\":\"0x{:016x}\",",
            "\"invariants\":\"ok\"}}"
        ),
        active_policy(&world),
        RNG_ALGORITHM_VERSION,
        WORLDGEN_VERSION,
        world.config().world_seed,
        world.config_hash(),
        ticks,
        world.tick_number(),
        metrics.population,
        metrics.births_total,
        metrics.deaths_starvation_total,
        metrics.deaths_old_age_total,
        metrics.capacity_rejections_total,
        metrics.dropped_events_total,
        metrics.total_energy_milli,
        metrics.total_biomass_milli,
        metrics.extinct,
        paused_ticks_verified,
        metrics.phase2_enabled,
        metrics.paired_births_total,
        metrics.pair_rejected_capacity_total,
        metrics.pair_rejected_placement_total,
        metrics.pair_rejected_energy_total,
        metrics.controller_faults_total,
        metrics.max_ancestry_depth,
        world.terrain().terrain_checksum,
        world.state_checksum()
    );
    Ok(())
}

// --- batch, report, fields, verify-events (Phase 5) -------------------------

/// Run a declared campaign across every (condition, seed) unit and write a
/// manifest. Worker count is execution policy only: A5.2 requires the
/// manifest to be identical at every worker count.
fn command_batch(options: Options) -> Result<(), String> {
    let campaign_path = options
        .campaign
        .as_ref()
        .ok_or_else(|| format!("batch requires --campaign\n{}", usage()))?;
    let output = options
        .output
        .as_ref()
        .ok_or_else(|| format!("batch requires --output\n{}", usage()))?;
    let source = fs::read_to_string(campaign_path)
        .map_err(|error| format!("{}: {error}", campaign_path.display()))?;
    let mut campaign =
        sim_experiment::Campaign::parse(&source).map_err(|error| error.to_string())?;
    if let Some(workers) = options.workers {
        campaign.workers = workers.clamp(1, 64);
    }
    fs::create_dir_all(output).map_err(|error| error.to_string())?;

    eprintln!(
        "campaign {} hash 0x{:016x}: {} conditions x {} seeds = {} worlds, {} ticks each, {} workers",
        campaign.id,
        campaign.stable_hash(),
        campaign.conditions.len(),
        campaign.seeds.len(),
        campaign.run_count(),
        campaign.ticks,
        campaign.workers
    );

    // Refuse before spending compute: a campaign that can only build some
    // of its declared worlds would execute a different design from the one
    // it declares, and the seeds it dropped were selected by terrain, not
    // at random.
    if !options.no_preflight {
        let failures = sim_experiment::preflight(&campaign);
        if !failures.is_empty() {
            for failure in failures.iter().take(20) {
                eprintln!(
                    "preflight: {} seed 0x{:016x}: {}",
                    failure.condition, failure.seed, failure.reason
                );
            }
            return Err(format!(
                "{} of {} declared worlds cannot be generated; fix the seed set or the \
                 world parameters rather than running a design you did not declare",
                failures.len(),
                campaign.run_count()
            ));
        }
    }

    let completed = std::sync::atomic::AtomicUsize::new(0);
    let total = campaign.run_count();
    let progress = std::sync::Arc::new(
        move |unit: &sim_experiment::RunUnit,
              result: &Result<sim_experiment::RunResult, String>| {
            let done = completed.fetch_add(1, Ordering::Relaxed) + 1;
            match result {
                Ok(run) => eprintln!(
                    "[{done}/{total}] {} seed 0x{:016x} state 0x{:016x}",
                    unit.condition, unit.seed, run.state_checksum
                ),
                Err(error) => eprintln!(
                    "[{done}/{total}] {} seed 0x{:016x} FAILED: {error}",
                    unit.condition, unit.seed
                ),
            }
        },
    );

    let started = Instant::now();
    let results = sim_experiment::run_campaign(
        &campaign,
        &sim_experiment::SchedulerOptions {
            workers: campaign.workers,
            output_dir: Some(output.clone()),
            progress: Some(progress),
        },
    );
    let elapsed = started.elapsed();

    let units = sim_experiment::enumerate_units(&campaign);
    let mut runs = Vec::new();
    let mut failed = Vec::new();
    for (unit, result) in units.iter().zip(results) {
        match result {
            Ok(run) => runs.push(run),
            Err(reason) => failed.push(sim_experiment::FailedRun {
                index: unit.index,
                condition: unit.condition.clone(),
                seed: unit.seed,
                reason,
            }),
        }
    }

    let workers = campaign.workers;
    let manifest = sim_experiment::Manifest {
        campaign,
        campaign_source: source,
        build_version: sim_persist::BUILD_VERSION.to_owned(),
        behavior_policy_versions: vec![
            BEHAVIOR_POLICY_VERSION.to_owned(),
            PHASE2_BEHAVIOR_POLICY_VERSION.to_owned(),
        ],
        rng_algorithm_version: RNG_ALGORITHM_VERSION.to_owned(),
        worldgen_version: WORLDGEN_VERSION.to_owned(),
        genome_schema_version: GENOME_SCHEMA_VERSION,
        event_schema_version: sim_core::EVENT_SCHEMA_VERSION,
        analysis_versions: vec![sim_core::SIMILARITY_ALGORITHM_VERSION.to_owned()],
        workers,
        runs,
        failed,
    };
    let manifest_path = output.join("manifest.txt");
    fs::write(&manifest_path, manifest.render()).map_err(|error| error.to_string())?;

    let total_ticks = manifest.campaign.ticks * manifest.runs.len() as u64;
    let seconds = elapsed.as_secs_f64().max(f64::MIN_POSITIVE);
    println!(
        concat!(
            "{{\"batch_schema_version\":1,\"campaign\":\"{}\",",
            "\"campaign_hash\":\"0x{:016x}\",\"worlds\":{},\"failed\":{},",
            "\"ticks_per_world\":{},\"workers\":{},",
            "\"wall_seconds\":{:.3},\"aggregate_ticks_per_second\":{:.1},",
            "\"manifest\":\"{}\"}}"
        ),
        manifest.campaign.id,
        manifest.campaign.stable_hash(),
        manifest.runs.len(),
        manifest.failed.len(),
        manifest.campaign.ticks,
        workers,
        elapsed.as_secs_f64(),
        total_ticks as f64 / seconds,
        manifest_path.display()
    );
    if !manifest.failed.is_empty() {
        // A failed world is reported, never silently dropped.
        for failure in &manifest.failed {
            eprintln!(
                "failed: {} seed 0x{:016x}: {}",
                failure.condition, failure.seed, failure.reason
            );
        }
        return Err(format!("{} worlds failed", manifest.failed.len()));
    }
    Ok(())
}

/// Compare conditions in a manifest, or refuse with the reason.
fn command_report(options: Options) -> Result<(), String> {
    let path = options
        .manifest
        .as_ref()
        .ok_or_else(|| format!("report requires --manifest\n{}", usage()))?;
    let text = fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let manifest = sim_experiment::Manifest::parse(&text).map_err(|error| error.to_string())?;
    let report = sim_experiment::compare(&manifest, options.baseline.as_deref())
        .map_err(|refusal| refusal.to_string())?;
    print!("{}", report.render());
    Ok(())
}

/// Phase 7 C7.1: the world-level spatial-structure indices and their
/// seed-paired contrasts against a baseline condition.
///
/// Every condition other than the baseline is contrasted against it. The
/// analysis plan (burn-in, scales, SESOI, decision bar, bootstrap seed) is
/// echoed into the report so a reader can check it was not tuned after the
/// data was seen.
fn command_spatial(options: Options) -> Result<(), String> {
    let path = options
        .manifest
        .as_ref()
        .ok_or_else(|| format!("spatial requires --manifest\n{}", usage()))?;
    let baseline = options
        .baseline
        .as_deref()
        .ok_or_else(|| format!("spatial requires --baseline\n{}", usage()))?;
    let text = fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let manifest = sim_experiment::Manifest::parse(&text).map_err(|error| error.to_string())?;
    let directory = path.parent().unwrap_or_else(|| std::path::Path::new("."));

    let mut plan = sim_analysis::SpatialPlan::default();
    if let Some(burn_in) = options.burn_in {
        plan.burn_in_ticks = burn_in;
    }
    if let Some(sesoi) = options.sesoi {
        plan.sesoi_milli = sesoi;
    }
    if let Some(seed) = options.analysis_seed {
        plan.analysis_seed = seed;
    }

    let worlds = sim_analysis::analyse_worlds(&manifest, directory, &plan)
        .map_err(|error| error.to_string())?;
    if !manifest
        .campaign
        .conditions
        .iter()
        .any(|condition| condition.name == baseline)
    {
        return Err(format!("no condition named '{baseline}' in this campaign"));
    }

    let contrasts: Vec<sim_analysis::Contrast> = manifest
        .campaign
        .conditions
        .iter()
        .filter(|condition| condition.name != baseline)
        .map(|condition| sim_analysis::contrast(&worlds, &condition.name, baseline, &plan))
        .collect();

    let report = sim_analysis::SpatialReport {
        campaign_id: manifest.campaign.id.clone(),
        plan,
        index_version: sim_analysis::SPATIAL_INDEX_VERSION.to_owned(),
        stats_version: sim_analysis::PAIRED_STATS_VERSION.to_owned(),
        worlds: worlds.clone(),
        contrasts,
    };
    print!("{}", sim_analysis::render(&report));

    if options.power {
        for condition in manifest
            .campaign
            .conditions
            .iter()
            .filter(|condition| condition.name != baseline)
        {
            for (label, kind) in [
                ("aggregation", sim_analysis::IndexKind::Aggregation),
                ("encounter", sim_analysis::IndexKind::Encounter),
            ] {
                let pairs = sim_analysis::pairs_for(&worlds, &condition.name, baseline, kind);
                let rate = sim_analysis::observed_success_rate_milli(
                    &pairs,
                    plan.sesoi_milli,
                    plan.direction,
                );
                let curve = sim_analysis::power_curve(
                    &pairs,
                    plan.sesoi_milli,
                    plan.direction,
                    &[30, 40, 50, 60, 80, 100, 120, 150, 200],
                    plan.required_worlds,
                    plan.analysis_seed,
                );
                for point in &curve {
                    println!(
                        "power index={} treatment={} control={} pilot_pairs={} \
                         pilot_rate_milli={} worlds={} required={} power_milli={}",
                        label,
                        condition.name,
                        baseline,
                        pairs.len(),
                        rate,
                        point.worlds,
                        point.required,
                        point.power_milli,
                    );
                }
            }
        }
    }
    Ok(())
}

/// Phase 10 C10.3 and C10.6: morphological evolution against a
/// fixed-morphology control.
///
/// The baseline condition is the fixed-morphology control C10.6 contrasts
/// against. C10.3's three clauses are reported per world and per condition,
/// with the count of worlds that had no morphological variance separated
/// out - those worlds cannot speak to consequence and are not counted as
/// having refuted it.
fn command_morph(options: Options) -> Result<(), String> {
    let path = options
        .manifest
        .as_ref()
        .ok_or_else(|| format!("morph requires --manifest\n{}", usage()))?;
    let baseline = options
        .baseline
        .as_deref()
        .ok_or_else(|| format!("morph requires --baseline\n{}", usage()))?;
    let text = fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let manifest = sim_experiment::Manifest::parse(&text).map_err(|error| error.to_string())?;
    let directory = path.parent().unwrap_or_else(|| std::path::Path::new("."));
    if !manifest
        .campaign
        .conditions
        .iter()
        .any(|condition| condition.name == baseline)
    {
        return Err(format!("no condition named '{baseline}' in this campaign"));
    }

    let mut plan = sim_analysis::MorphPlan::default();
    if let Some(seed) = options.analysis_seed {
        plan.analysis_seed = seed;
    }
    if let Some(sesoi) = options.sesoi {
        plan.stability_tolerance_milli = sesoi;
    }

    let mut per_world: Vec<(String, Vec<sim_analysis::WorldMorph>)> = Vec::new();
    for condition in &manifest.campaign.conditions {
        let mut worlds = Vec::new();
        for run in manifest.runs_for(&condition.name) {
            let stem = sim_experiment::run_stem(&run.condition, run.seed);
            let median_lifespan_ticks = fs::read(directory.join(format!("{stem}.alev")))
                .ok()
                .and_then(|bytes| sim_persist::decode_log_events(&bytes).ok())
                .map(|(_, events)| sim_analysis::world_demography(&events).median_lifespan_ticks)
                .unwrap_or(0);
            // **A restore failure is reported, never swallowed.** An
            // earlier version used `.ok()` here, so a world whose snapshot
            // could not be restored produced an empty census - and the
            // report said "no organism was mature" when the truth was "the
            // config did not survive the codec". The two are opposite
            // conclusions and only one of them is about biology.
            let census = match fs::read(directory.join(format!("{stem}.alif"))) {
                Ok(bytes) => {
                    let (_, state) = sim_persist::decode_snapshot(&bytes)
                        .map_err(|error| format!("{stem}: decode: {error}"))?;
                    let world = sim_core::World::from_state(state)
                        .map_err(|error| format!("{stem}: restore: {error}"))?;
                    if world.morphology_enabled() != (run.mean_modules_milli > 0) {
                        return Err(format!(
                            "{stem}: the restored world's morphology is {} but the manifest \
                             recorded mean_modules_milli={} - the config did not survive the \
                             snapshot",
                            world.morphology_enabled(),
                            run.mean_modules_milli
                        ));
                    }
                    world.morphology_census()
                }
                Err(error) => return Err(format!("{stem}: {error}")),
            };
            let (compared, rho_milli, null_p95_milli, no_variance) =
                sim_analysis::consequence_of(&census, plan.analysis_seed ^ run.seed);
            let series = fs::read_to_string(directory.join(format!("{stem}.almo")))
                .map(|text| sim_analysis::parse_series(&text))
                .unwrap_or_default();
            let diverged = |sample: &sim_analysis::MorphSample| {
                sample.distinct > 1 && sample.median_modules > plan.founder_modules
            };
            let halfway = series.get(series.len() / 2).copied();
            worlds.push(sim_analysis::WorldMorph {
                seed: run.seed,
                population: run.population,
                extinct: run.extinct,
                median_modules: run.median_modules,
                mean_modules_milli: run.mean_modules_milli,
                distinct_morphologies: run.distinct_morphologies,
                median_lifespan_ticks,
                compared,
                rho_milli,
                null_p95_milli,
                no_variance,
                diverged_at_halfway: halfway.as_ref().is_some_and(diverged),
                diverged_at_end: series.last().is_some_and(diverged),
                series_samples: series.len(),
            });
        }
        per_world.push((condition.name.clone(), worlds));
    }

    let outcomes: Vec<sim_analysis::MorphOutcome> = per_world
        .iter()
        .map(|(name, worlds)| sim_analysis::summarise_morph(name, worlds, &plan))
        .collect();
    let control = per_world
        .iter()
        .find(|(name, _)| name == baseline)
        .map(|(_, worlds)| worlds.clone())
        .unwrap_or_default();
    let mut stabilities = Vec::new();
    for (name, worlds) in &per_world {
        if name == baseline {
            continue;
        }
        for (label, quantity) in [
            (
                "population",
                (|world: &sim_analysis::WorldMorph| world.population as i64)
                    as fn(&sim_analysis::WorldMorph) -> i64,
            ),
            ("median_lifespan", |world: &sim_analysis::WorldMorph| {
                world.median_lifespan_ticks as i64
            }),
        ] {
            let pairs = sim_analysis::morph_pairs(worlds, &control, quantity);
            let (within, paired) = sim_analysis::morph_stability(&pairs, &plan);
            stabilities.push((name.clone(), label.to_owned(), within, paired));
        }
    }

    print!(
        "{}",
        sim_analysis::render_morph(
            &manifest.campaign.id,
            &plan,
            &per_world,
            &outcomes,
            &stabilities
        )
    );
    Ok(())
}

/// Phase 11 C11.1 and C11.2: within-lifetime behavioural change, and whether
/// plasticity is under selection against a neutral marker.
///
/// Two conditions are named rather than one. `--baseline` is the control the
/// criterion is decided against - the plasticity-disabled arm under the same
/// environmental schedule - and `--treatment` is the arm the bar applies to.
/// Every condition in the campaign is still reduced and printed, so the
/// E-stationary arms and any other contrast a reader wants are in the report;
/// only the decision is restricted to the pre-registered pair.
///
/// **A restore or decode failure is reported, never swallowed** - the
/// discipline `command_morph` records. A world whose snapshot did not survive
/// the codec would otherwise produce an empty allele census, and the report
/// would say "plasticity never arose" when the truth was "the config did not
/// survive the codec". The two are opposite conclusions and only one of them
/// is about biology. The config check here is the strongest available: the
/// restored config's own stable hash must equal the hash the manifest recorded
/// for the run, which covers `plasticity.enabled` and
/// `probe.marker_locus_enabled` along with every other field.
fn command_plasticity(options: Options) -> Result<(), String> {
    let path = options
        .manifest
        .as_ref()
        .ok_or_else(|| format!("plasticity requires --manifest\n{}", usage()))?;
    let baseline = options
        .baseline
        .as_deref()
        .ok_or_else(|| format!("plasticity requires --baseline\n{}", usage()))?;
    let treatment_name = options
        .treatment
        .as_deref()
        .ok_or_else(|| format!("plasticity requires --treatment\n{}", usage()))?;
    let text = fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let manifest = sim_experiment::Manifest::parse(&text).map_err(|error| error.to_string())?;
    let directory = path.parent().unwrap_or_else(|| std::path::Path::new("."));
    for name in [baseline, treatment_name] {
        if !manifest
            .campaign
            .conditions
            .iter()
            .any(|condition| condition.name == name)
        {
            return Err(format!("no condition named '{name}' in this campaign"));
        }
    }
    if baseline == treatment_name {
        return Err("--treatment and --baseline name the same condition".to_owned());
    }

    let mut plan = sim_analysis::PlasticityPlan::default();
    if let Some(seed) = options.analysis_seed {
        plan.analysis_seed = seed;
    }
    if let Some(burn_in) = options.burn_in {
        plan.burn_in_ticks = burn_in;
    }
    if let Some(sesoi) = options.sesoi {
        plan.drift_margin_milli = sesoi;
    }

    let mut per_world: Vec<(String, Vec<sim_analysis::WorldPlasticity>)> = Vec::new();
    for condition in &manifest.campaign.conditions {
        let mut worlds = Vec::new();
        for run in manifest.runs_for(&condition.name) {
            let stem = sim_experiment::run_stem(&run.condition, run.seed);
            let config = manifest
                .campaign
                .config_for(condition, run.seed)
                .map_err(|error| format!("{stem}: {error}"))?;

            // --- the action series -------------------------------------
            let action_path = directory.join(format!("{stem}.alac"));
            let bytes = fs::read(&action_path)
                .map_err(|error| format!("{}: {error}", action_path.display()))?;
            let scan = sim_persist::decode_action(&bytes)
                .map_err(|error| format!("{stem}: decode actions: {error}"))?;
            if scan.info.config_hash != run.config_hash {
                return Err(format!(
                    "{stem}: the action log's config hash {:#018x} is not the run's {:#018x}",
                    scan.info.config_hash, run.config_hash
                ));
            }
            if scan.info.terrain_checksum != run.terrain_checksum {
                return Err(format!(
                    "{stem}: the action log's terrain {:#018x} is not the run's {:#018x}",
                    scan.info.terrain_checksum, run.terrain_checksum
                ));
            }
            if scan.samples.len() as u64 != run.action_samples {
                return Err(format!(
                    "{stem}: the action log holds {} samples, the manifest recorded {}",
                    scan.samples.len(),
                    run.action_samples
                ));
            }
            let relocate = if config.worldmod.enabled && config.worldmod.patch_enabled {
                config.worldmod.relocate_interval_ticks
            } else {
                0
            };
            let shift =
                sim_analysis::world_shift(&scan, relocate, &plan, plan.analysis_seed ^ run.seed);

            // --- the final genomes -------------------------------------
            let snapshot_path = directory.join(format!("{stem}.alif"));
            let snapshot = fs::read(&snapshot_path)
                .map_err(|error| format!("{}: {error}", snapshot_path.display()))?;
            let (_, state) = sim_persist::decode_snapshot(&snapshot)
                .map_err(|error| format!("{stem}: decode snapshot: {error}"))?;
            let restored_hash = state.config.stable_hash();
            if restored_hash != run.config_hash {
                return Err(format!(
                    "{stem}: the restored config hashes {restored_hash:#018x} but the manifest \
                     recorded {:#018x} - the config did not survive the snapshot, so a zero \
                     allele census here would mean a codec defect and not an absence of \
                     plasticity (plasticity.enabled={}, probe.marker_locus_enabled={})",
                    run.config_hash,
                    state.config.plasticity.enabled,
                    state.config.probe.marker_locus_enabled
                ));
            }
            let caps = state.config.genome2.caps;
            let genomes: Vec<sim_core::Genome2> = match state.schema2.as_ref() {
                Some(schema2) => schema2
                    .genomes
                    .iter()
                    .map(|encoded| {
                        sim_core::Genome2::decode(encoded, &caps)
                            .map_err(|error| format!("{stem}: decode genome: {error}"))
                    })
                    .collect::<Result<Vec<_>, String>>()?,
                None => {
                    return Err(format!(
                        "{stem}: the snapshot carries no schema-2 section, so there is no \
                         genome to read eta or the marker from"
                    ));
                }
            };
            let alleles = sim_analysis::allele_census(&genomes);
            let world = sim_core::World::from_state(state)
                .map_err(|error| format!("{stem}: restore: {error}"))?;
            let metrics = world.metrics();

            worlds.push(sim_analysis::WorldPlasticity {
                condition: run.condition.clone(),
                seed: run.seed,
                population: run.population,
                extinct: run.extinct,
                shift,
                alleles,
                plastic_edges_total: metrics.plastic_edges_total,
                plasticity_updates_total: metrics.plasticity_updates_total,
                plasticity_updates_applied: metrics.plasticity_updates_applied,
                mean_abs_learned_milli: metrics.mean_abs_learned_milli,
            });
        }
        per_world.push((condition.name.clone(), worlds));
    }

    let outcomes: Vec<sim_analysis::PlasticityOutcome> = per_world
        .iter()
        .map(|(name, worlds)| sim_analysis::summarise_plasticity(name, worlds, &plan))
        .collect();
    let control_worlds = per_world
        .iter()
        .find(|(name, _)| name == baseline)
        .map(|(_, worlds)| worlds.clone())
        .unwrap_or_default();
    let mut contrasts = Vec::new();
    for (name, worlds) in &per_world {
        if name == baseline {
            continue;
        }
        for (label, quantity) in [
            (
                "rho_milli",
                (|world: &sim_analysis::WorldPlasticity| {
                    world.shift.as_ref().map_or(0, |shift| shift.rho_milli)
                }) as fn(&sim_analysis::WorldPlasticity) -> i64,
            ),
            ("event_distance_milli", |world| {
                world
                    .shift
                    .as_ref()
                    .map_or(0, |shift| shift.median_event_milli)
            }),
            ("eta_excess_milli", |world| world.alleles.eta_excess_milli()),
            ("plastic_excess_milli", |world| {
                world.alleles.plastic_excess_milli()
            }),
        ] {
            let pairs = sim_analysis::plasticity_pairs(worlds, &control_worlds, quantity);
            contrasts.push((
                name.clone(),
                baseline.to_owned(),
                label.to_owned(),
                sim_analysis::plasticity_contrast(&pairs, &plan),
            ));
        }
    }

    let find = |name: &str| {
        outcomes
            .iter()
            .find(|outcome| outcome.condition == name)
            .cloned()
            .unwrap_or_else(|| sim_analysis::summarise_plasticity(name, &[], &plan))
    };
    let treatment = find(treatment_name);
    let control = find(baseline);
    let verdicts = vec![
        sim_analysis::Verdict::decide(
            "C11.1",
            &treatment,
            &control,
            treatment.shifted,
            control.shifted,
            treatment.shift_no_variance + treatment.shift_refused,
            plan.shift_bar,
            plan.control_ceiling,
        ),
        sim_analysis::Verdict::decide(
            "C11.2",
            &treatment,
            &control,
            treatment.selected,
            control.selected,
            treatment.drift_no_variance,
            plan.drift_bar,
            plan.control_ceiling,
        ),
    ];

    print!(
        "{}",
        sim_analysis::render_plasticity(
            &manifest.campaign.id,
            &plan,
            &per_world,
            &outcomes,
            &contrasts,
            &verdicts,
        )
    );
    Ok(())
}

/// A **descriptive census** of the plasticity conjunction and the learned
/// state, over the snapshots a campaign already wrote.
///
/// # This command decides nothing and must never be given a bar
///
/// `lifesim plasticity` decides C11.1 and C11.2 against thresholds
/// pre-registered before their campaign ran. This one is run afterwards, on
/// the same artifacts, with the outcome already known, and it exists to say
/// *which* of two very different things a null of those criteria was: a
/// genotype space in which the learning phenotype was never assembled, or one
/// in which it was assembled and did nothing. Those need opposite follow-ups.
/// A statistic computed with the answer in view cannot be a test of it, so
/// there is no `--treatment`, no `--baseline`, no bar and no verdict here,
/// and the report labels itself on its first line.
///
/// # It refuses a snapshot that is not what it claims
///
/// `command_plasticity`'s discipline, and for the same reason: a world whose
/// config did not survive the codec would produce an empty census, and "no
/// edge ever assembled the conjunction" and "the config that would have let
/// one is not in this file" are opposite conclusions. The restored config's
/// own stable hash must equal the hash the manifest recorded for the run.
///
/// The expressed census is checked a second way. It re-expresses and
/// re-compiles every genome with the world's own plasticity budget, which is
/// how the plan the learn phase ran was built, so its plastic-edge count must
/// equal `plastic_edges_total`. A disagreement means the census is describing
/// a different plan from the one that ran, and it is a refusal rather than a
/// footnote.
/// Phase 12 C12.1, C12.2 and C12.3, decided against the pre-registered
/// plan in `sim_analysis::ArtifactPlan::preregistered`.
///
/// `--treatment` names condition A, `--control` condition C (actions inert),
/// `--disabled` condition D (combination disabled). Every world's event log
/// and final snapshot are read; a missing or undecodable file is an error,
/// never an empty world, on the grounds `command_morph` states.
fn command_artifact(options: Options) -> Result<(), String> {
    let path = options
        .manifest
        .as_ref()
        .ok_or_else(|| format!("artifact requires --manifest\n{}", usage()))?;
    let treatment = options
        .treatment
        .clone()
        .ok_or_else(|| format!("artifact requires --treatment\n{}", usage()))?;
    let control = options.baseline.clone().ok_or_else(|| {
        format!(
            "artifact requires --control (given as --baseline)\n{}",
            usage()
        )
    })?;
    let disabled = options
        .disabled
        .clone()
        .ok_or_else(|| format!("artifact requires --disabled\n{}", usage()))?;
    let text = fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let manifest = sim_experiment::Manifest::parse(&text).map_err(|error| error.to_string())?;
    let directory = path.parent().unwrap_or_else(|| std::path::Path::new("."));
    for name in [&treatment, &control, &disabled] {
        if !manifest
            .campaign
            .conditions
            .iter()
            .any(|condition| condition.name == *name)
        {
            return Err(format!("no condition named '{name}' in this campaign"));
        }
    }
    let mut plan = sim_analysis::ArtifactPlan::preregistered();
    if let Some(seed) = options.analysis_seed {
        plan.analysis_seed = seed;
    }
    plan.seeds = manifest.campaign.seeds.len();
    let mut worlds = Vec::new();
    for condition in &manifest.campaign.conditions {
        for run in manifest.runs_for(&condition.name) {
            let stem = sim_experiment::run_stem(&run.condition, run.seed);
            let artifact = run.artifact.ok_or_else(|| {
                format!("{stem}: the manifest carries no artifact block - the section was off")
            })?;
            let bytes = fs::read(directory.join(format!("{stem}.alev")))
                .map_err(|error| format!("{stem}.alev: {error}"))?;
            let (_, events) = sim_persist::decode_log_events(&bytes)
                .map_err(|error| format!("{stem}.alev: {error:?}"))?;
            let bytes = fs::read(directory.join(format!("{stem}.alif")))
                .map_err(|error| format!("{stem}.alif: {error}"))?;
            let (_, state) = sim_persist::decode_snapshot(&bytes)
                .map_err(|error| format!("{stem}: decode: {error}"))?;
            if state.config.stable_hash() != run.config_hash {
                return Err(format!(
                    "{stem}: the snapshot's config hash 0x{:016x} is not the manifest's 0x{:016x}",
                    state.config.stable_hash(),
                    run.config_hash
                ));
            }
            let table = state
                .objects
                .as_ref()
                .ok_or_else(|| format!("{stem}: the snapshot carries no object table"))?;
            if table.exposure_ticks.len() != state.ids.len() {
                return Err(format!(
                    "{stem}: object observation rows do not match the population"
                ));
            }
            let living: Vec<sim_analysis::LivingOrganism> = state
                .ids
                .iter()
                .enumerate()
                .map(|(index, &id)| sim_analysis::LivingOrganism {
                    id,
                    age_ticks: state.age_ticks[index],
                    exposure_ticks: table.exposure_ticks[index],
                    birth_band: table.birth_band[index],
                })
                .collect();
            // The reachability census: which object actions the living
            // genomes bind at the horizon (pre-registration section 6).
            let caps = state.config.genome2.caps;
            let genomes: Vec<sim_core::Genome2> = match state.schema2.as_ref() {
                Some(schema2) => schema2
                    .genomes
                    .iter()
                    .map(|encoded| {
                        sim_core::Genome2::decode(encoded, &caps)
                            .map_err(|error| format!("{stem}: decode genome: {error}"))
                    })
                    .collect::<Result<_, _>>()?,
                None => Vec::new(),
            };
            let census = sim_analysis::binding_census(
                &genomes,
                run.mutation.map_or(0, |mutation| mutation.binding_applied),
            );
            let inputs = sim_analysis::WorldInputs {
                condition: &run.condition,
                seed: run.seed,
                horizon: manifest.campaign.ticks,
                initial_organisms: u64::from(state.config.initial_organisms),
                extinct: run.extinct,
                organism_ticks: artifact.organism_ticks,
                picked_up: artifact.counters.picked_up,
                placed: artifact.counters.placed,
                combined: artifact.counters.combined,
                composites_depth2_final: artifact.composites_depth2,
                events: &events,
                living: &living,
                census,
            };
            worlds.push(sim_analysis::world_artifact(&inputs, &plan));
        }
    }
    let report = sim_analysis::decide_artifact(&plan, worlds, &treatment, &control, &disabled);
    print!(
        "{}",
        sim_analysis::render_artifact(&manifest.campaign.id, &report)
    );
    Ok(())
}

fn command_conjunction(options: Options) -> Result<(), String> {
    let path = options
        .manifest
        .as_ref()
        .ok_or_else(|| format!("conjunction requires --manifest\n{}", usage()))?;
    let text = fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let manifest = sim_experiment::Manifest::parse(&text).map_err(|error| error.to_string())?;
    let directory = path.parent().unwrap_or_else(|| std::path::Path::new("."));

    let mut per_world: Vec<(String, Vec<sim_analysis::WorldConjunction>)> = Vec::new();
    for condition in &manifest.campaign.conditions {
        let mut worlds = Vec::new();
        for run in manifest.runs_for(&condition.name) {
            let stem = sim_experiment::run_stem(&run.condition, run.seed);
            let snapshot_path = directory.join(format!("{stem}.alif"));
            let snapshot = fs::read(&snapshot_path)
                .map_err(|error| format!("{}: {error}", snapshot_path.display()))?;
            let (_, state) = sim_persist::decode_snapshot(&snapshot)
                .map_err(|error| format!("{stem}: decode snapshot: {error}"))?;
            let restored_hash = state.config.stable_hash();
            if restored_hash != run.config_hash {
                return Err(format!(
                    "{stem}: the restored config hashes {restored_hash:#018x} but the manifest \
                     recorded {:#018x} - the config did not survive the snapshot, so a zero \
                     conjunction count here would mean a codec defect and not an unassembled \
                     phenotype (plasticity.enabled={}, max_plastic_edges={})",
                    run.config_hash,
                    state.config.plasticity.enabled,
                    state.config.plasticity.max_plastic_edges,
                ));
            }
            let caps = state.config.genome2.caps;
            let budget = state.config.plasticity_budget();
            let genomes: Vec<sim_core::Genome2> = match state.schema2.as_ref() {
                Some(schema2) => schema2
                    .genomes
                    .iter()
                    .map(|encoded| {
                        sim_core::Genome2::decode(encoded, &caps)
                            .map_err(|error| format!("{stem}: decode genome: {error}"))
                    })
                    .collect::<Result<Vec<_>, String>>()?,
                None => {
                    return Err(format!(
                        "{stem}: the snapshot carries no schema-2 section, so there is no \
                         edge locus to census"
                    ));
                }
            };
            let alleles = sim_analysis::allele_conjunction_census(&genomes);
            let expressed = sim_analysis::expressed_conjunction_census(&genomes, budget)
                .map_err(|error| format!("{stem}: {error}"))?;
            let learned = state.learn.as_ref().map(sim_analysis::learned_state_census);
            let world = sim_core::World::from_state(state)
                .map_err(|error| format!("{stem}: restore: {error}"))?;
            let metrics = world.metrics();
            if expressed.plastic_edges != metrics.plastic_edges_total {
                return Err(format!(
                    "{stem}: re-compiling every genome with this world's own plasticity budget \
                     gives {} plastic edges but the restored world reports {} - the census is \
                     describing a different plan from the one the learn phase ran",
                    expressed.plastic_edges, metrics.plastic_edges_total
                ));
            }
            if let Some(learned) = learned.as_ref()
                && learned.rows != metrics.plastic_edges_total
            {
                return Err(format!(
                    "{stem}: the learned-state section holds {} rows but the world reports {} \
                     plastic edges",
                    learned.rows, metrics.plastic_edges_total
                ));
            }
            worlds.push(sim_analysis::WorldConjunction {
                condition: run.condition.clone(),
                seed: run.seed,
                population: run.population,
                extinct: run.extinct,
                alleles,
                expressed,
                learned,
                plastic_edges_total: metrics.plastic_edges_total,
                plasticity_updates_total: metrics.plasticity_updates_total,
                reported_mean_abs_learned_milli: metrics.mean_abs_learned_milli,
            });
        }
        per_world.push((condition.name.clone(), worlds));
    }

    let arms: Vec<sim_analysis::ArmConjunction> = per_world
        .iter()
        .map(|(name, worlds)| sim_analysis::summarise_conjunction(name, worlds))
        .collect();
    print!(
        "{}",
        sim_analysis::render_conjunction(&manifest.campaign.id, &per_world, &arms)
    );
    Ok(())
}

/// Phase 9 C9.1, C9.2, and C9.5: structural evolution against a control.
///
/// The baseline condition is the structural-mutation-disabled control every
/// stability contrast is drawn against. Criterion decisions are printed with
/// the counts that produced them, and the analysis plan is echoed, so a
/// reader can check the bars against the campaign source rather than trusting
/// this program's arithmetic.
fn command_structure(options: Options) -> Result<(), String> {
    let path = options
        .manifest
        .as_ref()
        .ok_or_else(|| format!("structure requires --manifest\n{}", usage()))?;
    let baseline = options
        .baseline
        .as_deref()
        .ok_or_else(|| format!("structure requires --baseline\n{}", usage()))?;
    let text = fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let manifest = sim_experiment::Manifest::parse(&text).map_err(|error| error.to_string())?;
    let directory = path.parent().unwrap_or_else(|| std::path::Path::new("."));
    if !manifest
        .campaign
        .conditions
        .iter()
        .any(|condition| condition.name == baseline)
    {
        return Err(format!("no condition named '{baseline}' in this campaign"));
    }

    let mut plan = sim_analysis::StructurePlan::default();
    if let Some(seed) = options.analysis_seed {
        plan.analysis_seed = seed;
    }
    if let Some(sesoi) = options.sesoi {
        plan.stability_tolerance_milli = sesoi;
    }

    let per_world: Vec<(String, Vec<sim_analysis::WorldStructure>)> = manifest
        .campaign
        .conditions
        .iter()
        .map(|condition| {
            (
                condition.name.clone(),
                sim_analysis::structure_worlds(&manifest, directory, &condition.name),
            )
        })
        .collect();
    let outcomes: Vec<sim_analysis::StructureOutcome> = per_world
        .iter()
        .map(|(name, worlds)| sim_analysis::summarise_structure(name, worlds, &plan))
        .collect();

    let control = per_world
        .iter()
        .find(|(name, _)| name == baseline)
        .map(|(_, worlds)| worlds.clone())
        .unwrap_or_default();
    let mut stabilities = Vec::new();
    for (name, worlds) in &per_world {
        if name == baseline {
            continue;
        }
        for (label, quantity) in [
            (
                "population",
                (|world: &sim_analysis::WorldStructure| world.population as i64)
                    as fn(&sim_analysis::WorldStructure) -> i64,
            ),
            ("median_lifespan", |world: &sim_analysis::WorldStructure| {
                world.median_lifespan_ticks as i64
            }),
        ] {
            let pairs = sim_analysis::structure_pairs(worlds, &control, quantity);
            stabilities.push(sim_analysis::stability(
                name, baseline, label, &pairs, &plan,
            ));
        }
    }

    print!(
        "{}",
        sim_analysis::render_structure(
            &manifest.campaign.id,
            &plan,
            &per_world,
            &outcomes,
            &stabilities
        )
    );
    Ok(())
}

/// Phase 8 C8.1 to C8.7: per-world demography reduced from each run's
/// event log, plus the snapshot-derived thermal-matching statistic.
///
/// One flat line per world. The contrasts are deliberately left to the
/// reader rather than baked in, because Phase 8's criteria compare
/// different condition pairs from each other (A vs B for C8.1, M-low vs
/// M-high for C8.5) and a command that picked one would hide the rest.
fn command_demography(options: Options) -> Result<(), String> {
    let path = options
        .manifest
        .as_ref()
        .ok_or_else(|| format!("demography requires --manifest\n{}", usage()))?;
    let text = fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let manifest = sim_experiment::Manifest::parse(&text).map_err(|error| error.to_string())?;
    let directory = path.parent().unwrap_or_else(|| std::path::Path::new("."));

    println!("demography-report 1 campaign {}", manifest.campaign.id);
    println!("index_version {}", sim_analysis::DEMOGRAPHY_INDEX_VERSION);
    for run in &manifest.runs {
        let stem = sim_experiment::run_stem(&run.condition, run.seed);
        let log_path = directory.join(format!("{stem}.alev"));
        let bytes =
            fs::read(&log_path).map_err(|error| format!("{}: {error}", log_path.display()))?;
        let (_, events) =
            sim_persist::decode_log_events(&bytes).map_err(|error| error.to_string())?;
        let summary = sim_analysis::world_demography(&events);

        // C8.7 needs the phenotype and the position, which only the
        // snapshot carries. Absent snapshots are reported as absent rather
        // than defaulted to zero, which would read as "no correlation".
        let snapshot_path = directory.join(format!("{stem}.alif"));
        // A failure here is reported with its reason, never as a bare
        // "absent" that a reader could mistake for "no correlation".
        let thermal = fs::read(&snapshot_path)
            .map_err(|error| error.to_string())
            .and_then(|bytes| {
                sim_persist::decode_snapshot(&bytes).map_err(|error| error.to_string())
            })
            .and_then(|(_, state)| {
                sim_core::World::from_state(state).map_err(|error| error.to_string())
            })
            .map(|world| sim_analysis::thermal_match_rho_milli(&world));
        let (thermal_rho, thermal_n) = match thermal {
            Ok(Some((rho, observed))) => (rho.to_string(), observed.to_string()),
            Ok(None) => ("no-temperature-field".to_owned(), "0".to_owned()),
            Err(reason) => (
                format!("error:{}", reason.replace(' ', "_")),
                "0".to_owned(),
            ),
        };

        // C8.2's surplus measures. Population over capacity is not a
        // ratio -- one is a count and the other is milli-biomass -- so the
        // saturation of the food field stands in for "how close to
        // carrying capacity", and per-capita energy is the surplus itself.
        let biomass_saturation_micro = if run.total_capacity_milli > 0 {
            i128::from(run.total_biomass_milli) * 1_000_000 / i128::from(run.total_capacity_milli)
        } else {
            0
        };
        let energy_per_capita_milli = if run.population > 0 {
            i128::from(run.total_energy_milli) / i128::from(run.population)
        } else {
            0
        };
        println!(
            "world condition={} seed={:#018x} population={} capacity_milli={} \
             biomass_saturation_micro={} energy_per_capita_milli={} \
             deaths_total={} starvation_share_milli={} \
             causes_above_5pct={} median_lifespan={} completed={} censored={} \
             investment_offspring_rho_milli={} parents={} max_age_observed={} \
             thermal_rho_milli={} thermal_n={}",
            run.condition,
            run.seed,
            run.population,
            run.total_capacity_milli,
            biomass_saturation_micro,
            energy_per_capita_milli,
            summary.deaths_total,
            summary.starvation_share_milli,
            summary.causes_above_five_percent,
            summary.median_lifespan_ticks,
            summary.completed_lifespans,
            summary.censored_individuals,
            summary.investment_offspring_rho_milli,
            summary.parents_observed,
            run.max_age_ticks_observed,
            thermal_rho,
            thermal_n,
        );
    }
    Ok(())
}

/// The Phase 13 observer report: C13.1's arrival census and the social
/// reachability census, one line per world, no threshold and no verdict
/// (ADR-0016). The A-versus-C and A-versus-D decisions belong to the
/// pre-registered campaign analysis, which reads these lines; a command
/// that decided here would hide the census a null must be read against.
///
/// `--epoch N` names the relocation that gives C13.1 its patch P and its
/// clock: the patch active during `[N*interval, (N+1)*interval)` is P,
/// the relocation tick is the naive cut, and arrivals are counted only
/// inside that window - an arrival after the next relocation is an
/// arrival at a place the food has left. The patch centre is recomputed
/// through the kernel's own schedule accessor, never re-derived here.
fn command_social(options: Options) -> Result<(), String> {
    let path = options
        .manifest
        .as_ref()
        .ok_or_else(|| format!("social requires --manifest\n{}", usage()))?;
    let epoch = options
        .epoch
        .ok_or_else(|| format!("social requires --epoch\n{}", usage()))?;
    if epoch == 0 {
        return Err("epoch 0 is the interval before the first relocation and has no patch".into());
    }
    let text = fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let manifest = sim_experiment::Manifest::parse(&text).map_err(|error| error.to_string())?;
    let directory = path.parent().unwrap_or_else(|| std::path::Path::new("."));

    println!("social-report 1 campaign {}", manifest.campaign.id);
    println!(
        "detector {} epoch {}",
        sim_analysis::ARRIVAL_DETECTOR_VERSION,
        epoch
    );
    for condition in &manifest.campaign.conditions {
        for run in manifest.runs_for(&condition.name) {
            let stem = sim_experiment::run_stem(&run.condition, run.seed);
            let config = manifest
                .campaign
                .config_for(condition, run.seed)
                .map_err(|error| format!("{stem}: {error}"))?;
            if !config.worldmod.enabled || !config.worldmod.patch_enabled {
                return Err(format!(
                    "{stem}: no relocating patch (worldmod.patch_enabled is off), so C13.1 \
                     has no patch P; this command cannot report this campaign"
                ));
            }
            let interval = config.worldmod.relocate_interval_ticks;
            let naive_after = epoch * interval;
            let window_end = naive_after + interval;

            let log_path = directory.join(format!("{stem}.alev"));
            let bytes =
                fs::read(&log_path).map_err(|error| format!("{}: {error}", log_path.display()))?;
            let (_, events) =
                sim_persist::decode_log_events(&bytes).map_err(|error| error.to_string())?;

            let spatial_path = directory.join(format!("{stem}.alss"));
            let bytes = fs::read(&spatial_path)
                .map_err(|error| format!("{}: {error}", spatial_path.display()))?;
            let scan = sim_persist::decode_spatial(&bytes)
                .map_err(|error| format!("{stem}: decode spatial: {error}"))?;
            if scan.info.config_hash != run.config_hash {
                return Err(format!(
                    "{stem}: the spatial log's config hash {:#018x} is not the run's {:#018x}",
                    scan.info.config_hash, run.config_hash
                ));
            }
            if scan.info.terrain_checksum != run.terrain_checksum {
                return Err(format!(
                    "{stem}: the spatial log's terrain {:#018x} is not the run's {:#018x}",
                    scan.info.terrain_checksum, run.terrain_checksum
                ));
            }
            // Prove the whole series was read, not a silently shortened
            // one - the manifest records the count for exactly this check.
            if scan.samples.len() as u64 != run.spatial_samples {
                return Err(format!(
                    "{stem}: {} spatial samples decoded but the manifest recorded {}",
                    scan.samples.len(),
                    run.spatial_samples
                ));
            }
            let window: Vec<sim_persist::SpatialSample> = scan
                .samples
                .iter()
                .filter(|sample| sample.tick >= naive_after && sample.tick < window_end)
                .cloned()
                .collect();
            if window.is_empty() {
                return Err(format!(
                    "{stem}: no spatial samples in epoch {epoch}'s window \
                     [{naive_after},{window_end}); the run is {} ticks at interval {}",
                    run.ticks, scan.info.sample_interval_ticks
                ));
            }

            // The patch through the kernel's own schedule, never a
            // re-derivation of the draw.
            let world = sim_core::World::new(config.clone())
                .map_err(|error| format!("{stem}: rebuild world: {error}"))?;
            let centre_cell = world
                .patch_centre_for_epoch(epoch)
                .ok_or_else(|| format!("{stem}: the kernel reports no patch for epoch {epoch}"))?;
            let patch = sim_analysis::PatchSpec::CellSquare {
                centre_cell,
                cells_x: config.cells_x,
                cells_y: config.cells_y,
                cell_size_fp: config.cell_size_fp(),
                radius_cells: config.worldmod.patch_radius_cells,
            };
            let arrival = sim_analysis::arrival_census(
                &events,
                &window,
                config.initial_organisms,
                scan.info.sample_interval_ticks,
                patch,
                naive_after,
            )
            .map_err(|error| format!("{stem}: arrival census: {error:?}"))?;

            let snapshot_path = directory.join(format!("{stem}.alif"));
            let bytes = fs::read(&snapshot_path)
                .map_err(|error| format!("{}: {error}", snapshot_path.display()))?;
            let (_, state) = sim_persist::decode_snapshot(&bytes)
                .map_err(|error| format!("{stem}: decode snapshot: {error}"))?;
            if state.config.stable_hash() != run.config_hash {
                return Err(format!(
                    "{stem}: the snapshot's config hash {:#018x} is not the manifest's {:#018x}",
                    state.config.stable_hash(),
                    run.config_hash
                ));
            }
            let caps = state.config.genome2.caps;
            let genomes: Vec<sim_core::Genome2> = match state.schema2.as_ref() {
                Some(schema2) => schema2
                    .genomes
                    .iter()
                    .map(|encoded| {
                        sim_core::Genome2::decode(encoded, &caps)
                            .map_err(|error| format!("{stem}: decode genome: {error}"))
                    })
                    .collect::<Result<_, _>>()?,
                None => Vec::new(),
            };
            let census = sim_analysis::social::social_binding_census(
                &genomes,
                state.config.plasticity_budget(),
            )
            .map_err(|error| format!("{stem}: {error}"))?;

            let signals_emitted = events
                .iter()
                .filter(|event| matches!(event.kind, sim_core::EventKind::SignalEmitted { .. }))
                .count();
            let perception_faults = events
                .iter()
                .filter(|event| matches!(event.kind, sim_core::EventKind::PerceptionFault { .. }))
                .count();

            println!(
                "world condition={} seed={:#018x} patch_cell={} window=[{},{}) \
                 naive={} arrived={} censored={} born_in_patch={} never_observed={} \
                 median_latency={} arrival_fraction_milli={} sample_interval={} \
                 population={} binds_cues={} hearers={} speakers={} emit_and_in={} \
                 in_and_cues={} rule5_alleles={} rule5_carriers={} rule5_edges={} \
                 rule5_expressed_organisms={} signals_emitted={} perception_faults={}",
                run.condition,
                run.seed,
                centre_cell,
                naive_after,
                window_end,
                arrival.naive_total,
                arrival.naive_arrived,
                arrival.naive_censored,
                arrival.born_in_patch,
                arrival.never_observed,
                arrival
                    .median_naive_latency_ticks
                    .map_or_else(|| "none".to_owned(), |ticks| ticks.to_string()),
                arrival.arrival_fraction_milli,
                arrival.sample_interval_ticks,
                census.population,
                census.binds_neighbour_cues,
                census.binds_signal_in,
                census.binds_signal_emit,
                census.binds_emit_and_in,
                census.binds_in_and_cues,
                census.rule5_alleles,
                census.organisms_with_rule5_allele,
                census.rule5_expressed_edges,
                census.organisms_with_rule5_expressed,
                signals_emitted,
                perception_faults,
            );
        }
    }
    Ok(())
}

/// C13.1's seed-paired contrast: the world-level naive arrival fraction
/// under the treatment arm against the baseline arm, paired by seed, with
/// the SESOI count on the ABSOLUTE paired difference (the relative form
/// is undefined at a zero-fraction control, which an expected-null
/// baseline mostly is - D-120's blindness lesson, recorded on
/// `PairedResult::reaching_absolute_directed`).
///
/// Per world: for each pre-registered epoch, the arrival census's
/// fraction (naive_arrived / naive_total, milli); the world statistic is
/// the mean over epochs with a nonzero naive cohort (ADR-0022 A5). A
/// world where every named epoch has an empty cohort is unusable and
/// reported by seed, never imputed. The A-versus-C and A-versus-D calls
/// are two invocations of this command; the criterion needs both.
fn command_social_contrast(options: Options) -> Result<(), String> {
    let path = options
        .manifest
        .as_ref()
        .ok_or_else(|| format!("social-contrast requires --manifest\n{}", usage()))?;
    let treatment = options
        .treatment
        .as_ref()
        .ok_or_else(|| format!("social-contrast requires --treatment\n{}", usage()))?;
    let baseline = options
        .baseline
        .as_ref()
        .ok_or_else(|| format!("social-contrast requires --baseline\n{}", usage()))?;
    let epochs: Vec<u64> = options
        .epochs
        .as_ref()
        .ok_or_else(|| format!("social-contrast requires --epochs\n{}", usage()))?
        .split(',')
        .map(|part| {
            part.trim()
                .parse::<u64>()
                .map_err(|_| format!("invalid epoch '{part}' in --epochs"))
        })
        .collect::<Result<_, _>>()?;
    if epochs.is_empty() || epochs.contains(&0) {
        return Err("--epochs needs at least one epoch, none of them 0".into());
    }
    let sesoi = options
        .sesoi
        .ok_or_else(|| format!("social-contrast requires --sesoi\n{}", usage()))?;
    let analysis_seed = options.analysis_seed.unwrap_or(0x1373_5eed_0000_0001);

    let text = fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let manifest = sim_experiment::Manifest::parse(&text).map_err(|error| error.to_string())?;
    let directory = path.parent().unwrap_or_else(|| std::path::Path::new("."));

    println!(
        "social-contrast 1 campaign {} detector {} stats {}",
        manifest.campaign.id,
        sim_analysis::ARRIVAL_DETECTOR_VERSION,
        sim_analysis::PAIRED_STATS_VERSION,
    );
    println!(
        "treatment {} baseline {} epochs {:?} sesoi_milli {} direction increase \
         analysis_seed {:#018x}",
        treatment, baseline, epochs, sesoi, analysis_seed
    );

    // Per (condition, seed): mean arrival fraction over contributing
    // epochs, or the reason there is none.
    let mut fractions: std::collections::BTreeMap<(String, u64), Result<i64, String>> =
        std::collections::BTreeMap::new();
    for name in [treatment.as_str(), baseline.as_str()] {
        let condition = manifest
            .campaign
            .conditions
            .iter()
            .find(|condition| condition.name == name)
            .ok_or_else(|| format!("condition {name} is not in the campaign"))?;
        for run in manifest.runs_for(name) {
            let stem = sim_experiment::run_stem(&run.condition, run.seed);
            let config = manifest
                .campaign
                .config_for(condition, run.seed)
                .map_err(|error| format!("{stem}: {error}"))?;
            if !config.worldmod.enabled || !config.worldmod.patch_enabled {
                return Err(format!(
                    "{stem}: no relocating patch, so C13.1 has no patch P"
                ));
            }
            let interval = config.worldmod.relocate_interval_ticks;

            let bytes = fs::read(directory.join(format!("{stem}.alev")))
                .map_err(|error| format!("{stem}.alev: {error}"))?;
            let (_, events) =
                sim_persist::decode_log_events(&bytes).map_err(|error| error.to_string())?;
            let bytes = fs::read(directory.join(format!("{stem}.alss")))
                .map_err(|error| format!("{stem}.alss: {error}"))?;
            let scan = sim_persist::decode_spatial(&bytes)
                .map_err(|error| format!("{stem}: decode spatial: {error}"))?;
            if scan.info.config_hash != run.config_hash
                || scan.info.terrain_checksum != run.terrain_checksum
            {
                return Err(format!("{stem}: the spatial log is not this run's"));
            }
            if scan.samples.len() as u64 != run.spatial_samples {
                return Err(format!(
                    "{stem}: {} spatial samples decoded but the manifest recorded {}",
                    scan.samples.len(),
                    run.spatial_samples
                ));
            }
            let world = sim_core::World::new(config.clone())
                .map_err(|error| format!("{stem}: rebuild world: {error}"))?;

            let mut contributing: Vec<i64> = Vec::new();
            for &epoch in &epochs {
                let naive_after = epoch * interval;
                let window_end = naive_after + interval;
                let window: Vec<sim_persist::SpatialSample> = scan
                    .samples
                    .iter()
                    .filter(|sample| sample.tick >= naive_after && sample.tick < window_end)
                    .cloned()
                    .collect();
                if window.is_empty() {
                    return Err(format!(
                        "{stem}: epoch {epoch} window [{naive_after},{window_end}) holds no \
                         samples; the epoch set does not fit this campaign"
                    ));
                }
                let centre_cell = world.patch_centre_for_epoch(epoch).ok_or_else(|| {
                    format!("{stem}: the kernel reports no patch for epoch {epoch}")
                })?;
                let census = sim_analysis::arrival_census(
                    &events,
                    &window,
                    config.initial_organisms,
                    scan.info.sample_interval_ticks,
                    sim_analysis::PatchSpec::CellSquare {
                        centre_cell,
                        cells_x: config.cells_x,
                        cells_y: config.cells_y,
                        cell_size_fp: config.cell_size_fp(),
                        radius_cells: config.worldmod.patch_radius_cells,
                    },
                    naive_after,
                )
                .map_err(|error| format!("{stem}: arrival census: {error:?}"))?;
                if census.naive_total > 0 {
                    contributing.push(census.arrival_fraction_milli);
                }
            }
            let outcome = if contributing.is_empty() {
                Err("every named epoch had an empty naive cohort".to_owned())
            } else {
                Ok(contributing.iter().sum::<i64>() / contributing.len() as i64)
            };
            match &outcome {
                Ok(fraction) => println!(
                    "world condition={} seed={:#018x} fraction_milli={} epochs_contributing={}",
                    name,
                    run.seed,
                    fraction,
                    contributing.len()
                ),
                Err(reason) => println!(
                    "world condition={} seed={:#018x} unusable reason={}",
                    name,
                    run.seed,
                    reason.replace(' ', "_")
                ),
            }
            fractions.insert((name.to_owned(), run.seed), outcome);
        }
    }

    let mut pairs: Vec<sim_analysis::Pair> = Vec::new();
    let mut unusable = 0_usize;
    for run in manifest.runs_for(treatment) {
        let t = fractions.get(&(treatment.clone(), run.seed));
        let c = fractions.get(&(baseline.clone(), run.seed));
        match (t, c) {
            (Some(Ok(t)), Some(Ok(c))) => pairs.push(sim_analysis::Pair {
                seed: run.seed,
                treatment_milli: *t,
                control_milli: *c,
            }),
            _ => unusable += 1,
        }
    }
    // The prespecified direction: the treatment arm arrives faster, so
    // its fraction is higher (the pre-registration fixes this; A-vs-C
    // and A-vs-D both expect increase).
    let result = sim_analysis::compare(
        &pairs,
        sesoi,
        500,
        sim_analysis::Direction::Increase,
        analysis_seed,
    );
    let absolute_p = sim_analysis::binomial_upper_tail_milli(
        result.reaching_absolute_directed,
        result.pairs,
        500,
    );
    println!(
        "contrast pairs={} unusable_seeds={} reaching_absolute_directed={} \
         absolute_p_milli={} positive_differences={} mean_difference_milli={} \
         ci95=[{},{}] median_difference_milli={} reaching_relative_directed={}",
        result.pairs,
        unusable,
        result.reaching_absolute_directed,
        absolute_p,
        result.positive_differences,
        result.mean_difference_milli,
        result.ci_low_milli,
        result.ci_high_milli,
        result.median_difference_milli,
        result.reaching_sesoi_directed,
    );
    Ok(())
}

/// List every config field a campaign may set.
fn command_fields() -> Result<(), String> {
    let config = SimConfig::phase2_default(0);
    for name in sim_experiment::FIELD_NAMES {
        let value = sim_experiment::read_field(&config, name)
            .map(|value| value.to_string())
            .unwrap_or_default();
        println!("{name}\t{value}");
    }
    Ok(())
}

/// Replay an event log and report what it contains. A torn tail is reported
/// with the size of the trustworthy prefix, never repaired.
fn command_verify_events(args: Vec<String>) -> Result<(), String> {
    let path = args
        .first()
        .ok_or_else(|| format!("verify-events requires a path\n{}", usage()))?;
    let bytes = fs::read(path).map_err(|error| format!("{path}: {error}"))?;
    let (scan, error) =
        sim_persist::decode_log_prefix(&bytes).map_err(|error| error.to_string())?;
    println!(
        concat!(
            "{{\"event_log_schema_version\":1,\"path\":\"{}\",",
            "\"format_version\":{},\"world_id\":{},\"seed\":\"0x{:016x}\",",
            "\"config_hash\":\"0x{:016x}\",\"event_schema_version\":{},",
            "\"build\":\"{}\",\"segments\":{},\"events\":{},\"dropped\":{},",
            "\"first_tick\":{},\"last_tick\":{},\"bytes_valid\":{},\"bytes_total\":{},",
            "\"births_total\":{},\"deaths_starvation_total\":{},",
            "\"deaths_old_age_total\":{},\"capacity_rejections_total\":{},",
            "\"paired_births_total\":{},\"controller_faults_total\":{},",
            "\"status\":\"{}\"}}"
        ),
        path,
        scan.info.format_version,
        scan.info.world_id,
        scan.info.seed,
        scan.info.config_hash,
        scan.info.event_schema_version,
        scan.info.build_version,
        scan.segments,
        scan.events,
        scan.dropped,
        scan.first_tick.map_or(-1_i64, |tick| tick as i64),
        scan.last_tick.map_or(-1_i64, |tick| tick as i64),
        scan.bytes_consumed,
        bytes.len(),
        scan.counters.births_total,
        scan.counters.deaths_starvation_total,
        scan.counters.deaths_old_age_total,
        scan.counters.capacity_rejections_total,
        scan.counters.paired_births_total,
        scan.counters.controller_faults_total,
        match &error {
            None => "complete".to_owned(),
            Some(error) => format!("truncated: {error}"),
        }
    );
    match error {
        Some(error) => Err(format!("event log is not fully valid: {error}")),
        None => Ok(()),
    }
}

/// Verify a snapshot file through an isolated restore. Never modifies
/// anything; prints a provenance report.
fn command_verify_save(args: Vec<String>) -> Result<(), String> {
    let path = args.first().ok_or_else(usage)?;
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    // **Peek the version, then consult the registry, then read.** The order
    // used to be the other way round: `read_info` ran first and refuses any
    // version but the current one, so `migration_for` was dead code from the
    // day it was written - an old file failed with `UnsupportedFormat` before
    // the registry that exists to handle it was ever asked. The only command
    // in the product that consults the registry could not reach it.
    let format_version = sim_persist::peek_format_version(&bytes)
        .map_err(|error| format!("invalid save: {error}"))?;
    let migration = sim_persist::migration_for(format_version)?;
    let (info, world) = sim_persist::SnapshotStore::load_world(std::path::Path::new(path))
        .map_err(|error| format!("restore failed: {error}"))?;
    println!(
        concat!(
            "{{\"verify_schema_version\":2,\"path\":\"{}\",\"format_version\":{},",
            "\"migrated_to\":{},\"compressed\":{},\"world_id\":{},\"tick\":{},",
            "\"seed\":\"0x{:016x}\",",
            "\"config_hash\":\"0x{:016x}\",\"state_checksum\":\"0x{:016x}\",",
            "\"terrain_checksum\":\"0x{:016x}\",\"build_version\":\"{}\",",
            "\"population\":{},\"result\":\"ok\"}}"
        ),
        json_escape(path),
        format_version,
        // `null` when the file was already current, so a reader can tell a
        // native load from a migrated one without comparing version numbers
        // against a build constant it does not have.
        match migration {
            Some(migration) => migration.to_format.to_string(),
            None => "null".to_owned(),
        },
        info.compressed,
        info.world_id,
        info.tick,
        info.seed,
        info.config_hash,
        info.state_checksum,
        info.terrain_checksum,
        json_escape(&info.build_version),
        world.population()
    );
    Ok(())
}

/// Compare two run-summary JSON files (stdout of `lifesim run`). Reports
/// whether the runs share an experiment lineage and the metric deltas.
fn command_compare(args: Vec<String>) -> Result<(), String> {
    if args.len() != 2 {
        return Err(usage());
    }
    let read_summary = |path: &str| -> Result<String, String> {
        fs::read_to_string(path).map_err(|error| format!("{path}: {error}"))
    };
    let first = read_summary(&args[0])?;
    let second = read_summary(&args[1])?;
    let field = |body: &str, name: &str| -> String {
        body.split(&format!("\"{name}\":"))
            .nth(1)
            .map(|rest| {
                rest.trim_start()
                    .trim_start_matches('"')
                    .split([',', '}', '"'])
                    .next()
                    .unwrap_or("")
                    .to_owned()
            })
            .unwrap_or_default()
    };
    let same_lineage = field(&first, "config_hash") == field(&second, "config_hash")
        && field(&first, "seed") == field(&second, "seed")
        && field(&first, "behavior_policy") == field(&second, "behavior_policy");
    let numeric = |body: &str, name: &str| -> i128 { field(body, name).parse().unwrap_or(0) };
    let mut deltas = String::new();
    for name in [
        "final_tick",
        "population",
        "births_total",
        "deaths_starvation_total",
        "deaths_old_age_total",
        "paired_births_total",
        "max_ancestry_depth",
    ] {
        if !deltas.is_empty() {
            deltas.push(',');
        }
        deltas.push_str(&format!(
            "\"{name}\":{{\"a\":{},\"b\":{},\"delta\":{}}}",
            numeric(&first, name),
            numeric(&second, name),
            numeric(&second, name) - numeric(&first, name)
        ));
    }
    println!(
        concat!(
            "{{\"compare_schema_version\":1,\"same_experiment_lineage\":{},",
            "\"config_hash_a\":\"{}\",\"config_hash_b\":\"{}\",",
            "\"state_checksum_a\":\"{}\",\"state_checksum_b\":\"{}\",",
            "\"identical_final_state\":{},\"metrics\":{{{}}},",
            "\"note\":\"runs with different config hashes are different experiments and must not be labeled identical\"}}"
        ),
        same_lineage,
        field(&first, "config_hash"),
        field(&second, "config_hash"),
        field(&first, "state_checksum"),
        field(&second, "state_checksum"),
        same_lineage && field(&first, "state_checksum") == field(&second, "state_checksum"),
        deltas
    );
    Ok(())
}

fn active_policy(world: &World) -> &'static str {
    if world.phase2_enabled() {
        PHASE2_BEHAVIOR_POLICY_VERSION
    } else {
        BEHAVIOR_POLICY_VERSION
    }
}

// --- fixture ----------------------------------------------------------------

fn command_fixture(options: Options) -> Result<(), String> {
    let ticks = options.ticks.unwrap_or(500);
    let mut world = build_world(&options)?;
    for _ in 0..ticks {
        world.step();
    }
    world
        .check_invariants()
        .map_err(|violation| format!("invariant violation: {violation}"))?;
    let metrics = world.metrics();
    if let Some(social) = world.social_counters() {
        // Fixture schema 9: the Phase 13 social trace. A separate schema on
        // the grounds every earlier one was separated: a social world
        // perceives and signals, and a reader that parsed it as a Phase 12
        // fixture would be comparing an ecology with a channel to one
        // without. Every mechanism the phase added has a field, so the
        // fixture cannot silently become a control (trap 1):
        // `verify-phase13-determinism.sh` refuses the load-bearing ones at
        // zero. The artifact trace keeps running underneath, and its own
        // non-vacuity fields ride along so the composed trace cannot lose
        // its substrate unnoticed.
        let table = world.social_table().expect("a social world has a table");
        let field_nonzero_cells = table
            .committed_field_q16
            .iter()
            .filter(|&&value| value > 0)
            .count();
        let contact_committed = table.prior_contact.iter().filter(|&&flag| flag).count();
        let objects = world
            .object_table()
            .expect("a social world is an artifact world");
        let counters = objects.counters;
        let mutation = world
            .mutation_counters()
            .expect("a social world is a genome2 world");
        println!(
            concat!(
                "{{\"fixture_schema_version\":9,\"phase\":\"phase13\",",
                "\"behavior_policy\":\"{}\",\"genome2_policy\":\"{}\",",
                "\"social_policy\":\"{}\",\"artifact_policy\":\"{}\",",
                "\"plasticity_policy\":\"{}\",\"channel_registry\":{},",
                "\"rule_registry\":{},",
                "\"organisms\":{},\"ticks\":{},\"seed\":\"0x{:016x}\",",
                "\"config_hash\":\"0x{:016x}\",\"terrain_checksum\":\"0x{:016x}\",",
                "\"state_checksum\":\"0x{:016x}\",\"population\":{},",
                "\"births_total\":{},\"binding_applied\":{},",
                "\"signals_emitted\":{},\"signal_cost_milli\":{},",
                "\"corruption_draws\":{},\"scrambled_deliveries\":{},",
                "\"rule5_updates\":{},\"perception_faults\":{},",
                "\"field_nonzero_cells\":{},\"contact_committed\":{},",
                "\"struck_terrain\":{},\"picked_up\":{},",
                "\"consumed_events\":{},\"created_carcass\":{},",
                "\"controller_faults_total\":{}}}"
            ),
            PHASE2_BEHAVIOR_POLICY_VERSION,
            GENOME2_POLICY_VERSION,
            sim_core::SOCIAL_POLICY_VERSION,
            sim_core::ARTIFACT_POLICY_VERSION,
            sim_core::PLASTICITY_POLICY_VERSION,
            world.config().channel_registry_version(),
            world.config().plasticity_rule_registry_version(),
            world.config().initial_organisms,
            ticks,
            world.config().world_seed,
            world.config_hash(),
            world.terrain().terrain_checksum,
            world.state_checksum(),
            metrics.population,
            metrics.births_total,
            mutation.binding_applied,
            social.signals_emitted_total,
            social.signal_cost_milli_total,
            social.corruption_draws_total,
            social.scrambled_deliveries_total,
            social.rule5_updates_total,
            social.perception_faults_total,
            field_nonzero_cells,
            contact_committed,
            counters.struck_terrain,
            counters.picked_up,
            counters.consumed_events,
            counters.created_carcass,
            metrics.controller_faults_total
        );
    } else if let Some(objects) = world.object_table() {
        // Fixture schema 8: the Phase 12 artifact trace. A separate schema,
        // on the grounds every earlier one was separated: an artifact world
        // has objects in it, and a reader that parsed it as a Phase 9
        // fixture would be comparing an ecology with objects to one without.
        //
        // The counts beside the checksum are what stop this fixture from
        // silently becoming a control: extraction, pick-up, placement, drop,
        // strikes on objects, fracture, combination, consumption, carcass
        // objects, decay, and the bind operator each have a field, and
        // `verify-phase12-determinism.sh` refuses every one of them at zero.
        // `depth2` is reported and not refused, because a depth-two
        // composite is a chance event of the script, not a mechanism.
        let counters = objects.counters;
        let mutation = world
            .mutation_counters()
            .expect("an artifact world is a genome2 world");
        let composites_depth2 = objects.count_with_depth_at_least(2);
        println!(
            concat!(
                "{{\"fixture_schema_version\":8,\"phase\":\"phase12\",",
                "\"behavior_policy\":\"{}\",\"genome2_policy\":\"{}\",",
                "\"artifact_policy\":\"{}\",\"material_policy\":\"{}\",",
                "\"material_registry\":{},\"channel_registry\":{},",
                "\"organisms\":{},\"ticks\":{},\"seed\":\"0x{:016x}\",",
                "\"config_hash\":\"0x{:016x}\",\"terrain_checksum\":\"0x{:016x}\",",
                "\"state_checksum\":\"0x{:016x}\",\"population\":{},",
                "\"births_total\":{},\"binding_applied\":{},",
                "\"objects_total\":{},\"objects_free\":{},\"objects_depth2\":{},",
                "\"created_extracted\":{},\"created_fractured\":{},",
                "\"created_combined\":{},\"created_carcass\":{},",
                "\"picked_up\":{},\"dropped\":{},\"placed\":{},",
                "\"struck_objects\":{},\"struck_terrain\":{},",
                "\"fractured\":{},\"combined\":{},\"consumed_events\":{},",
                "\"decayed_away\":{},\"refusals\":{},\"cap_refusals\":{},",
                "\"mass_in_pool_milli\":{},\"energy_in_pool_milli\":{},",
                "\"controller_faults_total\":{}}}"
            ),
            PHASE2_BEHAVIOR_POLICY_VERSION,
            GENOME2_POLICY_VERSION,
            sim_core::ARTIFACT_POLICY_VERSION,
            sim_core::MATERIAL_POLICY_VERSION,
            sim_core::MATERIAL_REGISTRY_VERSION,
            world.config().channel_registry_version(),
            world.config().initial_organisms,
            ticks,
            world.config().world_seed,
            world.config_hash(),
            world.terrain().terrain_checksum,
            world.state_checksum(),
            metrics.population,
            metrics.births_total,
            mutation.binding_applied,
            objects.len(),
            objects.free_count(),
            composites_depth2,
            counters.created_extracted,
            counters.created_fractured,
            counters.created_combined,
            counters.created_carcass,
            counters.picked_up,
            counters.dropped,
            counters.placed,
            counters.struck_objects,
            counters.struck_terrain,
            counters.fractured,
            counters.combined,
            counters.consumed_events,
            counters.decayed_away,
            counters.refusals(),
            counters.cap_refusals(),
            objects.total_mass_milli(),
            objects.total_energy_milli(),
            metrics.controller_faults_total
        );
    } else if metrics.plasticity_enabled {
        // Fixture schema 5: the Phase 11 numeric-safety trace. A separate
        // schema rather than extra fields on schema 4, on the same grounds
        // schema 4 was separated from 3: this is one immortal sterile
        // organism, not a population, and a reader that parsed it as a Phase
        // 9 fixture would be comparing a trace with an ecology.
        //
        // The learned-state fields are here so the fixture cannot silently
        // become a control. A `state_checksum` alone would keep reproducing
        // if the learn phase stopped running, if the organism died at tick 3,
        // or if every delta sat at zero - the checksum would simply be a
        // different constant that two runs still agreed on. `population`,
        // `plastic_edges_total`, `plasticity_updates_total` and
        // `mean_abs_learned_milli` are what make a zero visible, and
        // `verify-phase11-determinism.sh` refuses each of them.
        //
        // Schema 6 adds `learned_edges_nonzero` and `max_abs_learned_milli`
        // beside the mean. The mean alone is not sufficient and this fixture
        // is the reason to say so here: it is a mean over every plastic edge
        // alive, so a run in which a handful of edges learned substantially
        // reports the same 0 as a run in which nothing learned at all. That
        // is not hypothetical - it is what Phase 11's confirmatory campaign
        // published (D-098). The count is the field that can distinguish
        // them, so the count is the one the verify script now refuses at
        // zero.
        //
        // `plasticity_anomalies_total` is reported rather than asserted
        // nonzero: it counts faults and clamp saturations, and both are
        // *supposed* to stay at zero here. It is in the line so that a run
        // which starts saturating - the runaway-plasticity risk - shows it.
        let (channel_registry, activation_registry) = registry_versions();
        println!(
            concat!(
                "{{\"fixture_schema_version\":7,\"phase\":\"phase11\",",
                "\"behavior_policy\":\"{}\",\"genome2_policy\":\"{}\",",
                "\"meiosis_policy\":\"{}\",\"structmut_policy\":\"{}\",",
                "\"controller2_policy\":\"{}\",\"plasticity_policy\":\"{}\",",
                "\"rule_registry\":{},\"genome2_schema\":{},",
                "\"channel_registry\":{},\"activation_registry\":{},",
                "\"organisms\":{},\"ticks\":{},\"seed\":\"0x{:016x}\",",
                "\"config_hash\":\"0x{:016x}\",\"terrain_checksum\":\"0x{:016x}\",",
                "\"state_checksum\":\"0x{:016x}\",\"population\":{},",
                "\"plastic_edges_total\":{},\"plasticity_updates_total\":{},",
                // Inserted **before** `controller_faults_total`, not appended.
                // `scripts/verify-phase11-determinism.sh` greps for
                // `"controller_faults_total":0}` with the closing brace, which
                // pins that field as the last one in the object; appending
                // here would fail the script with a message about faults.
                "\"plasticity_updates_applied\":{},\"plasticity_updates_static\":{},",
                "\"plasticity_updates_refused\":{},",
                "\"plasticity_anomalies_total\":{},\"mean_abs_learned_milli\":{},",
                "\"learned_edges_nonzero\":{},\"max_abs_learned_milli\":{},",
                "\"plasticity_cost_milli\":{},\"controller_faults_total\":{}}}"
            ),
            PHASE2_BEHAVIOR_POLICY_VERSION,
            GENOME2_POLICY_VERSION,
            MEIOSIS_POLICY_VERSION,
            STRUCTMUT_POLICY_VERSION,
            CONTROLLER2_POLICY_VERSION,
            PLASTICITY_POLICY_VERSION,
            RULE_REGISTRY_VERSION,
            GENOME2_SCHEMA_VERSION,
            channel_registry,
            activation_registry,
            world.config().initial_organisms,
            ticks,
            world.config().world_seed,
            world.config_hash(),
            world.terrain().terrain_checksum,
            world.state_checksum(),
            metrics.population,
            metrics.plastic_edges_total,
            metrics.plasticity_updates_total,
            metrics.plasticity_updates_applied,
            metrics.plasticity_updates_static,
            metrics.plasticity_updates_refused,
            metrics.plasticity_anomalies_total,
            metrics.mean_abs_learned_milli,
            metrics.learned_edges_nonzero,
            metrics.max_abs_learned_milli,
            metrics.plasticity_cost_milli,
            metrics.controller_faults_total
        );
    } else if world.genome2_enabled() {
        // Fixture schema 4: the Phase 9 lineage. A separate schema rather
        // than extra fields on schema 3, because a schema-2 world is a
        // different genome and a different controller and a reader that
        // parsed it as a Phase 2 fixture would be comparing two different
        // simulations.
        //
        // The structure metrics and the applied-mutation counts are here so
        // the fixture cannot silently become a control. At the Phase 1/2
        // horizon of 500 ticks nothing has reproduced at all - founders
        // spawn at age 0 and `maturity_age_ticks` is 600 - so a 500-tick
        // schema-2 fixture would pin meiosis, structural mutation, and the
        // schema-2 birth path by pinning none of them. `duplications_applied`
        // is reported separately from `structural_mutations_applied` because
        // the latter counts point mutations too, and a point mutation
        // changes no structure.
        let counters = world
            .mutation_counters()
            .expect("a genome2 world has schema-2 counters");
        let (channel_registry, activation_registry) = registry_versions();
        println!(
            concat!(
                "{{\"fixture_schema_version\":4,\"phase\":\"phase9\",",
                "\"behavior_policy\":\"{}\",\"genome2_policy\":\"{}\",",
                "\"meiosis_policy\":\"{}\",\"structmut_policy\":\"{}\",",
                "\"controller2_policy\":\"{}\",\"genome2_schema\":{},",
                "\"channel_registry\":{},\"activation_registry\":{},",
                "\"organisms\":{},\"ticks\":{},\"seed\":\"0x{:016x}\",",
                "\"config_hash\":\"0x{:016x}\",\"terrain_checksum\":\"0x{:016x}\",",
                "\"state_checksum\":\"0x{:016x}\",\"population\":{},",
                "\"births_total\":{},\"paired_births_total\":{},\"deaths_total\":{},",
                "\"controller_faults_total\":{},\"max_ancestry_depth\":{},",
                "\"mean_nodes_milli\":{},\"mean_edges_milli\":{},",
                "\"median_nodes\":{},\"median_edges\":{},\"distinct_structures\":{},",
                "\"structural_mutations_applied\":{},\"duplications_applied\":{},",
                "\"structural_mutations_rejected\":{}}}"
            ),
            PHASE2_BEHAVIOR_POLICY_VERSION,
            GENOME2_POLICY_VERSION,
            MEIOSIS_POLICY_VERSION,
            STRUCTMUT_POLICY_VERSION,
            CONTROLLER2_POLICY_VERSION,
            GENOME2_SCHEMA_VERSION,
            channel_registry,
            activation_registry,
            world.config().initial_organisms,
            ticks,
            world.config().world_seed,
            world.config_hash(),
            world.terrain().terrain_checksum,
            world.state_checksum(),
            metrics.population,
            metrics.births_total,
            metrics.paired_births_total,
            metrics.deaths_starvation_total + metrics.deaths_old_age_total,
            metrics.controller_faults_total,
            metrics.max_ancestry_depth,
            metrics.mean_nodes_milli,
            metrics.mean_edges_milli,
            metrics.median_nodes,
            metrics.median_edges,
            metrics.distinct_structures,
            metrics.structural_mutations_applied,
            counters.duplication_applied,
            metrics.structural_mutations_rejected
        );
    } else if world.phase2_enabled() {
        // Fixture schema 3: the Phase 2 lineage. Never relabeled as v2.
        println!(
            concat!(
                "{{\"fixture_schema_version\":3,\"phase\":\"phase2\",",
                "\"behavior_policy\":\"{}\",\"genome_policy\":\"{}\",",
                "\"controller_policy\":\"{}\",\"genome_schema\":{},\"topology_id\":{},",
                "\"organisms\":{},\"ticks\":{},\"seed\":\"0x{:016x}\",",
                "\"config_hash\":\"0x{:016x}\",\"terrain_checksum\":\"0x{:016x}\",",
                "\"state_checksum\":\"0x{:016x}\",\"population\":{},",
                "\"births_total\":{},\"paired_births_total\":{},\"deaths_total\":{},",
                "\"controller_faults_total\":{},\"max_ancestry_depth\":{}}}"
            ),
            PHASE2_BEHAVIOR_POLICY_VERSION,
            GENOME_POLICY_VERSION,
            CONTROLLER_POLICY_VERSION,
            GENOME_SCHEMA_VERSION,
            TOPOLOGY_ID,
            world.config().initial_organisms,
            ticks,
            world.config().world_seed,
            world.config_hash(),
            world.terrain().terrain_checksum,
            world.state_checksum(),
            metrics.population,
            metrics.births_total,
            metrics.paired_births_total,
            metrics.deaths_starvation_total + metrics.deaths_old_age_total,
            metrics.controller_faults_total,
            metrics.max_ancestry_depth
        );
    } else {
        println!(
            concat!(
                "{{\"fixture_schema_version\":2,\"phase\":\"phase1\",",
                "\"organisms\":{},\"ticks\":{},\"seed\":\"0x{:016x}\",",
                "\"config_hash\":\"0x{:016x}\",\"terrain_checksum\":\"0x{:016x}\",",
                "\"state_checksum\":\"0x{:016x}\",\"population\":{},",
                "\"births_total\":{},\"deaths_total\":{}}}"
            ),
            world.config().initial_organisms,
            ticks,
            world.config().world_seed,
            world.config_hash(),
            world.terrain().terrain_checksum,
            world.state_checksum(),
            metrics.population,
            metrics.births_total,
            metrics.deaths_starvation_total + metrics.deaths_old_age_total
        );
    }
    Ok(())
}

/// Run a Phase 2 world for the requested ticks, then execute the offline
/// similarity analysis and print its report with timing.
fn command_analyze(options: Options) -> Result<(), String> {
    let ticks = options.ticks.unwrap_or(0);
    let mut world = build_world(&options)?;
    if !world.phase2_enabled() {
        return Err("analyze requires --phase2".to_owned());
    }
    for _ in 0..ticks {
        world.step();
    }
    world
        .check_invariants()
        .map_err(|violation| format!("invariant violation: {violation}"))?;
    let started = Instant::now();
    let report = analyze(&world).expect("phase2 world always yields a report");
    let elapsed = started.elapsed();
    let mut sizes = String::new();
    for (index, size) in report.cluster_sizes.iter().enumerate() {
        if index > 0 {
            sizes.push(',');
        }
        sizes.push_str(&size.to_string());
    }
    println!(
        concat!(
            "{{\"analysis_schema_version\":1,\"algorithm\":\"{}\",",
            "\"analysis_tick\":{},\"config_hash\":\"0x{:016x}\",",
            "\"genome_schema\":{},\"population\":{},\"sampled\":{},",
            "\"sample_stride\":{},\"threshold_q16\":{},\"neural_weight_q16\":{},",
            "\"cluster_count\":{},\"cluster_sizes\":[{}],",
            "\"mean_pairwise_distance\":{:.6},",
            "\"analysis_runtime_microseconds\":{:.1}}}"
        ),
        report.algorithm,
        report.analysis_tick,
        report.config_hash,
        report.genome_schema_version,
        report.population,
        report.sampled,
        report.sample_stride,
        report.threshold_q16,
        report.neural_weight_q16,
        report.cluster_count,
        sizes,
        report.mean_pairwise_distance,
        elapsed.as_secs_f64() * 1_000_000.0
    );
    Ok(())
}

// --- inspect ----------------------------------------------------------------

fn command_inspect(options: Options) -> Result<(), String> {
    let world = build_world(&options)?;
    world
        .check_invariants()
        .map_err(|violation| format!("invariant violation: {violation}"))?;
    let terrain = world.terrain();
    println!(
        concat!(
            "{{\"inspect_schema_version\":1,",
            "\"seed\":\"0x{:016x}\",\"config_hash\":\"0x{:016x}\",",
            "\"cells_x\":{},\"cells_y\":{},\"cell_size_m\":{},",
            "\"land_cells\":{},\"habitable_cells\":{},\"land_fraction_q16\":{},",
            "\"initial_population\":{},\"total_biomass_milli\":{},",
            "\"terrain_checksum\":\"0x{:016x}\",\"state_checksum\":\"0x{:016x}\"}}"
        ),
        world.config().world_seed,
        world.config_hash(),
        terrain.cells_x,
        terrain.cells_y,
        world.config().cell_size_m,
        terrain.land_cells,
        terrain.habitable_cells,
        terrain.land_fraction_q16(),
        world.population(),
        world.total_biomass_milli(),
        terrain.terrain_checksum,
        world.state_checksum()
    );
    Ok(())
}

// --- benchmark ----------------------------------------------------------------

struct PhaseTimer {
    started: Option<Instant>,
    totals: [Duration; TickPhase::ALL.len()],
}

impl PhaseTimer {
    fn new() -> Self {
        Self {
            started: None,
            totals: [Duration::ZERO; TickPhase::ALL.len()],
        }
    }

    fn reset(&mut self) {
        self.totals = [Duration::ZERO; TickPhase::ALL.len()];
    }
}

fn phase_index(phase: TickPhase) -> usize {
    TickPhase::ALL
        .iter()
        .position(|&candidate| candidate == phase)
        .expect("phase is in ALL")
}

impl TickObserver for PhaseTimer {
    fn phase_started(&mut self, _phase: TickPhase) {
        self.started = Some(Instant::now());
    }

    fn phase_finished(&mut self, phase: TickPhase) {
        if let Some(started) = self.started.take() {
            self.totals[phase_index(phase)] += started.elapsed();
        }
    }
}

fn command_benchmark(options: Options) -> Result<(), String> {
    let benchmark_id = options
        .benchmark_id
        .clone()
        .ok_or_else(|| "--benchmark-id is required".to_owned())?;
    let output_dir = options
        .output
        .clone()
        .ok_or_else(|| "--output is required".to_owned())?;
    let warmup = options.warmup.unwrap_or(200);
    let samples = options.samples.unwrap_or(50);
    let ticks_per_sample = options.ticks_per_sample.unwrap_or(10);
    if samples == 0 || ticks_per_sample == 0 {
        return Err("samples and ticks-per-sample must be positive".to_owned());
    }
    fs::create_dir_all(&output_dir).map_err(|error| error.to_string())?;

    let mut world = build_world(&options)?;
    let organisms = world.config().initial_organisms;
    let phase_label = if world.phase2_enabled() {
        "phase2"
    } else {
        "phase1"
    };
    // Fixed-population (reproduction-off) scenarios get their own label so
    // records with different conditions never collide or get compared as
    // the same scenario.
    let scenario_label = if world.config().reproduction_enabled {
        format!("{organisms}")
    } else {
        format!("{organisms}-fixedpop")
    };
    let population_at_start = world.population();
    for _ in 0..warmup {
        world.step();
    }
    let population_after_warmup = world.population();

    let phase_count = TickPhase::ALL.len();
    let mut timer = PhaseTimer::new();
    let mut tick_samples: Vec<f64> = Vec::with_capacity(samples);
    let mut phase_samples: Vec<Vec<f64>> = vec![Vec::with_capacity(samples); phase_count];
    let mut allocation_samples: Vec<f64> = Vec::with_capacity(samples);

    for _ in 0..samples {
        timer.reset();
        let allocations_before = ALLOCATION_COUNT.load(Ordering::Relaxed);
        let started = Instant::now();
        for _ in 0..ticks_per_sample {
            world.step_with_observer(&mut timer);
        }
        let elapsed = started.elapsed();
        let allocations_after = ALLOCATION_COUNT.load(Ordering::Relaxed);
        let divisor = ticks_per_sample as f64;
        tick_samples.push(elapsed.as_secs_f64() * 1_000_000.0 / divisor);
        for (samples_for_phase, total) in phase_samples.iter_mut().zip(timer.totals.iter()) {
            samples_for_phase.push(total.as_secs_f64() * 1_000_000.0 / divisor);
        }
        allocation_samples.push((allocations_after - allocations_before) as f64 / divisor);
    }
    world
        .check_invariants()
        .map_err(|violation| format!("invariant violation after benchmark: {violation}"))?;

    let raw_path = output_dir.join(format!("{phase_label}-rust-{scenario_label}-raw.csv"));
    let mut raw = File::create(&raw_path).map_err(|error| error.to_string())?;
    write!(raw, "sample,tick_us").map_err(|error| error.to_string())?;
    for phase in TickPhase::ALL {
        write!(raw, ",{}_us", phase.name()).map_err(|error| error.to_string())?;
    }
    writeln!(raw, ",allocations_per_tick").map_err(|error| error.to_string())?;
    for sample in 0..samples {
        write!(raw, "{sample},{:.3}", tick_samples[sample]).map_err(|error| error.to_string())?;
        for samples_for_phase in &phase_samples {
            write!(raw, ",{:.3}", samples_for_phase[sample]).map_err(|error| error.to_string())?;
        }
        writeln!(raw, ",{:.1}", allocation_samples[sample]).map_err(|error| error.to_string())?;
    }

    let generated_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_secs();
    let metrics = world.metrics();

    // Phase 2 extras: policy versions, audit counters, and the offline
    // similarity-analysis runtime (measured separately from tick cost).
    let phase2_json = if world.phase2_enabled() {
        let started = Instant::now();
        let report = analyze(&world).expect("phase2 world yields a report");
        let similarity_us = started.elapsed().as_secs_f64() * 1_000_000.0;
        format!(
            concat!(
                "  \"phase2\": {{\"behavior_policy\": \"{}\", \"genome_policy\": \"{}\", ",
                "\"controller_policy\": \"{}\", \"genome_schema\": {}, \"topology_id\": {}, ",
                "\"paired_births_total\": {}, \"pair_rejected_capacity_total\": {}, ",
                "\"pair_rejected_placement_total\": {}, \"pair_rejected_energy_total\": {}, ",
                "\"controller_faults_total\": {}, \"invalid_records_admitted\": 0, ",
                "\"max_ancestry_depth\": {}, ",
                "\"similarity\": {{\"algorithm\": \"{}\", \"sampled\": {}, \"sample_stride\": {}, ",
                "\"cluster_count\": {}, \"mean_pairwise_distance\": {:.6}, ",
                "\"runtime_microseconds\": {:.1}}}}},\n"
            ),
            PHASE2_BEHAVIOR_POLICY_VERSION,
            GENOME_POLICY_VERSION,
            CONTROLLER_POLICY_VERSION,
            GENOME_SCHEMA_VERSION,
            TOPOLOGY_ID,
            metrics.paired_births_total,
            metrics.pair_rejected_capacity_total,
            metrics.pair_rejected_placement_total,
            metrics.pair_rejected_energy_total,
            metrics.controller_faults_total,
            metrics.max_ancestry_depth,
            report.algorithm,
            report.sampled,
            report.sample_stride,
            report.cluster_count,
            report.mean_pairwise_distance,
            similarity_us
        )
    } else {
        String::new()
    };

    let mut phase_json = String::new();
    for (index, phase) in TickPhase::ALL.iter().enumerate() {
        if index > 0 {
            phase_json.push_str(", ");
        }
        phase_json.push_str(&format!(
            "\"{}\": {}",
            phase.name(),
            stats_json(summarize(&phase_samples[index]))
        ));
    }

    let summary = format!(
        concat!(
            "{{\n",
            "  \"benchmark_schema_version\": {},\n",
            "  \"benchmark_id\": \"{}-{}-rust-{}\",\n",
            "  \"generated_at_unix_seconds\": {},\n",
            "  \"revision\": \"{}\",\n",
            "  \"working_tree_dirty\": {},\n",
            "  \"toolchain\": \"{}\",\n",
            "  \"build_profile\": \"release-lto-thin\",\n",
            "  \"os\": \"{}\",\n",
            "  \"architecture\": \"{}\",\n",
            "  \"cpu\": \"{}\",\n",
            "  \"host_memory_bytes\": {},\n",
            "  \"scenario\": {{\"initial_organisms\": {}, \"cells_x\": {}, \"cells_y\": {}, \"cell_size_m\": {}, \"max_entities\": {}, \"reproduction\": {}, \"observers\": 0, \"dt_ms\": {}}},\n",
            "{}",
            "  \"behavior_policy\": \"{}\",\n",
            "  \"config_hash\": \"0x{:016x}\",\n",
            "  \"seed\": \"0x{:016x}\",\n",
            "  \"method\": {{\"warmup_ticks\": {}, \"samples\": {}, \"ticks_per_sample\": {}, \"deterministic_mode\": \"strict\", \"checksum_excluded_from_tick\": true}},\n",
            "  \"population\": {{\"at_start\": {}, \"after_warmup\": {}, \"at_end\": {}}},\n",
            "  \"tick_microseconds\": {},\n",
            "  \"phase_microseconds\": {{{}}},\n",
            "  \"allocations_per_tick\": {},\n",
            "  \"rss_bytes_at_completion\": {},\n",
            "  \"final_tick\": {},\n",
            "  \"final_state_checksum\": \"0x{:016x}\",\n",
            "  \"raw_samples\": \"{}\",\n",
            "  \"limitations\": [\"local development host, not deployment VM\", \"RSS is sampled at completion, not peak\", \"{}\", \"spatial-query cost is the spatial_index plus sense phases\"]\n",
            "}}\n"
        ),
        if world.phase2_enabled() { 2 } else { 1 },
        json_escape(&benchmark_id),
        phase_label,
        scenario_label,
        generated_at,
        json_escape(&git_revision()),
        git_dirty(),
        json_escape(
            &command_output("rustc", &["--version"]).unwrap_or_else(|| "unknown".to_owned())
        ),
        json_escape(&os_description()),
        env::consts::ARCH,
        json_escape(&sysctl("machdep.cpu.brand_string").unwrap_or_else(|| "unknown".to_owned())),
        sysctl("hw.memsize").unwrap_or_else(|| "0".to_owned()),
        organisms,
        world.config().cells_x,
        world.config().cells_y,
        world.config().cell_size_m,
        world.config().max_entities,
        world.config().reproduction_enabled,
        world.config().dt_ms,
        phase2_json,
        active_policy(&world),
        world.config_hash(),
        world.config().world_seed,
        warmup,
        samples,
        ticks_per_sample,
        population_at_start,
        population_after_warmup,
        metrics.population,
        stats_json(summarize(&tick_samples)),
        phase_json,
        stats_json(summarize(&allocation_samples)),
        current_rss_bytes(),
        world.tick_number(),
        world.state_checksum(),
        json_escape(&raw_path.to_string_lossy()),
        if world.config().reproduction_enabled {
            "population varies across the run; per-tick cost reflects the live population trajectory"
        } else {
            "fixed-population scenario: no births occur; population changes only through deaths"
        }
    );
    let summary_path = output_dir.join(format!("{phase_label}-rust-{scenario_label}-summary.json"));
    fs::write(&summary_path, &summary).map_err(|error| error.to_string())?;
    print!("{summary}");
    Ok(())
}

// --- metrics -----------------------------------------------------------------

fn prometheus_text(world: &World, tick_durations: &[Duration], wall_elapsed: Duration) -> String {
    let metrics = world.metrics();
    let mut text = String::new();
    let world_label = "local";

    text.push_str("# TYPE lifesim_organisms gauge\n");
    text.push_str(&format!(
        "lifesim_organisms{{world=\"{world_label}\",life_state=\"alive\"}} {}\n",
        metrics.population
    ));
    text.push_str("# TYPE lifesim_births_total counter\n");
    text.push_str(&format!(
        "lifesim_births_total{{world=\"{world_label}\"}} {}\n",
        metrics.births_total
    ));
    text.push_str("# TYPE lifesim_deaths_total counter\n");
    text.push_str(&format!(
        "lifesim_deaths_total{{world=\"{world_label}\",cause=\"starvation\"}} {}\n",
        metrics.deaths_starvation_total
    ));
    text.push_str(&format!(
        "lifesim_deaths_total{{world=\"{world_label}\",cause=\"old_age\"}} {}\n",
        metrics.deaths_old_age_total
    ));
    text.push_str("# TYPE lifesim_births_rejected_capacity_total counter\n");
    text.push_str(&format!(
        "lifesim_births_rejected_capacity_total{{world=\"{world_label}\"}} {}\n",
        metrics.capacity_rejections_total
    ));
    text.push_str("# TYPE lifesim_food_biomass gauge\n");
    text.push_str(&format!(
        "lifesim_food_biomass{{world=\"{world_label}\",biome=\"all\"}} {}\n",
        format_units(metrics.total_biomass_milli)
    ));
    if metrics.phase2_enabled {
        text.push_str("# TYPE lifesim_paired_births_total counter\n");
        text.push_str(&format!(
            "lifesim_paired_births_total{{world=\"{world_label}\"}} {}\n",
            metrics.paired_births_total
        ));
        text.push_str("# TYPE lifesim_pair_rejected_total counter\n");
        for (reason, value) in [
            ("capacity", metrics.pair_rejected_capacity_total),
            ("placement", metrics.pair_rejected_placement_total),
            ("energy", metrics.pair_rejected_energy_total),
        ] {
            text.push_str(&format!(
                "lifesim_pair_rejected_total{{world=\"{world_label}\",reason=\"{reason}\"}} {value}\n"
            ));
        }
        text.push_str("# TYPE lifesim_controller_faults_total counter\n");
        text.push_str(&format!(
            "lifesim_controller_faults_total{{world=\"{world_label}\"}} {}\n",
            metrics.controller_faults_total
        ));
        text.push_str("# TYPE lifesim_max_ancestry_depth gauge\n");
        text.push_str(&format!(
            "lifesim_max_ancestry_depth{{world=\"{world_label}\"}} {}\n",
            metrics.max_ancestry_depth
        ));
    }

    // Tick duration histogram measured by this host process.
    let buckets_seconds = [
        0.000_1, 0.000_25, 0.000_5, 0.001, 0.002_5, 0.005, 0.01, 0.025,
    ];
    let mut counts = [0_u64; 8];
    let mut sum_seconds = 0.0_f64;
    for duration in tick_durations {
        let seconds = duration.as_secs_f64();
        sum_seconds += seconds;
        for (index, bucket) in buckets_seconds.iter().enumerate() {
            if seconds <= *bucket {
                counts[index] += 1;
            }
        }
    }
    text.push_str("# TYPE lifesim_tick_duration_seconds histogram\n");
    for (index, bucket) in buckets_seconds.iter().enumerate() {
        text.push_str(&format!(
            "lifesim_tick_duration_seconds_bucket{{world=\"{world_label}\",le=\"{bucket}\"}} {}\n",
            counts[index]
        ));
    }
    text.push_str(&format!(
        "lifesim_tick_duration_seconds_bucket{{world=\"{world_label}\",le=\"+Inf\"}} {}\n",
        tick_durations.len()
    ));
    text.push_str(&format!(
        "lifesim_tick_duration_seconds_sum{{world=\"{world_label}\"}} {sum_seconds:.9}\n"
    ));
    text.push_str(&format!(
        "lifesim_tick_duration_seconds_count{{world=\"{world_label}\"}} {}\n",
        tick_durations.len()
    ));

    let rate = if wall_elapsed.as_secs_f64() > 0.0 {
        tick_durations.len() as f64 / wall_elapsed.as_secs_f64()
    } else {
        0.0
    };
    text.push_str("# TYPE lifesim_tick_rate_hz gauge\n");
    text.push_str(&format!(
        "lifesim_tick_rate_hz{{world=\"{world_label}\"}} {rate:.3}\n"
    ));
    text
}

fn format_units(milli: i64) -> String {
    format!("{}.{:03}", milli / 1000, (milli % 1000).unsigned_abs())
}

// --- shared helpers ------------------------------------------------------------

#[derive(Clone, Copy)]
struct Stats {
    p50: f64,
    p95: f64,
    p99: f64,
    min: f64,
    max: f64,
}

fn summarize(samples: &[f64]) -> Stats {
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    Stats {
        p50: percentile(&sorted, 0.50),
        p95: percentile(&sorted, 0.95),
        p99: percentile(&sorted, 0.99),
        min: sorted[0],
        max: *sorted.last().expect("non-empty samples"),
    }
}

fn percentile(sorted: &[f64], percentile: f64) -> f64 {
    let index = ((sorted.len() - 1) as f64 * percentile).ceil() as usize;
    sorted[index]
}

fn stats_json(stats: Stats) -> String {
    format!(
        "{{\"p50\":{:.3},\"p95\":{:.3},\"p99\":{:.3},\"min\":{:.3},\"max\":{:.3}}}",
        stats.p50, stats.p95, stats.p99, stats.min, stats.max
    )
}

fn current_rss_bytes() -> u64 {
    let pid = std::process::id().to_string();
    command_output("ps", &["-o", "rss=", "-p", &pid])
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(0)
        .saturating_mul(1024)
}

fn git_revision() -> String {
    command_output("git", &["rev-parse", "HEAD"]).unwrap_or_else(|| "unborn-main".to_owned())
}

fn git_dirty() -> bool {
    Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .map(|output| !output.stdout.is_empty())
        .unwrap_or(true)
}

fn os_description() -> String {
    command_output("sw_vers", &["-productName"])
        .zip(command_output("sw_vers", &["-productVersion"]))
        .map(|(name, version)| format!("{name} {version}"))
        .unwrap_or_else(|| env::consts::OS.to_owned())
}

fn sysctl(name: &str) -> Option<String> {
    command_output("sysctl", &["-n", name])
}

fn command_output(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program).args(args).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn json_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}
