//! Circuit representation, parsing, and validation

use serde::{Deserialize, Serialize};
use crate::error::{Error, Result};

/// A quantum gate instruction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateInstruction {
    pub gate_type: String,
    pub target: Option<usize>,
    pub control: Option<usize>,
    pub parameter: Option<f64>,
    #[serde(default)]
    pub noise: Option<NoiseConfig>,
}

/// Noise configuration for a gate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoiseConfig {
    pub noise_type: String,
    pub probability: f64,
}

/// A quantum circuit definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Circuit {
    pub qubits: usize,
    pub gates: Vec<GateInstruction>,
    #[serde(default)]
    pub global_noise: Option<NoiseConfig>,
}

impl Circuit {
    /// Parse a circuit from JSON
    pub fn from_json(json: &str) -> Result<Self> {
        let circuit: Circuit = serde_json::from_str(json)?;
        circuit.validate()?;
        Ok(circuit)
    }

    /// Convert circuit to JSON
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Validate circuit correctness
    pub fn validate(&self) -> Result<()> {
        if self.qubits == 0 || self.qubits > 30 {
            return Err(Error::CircuitValidationError {
                reason: format!(
                    "Invalid qubit count: {}. Must be between 1 and 30.",
                    self.qubits
                ),
            });
        }

        for (idx, gate) in self.gates.iter().enumerate() {
            self.validate_gate(gate, idx)?;
        }

        Ok(())
    }

    fn validate_gate(&self, gate: &GateInstruction, idx: usize) -> Result<()> {
        let gate_type = gate.gate_type.to_uppercase();

        match gate_type.as_str() {
            // Single-qubit gates
            "X" | "Y" | "Z" | "H" | "S" | "T" => {
                if gate.target.is_none() {
                    return Err(Error::CircuitValidationError {
                        reason: format!("Gate {} at position {} requires 'target'", gate_type, idx),
                    });
                }
                let target = gate.target.unwrap();
                self.validate_qubit(target)?;
            }

            // Single-qubit parameterized gates
            "RX" | "RY" | "RZ" => {
                if gate.target.is_none() {
                    return Err(Error::CircuitValidationError {
                        reason: format!(
                            "Gate {} at position {} requires 'target'",
                            gate_type, idx
                        ),
                    });
                }
                if gate.parameter.is_none() {
                    return Err(Error::CircuitValidationError {
                        reason: format!(
                            "Gate {} at position {} requires 'parameter'",
                            gate_type, idx
                        ),
                    });
                }
                let target = gate.target.unwrap();
                self.validate_qubit(target)?;
            }

            // Two-qubit gates
            "CNOT" | "CX" => {
                if gate.control.is_none() || gate.target.is_none() {
                    return Err(Error::CircuitValidationError {
                        reason: format!(
                            "Gate {} at position {} requires 'control' and 'target'",
                            gate_type, idx
                        ),
                    });
                }
                let control = gate.control.unwrap();
                let target = gate.target.unwrap();
                self.validate_qubit(control)?;
                self.validate_qubit(target)?;
                if control == target {
                    return Err(Error::CircuitValidationError {
                        reason: format!(
                            "Gate {} at position {}: control and target must be different",
                            gate_type, idx
                        ),
                    });
                }
            }

            "SWAP" => {
                if gate.control.is_none() || gate.target.is_none() {
                    return Err(Error::CircuitValidationError {
                        reason: format!(
                            "Gate {} at position {} requires 'control' and 'target'",
                            gate_type, idx
                        ),
                    });
                }
                let q1 = gate.control.unwrap();
                let q2 = gate.target.unwrap();
                self.validate_qubit(q1)?;
                self.validate_qubit(q2)?;
                if q1 == q2 {
                    return Err(Error::CircuitValidationError {
                        reason: format!(
                            "Gate {} at position {}: qubits must be different",
                            gate_type, idx
                        ),
                    });
                }
            }

            "MEASURE" => {
                if gate.target.is_none() {
                    return Err(Error::CircuitValidationError {
                        reason: format!(
                            "Gate {} at position {} requires 'target'",
                            gate_type, idx
                        ),
                    });
                }
                let target = gate.target.unwrap();
                self.validate_qubit(target)?;
            }

            _ => {
                return Err(Error::CircuitValidationError {
                    reason: format!("Unknown gate type: {}", gate_type),
                });
            }
        }

        // Validate noise config if present
        if let Some(noise) = &gate.noise {
            self.validate_noise(noise)?;
        }

        // Validate global noise if present
        if let Some(noise) = &self.global_noise {
            self.validate_noise(noise)?;
        }

        Ok(())
    }

    fn validate_qubit(&self, qubit: usize) -> Result<()> {
        if qubit >= self.qubits {
            return Err(Error::InvalidQubitIndex {
                index: qubit,
                max: self.qubits,
            });
        }
        Ok(())
    }

    fn validate_noise(&self, noise: &NoiseConfig) -> Result<()> {
        let noise_type = noise.noise_type.to_uppercase();

        if !matches!(
            noise_type.as_str(),
            "BIT_FLIP" | "PHASE_FLIP" | "DEPOLARIZING"
        ) {
            return Err(Error::InvalidNoiseConfig {
                reason: format!("Unknown noise type: {}", noise_type),
            });
        }

        if !(0.0..=1.0).contains(&noise.probability) {
            return Err(Error::InvalidNoiseConfig {
                reason: format!(
                    "Noise probability must be in [0, 1], got {}",
                    noise.probability
                ),
            });
        }

        Ok(())
    }

    /// Get circuit description
    pub fn description(&self) -> String {
        format!(
            "Circuit: {} qubits, {} gates",
            self.qubits,
            self.gates.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_validation() {
        let json = r#"{
            "qubits": 2,
            "gates": [
                {"gate_type": "H", "target": 0},
                {"gate_type": "CNOT", "control": 0, "target": 1}
            ]
        }"#;

        let circuit = Circuit::from_json(json).unwrap();
        assert_eq!(circuit.qubits, 2);
        assert_eq!(circuit.gates.len(), 2);
    }

    #[test]
    fn test_invalid_gate() {
        let json = r#"{
            "qubits": 2,
            "gates": [
                {"gate_type": "CNOT", "target": 0}
            ]
        }"#;

        assert!(Circuit::from_json(json).is_err());
    }

    #[test]
    fn test_invalid_qubit_index() {
        let json = r#"{
            "qubits": 2,
            "gates": [
                {"gate_type": "H", "target": 5}
            ]
        }"#;

        assert!(Circuit::from_json(json).is_err());
    }
}
