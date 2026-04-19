# Quantum Simulator Engine - Index

## Start here
1. `QUICK_START.md` - shortest path to running the engine.
2. `DELIVERY_SUMMARY.md` - what is implemented and verified.
3. `quantum_engine/README.md` - detailed technical reference.

## Project map
- `quantum_engine/src/state.rs` - state-vector representation and utilities.
- `quantum_engine/src/gates.rs` - gate kernels including CZ/CP/CCX and controlled unitary.
- `quantum_engine/src/circuit.rs` - schema, parsing, and validation for gates/noise/classical flow.
- `quantum_engine/src/simulator.rs` - execution engine, seeded RNG, classical-state path.
- `quantum_engine/src/noise.rs` - basic + advanced + composite channels.
- `quantum_engine/src/measurement.rs` - sampling and readout-error integration.
- `quantum_engine/src/analysis.rs` - observables and analysis toolkit.
- `quantum_engine/src/optimizer.rs` - optimizer metrics and rewrites.
- `quantum_engine/src/runtime.rs` - execution planning.
- `quantum_engine/src/main.rs` - CLI surface.

## Example circuits
- `quantum_engine/examples/bell_state.json`
- `quantum_engine/examples/ghz_state.json`
- `quantum_engine/examples/parametrized_circuit.json`
- `quantum_engine/examples/noisy_circuit.json`
- `quantum_engine/examples/advanced_noise.json`
- `quantum_engine/examples/conditional_repeat.json`
- `quantum_engine/examples/readout_noise.json`

## Common commands
```bash
cd quantum_engine

# build
cargo build --release

# test
cargo test --all-targets

# benchmark
cargo bench --bench gate_benchmarks

# deterministic simulation with JSON payload
./target/release/quantum_engine simulate ./examples/readout_noise.json --shots 1024 --seed 42 --format json
```

## Status summary
- Engine phases 1 through 8 completed.
- Current automated test status: 51 passing tests.
- Bench suite updated for newer gate/noise/analysis paths.
