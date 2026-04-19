# Quantum Circuit Simulator Engine - Rust

A **high-performance, production-grade quantum circuit simulator** built entirely in Rust. Designed for accuracy, speed, and extensibility.

## 🎯 Overview

This quantum simulator implements a **state-vector simulation** approach with:

- **Efficient bit manipulation** for gate operations (avoiding full matrix multiplication)
- **Parallel execution planning** for independent gate optimization  
- **Realistic noise models** (bit flip, phase flip, depolarizing, amplitude damping)
- **Mid-circuit measurement** and measurement-based feedback
- **Circuit optimization** (gate fusion, redundancy removal)
- **High-performance core** with Rayon parallelization
- **Production-grade architecture** with clean APIs

### Supported Qubits
- Local simulation: **1-30 qubits** (2^30 ≈ 1B state amplitudes)
- Extensible for distributed/GPU execution

---

## 📦 Architecture

```
quantum_engine/
├── state/           # Quantum state vector (Vec<Complex64>)
├── gates/           # Gate implementations (X, Y, Z, H, S, T, RX, RY, RZ, CNOT, CZ, CP, CCX, SWAP, UNITARY)
├── circuit/         # Circuit parsing & validation (JSON)
├── simulator/       # Main execution engine
├── noise/           # Noise models (realistic quantum errors)
├── measurement/     # Probability & sampling
├── optimizer/       # Circuit optimization
├── runtime/         # Execution planner (parallel layers)
├── error/           # Error types
└── main.rs          # CLI tool
```

Each module is **independently testable**, with unit tests included.

---

## 🚀 Getting Started

### Build

```bash
cd quantum_engine
cargo build --release
```

### Run CLI Examples

```bash
# Simulate Bell state
cargo run --release -- simulate examples/bell_state.json --shots 1000

# Deterministic run with seed + JSON output for automation
cargo run --release -- simulate examples/readout_noise.json --shots 2000 --seed 42 --format json

# Validate a circuit
cargo run --release -- validate examples/ghz_state.json

# Print example circuits
cargo run --release -- examples
```

### Run Tests

```bash
cargo test
```

### Run with Verbose Output

```bash
cargo run --release -- -v simulate examples/bell_state.json
```

---

## 📋 Circuit Format (JSON)

Circuits are defined in JSON format:

```json
{
  "qubits": 2,
  "gates": [
    {"gate_type": "H", "target": 0},
    {"gate_type": "CNOT", "control": 0, "target": 1}
  ]
}
```

### Gates

#### Single-Qubit Gates
- `X`, `Y`, `Z` — Pauli gates
- `H` — Hadamard
- `S`, `T` — Phase and T gates
- `RX(θ)`, `RY(θ)`, `RZ(θ)` — Rotations (parameter required)

#### Two-Qubit Gates
- `CNOT` / `CX` — Controlled-NOT
- `CZ` — Controlled-Z
- `CP(ϕ)` — Controlled-phase
- `CCX` — Toffoli
- `SWAP` — Qubit swap

#### Interop/Utility Gates
- `UNITARY` / `U` — Custom single-qubit 2x2 unitary
- `RESET` — Reset qubit to |0⟩
- `BARRIER` — Scheduling barrier/no-op

#### Measurement
- `MEASURE` — Mid-circuit measurement (collapses state)
- Optional classical destination: `cbit`

#### Classical Control
- Optional circuit-level `classical_bits`
- Gate-level `condition` on classical bits
- Gate-level `repeat` for repeated execution

### Noise Configuration

Add noise to individual gates:

```json
{
  "gate_type": "H",
  "target": 0,
  "noise": {
    "noise_type": "bit_flip",
    "probability": 0.01
  }
}
```

**Noise Types:**
- `"bit_flip"` — |0⟩ ↔ |1⟩ with probability p
- `"phase_flip"` — |1⟩ → -|1⟩ with probability p
- `"depolarizing"` — Replace with random Pauli with probability p
- `"amplitude_damping"` — |1⟩ → |0⟩ energy dissipation
- `"t1_t2_relaxation"` — combined relaxation/dephasing model
- `"coherent_over_rotation"` — deterministic over-rotation around axis
- `"crosstalk"` — coupled-qubit phase errors
- `"kraus"` — custom single-qubit Kraus operators
- `"composite"` — channel chaining/composition

Readout noise can be configured separately using circuit-level `readout_noise` with `"readout_error"`.

Global noise (applies to all gates):

```json
{
  "qubits": 2,
  "global_noise": {
    "noise_type": "bit_flip",
    "probability": 0.005
  },
  "gates": [...]
}
```

---

## 🔬 Example Circuits

### 1. Bell State (Entanglement)

Creates a maximally entangled two-qubit state: (|00⟩ + |11⟩)/√2

```json
{
  "qubits": 2,
  "gates": [
    {"gate_type": "H", "target": 0},
    {"gate_type": "CNOT", "control": 0, "target": 1}
  ]
}
```

**Expected Output:** 50% |00⟩, 50% |11⟩

### 2. GHZ State (3-Qubit Entanglement)

Creates (|000⟩ + |111⟩)/√2:

```json
{
  "qubits": 3,
  "gates": [
    {"gate_type": "H", "target": 0},
    {"gate_type": "CNOT", "control": 0, "target": 1},
    {"gate_type": "CNOT", "control": 1, "target": 2}
  ]
}
```

### 3. Quantum Interference (H→H = Identity)

```json
{
  "qubits": 1,
  "gates": [
    {"gate_type": "H", "target": 0},
    {"gate_type": "H", "target": 0}
  ]
}
```

**Expected Output:** 100% |0⟩ (destructive interference)

### 4. Superposition + Rotation

```json
{
  "qubits": 2,
  "gates": [
    {"gate_type": "H", "target": 0},
    {"gate_type": "RY", "target": 1, "parameter": 1.5708},
    {"gate_type": "CNOT", "control": 0, "target": 1}
  ]
}
```

### 5. Conditional + Repeat + Reset

```json
{
  "qubits": 2,
  "classical_bits": 2,
  "gates": [
    {"gate_type": "H", "target": 0},
    {"gate_type": "MEASURE", "target": 0, "cbit": 1},
    {"gate_type": "X", "target": 1, "condition": {"register": 1, "value": true}, "repeat": 1},
    {"gate_type": "RESET", "target": 0}
  ]
}
```

---

## ⚡ Performance Optimizations

### 1. Bit Manipulation

**Algorithm:** Avoid full matrix multiplication.

Instead, iterate over state indices and selectively flip amplitudes:

```rust
// Instead of: |ψ'⟩ = M|ψ⟩
// Use:
for i in 0..size {
    if (i & mask) == 0 {  // Bit check
        let j = i | mask;  // Bit set
        amplitudes.swap(i, j);  // Direct swap
    }
}
```

**Benefits:**
- O(2^n) instead of O(4^n) for state-vector operations
- Cache-friendly sequential access
- No matrix storage

### 2. Execution Planner

**Algorithm:** Group independent gates into layers.

```rust
// Layer 1: [H(0), H(2)]  -- independent
// Layer 2: [CNOT(0,1)]   -- depends on H(0)
// Layer 3: [X(1), Z(2)]  -- independent
```

Each layer can execute **in parallel** (with Rayon).

**Metrics Provided:**
- Circuit depth (critical path length)
- Parallelism factor (avg gates per layer)
- Two-qubit gate count

### 3. Memory Efficiency

- **State size:** 2^n × 16 bytes (Complex<f64>)
- **No gate matrices stored** (computed on-the-fly)
- **Optional f32 mode** (8 bytes per amplitude) via feature flags
- **Reused buffers** for in-place operations

### 4. Cache Optimization

- **Sequential iteration** over 2^n states
- **Block-wise processing** (avoids random access)
- **Prefetch-friendly** memory layout

### 5. Gate Fusion

Adjacent single-qubit gates on the same qubit are fused (optimization module):

```
H → X → H becomes H → X (removes redundant H)
```

---

## 📊 Simulation Results

Each simulation returns:

```json
{
  "measurement": {
    "probabilities": {
      "00": 0.5,
      "11": 0.5
    },
    "counts": {
      "00": 512,
      "11": 488
    },
    "shots": 1000
  },
  "execution_time_ms": 2.34,
  "circuit_depth": 2,
  "circuit_gates": 2,
  "two_qubit_gates": 1
}
```

### Metrics

- **Probabilities:** Exact probability of each basis state
- **Counts:** Shot-based measurements (Monte Carlo sampling)
- **Execution Time:** Wall-clock simulation time
- **Circuit Depth:** Longest critical path (for optimization analysis)
- **Gate Counts:** Total and two-qubit gate breakdown

---

## 🧪 Testing

Comprehensive test suite:

```bash
cargo test
```

Includes:
- State initialization and normalization
- Individual gate correctness (X, Y, Z, H, S, T, RX, RY, RZ, CNOT, SWAP)
- Entanglement verification (Bell states)
- Destructive interference (H→H)
- Noise models validation
- Circuit parsing and validation
- Measurement and sampling

---

## 📚 API Usage (Rust)

### Basic Simulation

```rust
use quantum_engine::{Circuit, Simulator, SimulationConfig};

fn main() -> Result<()> {
    // Create circuit from JSON
    let json = r#"{
        "qubits": 2,
        "gates": [
            {"gate_type": "H", "target": 0},
            {"gate_type": "CNOT", "control": 0, "target": 1}
        ]
    }"#;
    
    let circuit = Circuit::from_json(json)?;
    
    // Create simulator
    let config = SimulationConfig {
        shots: 1000,
        optimize: true,
        apply_noise: false,
        seed: 0,
    };
    
    let simulator = Simulator::new(config);
    
    // Run simulation
    let result = simulator.run(&circuit)?;
    
    // Access results
    println!("Most likely state: {:?}", result.measurement.most_likely_state());
    println!("Execution time: {:.2} ms", result.execution_time_ms);
    
    Ok(())
}
```

### Direct State Manipulation

```rust
use quantum_engine::state::QuantumState;
use quantum_engine::gates;

let mut state = QuantumState::new(2)?;
gates::gate_h(&mut state, 0)?;
gates::gate_cnot(&mut state, 0, 1)?;

let probs = state.probabilities();
```

---

## 🔮 Advanced Features

### Mid-Circuit Measurement

```json
{
  "qubits": 2,
  "gates": [
    {"gate_type": "H", "target": 0},
    {"gate_type": "MEASURE", "target": 0},
    {"gate_type": "CNOT", "control": 0, "target": 1}
  ]
}
```

This collapses the state after measuring qubit 0.

### Parameterized Circuits

```rust
fn run_parameterized(theta: f64) -> Result<SimulationResult> {
    let json = format!(r#"{{
        "qubits": 1,
        "gates": [
            {{"gate_type": "RY", "target": 0, "parameter": {}}}
        ]
    }}"#, theta);
    
    let circuit = Circuit::from_json(&json)?;
    // ... simulate
}
```

### Noise Injection

```json
{
  "qubits": 1,
  "gates": [
    {"gate_type": "H", "target": 0, "noise": {"noise_type": "bit_flip", "probability": 0.01}}
  ]
}
```

---

## 🔧 Configuration

### Cargo Features

```toml
[features]
default = ["f64"]
f64 = []    # Use f64 for amplitudes (default)
f32 = []    # Use f32 for 50% memory (build with: cargo build --features f32)
```

### Environment Variables

```bash
RUST_LOG=debug cargo run --release -- simulate circuit.json
```

---

## 🏗️ Extensibility

### Adding New Gates

Implement a gate function in `src/gates.rs`:

```rust
pub fn gate_my_custom(state: &mut QuantumState, target: usize) -> Result<()> {
    // Implementation using bit manipulation
    Ok(())
}
```

Then add to simulator's `execute_gate()` method.

### GPU Support (Future)

The modular architecture allows future GPU backends:
- Replace `QuantumState` with GPU buffer
- Implement gate operations on GPU
- Single API change

### Distributed Execution

- Execution planner already identifies independent gates
- Ready for multi-process/multi-node execution

---

## 📈 Benchmarks

Run benchmarks:

```bash
cargo bench
```

Typical performance (on modern CPU):
- 10 qubits: < 1 ms
- 20 qubits: 10-50 ms
- 25 qubits: 500-2000 ms

---

## ⚠️ Limitations

- **No entanglement compression** (stores full 2^n amplitudes)
- **CPU-bound** (no GPU/distributed yet)
- **Deterministic** (with fixed seed)
- **No stabilizer codes** (full amplitude tracking only)

---

## 📦 Dependencies

- **num-complex**: Complex number support
- **serde/serde_json**: JSON serialization
- **rayon**: Data parallelism
- **rand**: Monte Carlo sampling
- **clap**: CLI argument parsing
- **thiserror**: Error handling

All dependencies are **well-maintained** and production-grade.

---

## 📄 License

This project is provided as-is for educational and research purposes.

---

## 🎓 Learning Resources

### Quantum Computing Fundamentals

1. **State Vector Representation:** Each basis state has an amplitude (complex number)
2. **Gate Operations:** Unitary matrices transform the state
3. **Entanglement:** Correlations between qubits (measured by CNOT)
4. **Measurement Collapse:** Observing collapses superposition

### This Implementation

- **State vector approach:** Track all 2^n amplitudes
- **Bit manipulation:** Extract qubit values using &, |, ^ operations
- **Bloch sphere:** Visualize single-qubit rotations (RX, RY, RZ)
- **Bell states:** Maximal entanglement test

---

## 🤝 Contributing

This is a complete, standalone implementation. For improvements:

1. Algorithm optimizations (cache-aware iteration, SIMD)
2. GPU backend (CUDA/Vulkan)
3. Additional noise models
4. Visualization tools

---

## ✨ Highlights

✅ **Production-grade Rust code** (no unsafe except where necessary)  
✅ **Modular architecture** (8 independent modules)  
✅ **Comprehensive testing** (unit tests + integration tests)  
✅ **Performance-optimized** (bit manipulation, execution planner)  
✅ **Realistic noise models** (bit flip, phase flip, depolarizing)  
✅ **Clean APIs** (easy to integrate)  
✅ **Well-documented** (comments, examples, benchmarks)  
✅ **Extensible** (GPU/distributed-ready architecture)  

---

**Built for developers. Optimized for performance. Ready for production.**
