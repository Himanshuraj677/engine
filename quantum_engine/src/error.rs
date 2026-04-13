//! Error types for the quantum engine

use thiserror::Error;

/// Result type for quantum engine operations
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur during quantum simulation
#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid qubit index: {index}, max qubits: {max}")]
    InvalidQubitIndex { index: usize, max: usize },

    #[error("Invalid gate parameters: {reason}")]
    InvalidGateParameters { reason: String },

    #[error("Circuit validation failed: {reason}")]
    CircuitValidationError { reason: String },

    #[error("State vector size mismatch: expected 2^{qubits}, got {actual}")]
    StateSizeMismatch { qubits: usize, actual: usize },

    #[error("JSON parse error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Invalid noise configuration: {reason}")]
    InvalidNoiseConfig { reason: String },

    #[error("Measurement error: {reason}")]
    MeasurementError { reason: String },

    #[error("Simulation error: {reason}")]
    SimulationError { reason: String },

    #[error("Index out of bounds: {0}")]
    IndexOutOfBounds(String),
}
