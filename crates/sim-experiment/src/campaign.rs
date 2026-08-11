//! Declarative campaign definitions: seeds, conditions, and run policy.
//!
//! A **condition** is a named config delta with its own hash. A **campaign**
//! declares a seed set, a condition set, a run length, and an output policy
//! (`specifications/experiment-config-schema.md`).
//!
//! The file format is line-oriented and hand-parsed, matching this
//! repository's existing policy of hand-written codecs with typed
//! rejections rather than a serialization dependency. Every directive is a
//! whole line, so the format has no indentation semantics and no ambiguity
//! about which condition a `set` belongs to.
//!
//! Three validations happen at load time rather than at run time, because a
//! campaign that is wrong is much cheaper to reject before it burns hours of
//! compute:
//!
//! 1. Every condition's effective config validates.
//! 2. All conditions produce pairwise-distinct effective config hashes, so a
//!    control and a treatment can never be the same experiment under two
//!    names (A5.6).
//! 3. Every field on which two conditions actually differ is declared in a
//!    `vary` directive. This is what lets the comparison report check its
//!    aggregation precondition against a declaration the campaign author
//!    wrote down, instead of against whatever the data happens to show.

use crate::fields::{self, FieldError};
use sim_core::{ConfigError, SimConfig};
use std::collections::BTreeSet;
use std::fmt;

/// Grammar version. A campaign file records nothing about the engine build;
/// the manifest does that.
pub const CAMPAIGN_FORMAT_VERSION: u32 = 1;

/// Which documented default set a campaign starts from.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Preset {
    Phase1,
    Phase2,
}

impl Preset {
    pub fn name(self) -> &'static str {
        match self {
            Preset::Phase1 => "phase1",
            Preset::Phase2 => "phase2",
        }
    }

    fn config(self, seed: u64) -> SimConfig {
        match self {
            Preset::Phase1 => SimConfig::phase1_default(seed),
            Preset::Phase2 => SimConfig::phase2_default(seed),
        }
    }
}

/// A named config delta.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Condition {
    pub name: String,
    /// Field overrides, canonically sorted by field name.
    pub overrides: Vec<(String, String)>,
}

impl Condition {
    /// Hash over the condition's identity and its ordered delta. This is
    /// the condition's own hash, distinct from the effective config hash it
    /// produces for a given seed and base.
    pub fn delta_hash(&self) -> u64 {
        let mut hasher = sim_core::Fnv1a64::new();
        hasher.update(b"lifesim-condition-v1");
        hasher.update_u32(CAMPAIGN_FORMAT_VERSION);
        hasher.update(self.name.as_bytes());
        hasher.update_u32(self.overrides.len() as u32);
        for (field, value) in &self.overrides {
            hasher.update(field.as_bytes());
            hasher.update(b"=");
            hasher.update(value.as_bytes());
            hasher.update(b";");
        }
        hasher.finish()
    }
}

/// What a campaign writes for each run.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OutputPolicy {
    pub events: bool,
    pub snapshot: bool,
    /// zstd level for the final snapshot; `None` stores uncompressed.
    pub compression_level: Option<i32>,
    /// Ticks between spatial position samples; `0` writes no `.alss` file.
    ///
    /// Off by default. A spatial sample file is only needed by the analyses
    /// that measure spatial structure, and a campaign that does not ask for
    /// one should not pay for it.
    pub spatial_interval: u64,
    /// Ticks between morphology samples; `0` writes no `.almo` file.
    ///
    /// Off by default, on the same terms as spatial sampling. C10.3's
    /// persistence clause is the reason it exists at all: "the change
    /// persists beyond the stated window" is a statement about a series, and
    /// a campaign that records only its final tick cannot tell a durable
    /// morphological shift from one that happened to be there at the end.
    pub morphology_interval: u64,
    /// Ticks between per-individual action samples; `0` writes no `.alac`
    /// file.
    ///
    /// Off by default, on the same terms as the two above. C11.1's clause is
    /// the reason it exists: "this organism's action distribution changed
    /// after the patch relocated" is a statement about **two points in one
    /// lifetime**, and a run that records only a terminal census cannot make
    /// it at all - a terminal census has no before.
    ///
    /// Binary rather than text, unlike `morphology_interval`'s series, and
    /// the reason is arithmetic rather than taste: a morphology sample is six
    /// world-level scalars while an action sample is population x classes, so
    /// the artifact that justified a readable series does not apply. See
    /// `sim_persist::actionlog`.
    pub action_interval: u64,
}

impl Default for OutputPolicy {
    fn default() -> Self {
        Self {
            events: true,
            snapshot: true,
            compression_level: Some(3),
            spatial_interval: 0,
            morphology_interval: 0,
            action_interval: 0,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Campaign {
    pub id: String,
    pub ticks: u64,
    pub workers: usize,
    /// Ascending and unique.
    pub seeds: Vec<u64>,
    pub preset: Preset,
    /// Base overrides applied to every condition, canonically sorted.
    pub base: Vec<(String, String)>,
    pub conditions: Vec<Condition>,
    /// Fields conditions are permitted to differ in, canonically sorted.
    pub varied: Vec<String>,
    pub output: OutputPolicy,
    /// Verify kernel invariants every N ticks; 0 disables mid-run checks.
    pub check_interval: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CampaignError {
    Syntax {
        line: usize,
        message: String,
    },
    Field {
        line: usize,
        error: FieldError,
    },
    Missing(&'static str),
    UnknownCondition {
        line: usize,
        name: String,
    },
    DuplicateCondition(String),
    DuplicateOverride {
        condition: String,
        field: String,
    },
    DuplicateSeed(u64),
    EmptySeeds,
    NoConditions,
    InvalidConfig {
        condition: String,
        error: ConfigError,
    },
    /// Two conditions produce the same effective config: they are one
    /// experiment wearing two names.
    IndistinguishableConditions {
        left: String,
        right: String,
        config_hash: u64,
    },
    /// Two conditions differ in a field no `vary` directive declares.
    UndeclaredVariation {
        left: String,
        right: String,
        field: &'static str,
    },
    /// A `vary` directive names a field no condition actually varies.
    UnusedVariation(String),
    TooManyWorkers(usize),
}

impl fmt::Display for CampaignError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Syntax { line, message } => write!(formatter, "line {line}: {message}"),
            Self::Field { line, error } => write!(formatter, "line {line}: {error}"),
            Self::Missing(what) => write!(formatter, "campaign is missing a `{what}` directive"),
            Self::UnknownCondition { line, name } => write!(
                formatter,
                "line {line}: `set` refers to condition '{name}', which is not declared"
            ),
            Self::DuplicateCondition(name) => {
                write!(formatter, "condition '{name}' is declared more than once")
            }
            Self::DuplicateOverride { condition, field } => write!(
                formatter,
                "condition '{condition}' sets '{field}' more than once"
            ),
            Self::DuplicateSeed(seed) => write!(formatter, "seed {seed} is listed more than once"),
            Self::EmptySeeds => write!(formatter, "the seed set is empty"),
            Self::NoConditions => write!(formatter, "a campaign needs at least one condition"),
            Self::InvalidConfig { condition, error } => write!(
                formatter,
                "condition '{condition}' produces an invalid config: {error}"
            ),
            Self::IndistinguishableConditions {
                left,
                right,
                config_hash,
            } => write!(
                formatter,
                "conditions '{left}' and '{right}' produce the same effective config \
                 (hash 0x{config_hash:016x}); they are one experiment under two names"
            ),
            Self::UndeclaredVariation { left, right, field } => write!(
                formatter,
                "conditions '{left}' and '{right}' differ in '{field}', which no `vary` \
                 directive declares; add `vary {field}` or remove the difference"
            ),
            Self::UnusedVariation(field) => write!(
                formatter,
                "`vary {field}` is declared but no two conditions differ in it"
            ),
            Self::TooManyWorkers(count) => {
                write!(formatter, "worker count {count} is outside 1..=64")
            }
        }
    }
}

impl std::error::Error for CampaignError {}

impl Campaign {
    /// Effective config for one condition and one seed.
    pub fn config_for(&self, condition: &Condition, seed: u64) -> Result<SimConfig, CampaignError> {
        let mut config = self.preset.config(seed);
        for (field, value) in self.base.iter().chain(condition.overrides.iter()) {
            fields::set_field(&mut config, field, value)
                .map_err(|error| CampaignError::Field { line: 0, error })?;
        }
        config
            .validate()
            .map_err(|error| CampaignError::InvalidConfig {
                condition: condition.name.clone(),
                error,
            })?;
        Ok(config)
    }

    /// Total run count: one world per (condition, seed).
    pub fn run_count(&self) -> usize {
        self.conditions.len() * self.seeds.len()
    }

    /// Canonical provenance hash over everything that defines the campaign.
    pub fn stable_hash(&self) -> u64 {
        let mut hasher = sim_core::Fnv1a64::new();
        hasher.update(b"lifesim-campaign-v1");
        hasher.update_u32(CAMPAIGN_FORMAT_VERSION);
        hasher.update(self.id.as_bytes());
        hasher.update_u64(self.ticks);
        hasher.update(self.preset.name().as_bytes());
        for seed in &self.seeds {
            hasher.update_u64(*seed);
        }
        for (field, value) in &self.base {
            hasher.update(field.as_bytes());
            hasher.update(b"=");
            hasher.update(value.as_bytes());
        }
        for condition in &self.conditions {
            hasher.update_u64(condition.delta_hash());
        }
        for field in &self.varied {
            hasher.update(field.as_bytes());
        }
        hasher.update_u32(u32::from(self.output.events));
        hasher.update_u32(u32::from(self.output.snapshot));
        hasher.update_i32(self.output.compression_level.unwrap_or(-1));
        hasher.update_u64(self.check_interval);
        // Worker count is execution policy, not experiment identity: A5.2
        // requires results to be identical across worker counts, so folding
        // it into this hash would assert the opposite.
        hasher.finish()
    }

    /// Parse and fully validate a campaign definition.
    pub fn parse(text: &str) -> Result<Self, CampaignError> {
        let mut id: Option<String> = None;
        let mut ticks: Option<u64> = None;
        let mut workers = 1_usize;
        let mut check_interval = 0_u64;
        let mut seeds: Vec<u64> = Vec::new();
        let mut preset = Preset::Phase1;
        let mut base: Vec<(String, String)> = Vec::new();
        let mut conditions: Vec<Condition> = Vec::new();
        let mut varied: BTreeSet<String> = BTreeSet::new();
        let mut output = OutputPolicy::default();

        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw.split('#').next().unwrap_or("").trim();
            if trimmed.is_empty() {
                continue;
            }
            let mut words = trimmed.split_whitespace();
            let directive = words.next().unwrap_or_default();
            let rest: Vec<&str> = words.collect();
            let syntax = |message: &str| CampaignError::Syntax {
                line,
                message: message.to_owned(),
            };

            match directive {
                "campaign" => {
                    if rest.len() != 1 {
                        return Err(syntax("usage: campaign <id>"));
                    }
                    id = Some(rest[0].to_owned());
                }
                "ticks" => {
                    if rest.len() != 1 {
                        return Err(syntax("usage: ticks <count>"));
                    }
                    ticks = Some(
                        rest[0]
                            .parse()
                            .map_err(|_| syntax("ticks must be a non-negative integer"))?,
                    );
                }
                "workers" => {
                    if rest.len() != 1 {
                        return Err(syntax("usage: workers <count>"));
                    }
                    workers = rest[0]
                        .parse()
                        .map_err(|_| syntax("workers must be a positive integer"))?;
                }
                "check-interval" => {
                    if rest.len() != 1 {
                        return Err(syntax("usage: check-interval <ticks>"));
                    }
                    check_interval = rest[0]
                        .parse()
                        .map_err(|_| syntax("check-interval must be an integer"))?;
                }
                "seeds" => {
                    if rest.is_empty() {
                        return Err(syntax("usage: seeds <a>..<b> | seeds <a> <b> ..."));
                    }
                    for word in rest {
                        match word.split_once("..") {
                            Some((start, end)) => {
                                let start: u64 = parse_seed(start)
                                    .ok_or_else(|| syntax("invalid seed range start"))?;
                                let end: u64 = parse_seed(end)
                                    .ok_or_else(|| syntax("invalid seed range end"))?;
                                if end < start {
                                    return Err(syntax("seed range end precedes its start"));
                                }
                                if end - start >= 100_000 {
                                    return Err(syntax("seed range exceeds 100,000 worlds"));
                                }
                                for seed in start..=end {
                                    seeds.push(seed);
                                }
                            }
                            None => {
                                seeds.push(parse_seed(word).ok_or_else(|| syntax("invalid seed"))?)
                            }
                        }
                    }
                }
                "base" => match rest.as_slice() {
                    ["preset", name] => {
                        preset = match *name {
                            "phase1" => Preset::Phase1,
                            "phase2" => Preset::Phase2,
                            _ => return Err(syntax("preset must be phase1 or phase2")),
                        };
                    }
                    [field, value] => {
                        if !fields::FIELD_NAMES.contains(field) {
                            return Err(CampaignError::Field {
                                line,
                                error: if *field == "world_seed" {
                                    FieldError::SeedIsNotAField
                                } else {
                                    FieldError::Unknown((*field).to_owned())
                                },
                            });
                        }
                        base.push(((*field).to_owned(), (*value).to_owned()));
                    }
                    _ => return Err(syntax("usage: base preset <name> | base <field> <value>")),
                },
                "condition" => {
                    if rest.len() != 1 {
                        return Err(syntax("usage: condition <name>"));
                    }
                    let name = rest[0].to_owned();
                    if conditions.iter().any(|existing| existing.name == name) {
                        return Err(CampaignError::DuplicateCondition(name));
                    }
                    conditions.push(Condition {
                        name,
                        overrides: Vec::new(),
                    });
                }
                "set" => {
                    let [condition_name, field, value] = rest.as_slice() else {
                        return Err(syntax("usage: set <condition> <field> <value>"));
                    };
                    if !fields::FIELD_NAMES.contains(field) {
                        return Err(CampaignError::Field {
                            line,
                            error: if *field == "world_seed" {
                                FieldError::SeedIsNotAField
                            } else {
                                FieldError::Unknown((*field).to_owned())
                            },
                        });
                    }
                    let condition = conditions
                        .iter_mut()
                        .find(|candidate| candidate.name == *condition_name)
                        .ok_or_else(|| CampaignError::UnknownCondition {
                            line,
                            name: (*condition_name).to_owned(),
                        })?;
                    if condition
                        .overrides
                        .iter()
                        .any(|(existing, _)| existing == field)
                    {
                        return Err(CampaignError::DuplicateOverride {
                            condition: condition.name.clone(),
                            field: (*field).to_owned(),
                        });
                    }
                    condition
                        .overrides
                        .push(((*field).to_owned(), (*value).to_owned()));
                }
                "vary" => {
                    if rest.len() != 1 {
                        return Err(syntax("usage: vary <field>"));
                    }
                    if !fields::FIELD_NAMES.contains(&rest[0]) {
                        return Err(CampaignError::Field {
                            line,
                            error: FieldError::Unknown(rest[0].to_owned()),
                        });
                    }
                    varied.insert(rest[0].to_owned());
                }
                "output" => match rest.as_slice() {
                    ["events", value] => {
                        output.events =
                            parse_switch(value).ok_or_else(|| syntax("expected on or off"))?
                    }
                    ["snapshots", value] => {
                        output.snapshot =
                            parse_switch(value).ok_or_else(|| syntax("expected on or off"))?;
                    }
                    ["compress", value] => {
                        output.compression_level = if *value == "off" {
                            None
                        } else {
                            Some(
                                value
                                    .parse()
                                    .map_err(|_| syntax("compress takes a zstd level or off"))?,
                            )
                        };
                    }
                    ["spatial", value] => {
                        output.spatial_interval = if *value == "off" {
                            0
                        } else {
                            let interval: u64 = value
                                .parse()
                                .map_err(|_| syntax("spatial takes a tick interval or off"))?;
                            if interval == 0 {
                                return Err(syntax(
                                    "spatial interval must be positive; use 'off' to disable",
                                ));
                            }
                            interval
                        };
                    }
                    ["morphology", value] => {
                        output.morphology_interval = if *value == "off" {
                            0
                        } else {
                            let interval: u64 = value
                                .parse()
                                .map_err(|_| syntax("morphology takes a tick interval or off"))?;
                            if interval == 0 {
                                return Err(syntax(
                                    "morphology interval must be positive; use 'off' to disable",
                                ));
                            }
                            interval
                        };
                    }
                    ["actions", value] => {
                        output.action_interval = if *value == "off" {
                            0
                        } else {
                            let interval: u64 = value
                                .parse()
                                .map_err(|_| syntax("actions takes a tick interval or off"))?;
                            if interval == 0 {
                                return Err(syntax(
                                    "actions interval must be positive; use 'off' to disable",
                                ));
                            }
                            interval
                        };
                    }
                    _ => {
                        return Err(syntax(
                            "usage: output events on|off | output snapshots on|off | output compress <level>|off | output spatial <ticks>|off | output morphology <ticks>|off | output actions <ticks>|off",
                        ));
                    }
                },
                other => {
                    return Err(syntax(&format!("unknown directive '{other}'")));
                }
            }
        }

        let id = id.ok_or(CampaignError::Missing("campaign"))?;
        let ticks = ticks.ok_or(CampaignError::Missing("ticks"))?;
        if seeds.is_empty() {
            return Err(CampaignError::EmptySeeds);
        }
        if conditions.is_empty() {
            return Err(CampaignError::NoConditions);
        }
        if workers == 0 || workers > 64 {
            return Err(CampaignError::TooManyWorkers(workers));
        }
        seeds.sort_unstable();
        if let Some(pair) = seeds.windows(2).find(|pair| pair[0] == pair[1]) {
            return Err(CampaignError::DuplicateSeed(pair[0]));
        }
        base.sort_by(|left, right| left.0.cmp(&right.0));
        for condition in &mut conditions {
            condition
                .overrides
                .sort_by(|left, right| left.0.cmp(&right.0));
        }

        let campaign = Self {
            id,
            ticks,
            workers,
            seeds,
            preset,
            base,
            conditions,
            varied: varied.into_iter().collect(),
            output,
            check_interval,
        };
        campaign.validate()?;
        Ok(campaign)
    }

    /// Load-time checks that make A5.6 true by construction.
    fn validate(&self) -> Result<(), CampaignError> {
        let probe_seed = self.seeds[0];
        let mut configs = Vec::with_capacity(self.conditions.len());
        for condition in &self.conditions {
            configs.push(self.config_for(condition, probe_seed)?);
        }
        let mut observed: BTreeSet<&'static str> = BTreeSet::new();
        for left in 0..self.conditions.len() {
            for right in (left + 1)..self.conditions.len() {
                let hash_left = configs[left].stable_hash();
                if hash_left == configs[right].stable_hash() {
                    return Err(CampaignError::IndistinguishableConditions {
                        left: self.conditions[left].name.clone(),
                        right: self.conditions[right].name.clone(),
                        config_hash: hash_left,
                    });
                }
                for field in fields::differing_fields(&configs[left], &configs[right]) {
                    if !self.varied.iter().any(|declared| declared == field) {
                        return Err(CampaignError::UndeclaredVariation {
                            left: self.conditions[left].name.clone(),
                            right: self.conditions[right].name.clone(),
                            field,
                        });
                    }
                    observed.insert(field);
                }
            }
        }
        for declared in &self.varied {
            if !observed.contains(declared.as_str()) {
                return Err(CampaignError::UnusedVariation(declared.clone()));
            }
        }
        Ok(())
    }
}

fn parse_seed(value: &str) -> Option<u64> {
    match value.strip_prefix("0x") {
        Some(hex) => u64::from_str_radix(hex, 16).ok(),
        None => value.parse().ok(),
    }
}

fn parse_switch(value: &str) -> Option<bool> {
    match value {
        "on" | "true" | "yes" => Some(true),
        "off" | "false" | "no" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
# A minimal two-condition campaign.
campaign contest-pilot
ticks 500
workers 2
seeds 1
base preset phase2
base cells_x 64
base cells_y 64
base initial_organisms 40
base max_entities 400
condition control
condition treatment
set treatment crowding_cost_milli_per_s 400
vary crowding_cost_milli_per_s
output events on
output snapshots off
";

    #[test]
    fn a_valid_campaign_parses_canonically() {
        let campaign = Campaign::parse(SAMPLE).unwrap();
        assert_eq!(campaign.id, "contest-pilot");
        assert_eq!(campaign.ticks, 500);
        assert_eq!(campaign.workers, 2);
        assert_eq!(campaign.preset, Preset::Phase2);
        assert_eq!(campaign.conditions.len(), 2);
        assert_eq!(campaign.varied, vec!["crowding_cost_milli_per_s"]);
        assert!(campaign.output.events);
        assert!(!campaign.output.snapshot);
        // Base overrides are canonically sorted regardless of file order.
        let names: Vec<&str> = campaign
            .base
            .iter()
            .map(|(name, _)| name.as_str())
            .collect();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        assert_eq!(names, sorted);
        assert_eq!(campaign.run_count(), 2);
    }

    #[test]
    fn conditions_produce_distinct_effective_config_hashes() {
        let campaign = Campaign::parse(SAMPLE).unwrap();
        let control = campaign.config_for(&campaign.conditions[0], 7).unwrap();
        let treatment = campaign.config_for(&campaign.conditions[1], 7).unwrap();
        assert_ne!(control.stable_hash(), treatment.stable_hash());
        assert_ne!(
            campaign.conditions[0].delta_hash(),
            campaign.conditions[1].delta_hash()
        );
        assert_eq!(
            fields::differing_fields(&control, &treatment),
            vec!["crowding_cost_milli_per_s"]
        );
    }

    #[test]
    fn two_conditions_with_the_same_effective_config_are_rejected() {
        let text = "\
campaign twins
ticks 10
seeds 1
condition control
condition treatment
";
        assert!(matches!(
            Campaign::parse(text),
            Err(CampaignError::IndistinguishableConditions { .. })
        ));

        // Setting a field to the value it already has is the same defect
        // wearing a delta.
        let text = "\
campaign twins
ticks 10
seeds 1
condition control
condition treatment
set treatment crowding_threshold 4
vary crowding_threshold
";
        assert!(matches!(
            Campaign::parse(text),
            Err(CampaignError::IndistinguishableConditions { .. })
        ));
    }

    #[test]
    fn an_undeclared_difference_between_conditions_is_rejected() {
        let text = "\
campaign sloppy
ticks 10
seeds 1
condition control
condition treatment
set treatment crowding_threshold 9
";
        assert!(matches!(
            Campaign::parse(text),
            Err(CampaignError::UndeclaredVariation {
                field: "crowding_threshold",
                ..
            })
        ));
    }

    #[test]
    fn a_vary_directive_nothing_varies_is_rejected() {
        let text = "\
campaign stale
ticks 10
seeds 1
condition control
condition treatment
set treatment crowding_threshold 9
vary crowding_threshold
vary max_entities
";
        assert!(matches!(
            Campaign::parse(text),
            Err(CampaignError::UnusedVariation(field)) if field == "max_entities"
        ));
    }

    #[test]
    fn seeds_accept_ranges_and_lists_and_reject_duplicates() {
        let campaign =
            Campaign::parse("campaign s\nticks 1\nseeds 3..5 9 0x10\ncondition only\n").unwrap();
        assert_eq!(campaign.seeds, vec![3, 4, 5, 9, 16]);

        assert!(matches!(
            Campaign::parse("campaign s\nticks 1\nseeds 4 4\ncondition only\n"),
            Err(CampaignError::DuplicateSeed(4))
        ));
        assert!(matches!(
            Campaign::parse("campaign s\nticks 1\nseeds 9..2\ncondition only\n"),
            Err(CampaignError::Syntax { .. })
        ));
    }

    #[test]
    fn structural_mistakes_are_typed_and_actionable() {
        assert!(matches!(
            Campaign::parse("ticks 1\nseeds 1\ncondition a\n"),
            Err(CampaignError::Missing("campaign"))
        ));
        assert!(matches!(
            Campaign::parse("campaign a\nseeds 1\ncondition a\n"),
            Err(CampaignError::Missing("ticks"))
        ));
        assert!(matches!(
            Campaign::parse("campaign a\nticks 1\ncondition a\n"),
            Err(CampaignError::EmptySeeds)
        ));
        assert!(matches!(
            Campaign::parse("campaign a\nticks 1\nseeds 1\n"),
            Err(CampaignError::NoConditions)
        ));
        assert!(matches!(
            Campaign::parse("campaign a\nticks 1\nseeds 1\ncondition x\nset y cells_x 64\n"),
            Err(CampaignError::UnknownCondition { .. })
        ));
        assert!(matches!(
            Campaign::parse("campaign a\nticks 1\nseeds 1\ncondition x\ncondition x\n"),
            Err(CampaignError::DuplicateCondition(_))
        ));
        assert!(matches!(
            Campaign::parse("campaign a\nticks 1\nseeds 1\ncondition x\nfrobnicate 3\n"),
            Err(CampaignError::Syntax { .. })
        ));
        assert!(matches!(
            Campaign::parse("campaign a\nticks 1\nseeds 1\nbase world_seed 4\ncondition x\n"),
            Err(CampaignError::Field {
                error: FieldError::SeedIsNotAField,
                ..
            })
        ));
        // A config that cannot exist is rejected before any world is built.
        assert!(matches!(
            Campaign::parse("campaign a\nticks 1\nseeds 1\nbase cells_x 2\ncondition x\n"),
            Err(CampaignError::InvalidConfig { .. })
        ));
    }

    #[test]
    fn campaign_hash_ignores_worker_count_but_tracks_everything_else() {
        let campaign = Campaign::parse(SAMPLE).unwrap();
        let mut more_workers = campaign.clone();
        more_workers.workers = 8;
        assert_eq!(campaign.stable_hash(), more_workers.stable_hash());

        let mut longer = campaign.clone();
        longer.ticks += 1;
        assert_ne!(campaign.stable_hash(), longer.stable_hash());
    }
}
