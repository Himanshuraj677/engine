//! Quantum Circuit Simulator Engine
//!
//! A high-performance, production-grade quantum circuit simulator built in Rust.
//! Supports state-vector simulation, realistic noise models, and advanced optimizations.

pub mod state;
pub mod gates;
pub mod circuit;
pub mod simulator;
pub mod noise;
pub mod measurement;
pub mod optimizer;
pub mod runtime;
pub mod error;

pub use circuit::Circuit;
pub use simulator::Simulator;
pub use measurement::MeasurementResult;
pub use error::{Error, Result};

/// Version string for the engine
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
