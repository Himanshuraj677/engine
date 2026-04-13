//! Main simulation engine: orchestrates circuit execution

use crate::circuit::{Circuit, GateInstruction};
use crate::state::QuantumState;
use crate::measurement::MeasurementResult;
use crate::error::{Error, Result};
use crate::{gates, noise, optimizer, runtime};
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

        // Execute circuit
        self.execute_circuit(&circuit, &mut state)?;

        // Get measurement results
        let measurement = if self.config.shots > 0 {
            MeasurementResult::sample(&state, self.config.shots)?
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

    /// Execute a circuit on a quantum state
    fn execute_circuit(&self, circuit: &Circuit, state: &mut QuantumState) -> Result<()> {
        let plan = runtime::ExecutionPlan::from_circuit(circuit)?;
        debug!("Execution plan: {} layers", plan.num_layers());
        debug!("Parallelism factor: {:.2}", plan.parallelism_factor());

        for (layer_idx, layer) in plan.layers.iter().enumerate() {
            debug!("Executing layer {}: {} gates", layer_idx, layer.gates.len());

            for gate in &layer.gates {
                self.execute_gate(state, gate)?;

                // Apply noise if configured
                if self.config.apply_noise {
                    if let Some(noise_config) = &gate.noise {
                        if let Some(target) = gate.target {
                            noise::apply_noise(state, target, noise_config)?;
                        }
                    } else if let Some(noise_config) = &circuit.global_noise {
                        if let Some(target) = gate.target {
                            noise::apply_noise(state, target, noise_config)?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Execute a single gate
    fn execute_gate(&self, state: &mut QuantumState, gate: &GateInstruction) -> Result<()> {
        let gate_type = gate.gate_type.to_uppercase();

        match gate_type.as_str() {
            // Single-qubit gates
            "X" => gates::gate_x(state, gate.target.ok_or_else(|| {
                Error::SimulationError {
                    reason: "H gate requires target".to_string(),
                }
            })?)?,

            "Y" => gates::gate_y(state, gate.target.ok_or_else(|| {
                Error::SimulationError {
                    reason: "Y gate requires target".to_string(),
                }
            })?)?,

            "Z" => gates::gate_z(state, gate.target.ok_or_else(|| {
                Error::SimulationError {
                    reason: "Z gate requires target".to_string(),
                }
            })?)?,

            "H" => gates::gate_h(state, gate.target.ok_or_else(|| {
                Error::SimulationError {
                    reason: "H gate requires target".to_string(),
                }
            })?)?,

            "S" => gates::gate_s(state, gate.target.ok_or_else(|| {
                Error::SimulationError {
                    reason: "S gate requires target".to_string(),
                }
            })?)?,

            "T" => gates::gate_t(state, gate.target.ok_or_else(|| {
                Error::SimulationError {
                    reason: "T gate requires target".to_string(),
                }
            })?)?,

            // Parameterized gates
            "RX" => {
                let target = gate.target.ok_or_else(|| {
                    Error::SimulationError {
                        reason: "RX gate requires target".to_string(),
                    }
                })?;
                let param = gate.parameter.ok_or_else(|| {
                    Error::SimulationError {
                        reason: "RX gate requires parameter".to_string(),
                    }
                })?;
                gates::gate_rx(state, target, param)?;
            }

            "RY" => {
                let target = gate.target.ok_or_else(|| {
                    Error::SimulationError {
                        reason: "RY gate requires target".to_string(),
                    }
                })?;
                let param = gate.parameter.ok_or_else(|| {
                    Error::SimulationError {
                        reason: "RY gate requires parameter".to_string(),
                    }
                })?;
                gates::gate_ry(state, target, param)?;
            }

            "RZ" => {
                let target = gate.target.ok_or_else(|| {
                    Error::SimulationError {
                        reason: "RZ gate requires target".to_string(),
                    }
                })?;
                let param = gate.parameter.ok_or_else(|| {
                    Error::SimulationError {
                        reason: "RZ gate requires parameter".to_string(),
                    }
                })?;
                gates::gate_rz(state, target, param)?;
            }

            // Two-qubit gates
            "CNOT" | "CX" => {
                let control = gate.control.ok_or_else(|| {
                    Error::SimulationError {
                        reason: "CNOT gate requires control".to_string(),
                    }
                })?;
                let target = gate.target.ok_or_else(|| {
                    Error::SimulationError {
                        reason: "CNOT gate requires target".to_string(),
                    }
                })?;
                gates::gate_cnot(state, control, target)?;
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
                let (_result, _bit) = crate::measurement::MeasurementResult::measure_single(state, target)?;
                debug!("Mid-circuit measurement on qubit {}: {}", target, _bit);
            }

            _ => {
                return Err(Error::SimulationError {
                    reason: format!("Unknown gate: {}", gate_type),
                });
            }
        }

        Ok(())
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
}
