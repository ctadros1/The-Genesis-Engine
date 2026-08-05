//! Multi-seed, multi-condition experiment harness (Phase 5).
//!
//! Every acceptance criterion from Phase 7 onward is a claim of the form
//! "the effect occurs in N of M seeds under condition A and fewer under
//! condition B" (`planning/phase-5-headless-scale-and-experiments.md`).
//! This crate is the instrument that makes such a claim measurable. It adds
//! no organism behavior at all.
//!
//! Boundary rules: this crate depends on `sim-core` and `sim-persist` and is
//! depended on by neither. It observes worlds and never instructs them
//! (ADR-0016), it introduces no randomness of its own, and nothing it
//! computes can reach world state.
//!
//! Determinism obligations it carries, from
//! `specifications/determinism-extensions.md` rule 10:
//!
//! - Worlds run in schedulable units sharing no mutable state.
//! - Results are indexed by unit, never appended on completion, so the
//!   output is identical at every worker count.
//! - Worker count is execution policy and is excluded from the campaign
//!   hash, because A5.2 asserts results do not depend on it.

pub mod campaign;
pub mod fields;
pub mod manifest;
pub mod report;
pub mod scheduler;

pub use campaign::{
    CAMPAIGN_FORMAT_VERSION, Campaign, CampaignError, Condition, OutputPolicy, Preset,
};
pub use fields::{FIELD_NAMES, FieldError, FieldValue, differing_fields, read_field, set_field};
pub use manifest::{
    FailedRun, MANIFEST_FORMAT, MANIFEST_VERSION, Manifest, ManifestError, RunResult,
};
pub use report::{
    ComparisonReport, ConditionSummary, MetricFn, PairedComparison, REPORT_POLICY_VERSION,
    ReportRefusal, Summary, compare,
};
pub use scheduler::{
    PreflightFailure, RunUnit, SchedulerOptions, enumerate_units, open_store, preflight,
    run_campaign, run_stem,
};
