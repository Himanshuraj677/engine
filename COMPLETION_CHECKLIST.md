# Completion Checklist

## Phase 1 - Deterministic RNG plumbing
- [x] RNG call sites audited.
- [x] Seeded RNG flow designed and integrated.
- [x] Deterministic simulation/noise/measurement paths implemented.
- [x] Determinism regression tests added.
- [x] Validation run completed.

## Phase 2 - Gate and circuit model expansion
- [x] Added CZ, CP, CCX, UNITARY/U, RESET, BARRIER.
- [x] Added generic `controls` abstraction and validation.
- [x] Updated runtime/optimizer behavior for new involved-qubit semantics.
- [x] Regression tests for new gate behavior added.

## Phase 3 - Classical registers and control flow
- [x] Added optional `classical_bits` to circuit model.
- [x] Added `cbit` measurement target mapping.
- [x] Added gate `condition` support.
- [x] Added gate `repeat` support.
- [x] Simulator classical-state execution path validated.

## Phase 4 - Analysis toolkit
- [x] Added expectation for Pauli strings.
- [x] Added reduced density matrix computation.
- [x] Added single-qubit entanglement entropy.
- [x] Added fidelity and Bloch vector utilities.
- [x] Exported analysis module via library interface.

## Phase 5 - Advanced and composable noise
- [x] Added advanced channels (T1/T2, coherent, crosstalk, Kraus).
- [x] Added COMPOSITE channel chaining.
- [x] Added circuit-level readout noise support.
- [x] Integrated readout-error behavior in measurement sampling.
- [x] Validation/tests for composite and readout behavior completed.

## Phase 6 - CLI/product usability
- [x] Added `--seed` and `--format` flags.
- [x] Enabled full-result JSON serialization for output files.
- [x] Ensured JSON mode outputs clean machine-readable stdout.
- [x] Added advanced example circuits for new capabilities.

## Phase 7 - Benchmark hardening
- [x] Refactored benchmark setup for stable per-iteration inputs.
- [x] Expanded benchmark coverage to new gates/noise/analysis paths.
- [x] Removed benchmark warning drift and validated bench execution.

## Phase 8 - Documentation alignment
- [x] Updated quick start and delivery summary to current feature set.
- [x] Updated index with current examples and commands.
- [x] Normalized verification numbers to current test status.

## Current status
- [x] Library + CLI compile and test cleanly.
- [x] 51 tests passing.
- [x] Benchmarks compile and run.
