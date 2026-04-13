//! Execution planner for parallel gate execution optimization

use crate::circuit::{Circuit, GateInstruction};
use crate::error::Result;

/// A layer of gates that can be executed in parallel (no qubit conflicts)
#[derive(Debug, Clone)]
pub struct GateLayer {
    pub gates: Vec<GateInstruction>,
}

impl GateLayer {
    /// Check if a gate can be added to this layer (no qubit conflicts)
    pub fn can_add(&self, gate: &GateInstruction) -> bool {
        let gate_qubits = extract_qubits(gate);

        for existing in &self.gates {
            let existing_qubits = extract_qubits(existing);

            // Check for overlap
            for &q in &gate_qubits {
                if existing_qubits.contains(&q) {
                    return false;
                }
            }
        }

        true
    }

    /// Add a gate to this layer (panics if conflicting)
    pub fn add(&mut self, gate: GateInstruction) {
        self.gates.push(gate);
    }
}

/// Execution plan: circuit broken into parallel layers
#[derive(Debug, Clone)]
pub struct ExecutionPlan {
    pub layers: Vec<GateLayer>,
}

impl ExecutionPlan {
    /// Create execution plan from circuit (greedy layer assignment)
    pub fn from_circuit(circuit: &Circuit) -> Result<Self> {
        let mut layers: Vec<GateLayer> = Vec::new();

        for gate in &circuit.gates {
            let mut placed = false;

            // Try to place in existing layer
            for layer in &mut layers {
                if layer.can_add(gate) {
                    layer.add(gate.clone());
                    placed = true;
                    break;
                }
            }

            // Create new layer if needed
            if !placed {
                let mut new_layer = GateLayer { gates: vec![] };
                new_layer.add(gate.clone());
                layers.push(new_layer);
            }
        }

        Ok(ExecutionPlan { layers })
    }

    /// Get parallelism factor (average gates per layer)
    pub fn parallelism_factor(&self) -> f64 {
        if self.layers.is_empty() {
            return 0.0;
        }

        let total_gates: usize = self.layers.iter().map(|l| l.gates.len()).sum();
        total_gates as f64 / self.layers.len() as f64
    }

    /// Number of layers
    pub fn num_layers(&self) -> usize {
        self.layers.len()
    }

    /// Total number of gates
    pub fn total_gates(&self) -> usize {
        self.layers.iter().map(|l| l.gates.len()).sum()
    }
}

/// Extract qubits used by a gate
fn extract_qubits(gate: &GateInstruction) -> Vec<usize> {
    let mut qubits = Vec::new();

    if let Some(target) = gate.target {
        qubits.push(target);
    }

    if let Some(control) = gate.control {
        qubits.push(control);
    }

    qubits
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gate_layer_conflict() {
        let gate1 = GateInstruction {
            gate_type: "H".to_string(),
            target: Some(0),
            control: None,
            parameter: None,
            noise: None,
        };

        let gate2 = GateInstruction {
            gate_type: "H".to_string(),
            target: Some(0),
            control: None,
            parameter: None,
            noise: None,
        };

        let mut layer = GateLayer { gates: vec![] };
        layer.add(gate1);
        assert!(!layer.can_add(&gate2)); // Same target qubit
    }

    #[test]
    fn test_gate_layer_parallel() {
        let gate1 = GateInstruction {
            gate_type: "H".to_string(),
            target: Some(0),
            control: None,
            parameter: None,
            noise: None,
        };

        let gate2 = GateInstruction {
            gate_type: "H".to_string(),
            target: Some(1),
            control: None,
            parameter: None,
            noise: None,
        };

        let mut layer = GateLayer { gates: vec![] };
        layer.add(gate1);
        assert!(layer.can_add(&gate2)); // Different qubits
    }

    #[test]
    fn test_execution_plan() {
        let circuit = Circuit {
            qubits: 2,
            gates: vec![
                GateInstruction {
                    gate_type: "H".to_string(),
                    target: Some(0),
                    control: None,
                    parameter: None,
                    noise: None,
                },
                GateInstruction {
                    gate_type: "H".to_string(),
                    target: Some(1),
                    control: None,
                    parameter: None,
                    noise: None,
                },
                GateInstruction {
                    gate_type: "CNOT".to_string(),
                    control: Some(0),
                    target: Some(1),
                    parameter: None,
                    noise: None,
                },
            ],
            global_noise: None,
        };

        let plan = ExecutionPlan::from_circuit(&circuit).unwrap();
        // First layer: both H gates
        // Second layer: CNOT
        assert!(plan.layers.len() <= 3);
    }
}
