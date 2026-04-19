//! Noise models for realistic quantum simulation

use num_complex::Complex64;
use rand::Rng;
use crate::state::QuantumState;
use crate::error::{Error, Result};
use crate::circuit::NoiseConfig;
use crate::gates;

/// Bit flip noise: |0⟩ → |1⟩ or |1⟩ → |0⟩ with probability p
pub fn bit_flip_noise(state: &mut QuantumState, target: usize, prob: f64) -> Result<()> {
    let mut rng = rand::thread_rng();
    bit_flip_noise_with_rng(state, target, prob, &mut rng)
}

/// Bit flip noise with caller-provided RNG.
pub fn bit_flip_noise_with_rng<R: Rng + ?Sized>(
    state: &mut QuantumState,
    target: usize,
    prob: f64,
    rng: &mut R,
) -> Result<()> {
    if !(0.0..=1.0).contains(&prob) {
        return Err(Error::InvalidNoiseConfig {
            reason: format!("Probability must be in [0, 1], got {}", prob),
        });
    }

    if rng.gen::<f64>() < prob {
        gates::gate_x(state, target)?;
    }

    Ok(())
}

/// Phase flip noise: |0⟩ → |0⟩ or |1⟩ → -|1⟩ with probability p
pub fn phase_flip_noise(state: &mut QuantumState, target: usize, prob: f64) -> Result<()> {
    let mut rng = rand::thread_rng();
    phase_flip_noise_with_rng(state, target, prob, &mut rng)
}

/// Phase flip noise with caller-provided RNG.
pub fn phase_flip_noise_with_rng<R: Rng + ?Sized>(
    state: &mut QuantumState,
    target: usize,
    prob: f64,
    rng: &mut R,
) -> Result<()> {
    if !(0.0..=1.0).contains(&prob) {
        return Err(Error::InvalidNoiseConfig {
            reason: format!("Probability must be in [0, 1], got {}", prob),
        });
    }

    if rng.gen::<f64>() < prob {
        gates::gate_z(state, target)?;
    }

    Ok(())
}

/// Depolarizing noise: with probability p, replace with mixed state
/// ρ → (1-p)ρ + p(I/2) for single qubit
pub fn depolarizing_noise(state: &mut QuantumState, target: usize, prob: f64) -> Result<()> {
    let mut rng = rand::thread_rng();
    depolarizing_noise_with_rng(state, target, prob, &mut rng)
}

/// Depolarizing noise with caller-provided RNG.
pub fn depolarizing_noise_with_rng<R: Rng + ?Sized>(
    state: &mut QuantumState,
    target: usize,
    prob: f64,
    rng: &mut R,
) -> Result<()> {
    if !(0.0..=1.0).contains(&prob) {
        return Err(Error::InvalidNoiseConfig {
            reason: format!("Probability must be in [0, 1], got {}", prob),
        });
    }

    let rand_val = rng.gen::<f64>();

    // Apply random Pauli: I (do nothing), X, Y, or Z
    if rand_val < prob {
        match rng.gen_range(0..4) {
            0 => {} // Identity
            1 => gates::gate_x(state, target)?,
            2 => gates::gate_y(state, target)?,
            3 => gates::gate_z(state, target)?,
            _ => unreachable!(),
        }
    }

    Ok(())
}

/// Apply noise to a single qubit based on noise configuration
pub fn apply_noise(state: &mut QuantumState, target: usize, noise: &NoiseConfig) -> Result<()> {
    let mut rng = rand::thread_rng();
    apply_noise_with_rng(state, target, noise, &mut rng)
}

/// Apply noise to a single qubit based on noise configuration with caller-provided RNG.
pub fn apply_noise_with_rng<R: Rng + ?Sized>(
    state: &mut QuantumState,
    target: usize,
    noise: &NoiseConfig,
    rng: &mut R,
) -> Result<()> {
    let noise_type = noise.noise_type.to_uppercase();

    match noise_type.as_str() {
        "BIT_FLIP" => bit_flip_noise_with_rng(state, target, noise.probability, rng),
        "PHASE_FLIP" => phase_flip_noise_with_rng(state, target, noise.probability, rng),
        "DEPOLARIZING" => depolarizing_noise_with_rng(state, target, noise.probability, rng),
        "AMPLITUDE_DAMPING" => amplitude_damping_noise(state, target, noise.probability),
        "T1_T2_RELAXATION" => {
            let t1 = noise.t1.ok_or_else(|| Error::InvalidNoiseConfig {
                reason: "T1_T2_RELAXATION requires t1".to_string(),
            })?;
            let t2 = noise.t2.ok_or_else(|| Error::InvalidNoiseConfig {
                reason: "T1_T2_RELAXATION requires t2".to_string(),
            })?;
            let dt = noise.dt.ok_or_else(|| Error::InvalidNoiseConfig {
                reason: "T1_T2_RELAXATION requires dt".to_string(),
            })?;
            t1_t2_relaxation_noise(state, target, t1, t2, dt, rng)
        }
        "COHERENT_OVER_ROTATION" => {
            let axis = noise.axis.as_deref().unwrap_or("Z");
            let angle = noise.angle.ok_or_else(|| Error::InvalidNoiseConfig {
                reason: "COHERENT_OVER_ROTATION requires angle".to_string(),
            })?;
            coherent_over_rotation_noise(state, target, axis, angle)
        }
        "CROSSTALK" => {
            let coupled = noise.coupled_qubits.as_deref().ok_or_else(|| Error::InvalidNoiseConfig {
                reason: "CROSSTALK requires coupled_qubits".to_string(),
            })?;
            crosstalk_noise(state, coupled, noise.probability, rng)
        }
        "KRAUS" => {
            let ops = noise.kraus.as_ref().ok_or_else(|| Error::InvalidNoiseConfig {
                reason: "KRAUS requires kraus operators".to_string(),
            })?;
            kraus_noise(state, target, ops, rng)
        }
        "COMPOSITE" => {
            let channels = noise.channels.as_ref().ok_or_else(|| Error::InvalidNoiseConfig {
                reason: "COMPOSITE requires channels".to_string(),
            })?;
            apply_noise_chain_with_rng(state, target, channels, rng)
        }
        "READOUT_ERROR" => Ok(()), // Measurement-time noise handled during sampling.
        _ => Err(Error::InvalidNoiseConfig {
            reason: format!("Unknown noise type: {}", noise_type),
        }),
    }
}

/// Apply a sequence of channels to the same target in order.
pub fn apply_noise_chain_with_rng<R: Rng + ?Sized>(
    state: &mut QuantumState,
    target: usize,
    channels: &[NoiseConfig],
    rng: &mut R,
) -> Result<()> {
    for channel in channels {
        apply_noise_with_rng(state, target, channel, rng)?;
    }
    Ok(())
}

/// Amplitude damping noise: dissipation of energy
/// |1⟩ → |0⟩ with probability γ
pub fn amplitude_damping_noise(state: &mut QuantumState, target: usize, gamma: f64) -> Result<()> {
    if !(0.0..=1.0).contains(&gamma) {
        return Err(Error::InvalidNoiseConfig {
            reason: format!("Gamma must be in [0, 1], got {}", gamma),
        });
    }

    let mask = 1 << target;
    let sqrt_gamma = gamma.sqrt();
    let sqrt_1_minus_gamma = (1.0 - gamma).sqrt();
    let amplitudes = state.amplitudes_mut();

    let mut new_amplitudes = amplitudes.clone();

    // E0: |0⟩ -> |0⟩ and |1⟩ -> sqrt(1-γ)|1⟩
    // E1: |1⟩ -> sqrt(γ)|0⟩
    for (idx, &amp) in amplitudes.iter().enumerate() {
        if (idx & mask) != 0 {
            // This is a |1⟩ state for target qubit
            let idx_0 = idx & !mask; // Flip to |0⟩
            new_amplitudes[idx] *= sqrt_1_minus_gamma;
            new_amplitudes[idx_0] += sqrt_gamma * amp;
        }
    }

    *amplitudes = new_amplitudes;
    Ok(())
}

/// Combined T1/T2 relaxation channel using simple exponential decay probabilities.
pub fn t1_t2_relaxation_noise<R: Rng + ?Sized>(
    state: &mut QuantumState,
    target: usize,
    t1: f64,
    t2: f64,
    dt: f64,
    rng: &mut R,
) -> Result<()> {
    if t1 <= 0.0 || t2 <= 0.0 || dt < 0.0 {
        return Err(Error::InvalidNoiseConfig {
            reason: "T1_T2_RELAXATION requires t1,t2 > 0 and dt >= 0".to_string(),
        });
    }

    let gamma1 = 1.0 - (-dt / t1).exp();
    let gamma_phi = 1.0 - (-dt / t2).exp();
    amplitude_damping_noise(state, target, gamma1.clamp(0.0, 1.0))?;
    phase_flip_noise_with_rng(state, target, gamma_phi.clamp(0.0, 1.0), rng)
}

/// Deterministic coherent over-rotation channel.
pub fn coherent_over_rotation_noise(
    state: &mut QuantumState,
    target: usize,
    axis: &str,
    angle: f64,
) -> Result<()> {
    match axis.to_uppercase().as_str() {
        "X" => gates::gate_rx(state, target, angle),
        "Y" => gates::gate_ry(state, target, angle),
        "Z" => gates::gate_rz(state, target, angle),
        other => Err(Error::InvalidNoiseConfig {
            reason: format!("Invalid COHERENT_OVER_ROTATION axis: {}", other),
        }),
    }
}

/// Crosstalk channel: with probability p, apply Z to each coupled qubit.
pub fn crosstalk_noise<R: Rng + ?Sized>(
    state: &mut QuantumState,
    coupled_qubits: &[usize],
    prob: f64,
    rng: &mut R,
) -> Result<()> {
    if !(0.0..=1.0).contains(&prob) {
        return Err(Error::InvalidNoiseConfig {
            reason: format!("Probability must be in [0, 1], got {}", prob),
        });
    }

    for &q in coupled_qubits {
        if rng.gen::<f64>() < prob {
            gates::gate_z(state, q)?;
        }
    }

    Ok(())
}

/// Single-qubit Kraus noise channel.
pub fn kraus_noise<R: Rng + ?Sized>(
    state: &mut QuantumState,
    target: usize,
    operators: &[[[[f64; 2]; 2]; 2]],
    rng: &mut R,
) -> Result<()> {
    if operators.is_empty() {
        return Err(Error::InvalidNoiseConfig {
            reason: "KRAUS requires at least one operator".to_string(),
        });
    }

    let matrices: Vec<[[Complex64; 2]; 2]> = operators
        .iter()
        .map(|op| {
            [
                [
                    Complex64::new(op[0][0][0], op[0][0][1]),
                    Complex64::new(op[0][1][0], op[0][1][1]),
                ],
                [
                    Complex64::new(op[1][0][0], op[1][0][1]),
                    Complex64::new(op[1][1][0], op[1][1][1]),
                ],
            ]
        })
        .collect();

    let probs: Vec<f64> = matrices
        .iter()
        .map(|k| kraus_probability(state, target, k))
        .collect();

    let total: f64 = probs.iter().sum();
    if total <= 1e-15 {
        return Err(Error::InvalidNoiseConfig {
            reason: "KRAUS operators produced zero probability mass".to_string(),
        });
    }

    let mut draw = rng.gen::<f64>() * total;
    let mut idx = 0usize;
    for (i, p) in probs.iter().enumerate() {
        if draw <= *p {
            idx = i;
            break;
        }
        draw -= *p;
        idx = i;
    }

    apply_single_qubit_linear_operator(state, target, &matrices[idx]);
    state.normalize();
    Ok(())
}

/// Extract effective readout-error probability from a noise config.
pub fn readout_error_probability(noise: &NoiseConfig) -> Option<f64> {
    match noise.noise_type.to_uppercase().as_str() {
        "READOUT_ERROR" => Some(noise.probability),
        "COMPOSITE" => {
            let channels = noise.channels.as_ref()?;
            let mut p_not = 1.0;
            let mut found = false;
            for channel in channels {
                if let Some(p) = readout_error_probability(channel) {
                    p_not *= 1.0 - p;
                    found = true;
                }
            }
            if found { Some(1.0 - p_not) } else { None }
        }
        _ => None,
    }
}

fn kraus_probability(state: &QuantumState, target: usize, k: &[[Complex64; 2]; 2]) -> f64 {
    let target_mask = 1 << target;
    let mut prob = 0.0;

    for i in 0..state.size() {
        if (i & target_mask) == 0 {
            let j = i | target_mask;
            let a0 = state.amplitudes()[i];
            let a1 = state.amplitudes()[j];
            let b0 = k[0][0] * a0 + k[0][1] * a1;
            let b1 = k[1][0] * a0 + k[1][1] * a1;
            prob += b0.norm_sqr() + b1.norm_sqr();
        }
    }

    prob
}

fn apply_single_qubit_linear_operator(
    state: &mut QuantumState,
    target: usize,
    op: &[[Complex64; 2]; 2],
) {
    let target_mask = 1 << target;
    let size = state.size();
    let amps = state.amplitudes_mut();

    for i in 0..size {
        if (i & target_mask) == 0 {
            let j = i | target_mask;
            let a0 = amps[i];
            let a1 = amps[j];
            amps[i] = op[0][0] * a0 + op[0][1] * a1;
            amps[j] = op[1][0] * a0 + op[1][1] * a1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{rngs::StdRng, SeedableRng};

    #[test]
    fn test_bit_flip_noise() {
        let mut state = QuantumState::new(1).unwrap();
        // Apply bit flip with 100% probability
        bit_flip_noise(&mut state, 0, 1.0).unwrap();
        // Should flip to |1⟩
        assert!(state.get_amplitude(1).unwrap().re.abs() > 0.99);
    }

    #[test]
    fn test_phase_flip_noise() {
        let mut state = QuantumState::new(1).unwrap();
        gates::gate_x(&mut state, 0).unwrap(); // Set to |1⟩
        phase_flip_noise(&mut state, 0, 1.0).unwrap();
        // |1⟩ -> -|1⟩, but |...| should still be 1
        assert!(state.probability(1).unwrap() > 0.99);
    }

    #[test]
    fn test_depolarizing_noise() {
        let mut state = QuantumState::new(1).unwrap();
        depolarizing_noise(&mut state, 0, 0.5).unwrap();
        let probs: f64 = state.probabilities().iter().sum();
        assert!((probs - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_invalid_probability() {
        let mut state = QuantumState::new(1).unwrap();
        assert!(bit_flip_noise(&mut state, 0, 1.5).is_err());
    }

    #[test]
    fn test_apply_noise_reproducible_with_seeded_rng() {
        let noise = NoiseConfig {
            noise_type: "depolarizing".to_string(),
            probability: 1.0,
            channels: None,
            t1: None,
            t2: None,
            dt: None,
            angle: None,
            axis: None,
            coupled_qubits: None,
            kraus: None,
        };

        let mut state1 = QuantumState::new(1).unwrap();
        let mut state2 = QuantumState::new(1).unwrap();

        let mut rng1 = StdRng::seed_from_u64(777);
        let mut rng2 = StdRng::seed_from_u64(777);

        apply_noise_with_rng(&mut state1, 0, &noise, &mut rng1).unwrap();
        apply_noise_with_rng(&mut state2, 0, &noise, &mut rng2).unwrap();

        assert_eq!(state1.probabilities(), state2.probabilities());
    }

    #[test]
    fn test_composite_noise_chain() {
        let noise = NoiseConfig {
            noise_type: "COMPOSITE".to_string(),
            probability: 1.0,
            channels: Some(vec![
                NoiseConfig {
                    noise_type: "BIT_FLIP".to_string(),
                    probability: 1.0,
                    channels: None,
                    t1: None,
                    t2: None,
                    dt: None,
                    angle: None,
                    axis: None,
                    coupled_qubits: None,
                    kraus: None,
                },
                NoiseConfig {
                    noise_type: "PHASE_FLIP".to_string(),
                    probability: 1.0,
                    channels: None,
                    t1: None,
                    t2: None,
                    dt: None,
                    angle: None,
                    axis: None,
                    coupled_qubits: None,
                    kraus: None,
                },
            ]),
            t1: None,
            t2: None,
            dt: None,
            angle: None,
            axis: None,
            coupled_qubits: None,
            kraus: None,
        };

        let mut state = QuantumState::new(1).unwrap();
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        apply_noise_with_rng(&mut state, 0, &noise, &mut rng).unwrap();
        assert!(state.probability(1).unwrap() > 0.99);
    }

    #[test]
    fn test_readout_probability_extraction() {
        let noise = NoiseConfig {
            noise_type: "READOUT_ERROR".to_string(),
            probability: 0.1,
            channels: None,
            t1: None,
            t2: None,
            dt: None,
            angle: None,
            axis: None,
            coupled_qubits: None,
            kraus: None,
        };
        assert_eq!(readout_error_probability(&noise), Some(0.1));
    }
}
