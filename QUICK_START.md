# Quantum Simulator Engine - Quick Start

## What is included
- Production Rust state-vector simulator.
- 17 gate types and advanced controls: controls list, condition, cbit, repeat, reset, barrier, custom unitary.
- Advanced noise stack: bit/phase/depolarizing/amplitude damping, T1/T2 relaxation, coherent over-rotation, crosstalk, Kraus, composite channels, readout error.
- Analysis toolkit: Pauli expectations, reduced density matrix, single-qubit entanglement entropy, fidelity, Bloch vector.
- Deterministic execution via seedable RNG.
- CLI with human-readable and JSON output modes.

## Fast run
```bash
cd quantum_engine
cargo build --release

# Basic run
./target/release/quantum_engine simulate ./examples/bell_state.json --shots 1000

# Deterministic + JSON output
./target/release/quantum_engine simulate ./examples/readout_noise.json --shots 512 --seed 42 --format json
```

## Key CLI options
- `--shots <N>`: measurement samples.
- `--no-optimize`: disable optimizer.
- `--no-noise`: disable noise injection.
- `--seed <u64>`: reproducible RNG stream (`0` = entropy).
- `--format <pretty|json>`: stdout/file payload style.
- `-o, --output <FILE>`: save full simulation result payload.

## Example circuits
- `examples/bell_state.json`
- `examples/ghz_state.json`
- `examples/parametrized_circuit.json`
- `examples/noisy_circuit.json`
- `examples/advanced_noise.json`
- `examples/conditional_repeat.json`
- `examples/readout_noise.json`

## Core commands
```bash
# Validate a circuit
./target/release/quantum_engine validate ./examples/advanced_noise.json

# Print embedded examples
./target/release/quantum_engine examples

# Run tests
cargo test --all-targets

# Run benchmarks
cargo bench --bench gate_benchmarks
```

## Current verification snapshot
- Unit tests passing: 51/51.
- Bench target compiles and executes.
- JSON CLI mode emits machine-parseable payload.

## Read next
- Technical details: `quantum_engine/README.md`
- Design notes: `IMPLEMENTATION_SUMMARY.md`
- Integration patterns: `INTEGRATION_GUIDE.md`
