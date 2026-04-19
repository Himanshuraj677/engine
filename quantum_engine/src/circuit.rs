//! Circuit representation, parsing, and validation

use num_complex::Complex64;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// Complex value represented as [real, imag] in JSON.
pub type ComplexJson = [f64; 2];

/// Single-qubit matrix payload for UNITARY gate.
pub type Unitary2x2Json = [[ComplexJson; 2]; 2];

/// Generic single-qubit matrix payload used by noise channels (e.g., Kraus operators).
pub type Matrix2x2Json = [[ComplexJson; 2]; 2];

/// Classical condition for conditional gate execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassicalCondition {
    /// Classical register index to inspect.
    pub register: usize,
    /// Required boolean value for execution.
    pub value: bool,
}

/// A quantum gate instruction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateInstruction {
    pub gate_type: String,
    pub target: Option<usize>,
    pub control: Option<usize>,
    #[serde(default)]
    pub controls: Option<Vec<usize>>,
    pub parameter: Option<f64>,
    #[serde(default)]
    pub matrix: Option<Unitary2x2Json>,
    #[serde(default)]
    pub condition: Option<ClassicalCondition>,
    #[serde(default)]
    pub cbit: Option<usize>,
    #[serde(default)]
    pub repeat: Option<usize>,
    #[serde(default)]
    pub noise: Option<NoiseConfig>,
}

impl GateInstruction {
    /// Resolve all control qubits while preserving backward compatibility with `control`.
    pub fn resolved_controls(&self) -> Vec<usize> {
        let mut controls = Vec::new();

        if let Some(control) = self.control {
            controls.push(control);
        }

        if let Some(extra_controls) = &self.controls {
            for &control in extra_controls {
                if !controls.contains(&control) {
                    controls.push(control);
                }
            }
        }

        controls
    }

    /// Return all qubits touched by this gate.
    pub fn involved_qubits(&self) -> Vec<usize> {
        let mut qubits = self.resolved_controls();

        if let Some(target) = self.target {
            if !qubits.contains(&target) {
                qubits.push(target);
            }
        }

        qubits
    }
}

/// Noise configuration for a gate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoiseConfig {
    pub noise_type: String,
    pub probability: f64,
    #[serde(default)]
    pub channels: Option<Vec<NoiseConfig>>,
    #[serde(default)]
    pub t1: Option<f64>,
    #[serde(default)]
    pub t2: Option<f64>,
    #[serde(default)]
    pub dt: Option<f64>,
    #[serde(default)]
    pub angle: Option<f64>,
    #[serde(default)]
    pub axis: Option<String>,
    #[serde(default)]
    pub coupled_qubits: Option<Vec<usize>>,
    #[serde(default)]
    pub kraus: Option<Vec<Matrix2x2Json>>,
}

/// A quantum circuit definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Circuit {
    pub qubits: usize,
    #[serde(default)]
    pub classical_bits: Option<usize>,
    pub gates: Vec<GateInstruction>,
    #[serde(default)]
    pub global_noise: Option<NoiseConfig>,
    #[serde(default)]
    pub readout_noise: Option<NoiseConfig>,
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

        if let Some(noise) = &self.readout_noise {
            self.validate_noise(noise)?;
            if !Self::contains_readout_error(noise) {
                return Err(Error::InvalidNoiseConfig {
                    reason: "readout_noise must be READOUT_ERROR or COMPOSITE containing READOUT_ERROR"
                        .to_string(),
                });
            }
        }

        if let Some(classical_bits) = self.classical_bits {
            if classical_bits == 0 {
                return Err(Error::CircuitValidationError {
                    reason: "classical_bits must be > 0 when provided".to_string(),
                });
            }
        }

        for (idx, gate) in self.gates.iter().enumerate() {
            self.validate_gate(gate, idx)?;
        }

        Ok(())
    }

    fn validate_gate(&self, gate: &GateInstruction, idx: usize) -> Result<()> {
        let gate_type = gate.gate_type.to_uppercase();
        let controls = gate.resolved_controls();

        if let Some(repeat) = gate.repeat {
            if repeat == 0 {
                return Err(Error::CircuitValidationError {
                    reason: format!("Gate {} at position {} has invalid repeat=0", gate_type, idx),
                });
            }
        }

        self.validate_condition(gate, &gate_type, idx)?;

        match gate_type.as_str() {
            // Single-qubit gates
            "X" | "Y" | "Z" | "H" | "S" | "T" => {
                let target = self.validate_target_required(gate, &gate_type, idx)?;
                self.validate_controls(&controls, target, &gate_type, idx)?;
            }

            // Single-qubit parameterized gates
            "RX" | "RY" | "RZ" => {
                if gate.parameter.is_none() {
                    return Err(Error::CircuitValidationError {
                        reason: format!(
                            "Gate {} at position {} requires 'parameter'",
                            gate_type, idx
                        ),
                    });
                }
                let target = self.validate_target_required(gate, &gate_type, idx)?;
                self.validate_controls(&controls, target, &gate_type, idx)?;
            }

            // Controlled gates
            "CNOT" | "CX" | "CZ" => {
                let target = self.validate_target_required(gate, &gate_type, idx)?;
                self.validate_controls_count(&controls, 1, &gate_type, idx)?;
                self.validate_controls(&controls, target, &gate_type, idx)?;
            }

            "CP" => {
                if gate.parameter.is_none() {
                    return Err(Error::CircuitValidationError {
                        reason: format!(
                            "Gate {} at position {} requires 'parameter'",
                            gate_type, idx
                        ),
                    });
                }
                let target = self.validate_target_required(gate, &gate_type, idx)?;
                self.validate_controls_count(&controls, 1, &gate_type, idx)?;
                self.validate_controls(&controls, target, &gate_type, idx)?;
            }

            "CCX" => {
                let target = self.validate_target_required(gate, &gate_type, idx)?;
                self.validate_controls_count(&controls, 2, &gate_type, idx)?;
                self.validate_controls(&controls, target, &gate_type, idx)?;
            }

            "SWAP" => {
                if gate.controls.as_ref().is_some_and(|v| !v.is_empty()) {
                    return Err(Error::CircuitValidationError {
                        reason: format!(
                            "Gate {} at position {} does not support 'controls' list",
                            gate_type, idx
                        ),
                    });
                }

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

            "MEASURE" | "RESET" => {
                if !controls.is_empty() {
                    return Err(Error::CircuitValidationError {
                        reason: format!(
                            "Gate {} at position {} does not support controls",
                            gate_type, idx
                        ),
                    });
                }
                let target = self.validate_target_required(gate, &gate_type, idx)?;
                self.validate_qubit(target)?;

                if gate_type == "MEASURE" {
                    self.validate_cbit(gate.cbit.unwrap_or(target), &gate_type, idx)?;
                }
            }

            "BARRIER" => {
                for &qubit in &gate.involved_qubits() {
                    self.validate_qubit(qubit)?;
                }
            }

            "U" | "UNITARY" => {
                let target = self.validate_target_required(gate, &gate_type, idx)?;
                self.validate_controls(&controls, target, &gate_type, idx)?;

                let matrix = gate.matrix.ok_or_else(|| Error::CircuitValidationError {
                    reason: format!(
                        "Gate {} at position {} requires 'matrix' as 2x2 complex entries",
                        gate_type, idx
                    ),
                })?;

                self.validate_unitary_matrix(&matrix, &gate_type, idx)?;
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

    fn validate_target_required(
        &self,
        gate: &GateInstruction,
        gate_type: &str,
        idx: usize,
    ) -> Result<usize> {
        if gate.target.is_none() {
            return Err(Error::CircuitValidationError {
                reason: format!("Gate {} at position {} requires 'target'", gate_type, idx),
            });
        }

        let target = gate.target.unwrap();
        self.validate_qubit(target)?;
        Ok(target)
    }

    fn validate_controls(
        &self,
        controls: &[usize],
        target: usize,
        gate_type: &str,
        idx: usize,
    ) -> Result<()> {
        for &control in controls {
            self.validate_qubit(control)?;

            if control == target {
                return Err(Error::CircuitValidationError {
                    reason: format!(
                        "Gate {} at position {}: control and target must be different",
                        gate_type, idx
                    ),
                });
            }
        }

        if let Some(extra_controls) = &self.gates[idx].controls {
            if extra_controls.len()
                != extra_controls
                    .iter()
                    .copied()
                    .collect::<std::collections::BTreeSet<_>>()
                    .len()
            {
                return Err(Error::CircuitValidationError {
                    reason: format!("Gate {} at position {} has duplicate controls", gate_type, idx),
                });
            }
        }

        Ok(())
    }

    fn validate_controls_count(
        &self,
        controls: &[usize],
        expected: usize,
        gate_type: &str,
        idx: usize,
    ) -> Result<()> {
        if controls.len() != expected {
            return Err(Error::CircuitValidationError {
                reason: format!(
                    "Gate {} at position {} requires exactly {} control qubit(s), got {}",
                    gate_type,
                    idx,
                    expected,
                    controls.len()
                ),
            });
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

        if !(0.0..=1.0).contains(&noise.probability) {
            return Err(Error::InvalidNoiseConfig {
                reason: format!(
                    "Noise probability must be in [0, 1], got {}",
                    noise.probability
                ),
            });
        }

        match noise_type.as_str() {
            "BIT_FLIP" | "PHASE_FLIP" | "DEPOLARIZING" | "AMPLITUDE_DAMPING" | "READOUT_ERROR" => {}

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

                if t1 <= 0.0 || t2 <= 0.0 || dt < 0.0 {
                    return Err(Error::InvalidNoiseConfig {
                        reason: "T1_T2_RELAXATION requires t1,t2 > 0 and dt >= 0".to_string(),
                    });
                }
            }

            "COHERENT_OVER_ROTATION" => {
                if noise.angle.is_none() {
                    return Err(Error::InvalidNoiseConfig {
                        reason: "COHERENT_OVER_ROTATION requires angle".to_string(),
                    });
                }
                if let Some(axis) = &noise.axis {
                    let axis = axis.to_uppercase();
                    if !matches!(axis.as_str(), "X" | "Y" | "Z") {
                        return Err(Error::InvalidNoiseConfig {
                            reason: format!("Invalid COHERENT_OVER_ROTATION axis: {}", axis),
                        });
                    }
                }
            }

            "CROSSTALK" => {
                let coupled = noise.coupled_qubits.as_ref().ok_or_else(|| Error::InvalidNoiseConfig {
                    reason: "CROSSTALK requires coupled_qubits".to_string(),
                })?;
                if coupled.is_empty() {
                    return Err(Error::InvalidNoiseConfig {
                        reason: "CROSSTALK coupled_qubits cannot be empty".to_string(),
                    });
                }
                for &q in coupled {
                    self.validate_qubit(q)?;
                }
            }

            "KRAUS" => {
                let ops = noise.kraus.as_ref().ok_or_else(|| Error::InvalidNoiseConfig {
                    reason: "KRAUS requires kraus operators".to_string(),
                })?;
                if ops.is_empty() {
                    return Err(Error::InvalidNoiseConfig {
                        reason: "KRAUS requires at least one operator".to_string(),
                    });
                }

                // Validate completeness sum K_i^dagger K_i = I
                let mut acc = [[Complex64::new(0.0, 0.0); 2]; 2];
                for op in ops {
                    let k = [
                        [
                            Complex64::new(op[0][0][0], op[0][0][1]),
                            Complex64::new(op[0][1][0], op[0][1][1]),
                        ],
                        [
                            Complex64::new(op[1][0][0], op[1][0][1]),
                            Complex64::new(op[1][1][0], op[1][1][1]),
                        ],
                    ];

                    let kd_k = [
                        [
                            k[0][0].conj() * k[0][0] + k[1][0].conj() * k[1][0],
                            k[0][0].conj() * k[0][1] + k[1][0].conj() * k[1][1],
                        ],
                        [
                            k[0][1].conj() * k[0][0] + k[1][1].conj() * k[1][0],
                            k[0][1].conj() * k[0][1] + k[1][1].conj() * k[1][1],
                        ],
                    ];
                    acc[0][0] += kd_k[0][0];
                    acc[0][1] += kd_k[0][1];
                    acc[1][0] += kd_k[1][0];
                    acc[1][1] += kd_k[1][1];
                }

                let eps = 1e-6;
                let valid =
                    (acc[0][0] - Complex64::new(1.0, 0.0)).norm() < eps
                        && (acc[1][1] - Complex64::new(1.0, 0.0)).norm() < eps
                        && acc[0][1].norm() < eps
                        && acc[1][0].norm() < eps;
                if !valid {
                    return Err(Error::InvalidNoiseConfig {
                        reason: "KRAUS operators violate completeness relation".to_string(),
                    });
                }
            }

            "COMPOSITE" => {
                let channels = noise.channels.as_ref().ok_or_else(|| Error::InvalidNoiseConfig {
                    reason: "COMPOSITE requires channels".to_string(),
                })?;
                if channels.is_empty() {
                    return Err(Error::InvalidNoiseConfig {
                        reason: "COMPOSITE channels cannot be empty".to_string(),
                    });
                }
                for channel in channels {
                    self.validate_noise(channel)?;
                }
            }

            _ => {
                return Err(Error::InvalidNoiseConfig {
                    reason: format!("Unknown noise type: {}", noise_type),
                });
            }
        }

        Ok(())
    }

    fn classical_bits_count(&self) -> usize {
        self.classical_bits.unwrap_or(self.qubits)
    }

    fn validate_cbit(&self, cbit: usize, gate_type: &str, idx: usize) -> Result<()> {
        if cbit >= self.classical_bits_count() {
            return Err(Error::CircuitValidationError {
                reason: format!(
                    "Gate {} at position {} references invalid classical bit {} (max {})",
                    gate_type,
                    idx,
                    cbit,
                    self.classical_bits_count().saturating_sub(1)
                ),
            });
        }
        Ok(())
    }

    fn validate_condition(&self, gate: &GateInstruction, gate_type: &str, idx: usize) -> Result<()> {
        if let Some(condition) = &gate.condition {
            self.validate_cbit(condition.register, gate_type, idx)?;
        }
        Ok(())
    }

    fn validate_unitary_matrix(
        &self,
        matrix: &Unitary2x2Json,
        gate_type: &str,
        idx: usize,
    ) -> Result<()> {
        let u = [
            [
                Complex64::new(matrix[0][0][0], matrix[0][0][1]),
                Complex64::new(matrix[0][1][0], matrix[0][1][1]),
            ],
            [
                Complex64::new(matrix[1][0][0], matrix[1][0][1]),
                Complex64::new(matrix[1][1][0], matrix[1][1][1]),
            ],
        ];

        let eps = 1e-8;
        let r00 = u[0][0] * u[0][0].conj() + u[0][1] * u[0][1].conj();
        let r11 = u[1][0] * u[1][0].conj() + u[1][1] * u[1][1].conj();
        let r01 = u[0][0] * u[1][0].conj() + u[0][1] * u[1][1].conj();
        let r10 = u[1][0] * u[0][0].conj() + u[1][1] * u[0][1].conj();

        let is_unitary =
            (r00 - Complex64::new(1.0, 0.0)).norm() < eps
                && (r11 - Complex64::new(1.0, 0.0)).norm() < eps
                && r01.norm() < eps
                && r10.norm() < eps;

        if !is_unitary {
            return Err(Error::CircuitValidationError {
                reason: format!("Gate {} at position {} has non-unitary matrix", gate_type, idx),
            });
        }

        Ok(())
    }

    fn contains_readout_error(noise: &NoiseConfig) -> bool {
        if noise.noise_type.to_uppercase() == "READOUT_ERROR" {
            return true;
        }
        if noise.noise_type.to_uppercase() == "COMPOSITE" {
            if let Some(channels) = &noise.channels {
                return channels.iter().any(Self::contains_readout_error);
            }
        }
        false
    }

    /// Get circuit description
    pub fn description(&self) -> String {
        format!("Circuit: {} qubits, {} gates", self.qubits, self.gates.len())
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

    #[test]
    fn test_ccx_with_controls_list() {
        let json = r#"{
            "qubits": 3,
            "gates": [
                {"gate_type": "CCX", "controls": [0, 1], "target": 2}
            ]
        }"#;

        assert!(Circuit::from_json(json).is_ok());
    }

    #[test]
    fn test_unitary_gate_validation() {
        let json = r#"{
            "qubits": 1,
            "gates": [
                {
                    "gate_type": "UNITARY",
                    "target": 0,
                    "matrix": [[[0.0, 0.0], [1.0, 0.0]], [[1.0, 0.0], [0.0, 0.0]]]
                }
            ]
        }"#;

        assert!(Circuit::from_json(json).is_ok());
    }

    #[test]
    fn test_non_unitary_gate_rejected() {
        let json = r#"{
            "qubits": 1,
            "gates": [
                {
                    "gate_type": "UNITARY",
                    "target": 0,
                    "matrix": [[[1.0, 0.0], [1.0, 0.0]], [[0.0, 0.0], [1.0, 0.0]]]
                }
            ]
        }"#;

        assert!(Circuit::from_json(json).is_err());
    }

    #[test]
    fn test_measure_with_classical_target_and_condition() {
        let json = r#"{
            "qubits": 2,
            "classical_bits": 4,
            "gates": [
                {"gate_type": "MEASURE", "target": 0, "cbit": 2},
                {"gate_type": "X", "target": 1, "condition": {"register": 2, "value": true}}
            ]
        }"#;

        assert!(Circuit::from_json(json).is_ok());
    }

    #[test]
    fn test_repeat_zero_rejected() {
        let json = r#"{
            "qubits": 1,
            "gates": [
                {"gate_type": "X", "target": 0, "repeat": 0}
            ]
        }"#;

        assert!(Circuit::from_json(json).is_err());
    }

    #[test]
    fn test_composite_noise_validation() {
        let json = r#"{
            "qubits": 1,
            "gates": [
                {
                    "gate_type": "H",
                    "target": 0,
                    "noise": {
                        "noise_type": "COMPOSITE",
                        "probability": 1.0,
                        "channels": [
                            {"noise_type": "BIT_FLIP", "probability": 0.1},
                            {"noise_type": "PHASE_FLIP", "probability": 0.2}
                        ]
                    }
                }
            ]
        }"#;

        assert!(Circuit::from_json(json).is_ok());
    }
}
