//! Circuit optimization: gate fusion, redundancy removal

use crate::circuit::Circuit;
use crate::error::Result;

/// Optimize a circuit by removing redundant gates and fusing operations
pub fn optimize_circuit(circuit: &Circuit) -> Result<Circuit> {
    let mut optimized = circuit.clone();

    // remove self-inverse gates (H, X)
    optimized = remove_self_inverses(&optimized)?;

    // Fuse consecutive single-qubit gates on the same qubit
    optimized = fuse_single_qubit_gates(&optimized)?;

    Ok(optimized)
}

/// Remove gates that are self-inverse (H H = I, X X = I)
fn remove_self_inverses(circuit: &Circuit) -> Result<Circuit> {
    let mut gates = circuit.gates.clone();
    let mut i = 0;

    while i + 1 < gates.len() {
        let current = &gates[i];
        let next = &gates[i + 1];

        let current_type = current.gate_type.to_uppercase();
        let next_type = next.gate_type.to_uppercase();
        let self_inverse = matches!(current_type.as_str(), "H" | "X" | "Y" | "Z");

        if self_inverse && current_type == next_type && current.target == next.target {
            // Check if both gates have same target and no noise
            if current.noise.is_none() && next.noise.is_none() {
                // Remove both gates
                gates.remove(i + 1);
                gates.remove(i);
                continue;
            }
        }

        i += 1;
    }

    Ok(Circuit {
        qubits: circuit.qubits,
        classical_bits: circuit.classical_bits,
        gates,
        global_noise: circuit.global_noise.clone(),
        readout_noise: circuit.readout_noise.clone(),
    })
}

/// Fuse consecutive single-qubit gates (for T-depth reduction)
fn fuse_single_qubit_gates(circuit: &Circuit) -> Result<Circuit> {
    let gates = circuit.gates.clone();
    let mut optimized = Vec::new();

    let mut i = 0;
    while i < gates.len() {
        let gate = &gates[i];
        let gate_type = gate.gate_type.to_uppercase();

        if is_single_qubit_gate(&gate_type) && gate.target.is_some() && gate.noise.is_none() {
            // Look ahead for more consecutive single-qubit gates on same target
            let target = gate.target.unwrap();
            let mut fused = vec![gate.clone()];
            let mut j = i + 1;

            while j < gates.len()
                && gates[j].target == Some(target)
                && is_single_qubit_gate(&gates[j].gate_type.to_uppercase())
                && gates[j].noise.is_none()
            {
                fused.push(gates[j].clone());
                j += 1;
            }

            // For now, just keep all gates (full fusion would require matrix multiplication)
            optimized.extend(fused);
            i = j;
        } else {
            optimized.push(gate.clone());
            i += 1;
        }
    }

    Ok(Circuit {
        qubits: circuit.qubits,
        classical_bits: circuit.classical_bits,
        gates: optimized,
        global_noise: circuit.global_noise.clone(),
        readout_noise: circuit.readout_noise.clone(),
    })
}

/// Check if a gate is single-qubit
fn is_single_qubit_gate(gate_type: &str) -> bool {
    matches!(
        gate_type,
        "X" | "Y" | "Z" | "H" | "S" | "T" | "RX" | "RY" | "RZ"
    )
}

/// Calculate circuit depth (longest path in DAG)
pub fn circuit_depth(circuit: &Circuit) -> usize {
    let mut depth = vec![0; circuit.qubits];

    for gate in &circuit.gates {
        let involved = gate.involved_qubits();
        if involved.is_empty() {
            continue;
        }

        let base = involved
            .iter()
            .map(|&q| depth[q])
            .max()
            .unwrap_or(0)
            + 1;

        for &q in &involved {
            if q < depth.len() {
                depth[q] = depth[q].max(base);
            }
        }
    }

    *depth.iter().max().unwrap_or(&0)
}

/// Count the number of two-qubit gates
pub fn count_two_qubit_gates(circuit: &Circuit) -> usize {
    circuit
        .gates
        .iter()
        .filter(|gate| gate.involved_qubits().len() == 2)
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::circuit::GateInstruction;

    #[test]
    fn test_remove_self_inverses() {
        let circuit = Circuit {
            qubits: 1,
            gates: vec![
                GateInstruction {
                    gate_type: "H".to_string(),
                    target: Some(0),
                    control: None,
                    controls: None,
                    parameter: None,
                    matrix: None,
                    condition: None,
                    cbit: None,
                    repeat: None,
                    noise: None,
                },
                GateInstruction {
                    gate_type: "H".to_string(),
                    target: Some(0),
                    control: None,
                    controls: None,
                    parameter: None,
                    matrix: None,
                    condition: None,
                    cbit: None,
                    repeat: None,
                    noise: None,
                },
            ],
            classical_bits: None,
            global_noise: None,
            readout_noise: None,
        };

        let optimized = remove_self_inverses(&circuit).unwrap();
        assert_eq!(optimized.gates.len(), 0);
    }

    #[test]
    fn test_circuit_depth() {
        let circuit = Circuit {
            qubits: 2,
            gates: vec![
                GateInstruction {
                    gate_type: "H".to_string(),
                    target: Some(0),
                    control: None,
                    controls: None,
                    parameter: None,
                    matrix: None,
                    condition: None,
                    cbit: None,
                    repeat: None,
                    noise: None,
                },
                GateInstruction {
                    gate_type: "CNOT".to_string(),
                    control: Some(0),
                    target: Some(1),
                    controls: None,
                    parameter: None,
                    matrix: None,
                    condition: None,
                    cbit: None,
                    repeat: None,
                    noise: None,
                },
            ],
            classical_bits: None,
            global_noise: None,
            readout_noise: None,
        };

        assert_eq!(circuit_depth(&circuit), 2);
    }
}
