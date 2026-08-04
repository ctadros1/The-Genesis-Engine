//! Fixed-topology controller evaluator (topology ID 1).
//!
//! Evaluation uses only f32 add/multiply/divide/compare with a rational
//! tanh approximation, so no libm transcendental is involved and results
//! are reproducible on the recorded build/platform. The evaluator performs
//! zero heap allocation; all intermediates live on the stack.
//!
//! Controllers request actions; they never mutate world state. Output
//! interpretation and thresholds are versioned config policy.

use crate::genome::{
    CONTROLLER_INPUTS, CONTROLLER_OUTPUTS, Genome, HIDDEN_1, HIDDEN_2, MEMORY_VALUES,
};

/// Version string folded into the config hash when Phase 2 is enabled.
/// Covers topology interpretation, activation approximation, clamps, and
/// the non-finite neutralization policy.
pub const CONTROLLER_POLICY_VERSION: &str = "lifesim-controller-v1";

/// Output channel indices (see `docs/07-neural-network-design.md`).
pub const OUT_TURN: usize = 0;
pub const OUT_THROTTLE: usize = 1;
pub const OUT_EAT: usize = 2;
pub const OUT_ATTACK: usize = 3; // documented no-op in Phase 2
pub const OUT_REST: usize = 4;
pub const OUT_MATE: usize = 5;
pub const OUT_FOLLOW: usize = 6;
pub const OUT_AVOID: usize = 7;
pub const OUT_MEMORY_BASE: usize = 8;

/// Pre-activation sums are clamped here before the activation function.
const ACTIVATION_LIMIT: f32 = 8.0;

/// Result of one controller evaluation. `faults` counts neutralized
/// non-finite values (inputs or intermediate sums).
#[derive(Clone, Copy, Debug)]
pub struct ControllerOutput {
    pub outputs: [f32; CONTROLLER_OUTPUTS],
    pub faults: u32,
}

/// Deterministic rational tanh approximation: x * (27 + x^2) / (27 + 9x^2)
/// on the clamped domain, then clamped to [-1, 1]. This is a versioned
/// policy choice, not an accuracy claim about libm tanh.
pub fn tanh_approx(value: f32) -> f32 {
    let x = value.clamp(-3.0, 3.0);
    let x2 = x * x;
    (x * (27.0 + x2) / (27.0 + 9.0 * x2)).clamp(-1.0, 1.0)
}

/// Evaluate one controller. Inputs are sanitized (non-finite becomes 0.0
/// and counts as a fault) and clamped to [-1, 1] before use. The genome is
/// valid by construction, so weights and biases are already bounded.
pub fn evaluate(genome: &Genome, raw_inputs: &[f32; CONTROLLER_INPUTS]) -> ControllerOutput {
    let mut faults = 0_u32;
    let mut inputs = [0.0_f32; CONTROLLER_INPUTS];
    for (index, &raw) in raw_inputs.iter().enumerate() {
        if raw.is_finite() {
            inputs[index] = raw.clamp(-1.0, 1.0);
        } else {
            faults += 1;
        }
    }

    let neural = genome.neural();
    let mut offset = 0_usize;
    let mut hidden_1 = [0.0_f32; HIDDEN_1];
    layer(
        &inputs,
        &mut hidden_1,
        &neural[offset..offset + CONTROLLER_INPUTS * HIDDEN_1 + HIDDEN_1],
        &mut faults,
    );
    offset += CONTROLLER_INPUTS * HIDDEN_1 + HIDDEN_1;

    let mut hidden_2 = [0.0_f32; HIDDEN_2];
    layer(
        &hidden_1,
        &mut hidden_2,
        &neural[offset..offset + HIDDEN_1 * HIDDEN_2 + HIDDEN_2],
        &mut faults,
    );
    offset += HIDDEN_1 * HIDDEN_2 + HIDDEN_2;

    let mut outputs = [0.0_f32; CONTROLLER_OUTPUTS];
    layer(
        &hidden_2,
        &mut outputs,
        &neural[offset..offset + HIDDEN_2 * CONTROLLER_OUTPUTS + CONTROLLER_OUTPUTS],
        &mut faults,
    );

    ControllerOutput { outputs, faults }
}

/// Dense layer: layer-major weights (weight[out][in]) followed by biases,
/// pre-activation clamp, rational tanh, post-activation clamp.
fn layer(inputs: &[f32], outputs: &mut [f32], parameters: &[f32], faults: &mut u32) {
    let input_count = inputs.len();
    let output_count = outputs.len();
    debug_assert_eq!(parameters.len(), input_count * output_count + output_count);
    let biases = &parameters[input_count * output_count..];
    for (out_index, output) in outputs.iter_mut().enumerate() {
        let weights = &parameters[out_index * input_count..(out_index + 1) * input_count];
        let mut sum = biases[out_index];
        for (weight, input) in weights.iter().zip(inputs.iter()) {
            sum += weight * input;
        }
        if !sum.is_finite() {
            *faults += 1;
            sum = 0.0;
        }
        *output = tanh_approx(sum.clamp(-ACTIVATION_LIMIT, ACTIVATION_LIMIT));
    }
}

/// Extract the next memory values from a controller output.
pub fn next_memory(output: &ControllerOutput) -> [f32; MEMORY_VALUES] {
    let mut memory = [0.0_f32; MEMORY_VALUES];
    for (index, slot) in memory.iter_mut().enumerate() {
        *slot = output.outputs[OUT_MEMORY_BASE + index].clamp(-1.0, 1.0);
    }
    memory
}

// --- Deterministic fixed-point trigonometry for movement ------------------
//
// Headings are u16 binary angular measure (BAM): 65536 units per full turn.
// sin/cos use the integer Bhaskara I approximation, Q15 output. Pure
// integer arithmetic; part of the phase2 behavior policy.

const HALF_TURN: i64 = 32768;

/// Q15 sine of a BAM angle (32768 == 1.0). Maximum absolute error of the
/// Bhaskara approximation is under 0.2 percent of full scale.
pub fn sin_bam_q15(angle: u16) -> i32 {
    let bam = i64::from(angle);
    let (half_angle, sign) = if bam < HALF_TURN {
        (bam, 1_i64)
    } else {
        (bam - HALF_TURN, -1_i64)
    };
    let product = half_angle * (HALF_TURN - half_angle);
    // Bhaskara I: sin = 16 t / (5 pi^2 - 4 t) with pi == HALF_TURN BAM.
    let numerator = 32768 * 16 * product;
    let denominator = 5 * HALF_TURN * HALF_TURN - 4 * product;
    (sign * numerator / denominator) as i32
}

/// Q15 cosine of a BAM angle.
pub fn cos_bam_q15(angle: u16) -> i32 {
    sin_bam_q15(angle.wrapping_add(16384))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genome::{NEURAL_COUNT, TRAIT_COUNT, WEIGHT_LIMIT};

    fn genome_with(neural: Vec<f32>) -> Genome {
        Genome::validated([0.5; TRAIT_COUNT], neural).unwrap()
    }

    #[test]
    fn all_zero_genome_produces_all_zero_outputs() {
        let genome = genome_with(vec![0.0; NEURAL_COUNT]);
        let output = evaluate(&genome, &[0.25; CONTROLLER_INPUTS]);
        assert_eq!(output.faults, 0);
        for &value in &output.outputs {
            assert_eq!(value, 0.0);
        }
    }

    #[test]
    fn known_directional_fixture_is_stable() {
        // First hidden unit reads input 0 with weight 1; all other first-
        // layer weights zero. Second layer unit 0 reads hidden 0 with
        // weight 2; output 0 reads hidden2 0 with weight 2.
        let mut neural = vec![0.0_f32; NEURAL_COUNT];
        neural[0] = 1.0; // w1[0][0]
        let l2_weights = 20 * 16 + 16;
        neural[l2_weights] = 2.0; // w2[0][0]
        let l3_weights = l2_weights + 16 * 12 + 12;
        neural[l3_weights] = 2.0; // w3[0][0]
        let genome = genome_with(neural);

        let mut inputs = [0.0_f32; CONTROLLER_INPUTS];
        inputs[0] = 1.0;
        let output = evaluate(&genome, &inputs);
        assert_eq!(output.faults, 0);
        // tanh chain: tanh(1) ~ 0.7616 -> tanh(1.523) ~ 0.909 -> tanh(1.819) ~ 0.949
        // (rational approximation, exact expected value pinned by policy).
        let expected = {
            let h1 = tanh_approx(1.0);
            let h2 = tanh_approx(2.0 * h1);
            tanh_approx(2.0 * h2)
        };
        assert_eq!(output.outputs[0], expected);
        assert!(output.outputs[0] > 0.9);
        // Same inputs, same genome: bit-identical result.
        let repeat = evaluate(&genome, &inputs);
        assert_eq!(output.outputs, repeat.outputs);
        // Negated input flips the sign symmetric chain.
        inputs[0] = -1.0;
        let negated = evaluate(&genome, &inputs);
        assert_eq!(negated.outputs[0], -expected);
    }

    #[test]
    fn saturated_and_extreme_weight_fixtures_stay_bounded() {
        for fill in [WEIGHT_LIMIT, -WEIGHT_LIMIT] {
            let genome = genome_with(vec![fill; NEURAL_COUNT]);
            for input_fill in [-1.0_f32, 0.0, 1.0] {
                let output = evaluate(&genome, &[input_fill; CONTROLLER_INPUTS]);
                assert_eq!(output.faults, 0);
                for &value in &output.outputs {
                    assert!((-1.0..=1.0).contains(&value));
                    assert!(value.is_finite());
                }
            }
        }
    }

    #[test]
    fn non_finite_inputs_are_neutralized_and_counted() {
        let genome = genome_with(vec![1.0; NEURAL_COUNT]);
        let mut inputs = [0.5_f32; CONTROLLER_INPUTS];
        inputs[3] = f32::NAN;
        inputs[7] = f32::INFINITY;
        inputs[9] = f32::NEG_INFINITY;
        let output = evaluate(&genome, &inputs);
        assert_eq!(output.faults, 3);
        for &value in &output.outputs {
            assert!(value.is_finite());
            assert!((-1.0..=1.0).contains(&value));
        }
        // Neutralization is equivalent to zeroing those channels.
        let mut zeroed = inputs;
        zeroed[3] = 0.0;
        zeroed[7] = 0.0;
        zeroed[9] = 0.0;
        assert_eq!(output.outputs, evaluate(&genome, &zeroed).outputs);
    }

    #[test]
    fn out_of_range_inputs_are_clamped() {
        let genome = genome_with(vec![0.5; NEURAL_COUNT]);
        let wild = evaluate(&genome, &[1_000.0; CONTROLLER_INPUTS]);
        let clamped = evaluate(&genome, &[1.0; CONTROLLER_INPUTS]);
        assert_eq!(wild.outputs, clamped.outputs);
        assert_eq!(wild.faults, 0);
    }

    #[test]
    fn tanh_approx_is_bounded_monotone_and_odd() {
        assert_eq!(tanh_approx(0.0), 0.0);
        let mut previous = -1.0_f32;
        let mut value = -6.0_f32;
        while value <= 6.0 {
            let result = tanh_approx(value);
            assert!((-1.0..=1.0).contains(&result));
            assert!(result >= previous);
            assert_eq!(tanh_approx(-value), -result);
            previous = result;
            value += 0.125;
        }
        assert!(tanh_approx(3.0) > 0.98);
    }

    #[test]
    fn memory_extraction_uses_channels_8_to_11() {
        let mut output = ControllerOutput {
            outputs: [0.0; CONTROLLER_OUTPUTS],
            faults: 0,
        };
        output.outputs[OUT_MEMORY_BASE] = 0.5;
        output.outputs[OUT_MEMORY_BASE + 3] = -0.25;
        let memory = next_memory(&output);
        assert_eq!(memory, [0.5, 0.0, 0.0, -0.25]);
    }

    #[test]
    fn bam_trig_hits_cardinal_points_and_stays_bounded() {
        assert_eq!(sin_bam_q15(0), 0);
        assert_eq!(sin_bam_q15(16384), 32768);
        assert_eq!(sin_bam_q15(32768), 0);
        assert_eq!(sin_bam_q15(49152), -32768);
        assert_eq!(cos_bam_q15(0), 32768);
        assert_eq!(cos_bam_q15(32768), -32768);
        for step in 0..=1024 {
            let angle = (step * 64) as u16;
            let sin = sin_bam_q15(angle);
            let cos = cos_bam_q15(angle);
            assert!((-32768..=32768).contains(&sin));
            assert!((-32768..=32768).contains(&cos));
        }
    }
}
