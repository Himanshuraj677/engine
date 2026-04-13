# Integration & Extension Guide

## 🔗 Using the Engine in Your Project

### As a Rust Library

Add to your `Cargo.toml`:
```toml
[dependencies]
quantum_engine = { path = "../../qs-scratch/quantum_engine" }
```

Basic usage:
```rust
use quantum_engine::{Circuit, Simulator, SimulationConfig};

fn main() -> quantum_engine::error::Result<()> {
    // Create circuit
    let circuit = Circuit::from_json(r#"{
        "qubits": 2,
        "gates": [
            {"gate_type": "H", "target": 0},
            {"gate_type": "CNOT", "control": 0, "target": 1}
        ]
    }"#)?;
    
    // Configure simulation
    let config = SimulationConfig {
        shots: 1024,
        optimize: true,
        apply_noise: false,
        seed: 0,
    };
    
    // Run simulation
    let simulator = Simulator::new(config);
    let result = simulator.run(&circuit)?;
    
    // Access results
    println!("Most likely: {:?}", result.measurement.most_likely_state());
    println!("Execution time: {:.2} ms", result.execution_time_ms);
    
    Ok(())
}
```

---

## 🔧 Adding New Features

### 1. Adding a New Gate

**Step 1**: Implement in `src/gates.rs`
```rust
pub fn gate_my_gate(state: &mut QuantumState, target: usize) -> Result<()> {
    validate_qubit(target, state.num_qubits())?;
    
    let mask = 1 << target;
    let amplitudes = state.amplitudes_mut();
    
    // Implement gate logic here...
    
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_my_gate() {
        // Test here
    }
}
```

**Step 2**: Add to `execute_gate()` in `src/simulator.rs`
```rust
"MYGATE" => {
    let target = gate.target.ok_or_else(|| Error::SimulationError {
        reason: "MY_GATE requires target".to_string(),
    })?;
    gates::gate_my_gate(state, target)?;
}
```

**Step 3**: Update circuit validation in `src/circuit.rs`
```rust
"MYGATE" => {
    if gate.target.is_none() {
        return Err(Error::CircuitValidationError {
            reason: "MYGATE requires 'target'".to_string(),
        });
    }
    // ... validation logic
}
```

**Step 4**: Test with JSON
```json
{
  "qubits": 1,
  "gates": [
    {"gate_type": "MYGATE", "target": 0}
  ]
}
```

---

### 2. Adding a New Noise Model

**Step 1**: Implement in `src/noise.rs`
```rust
pub fn my_noise_model(state: &mut QuantumState, target: usize, param: f64) -> Result<()> {
    if !(0.0..=1.0).contains(&param) {
        return Err(Error::InvalidNoiseConfig {
            reason: format!("Parameter must be in [0, 1], got {}", param),
        });
    }
    
    // Implement noise model
    
    Ok(())
}
```

**Step 2**: Add to `apply_noise()` dispatcher
```rust
"MY_NOISE_MODEL" => my_noise_model(state, target, noise.probability),
```

**Step 3**: Update circuit validation in `src/circuit.rs`
```rust
if !matches!(noise_type.as_str(), 
    "BIT_FLIP" | "PHASE_FLIP" | "DEPOLARIZING" | "MY_NOISE_MODEL") {
    return Err(...);
}
```

**Step 4**: Use in circuit
```json
{
  "qubits": 1,
  "gates": [
    {
      "gate_type": "H",
      "target": 0,
      "noise": {
        "noise_type": "my_noise_model",
        "probability": 0.01
      }
    }
  ]
}
```

---

### 3. Custom Simulation Analysis

```rust
use quantum_engine::state::QuantumState;

let mut state = QuantumState::new(2)?;

// Custom analysis
for i in 0..state.size() {
    let prob = state.probability(i)?;
    if prob > 1e-6 {
        println!("State {}: prob = {}", i, prob);
    }
}

// Marginal distributions
let marginal_q0 = state.marginal_distribution(0)?;
println!("Qubit 0: P(0)={}, P(1)={}", marginal_q0[0], marginal_q0[1]);
```

---

## 🚀 Deployment Options

### 1. CLI Tool (Ready Now)
```bash
./quantum_engine simulate circuit.json --shots 1000
```

### 2. Rest API (Future)
```rust
// Add actix-web dependency
use actix_web::{post, web, App, HttpServer};

#[post("/simulate")]
async fn simulate(circuit: web::Json<Circuit>) -> web::Json<SimulationResult> {
    let simulator = Simulator::new(SimulationConfig::default());
    let result = simulator.run(&circuit).unwrap();
    web::Json(result)
}
```

### 3. Python Binding (Future)
```rust
use pyo3::prelude::*;

#[pyfunction]
fn simulate_py(circuit_json: String, shots: usize) -> PyResult<String> {
    let circuit = Circuit::from_json(&circuit_json)?;
    let config = SimulationConfig {
        shots,
        ..Default::default()
    };
    let simulator = Simulator::new(config);
    let result = simulator.run(&circuit)?;
    Ok(result.measurement.to_json()?)
}

#[pymodule]
fn quantum_engine(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(simulate_py, m)?)?;
    Ok(())
}
```

### 4. WebAssembly (Future)
```rust
// Add wasm-bindgen, wasm-pack
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn simulate_wasm(circuit_json: &str, shots: usize) -> Result<JsValue, JsValue> {
    let circuit = Circuit::from_json(circuit_json)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    
    let simulator = Simulator::new(SimulationConfig {
        shots,
        ..Default::default()
    });
    
    let result = simulator.run(&circuit)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    
    Ok(serde_wasm_bindgen::to_value(&result.measurement)?)
}
```

---

## 📊 Performance Tuning

### 1. Optimize Circuit Before Running
```rust
let optimized = optimizer::optimize_circuit(&circuit)?;
let result = simulator.run(&optimized)?;
```

### 2. Use No-shot Mode for Probabilities
```rust
let config = SimulationConfig {
    shots: 0,  // Get exact probabilities, no sampling
    optimize: true,
    apply_noise: false,
    seed: 0,
};
```

### 3. Disable Optimization for Debugging
```rust
let config = SimulationConfig {
    shots: 1024,
    optimize: false,  // See raw execution order
    apply_noise: false,
    seed: 0,
};
```

### 4. Monitor Execution Plan
```rust
use quantum_engine::runtime::ExecutionPlan;

let plan = ExecutionPlan::from_circuit(&circuit)?;
println!("Layers: {}", plan.num_layers());
println!("Parallelism: {:.2}x", plan.parallelism_factor());
```

---

## 🔬 Testing Your Extensions

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_my_feature() {
        let circuit = Circuit::from_json(r#"{...}"#).unwrap();
        let simulator = Simulator::new(SimulationConfig::default());
        let result = simulator.run(&circuit).unwrap();
        
        assert!(result.measurement.probabilities.len() > 0);
    }
}
```

Run tests:
```bash
cargo test --lib
```

---

## 📈 Benchmarking Your Code

```rust
// In benches/my_benchmarks.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_my_circuit(c: &mut Criterion) {
    c.bench_function("my_circuit", |b| {
        let circuit = Circuit::from_json(/* ... */).unwrap();
        let simulator = Simulator::new(Default::default());
        
        b.iter(|| simulator.run(black_box(&circuit)))
    });
}

criterion_group!(benches, bench_my_circuit);
criterion_main!(benches);
```

Run benchmarks:
```bash
cargo bench --bench my_benchmarks
```

---

## 🐛 Debugging Tips

### 1. Enable Debug Logging
```bash
RUST_LOG=debug cargo run --release -- simulate circuit.json
```

### 2. Inspect State
```rust
println!("Probabilities: {:?}", state.probabilities());
println!("Marginal: {:?}", state.marginal_distribution(0)?);
```

### 3. Validate Circuit
```bash
cargo run --release -- validate circuit.json
```

### 4. Test Individual Gates
```rust
let mut state = QuantumState::new(2)?;
gates::gate_h(&mut state, 0)?;
println!("After H: {:?}", state.probabilities());
```

---

## 🤝 Contributing Back

If you improve the engine:

1. **Add tests** for new features
2. **Update documentation** in code comments
3. **Run full test suite**: `cargo test`
4. **Check performance**: `cargo bench`
5. **Maintain style**: Use `cargo fmt`
6. **Check clippy**: `cargo clippy`

---

## 📚 Resources

- **Quantum simulation**: IBM Qiskit docs
- **Rust async/parallel**: Rayon docs
- **Numerical computing**: ndarray docs
- **JSON serialization**: serde docs

---

## 🎓 Example: VQE (Variational Quantum Eigensolver)

```rust
use quantum_engine::{Circuit, Simulator, SimulationConfig};

fn vqe_objective(theta: f64) -> f64 {
    let circuit = Circuit::from_json(&format!(r#"{{
        "qubits": 1,
        "gates": [
            {{"gate_type": "RY", "target": 0, "parameter": {}}}
        ]
    }}"#, theta)).unwrap();
    
    let simulator = Simulator::new(SimulationConfig::default());
    let result = simulator.run(&circuit).unwrap();
    
    // Compute <Z>
    result.measurement.expected_value_z(0)
}

fn main() {
    // Optimize theta
    for theta in (0..100).map(|i| i as f64 * 0.1) {
        let value = vqe_objective(theta);
        println!("θ={:.2}, <Z>={:.4}", theta, value);
    }
}
```

---

## 🚀 Ready to Extend?

1. Start small: Add one gate or noise model
2. Write tests: Verify correctness
3. Benchmark: Measure performance
4. Integrate: Use in your application
5. Scale: Deploy with your backend

**Documentation**: See `quantum_engine/README.md` for API details.

**Questions**: Review the source code—it's well-commented!

**Ready for production**: The engine is fully tested and optimized.

---

**Happy quantum computing! 🚀**
