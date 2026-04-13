use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use quantum_engine::{state::QuantumState, gates};

fn benchmark_gate_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("gates");

    for num_qubits in [5, 10, 15, 20].iter() {
        let mut state = QuantumState::new(*num_qubits).unwrap();

        group.bench_with_input(
            BenchmarkId::new("hadamard", num_qubits),
            num_qubits,
            |b, _| {
                b.iter(|| {
                    gates::gate_h(black_box(&mut state), 0).unwrap();
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("pauli_x", num_qubits),
            num_qubits,
            |b, _| {
                b.iter(|| {
                    gates::gate_x(black_box(&mut state), 0).unwrap();
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("rx", num_qubits),
            num_qubits,
            |b, _| {
                b.iter(|| {
                    gates::gate_rx(black_box(&mut state), 0, std::f64::consts::PI / 4.0).unwrap();
                });
            },
        );

        if *num_qubits <= 15 {
            group.bench_with_input(
                BenchmarkId::new("cnot", num_qubits),
                num_qubits,
                |b, _| {
                    b.iter(|| {
                        gates::gate_cnot(black_box(&mut state), 0, 1).unwrap();
                    });
                },
            );
        }
    }

    group.finish();
}

fn benchmark_state_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("state");

    for num_qubits in [5, 10, 15, 20].iter() {
        let state = QuantumState::new(*num_qubits).unwrap();

        group.bench_with_input(
            BenchmarkId::new("probabilities", num_qubits),
            num_qubits,
            |b, _| {
                b.iter(|| {
                    let _ = state.probabilities();
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("normalize", num_qubits),
            num_qubits,
            |b, _| {
                let mut s = state.clone();
                b.iter(|| {
                    s.normalize();
                });
            },
        );
    }

    group.finish();
}

fn benchmark_full_circuit(c: &mut Criterion) {
    use quantum_engine::{state::QuantumState, gates};
    use quantum_engine::simulator::{Simulator, SimulationConfig};
    use quantum_engine::Circuit;

    let mut group = c.benchmark_group("circuits");
    group.sample_size(10); // Reduce sample size for slower benchmarks

    let bell_circuit = r#"{
        "qubits": 2,
        "gates": [
            {"gate_type": "H", "target": 0},
            {"gate_type": "CNOT", "control": 0, "target": 1}
        ]
    }"#;

    group.bench_function("bell_state", |b| {
        let circuit = Circuit::from_json(bell_circuit).unwrap();
        let config = SimulationConfig {
            shots: 1024,
            optimize: true,
            apply_noise: false,
            seed: 0,
        };
        let simulator = Simulator::new(config);

        b.iter(|| {
            simulator.run(black_box(&circuit)).unwrap();
        });
    });

    let ghz_circuit = r#"{
        "qubits": 3,
        "gates": [
            {"gate_type": "H", "target": 0},
            {"gate_type": "CNOT", "control": 0, "target": 1},
            {"gate_type": "CNOT", "control": 1, "target": 2}
        ]
    }"#;

    group.bench_function("ghz_state", |b| {
        let circuit = Circuit::from_json(ghz_circuit).unwrap();
        let config = SimulationConfig {
            shots: 1024,
            optimize: true,
            apply_noise: false,
            seed: 0,
        };
        let simulator = Simulator::new(config);

        b.iter(|| {
            simulator.run(black_box(&circuit)).unwrap();
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_gate_operations,
    benchmark_state_operations,
    benchmark_full_circuit
);
criterion_main!(benches);
