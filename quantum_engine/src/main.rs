//! CLI tool for quantum circuit simulation

use clap::{Parser, Subcommand, ValueEnum};
use quantum_engine::{Circuit, Simulator};
use quantum_engine::simulator::SimulationConfig;
use quantum_engine::simulator::SimulationResult;
use std::fs;

#[derive(Parser)]
#[command(name = "Quantum Engine CLI")]
#[command(about = "High-performance quantum circuit simulator", long_about = None)]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(global = true, short, long)]
    verbose: bool,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum OutputFormat {
    /// Human-readable report.
    Pretty,
    /// JSON payload for automation.
    Json,
}

#[derive(Subcommand)]
enum Commands {
    /// Simulate a circuit from JSON file
    Simulate {
        /// Path to circuit JSON file
        #[arg(value_name = "FILE")]
        circuit: String,

        /// Number of measurement shots
        #[arg(short, long, default_value = "1024")]
        shots: usize,

        /// Disable circuit optimization
        #[arg(long)]
        no_optimize: bool,

        /// Disable noise injection
        #[arg(long)]
        no_noise: bool,

        /// Random seed (0 = random seed)
        #[arg(long, default_value = "0")]
        seed: u64,

        /// Output format for stdout and output files
        #[arg(long, value_enum, default_value = "pretty")]
        format: OutputFormat,

        /// Output file (default: stdout)
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Validate a circuit file
    Validate {
        /// Path to circuit JSON file
        #[arg(value_name = "FILE")]
        circuit: String,
    },

    /// Print example circuits
    Examples,
}

fn main() {
    let cli = Cli::parse();

    // Setup logging
    if cli.verbose {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Debug)
            .init();
    } else {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Info)
            .init();
    }

    match cli.command {
        Commands::Simulate {
            circuit,
            shots,
            no_optimize,
            no_noise,
            seed,
            format,
            output,
        } => {
            simulate_circuit(&circuit, shots, !no_optimize, !no_noise, seed, format, output);
        }
        Commands::Validate { circuit } => {
            validate_circuit(&circuit);
        }
        Commands::Examples => {
            print_examples();
        }
    }
}

fn simulate_circuit(
    circuit_file: &str,
    shots: usize,
    optimize: bool,
    apply_noise: bool,
    seed: u64,
    format: OutputFormat,
    output_file: Option<String>,
) {
    if matches!(format, OutputFormat::Pretty) {
        println!("Loading circuit from: {}", circuit_file);
    }

    let circuit_json = match fs::read_to_string(circuit_file) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error reading circuit file: {}", e);
            return;
        }
    };

    let circuit = match Circuit::from_json(&circuit_json) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Circuit validation error: {}", e);
            return;
        }
    };

    if matches!(format, OutputFormat::Pretty) {
        println!("Circuit: {} qubits, {} gates", circuit.qubits, circuit.gates.len());
    }

    let config = SimulationConfig {
        shots,
        optimize,
        apply_noise,
        seed,
    };

    let simulator = Simulator::new(config);
    let result = match simulator.run(&circuit) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Simulation error: {}", e);
            return;
        }
    };

    print_simulation_result(&result, format);

    // Write to file if specified
    if let Some(output) = output_file {
        match serialize_simulation_result(&result, format) {
            Ok(payload) => {
                if let Err(e) = fs::write(&output, payload) {
                    eprintln!("Error writing output file: {}", e);
                } else {
                    println!("\nResults written to: {}", output);
                }
            }
            Err(e) => {
                eprintln!("Failed to serialize results: {}", e);
            }
        }
    }
}

fn serialize_simulation_result(result: &SimulationResult, format: OutputFormat) -> Result<String, serde_json::Error> {
    match format {
        OutputFormat::Pretty => serde_json::to_string_pretty(result),
        OutputFormat::Json => serde_json::to_string(result),
    }
}

fn print_simulation_result(result: &SimulationResult, format: OutputFormat) {
    match format {
        OutputFormat::Pretty => {
            println!("\n=== Simulation Results ===");
            println!("Execution time: {:.2} ms", result.execution_time_ms);
            println!("Circuit depth: {}", result.circuit_depth);
            println!("Total gates: {}", result.circuit_gates);
            println!("Two-qubit gates: {}", result.two_qubit_gates);
            println!("\nMeasurement results ({} shots):", result.measurement.shots);
            println!("Probabilities:");

            for (state, prob) in &result.measurement.probabilities {
                println!(
                    "  {}: {:.4} ({} counts)",
                    state,
                    prob,
                    result.measurement.counts.get(state).unwrap_or(&0)
                );
            }

            if let Some(most_likely) = result.measurement.most_likely_state() {
                println!(
                    "\nMost likely state: {} (probability: {:.4})",
                    most_likely.0,
                    most_likely.1
                );
            }
        }
        OutputFormat::Json => match serde_json::to_string(result) {
            Ok(json) => println!("{}", json),
            Err(e) => eprintln!("Failed to serialize result as JSON: {}", e),
        },
    }
}

fn validate_circuit(circuit_file: &str) {
    println!("Validating circuit: {}", circuit_file);

    let circuit_json = match fs::read_to_string(circuit_file) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error reading circuit file: {}", e);
            return;
        }
    };

    match Circuit::from_json(&circuit_json) {
        Ok(circuit) => {
            println!("✓ Circuit is valid");
            println!("  Qubits: {}", circuit.qubits);
            println!("  Gates: {}", circuit.gates.len());
            for (i, gate) in circuit.gates.iter().enumerate() {
                println!("    {}: {} (target: {:?}, control: {:?})", i, gate.gate_type, gate.target, gate.control);
            }
        }
        Err(e) => {
            eprintln!("✗ Circuit validation failed: {}", e);
        }
    }
}

fn print_examples() {
    println!("=== Example Circuits ===\n");

    println!("1. Bell State (Entanglement):");
    println!(r#"{{"qubits": 2, "gates": [{{"gate_type": "H", "target": 0}}, {{"gate_type": "CNOT", "control": 0, "target": 1}}]}}"#);
    println!();

    println!("2. Hadamard Superposition:");
    println!(r#"{{"qubits": 1, "gates": [{{"gate_type": "H", "target": 0}}]}}"#);
    println!();

    println!("3. Interference (H → H = Identity):");
    println!(r#"{{"qubits": 1, "gates": [{{"gate_type": "H", "target": 0}}, {{"gate_type": "H", "target": 0}}]}}"#);
    println!();

    println!("4. Three-Qubit GHZ State:");
    println!(r#"{{"qubits": 3, "gates": [{{"gate_type": "H", "target": 0}}, {{"gate_type": "CNOT", "control": 0, "target": 1}}, {{"gate_type": "CNOT", "control": 1, "target": 2}}]}}"#);
    println!();

    println!("5. Quantum Fourier Transform (2 qubits):");
    println!(r#"{{"qubits": 2, "gates": [{{"gate_type": "H", "target": 0}}, {{"gate_type": "RZ", "target": 0, "parameter": 0.785398}}, {{"gate_type": "H", "target": 1}}, {{"gate_type": "SWAP", "control": 0, "target": 1}}]}}"#);
    println!();

    println!("6. Advanced noise chain (see examples/advanced_noise.json):");
    println!(r#"{{"qubits": 2, "global_noise": {{"noise_type": "COMPOSITE", "probability": 1.0, "channels": [...] }}, "gates": [...]}}"#);
    println!();

    println!("7. Conditional + repeat + reset (see examples/conditional_repeat.json):");
    println!(r#"{{"qubits": 2, "classical_bits": 2, "gates": [{{"gate_type": "MEASURE", "target": 0, "cbit": 1}}, {{"gate_type": "X", "target": 1, "condition": {{"register": 1, "value": true}}, "repeat": 1}}, {{"gate_type": "RESET", "target": 0}}]}}"#);
    println!();

    println!("8. Readout noise model (see examples/readout_noise.json):");
    println!(r#"{{"qubits": 1, "readout_noise": {{"noise_type": "READOUT_ERROR", "probability": 0.1}}, "gates": [{{"gate_type": "H", "target": 0}}]}}"#);
}
