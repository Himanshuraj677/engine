//! Quantum measurement and sampling operations

use std::collections::BTreeMap;
use rand::distributions::WeightedIndex;
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use crate::state::QuantumState;
use crate::error::{Error, Result};

/// Measurement result statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeasurementResult {
    /// Exact probability distribution
    pub probabilities: BTreeMap<String, f64>,
    /// Shot counts from sampling
    pub counts: BTreeMap<String, usize>,
    /// Total number of shots
    pub shots: usize,
}

impl MeasurementResult {
    /// Create measurement result from probabilities
    pub fn from_probabilities(probabilities: Vec<f64>, num_qubits: usize) -> Result<Self> {
        let mut probs = BTreeMap::new();
        let mut total_prob = 0.0;

        for (idx, &prob) in probabilities.iter().enumerate() {
            if prob > 1e-15 {
                // Only include non-zero probabilities
                let bitstring = format_bitstring(idx, num_qubits);
                probs.insert(bitstring, prob);
                total_prob += prob;
            }
        }

        // Verify normalization
        if (total_prob - 1.0).abs() > 1e-6 {
            return Err(Error::MeasurementError {
                reason: format!("Probabilities do not sum to 1: {}", total_prob),
            });
        }

        Ok(MeasurementResult {
            probabilities: probs,
            counts: BTreeMap::new(),
            shots: 0,
        })
    }

    /// Sample from the state using Monte Carlo method (importance sampling)
    pub fn sample(state: &QuantumState, shots: usize) -> Result<Self> {
        let probabilities = state.probabilities();

        // Filter out near-zero probabilities for numerical stability
        let nonzero_probs: Vec<f64> = probabilities
            .iter()
            .map(|&p| if p < 1e-15 { 0.0 } else { p })
            .collect();

        // Create weighted distribution
        let dist = WeightedIndex::new(&nonzero_probs)
            .map_err(|e| Error::MeasurementError {
                reason: format!("Failed to create weighted distribution: {}", e),
            })?;

        let mut rng = thread_rng();
        let mut counts = BTreeMap::new();

        // Perform shots
        for _ in 0..shots {
            let idx = dist.sample(&mut rng);
            let bitstring = format_bitstring(idx, state.num_qubits());
            *counts.entry(bitstring).or_insert(0) += 1;
        }

        // Calculate empirical probabilities
        let mut probabilities = BTreeMap::new();
        for (bitstring, count) in &counts {
            let prob = *count as f64 / shots as f64;
            probabilities.insert(bitstring.clone(), prob);
        }

        Ok(MeasurementResult {
            probabilities,
            counts,
            shots,
        })
    }

    /// Measure a single qubit, returning its value and collapsed state index
    /// This is used for mid-circuit measurement
    pub fn measure_single(state: &mut QuantumState, target: usize) -> Result<(bool, usize)> {
        let marginal = state.marginal_distribution(target)?;
        let _prob_0 = marginal[0];
        let prob_1 = marginal[1];

        let mut rng = thread_rng();
        let result = rng.gen::<f64>() < prob_1;

        // Project to measured state
        let mask = 1 << target;
        let mut collapsed_amplitudes = vec![num_complex::Complex64::new(0.0, 0.0); state.size()];

        if result {
            // Measured 1
            for (idx, &amp) in state.amplitudes().iter().enumerate() {
                if (idx & mask) != 0 {
                    collapsed_amplitudes[idx] = amp;
                }
            }
        } else {
            // Measured 0
            for (idx, &amp) in state.amplitudes().iter().enumerate() {
                if (idx & mask) == 0 {
                    collapsed_amplitudes[idx] = amp;
                }
            }
        }

        // Normalize
        let norm: f64 = collapsed_amplitudes
            .iter()
            .map(|a| a.norm_sqr())
            .sum::<f64>()
            .sqrt();

        if norm > 1e-15 {
            for amp in &mut collapsed_amplitudes {
                *amp /= norm;
            }
        }

        *state.amplitudes_mut() = collapsed_amplitudes;

        Ok((result, if result { 1 } else { 0 }))
    }

    /// Get the most likely state (bitstring)
    pub fn most_likely_state(&self) -> Option<(String, f64)> {
        self.probabilities
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(k, v)| (k.clone(), *v))
    }

    /// Get expected value for Pauli operator
    /// For now, simplified for Z measurement
    pub fn expected_value_z(&self, qubit: usize) -> f64 {
        let mut value = 0.0;
        for (bitstring, prob) in &self.probabilities {
            let bit = bitstring.chars().rev().nth(qubit).unwrap() == '1';
            value += if bit { *prob } else { -*prob };
        }
        value
    }

    /// Convert to JSON format
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

/// Format an index as a binary bitstring (big-endian)
fn format_bitstring(mut idx: usize, num_qubits: usize) -> String {
    let mut result = String::with_capacity(num_qubits);
    for _ in 0..num_qubits {
        result.push(if idx & 1 == 1 { '1' } else { '0' });
        idx >>= 1;
    }
    result.chars().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitstring_format() {
        assert_eq!(format_bitstring(0, 2), "00");
        assert_eq!(format_bitstring(1, 2), "01");
        assert_eq!(format_bitstring(2, 2), "10");
        assert_eq!(format_bitstring(3, 2), "11");
    }

    #[test]
    fn test_measurement_from_probabilities() {
        let probs = vec![0.5, 0.0, 0.0, 0.5]; // Bell state
        let measurement = MeasurementResult::from_probabilities(probs, 2).unwrap();
        assert_eq!(measurement.probabilities.len(), 2);
    }

    #[test]
    fn test_sampling() {
        let state = crate::state::QuantumState::new(1).unwrap();
        let result = MeasurementResult::sample(&state, 1000).unwrap();
        assert_eq!(result.shots, 1000);
        // First qubit should be in state |0⟩
        assert!(result.counts.contains_key("0"));
        assert!(result.counts["0"] > 900); // Should be mostly 0
    }
}
