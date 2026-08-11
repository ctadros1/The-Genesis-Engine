//! Phase 11's two measurement instruments, at the whole-world level.
//!
//! The arithmetic is unit-tested in `actioncensus.rs` and the codec seams in
//! `genome2.rs`. What is left, and what this file is for, is everything that
//! only exists once a world is running:
//!
//! - **the census cannot change what the world computes.** It is written
//!   every tick and read by nothing, so a probe world and a probe-free world
//!   must agree on every behavioural quantity. A counter that is only written
//!   cannot move a trajectory, and this is where that stops being an argument;
//! - **the census stays in lockstep** across both birth paths and the death
//!   path, so a per-individual series is per-individual;
//! - **the census survives a save**, which is the whole justification for
//!   checksumming it;
//! - **the marker locus is never expressed**, is inherited and recombined,
//!   and is reachable by mutation.
//!
//! # The trap this file was written against
//!
//! `probe.enabled` and the sub-gates are two layers that can both refuse the
//! same config, so `matches!(err, A | B)` would pass with either guard
//! deleted. Every refusal below is pinned to the diagnostic of the guard that
//! is meant to fire.

use sim_core::{
    ActionClass, Genome2, GenomeCaps, InvariantViolation, LOCOMOTION_CLASS_COUNT, LocusKind,
    MARKER_HOMOLOGY_ID, RestoreError, SimConfig, World, locomotion_class,
};

const SEED: u64 = 0x5eed_cafe_f00d_beef;

/// A small schema-2 world. `probe` is left off; the helpers below turn on
/// exactly the instrument each test is about.
fn base_config(seed: u64) -> SimConfig {
    let mut config = SimConfig::phase11_default(seed);
    config.cells_x = 64;
    config.cells_y = 64;
    config.initial_organisms = 120;
    config.max_entities = 1_200;
    config
}

fn with_census(seed: u64) -> SimConfig {
    let mut config = base_config(seed);
    config.probe.enabled = true;
    config.probe.action_census_enabled = true;
    config
}

fn with_marker(seed: u64) -> SimConfig {
    let mut config = base_config(seed);
    config.probe.enabled = true;
    config.probe.marker_locus_enabled = true;
    config
}

fn advance(config: SimConfig, ticks: u64) -> World {
    let mut world = World::new(config).expect("world");
    for _ in 0..ticks {
        world.step();
    }
    world
}

/// Every behavioural quantity a world exposes that is not the checksum.
///
/// The checksum is deliberately **not** here. `state_checksum` hashes the
/// config hash into its preamble, so two worlds whose configs differ always
/// produce different checksums whether or not anything behavioural changed -
/// which is the trap `scheduler.rs`'s
/// `a_checksum_difference_alone_does_not_prove_a_behavioral_difference`
/// records. Only measured quantities can say the probe was inert.
fn behaviour(
    world: &World,
) -> (
    Vec<u64>,
    Vec<i64>,
    i64,
    i64,
    u64,
    sim_core::Counters,
    Vec<u32>,
) {
    let metrics = world.metrics();
    (
        world.organism_ids_view().to_vec(),
        world.biomass_cells().to_vec(),
        metrics.total_energy_milli,
        metrics.total_biomass_milli,
        metrics.max_ancestry_depth as u64,
        world.counters(),
        world
            .structure_census()
            .iter()
            .flat_map(|sample| [sample.nodes, sample.edges, sample.genome_bytes])
            .collect(),
    )
}

// --- A. The action census ----------------------------------------------

#[test]
fn counting_actions_changes_nothing_the_world_computes() {
    // **The honest question this instrument has to answer.** A per-organism
    // accumulator that is written every tick is a write into world state, and
    // "it is only observation" is a claim about the read side. This is the
    // measurement of it: two worlds identical but for the census gate, run
    // long enough for births and deaths, agreeing on every quantity that
    // describes what happened.
    //
    // Deliberately not asserted on the checksum: the config hash is in the
    // checksum preamble and the census section is appended to it, so the two
    // checksums differ by construction and an equality there would be false
    // while an inequality would prove nothing.
    // Three seeds, each verified below to produce births and deaths: a
    // world that sat still would make this equality trivially true.
    for seed in [SEED, SEED ^ 0x9e37, 11, 23] {
        let plain = advance(base_config(seed), 2_000);
        let probed = advance(with_census(seed), 2_000);
        assert!(
            plain.population() > 20 && plain.counters().births_total > 0,
            "the world was too quiet for this comparison to mean anything"
        );
        assert_eq!(
            behaviour(&plain),
            behaviour(&probed),
            "the action census changed the trajectory (seed {seed:#x})"
        );
        // ...and the instrument was actually running, so the equality above
        // is not the equality of two worlds that both counted nothing.
        assert!(plain.action_census().is_empty());
        let counters = probed
            .action_census_counters()
            .expect("the probe world has counters");
        assert!(counters.classified_total > 0);
        assert_eq!(counters.resets_total, 0);
    }
}

#[test]
fn the_locomotion_block_counts_exactly_the_ticks_its_organism_has_been_alive_for() {
    // The partition property, at world scale: exactly one locomotion column
    // per organism per tick means that block sums to the organism's age. A
    // classifier that fell through without recording, or recorded twice,
    // breaks this and nothing else would notice - the counts would still look
    // plausible. The indicator columns are bounded by the age rather than
    // summing to it, because they can co-occur, and that is checked too: a
    // count above the age would mean an indicator fired twice in one tick.
    let world = advance(with_census(SEED), 900);
    let census = world.action_census();
    assert!(census.len() > 20, "too few organisms to check");
    let mut newborns = 0;
    for sample in &census {
        let block: u64 = sample.counts[..LOCOMOTION_CLASS_COUNT]
            .iter()
            .map(|value| u64::from(*value))
            .sum();
        assert_eq!(
            block, sample.age_ticks,
            "organism {} counted {block} locomotion ticks and is {} old",
            sample.id, sample.age_ticks
        );
        for slot in LOCOMOTION_CLASS_COUNT..sim_core::ACTION_CLASS_COUNT {
            assert!(
                u64::from(sample.counts[slot]) <= sample.age_ticks,
                "indicator {slot} fired more often than the organism has lived"
            );
        }
        if sample.age_ticks < 900 {
            newborns += 1;
        }
    }
    assert!(
        newborns > 0,
        "no organism was born mid-run, so this proves nothing about the birth path"
    );
    // The census-wide total counts organism-ticks, so it is the sum of the
    // living locomotion blocks plus everything the dead took with them -
    // strictly larger once anything has died.
    let live: u64 = census
        .iter()
        .map(|sample| {
            sample.counts[..LOCOMOTION_CLASS_COUNT]
                .iter()
                .map(|value| u64::from(*value))
                .sum::<u64>()
        })
        .sum();
    let counters = world.action_census_counters().expect("counters");
    assert!(
        counters.classified_total > live,
        "nothing died, so the dead-take-their-history-with-them path is untested"
    );
}

#[test]
fn the_census_stays_in_lockstep_and_a_desync_is_a_typed_failure() {
    let world = advance(with_census(SEED), 600);
    world
        .check_invariants()
        .expect("a running world is in step");
    assert_eq!(world.action_census().len(), world.population());

    // A desync is reachable only by hand, which is the point: the invariant
    // is what turns a missed push on some future birth path into a named
    // failure instead of an index panic thousands of ticks later. Built
    // through the save, because that is the only public way to hand the
    // kernel a census of the wrong length.
    let mut state = world.export_state();
    state
        .action_census
        .as_mut()
        .expect("the probe world saves a census")
        .counts
        .pop();
    // Pinned to the field name, not to `matches!(_, StateInvalid | Length)`:
    // two layers can refuse this - the length check in `from_state` and
    // `check_invariants` afterwards - and an either-or assertion passes with
    // the near guard deleted.
    assert_eq!(
        World::from_state(state).err(),
        Some(RestoreError::LengthMismatch {
            field: "action_census.counts"
        })
    );

    // The invariant itself, exercised against a world built the only other
    // way it can be: by pushing a row nobody owns.
    let violation = InvariantViolation::ActionCensusDesync {
        organisms: 5,
        census: 6,
    };
    assert_eq!(format!("{violation}"), format!("{violation:?}"));
}

#[test]
fn the_census_survives_a_save_and_the_restored_world_continues_identically() {
    let mut original = advance(with_census(SEED), 700);
    let before = original.action_census();
    assert!(
        before
            .iter()
            .any(|sample| sample.counts.iter().filter(|value| **value > 0).count() > 1),
        "no organism has a nonempty distribution, so a round trip proves nothing"
    );
    // ...and the distribution is spread across the locomotion partition
    // rather than sitting in one column, which is what a saturated-indicator
    // design would have produced and what the single-precedence design
    // actually did produce.
    let mut columns = [0_u64; sim_core::ACTION_CLASS_COUNT];
    for sample in &before {
        for (slot, value) in sample.counts.iter().enumerate() {
            columns[slot] += u64::from(*value);
        }
    }
    assert!(
        columns[..sim_core::LOCOMOTION_CLASS_COUNT]
            .iter()
            .filter(|value| **value > 0)
            .count()
            >= 2,
        "the locomotion partition collapsed into one column: {columns:?}"
    );
    let checksum = original.state_checksum();
    let state = original.export_state();

    let mut restored = World::from_state(state).expect("restores");
    assert_eq!(restored.state_checksum(), checksum);
    // Field by field, not only by checksum: a checksum match also holds for
    // a pair of cancelling defects (zeroing on restore *and* dropping the
    // section from the hash), which is the trap C11.3 records.
    assert_eq!(restored.action_census(), before);
    for _ in 0..200 {
        original.step();
        restored.step();
    }
    assert_eq!(restored.state_checksum(), original.state_checksum());
    assert_eq!(restored.action_census(), original.action_census());
}

#[test]
fn a_restore_refuses_a_census_whose_presence_disagrees_with_the_config() {
    let world = advance(with_census(SEED), 200);
    let mut state = world.export_state();
    state.action_census = None;
    let error = World::from_state(state).expect_err("a missing section is refused");
    // Pinned to the message, for the reason at the top of this file: several
    // presence checks return `StateInvalid`, and `matches!(_, StateInvalid(_))`
    // would pass if this one were deleted and a later one fired instead.
    assert!(
        matches!(&error, RestoreError::StateInvalid(message)
            if message.contains("action census section presence")),
        "wrong guard fired: {error:?}"
    );

    // ...and the mirror image: a census smuggled into a world that has none.
    let plain = advance(base_config(SEED), 200);
    let mut state = plain.export_state();
    state.action_census = Some(sim_core::ActionCensusSaveState {
        counts: vec![[0; sim_core::ACTION_CLASS_COUNT]; plain.population()],
        counters: sim_core::ActionCensusCounters::default(),
    });
    let error = World::from_state(state).expect_err("an unexpected section is refused");
    assert!(
        matches!(&error, RestoreError::StateInvalid(message)
            if message.contains("action census section presence")),
        "wrong guard fired: {error:?}"
    );
}

#[test]
fn a_probe_boundary_zeroes_the_rows_and_is_visible_in_the_checksum() {
    // The reset is a state change and says so. If it were hidden - rows
    // zeroed without a counter - a replay of the same run with the boundary
    // at a different tick would produce the same checksum, and the boundary
    // would be a thing that happened to the measurement and not to the world.
    let mut world = advance(with_census(SEED), 500);
    let before = world.state_checksum();
    assert!(
        world
            .action_census()
            .iter()
            .any(|sample| sample.counts.iter().any(|value| *value > 0))
    );

    world.reset_action_census();
    assert!(
        world
            .action_census()
            .iter()
            .all(|sample| sample.counts.iter().all(|value| *value == 0))
    );
    assert_ne!(world.state_checksum(), before);
    assert_eq!(
        world
            .action_census_counters()
            .expect("counters")
            .resets_total,
        1
    );
    // The population is untouched: a reset is not a cull.
    world.check_invariants().expect("still in step");

    // A second reset with every row already zero still moves the checksum,
    // which is what `resets_total` is for.
    let after_first = world.state_checksum();
    world.reset_action_census();
    assert_ne!(world.state_checksum(), after_first);

    // ...and on a world with no instrument it is a no-op rather than an
    // error, so a scripted probe can run against either arm unchanged.
    let mut plain = advance(base_config(SEED), 50);
    let plain_before = plain.state_checksum();
    plain.reset_action_census();
    assert_eq!(plain.state_checksum(), plain_before);
}

#[test]
fn the_recorded_columns_are_intents_and_the_indicators_saturate_in_this_world() {
    // **A finding, recorded as an assertion so it cannot rot.** With the
    // shipped thresholds an unbound action channel reads 0, `eat_threshold`
    // and `mate_threshold` are both negative, and every founder therefore
    // *asks* to eat and to mate on every tick. Both indicator columns are
    // consequently saturated at the organism's age for the whole of a short
    // run, and only the locomotion block carries information.
    //
    // This is exactly why the class set is a partition block plus
    // indicators rather than one precedence-ordered partition: under a
    // precedence, `Mate` sat at the top and swallowed every other column, and
    // C11.1 would have been measured against a constant.
    let world = advance(with_census(SEED), 900);
    let census = world.action_census();
    assert!(
        census
            .iter()
            .all(|sample| u64::from(sample.counts[ActionClass::Mate as usize]) == sample.age_ticks),
        "the mate indicator is no longer saturated; this note needs re-measuring"
    );
    assert!(
        census
            .iter()
            .any(|sample| sample.counts[ActionClass::TurnRight as usize] > 0
                && sample.counts[ActionClass::MoveAhead as usize] > 0),
        "no organism used more than one heading band, so nothing here can vary"
    );
    assert!(
        census
            .iter()
            .all(|sample| sample.counts[ActionClass::Attack as usize] == 0),
        "the contest section is off, so nothing can have asked to attack"
    );

    // The reason the accumulation site is at the top of `apply_phase2`: an
    // organism that intends to eat where there is no biomass has still asked
    // to eat, and C11.1 is a question about the policy the organism runs, not
    // about what the world let it do. Asserted through the public classifier,
    // which is the same function the world calls.
    assert_eq!(locomotion_class(0.0, 0), ActionClass::Rest);
    assert_eq!(locomotion_class(0.0, 500), ActionClass::MoveAhead);
}

// --- B. The neutral marker locus ---------------------------------------

fn marker_alleles_of(world: &World) -> Vec<(u32, u32, u32)> {
    world
        .marker_census()
        .into_iter()
        .map(|sample| (sample.alleles, sample.sum_value_milli, sample.set_alleles))
        .collect()
}

#[test]
fn a_marker_world_carries_two_inert_alleles_per_founder_and_a_plain_world_carries_none() {
    let plain = World::new(base_config(SEED)).expect("world");
    assert!(
        plain
            .marker_census()
            .iter()
            .all(|sample| sample.alleles == 0)
    );

    let marked = World::new(with_marker(SEED)).expect("world");
    let census = marked.marker_census();
    assert!(!census.is_empty());
    for sample in &census {
        // One per haplotype, and both at the same place `eta` and the plastic
        // flag start: value zero, flag clear. A founder population
        // monomorphic at the marker matches a founder population monomorphic
        // at `eta`, which is what makes the two distributions comparable.
        assert_eq!(sample.alleles, 2);
        assert_eq!(sample.sum_value_milli, 0);
        assert_eq!(sample.set_alleles, 0);
    }
}

#[test]
fn the_marker_is_never_expressed() {
    // **The property the whole control rests on.** Two genomes differing only
    // in their marker alleles must express identically - same network, same
    // traits - or the "control" is under selection too and C11.2's
    // comparison is between two selected quantities.
    let traits = [0.5_f32; sim_core::TRAIT_COUNT];
    let plain = sim_core::founder_from_traits(&traits);
    let marked = sim_core::with_marker_locus(plain.clone());
    let mut loud = marked.clone();
    for haplotype in &mut loud.haplotypes {
        for chromosome in &mut haplotype.chromosomes {
            for locus in chromosome.iter_mut() {
                if let LocusKind::Marker { value, flags } = &mut locus.kind {
                    *value = 1.0;
                    *flags = sim_core::MARKER_FLAG_NEUTRAL;
                }
            }
        }
    }
    assert_ne!(marked, loud, "the two genomes are the same record");
    assert_eq!(marked.express_network(), loud.express_network());
    assert_eq!(marked.express_traits(), loud.express_traits());
    // ...and the marker does not perturb expression relative to a genome that
    // has no marker at all, which is the stronger statement: not merely "the
    // allele value is ignored" but "the locus contributes nothing".
    assert_eq!(plain.express_network(), marked.express_network());
    assert_eq!(plain.express_traits(), marked.express_traits());
    // The growth program is the other reader of a non-controller locus.
    assert_eq!(
        sim_core::rules_of(&plain).len(),
        sim_core::rules_of(&marked).len()
    );
}

#[test]
fn a_world_whose_founders_carry_loud_markers_behaves_exactly_like_one_whose_markers_are_silent() {
    // The expression test above is a statement about one function. This is
    // the statement about the running world, which is what the criterion
    // needs: flip every founder's marker alleles to their opposite extreme
    // through the save path and the trajectory must not move.
    let world = advance(with_marker(SEED), 5);
    let quiet = {
        let mut world = World::from_state(world.export_state()).expect("restores");
        for _ in 0..900 {
            world.step();
        }
        world
    };
    let loud = {
        let mut state = world.export_state();
        let caps = GenomeCaps::provisional();
        let schema2 = state.schema2.as_mut().expect("schema2 section");
        for bytes in schema2.genomes.iter_mut() {
            let mut genome = Genome2::decode(bytes, &caps).expect("decodes");
            for haplotype in &mut genome.haplotypes {
                for chromosome in &mut haplotype.chromosomes {
                    for locus in chromosome.iter_mut() {
                        if let LocusKind::Marker { value, flags } = &mut locus.kind {
                            *value = 1.0;
                            *flags = sim_core::MARKER_FLAG_NEUTRAL;
                        }
                    }
                }
            }
            *bytes = genome.encode();
        }
        let mut world = World::from_state(state).expect("the loud genomes are legal records");
        for _ in 0..900 {
            world.step();
        }
        world
    };
    assert_ne!(
        marker_alleles_of(&quiet),
        marker_alleles_of(&loud),
        "the two worlds carry the same alleles, so this compares a value with itself"
    );
    assert_eq!(
        behaviour(&quiet),
        behaviour(&loud),
        "the marker locus reached the phenotype"
    );
}

#[test]
fn the_marker_segregates_and_recombines_like_the_locus_beside_it() {
    // Inheritance and recombination need no code of their own - `meiosis.rs`
    // copies whole loci by homology id - so the thing to check is that the
    // claim survives contact with a real run rather than that a new code path
    // works.
    //
    // The founder population is monomorphic at the marker, by design (it
    // starts where `eta` starts), so there is nothing to segregate until
    // mutation creates variation - and at the shipped per-locus rate that
    // takes far longer than a test can run. Variation is therefore *seeded*
    // through the save path, exactly as `phase11_learning`'s `plastic_world`
    // seeds plastic flags, and for the same reason: waiting for evolution to
    // produce the starting condition would make this test slow, seed
    // dependent, and a test of the mutation rate rather than of inheritance.
    // The genomes it writes are ordinary legal records.
    let world = advance(with_marker(SEED), 5);
    let mut state = world.export_state();
    let caps = GenomeCaps::provisional();
    let schema2 = state.schema2.as_mut().expect("schema2 section");
    for (index, bytes) in schema2.genomes.iter_mut().enumerate() {
        let mut genome = Genome2::decode(bytes, &caps).expect("decodes");
        // Every founder heterozygous the same way: haplotype 0 carries the
        // high allele with the neutral bit set, haplotype 1 the low allele.
        // Half the population gets the reverse, so a run that lost one whole
        // haplotype slot would be visible rather than symmetric.
        let flip = index % 2 == 1;
        for (slot, haplotype) in genome.haplotypes.iter_mut().enumerate() {
            let high = (slot == 0) != flip;
            for chromosome in &mut haplotype.chromosomes {
                for locus in chromosome.iter_mut() {
                    if let LocusKind::Marker { value, flags } = &mut locus.kind {
                        *value = if high { 1.0 } else { 0.0 };
                        *flags = if high {
                            sim_core::MARKER_FLAG_NEUTRAL
                        } else {
                            0
                        };
                    }
                }
            }
        }
        *bytes = genome.encode();
    }
    let mut world = World::from_state(state).expect("the seeded genomes are legal records");
    for sample in world.marker_census() {
        assert_eq!(
            (sample.alleles, sample.sum_value_milli, sample.set_alleles),
            (2, 1_000, 1)
        );
    }
    for _ in 0..6_000 {
        world.step();
    }
    assert!(world.counters().births_total > 0, "no descendants");

    let census = world.marker_census();
    assert!(!census.is_empty());
    assert!(
        census.iter().all(|sample| sample.alleles == 2),
        "an organism lost a marker allele; inheritance is not carrying it"
    );
    // Segregation produced all three genotypes. A `PairedWholeGenome`-style
    // copy, or a meiosis that always read haplotype 0, would give only the
    // parental heterozygote and this is where that shows.
    let homozygous_high = census.iter().filter(|s| s.sum_value_milli == 2_000).count();
    let homozygous_low = census.iter().filter(|s| s.sum_value_milli == 0).count();
    let heterozygous = census.iter().filter(|s| s.sum_value_milli == 1_000).count();
    assert!(
        homozygous_high > 0 && homozygous_low > 0 && heterozygous > 0,
        "the marker did not segregate: high {homozygous_high}, low {homozygous_low}, \
         het {heterozygous}"
    );
    // The flag allele travelled with its own value allele rather than being
    // recombined independently of it - they are one locus, so `set_alleles`
    // must equal the count of high alleles in every organism.
    for sample in &census {
        assert_eq!(
            sample.set_alleles,
            sample.sum_value_milli / 1_000,
            "the marker's two alleles came apart, so they are not one locus"
        );
    }
}

#[test]
fn a_marker_locus_round_trips_through_the_codec_and_out_of_range_alleles_are_refused() {
    let caps = GenomeCaps::provisional();
    let genome = sim_core::with_marker_locus(sim_core::founder_from_traits(
        &[0.5_f32; sim_core::TRAIT_COUNT],
    ));
    let bytes = genome.encode();
    assert_eq!(Genome2::decode(&bytes, &caps).expect("decodes"), genome);

    // The marker is refused out of range rather than reduced, which is
    // `eta`'s behaviour and not the regulatory locus's. A marker that was
    // silently reduced would have a different mutational neighbourhood at
    // the clamp than the gene it controls for - and the clamp is exactly
    // where a walk starting at 0.0 spends its time.
    let mut broken = genome.clone();
    for haplotype in &mut broken.haplotypes {
        for chromosome in &mut haplotype.chromosomes {
            for locus in chromosome.iter_mut() {
                if let LocusKind::Marker { value, .. } = &mut locus.kind {
                    *value = 1.5;
                }
            }
        }
    }
    assert_eq!(
        Genome2::decode(&broken.encode(), &caps),
        Err(sim_core::Genome2Error::ValueOutOfRange("marker value"))
    );

    let mut flagged = genome.clone();
    for haplotype in &mut flagged.haplotypes {
        for chromosome in &mut haplotype.chromosomes {
            for locus in chromosome.iter_mut() {
                if let LocusKind::Marker { flags, .. } = &mut locus.kind {
                    *flags = 0x80;
                }
            }
        }
    }
    assert_eq!(
        Genome2::decode(&flagged.encode(), &caps),
        Err(sim_core::Genome2Error::ValueOutOfRange("marker flags"))
    );
}

#[test]
fn the_marker_sits_between_the_two_founder_edges() {
    // Linkage is the half of "matched control" that a rate test cannot see.
    // Crossover positions are drawn over merged rank, so what matters is that
    // the marker is one rank from each plastic-capable edge - as tightly
    // linked to each as those two edges are to each other. A marker parked at
    // the end of the chromosome would recombine away faster than the genes it
    // controls for and would drift under a different regime.
    let genome = sim_core::with_marker_locus(sim_core::founder_from_traits(
        &[0.5_f32; sim_core::TRAIT_COUNT],
    ));
    let chromosome = &genome.haplotypes[0].chromosomes[0];
    let marker = chromosome
        .iter()
        .position(|locus| matches!(locus.kind, LocusKind::Marker { .. }))
        .expect("the founder carries a marker");
    let edges: Vec<usize> = chromosome
        .iter()
        .enumerate()
        .filter(|(_, locus)| matches!(locus.kind, LocusKind::Edge { .. }))
        .map(|(index, _)| index)
        .collect();
    assert_eq!(edges.len(), 2);
    assert_eq!(marker, edges[0] + 1);
    assert_eq!(marker + 1, edges[1]);
    assert_eq!(chromosome[marker].homology_id, MARKER_HOMOLOGY_ID);
    // Sortedness is a decode-time invariant, so an insertion that broke it
    // would be a refused genome rather than a mis-linked one.
    assert!(
        chromosome
            .windows(2)
            .all(|pair| pair[0].homology_id < pair[1].homology_id)
    );
}

#[test]
fn a_marker_allele_is_not_part_of_structural_identity() {
    // Two markers at the same slot are the same structure whatever their
    // alleles say, exactly as two edges are the same structure whatever their
    // weights say - and the neutral flag is excluded for the same reason
    // `EDGE_FLAG_PLASTIC` is, so the control behaves under alignment exactly
    // as the thing it controls for.
    let make = |value: f32, flags: u8| sim_core::Locus {
        homology_id: MARKER_HOMOLOGY_ID,
        gene_lineage_id: 1,
        mutation_event_id: 2,
        kind: LocusKind::Marker { value, flags },
    };
    let quiet = make(0.0, 0);
    let loud = make(1.0, sim_core::MARKER_FLAG_NEUTRAL);
    assert_eq!(quiet.structural_signature(), loud.structural_signature());
    // ...and it is still distinguishable from every other locus type, so the
    // tag is genuinely in the hash.
    let edge = sim_core::Locus {
        homology_id: MARKER_HOMOLOGY_ID,
        gene_lineage_id: 1,
        mutation_event_id: 2,
        kind: LocusKind::Edge {
            source: sim_core::STRUCTURAL_HOMOLOGY_BASE + 1,
            target: sim_core::STRUCTURAL_HOMOLOGY_BASE + 2,
            weight: 0.0,
            flags: 0,
            plasticity: sim_core::PlasticityGenes::inert(),
        },
    };
    assert_ne!(quiet.structural_signature(), edge.structural_signature());
}

// --- Configuration gates ------------------------------------------------

#[test]
fn a_probe_feature_without_its_section_is_refused_by_the_guard_that_names_it() {
    // Two layers can refuse each of these, so each assertion is pinned to the
    // diagnostic of the guard that is supposed to fire. `matches!(err,
    // PhysiologyRange(_, _))` would pass with any one of the three deleted.
    let mut config = base_config(SEED);
    config.probe.action_census_enabled = true;
    assert_eq!(
        config.validate(),
        Err(sim_core::ConfigError::PhysiologyRange(
            "a probe feature is enabled while probe.enabled is false",
            0
        ))
    );

    let mut config = base_config(SEED);
    config.probe.enabled = true;
    config.probe.marker_locus_enabled = true;
    config.genome2.enabled = false;
    config.plasticity.enabled = false;
    config.genome2.mutation.plasticity_enabled = false;
    assert_eq!(
        config.validate(),
        Err(sim_core::ConfigError::PhysiologyRange(
            "probe.marker_locus_enabled requires genome2",
            0
        ))
    );

    let mut config = SimConfig::phase1_default(SEED);
    config.probe.enabled = true;
    config.probe.action_census_enabled = true;
    assert!(!config.phase2.enabled);
    assert_eq!(
        config.validate(),
        Err(sim_core::ConfigError::PhysiologyRange(
            "probe.action_census_enabled requires phase2",
            0
        ))
    );

    // Both features together validate, which is the arm C11.1 and C11.2 are
    // measured in.
    let mut config = base_config(SEED);
    config.probe.enabled = true;
    config.probe.action_census_enabled = true;
    config.probe.marker_locus_enabled = true;
    config
        .validate()
        .expect("the measured arm is a legal config");
}

#[test]
fn a_disabled_probe_section_is_excluded_from_the_config_hash() {
    // D-014's rule, and the reason all five fixtures survive. Checked with
    // the fields *moved*, which is the assertion an `enabled` check alone
    // would not make.
    let base = base_config(SEED);
    let mut moved = base;
    moved.probe.action_census_enabled = true;
    moved.probe.marker_locus_enabled = true;
    assert_eq!(base.stable_hash(), moved.stable_hash());

    // Enabled, each field reaches the hash: two conditions that differ only
    // in an instrument must not be one experiment under two names.
    let mut enabled = base;
    enabled.probe.enabled = true;
    let reference = enabled.stable_hash();
    assert_ne!(reference, base.stable_hash());
    let mutators: [fn(&mut SimConfig); 2] = [
        |config| config.probe.action_census_enabled = true,
        |config| config.probe.marker_locus_enabled = true,
    ];
    for (index, mutate) in mutators.into_iter().enumerate() {
        let mut changed = enabled;
        mutate(&mut changed);
        assert_ne!(changed.stable_hash(), reference, "field {index}");
    }
}
