//! Quantum state vector representation and operations
//!
//! Uses Vec<Complex<f64>> for state representation.
//! State size is 2^n for n qubits.

use num_complex::Complex64;
use crate::error::{Error, Result};

/// Quantum state vector representation
/// Size is 2^n for n qubits, initialized to |0...0⟩
#[derive(Clone, Debug)]
pub struct QuantumState {
    /// State amplitudes: [α₀, α₁, ..., α_{2^n-1}]
    amplitudes: Vec<Complex64>,
    /// Number of qubits
    num_qubits: usize,
}

impl QuantumState {
    /// Create a new quantum state initialized to |0...0⟩
    pub fn new(num_qubits: usize) -> Result<Self> {
        if num_qubits == 0 {
            return Err(Error::CircuitValidationError {
                reason: "Number of qubits must be > 0".to_string(),
            });
        }

        if num_qubits > 30 {
            return Err(Error::CircuitValidationError {
                reason: "Number of qubits cannot exceed 30 (2^30 states)".to_string(),
            });
        }

        let size = 1 << num_qubits; // 2^num_qubits
        let mut amplitudes = vec![Complex64::new(0.0, 0.0); size];
        amplitudes[0] = Complex64::new(1.0, 0.0); // |0...0⟩

        Ok(QuantumState {
            amplitudes,
            num_qubits,
        })
    }

    /// Get the number of qubits
    #[inline]
    pub fn num_qubits(&self) -> usize {
        self.num_qubits
    }

    /// Get the state amplitude at basis state index
    #[inline]
    pub fn get_amplitude(&self, index: usize) -> Result<Complex64> {
        self.amplitudes.get(index)
            .copied()
            .ok_or_else(|| Error::IndexOutOfBounds(format!("Index {} out of bounds", index)))
    }

    /// Get mutable access to amplitudes (for gate operations)
    #[inline]
    pub fn amplitudes_mut(&mut self) -> &mut Vec<Complex64> {
        &mut self.amplitudes
    }

    /// Get immutable access to amplitudes
    #[inline]
    pub fn amplitudes(&self) -> &Vec<Complex64> {
        &self.amplitudes
    }

    /// Get the probability of a basis state
    #[inline]
    pub fn probability(&self, index: usize) -> Result<f64> {
        let amp = self.get_amplitude(index)?;
        Ok((amp.norm_sqr()) as f64)
    }

    /// Get all probabilities (for measurement)
    pub fn probabilities(&self) -> Vec<f64> {
        self.amplitudes.iter()
            .map(|amp| (amp.norm_sqr()) as f64)
            .collect()
    }

    /// Normalize state (sum of |α|² = 1)
    pub fn normalize(&mut self) {
        let norm: f64 = self.amplitudes.iter()
            .map(|amp| amp.norm_sqr())
            .sum::<f64>()
            .sqrt();

        if norm > 1e-15 {
            let factor = Complex64::new(1.0 / norm, 0.0);
            self.amplitudes.iter_mut().for_each(|amp| *amp *= factor);
        }
    }

    /// Reset state to |0...0⟩
    pub fn reset(&mut self) {
        self.amplitudes.iter_mut().for_each(|amp| *amp = Complex64::new(0.0, 0.0));
        self.amplitudes[0] = Complex64::new(1.0, 0.0);
    }

    /// Get state vector size (2^n)
    #[inline]
    pub fn size(&self) -> usize {
        self.amplitudes.len()
    }

    /// Collapse to a basis state
    /// Sets all amplitudes to 0 except the specified state, then normalizes
    pub fn collapse_to(&mut self, state_index: usize) -> Result<()> {
        if state_index >= self.size() {
            return Err(Error::IndexOutOfBounds(format!(
                "State index {} out of bounds",
                state_index
            )));
        }

        self.amplitudes.iter_mut().for_each(|amp| *amp = Complex64::new(0.0, 0.0));
        self.amplitudes[state_index] = Complex64::new(1.0, 0.0);
        Ok(())
    }

    /// Get partial trace for subsystem (for debugging)
    /// Returns reduced density matrix diagonal (probabilities) for target qubit
    pub fn marginal_distribution(&self, target_qubit: usize) -> Result<Vec<f64>> {
        if target_qubit >= self.num_qubits {
            return Err(Error::InvalidQubitIndex {
                index: target_qubit,
                max: self.num_qubits,
            });
        }

        let mut probs = vec![0.0; 2];
        let mask = 1 << target_qubit;

        for (idx, amp) in self.amplitudes.iter().enumerate() {
            let bit = if (idx & mask) != 0 { 1 } else { 0 };
            probs[bit] += amp.norm_sqr();
        }

        Ok(probs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_initialization() {
        let state = QuantumState::new(2).unwrap();
        assert_eq!(state.num_qubits(), 2);
        assert_eq!(state.size(), 4);
        assert_eq!(state.get_amplitude(0).unwrap().re, 1.0);
    }

    #[test]
    fn test_probability_normalization() {
        let state = QuantumState::new(1).unwrap();
        let probs: f64 = state.probabilities().iter().sum();
        assert!((probs - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_collapse() {
        let mut state = QuantumState::new(2).unwrap();
        state.collapse_to(3).unwrap();
        assert_eq!(state.get_amplitude(3).unwrap().re, 1.0);
    }

    #[test]
    fn test_invalid_qubit_count() {
        assert!(QuantumState::new(0).is_err());
        assert!(QuantumState::new(31).is_err());
    }
}
