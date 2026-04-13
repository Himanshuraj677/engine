//! Noise models for realistic quantum simulation

use rand::Rng;
use crate::state::QuantumState;
use crate::error::{Error, Result};
use crate::circuit::NoiseConfig;
use crate::gates;

/// Bit flip noise: |0⟩ → |1⟩ or |1⟩ → |0⟩ with probability p
pub fn bit_flip_noise(state: &mut QuantumState, target: usize, prob: f64) -> Result<()> {
    if !(0.0..=1.0).contains(&prob) {
        return Err(Error::InvalidNoiseConfig {
            reason: format!("Probability must be in [0, 1], got {}", prob),
        });
    }

    let mut rng = rand::thread_rng();
    if rng.gen::<f64>() < prob {
        gates::gate_x(state, target)?;
    }

    Ok(())
}

/// Phase flip noise: |0⟩ → |0⟩ or |1⟩ → -|1⟩ with probability p
pub fn phase_flip_noise(state: &mut QuantumState, target: usize, prob: f64) -> Result<()> {
    if !(0.0..=1.0).contains(&prob) {
        return Err(Error::InvalidNoiseConfig {
            reason: format!("Probability must be in [0, 1], got {}", prob),
        });
    }

    let mut rng = rand::thread_rng();
    if rng.gen::<f64>() < prob {
        gates::gate_z(state, target)?;
    }

    Ok(())
}

/// Depolarizing noise: with probability p, replace with mixed state
/// ρ → (1-p)ρ + p(I/2) for single qubit
pub fn depolarizing_noise(state: &mut QuantumState, target: usize, prob: f64) -> Result<()> {
    if !(0.0..=1.0).contains(&prob) {
        return Err(Error::InvalidNoiseConfig {
            reason: format!("Probability must be in [0, 1], got {}", prob),
        });
    }

    let mut rng = rand::thread_rng();
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
    let noise_type = noise.noise_type.to_uppercase();

    match noise_type.as_str() {
        "BIT_FLIP" => bit_flip_noise(state, target, noise.probability),
        "PHASE_FLIP" => phase_flip_noise(state, target, noise.probability),
        "DEPOLARIZING" => depolarizing_noise(state, target, noise.probability),
        _ => Err(Error::InvalidNoiseConfig {
            reason: format!("Unknown noise type: {}", noise_type),
        }),
    }
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
