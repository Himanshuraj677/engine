//! Quantum gate implementations with performance optimizations
//!
//! Uses bit manipulation and efficient state vector updates instead of full matrix multiplication.

use num_complex::Complex64;
use std::f64::consts::PI;
use crate::state::QuantumState;
use crate::error::{Error, Result};

// Constants


/// Pauli X gate: |0⟩ → |1⟩, |1⟩ → |0⟩
pub fn gate_x(state: &mut QuantumState, target: usize) -> Result<()> {
    validate_qubit(target, state.num_qubits())?;

    let mask = 1 << target;
    let size = state.size();
    let amplitudes = state.amplitudes_mut();

    // Swap amplitudes for states differing only in target qubit
    for i in 0..size {
        if (i & mask) == 0 {
            let j = i | mask;
            amplitudes.swap(i, j);
        }
    }
    Ok(())
}

/// Pauli Y gate: |0⟩ → i|1⟩, |1⟩ → -i|0⟩
pub fn gate_y(state: &mut QuantumState, target: usize) -> Result<()> {
    validate_qubit(target, state.num_qubits())?;

    let mask = 1 << target;
    let size = state.size();
    let amplitudes = state.amplitudes_mut();
    let i = Complex64::new(0.0, 1.0);

    for idx in 0..size {
        if (idx & mask) == 0 {
            let j = idx | mask;
            let temp = amplitudes[idx];
            amplitudes[idx] = -i * amplitudes[j];
            amplitudes[j] = i * temp;
        }
    }
    Ok(())
}

/// Pauli Z gate: |0⟩ → |0⟩, |1⟩ → -|1⟩
pub fn gate_z(state: &mut QuantumState, target: usize) -> Result<()> {
    validate_qubit(target, state.num_qubits())?;

    let mask = 1 << target;
    let amplitudes = state.amplitudes_mut();

    for (idx, amp) in amplitudes.iter_mut().enumerate() {
        if (idx & mask) != 0 {
            *amp = -*amp;
        }
    }
    Ok(())
}

/// Hadamard gate: H = 1/√2 * [[1, 1], [1, -1]]
pub fn gate_h(state: &mut QuantumState, target: usize) -> Result<()> {
    validate_qubit(target, state.num_qubits())?;

    let mask = 1 << target;
    let size = state.size();
    let factor = 1.0 / std::f64::consts::SQRT_2 as f64;
    let amplitudes = state.amplitudes_mut();

    for i in 0..size {
        if (i & mask) == 0 {
            let j = i | mask;
            let a0 = amplitudes[i];
            let a1 = amplitudes[j];
            amplitudes[i] = factor * (a0 + a1);
            amplitudes[j] = factor * (a0 - a1);
        }
    }
    Ok(())
}

/// S gate (Phase gate): |0⟩ → |0⟩, |1⟩ → i|1⟩
pub fn gate_s(state: &mut QuantumState, target: usize) -> Result<()> {
    validate_qubit(target, state.num_qubits())?;

    let mask = 1 << target;
    let amplitudes = state.amplitudes_mut();
    let i = Complex64::new(0.0, 1.0);

    for (idx, amp) in amplitudes.iter_mut().enumerate() {
        if (idx & mask) != 0 {
            *amp *= i;
        }
    }
    Ok(())
}

/// T gate: |0⟩ → |0⟩, |1⟩ → e^(iπ/4)|1⟩
pub fn gate_t(state: &mut QuantumState, target: usize) -> Result<()> {
    validate_qubit(target, state.num_qubits())?;

    let mask = 1 << target;
    let amplitudes = state.amplitudes_mut();
    let phase = Complex64::new((PI / 4.0).cos(), (PI / 4.0).sin());

    for (idx, amp) in amplitudes.iter_mut().enumerate() {
        if (idx & mask) != 0 {
            *amp *= phase;
        }
    }
    Ok(())
}

/// RX rotation: e^(-iθX/2)
pub fn gate_rx(state: &mut QuantumState, target: usize, theta: f64) -> Result<()> {
    validate_qubit(target, state.num_qubits())?;

    let mask = 1 << target;
    let size = state.size();
    let half_theta = theta / 2.0;
    let c = Complex64::new(half_theta.cos(), 0.0);
    let s = Complex64::new(0.0, -half_theta.sin());
    let amplitudes = state.amplitudes_mut();

    for i in 0..size {
        if (i & mask) == 0 {
            let j = i | mask;
            let a0 = amplitudes[i];
            let a1 = amplitudes[j];
            amplitudes[i] = c * a0 + s * a1;
            amplitudes[j] = s * a0 + c * a1;
        }
    }
    Ok(())
}

/// RY rotation: e^(-iθY/2)
pub fn gate_ry(state: &mut QuantumState, target: usize, theta: f64) -> Result<()> {
    validate_qubit(target, state.num_qubits())?;

    let mask = 1 << target;
    let size = state.size();
    let half_theta = theta / 2.0;
    let c = Complex64::new(half_theta.cos(), 0.0);
    let s = Complex64::new(half_theta.sin(), 0.0);
    let amplitudes = state.amplitudes_mut();

    for i in 0..size {
        if (i & mask) == 0 {
            let j = i | mask;
            let a0 = amplitudes[i];
            let a1 = amplitudes[j];
            amplitudes[i] = c * a0 - s * a1;
            amplitudes[j] = s * a0 + c * a1;
        }
    }
    Ok(())
}

/// RZ rotation: e^(-iθZ/2)
pub fn gate_rz(state: &mut QuantumState, target: usize, theta: f64) -> Result<()> {
    validate_qubit(target, state.num_qubits())?;

    let mask = 1 << target;
    let amplitudes = state.amplitudes_mut();
    let phase = Complex64::new((-theta / 2.0).cos(), (-theta / 2.0).sin());

    for (idx, amp) in amplitudes.iter_mut().enumerate() {
        if (idx & mask) != 0 {
            *amp *= phase;
        }
    }
    Ok(())
}

/// CNOT (Controlled-NOT): flips target if control is |1⟩
pub fn gate_cnot(state: &mut QuantumState, control: usize, target: usize) -> Result<()> {
    validate_qubit(control, state.num_qubits())?;
    validate_qubit(target, state.num_qubits())?;

    if control == target {
        return Err(Error::InvalidGateParameters {
            reason: "Control and target must be different qubits".to_string(),
        });
    }

    let control_mask = 1 << control;
    let target_mask = 1 << target;
    let size = state.size();
    let amplitudes = state.amplitudes_mut();

    // Swap states where control=1: |...1...0...⟩ <-> |...1...1...⟩
    for i in 0..size {
        if (i & control_mask) != 0 && (i & target_mask) == 0 {
            let j = i | target_mask;
            amplitudes.swap(i, j);
        }
    }
    Ok(())
}

/// SWAP gate: exchanges two qubits
pub fn gate_swap(state: &mut QuantumState, qubit1: usize, qubit2: usize) -> Result<()> {
    validate_qubit(qubit1, state.num_qubits())?;
    validate_qubit(qubit2, state.num_qubits())?;

    if qubit1 == qubit2 {
        return Err(Error::InvalidGateParameters {
            reason: "Swap qubits must be different".to_string(),
        });
    }

    let mask1 = 1 << qubit1;
    let mask2 = 1 << qubit2;
    let size = state.size();
    let amplitudes = state.amplitudes_mut();

    for i in 0..size {
        if ((i & mask1) >> qubit1) != ((i & mask2) >> qubit2) {
            let j = (i ^ mask1) ^ mask2;
            if i < j {
                amplitudes.swap(i, j);
            }
        }
    }
    Ok(())
}

/// Apply a single-qubit unitary with optional controls.
/// If `controls` is empty, this behaves as a standard single-qubit gate.
pub fn gate_controlled_unitary(
    state: &mut QuantumState,
    controls: &[usize],
    target: usize,
    matrix: [[Complex64; 2]; 2],
) -> Result<()> {
    validate_qubit(target, state.num_qubits())?;

    let mut control_mask = 0usize;
    for &control in controls {
        validate_qubit(control, state.num_qubits())?;
        if control == target {
            return Err(Error::InvalidGateParameters {
                reason: "Control and target must be different qubits".to_string(),
            });
        }
        control_mask |= 1 << control;
    }

    apply_single_qubit_unitary(state, control_mask, target, matrix);
    Ok(())
}

/// Controlled-Z gate.
pub fn gate_cz(state: &mut QuantumState, control: usize, target: usize) -> Result<()> {
    gate_controlled_unitary(
        state,
        &[control],
        target,
        [
            [Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0)],
            [Complex64::new(0.0, 0.0), Complex64::new(-1.0, 0.0)],
        ],
    )
}

/// Controlled-phase gate with phase parameter phi.
pub fn gate_cp(state: &mut QuantumState, control: usize, target: usize, phi: f64) -> Result<()> {
    let phase = Complex64::new(phi.cos(), phi.sin());
    gate_controlled_unitary(
        state,
        &[control],
        target,
        [
            [Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0)],
            [Complex64::new(0.0, 0.0), phase],
        ],
    )
}

/// Toffoli (CCX) gate.
pub fn gate_ccx(
    state: &mut QuantumState,
    control1: usize,
    control2: usize,
    target: usize,
) -> Result<()> {
    gate_controlled_unitary(
        state,
        &[control1, control2],
        target,
        [
            [Complex64::new(0.0, 0.0), Complex64::new(1.0, 0.0)],
            [Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0)],
        ],
    )
}

/// Apply arbitrary single-qubit unitary matrix.
pub fn gate_unitary(state: &mut QuantumState, target: usize, matrix: [[Complex64; 2]; 2]) -> Result<()> {
    gate_controlled_unitary(state, &[], target, matrix)
}

/// Internal single-qubit unitary application with a precomputed controls mask.
fn apply_single_qubit_unitary(
    state: &mut QuantumState,
    control_mask: usize,
    target: usize,
    matrix: [[Complex64; 2]; 2],
) {
    let target_mask = 1 << target;
    let size = state.size();
    let amplitudes = state.amplitudes_mut();

    for i in 0..size {
        if (i & target_mask) == 0 && (i & control_mask) == control_mask {
            let j = i | target_mask;
            let a0 = amplitudes[i];
            let a1 = amplitudes[j];
            amplitudes[i] = matrix[0][0] * a0 + matrix[0][1] * a1;
            amplitudes[j] = matrix[1][0] * a0 + matrix[1][1] * a1;
        }
    }
}

/// Helper: Validate qubit index
#[inline]
fn validate_qubit(qubit: usize, num_qubits: usize) -> Result<()> {
    if qubit >= num_qubits {
        return Err(Error::InvalidQubitIndex {
            index: qubit,
            max: num_qubits,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pauli_x() {
        let mut state = QuantumState::new(1).unwrap();
        gate_x(&mut state, 0).unwrap();
        assert!(state.get_amplitude(1).unwrap().re.abs() - 1.0 < 1e-10);
    }

    #[test]
    fn test_hadamard_double() {
        let mut state = QuantumState::new(1).unwrap();
        gate_h(&mut state, 0).unwrap();
        gate_h(&mut state, 0).unwrap();
        // Should return to |0⟩
        assert!(state.get_amplitude(0).unwrap().re.abs() - 1.0 < 1e-10);
    }

    #[test]
    fn test_cnot() {
        let mut state = QuantumState::new(2).unwrap();
        gate_h(&mut state, 0).unwrap(); // Create superposition
        gate_cnot(&mut state, 0, 1).unwrap(); // Entangle
        // Check that probabilities are equal for |00⟩ and |11⟩
        let p00 = state.probability(0).unwrap();
        let p11 = state.probability(3).unwrap();
        assert!((p00 - 0.5).abs() < 1e-10);
        assert!((p11 - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_rx_rotation() {
        let mut state = QuantumState::new(1).unwrap();
        gate_rx(&mut state, 0, PI).unwrap(); // Should rotate to |1⟩
        assert!(state.probability(1).unwrap() > 0.99);
    }

    #[test]
    fn test_cz_phase_flip() {
        let mut state = QuantumState::new(2).unwrap();
        gate_x(&mut state, 0).unwrap();
        gate_x(&mut state, 1).unwrap();
        let before = state.get_amplitude(3).unwrap();
        gate_cz(&mut state, 0, 1).unwrap();
        let after = state.get_amplitude(3).unwrap();
        assert!((after + before).norm() < 1e-10);
    }

    #[test]
    fn test_ccx() {
        let mut state = QuantumState::new(3).unwrap();
        gate_x(&mut state, 0).unwrap();
        gate_x(&mut state, 1).unwrap();
        gate_ccx(&mut state, 0, 1, 2).unwrap();
        assert!(state.probability(7).unwrap() > 0.99);
    }

    #[test]
    fn test_custom_unitary_x_matrix() {
        let mut state = QuantumState::new(1).unwrap();
        let x = [
            [Complex64::new(0.0, 0.0), Complex64::new(1.0, 0.0)],
            [Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0)],
        ];
        gate_unitary(&mut state, 0, x).unwrap();
        assert!(state.probability(1).unwrap() > 0.99);
    }
}
