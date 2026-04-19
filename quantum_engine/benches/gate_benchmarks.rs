use criterion::{black_box, criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use quantum_engine::{gates, noise, state::QuantumState};
use quantum_engine::circuit::NoiseConfig;
use quantum_engine::simulator::{SimulationConfig, Simulator};
use quantum_engine::{analysis, Circuit};
use rand::{rngs::StdRng, SeedableRng};

fn benchmark_gate_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("gates");
    group.sample_size(20);

    for num_qubits in [5, 10, 15, 20].iter() {
        let template = QuantumState::new(*num_qubits).unwrap();

        group.bench_with_input(
            BenchmarkId::new("hadamard", num_qubits),
            num_qubits,
            |b, _| {
                b.iter_batched(
                    || template.clone(),
                    |mut state| gates::gate_h(black_box(&mut state), 0).unwrap(),
                    BatchSize::SmallInput,
                );
            },
        );

        group.bench_with_input(
            BenchmarkId::new("pauli_x", num_qubits),
            num_qubits,
            |b, _| {
                b.iter_batched(
                    || template.clone(),
                    |mut state| gates::gate_x(black_box(&mut state), 0).unwrap(),
                    BatchSize::SmallInput,
                );
            },
        );

        group.bench_with_input(
            BenchmarkId::new("rx", num_qubits),
            num_qubits,
            |b, _| {
                b.iter_batched(
                    || template.clone(),
                    |mut state| {
                        gates::gate_rx(black_box(&mut state), 0, std::f64::consts::PI / 4.0)
                            .unwrap()
                    },
                    BatchSize::SmallInput,
                );
            },
        );

        if *num_qubits <= 15 {
            group.bench_with_input(
                BenchmarkId::new("cnot", num_qubits),
                num_qubits,
                |b, _| {
                    b.iter_batched(
                        || template.clone(),
                        |mut state| gates::gate_cnot(black_box(&mut state), 0, 1).unwrap(),
                        BatchSize::SmallInput,
                    );
                },
            );

            group.bench_with_input(BenchmarkId::new("cz", num_qubits), num_qubits, |b, _| {
                b.iter_batched(
                    || template.clone(),
                    |mut state| gates::gate_cz(black_box(&mut state), 0, 1).unwrap(),
                    BatchSize::SmallInput,
                );
            });
        }

        if *num_qubits >= 3 && *num_qubits <= 15 {
            group.bench_with_input(BenchmarkId::new("ccx", num_qubits), num_qubits, |b, _| {
                b.iter_batched(
                    || template.clone(),
                    |mut state| gates::gate_ccx(black_box(&mut state), 0, 1, 2).unwrap(),
                    BatchSize::SmallInput,
                );
            });
        }
    }

    group.finish();
}

fn benchmark_state_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("state");
    group.sample_size(20);

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

fn benchmark_noise_channels(c: &mut Criterion) {
    let mut group = c.benchmark_group("noise");
    group.sample_size(20);

    for num_qubits in [5, 10, 15].iter() {
        let state_template = QuantumState::new(*num_qubits).unwrap();

        let composite_noise = NoiseConfig {
            noise_type: "COMPOSITE".to_string(),
            probability: 1.0,
            channels: Some(vec![
                NoiseConfig {
                    noise_type: "BIT_FLIP".to_string(),
                    probability: 0.01,
                    channels: None,
                    t1: None,
                    t2: None,
                    dt: None,
                    angle: None,
                    axis: None,
                    coupled_qubits: None,
                    kraus: None,
                },
                NoiseConfig {
                    noise_type: "PHASE_FLIP".to_string(),
                    probability: 0.01,
                    channels: None,
                    t1: None,
                    t2: None,
                    dt: None,
                    angle: None,
                    axis: None,
                    coupled_qubits: None,
                    kraus: None,
                },
            ]),
            t1: None,
            t2: None,
            dt: None,
            angle: None,
            axis: None,
            coupled_qubits: None,
            kraus: None,
        };

        group.bench_with_input(
            BenchmarkId::new("composite_apply", num_qubits),
            num_qubits,
            |b, _| {
                b.iter_batched(
                    || (state_template.clone(), StdRng::seed_from_u64(42)),
                    |(mut state, mut rng)| {
                        noise::apply_noise_with_rng(
                            black_box(&mut state),
                            0,
                            &composite_noise,
                            &mut rng,
                        )
                        .unwrap();
                    },
                    BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

fn benchmark_analysis(c: &mut Criterion) {
    let mut group = c.benchmark_group("analysis");
    group.sample_size(20);

    let mut bell = QuantumState::new(2).unwrap();
    gates::gate_h(&mut bell, 0).unwrap();
    gates::gate_cnot(&mut bell, 0, 1).unwrap();

    group.bench_function("expectation_zz", |b| {
        b.iter(|| analysis::expectation_pauli_string(black_box(&bell), "ZZ").unwrap());
    });

    group.bench_function("reduced_density_single", |b| {
        b.iter(|| analysis::reduced_density_matrix(black_box(&bell), &[0]).unwrap());
    });

    let sim_config = SimulationConfig {
        shots: 1024,
        optimize: true,
        apply_noise: true,
        seed: 42,
    };
    let sim = Simulator::new(sim_config);
    let readout_circuit = Circuit::from_json(
        r#"{
        "qubits": 1,
        "readout_noise": {"noise_type": "READOUT_ERROR", "probability": 0.1},
        "gates": [{"gate_type": "H", "target": 0}]
    }"#,
    )
    .unwrap();

    group.bench_function("simulate_readout_noise", |b| {
        b.iter(|| sim.run(black_box(&readout_circuit)).unwrap());
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_gate_operations,
    benchmark_state_operations,
    benchmark_full_circuit,
    benchmark_noise_channels,
    benchmark_analysis
);
criterion_main!(benches);
