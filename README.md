<img src="assets/qlean-banner.jpg" />

# Qlean

A semantic static analyzer for Q# that detects quantum-state hygiene bugs Microsoft's `qsc_linter` does not cover.

## Problem Statement

Microsoft's built-in `qsc_linter` is a pattern-based linter that catches syntax and HIR code smells such as `needlessParens`, `divisionByZero`, and `NeedlessOperation`. It does not track the lifetime of a qubit across an operation.

This leaves critical quantum correctness bugs undetected:

1. Ancilla leaks: a qubit allocated with `use` is never reset or released cleanly before an operation returns.
2. Missing uncomputation: a reversible operation is applied into ancilla qubits, but the inverse is not applied before release, leaving the quantum state entangled with garbage.
3. Unintended mid-circuit entanglement: a measurement result is used to conditionally control operations on other qubits, introducing classical correlation across registers.

Qlean bridges this gap by performing a semantic dataflow pass over the Q# High-level Intermediate Representation (HIR).

## Comparison to `qsc_linter`

| Capability | Microsoft `qsc_linter` | Qlean |
|---|---|---|
| AST pattern smells | Yes | Optional via `--include-ast` |
| Qubit lifetime tracking | No | Yes |
| Ancilla leak detection (`ancilla-leak`) | No | Yes |
| Missing uncompute detection (`missing-uncompute`) | No | Yes |
| Mid-circuit entanglement check (`mid-circuit-entanglement`) | No | Yes |
| Dual native and WebAssembly support | CLI / VS Code | CLI and WebAssembly |

## Architecture

Qlean reuses the official Microsoft Q# compiler frontend:

- `qsc_frontend` for parsing and lowering Q# source code
- `qsc_hir` for traversing the typed High-level Intermediate Representation
- `LifetimeTracker` dataflow pass for tracking allocations, state transitions, and release points per qubit
- `check_uncompute` analyzer for verifying reversible subroutine cancellation

## Installation and Build

### Prerequisites

- Rust 1.80 or higher (Cargo)

### Build Native CLI

```bash
cargo build --release
```

The resulting binary will be in `target/release/qlean`.

### Run Tests

```bash
cargo test
```

## CLI Usage

### Parse Operations

Print all operations and functions defined in a Q# file:

```bash
qlean parse file.qs
```

### Trace Qubit Lifetimes

Inspect the lifetime timeline and transitions of every tracked qubit:

```bash
qlean trace file.qs
```

### Run Static Analysis (Lint)

Run semantic quantum hygiene passes and report diagnostics in human-readable format:

```bash
qlean lint file.qs
```

To include Microsoft's built-in AST and syntax pattern lints in the same report:

```bash
qlean lint file.qs --include-ast
```

Or report in JSON or SARIF format:

```bash
qlean lint file.qs --format json
qlean lint file.qs --format sarif
```

You can also pass a directory to recursively check all `.qs` files:

```bash
qlean lint tests/fixtures/
```

## GitHub Action

You can use Qlean directly as a GitHub Action in other Q# repositories.

### Basic Usage

Add `.github/workflows/qlean.yml` to your repository:

```yaml
name: Q# Static Analysis
on: [push, pull_request]

jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Run Qlean
        uses: Qlean-Group/Qlean@v0.1.0
        with:
          path: '.'
          include-ast: 'true'
```

### GitHub Code Scanning (SARIF) Integration

To display quantum hygiene bugs directly inside Pull Request diff annotations and GitHub Code Scanning alerts:

```yaml
name: Q# Code Scanning
on: [push, pull_request]

jobs:
  scan:
    runs-on: ubuntu-latest
    permissions:
      security-events: write
    steps:
      - uses: actions/checkout@v4
      - name: Run Qlean (SARIF)
        uses: Qlean-Group/Qlean@v0.1.0
        with:
          path: '.'
          format: 'sarif'
          output-file: 'qlean-results.sarif'
      - name: Upload SARIF
        uses: github/codeql-action/upload-sarif@v3
        with:
          sarif_file: 'qlean-results.sarif'
```

## Detected Rules

### `ancilla-leak`

Flags any qubit allocated with `use` that reaches its release point while in a dirty or measured state without being reset.

Example bad code:

```qsharp
namespace Example {
    operation Leak() : Unit {
        use q = Qubit();
        X(q);
    }
}
```

Diagnostic:

```text
file.qs:3:9: warning: qubit 'q' allocated but never reset before release [ancilla-leak]
```

### `missing-uncompute`

Flags reversible entangling gates (such as CNOT fan-out) applied to an ancilla qubit where the matching inverse operation is not applied before the ancilla is reset or released.

Example bad code:

```qsharp
namespace Example {
    operation Fanout(target : Qubit) : Unit {
        use anc = Qubit();
        CNOT(target, anc);
        Reset(anc);
    }
}
```

Diagnostic:

```text
file.qs:4:9: warning: qubit 'anc' has unreversed gate 'CNOT' leaving state entangled before release [missing-uncompute]
```

### `mid-circuit-entanglement`

Emits a warning when a measurement result is used in conditional branches or expressions that control gates on subsequent qubits.

Example code:

```qsharp
namespace Example {
    operation Feedforward() : Unit {
        use q1 = Qubit();
        use q2 = Qubit();
        let res = M(q1);
        if res == One {
            X(q2);
        }
        Reset(q1);
        Reset(q2);
    }
}
```

Diagnostic:

```text
file.qs:7:13: warning: measurement of qubit 'q1' conditionally controls operations on qubit 'q2' [mid-circuit-entanglement]
```

## WebAssembly and Demo

A browser demo is located in `demo/index.html`. It demonstrates client-side diagnostic reporting and interactive inspection of Q# code.

## License

Licensed under either of:

- Apache License, Version 2.0 (LICENSE-APACHE)
- MIT License (LICENSE-MIT)
