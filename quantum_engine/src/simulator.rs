//! Main simulation engine: orchestrates circuit execution

use crate::circuit::{Circuit, ClassicalCondition, GateInstruction};
use crate::state::QuantumState;
use crate::measurement::MeasurementResult;
use crate::error::{Error, Result};
use crate::{gates, noise, optimizer, runtime};
use num_complex::Complex64;
use rand::{rngs::StdRng, Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use log::debug;

/// Simulation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationConfig {
    /// Number of measurement shots
    pub shots: usize,
    /// Enable circuit optimization
    pub optimize: bool,
    /// Enable noise injection
    pub apply_noise: bool,
    /// Random seed (0 = random)
    pub seed: u64,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        SimulationConfig {
            shots: 1024,
            optimize: true,
            apply_noise: true,
            seed: 0,
        }
    }
}

/// Simulation result with statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    pub measurement: MeasurementResult,
    pub execution_time_ms: f64,
    pub circuit_depth: usize,
    pub circuit_gates: usize,
    pub two_qubit_gates: usize,
}

/// Main quantum simulator
pub struct Simulator {
    config: SimulationConfig,
}

impl Simulator {
    /// Create a new simulator
    pub fn new(config: SimulationConfig) -> Self {
        Simulator { config }
    }

    /// Run a circuit simulation
    pub fn run(&self, circuit: &Circuit) -> Result<SimulationResult> {
        let start = Instant::now();
        let mut rng = Self::build_rng(self.config.seed);

        // Optimize if requested
        let circuit = if self.config.optimize {
            debug!("Optimizing circuit...");
            optimizer::optimize_circuit(circuit)?
        } else {
            circuit.clone()
        };

        debug!("Circuit: {}", circuit.description());

        // Get metrics before simulation
        let circuit_depth = optimizer::circuit_depth(&circuit);
        let circuit_gates = circuit.gates.len();
        let two_qubit_gates = optimizer::count_two_qubit_gates(&circuit);

        // Initialize quantum state
        let mut state = QuantumState::new(circuit.qubits)?;
        let mut classical_state = vec![false; circuit.classical_bits.unwrap_or(circuit.qubits)];

        // Execute circuit
        self.execute_circuit(&circuit, &mut state, &mut classical_state, &mut rng)?;

        // Get measurement results
        let measurement = if self.config.shots > 0 {
            let readout_error_prob = if self.config.apply_noise {
                circuit
                    .readout_noise
                    .as_ref()
                    .and_then(noise::readout_error_probability)
            } else {
                None
            };

            MeasurementResult::sample_with_readout_error_with_rng(
                &state,
                self.config.shots,
                readout_error_prob,
                &mut rng,
            )?
        } else {
            // Just get probabilities without sampling
            let probs = state.probabilities();
            MeasurementResult::from_probabilities(probs, circuit.qubits)?
        };

        let execution_time_ms = start.elapsed().as_secs_f64() * 1000.0;

        Ok(SimulationResult {
            measurement,
            execution_time_ms,
            circuit_depth,
            circuit_gates,
            two_qubit_gates,
        })
    }

    fn build_rng(seed: u64) -> StdRng {
        if seed == 0 {
            StdRng::from_entropy()
        } else {
            StdRng::seed_from_u64(seed)
        }
    }

    /// Execute a circuit on a quantum state
    fn execute_circuit<R: Rng + ?Sized>(
        &self,
        circuit: &Circuit,
        state: &mut QuantumState,
        classical_state: &mut [bool],
        rng: &mut R,
    ) -> Result<()> {
        let plan = runtime::ExecutionPlan::from_circuit(circuit)?;
        debug!("Execution plan: {} layers", plan.num_layers());
        debug!("Parallelism factor: {:.2}", plan.parallelism_factor());

        for (layer_idx, layer) in plan.layers.iter().enumerate() {
            debug!("Executing layer {}: {} gates", layer_idx, layer.gates.len());

            for gate in &layer.gates {
                self.execute_gate(state, classical_state, gate, rng)?;

                // Apply noise if configured
                if self.config.apply_noise {
                    if let Some(target) = gate.target {
                        if let Some(noise_config) = &gate.noise {
                            noise::apply_noise_with_rng(state, target, noise_config, rng)?;
                        } else if let Some(noise_config) = &circuit.global_noise {
                            noise::apply_noise_with_rng(state, target, noise_config, rng)?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Execute a single gate
    fn execute_gate<R: Rng + ?Sized>(
        &self,
        state: &mut QuantumState,
        classical_state: &mut [bool],
        gate: &GateInstruction,
        rng: &mut R,
    ) -> Result<()> {
        if !Self::condition_matches(gate.condition.as_ref(), classical_state)? {
            return Ok(());
        }

        let repeat = gate.repeat.unwrap_or(1);
        for _ in 0..repeat {
            self.execute_gate_once(state, classical_state, gate, rng)?;
        }

        Ok(())
    }

    fn execute_gate_once<R: Rng + ?Sized>(
        &self,
        state: &mut QuantumState,
        classical_state: &mut [bool],
        gate: &GateInstruction,
        rng: &mut R,
    ) -> Result<()> {
        let gate_type = gate.gate_type.to_uppercase();
        let controls = gate.resolved_controls();

        match gate_type.as_str() {
            // Single-qubit gates
            "X" => {
                let target = Self::required_target(gate, "X")?;
                if controls.is_empty() {
                    gates::gate_x(state, target)?;
                } else {
                    gates::gate_controlled_unitary(state, &controls, target, Self::x_matrix())?;
                }
            }

            "Y" => {
                let target = Self::required_target(gate, "Y")?;
                if controls.is_empty() {
                    gates::gate_y(state, target)?;
                } else {
                    gates::gate_controlled_unitary(state, &controls, target, Self::y_matrix())?;
                }
            }

            "Z" => {
                let target = Self::required_target(gate, "Z")?;
                if controls.is_empty() {
                    gates::gate_z(state, target)?;
                } else {
                    gates::gate_controlled_unitary(state, &controls, target, Self::z_matrix())?;
                }
            }

            "H" => {
                let target = Self::required_target(gate, "H")?;
                if controls.is_empty() {
                    gates::gate_h(state, target)?;
                } else {
                    gates::gate_controlled_unitary(state, &controls, target, Self::h_matrix())?;
                }
            }

            "S" => {
                let target = Self::required_target(gate, "S")?;
                if controls.is_empty() {
                    gates::gate_s(state, target)?;
                } else {
                    gates::gate_controlled_unitary(state, &controls, target, Self::s_matrix())?;
                }
            }

            "T" => {
                let target = Self::required_target(gate, "T")?;
                if controls.is_empty() {
                    gates::gate_t(state, target)?;
                } else {
                    gates::gate_controlled_unitary(state, &controls, target, Self::t_matrix())?;
                }
            }

            // Parameterized gates
            "RX" => {
                let target = Self::required_target(gate, "RX")?;
                let param = Self::required_parameter(gate, "RX")?;
                if controls.is_empty() {
                    gates::gate_rx(state, target, param)?;
                } else {
                    gates::gate_controlled_unitary(state, &controls, target, Self::rx_matrix(param))?;
                }
            }

            "RY" => {
                let target = Self::required_target(gate, "RY")?;
                let param = Self::required_parameter(gate, "RY")?;
                if controls.is_empty() {
                    gates::gate_ry(state, target, param)?;
                } else {
                    gates::gate_controlled_unitary(state, &controls, target, Self::ry_matrix(param))?;
                }
            }

            "RZ" => {
                let target = Self::required_target(gate, "RZ")?;
                let param = Self::required_parameter(gate, "RZ")?;
                if controls.is_empty() {
                    gates::gate_rz(state, target, param)?;
                } else {
                    gates::gate_controlled_unitary(state, &controls, target, Self::rz_matrix(param))?;
                }
            }

            // Two-qubit gates
            "CNOT" | "CX" => {
                let target = Self::required_target(gate, "CNOT")?;
                if controls.len() != 1 {
                    return Err(Error::SimulationError {
                        reason: "CNOT gate requires exactly one control".to_string(),
                    });
                }
                gates::gate_cnot(state, controls[0], target)?;
            }

            "CZ" => {
                let target = Self::required_target(gate, "CZ")?;
                if controls.len() != 1 {
                    return Err(Error::SimulationError {
                        reason: "CZ gate requires exactly one control".to_string(),
                    });
                }
                gates::gate_cz(state, controls[0], target)?;
            }

            "CP" => {
                let target = Self::required_target(gate, "CP")?;
                let parameter = Self::required_parameter(gate, "CP")?;
                if controls.len() != 1 {
                    return Err(Error::SimulationError {
                        reason: "CP gate requires exactly one control".to_string(),
                    });
                }
                gates::gate_cp(state, controls[0], target, parameter)?;
            }

            "CCX" => {
                let target = Self::required_target(gate, "CCX")?;
                if controls.len() != 2 {
                    return Err(Error::SimulationError {
                        reason: "CCX gate requires exactly two controls".to_string(),
                    });
                }
                gates::gate_ccx(state, controls[0], controls[1], target)?;
            }

            "SWAP" => {
                let q1 = gate.control.ok_or_else(|| {
                    Error::SimulationError {
                        reason: "SWAP gate requires two qubits".to_string(),
                    }
                })?;
                let q2 = gate.target.ok_or_else(|| {
                    Error::SimulationError {
                        reason: "SWAP gate requires two qubits".to_string(),
                    }
                })?;
                gates::gate_swap(state, q1, q2)?;
            }

            "MEASURE" => {
                // Mid-circuit measurement
                let target = gate.target.ok_or_else(|| {
                    Error::SimulationError {
                        reason: "MEASURE gate requires target".to_string(),
                    }
                })?;
                let (_result, bit) = crate::measurement::MeasurementResult::measure_single_with_rng(state, target, rng)?;
                let cbit = gate.cbit.unwrap_or(target);
                if cbit >= classical_state.len() {
                    return Err(Error::SimulationError {
                        reason: format!("MEASURE cbit {} out of bounds", cbit),
                    });
                }
                classical_state[cbit] = bit == 1;
                debug!("Mid-circuit measurement on qubit {}: {}", target, bit);
            }

            "RESET" => {
                let target = Self::required_target(gate, "RESET")?;
                let (measured_one, _) = crate::measurement::MeasurementResult::measure_single_with_rng(state, target, rng)?;
                if measured_one {
                    gates::gate_x(state, target)?;
                }
            }

            "BARRIER" => {
                // Explicit no-op. Included for interoperability and scheduling boundaries.
            }

            "U" | "UNITARY" => {
                let target = Self::required_target(gate, "UNITARY")?;
                let matrix = gate.matrix.ok_or_else(|| Error::SimulationError {
                    reason: "UNITARY gate requires matrix".to_string(),
                })?;

                let matrix = [
                    [
                        Complex64::new(matrix[0][0][0], matrix[0][0][1]),
                        Complex64::new(matrix[0][1][0], matrix[0][1][1]),
                    ],
                    [
                        Complex64::new(matrix[1][0][0], matrix[1][0][1]),
                        Complex64::new(matrix[1][1][0], matrix[1][1][1]),
                    ],
                ];

                if controls.is_empty() {
                    gates::gate_unitary(state, target, matrix)?;
                } else {
                    gates::gate_controlled_unitary(state, &controls, target, matrix)?;
                }
            }

            _ => {
                return Err(Error::SimulationError {
                    reason: format!("Unknown gate: {}", gate_type),
                });
            }
        }

        Ok(())
    }

    fn condition_matches(condition: Option<&ClassicalCondition>, classical_state: &[bool]) -> Result<bool> {
        if let Some(condition) = condition {
            if condition.register >= classical_state.len() {
                return Err(Error::SimulationError {
                    reason: format!(
                        "Condition register {} out of bounds (max {})",
                        condition.register,
                        classical_state.len().saturating_sub(1)
                    ),
                });
            }
            return Ok(classical_state[condition.register] == condition.value);
        }

        Ok(true)
    }

    fn required_target(gate: &GateInstruction, gate_name: &str) -> Result<usize> {
        gate.target.ok_or_else(|| Error::SimulationError {
            reason: format!("{} gate requires target", gate_name),
        })
    }

    fn required_parameter(gate: &GateInstruction, gate_name: &str) -> Result<f64> {
        gate.parameter.ok_or_else(|| Error::SimulationError {
            reason: format!("{} gate requires parameter", gate_name),
        })
    }

    fn x_matrix() -> [[Complex64; 2]; 2] {
        [
            [Complex64::new(0.0, 0.0), Complex64::new(1.0, 0.0)],
            [Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0)],
        ]
    }

    fn y_matrix() -> [[Complex64; 2]; 2] {
        [
            [Complex64::new(0.0, 0.0), Complex64::new(0.0, -1.0)],
            [Complex64::new(0.0, 1.0), Complex64::new(0.0, 0.0)],
        ]
    }

    fn z_matrix() -> [[Complex64; 2]; 2] {
        [
            [Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0)],
            [Complex64::new(0.0, 0.0), Complex64::new(-1.0, 0.0)],
        ]
    }

    fn h_matrix() -> [[Complex64; 2]; 2] {
        let inv_sqrt2 = 1.0 / std::f64::consts::SQRT_2;
        [
            [Complex64::new(inv_sqrt2, 0.0), Complex64::new(inv_sqrt2, 0.0)],
            [Complex64::new(inv_sqrt2, 0.0), Complex64::new(-inv_sqrt2, 0.0)],
        ]
    }

    fn s_matrix() -> [[Complex64; 2]; 2] {
        [
            [Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0)],
            [Complex64::new(0.0, 0.0), Complex64::new(0.0, 1.0)],
        ]
    }

    fn t_matrix() -> [[Complex64; 2]; 2] {
        let phase = Complex64::new((std::f64::consts::PI / 4.0).cos(), (std::f64::consts::PI / 4.0).sin());
        [
            [Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0)],
            [Complex64::new(0.0, 0.0), phase],
        ]
    }

    fn rx_matrix(theta: f64) -> [[Complex64; 2]; 2] {
        let half_theta = theta / 2.0;
        let c = Complex64::new(half_theta.cos(), 0.0);
        let s = Complex64::new(0.0, -half_theta.sin());
        [[c, s], [s, c]]
    }

    fn ry_matrix(theta: f64) -> [[Complex64; 2]; 2] {
        let half_theta = theta / 2.0;
        let c = Complex64::new(half_theta.cos(), 0.0);
        let s = Complex64::new(half_theta.sin(), 0.0);
        [[c, -s], [s, c]]
    }

    fn rz_matrix(theta: f64) -> [[Complex64; 2]; 2] {
        let a = Complex64::new((-theta / 2.0).cos(), (-theta / 2.0).sin());
        let b = Complex64::new((theta / 2.0).cos(), (theta / 2.0).sin());
        [
            [a, Complex64::new(0.0, 0.0)],
            [Complex64::new(0.0, 0.0), b],
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bell_state_simulation() {
        let config = SimulationConfig {
            shots: 1000,
            optimize: true,
            apply_noise: false,
            seed: 0,
        };

        let simulator = Simulator::new(config);

        let circuit_json = r#"{
            "qubits": 2,
            "gates": [
                {"gate_type": "H", "target": 0},
                {"gate_type": "CNOT", "control": 0, "target": 1}
            ]
        }"#;

        let circuit = Circuit::from_json(circuit_json).unwrap();
        let result = simulator.run(&circuit).unwrap();

        // Bell state should have 50% |00⟩ and 50% |11⟩
        let p00 = result.measurement.probabilities.get("00").cloned().unwrap_or(0.0);
        let p11 = result.measurement.probabilities.get("11").cloned().unwrap_or(0.0);
        assert!(p00 > 0.3 && p00 < 0.7);
        assert!(p11 > 0.3 && p11 < 0.7);
    }

    #[test]
    fn test_seeded_simulation_is_deterministic_with_noise_and_sampling() {
        let config = SimulationConfig {
            shots: 512,
            optimize: true,
            apply_noise: true,
            seed: 12345,
        };

        let simulator = Simulator::new(config.clone());

        let circuit_json = r#"{
            "qubits": 2,
            "global_noise": {"noise_type": "depolarizing", "probability": 0.2},
            "gates": [
                {"gate_type": "H", "target": 0},
                {"gate_type": "CNOT", "control": 0, "target": 1},
                {"gate_type": "RX", "target": 1, "parameter": 0.7}
            ]
        }"#;

        let circuit = Circuit::from_json(circuit_json).unwrap();

        let r1 = simulator.run(&circuit).unwrap();
        let r2 = Simulator::new(config).run(&circuit).unwrap();

        assert_eq!(r1.measurement.counts, r2.measurement.counts);
        assert_eq!(r1.measurement.probabilities, r2.measurement.probabilities);
    }

    #[test]
    fn test_seeded_mid_circuit_measurement_is_deterministic() {
        let config = SimulationConfig {
            shots: 0,
            optimize: false,
            apply_noise: false,
            seed: 999,
        };

        let simulator = Simulator::new(config.clone());

        let circuit_json = r#"{
            "qubits": 2,
            "gates": [
                {"gate_type": "H", "target": 0},
                {"gate_type": "MEASURE", "target": 0},
                {"gate_type": "CNOT", "control": 0, "target": 1}
            ]
        }"#;

        let circuit = Circuit::from_json(circuit_json).unwrap();

        let r1 = simulator.run(&circuit).unwrap();
        let r2 = Simulator::new(config).run(&circuit).unwrap();

        assert_eq!(r1.measurement.probabilities, r2.measurement.probabilities);
    }

    #[test]
    fn test_phase2_new_gates_and_controls() {
        let config = SimulationConfig {
            shots: 0,
            optimize: false,
            apply_noise: false,
            seed: 7,
        };

        let simulator = Simulator::new(config);

        let circuit_json = r#"{
            "qubits": 3,
            "gates": [
                {"gate_type": "X", "target": 0},
                {"gate_type": "X", "target": 1},
                {"gate_type": "CCX", "controls": [0, 1], "target": 2},
                {"gate_type": "BARRIER"},
                {"gate_type": "RESET", "target": 2}
            ]
        }"#;

        let circuit = Circuit::from_json(circuit_json).unwrap();
        let result = simulator.run(&circuit).unwrap();

        // Final state should be |011> in this bitstring convention after reset on qubit 2.
        let p011 = result
            .measurement
            .probabilities
            .get("011")
            .copied()
            .unwrap_or(0.0);
        assert!(p011 > 0.99);
    }

    #[test]
    fn test_phase3_conditional_execution_with_classical_register() {
        let config = SimulationConfig {
            shots: 0,
            optimize: false,
            apply_noise: false,
            seed: 123,
        };

        let simulator = Simulator::new(config);

        let circuit_json = r#"{
            "qubits": 2,
            "classical_bits": 2,
            "gates": [
                {"gate_type": "H", "target": 0},
                {"gate_type": "MEASURE", "target": 0, "cbit": 1},
                {"gate_type": "X", "target": 1, "condition": {"register": 1, "value": true}}
            ]
        }"#;

        let circuit = Circuit::from_json(circuit_json).unwrap();
        let r1 = simulator.run(&circuit).unwrap();
        let r2 = simulator.run(&circuit).unwrap();

        assert_eq!(r1.measurement.probabilities, r2.measurement.probabilities);
    }

    #[test]
    fn test_phase3_repeat_loop_behavior() {
        let config = SimulationConfig {
            shots: 0,
            optimize: false,
            apply_noise: false,
            seed: 1,
        };

        let simulator = Simulator::new(config);

        let circuit_json = r#"{
            "qubits": 1,
            "gates": [
                {"gate_type": "X", "target": 0, "repeat": 3}
            ]
        }"#;

        let circuit = Circuit::from_json(circuit_json).unwrap();
        let result = simulator.run(&circuit).unwrap();
        let p1 = result.measurement.probabilities.get("1").copied().unwrap_or(0.0);
        assert!(p1 > 0.99);
    }

    #[test]
    fn test_phase5_readout_noise_applied() {
        let config = SimulationConfig {
            shots: 128,
            optimize: false,
            apply_noise: true,
            seed: 42,
        };

        let simulator = Simulator::new(config);
        let circuit_json = r#"{
            "qubits": 1,
            "readout_noise": {"noise_type": "READOUT_ERROR", "probability": 1.0},
            "gates": []
        }"#;

        let circuit = Circuit::from_json(circuit_json).unwrap();
        let result = simulator.run(&circuit).unwrap();
        let p1 = result.measurement.probabilities.get("1").copied().unwrap_or(0.0);
        assert!(p1 > 0.95);
    }
}
