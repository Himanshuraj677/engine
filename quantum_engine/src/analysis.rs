//! Observables and analysis toolkit for state-vector simulations.

use num_complex::Complex64;

use crate::error::{Error, Result};
use crate::state::QuantumState;

/// Bloch sphere coordinates for a single qubit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlochVector {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// Compute expectation value for an arbitrary Pauli string over all qubits.
///
/// The pauli_string length must exactly match the state qubit count and may contain
/// only characters I, X, Y, Z.
pub fn expectation_pauli_string(state: &QuantumState, pauli_string: &str) -> Result<f64> {
    if pauli_string.len() != state.num_qubits() {
        return Err(Error::MeasurementError {
            reason: format!(
                "Pauli string length {} does not match qubit count {}",
                pauli_string.len(),
                state.num_qubits()
            ),
        });
    }

    let mut exp = Complex64::new(0.0, 0.0);

    for (idx, &amp) in state.amplitudes().iter().enumerate() {
        let mut mapped_idx = idx;
        let mut phase = Complex64::new(1.0, 0.0);

        for (qubit, op) in pauli_string.chars().rev().enumerate() {
            let bit = ((idx >> qubit) & 1) == 1;
            match op {
                'I' => {}
                'X' => {
                    mapped_idx ^= 1 << qubit;
                }
                'Y' => {
                    mapped_idx ^= 1 << qubit;
                    phase *= if bit {
                        Complex64::new(0.0, -1.0)
                    } else {
                        Complex64::new(0.0, 1.0)
                    };
                }
                'Z' => {
                    if bit {
                        phase = -phase;
                    }
                }
                _ => {
                    return Err(Error::MeasurementError {
                        reason: format!("Invalid Pauli operator '{}'. Use only I/X/Y/Z", op),
                    });
                }
            }
        }

        exp += amp.conj() * phase * state.amplitudes()[mapped_idx];
    }

    Ok(exp.re)
}

/// Compute reduced density matrix for a selected subsystem.
///
/// The resulting matrix has dimension 2^k x 2^k where k = keep_qubits.len().
pub fn reduced_density_matrix(state: &QuantumState, keep_qubits: &[usize]) -> Result<Vec<Vec<Complex64>>> {
    validate_subsystem(state, keep_qubits)?;

    let k = keep_qubits.len();
    let dim = 1usize << k;
    let size = state.size();
    let keep_mask = bit_mask(keep_qubits);

    let mut rho = vec![vec![Complex64::new(0.0, 0.0); dim]; dim];

    for i in 0..size {
        for j in 0..size {
            if (i & !keep_mask) == (j & !keep_mask) {
                let sub_i = project_index(i, keep_qubits);
                let sub_j = project_index(j, keep_qubits);
                rho[sub_i][sub_j] += state.amplitudes()[i] * state.amplitudes()[j].conj();
            }
        }
    }

    Ok(rho)
}

/// Compute pure-state fidelity between two state vectors.
pub fn fidelity(state_a: &QuantumState, state_b: &QuantumState) -> Result<f64> {
    if state_a.num_qubits() != state_b.num_qubits() {
        return Err(Error::MeasurementError {
            reason: format!(
                "Fidelity requires equal qubit counts: {} vs {}",
                state_a.num_qubits(),
                state_b.num_qubits()
            ),
        });
    }

    let overlap = state_a
        .amplitudes()
        .iter()
        .zip(state_b.amplitudes().iter())
        .fold(Complex64::new(0.0, 0.0), |acc, (a, b)| acc + a.conj() * b);

    Ok(overlap.norm_sqr())
}

/// Compute entanglement entropy S = -Tr(rho log2 rho) for a single-qubit subsystem.
///
/// This implementation is intentionally minimal and currently supports only one qubit
/// in the subsystem for numerical robustness without external linear algebra backends.
pub fn entanglement_entropy(state: &QuantumState, subsystem: &[usize]) -> Result<f64> {
    if subsystem.len() != 1 {
        return Err(Error::MeasurementError {
            reason: "entanglement_entropy currently supports exactly one subsystem qubit".to_string(),
        });
    }

    let rho = reduced_density_matrix(state, subsystem)?;
    let a = rho[0][0].re;
    let d = rho[1][1].re;
    let b = rho[0][1];

    // Eigenvalues of a 2x2 Hermitian matrix.
    let trace = a + d;
    let det = a * d - b.norm_sqr();
    let disc = (trace * trace - 4.0 * det).max(0.0).sqrt();
    let lambda1 = 0.5 * (trace + disc);
    let lambda2 = 0.5 * (trace - disc);

    Ok(shannon_entropy_binary(lambda1) + shannon_entropy_binary(lambda2))
}

/// Compute Bloch vector for one qubit by tracing out all other qubits.
pub fn bloch_vector(state: &QuantumState, qubit: usize) -> Result<BlochVector> {
    validate_subsystem(state, &[qubit])?;
    let rho = reduced_density_matrix(state, &[qubit])?;

    let rho00 = rho[0][0].re;
    let rho11 = rho[1][1].re;
    let rho01 = rho[0][1];

    Ok(BlochVector {
        x: 2.0 * rho01.re,
        y: -2.0 * rho01.im,
        z: rho00 - rho11,
    })
}

fn validate_subsystem(state: &QuantumState, keep_qubits: &[usize]) -> Result<()> {
    if keep_qubits.is_empty() {
        return Err(Error::MeasurementError {
            reason: "Subsystem must include at least one qubit".to_string(),
        });
    }

    let mut seen = std::collections::BTreeSet::new();
    for &q in keep_qubits {
        if q >= state.num_qubits() {
            return Err(Error::InvalidQubitIndex {
                index: q,
                max: state.num_qubits(),
            });
        }
        if !seen.insert(q) {
            return Err(Error::MeasurementError {
                reason: format!("Subsystem contains duplicate qubit {}", q),
            });
        }
    }

    Ok(())
}

fn bit_mask(qubits: &[usize]) -> usize {
    qubits.iter().fold(0usize, |mask, &q| mask | (1 << q))
}

fn project_index(index: usize, keep_qubits: &[usize]) -> usize {
    let mut projected = 0usize;
    for (dst, &src_qubit) in keep_qubits.iter().enumerate() {
        let bit = (index >> src_qubit) & 1;
        projected |= bit << dst;
    }
    projected
}

fn shannon_entropy_binary(p: f64) -> f64 {
    if p <= 1e-15 {
        0.0
    } else {
        -p * p.log2()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gates;

    #[test]
    fn test_expectation_pauli_string_bell() {
        let mut state = QuantumState::new(2).unwrap();
        gates::gate_h(&mut state, 0).unwrap();
        gates::gate_cnot(&mut state, 0, 1).unwrap();

        let zz = expectation_pauli_string(&state, "ZZ").unwrap();
        let xx = expectation_pauli_string(&state, "XX").unwrap();
        let zi = expectation_pauli_string(&state, "ZI").unwrap();

        assert!((zz - 1.0).abs() < 1e-10);
        assert!((xx - 1.0).abs() < 1e-10);
        assert!(zi.abs() < 1e-10);
    }

    #[test]
    fn test_reduced_density_matrix_bell_single_qubit() {
        let mut state = QuantumState::new(2).unwrap();
        gates::gate_h(&mut state, 0).unwrap();
        gates::gate_cnot(&mut state, 0, 1).unwrap();

        let rho = reduced_density_matrix(&state, &[0]).unwrap();
        assert!((rho[0][0].re - 0.5).abs() < 1e-10);
        assert!((rho[1][1].re - 0.5).abs() < 1e-10);
        assert!(rho[0][1].norm() < 1e-10);
        assert!(rho[1][0].norm() < 1e-10);
    }

    #[test]
    fn test_entanglement_entropy_single_qubit_bell() {
        let mut state = QuantumState::new(2).unwrap();
        gates::gate_h(&mut state, 0).unwrap();
        gates::gate_cnot(&mut state, 0, 1).unwrap();

        let s = entanglement_entropy(&state, &[0]).unwrap();
        assert!((s - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_fidelity() {
        let state_a = QuantumState::new(1).unwrap();
        let mut state_b = QuantumState::new(1).unwrap();
        gates::gate_x(&mut state_b, 0).unwrap();

        let f_same = fidelity(&state_a, &state_a).unwrap();
        let f_orth = fidelity(&state_a, &state_b).unwrap();

        assert!((f_same - 1.0).abs() < 1e-10);
        assert!(f_orth < 1e-10);
    }

    #[test]
    fn test_bloch_vector() {
        let state_0 = QuantumState::new(1).unwrap();
        let bloch_0 = bloch_vector(&state_0, 0).unwrap();
        assert!(bloch_0.x.abs() < 1e-10);
        assert!(bloch_0.y.abs() < 1e-10);
        assert!((bloch_0.z - 1.0).abs() < 1e-10);

        let mut state_plus = QuantumState::new(1).unwrap();
        gates::gate_h(&mut state_plus, 0).unwrap();
        let bloch_plus = bloch_vector(&state_plus, 0).unwrap();
        assert!((bloch_plus.x - 1.0).abs() < 1e-10);
        assert!(bloch_plus.y.abs() < 1e-10);
        assert!(bloch_plus.z.abs() < 1e-10);
    }

    #[test]
    fn test_entropy_multi_qubit_subsystem_not_supported_yet() {
        let state = QuantumState::new(3).unwrap();
        assert!(entanglement_entropy(&state, &[0, 1]).is_err());
    }
}
