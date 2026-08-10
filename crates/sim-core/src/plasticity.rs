//! Plasticity arithmetic: what one plastic edge does in one tick (Phase 11,
//! `lifesim-plasticity-v1`).
//!
//! Deliberately standalone. Every function here is pure, takes scalars,
//! returns scalars, allocates nothing, and touches no `World`, no genome,
//! and no config. The tick integration - which edges are plastic, which
//! node's activation is the modulator, what order edges are visited in, what
//! the energy cost is, and what section tag the state hashes under - lives
//! elsewhere. Keeping the arithmetic separable is what makes the numeric
//! half of C11.5 testable at all: a bounds property over an adversarial
//! grid of coefficients is a unit test here and would be an unrunnable
//! campaign there.
//!
//! # There is no reward in this file and there must never be one
//!
//! Nothing here computes how well an organism is doing. `modulator` is an
//! ordinary activation of the organism's own evolved network, handed in by
//! the caller; this module cannot distinguish a modulator that fires on food
//! from one that fires on a wall, and it must stay unable to. If a future
//! change makes anything in this module depend on energy, age, offspring
//! count, or any other measure of success, that is the prohibited thing
//! (`docs/02-scope-and-non-goals.md`, ADR-0014), not a refinement of it.
//!
//! Authored here: that an edge *can* be plastic, the bounded registry of
//! rule forms, that a Modulatory node gates plasticity, the arithmetic, and
//! the clamps. Evolved everywhere else: which edges are plastic, which rule
//! each uses, its coefficients, and which nodes are modulatory - and
//! therefore what the organism treats as reinforcing.
//!
//! # Why the float/integer split is where it is
//!
//! Determinism Rule 7: anything that accumulates over a lifetime is fixed
//! point. Learned state accumulates over 10^5 ticks or more, which is
//! exactly the horizon over which f32 reassociation and contraction
//! differences stop being invisible - the fragility ADR-0011 exists to keep
//! out of this project. So the spec's five steps split:
//!
//! - steps 1 to 3 (rule form, eta and modulator scaling, conversion to Q16)
//!   are f32, evaluated once per tick, and same-build deterministic under
//!   ADR-0011 because they never feed themselves;
//! - steps 4 and 5 (decay, accumulate, clamp) are integer, because they do.
//!
//! Two integer hazards, both of which bite silently in `--release` where
//! overflow wraps instead of panicking, and both of which are guarded below:
//!
//! - `learned_q16 * decay_q16` overflows `i32` for any learned value past
//!   about 32k with a meaningful decay, so the product is taken in `i64` and
//!   narrowed only after the division has made it small again;
//! - the spec writes step 4's scaling as `>> 16` but calls it "truncating
//!   toward zero", and those two disagree for negative values: an arithmetic
//!   shift floors, so `-3` decays to `-1` under the shift and to `-2` under
//!   truncation. The prose wins, because the shift is asymmetric in exactly
//!   the way that biases the sign of everything an organism learns.

use crate::checksum::Fnv1a64;

/// Policy version for the rule registry and the update arithmetic. Enters
/// the canonical config hash only when the plasticity section is enabled, so
/// a world without plasticity is unaffected (Rule 0, Rule 9).
pub const PLASTICITY_POLICY_VERSION: &str = "lifesim-plasticity-v1";

/// Registry version, folded into the config hash alongside the policy
/// string. Adding rule 5 later increments this rather than redefining it.
pub const RULE_REGISTRY_VERSION: u16 = 1;

/// Rule ids `0..RULE_COUNT` are in the registry.
///
/// **Rule 5 "Observational" is deliberately absent.** Its form is rule 1
/// with `x` replaced by the perceived-action input of a selected neighbour,
/// which requires the Phase 13 social channel to exist. Admitting it now
/// would mean either an input that is always zero - a rule that silently
/// equals rule 1 with `b` disabled, which evolution would learn to use for
/// something else entirely before Phase 13 could reinterpret it - or an
/// authored stand-in for perception. Both are worse than a smaller
/// registry. Phase 13 raises `RULE_COUNT` and `RULE_REGISTRY_VERSION`
/// together.
pub const RULE_COUNT: u8 = 5;

/// Not plastic even when the edge carries `EDGE_FLAG_PLASTIC`.
pub const RULE_STATIC: u8 = 0;
/// `a*x*y + b*x + c*y + d`.
pub const RULE_HEBBIAN: u8 = 1;
/// `y*(x - y*w_eff)`; self-normalizing, ignores the coefficients.
pub const RULE_OJA: u8 = 2;
/// Rule 1 multiplied by the modulator activation.
pub const RULE_MODULATED_HEBBIAN: u8 = 3;
/// Rule 1 accumulated into a per-edge trace with decay, discharged through
/// the modulator.
pub const RULE_ELIGIBILITY_TRACE: u8 = 4;

/// 1.0 in Q16.
pub const ONE_Q16: i32 = 1 << 16;

/// The learned-delta clamp, `8.0` in Q16.
///
/// It equals `genome2::VALUE_LIMIT` and `controller2::ACTIVATION_LIMIT`, and
/// that is not a coincidence to be tidied away into one shared constant.
/// All three are the same bound arrived at three times: a genome weight may
/// not exceed 8, a pre-activation sum is clamped to 8, and the learned delta
/// may not push the effective weight outside the range the other two already
/// assume. Sharing one constant would make it look like one decision, and a
/// later phase that widened the activation clamp would silently widen what a
/// lifetime can learn. They are pinned equal by a test instead.
pub const LEARN_LIMIT_Q16: i32 = 8 << 16;

/// Q16 scale as f32. Exact: 65536 is a power of two.
const Q16_SCALE: f32 = 65_536.0;

/// The effective-weight bound in f32, the same 8.0 `LEARN_LIMIT_Q16` is.
const WEIGHT_LIMIT: f32 = 8.0;

/// Whether a rule id is in the registry this build implements.
pub fn rule_in_registry(rule_id: u8) -> bool {
    rule_id < RULE_COUNT
}

/// Whether a rule's update is gated by a modulator activation.
///
/// Rules 1 and 2 are ungated on purpose. They can produce unsupervised
/// change with no modulator at all, which gives the search a gradient toward
/// useful plasticity *before* it has to discover modulation - the partial
/// mitigation the phase plan records against the risk that the modulatory
/// design is too indirect for evolution to find.
pub fn rule_is_modulated(rule_id: u8) -> bool {
    rule_id == RULE_MODULATED_HEBBIAN || rule_id == RULE_ELIGIBILITY_TRACE
}

/// One plastic edge's rule, as the update needs it.
///
/// `decay_q16` is the genome's `decay` gene already converted by
/// [`decay_to_q16`]; the conversion is hoisted out of the per-tick path
/// because it is per-edge constant for a lifetime and doing it per tick
/// would be a float operation repeated 10^5 times for no reason.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlasticityRule {
    pub rule_id: u8,
    /// Learning rate. Bounded to [0, 1] by `PlasticityGenes::valid`; not
    /// re-clamped here, because step 5's clamp bounds the result whatever
    /// eta is and a second clamp would only hide a validation gap.
    pub eta: f32,
    /// `[a, b, c, d]`, each bounded to [-1, 1] by the genome validator.
    pub coefficients: [f32; 4],
    /// Q16 fraction of the learned delta pulled back toward zero per tick.
    pub decay_q16: i32,
}

/// The scalars one update reads. All are values already committed for this
/// tick by the controller phase; nothing here reads another organism's
/// current-tick state (Rule 4).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EdgeSignals {
    /// Presynaptic activation `x`.
    pub pre: f32,
    /// Postsynaptic activation `y`.
    pub post: f32,
    /// The gating activation, clamped to [-1, 1] inside the update.
    ///
    /// **For a modulated rule whose `modulator_node` is 0 or unresolvable,
    /// the caller passes 0.0, which makes the update inert.** The
    /// alternative reading - an absent modulator means "always on", so rule
    /// 3 degenerates to rule 1 - would make two registry entries the same
    /// rule and would hand every modulated edge a free ungated update it did
    /// not evolve. Ignored entirely by the ungated rules.
    pub modulator: f32,
    /// The edge's current effective weight, i.e. [`effective_weight`] of the
    /// genome weight and the learned delta. Read by rule 2 only.
    pub w_eff: f32,
}

/// Per-edge learned state. Zero at birth, always, and that is an invariant
/// rather than a default: learned state that were inherited would make a
/// discovery a heritable trait and Phase 13's transmission question
/// unaskable.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LearnedState {
    /// Q16, clamped to +/- [`LEARN_LIMIT_Q16`].
    pub learned_q16: i32,
    /// Q16 eligibility trace, rule 4 only, clamped the same way. Zero and
    /// untouched under every other rule.
    pub trace_q16: i32,
}

/// What one call to [`step`] did with the edge.
///
/// Exactly one kind per call, which is what lets the counters assert that
/// every edge visited was accounted for.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StepKind {
    /// The rule form ran and the learned state was rewritten (possibly to
    /// the same value).
    Applied,
    /// Rule 0. Nothing was read and nothing was written.
    Static,
    /// A rule id outside the registry. Unreachable through a validated
    /// genome, neutralized rather than panicked, and counted so that a
    /// validation gap shows up as a number instead of as silence.
    Refused,
}

/// The result of one update. `state` is the new state; the flags are for
/// counting and eventing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StepOutcome {
    pub state: LearnedState,
    pub kind: StepKind,
    /// A non-finite intermediate was neutralized to zero. Follows the
    /// controller-fault policy exactly: neutralize, count, event, never
    /// panic, and never let the value reach the checksum (Rule 8).
    pub fault: bool,
    /// The learned accumulate saturated at the clamp.
    pub clamped: bool,
    /// The eligibility trace accumulate saturated at the clamp.
    pub trace_clamped: bool,
}

impl StepOutcome {
    fn unchanged(state: LearnedState, kind: StepKind, fault: bool) -> Self {
        Self {
            state,
            kind,
            fault,
            clamped: false,
            trace_clamped: false,
        }
    }
}

/// Convert a `[0, 1]` decay gene to the Q16 fraction step 4 uses.
///
/// Out-of-range and non-finite inputs collapse to a no-decay 0 rather than
/// being trusted; the genome validator already bounds the gene, so a value
/// arriving here out of range is a bug, and the safe direction for a bug is
/// the one that changes nothing.
pub fn decay_to_q16(decay: f32) -> i32 {
    if !decay.is_finite() {
        return 0;
    }
    to_q16(decay).clamp(0, ONE_Q16)
}

/// f32 to Q16 with round-half-away-from-zero, the spec's step 3 exactly:
/// `trunc(value * 65536 + copysign(0.5, value))`.
///
/// Round-half-away-from-zero, not round-half-to-even. Half-to-even is the
/// natural mistake because it is what `round` does in several languages and
/// what "unbiased rounding" usually means, but it is unbiased over
/// *magnitudes*, not over *signs*, and a rule that rounds +0.5 to 0 and
/// -0.5 to 0 while rounding +1.5 and -1.5 to 2 is a tiny per-tick asymmetry
/// applied 10^5 times to a value that feeds back into its own future
/// updates. Away-from-zero is exactly sign-symmetric: `to_q16(-v) ==
/// -to_q16(v)` for every `v`.
///
/// The caller must have established that `value` is finite; use
/// [`to_q16_checked`] otherwise. A finite value large enough to saturate the
/// `as i32` cast (which saturates rather than wrapping, and is not UB) is
/// already far outside the clamp step 5 applies, so saturation here can only
/// ever be followed by clamping there.
///
/// The addition itself rounds, so this is deterministic rather than exactly
/// the mathematical function: a value a few ulps below a tie can round up
/// into it. Determinism on the recorded build is what ADR-0011 claims and
/// all this needs to be.
pub fn to_q16(value: f32) -> i32 {
    (value * Q16_SCALE + 0.5_f32.copysign(value)).trunc() as i32
}

/// [`to_q16`] with the non-finite case neutralized. Returns `(q16, fault)`.
pub fn to_q16_checked(value: f32) -> (i32, bool) {
    if value.is_finite() {
        (to_q16(value), false)
    } else {
        (0, true)
    }
}

/// Q16 to f32. Exact for every value inside the clamp: `LEARN_LIMIT_Q16` is
/// 2^19 and f32 represents every integer below 2^24 exactly, so no learned
/// value ever loses a bit on the way out.
pub fn q16_to_f32(value_q16: i32) -> f32 {
    value_q16 as f32 / Q16_SCALE
}

/// Step 4: pull a Q16 accumulator toward zero by a Q16 fraction, truncating
/// toward zero.
///
/// Two things are load-bearing and neither is visible from the spec's
/// one-line formula:
///
/// - the product is taken in `i64`. `learned_q16 * decay_q16` at the clamp
///   is 2^19 * 2^16 = 2^35, which overflows `i32` - and overflows it
///   *silently* in `--release`, where the wrap would turn a full decay into
///   no decay at all;
/// - the scaling is a division, not `>> 16`. They agree for non-negative
///   values and disagree for negative ones, where the shift floors: -3 with
///   a half decay loses 1 unit under truncation and 2 under the shift, so
///   the shift decays negative learned weights faster than positive ones.
///   That asymmetry would bias the sign of everything an organism learns.
///
/// `|pull| <= |value|` for any `decay_q16` in `[0, ONE_Q16]`, so the
/// subtraction cannot leave `i32` and the narrowing cast is safe.
pub fn decay_toward_zero(value_q16: i32, decay_q16: i32) -> i32 {
    debug_assert!((0..=ONE_Q16).contains(&decay_q16));
    let decay = i64::from(decay_q16.clamp(0, ONE_Q16));
    let pull = i64::from(value_q16) * decay / i64::from(ONE_Q16);
    (i64::from(value_q16) - pull) as i32
}

/// Step 5: accumulate and clamp. Returns `(value, saturated)`.
///
/// The sum is taken in `i64` because `delta_q16` can be any `i32` - a
/// saturating cast in [`to_q16`] can hand this `i32::MAX` - and `i32 + i32`
/// wraps in `--release`. Wrapping here would turn a runaway positive update
/// into a large negative learned weight, which is the single worst way this
/// arithmetic could fail: silent, bounded-looking, and sign-inverted.
pub fn accumulate_clamped(value_q16: i32, delta_q16: i32) -> (i32, bool) {
    let sum = i64::from(value_q16) + i64::from(delta_q16);
    let limit = i64::from(LEARN_LIMIT_Q16);
    let clamped = sum.clamp(-limit, limit);
    (clamped as i32, clamped != sum)
}

/// The weight evaluation uses: genome weight plus learned delta, clamped.
///
/// The clamp is what keeps `w_eff` inside the range every downstream bound
/// already assumes. A non-finite `genome_weight` cannot arrive - the genome
/// decoder rejects one - and is deliberately not defended against here,
/// because a silent repair would hide the decoder gap that produced it.
pub fn effective_weight(genome_weight: f32, learned_q16: i32) -> f32 {
    (genome_weight + q16_to_f32(learned_q16)).clamp(-WEIGHT_LIMIT, WEIGHT_LIMIT)
}

/// The generalized Hebbian form, `a*x*y + b*x + c*y + d`.
///
/// Written as one expression in this order and left to right. Rust does not
/// reassociate float arithmetic, so the order below *is* the policy; a
/// refactor that groups the terms differently is a lineage break, not a
/// tidy-up.
fn hebbian(coefficients: [f32; 4], pre: f32, post: f32) -> f32 {
    coefficients[0] * pre * post + coefficients[1] * pre + coefficients[2] * post + coefficients[3]
}

/// One plastic edge, one tick, in the spec's step order.
///
/// Allocation-free and branch-light: the `learn` phase runs once per plastic
/// edge per tick and must stay at zero allocations per organism per tick,
/// exactly as controller evaluation does.
pub fn step(rule: PlasticityRule, signals: EdgeSignals, state: LearnedState) -> StepOutcome {
    let PlasticityRule {
        rule_id,
        eta,
        coefficients,
        decay_q16,
    } = rule;
    let EdgeSignals {
        pre,
        post,
        modulator,
        w_eff,
    } = signals;

    // Rule 0 is "not plastic even if flagged", so it is a complete no-op:
    // no decay, no trace, no write. Decaying a rule-0 edge would be
    // unobservable anyway - the rule is fixed for a lifetime and learned
    // state is zero at birth - but making it a no-op means the edge's
    // stored state is provably still the birth value, which is what C11.4
    // asserts directly.
    if rule_id == RULE_STATIC {
        return StepOutcome::unchanged(state, StepKind::Static, false);
    }
    if !rule_in_registry(rule_id) {
        return StepOutcome::unchanged(state, StepKind::Refused, true);
    }

    let mut fault = false;
    let mut trace_clamped = false;
    let mut trace_q16 = state.trace_q16;

    // Steps 1 and 2, f32. `raw` is the rule form before any scaling.
    let raw = match rule_id {
        RULE_OJA => post * (pre - post * w_eff),
        // Rules 1, 3, and 4 all start from the same generalized Hebbian
        // form and differ only in what happens to it afterwards.
        _ => hebbian(coefficients, pre, post),
    };

    let delta = if rule_id == RULE_ELIGIBILITY_TRACE {
        // The trace accumulates across ticks, so it is fixed point for the
        // same Rule 7 reason `learned_q16` is: it is an integrator with a
        // lifetime-long horizon.
        //
        // eta is applied once, on the way *into* the trace, so an edge's
        // learning rate scales what it remembers rather than being charged
        // again every time the modulator fires.
        let (eligibility_q16, eligibility_fault) = to_q16_checked(raw * eta);
        fault |= eligibility_fault;
        // The same `decay` gene serves both integrators: it is the trace's
        // decay here and the learned delta's decay at step 4. The spec names
        // one decay gene and describes both integrators as decaying, and
        // inventing a second gene to separate them would be a schema change
        // this phase does not have.
        let decayed_trace = decay_toward_zero(trace_q16, decay_q16);
        let (accumulated, saturated) = accumulate_clamped(decayed_trace, eligibility_q16);
        trace_q16 = accumulated;
        trace_clamped = saturated;
        // "Applied when the modulator fires" is read as the continuous
        // activation rather than a threshold. A threshold would be an
        // authored constant deciding what counts as firing, which is the
        // one thing this design keeps out of our hands; multiplying by the
        // activation is the same gating rule 3 uses, so the two modulated
        // rules differ in memory and not in what modulation means.
        //
        // The trace-to-f32 conversion is exact inside the clamp, so this
        // reintroduces no error that could accumulate: the product feeds
        // step 5's integer accumulate and is never fed back into the trace.
        q16_to_f32(trace_q16) * modulator.clamp(-1.0, 1.0)
    } else if rule_is_modulated(rule_id) {
        raw * eta * modulator.clamp(-1.0, 1.0)
    } else {
        raw * eta
    };

    // Step 3. A non-finite delta is neutralized to zero and reported;
    // steps 4 and 5 still run, so a faulting edge keeps decaying rather
    // than freezing at whatever it had learned before the fault.
    let (delta_q16, delta_fault) = to_q16_checked(delta);
    fault |= delta_fault;

    // Step 4, then step 5. Both integer.
    let decayed = decay_toward_zero(state.learned_q16, decay_q16);
    let (learned_q16, clamped) = accumulate_clamped(decayed, delta_q16);

    StepOutcome {
        state: LearnedState {
            learned_q16,
            trace_q16,
        },
        kind: StepKind::Applied,
        fault,
        clamped,
        trace_clamped,
    }
}

/// Plasticity counters. Every call to [`step`] lands in exactly one of the
/// first three; the last three are orthogonal flags that can co-occur with
/// an applied update.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PlasticityCounters {
    pub updates_applied: u64,
    /// Edges flagged plastic whose rule is Static. Not noise: a rule-0
    /// plastic edge still pays the per-edge energy cost, so this is how a
    /// campaign tells "plasticity was selected down by turning the rule off"
    /// from "plasticity was selected down by dropping the flag".
    pub updates_static: u64,
    /// Rule ids outside the registry. Expected to stay at zero forever; a
    /// nonzero value is a genome-validation bug report.
    pub updates_refused: u64,
    pub faults: u64,
    pub clamped: u64,
    pub trace_clamped: u64,
}

impl PlasticityCounters {
    /// Every counter, split into the disposition half and the anomaly half.
    ///
    /// **Destructured with no `..`, never field-accessed** (D-077), for the
    /// same reason `MutationCounters::partitioned` and
    /// `DevelopCounters::partitioned` are. A counter added to the struct
    /// fails to compile here until it is put in one bucket or the other,
    /// which is what stops it from being reported while sitting outside the
    /// checksum - the defect that made two restored checksums differ in
    /// Phase 9.
    ///
    /// The concatenation order is the declaration order and is
    /// **permanent**: it is the byte order `hash_into` feeds the hasher, so
    /// a plasticity world's checksum is defined by it. Append, never
    /// reorder.
    fn partitioned(&self) -> ([u64; 3], [u64; 3]) {
        let Self {
            updates_applied,
            updates_static,
            updates_refused,
            faults,
            clamped,
            trace_clamped,
        } = *self;
        (
            [updates_applied, updates_static, updates_refused],
            [faults, clamped, trace_clamped],
        )
    }

    /// Plastic edges visited. Equals the number of [`step`] calls exactly,
    /// which is the assertion that catches a `learn` phase that quietly
    /// skipped edges.
    pub fn total_evaluated(&self) -> u64 {
        self.partitioned().0.iter().sum()
    }

    /// Faults plus saturations. Runaway plasticity destabilizing controllers
    /// into noise is a named risk of this phase; this is its measurement.
    pub fn total_anomalies(&self) -> u64 {
        self.partitioned().1.iter().sum()
    }

    /// Fold one update's outcome in.
    ///
    /// Destructured with no `..` and matched with no `_` arm, so a new
    /// outcome flag or a new [`StepKind`] cannot be added without deciding
    /// where it is counted.
    pub fn record(&mut self, outcome: &StepOutcome) {
        let StepOutcome {
            state: _,
            kind,
            fault,
            clamped,
            trace_clamped,
        } = *outcome;
        match kind {
            StepKind::Applied => self.updates_applied += 1,
            StepKind::Static => self.updates_static += 1,
            StepKind::Refused => self.updates_refused += 1,
        }
        if fault {
            self.faults += 1;
        }
        if clamped {
            self.clamped += 1;
        }
        if trace_clamped {
            self.trace_clamped += 1;
        }
    }

    /// Hash every counter under the counters sub-tag.
    ///
    /// This is the *counters* tag only, the analogue of
    /// `lifesim-structmut-counters-v1`. The checksum **section** tag
    /// `lifesim-learn-state-v1`, and the decision of when the section is
    /// present at all, belong to the tick integration - appending a section
    /// to `World::state_checksum` from here would put the Phase 9 fixture at
    /// risk from a module that cannot see it.
    pub fn hash_into(&self, hasher: &mut Fnv1a64) {
        let (disposition, anomalies) = self.partitioned();
        hasher.update(b"lifesim-plasticity-counters-v1");
        for value in disposition.into_iter().chain(anomalies) {
            hasher.update_u64(value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// eta 1, no decay, ungated: the shortest path from a rule form to a
    /// learned value, so an expected number in a test is the rule form
    /// itself in Q16.
    fn rule(rule_id: u8, coefficients: [f32; 4]) -> PlasticityRule {
        PlasticityRule {
            rule_id,
            eta: 1.0,
            coefficients,
            decay_q16: 0,
        }
    }

    fn signals(pre: f32, post: f32) -> EdgeSignals {
        EdgeSignals {
            pre,
            post,
            modulator: 0.0,
            w_eff: 0.0,
        }
    }

    fn learned(outcome: StepOutcome) -> i32 {
        outcome.state.learned_q16
    }

    /// `(coefficients, x, y, expected delta in whole units)` for the
    /// generalized Hebbian form, hand-computed from `a*x*y + b*x + c*y + d`
    /// and never from the implementation. Every activation is at a boundary
    /// and every coefficient is at a [-1, 1] limit or zero. Shared with the
    /// modulated rules, which are defined in terms of this form.
    const HEBBIAN_CASES: [([f32; 4], f32, f32, i32); 13] = [
        ([1.0, 0.0, 0.0, 0.0], 1.0, 1.0, 1),
        ([1.0, 0.0, 0.0, 0.0], -1.0, 1.0, -1),
        ([1.0, 0.0, 0.0, 0.0], -1.0, -1.0, 1),
        ([1.0, 0.0, 0.0, 0.0], 0.0, 1.0, 0),
        ([0.0, 1.0, 0.0, 0.0], -1.0, 1.0, -1),
        ([0.0, 0.0, 1.0, 0.0], -1.0, 1.0, 1),
        ([0.0, 0.0, 0.0, 1.0], 0.0, 0.0, 1),
        ([0.0, 0.0, 0.0, -1.0], 0.0, 0.0, -1),
        ([1.0, 1.0, 1.0, 1.0], 1.0, 1.0, 4),
        ([1.0, 1.0, 1.0, 1.0], -1.0, -1.0, 0),
        ([-1.0, -1.0, -1.0, -1.0], 1.0, 1.0, -4),
        ([1.0, -1.0, 1.0, -1.0], -1.0, 1.0, 0),
        ([-1.0, 1.0, -1.0, 1.0], 1.0, -1.0, 4),
    ];

    #[test]
    fn registry_is_bounded_and_observational_is_absent() {
        assert_eq!(RULE_COUNT, 5);
        assert_eq!(RULE_REGISTRY_VERSION, 1);
        assert_eq!(PLASTICITY_POLICY_VERSION, "lifesim-plasticity-v1");
        for rule_id in 0..RULE_COUNT {
            assert!(rule_in_registry(rule_id));
        }
        // Rule 5 is Observational and needs the Phase 13 social channel.
        // Admitting it early is the failure this asserts against.
        assert!(!rule_in_registry(5));
        assert!(!rule_in_registry(255));

        assert!(!rule_is_modulated(RULE_STATIC));
        assert!(!rule_is_modulated(RULE_HEBBIAN));
        assert!(!rule_is_modulated(RULE_OJA));
        assert!(rule_is_modulated(RULE_MODULATED_HEBBIAN));
        assert!(rule_is_modulated(RULE_ELIGIBILITY_TRACE));

        // An out-of-registry rule id is refused, not silently treated as
        // static and not panicked on.
        let outcome = step(
            rule(5, [1.0; 4]),
            signals(1.0, 1.0),
            LearnedState::default(),
        );
        assert_eq!(outcome.kind, StepKind::Refused);
        assert!(outcome.fault);
        assert_eq!(learned(outcome), 0);
    }

    #[test]
    fn learn_limit_matches_the_weight_and_activation_bound() {
        assert_eq!(LEARN_LIMIT_Q16, 8 << 16);
        assert_eq!(LEARN_LIMIT_Q16, 524_288);
        assert_eq!(ONE_Q16, 65_536);
        // Pinned equal to the genome's weight bound. `controller2`'s
        // ACTIVATION_LIMIT is private to that module so it cannot be
        // asserted from here; it is the same 8.0 and is stated in this
        // constant's documentation.
        assert_eq!(q16_to_f32(LEARN_LIMIT_Q16), crate::genome2::VALUE_LIMIT);
        assert_eq!(WEIGHT_LIMIT, crate::genome2::VALUE_LIMIT);
    }

    #[test]
    fn generalized_hebbian_form_at_boundary_activations_and_coefficient_limits() {
        let mut nonzero = 0;
        for (coefficients, pre, post, expected) in HEBBIAN_CASES {
            let outcome = step(
                rule(RULE_HEBBIAN, coefficients),
                signals(pre, post),
                LearnedState::default(),
            );
            assert_eq!(
                learned(outcome),
                expected * ONE_Q16,
                "coefficients {coefficients:?} x {pre} y {post}"
            );
            assert_eq!(outcome.kind, StepKind::Applied);
            assert!(!outcome.fault);
            // Rule 1 has no trace.
            assert_eq!(outcome.state.trace_q16, 0);
            if expected != 0 {
                nonzero += 1;
            }
        }
        // Ten of the thirteen cases move the weight; the other three are
        // coefficient combinations that cancel exactly. Without this count
        // the whole table could pass on an implementation that returns zero.
        assert_eq!(nonzero, 10);
    }

    #[test]
    fn oja_normalizes_toward_the_effective_weight_and_ignores_the_coefficients() {
        // y*(x - y*w_eff), hand-computed.
        let cases: [(f32, f32, f32, i32); 6] = [
            // (x, y, w_eff, expected)
            (1.0, 1.0, 0.0, 1),
            // The fixed point: w_eff == x/y, so the rule stops moving.
            (1.0, 1.0, 1.0, 0),
            (0.0, 1.0, 2.0, -2),
            (1.0, -1.0, 1.0, -2),
            (1.0, 0.0, 1.0, 0),
            (1.0, 2.0, 1.0, -2),
        ];
        for (pre, post, w_eff, expected) in cases {
            let mut input = signals(pre, post);
            input.w_eff = w_eff;
            let outcome = step(rule(RULE_OJA, [0.0; 4]), input, LearnedState::default());
            assert_eq!(
                learned(outcome),
                expected * ONE_Q16,
                "x {pre} y {post} w_eff {w_eff}"
            );
        }

        // Oja takes no coefficients. An implementation that routed rule 2
        // through the Hebbian form would pass every case above, because
        // they all use zero coefficients, and must fail here.
        //
        // **The coefficient sets have to be checked for cancellation or
        // this assertion is vacuous**, which is how it was first written:
        // [1, -1, 1, -1] evaluates to exactly 0 at x = y = 1, so a rule 2
        // that added the Hebbian form would have added nothing and the test
        // would have passed on the broken build. The guard below is the fix,
        // and it fails safe - a `hebbian` that returned zero would trip it.
        let mut input = signals(1.0, 1.0);
        input.w_eff = 0.5;
        let plain = step(rule(RULE_OJA, [0.0; 4]), input, LearnedState::default());
        assert_ne!(learned(plain), 0);
        for coefficients in [
            [1.0, 1.0, 1.0, 1.0],
            [-1.0, -1.0, -1.0, -1.0],
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, -1.0],
        ] {
            assert_ne!(
                hebbian(coefficients, 1.0, 1.0),
                0.0,
                "{coefficients:?} cancels at x = y = 1 and would prove nothing"
            );
            let loaded = step(rule(RULE_OJA, coefficients), input, LearnedState::default());
            assert_eq!(
                learned(plain),
                learned(loaded),
                "rule 2 read the coefficients {coefficients:?}"
            );
        }
    }

    #[test]
    fn modulated_hebbian_scales_by_the_clamped_modulator() {
        let base = [1.0, 0.0, 0.0, 0.0];
        // (modulator, expected delta in Q16). x = y = 1 so the Hebbian form
        // is exactly 1.0 and the modulator is the whole answer.
        let cases: [(f32, i32); 7] = [
            (1.0, ONE_Q16),
            (0.0, 0),
            (-1.0, -ONE_Q16),
            (0.5, ONE_Q16 / 2),
            (-0.25, -ONE_Q16 / 4),
            // Clamped to [-1, 1]: an activation outside the range cannot buy
            // a bigger update.
            (5.0, ONE_Q16),
            (-5.0, -ONE_Q16),
        ];
        for (modulator, expected) in cases {
            let mut input = signals(1.0, 1.0);
            input.modulator = modulator;
            let outcome = step(
                rule(RULE_MODULATED_HEBBIAN, base),
                input,
                LearnedState::default(),
            );
            assert_eq!(learned(outcome), expected, "modulator {modulator}");
        }
        // The 0.5 case above is what separates "scales by the activation"
        // from "is a boolean gate"; assert the relationship explicitly.
        let mut half = signals(1.0, 1.0);
        half.modulator = 0.5;
        let mut full = signals(1.0, 1.0);
        full.modulator = 1.0;
        let scaled = step(
            rule(RULE_MODULATED_HEBBIAN, base),
            half,
            LearnedState::default(),
        );
        let whole = step(
            rule(RULE_MODULATED_HEBBIAN, base),
            full,
            LearnedState::default(),
        );
        assert_eq!(learned(scaled) * 2, learned(whole));
    }

    #[test]
    fn modulated_rules_reduce_to_the_hebbian_form_at_full_modulation() {
        // Rules 3 and 4 are defined in terms of rule 1, so the whole
        // boundary table applies to them too and the relationship is
        // checkable rather than merely stated. Rule 4 reduces because a
        // one-tick trace starting from zero with no decay *is* the
        // eligibility, and full modulation discharges all of it.
        for (coefficients, pre, post, expected) in HEBBIAN_CASES {
            for rule_id in [RULE_MODULATED_HEBBIAN, RULE_ELIGIBILITY_TRACE] {
                for (modulator, sign) in [(1.0_f32, 1), (-1.0_f32, -1)] {
                    let input = EdgeSignals {
                        pre,
                        post,
                        modulator,
                        w_eff: 0.0,
                    };
                    let outcome = step(rule(rule_id, coefficients), input, LearnedState::default());
                    assert_eq!(
                        learned(outcome),
                        sign * expected * ONE_Q16,
                        "rule {rule_id} coefficients {coefficients:?} x {pre} y {post} \
                         modulator {modulator}"
                    );
                }
                // A silent modulator writes nothing, whatever the form says.
                let outcome = step(
                    rule(rule_id, coefficients),
                    signals(pre, post),
                    LearnedState::default(),
                );
                assert_eq!(learned(outcome), 0);
            }
        }
    }

    #[test]
    fn ungated_rules_ignore_the_modulator() {
        // Rules 1 and 2 must produce change with no modulator at all. An
        // implementation that gated every rule would make the whole
        // registry dependent on evolution finding modulation first.
        for rule_id in [RULE_HEBBIAN, RULE_OJA] {
            let mut previous = None;
            for modulator in [-1.0_f32, 0.0, 0.5, 1.0] {
                let mut input = signals(1.0, 1.0);
                input.modulator = modulator;
                input.w_eff = 2.0;
                let outcome = step(
                    rule(rule_id, [1.0, 0.0, 0.0, 0.0]),
                    input,
                    LearnedState::default(),
                );
                assert_ne!(learned(outcome), 0, "rule {rule_id} must be ungated");
                if let Some(before) = previous {
                    assert_eq!(
                        before,
                        learned(outcome),
                        "rule {rule_id} read the modulator"
                    );
                }
                previous = Some(learned(outcome));
            }
        }
    }

    #[test]
    fn eligibility_trace_accumulates_while_the_modulator_is_silent_and_discharges_when_it_fires() {
        let mut params = rule(RULE_ELIGIBILITY_TRACE, [1.0, 0.0, 0.0, 0.0]);
        params.eta = 0.5;
        let mut state = LearnedState::default();

        // Two ticks with the modulator silent. The trace must grow while the
        // learned weight stays exactly zero: that is the whole point of a
        // trace, and a test that only checked the learned weight would pass
        // on an implementation that never accumulated anything.
        for expected_trace in [ONE_Q16 / 2, ONE_Q16] {
            let outcome = step(params, signals(1.0, 1.0), state);
            state = outcome.state;
            assert_eq!(state.trace_q16, expected_trace);
            assert_eq!(state.learned_q16, 0);
            assert!(!outcome.trace_clamped);
        }

        // The modulator fires. eta was charged on the way into the trace, so
        // the discharge is the whole trace (1.5 after this tick's own
        // eligibility joins it), not the trace times eta again.
        let mut firing = signals(1.0, 1.0);
        firing.modulator = 1.0;
        let outcome = step(params, firing, state);
        state = outcome.state;
        assert_eq!(state.trace_q16, ONE_Q16 * 3 / 2);
        assert_eq!(state.learned_q16, ONE_Q16 * 3 / 2);

        // Half decay, no further eligibility: both integrators halve, and
        // they halve together because they share the one decay gene.
        params.decay_q16 = ONE_Q16 / 2;
        let quiet = signals(0.0, 0.0);
        let outcome = step(params, quiet, state);
        state = outcome.state;
        assert_eq!(state.trace_q16, ONE_Q16 * 3 / 4);
        assert_eq!(state.learned_q16, ONE_Q16 * 3 / 4);

        // A negative modulator discharges the trace the other way, so what
        // an organism treats as reinforcing has a sign it evolved.
        let mut opposing = signals(0.0, 0.0);
        opposing.modulator = -1.0;
        let before = state.learned_q16;
        let outcome = step(params, opposing, state);
        assert!(learned(outcome) < before / 2);

        // Rules 1, 2 and 3 never touch the trace.
        for rule_id in [RULE_HEBBIAN, RULE_OJA, RULE_MODULATED_HEBBIAN] {
            let seeded = LearnedState {
                learned_q16: 0,
                trace_q16: 12_345,
            };
            let outcome = step(rule(rule_id, [1.0; 4]), signals(1.0, 1.0), seeded);
            assert_eq!(outcome.state.trace_q16, 12_345);
        }
    }

    #[test]
    fn static_rule_is_a_complete_no_op() {
        let state = LearnedState {
            learned_q16: 300_000,
            trace_q16: 200_000,
        };
        let mut params = rule(RULE_STATIC, [1.0; 4]);
        params.decay_q16 = ONE_Q16; // a full decay, which would zero both
        let outcome = step(params, signals(1.0, 1.0), state);
        assert_eq!(outcome.kind, StepKind::Static);
        assert_eq!(outcome.state, state);
        assert!(!outcome.fault);

        // The control: identical parameters under rule 1 do move the state,
        // so the assertion above is about rule 0 and not about inputs that
        // happened to be inert.
        let mut moved = params;
        moved.rule_id = RULE_HEBBIAN;
        let outcome = step(moved, signals(1.0, 1.0), state);
        assert_ne!(outcome.state.learned_q16, state.learned_q16);
    }

    #[test]
    fn rounding_is_half_away_from_zero_at_ties() {
        // Exact ties in Q16 terms. Half-to-even - the natural mistake -
        // would give 0, 2, 2, 0, -2, -2 for these six.
        assert_eq!(to_q16(0.5 / Q16_SCALE), 1);
        assert_eq!(to_q16(1.5 / Q16_SCALE), 2);
        assert_eq!(to_q16(2.5 / Q16_SCALE), 3);
        assert_eq!(to_q16(-0.5 / Q16_SCALE), -1);
        assert_eq!(to_q16(-1.5 / Q16_SCALE), -2);
        assert_eq!(to_q16(-2.5 / Q16_SCALE), -3);

        // Below the tie rounds toward zero on both sides.
        assert_eq!(to_q16(0.25 / Q16_SCALE), 0);
        assert_eq!(to_q16(-0.25 / Q16_SCALE), 0);
        assert_eq!(to_q16(1.25 / Q16_SCALE), 1);
        assert_eq!(to_q16(-1.25 / Q16_SCALE), -1);

        // Whole values are unaffected by the rounding term.
        assert_eq!(to_q16(1.0), ONE_Q16);
        assert_eq!(to_q16(-1.0), -ONE_Q16);
        assert_eq!(to_q16(8.0), LEARN_LIMIT_Q16);
        assert_eq!(to_q16(-8.0), -LEARN_LIMIT_Q16);
    }

    #[test]
    fn rounding_is_sign_symmetric_across_the_zero_boundary() {
        // The property that matters: no magnitude rounds differently
        // depending on its sign, because such an asymmetry applied every
        // tick for a lifetime is a drift in what the organism learns.
        for numerator in 0..4_000_i32 {
            let value = f32::from(numerator as i16) * 0.25 / Q16_SCALE;
            assert_eq!(to_q16(-value), -to_q16(value), "value {value}");
        }
        assert_eq!(to_q16(0.0), 0);
        assert_eq!(to_q16(-0.0), 0);
        // Subnormal-scale inputs collapse to zero from both sides rather
        // than to +/-1.
        assert_eq!(to_q16(1e-30), 0);
        assert_eq!(to_q16(-1e-30), 0);

        // Non-finite input is neutralized and reported rather than
        // saturating the cast.
        assert_eq!(to_q16_checked(f32::INFINITY), (0, true));
        assert_eq!(to_q16_checked(f32::NEG_INFINITY), (0, true));
        assert_eq!(to_q16_checked(f32::NAN), (0, true));
        assert_eq!(to_q16_checked(1.0), (ONE_Q16, false));
    }

    #[test]
    fn decay_truncates_toward_zero_for_both_signs() {
        let half = ONE_Q16 / 2;
        // 3 - trunc(1.5) = 2 and -3 - trunc(-1.5) = -2. An arithmetic shift
        // would floor the second product to -2 and give -1, decaying
        // negative learned weights faster than positive ones.
        assert_eq!(decay_toward_zero(3, half), 2);
        assert_eq!(decay_toward_zero(-3, half), -2);
        let shifted = (-3_i32 * half) >> 16;
        assert_eq!(shifted, -2, "the shift floors the product");
        assert_ne!(-3 - shifted, decay_toward_zero(-3, half));

        // Sign symmetry over a wide sweep, which is the property the
        // individual cases are examples of.
        for value in [1, 2, 3, 7, 1_000, 65_535, 65_537, 300_000, LEARN_LIMIT_Q16] {
            for decay_q16 in [0, 1, 7, 1_000, half, ONE_Q16 - 1, ONE_Q16] {
                assert_eq!(
                    decay_toward_zero(-value, decay_q16),
                    -decay_toward_zero(value, decay_q16),
                    "value {value} decay {decay_q16}"
                );
                // Decay never changes the sign and never grows the value.
                assert!(decay_toward_zero(value, decay_q16) <= value);
                assert!(decay_toward_zero(value, decay_q16) >= 0);
            }
        }

        assert_eq!(decay_toward_zero(12_345, 0), 12_345);
        assert_eq!(decay_toward_zero(12_345, ONE_Q16), 0);
        assert_eq!(decay_toward_zero(-12_345, ONE_Q16), 0);
        // A value smaller than the decay resolution does not vanish, which
        // is what stops a large decay from being a hard reset.
        assert_eq!(decay_toward_zero(1, 1), 1);
        assert_eq!(decay_toward_zero(-1, 1), -1);
    }

    #[test]
    fn decay_of_a_large_learned_value_does_not_overflow_i32() {
        // LEARN_LIMIT_Q16 * ONE_Q16 is 2^35. Computed in i32 that wraps to
        // exactly 0 in --release, so a full decay would leave the value
        // untouched and nothing would panic to say so. This is the single
        // assertion that catches it.
        assert_eq!(decay_toward_zero(LEARN_LIMIT_Q16, ONE_Q16), 0);
        assert_eq!(decay_toward_zero(-LEARN_LIMIT_Q16, ONE_Q16), 0);
        assert_eq!(decay_toward_zero(LEARN_LIMIT_Q16, ONE_Q16 / 2), 262_144);
        assert_eq!(decay_toward_zero(-LEARN_LIMIT_Q16, ONE_Q16 / 2), -262_144);
        // Well beyond the clamp, in case a corrupt restore ever hands one in.
        assert_eq!(decay_toward_zero(i32::MAX, ONE_Q16), 0);
        assert_eq!(decay_toward_zero(i32::MIN + 1, ONE_Q16), 0);
        assert_eq!(decay_toward_zero(i32::MAX, ONE_Q16 / 2), 1_073_741_824);
    }

    #[test]
    fn clamp_saturates_from_both_directions_without_wrapping() {
        assert_eq!(accumulate_clamped(0, ONE_Q16), (ONE_Q16, false));
        assert_eq!(
            accumulate_clamped(LEARN_LIMIT_Q16 - 1, 1),
            (LEARN_LIMIT_Q16, false)
        );
        assert_eq!(
            accumulate_clamped(LEARN_LIMIT_Q16, 1),
            (LEARN_LIMIT_Q16, true)
        );
        assert_eq!(
            accumulate_clamped(-LEARN_LIMIT_Q16, -1),
            (-LEARN_LIMIT_Q16, true)
        );
        // i32 + i32 wraps in --release; a saturating cast in to_q16 can hand
        // this i32::MAX, and a wrap would turn a runaway positive update
        // into a large negative learned weight.
        assert_eq!(
            accumulate_clamped(LEARN_LIMIT_Q16, i32::MAX),
            (LEARN_LIMIT_Q16, true)
        );
        assert_eq!(
            accumulate_clamped(-LEARN_LIMIT_Q16, i32::MIN),
            (-LEARN_LIMIT_Q16, true)
        );
        assert_eq!(
            accumulate_clamped(i32::MAX, i32::MAX),
            (LEARN_LIMIT_Q16, true)
        );
        assert_eq!(
            accumulate_clamped(i32::MIN, i32::MIN),
            (-LEARN_LIMIT_Q16, true)
        );

        // A finite value large enough to saturate the float-to-int cast.
        // Unreachable through validated genes - coefficients are bounded to
        // [-1, 1] and activations to [-8, 8], so |raw| never exceeds 576 -
        // but the cast saturates rather than wrapping and step 5 then
        // clamps, so neither a corrupt restore nor a future wider bound can
        // turn a huge positive update into a negative learned weight.
        assert_eq!(to_q16(1e30), i32::MAX);
        assert_eq!(to_q16(-1e30), i32::MIN);
        let outcome = step(
            rule(RULE_HEBBIAN, [0.0, 0.0, 0.0, 1e30]),
            signals(0.0, 0.0),
            LearnedState::default(),
        );
        assert_eq!(outcome.state.learned_q16, LEARN_LIMIT_Q16);
        assert!(outcome.clamped);
        assert!(!outcome.fault);
        let outcome = step(
            rule(RULE_HEBBIAN, [0.0, 0.0, 0.0, -1e30]),
            signals(0.0, 0.0),
            LearnedState::default(),
        );
        assert_eq!(outcome.state.learned_q16, -LEARN_LIMIT_Q16);
    }

    #[test]
    fn repeated_saturating_updates_stay_pinned_at_the_limit() {
        // d = 1 with eta 1 drives +1.0 per tick, so the clamp is reached at
        // tick 8 and every later tick saturates.
        let params = rule(RULE_HEBBIAN, [0.0, 0.0, 0.0, 1.0]);
        let mut state = LearnedState::default();
        let mut counters = PlasticityCounters::default();
        for tick in 1..=64_u32 {
            let outcome = step(params, signals(0.0, 0.0), state);
            counters.record(&outcome);
            state = outcome.state;
            let expected = (i64::from(tick) * i64::from(ONE_Q16)).min(i64::from(LEARN_LIMIT_Q16));
            assert_eq!(i64::from(state.learned_q16), expected, "tick {tick}");
            assert!(state.learned_q16 > 0, "wrapped negative at tick {tick}");
        }
        assert_eq!(state.learned_q16, LEARN_LIMIT_Q16);
        // Saturation begins at tick 9: tick 8 lands exactly on the limit.
        assert_eq!(counters.clamped, 64 - 8);
        assert_eq!(counters.updates_applied, 64);
        assert_eq!(counters.faults, 0);

        // And the same from below.
        let negative = rule(RULE_HEBBIAN, [0.0, 0.0, 0.0, -1.0]);
        let mut state = LearnedState::default();
        for _ in 0..64 {
            state = step(negative, signals(0.0, 0.0), state).state;
            assert!(state.learned_q16 >= -LEARN_LIMIT_Q16);
        }
        assert_eq!(state.learned_q16, -LEARN_LIMIT_Q16);
    }

    #[test]
    fn effective_weight_adds_the_learned_delta_and_clamps() {
        assert_eq!(effective_weight(0.0, 0), 0.0);
        assert_eq!(effective_weight(1.0, ONE_Q16), 2.0);
        assert_eq!(effective_weight(1.0, -ONE_Q16), 0.0);
        assert_eq!(effective_weight(0.0, LEARN_LIMIT_Q16), 8.0);
        assert_eq!(effective_weight(0.0, -LEARN_LIMIT_Q16), -8.0);
        // Clamped: 8 + 8 is 16 before the clamp.
        assert_eq!(effective_weight(8.0, LEARN_LIMIT_Q16), 8.0);
        assert_eq!(effective_weight(-8.0, -LEARN_LIMIT_Q16), -8.0);
        // A single Q16 unit survives the round trip, so the resolution the
        // fixed-point state carries is really visible to evaluation.
        assert_eq!(effective_weight(0.0, 1), 1.0 / Q16_SCALE);
        assert_ne!(effective_weight(0.0, 1), 0.0);
    }

    #[test]
    fn non_finite_delta_is_neutralized_counted_and_does_not_panic() {
        // Start away from zero with no decay, so "the state did not move"
        // is a real assertion rather than 0 == 0.
        let start = LearnedState {
            learned_q16: 100_000,
            trace_q16: 0,
        };

        // (label, rule, signals) pairs that must fault, each with a control
        // that differs only in the value that was made non-finite.
        let mut oja_broken = signals(1.0, 1.0);
        oja_broken.w_eff = f32::INFINITY;
        let mut oja_ok = signals(1.0, 1.0);
        oja_ok.w_eff = 2.0;

        let mut modulated_broken = signals(1.0, 1.0);
        modulated_broken.modulator = f32::NAN;
        let mut modulated_ok = signals(1.0, 1.0);
        modulated_ok.modulator = 1.0;

        let cases: [(
            &str,
            PlasticityRule,
            EdgeSignals,
            PlasticityRule,
            EdgeSignals,
        ); 4] = [
            (
                "infinite w_eff through Oja",
                rule(RULE_OJA, [0.0; 4]),
                oja_broken,
                rule(RULE_OJA, [0.0; 4]),
                oja_ok,
            ),
            (
                "NaN coefficient through the Hebbian form",
                rule(RULE_HEBBIAN, [0.0, 0.0, 0.0, f32::NAN]),
                signals(1.0, 1.0),
                rule(RULE_HEBBIAN, [0.0, 0.0, 0.0, 0.5]),
                signals(1.0, 1.0),
            ),
            (
                "NaN modulator activation",
                rule(RULE_MODULATED_HEBBIAN, [1.0, 0.0, 0.0, 0.0]),
                modulated_broken,
                rule(RULE_MODULATED_HEBBIAN, [1.0, 0.0, 0.0, 0.0]),
                modulated_ok,
            ),
            (
                "infinite coefficient into the eligibility trace",
                rule(RULE_ELIGIBILITY_TRACE, [f32::INFINITY, 0.0, 0.0, 0.0]),
                modulated_ok,
                rule(RULE_ELIGIBILITY_TRACE, [1.0, 0.0, 0.0, 0.0]),
                modulated_ok,
            ),
        ];

        let mut counters = PlasticityCounters::default();
        for (label, broken, broken_signals, control, control_signals) in cases {
            let outcome = step(broken, broken_signals, start);
            counters.record(&outcome);
            assert!(outcome.fault, "{label} should fault");
            assert_eq!(outcome.kind, StepKind::Applied, "{label}");
            // Neutralized to zero: with no decay the state is untouched.
            assert_eq!(outcome.state.learned_q16, start.learned_q16, "{label}");
            assert_eq!(outcome.state.trace_q16, 0, "{label}");

            let control_outcome = step(control, control_signals, start);
            assert!(!control_outcome.fault, "{label} control should not fault");
            assert_ne!(
                control_outcome.state.learned_q16, start.learned_q16,
                "{label} control must move the state, or the fault case proves nothing"
            );
        }
        assert_eq!(counters.faults, 4);
        assert_eq!(counters.updates_applied, 4);

        // eta of zero does not make an infinity safe: 0 * inf is NaN, and an
        // implementation that checked only the rule form would miss it.
        let mut zero_eta = rule(RULE_OJA, [0.0; 4]);
        zero_eta.eta = 0.0;
        let outcome = step(zero_eta, oja_broken, start);
        assert!(outcome.fault);
        assert_eq!(outcome.state.learned_q16, start.learned_q16);

        // A faulting edge still decays rather than freezing at whatever it
        // had learned before the fault.
        let mut decaying = rule(RULE_OJA, [0.0; 4]);
        decaying.decay_q16 = ONE_Q16 / 2;
        let outcome = step(decaying, oja_broken, start);
        assert!(outcome.fault);
        assert_eq!(outcome.state.learned_q16, 50_000);
    }

    #[test]
    fn adversarial_sweep_keeps_learned_state_and_effective_weight_in_bounds() {
        // A fixed deterministic product sweep, not a random crate: the grid
        // is the same on every run and on every machine, so a failure is
        // reproducible from the printed indices alone.
        const RULES: [u8; 6] = [0, 1, 2, 3, 4, 9];
        const COEFFICIENTS: [[f32; 4]; 5] = [
            [1.0, 1.0, 1.0, 1.0],
            [-1.0, -1.0, -1.0, -1.0],
            [1.0, -1.0, 1.0, -1.0],
            [0.0, 0.0, 0.0, 1.0],
            [1.0, 0.0, 0.0, 0.0],
        ];
        const ACTIVATIONS: [f32; 5] = [-8.0, -1.0, 0.0, 1.0, 8.0];
        const W_EFF: [f32; 3] = [-8.0, 0.0, 8.0];
        const ETA: [f32; 3] = [0.0, 0.5, 1.0];
        const DECAY: [i32; 4] = [0, 1, ONE_Q16 / 2, ONE_Q16];
        const START: [i32; 3] = [-LEARN_LIMIT_Q16, 0, LEARN_LIMIT_Q16 / 2];
        const MODULATOR: [f32; 4] = [-1.0, 0.0, 0.5, 1.0];
        const GENOME_WEIGHTS: [f32; 3] = [-8.0, 0.0, 8.0];

        let mut counters = PlasticityCounters::default();
        let mut calls = 0_u64;
        let mut nonzero_learned = 0_u64;
        let mut hit_high = 0_u64;
        let mut hit_low = 0_u64;
        let mut nonzero_trace = 0_u64;

        for rule_id in RULES {
            for coefficients in COEFFICIENTS {
                for pre in ACTIVATIONS {
                    for post in ACTIVATIONS {
                        for w_eff in W_EFF {
                            for eta in ETA {
                                for decay_q16 in DECAY {
                                    for start in START {
                                        for modulator in MODULATOR {
                                            let params = PlasticityRule {
                                                rule_id,
                                                eta,
                                                coefficients,
                                                decay_q16,
                                            };
                                            let input = EdgeSignals {
                                                pre,
                                                post,
                                                modulator,
                                                w_eff,
                                            };
                                            let mut state = LearnedState {
                                                learned_q16: start,
                                                trace_q16: start,
                                            };
                                            // Several ticks, so the state
                                            // feeds back into itself the way
                                            // it does over a lifetime.
                                            for _ in 0..8 {
                                                let outcome = step(params, input, state);
                                                counters.record(&outcome);
                                                calls += 1;
                                                state = outcome.state;
                                                assert!(
                                                    (-LEARN_LIMIT_Q16..=LEARN_LIMIT_Q16)
                                                        .contains(&state.learned_q16),
                                                    "rule {rule_id} coefficients {coefficients:?} \
                                                     x {pre} y {post} w {w_eff} eta {eta} \
                                                     decay {decay_q16} start {start} \
                                                     modulator {modulator} left the clamp at \
                                                     {}",
                                                    state.learned_q16
                                                );
                                                assert!(
                                                    (-LEARN_LIMIT_Q16..=LEARN_LIMIT_Q16)
                                                        .contains(&state.trace_q16)
                                                );
                                                for genome_weight in GENOME_WEIGHTS {
                                                    let w = effective_weight(
                                                        genome_weight,
                                                        state.learned_q16,
                                                    );
                                                    assert!(w.is_finite());
                                                    assert!((-8.0..=8.0).contains(&w));
                                                }
                                                if state.learned_q16 != 0 {
                                                    nonzero_learned += 1;
                                                }
                                                if state.learned_q16 == LEARN_LIMIT_Q16 {
                                                    hit_high += 1;
                                                }
                                                if state.learned_q16 == -LEARN_LIMIT_Q16 {
                                                    hit_low += 1;
                                                }
                                                if state.trace_q16 != 0 {
                                                    nonzero_trace += 1;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // The sweep proved nothing unless the mechanism actually fired. All
        // five guards below are about that, not about the bound.
        assert_eq!(counters.total_evaluated(), calls);
        assert!(nonzero_learned > calls / 10, "learned state barely moved");
        assert!(hit_high > 0, "the upper clamp was never reached");
        assert!(hit_low > 0, "the lower clamp was never reached");
        assert!(nonzero_trace > 0, "the eligibility trace never accumulated");
        assert!(counters.clamped > 0, "the clamp never engaged");
        // Every input in the grid is finite, so nothing may fault. A fault
        // here means an intermediate went non-finite from finite inputs.
        assert_eq!(counters.faults, counters.updates_refused);
        // The out-of-registry rule id is refused on every visit, and rule 0
        // on every visit.
        let per_rule = calls / RULES.len() as u64;
        assert_eq!(counters.updates_refused, per_rule);
        assert_eq!(counters.updates_static, per_rule);
        assert_eq!(counters.updates_applied, calls - 2 * per_rule);
    }

    #[test]
    fn a_million_step_trace_is_drift_free_and_lands_on_a_hand_derived_fixed_point() {
        const TICKS: u32 = 1_000_000;

        // 1. Alternating drive with no decay. b = 1 makes the delta exactly
        //    x, so the learned value walks +1, -1, +1, ... and must return
        //    to *exactly* zero after an even number of ticks. Any float
        //    accumulation in the loop would drift off zero long before a
        //    million ticks; integer accumulation cannot.
        let params = rule(RULE_HEBBIAN, [0.0, 1.0, 0.0, 0.0]);
        let mut state = LearnedState::default();
        let mut peak = 0;
        for tick in 0..TICKS {
            let pre = if tick % 2 == 0 { 1.0 } else { -1.0 };
            state = step(params, signals(pre, 0.0), state).state;
            peak = peak.max(state.learned_q16);
        }
        assert_eq!(state.learned_q16, 0);
        assert_eq!(peak, ONE_Q16, "the drive never actually moved the weight");

        // 2. Constant drive with no decay saturates at tick 8 and stays
        //    exactly on the limit for the remaining million ticks - no
        //    creep, no wrap.
        let params = rule(RULE_HEBBIAN, [0.0, 0.0, 0.0, 1.0]);
        let mut state = LearnedState::default();
        for _ in 0..TICKS {
            state = step(params, signals(0.0, 0.0), state).state;
        }
        assert_eq!(state.learned_q16, LEARN_LIMIT_Q16);

        // 3. Constant drive against a half decay. The recurrence is
        //    l' = l - trunc(l/2) + 65536, whose only fixed point is
        //    l = 131072: 131072 - 65536 + 65536. Starting from zero the gap
        //    halves every tick, so it arrives exactly at tick 18 and never
        //    moves again. The value is hand-derived, not recorded from a
        //    previous run, so this pins the arithmetic rather than pinning
        //    whatever the arithmetic happened to do.
        let mut params = rule(RULE_HEBBIAN, [0.0, 0.0, 0.0, 1.0]);
        params.decay_q16 = ONE_Q16 / 2;
        let mut state = LearnedState::default();
        let mut arrived_at = None;
        for tick in 1..=TICKS {
            state = step(params, signals(0.0, 0.0), state).state;
            if state.learned_q16 == 2 * ONE_Q16 && arrived_at.is_none() {
                arrived_at = Some(tick);
            }
            assert!(state.learned_q16 <= 2 * ONE_Q16);
        }
        assert_eq!(arrived_at, Some(18));
        assert_eq!(state.learned_q16, 2 * ONE_Q16);
    }

    #[test]
    fn counters_partition_every_call_and_reach_the_checksum() {
        let base = PlasticityCounters {
            updates_applied: 11,
            updates_static: 22,
            updates_refused: 33,
            faults: 44,
            clamped: 55,
            trace_clamped: 66,
        };
        assert_eq!(base.total_evaluated(), 66);
        assert_eq!(base.total_anomalies(), 165);

        let digest = |counters: &PlasticityCounters| {
            let mut hasher = Fnv1a64::new();
            counters.hash_into(&mut hasher);
            hasher.finish()
        };
        let reference = digest(&base);

        // Every field must reach the hash. A field left out of
        // `partitioned` cannot compile, but a field hashed as a constant, or
        // a bucket dropped from the chain, would - and this is what catches
        // that.
        let mutators: [fn(&mut PlasticityCounters); 6] = [
            |c| c.updates_applied += 1,
            |c| c.updates_static += 1,
            |c| c.updates_refused += 1,
            |c| c.faults += 1,
            |c| c.clamped += 1,
            |c| c.trace_clamped += 1,
        ];
        for (index, mutate) in mutators.into_iter().enumerate() {
            let mut moved = base;
            mutate(&mut moved);
            assert_ne!(digest(&moved), reference, "field {index} missed the hash");
        }

        // Two counters that differ only by which bucket a count sits in must
        // hash differently, so the ordering is pinned and not just the sum.
        let swapped = PlasticityCounters {
            updates_applied: 22,
            updates_static: 11,
            ..base
        };
        assert_ne!(digest(&swapped), reference);

        // `record` puts each kind in exactly one bucket.
        let mut counters = PlasticityCounters::default();
        for kind in [StepKind::Applied, StepKind::Static, StepKind::Refused] {
            counters.record(&StepOutcome {
                state: LearnedState::default(),
                kind,
                fault: kind == StepKind::Refused,
                clamped: false,
                trace_clamped: false,
            });
        }
        assert_eq!(counters.total_evaluated(), 3);
        assert_eq!(counters.updates_applied, 1);
        assert_eq!(counters.updates_static, 1);
        assert_eq!(counters.updates_refused, 1);
        assert_eq!(counters.faults, 1);
    }

    #[test]
    fn decay_gene_conversion_is_bounded() {
        assert_eq!(decay_to_q16(0.0), 0);
        assert_eq!(decay_to_q16(1.0), ONE_Q16);
        assert_eq!(decay_to_q16(0.5), ONE_Q16 / 2);
        assert_eq!(decay_to_q16(0.25), ONE_Q16 / 4);
        // Out of range in either direction collapses to a value the update
        // can use, and the safe direction for a validation gap is the one
        // that changes nothing.
        assert_eq!(decay_to_q16(-1.0), 0);
        assert_eq!(decay_to_q16(2.0), ONE_Q16);
        assert_eq!(decay_to_q16(f32::NAN), 0);
        assert_eq!(decay_to_q16(f32::INFINITY), 0);
        assert_eq!(decay_to_q16(f32::NEG_INFINITY), 0);
        // Whatever comes out is a legal decay for the update.
        for gene in [-5.0_f32, 0.0, 0.001, 0.5, 0.999, 1.0, 5.0] {
            let converted = decay_to_q16(gene);
            assert!((0..=ONE_Q16).contains(&converted), "gene {gene}");
            assert_eq!(decay_toward_zero(0, converted), 0);
        }
    }
}
