//! Comparison report across conditions.
//!
//! The report's job is not to decide anything. It is to make an aggregation
//! either correct or refused:
//!
//! > "the comparison report refuses to aggregate runs whose hashes differ in
//! > any field the report does not explicitly name as the varied field"
//! > (A5.6)
//!
//! Config hashes alone cannot support that check. A hash tells you two
//! configs differ; it never tells you *where*, and every run in a campaign
//! has a different config hash anyway because the seed is in it. So the
//! report recomputes each run's effective config from the manifest's
//! embedded campaign and compares field by field, then refuses on the first
//! difference outside the declared varied set.
//!
//! What the report deliberately does **not** do is infer anything. It
//! reports counts, ranges, medians, and paired per-seed differences with a
//! sign count. No p-value, no effect size, no threshold. Phase 5 builds the
//! instrument; the phases that state hypotheses state their own
//! prespecified thresholds, and a threshold chosen after seeing the data is
//! a different experiment (`planning/backlog.md`).

use crate::fields;
use crate::manifest::{Manifest, RunResult};
use std::fmt;

pub const REPORT_POLICY_VERSION: &str = "lifesim-compare-v1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReportRefusal {
    /// Two conditions differ in a field the report was not told to vary.
    UndeclaredField {
        left: String,
        right: String,
        field: &'static str,
    },
    /// A recorded run does not match the config its campaign would produce.
    ConfigHashMismatch {
        condition: String,
        seed: u64,
        recorded: u64,
        recomputed: u64,
    },
    /// Conditions were run on different seed sets, so nothing is paired.
    SeedSetMismatch {
        left: String,
        right: String,
        only_in_left: Vec<u64>,
        only_in_right: Vec<u64>,
    },
    EmptyCondition(String),
    /// Fewer than two conditions: there is nothing to compare.
    NothingToCompare,
    /// Some worlds failed, so the surviving set is not the declared design.
    IncompleteCampaign {
        failed: usize,
        expected: usize,
    },
    Campaign(String),
}

impl fmt::Display for ReportRefusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UndeclaredField { left, right, field } => write!(
                formatter,
                "refusing to aggregate: conditions '{left}' and '{right}' differ in '{field}', \
                 which is not a declared varied field. Either declare it with `vary {field}` \
                 or these are not two conditions of one experiment."
            ),
            Self::ConfigHashMismatch {
                condition,
                seed,
                recorded,
                recomputed,
            } => write!(
                formatter,
                "refusing to aggregate: condition '{condition}' seed 0x{seed:016x} recorded \
                 config hash 0x{recorded:016x} but its campaign produces 0x{recomputed:016x}; \
                 the manifest does not describe the runs it contains"
            ),
            Self::SeedSetMismatch {
                left,
                right,
                only_in_left,
                only_in_right,
            } => write!(
                formatter,
                "refusing to aggregate: '{left}' and '{right}' ran different seed sets \
                 ({} only in '{left}', {} only in '{right}'); paired comparison is undefined",
                only_in_left.len(),
                only_in_right.len()
            ),
            Self::EmptyCondition(name) => write!(
                formatter,
                "refusing to aggregate: condition '{name}' has no successful runs"
            ),
            Self::NothingToCompare => write!(
                formatter,
                "refusing to report: a comparison needs at least two conditions"
            ),
            Self::IncompleteCampaign { failed, expected } => write!(
                formatter,
                "refusing to aggregate: {failed} of {expected} worlds failed; \
                 report the failures or re-run them before comparing"
            ),
            Self::Campaign(message) => write!(formatter, "campaign error: {message}"),
        }
    }
}

impl std::error::Error for ReportRefusal {}

/// Exact descriptive statistics over one metric across a condition's seeds.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Summary {
    pub count: usize,
    pub min: i64,
    pub max: i64,
    /// Lower median for even counts, so the value is always an observation.
    pub median: i64,
    /// Exact mean as a numerator/denominator pair; no rounding is applied
    /// before the reader sees it.
    pub sum: i128,
}

impl Summary {
    fn of(values: &mut [i64]) -> Self {
        values.sort_unstable();
        let count = values.len();
        Self {
            count,
            min: values.first().copied().unwrap_or(0),
            max: values.last().copied().unwrap_or(0),
            median: if count == 0 {
                0
            } else {
                values[(count - 1) / 2]
            },
            sum: values.iter().map(|&value| i128::from(value)).sum(),
        }
    }

    /// Mean scaled by 1,000 so it prints exactly without float formatting.
    pub fn mean_milli(&self) -> i128 {
        if self.count == 0 {
            0
        } else {
            self.sum * 1_000 / self.count as i128
        }
    }
}

/// Extracts one world-level quantity from a finished run.
pub type MetricFn = fn(&RunResult) -> i64;

/// The metrics the report summarizes. Each is a world-level quantity: per
/// ADR-0022 A5 the world is the replicate, so nothing here is a
/// per-organism value.
pub const METRICS: &[(&str, MetricFn)] = &[
    ("population", |run| run.population as i64),
    ("births_total", |run| run.counters.births_total as i64),
    ("deaths_starvation_total", |run| {
        run.counters.deaths_starvation_total as i64
    }),
    ("deaths_old_age_total", |run| {
        run.counters.deaths_old_age_total as i64
    }),
    ("max_ancestry_depth", |run| {
        i64::from(run.max_ancestry_depth)
    }),
    ("total_energy_milli", |run| run.total_energy_milli),
    ("total_biomass_milli", |run| run.total_biomass_milli),
    ("attacks_total", |run| run.attacks_total as i64),
    ("deaths_by_damage_total", |run| {
        run.deaths_by_damage_total as i64
    }),
    // Phase 12 artifact half. Zero when the section is disabled: these are
    // descriptive summaries, and `lifesim artifact` is where absence is
    // told apart from zero and the criteria are decided.
    ("artifact_successes", |run| {
        run.artifact.map_or(0, |a| a.counters.successes() as i64)
    }),
    ("artifact_picked_up", |run| {
        run.artifact.map_or(0, |a| a.counters.picked_up as i64)
    }),
    ("artifact_placed", |run| {
        run.artifact.map_or(0, |a| a.counters.placed as i64)
    }),
    ("artifact_combined", |run| {
        run.artifact.map_or(0, |a| a.counters.combined as i64)
    }),
    ("artifact_struck_terrain", |run| {
        run.artifact.map_or(0, |a| a.counters.struck_terrain as i64)
    }),
    ("artifact_refusals", |run| {
        run.artifact.map_or(0, |a| a.counters.refusals() as i64)
    }),
    ("artifact_cap_refusals", |run| {
        run.artifact.map_or(0, |a| a.counters.cap_refusals() as i64)
    }),
    ("artifact_objects_total", |run| {
        run.artifact.map_or(0, |a| a.objects_total as i64)
    }),
    ("artifact_composites_depth2", |run| {
        run.artifact.map_or(0, |a| a.composites_depth2 as i64)
    }),
    ("artifact_placed_total", |run| {
        run.artifact.map_or(0, |a| a.placed_total as i64)
    }),
];

#[derive(Clone, Debug)]
pub struct ConditionSummary {
    pub name: String,
    pub delta_hash: u64,
    pub seeds: Vec<u64>,
    pub extinct_worlds: usize,
    pub metrics: Vec<(&'static str, Summary)>,
}

/// Paired per-seed comparison of one condition against the baseline.
#[derive(Clone, Debug)]
pub struct PairedComparison {
    pub condition: String,
    pub metric: &'static str,
    /// Seeds where the condition's value exceeded the baseline's.
    pub greater: usize,
    pub equal: usize,
    pub less: usize,
    pub seeds: usize,
    pub min_difference: i64,
    pub max_difference: i64,
}

#[derive(Clone, Debug)]
pub struct ComparisonReport {
    pub campaign_id: String,
    pub campaign_hash: u64,
    pub policy_version: &'static str,
    pub varied: Vec<String>,
    pub baseline: String,
    pub conditions: Vec<ConditionSummary>,
    pub paired: Vec<PairedComparison>,
    pub build_version: String,
}

/// Build a comparison, or refuse with an actionable reason.
///
/// `baseline` names the condition every other condition is paired against;
/// `None` uses the campaign's first declared condition, which is the
/// control by convention.
pub fn compare(
    manifest: &Manifest,
    baseline: Option<&str>,
) -> Result<ComparisonReport, ReportRefusal> {
    let campaign = &manifest.campaign;
    if campaign.conditions.len() < 2 {
        return Err(ReportRefusal::NothingToCompare);
    }
    if !manifest.failed.is_empty() {
        return Err(ReportRefusal::IncompleteCampaign {
            failed: manifest.failed.len(),
            expected: campaign.run_count(),
        });
    }

    // Every recorded run must be the run its campaign describes.
    for condition in &campaign.conditions {
        let runs = manifest.runs_for(&condition.name);
        if runs.is_empty() {
            return Err(ReportRefusal::EmptyCondition(condition.name.clone()));
        }
        for run in &runs {
            let config = campaign
                .config_for(condition, run.seed)
                .map_err(|error| ReportRefusal::Campaign(error.to_string()))?;
            let recomputed = config.stable_hash();
            if recomputed != run.config_hash {
                return Err(ReportRefusal::ConfigHashMismatch {
                    condition: condition.name.clone(),
                    seed: run.seed,
                    recorded: run.config_hash,
                    recomputed,
                });
            }
        }
    }

    // The aggregation precondition: nothing outside the declared varied set
    // may differ between any two conditions.
    let probe = campaign.seeds[0];
    for left in 0..campaign.conditions.len() {
        for right in (left + 1)..campaign.conditions.len() {
            let left_condition = &campaign.conditions[left];
            let right_condition = &campaign.conditions[right];
            let left_config = campaign
                .config_for(left_condition, probe)
                .map_err(|error| ReportRefusal::Campaign(error.to_string()))?;
            let right_config = campaign
                .config_for(right_condition, probe)
                .map_err(|error| ReportRefusal::Campaign(error.to_string()))?;
            for field in fields::differing_fields(&left_config, &right_config) {
                if !campaign.varied.iter().any(|declared| declared == field) {
                    return Err(ReportRefusal::UndeclaredField {
                        left: left_condition.name.clone(),
                        right: right_condition.name.clone(),
                        field,
                    });
                }
            }
            // Paired comparison needs matched seed sets.
            let left_seeds: Vec<u64> = manifest
                .runs_for(&left_condition.name)
                .iter()
                .map(|run| run.seed)
                .collect();
            let right_seeds: Vec<u64> = manifest
                .runs_for(&right_condition.name)
                .iter()
                .map(|run| run.seed)
                .collect();
            if left_seeds != right_seeds {
                let only_in_left: Vec<u64> = left_seeds
                    .iter()
                    .copied()
                    .filter(|seed| !right_seeds.contains(seed))
                    .collect();
                let only_in_right: Vec<u64> = right_seeds
                    .iter()
                    .copied()
                    .filter(|seed| !left_seeds.contains(seed))
                    .collect();
                return Err(ReportRefusal::SeedSetMismatch {
                    left: left_condition.name.clone(),
                    right: right_condition.name.clone(),
                    only_in_left,
                    only_in_right,
                });
            }
        }
    }

    let baseline_name = baseline
        .map(str::to_owned)
        .unwrap_or_else(|| campaign.conditions[0].name.clone());
    if !campaign
        .conditions
        .iter()
        .any(|condition| condition.name == baseline_name)
    {
        return Err(ReportRefusal::EmptyCondition(baseline_name));
    }

    let mut conditions = Vec::new();
    for condition in &campaign.conditions {
        let runs = manifest.runs_for(&condition.name);
        let metrics = METRICS
            .iter()
            .map(|(name, extract)| {
                let mut values: Vec<i64> = runs.iter().map(|run| extract(run)).collect();
                (*name, Summary::of(&mut values))
            })
            .collect();
        conditions.push(ConditionSummary {
            name: condition.name.clone(),
            delta_hash: condition.delta_hash(),
            seeds: runs.iter().map(|run| run.seed).collect(),
            extinct_worlds: runs.iter().filter(|run| run.extinct).count(),
            metrics,
        });
    }

    let baseline_runs = manifest.runs_for(&baseline_name);
    let mut paired = Vec::new();
    for condition in &campaign.conditions {
        if condition.name == baseline_name {
            continue;
        }
        let runs = manifest.runs_for(&condition.name);
        for (metric, extract) in METRICS {
            let mut comparison = PairedComparison {
                condition: condition.name.clone(),
                metric,
                greater: 0,
                equal: 0,
                less: 0,
                seeds: runs.len(),
                min_difference: i64::MAX,
                max_difference: i64::MIN,
            };
            for (run, base) in runs.iter().zip(baseline_runs.iter()) {
                let difference = extract(run).saturating_sub(extract(base));
                match difference.cmp(&0) {
                    std::cmp::Ordering::Greater => comparison.greater += 1,
                    std::cmp::Ordering::Equal => comparison.equal += 1,
                    std::cmp::Ordering::Less => comparison.less += 1,
                }
                comparison.min_difference = comparison.min_difference.min(difference);
                comparison.max_difference = comparison.max_difference.max(difference);
            }
            if runs.is_empty() {
                comparison.min_difference = 0;
                comparison.max_difference = 0;
            }
            paired.push(comparison);
        }
    }

    Ok(ComparisonReport {
        campaign_id: campaign.id.clone(),
        campaign_hash: campaign.stable_hash(),
        policy_version: REPORT_POLICY_VERSION,
        varied: campaign.varied.clone(),
        baseline: baseline_name,
        conditions,
        paired,
        build_version: manifest.build_version.clone(),
    })
}

impl ComparisonReport {
    /// Deterministic plain-text rendering.
    pub fn render(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("campaign {}\n", self.campaign_id));
        out.push_str(&format!("campaign-hash 0x{:016x}\n", self.campaign_hash));
        out.push_str(&format!("report-policy {}\n", self.policy_version));
        out.push_str(&format!("build {}\n", self.build_version));
        out.push_str(&format!("baseline {}\n", self.baseline));
        out.push_str(&format!("varied {}\n", self.varied.join(",")));
        for condition in &self.conditions {
            out.push_str(&format!(
                "\ncondition {} delta-hash 0x{:016x} seeds {} extinct-worlds {}\n",
                condition.name,
                condition.delta_hash,
                condition.seeds.len(),
                condition.extinct_worlds
            ));
            for (metric, summary) in &condition.metrics {
                out.push_str(&format!(
                    "  {metric} n={} min={} median={} max={} mean_milli={}\n",
                    summary.count,
                    summary.min,
                    summary.median,
                    summary.max,
                    summary.mean_milli()
                ));
            }
        }
        if !self.paired.is_empty() {
            out.push_str("\npaired differences versus baseline (per seed)\n");
            for comparison in &self.paired {
                out.push_str(&format!(
                    "  {} {} seeds={} greater={} equal={} less={} min_diff={} max_diff={}\n",
                    comparison.condition,
                    comparison.metric,
                    comparison.seeds,
                    comparison.greater,
                    comparison.equal,
                    comparison.less,
                    comparison.min_difference,
                    comparison.max_difference
                ));
            }
        }
        out.push_str(
            "\nnote: these are descriptive statistics only. No significance test, effect \
             size, or threshold is applied here. A phase that states a hypothesis states its \
             own prespecified threshold and seed count before its campaign runs.\n",
        );
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::campaign::Campaign;
    use crate::manifest::{FailedRun, Manifest};
    use crate::scheduler::{SchedulerOptions, run_campaign};

    const SOURCE: &str = "\
campaign report-test
ticks 80
seeds 1..4
base preset phase2
base cells_x 32
base cells_y 32
base initial_organisms 20
base max_entities 200
condition control
condition treatment
set treatment crowding_cost_milli_per_s 900
vary crowding_cost_milli_per_s
output events off
output snapshots off
";

    fn manifest_from(source: &str) -> Manifest {
        let campaign = Campaign::parse(source).unwrap();
        let runs = run_campaign(&campaign, &SchedulerOptions::in_memory(2))
            .into_iter()
            .map(|result| result.expect("run succeeded"))
            .collect();
        Manifest {
            campaign,
            campaign_source: source.to_owned(),
            build_version: "lifesim-test".to_owned(),
            behavior_policy_versions: vec![
                sim_core::BEHAVIOR_POLICY_VERSION.to_owned(),
                sim_core::PHASE2_BEHAVIOR_POLICY_VERSION.to_owned(),
            ],
            rng_algorithm_version: sim_core::RNG_ALGORITHM_VERSION.to_owned(),
            worldgen_version: sim_core::WORLDGEN_VERSION.to_owned(),
            genome_schema_version: sim_core::GENOME_SCHEMA_VERSION,
            event_schema_version: sim_core::EVENT_SCHEMA_VERSION,
            analysis_versions: Vec::new(),
            workers: 2,
            runs,
            failed: Vec::new(),
        }
    }

    #[test]
    fn a_well_formed_campaign_compares_and_renders_deterministically() {
        let manifest = manifest_from(SOURCE);
        let report = compare(&manifest, None).expect("comparison");
        assert_eq!(report.baseline, "control");
        assert_eq!(report.conditions.len(), 2);
        assert_eq!(report.varied, vec!["crowding_cost_milli_per_s"]);
        // One paired comparison per metric for the single non-baseline
        // condition.
        assert_eq!(report.paired.len(), METRICS.len());
        for comparison in &report.paired {
            assert_eq!(comparison.seeds, 4);
            assert_eq!(
                comparison.greater + comparison.equal + comparison.less,
                4,
                "every seed must be classified"
            );
        }
        assert_eq!(report.render(), report.render());
        assert!(report.render().contains("No significance test"));
    }

    #[test]
    fn an_undeclared_difference_is_refused_with_the_field_named() {
        // Build a manifest, then swap in a campaign whose `vary` no longer
        // covers the difference. The manifest's own load-time validation
        // would catch this, so the refusal is constructed directly against
        // `compare` to prove the report is not merely trusting the loader.
        let manifest = manifest_from(SOURCE);
        let mut campaign = manifest.campaign.clone();
        campaign.varied.clear();
        let tampered = Manifest {
            campaign,
            ..manifest
        };
        let refusal = compare(&tampered, None).unwrap_err();
        assert!(matches!(
            refusal,
            ReportRefusal::UndeclaredField {
                field: "crowding_cost_milli_per_s",
                ..
            }
        ));
        assert!(
            refusal
                .to_string()
                .contains("vary crowding_cost_milli_per_s")
        );
    }

    #[test]
    fn a_run_that_does_not_match_its_campaign_is_refused() {
        let mut manifest = manifest_from(SOURCE);
        manifest.runs[0].config_hash ^= 0xdead_beef;
        assert!(matches!(
            compare(&manifest, None),
            Err(ReportRefusal::ConfigHashMismatch { .. })
        ));
    }

    #[test]
    fn mismatched_seed_sets_are_refused() {
        let mut manifest = manifest_from(SOURCE);
        // Drop one treatment run so the seed sets no longer pair.
        let position = manifest
            .runs
            .iter()
            .position(|run| run.condition == "treatment")
            .unwrap();
        manifest.runs.remove(position);
        assert!(matches!(
            compare(&manifest, None),
            Err(ReportRefusal::SeedSetMismatch { .. })
        ));
    }

    #[test]
    fn a_campaign_with_failures_is_refused_rather_than_silently_reduced() {
        let mut manifest = manifest_from(SOURCE);
        manifest.failed.push(FailedRun {
            index: 3,
            condition: "control".to_owned(),
            seed: 4,
            reason: "world panicked".to_owned(),
        });
        assert!(matches!(
            compare(&manifest, None),
            Err(ReportRefusal::IncompleteCampaign { failed: 1, .. })
        ));
    }

    #[test]
    fn summary_statistics_are_exact() {
        let mut values = vec![5_i64, 1, 9, 3];
        let summary = Summary::of(&mut values);
        assert_eq!(summary.count, 4);
        assert_eq!(summary.min, 1);
        assert_eq!(summary.max, 9);
        // Lower median of [1,3,5,9] is an observed value, not an average.
        assert_eq!(summary.median, 3);
        assert_eq!(summary.sum, 18);
        assert_eq!(summary.mean_milli(), 4_500);
    }
}
