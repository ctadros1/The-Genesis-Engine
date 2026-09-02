//! C14.2: is mate choice informed? (`lifesim-assortment-v1`)
//!
//! The census reads `MateChoice` events alone. Each record carries the
//! chosen candidate's TRUE cue values and the candidate set's TRUE cue
//! sums, so the per-choice deviation of the chosen from its own
//! opportunity mean is exact integer arithmetic with the opportunity
//! denominator built in - no re-simulation, no genotype access, and under
//! the null hypothesis of cue-blind choice its expectation is zero for
//! every candidate-set size.
//!
//! The world statistic per cue is the candidate-weighted mean deviation:
//! `sum(n_i * chosen_i - sum_i) / sum(n_i)`, in milli, truncated toward
//! zero - each choice contributes its deviation times its own opportunity
//! count, which keeps the arithmetic exact in i128 until one final
//! division. Choices with a single candidate carry no information about
//! preference and are excluded AND counted, never silently dropped. The
//! P-scramble arm is the empirical null: its choices are cue-blind by
//! construction, so the A-versus-P-scramble seed-paired contrast is the
//! decision form and this census carries the per-arm halves.

use sim_core::{Event, EventKind};

pub const ASSORTMENT_POLICY_VERSION: &str = "lifesim-assortment-v1";
pub const CUE_COUNT: usize = 9;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AssortmentCensus {
    /// Every `MateChoice` record seen.
    pub choices_total: u32,
    /// Records with at least two candidates - the ones that inform.
    pub choices_used: u32,
    /// Records with exactly one candidate: excluded, counted.
    pub single_candidate: u32,
    /// Records whose choice ran against a permuted cue assignment.
    pub scrambled: u32,
    /// Candidate-weighted mean deviation of the chosen cue from its
    /// opportunity mean, per cue, milli, truncated toward zero. `None`
    /// when no choice informed (zero weight).
    pub deviation_milli: [Option<i64>; CUE_COUNT],
    /// The shared denominator: total candidates over the used choices.
    pub weight_total: u64,
}

/// Reduce one world's event log to its assortment census.
pub fn assortment_census(events: &[Event]) -> AssortmentCensus {
    let mut census = AssortmentCensus::default();
    let mut numerators = [0_i128; CUE_COUNT];
    for event in events {
        let EventKind::MateChoice {
            candidates,
            scrambled,
            chosen_cues_milli,
            cue_sums_milli,
            ..
        } = event.kind
        else {
            continue;
        };
        census.choices_total += 1;
        if scrambled {
            census.scrambled += 1;
        }
        if candidates < 2 {
            census.single_candidate += 1;
            continue;
        }
        census.choices_used += 1;
        census.weight_total += u64::from(candidates);
        for cue in 0..CUE_COUNT {
            numerators[cue] += i128::from(candidates) * i128::from(chosen_cues_milli[cue])
                - i128::from(cue_sums_milli[cue]);
        }
    }
    if census.weight_total > 0 {
        for cue in 0..CUE_COUNT {
            census.deviation_milli[cue] =
                Some((numerators[cue] / census.weight_total as i128) as i64);
        }
    }
    census
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim_core::Event;

    fn choice(candidates: u32, scrambled: bool, chosen: [i32; 9], sums: [i64; 9]) -> Event {
        Event {
            tick: 1,
            kind: EventKind::MateChoice {
                chooser: 1,
                chosen: 2,
                candidates,
                scrambled,
                chosen_cues_milli: chosen,
                cue_sums_milli: sums,
            },
        }
    }

    #[test]
    fn a_chosen_cue_above_its_opportunity_mean_deviates_positive_exactly() {
        // Two candidates: chosen cue 7 at 800 against a sum of 1_400, so
        // the other sits at 600; deviation = (2*800 - 1400)/2 = +100.
        let events = [choice(
            2,
            false,
            [0, 0, 0, 0, 0, 0, 0, 800, 0],
            [0, 0, 0, 0, 0, 0, 0, 1_400, 0],
        )];
        let census = assortment_census(&events);
        assert_eq!(census.choices_used, 1);
        assert_eq!(census.weight_total, 2);
        assert_eq!(census.deviation_milli[7], Some(100));
        assert_eq!(census.deviation_milli[0], Some(0));
    }

    #[test]
    fn single_candidate_choices_are_excluded_and_counted() {
        let events = [
            choice(1, false, [500; 9], [500; 9]),
            choice(
                3,
                true,
                [0, 0, 0, 0, 0, 0, 0, 900, 0],
                [0, 0, 0, 0, 0, 0, 0, 1_800, 0],
            ),
        ];
        let census = assortment_census(&events);
        assert_eq!(census.choices_total, 2);
        assert_eq!(census.single_candidate, 1);
        assert_eq!(census.choices_used, 1);
        assert_eq!(census.scrambled, 1);
        // (3*900 - 1800)/3 = +300.
        assert_eq!(census.deviation_milli[7], Some(300));
    }

    #[test]
    fn no_informing_choice_reports_none_not_zero() {
        let events = [choice(1, false, [500; 9], [500; 9])];
        let census = assortment_census(&events);
        assert_eq!(census.deviation_milli[7], None);
        assert_eq!(census.weight_total, 0);
    }

    #[test]
    fn a_cue_blind_choice_stream_deviates_zero_by_symmetry() {
        // The same two-candidate set chosen both ways: the deviations
        // cancel exactly, which is the null's expectation made literal.
        let events = [
            choice(
                2,
                false,
                [0, 0, 0, 0, 0, 0, 0, 800, 0],
                [0, 0, 0, 0, 0, 0, 0, 1_400, 0],
            ),
            choice(
                2,
                false,
                [0, 0, 0, 0, 0, 0, 0, 600, 0],
                [0, 0, 0, 0, 0, 0, 0, 1_400, 0],
            ),
        ];
        let census = assortment_census(&events);
        assert_eq!(census.deviation_milli[7], Some(0));
    }
}
