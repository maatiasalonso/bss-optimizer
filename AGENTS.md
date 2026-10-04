# AGENTS.md — Agent & LLM Contributor Guidelines

This document provides instructions, operational rules, and architectural invariants for autonomous AI coding agents (and human contributors) working in the `bss-optimizer` repository.

---

## 1. Project Mission & Identity

`bss-optimizer` is a high-performance, State-of-the-Art (SOTA) Operations Research and Districting Engine written in **Rust**. It solves the districting, hub-location, and rebalancing problem for large-scale bicycle-sharing systems using real telemetry from **Mexico City ECOBICI (452 stations)**.

The project is a clean, modern re-architecture of the academic research presented by **Cabrera-Guerrero et al. (2022)** (*Mathematics* 10(22), 4175). It replaces a legacy 2017–2020 Julia/Gurobi implementation that took 40–120 minutes with a **< 1-second, zero-license (HiGHS), multithreaded (Rayon), memory-safe Rust engine**.

---

## 2. Fundamental Architectural Invariants (Non-Negotiable)

When proposing or writing code, every AI agent MUST preserve the following architecture:

### A. Hexagonal Architecture (Ports & Adapters)
* **Domain Layer (`src/domain/`)**: Pure domain logic and entities. **Zero dependencies** on external crates, I/O, database drivers, or CLI libraries.
* **Application Layer (`src/application/`)**: Use cases and decoupled port interfaces (`ports/`). All coordination passes through `AssignmentSolverPort`, `SpatialClustererPort`, and `RunRepositoryPort`.
* **Infrastructure Layer (`src/infrastructure/`)**: Concrete technological adapters (HiGHS solver adapter, Balanced K-Means, SQLite repository, GeoJSON exporter).
* **Presentation Layer (`src/presentation/`)**: CLI commands (`clap`) and terminal visualizations (`comfy-table`, `indicatif`).

```
Presentation (CLI / UI)
       │
       ▼
Application (Use Cases)  ──►  Ports (Interfaces)
       │                              ▲
       ▼                              │
Domain (Pure Business Logic)   Infrastructure (HiGHS / SQLite / GeoJSON)
```

### B. Strict Project Governance
The following lints in `Cargo.toml` are strictly enforced by the compiler:
```toml
[lints.rust]
unsafe_code = "forbid"
missing_debug_implementations = "warn"
unreachable_code = "deny"
unused_variables = "deny"
dead_code = "deny"

[lints.clippy]
pedantic = "warn"
all = "warn"
```
* **NEVER** use `unsafe` code under any circumstances.
* **NEVER** leave dead code, unused imports, or unhandled compiler warnings.
* Always run `cargo check` and `cargo test` before submitting changes.

---

## 3. Domain Rules & Primitive Types

To prevent the "Primitive Obsession" code smell, the codebase uses strongly-typed domain primitives:

| Domain Type | Inner Type | Semantics & Rules |
|---|---|---|
| `StationIndex` | `usize` | 0-based internal vector index ($0 \le i < 452$). Used for array lookups and matrices. |
| `StationId` | `usize` | 1-based real-world ECOBICI station ID ($1 \le \text{id} \le 452$). Used for receipts, UI, and GeoJSON. |
| `Meters` | `u32` | Metric distance. Cannot be confused with counts or floats. |
| `NetFlow` | `i32` | Station net bike flow ($\text{inflow} - \text{outflow}$). Formatted with sign indicators (`+12`, `-45`). |
| `PriorityTier` | `enum` | Criticality tier (Tier1 through Tier4). |
| `ReachabilityMap` | Sparse Map | Pre-indexes which stations are within $d_{\max}$ for each station. |
| `PriorityBuckets` | Sparse Buckets | Pre-indexes station indices by priority tier for $O(1)$ constraint construction. |

### Forbidden Patterns
* ❌ **DO NOT** use 3-level dense nested loops (`for l in ... for k in ... for i in ...`). Always use `ReachabilityMap` and `PriorityBuckets`.
* ❌ **DO NOT** hardcode $K=15$ districts anywhere in the logic. Always use dynamic $K$.
* ❌ **DO NOT** allocate full $N \times N$ dense decision matrices inside inner optimization loops.
* ❌ **DO NOT** import `rusqlite` or `good_lp` into `domain` or `application`.

---

## 4. Standard Commands Cheatsheet

Always use these exact shell commands in your verification steps:

```bash
# Check compilation without warnings
cargo check

# Run all automated integration tests
cargo test

# Build optimized release binary
cargo build --release

# Run single optimization (e.g., K=12, 30 iterations)
./target/release/bss-optimizer run -k 12 --iterations 30 --method balanced

# Run parametric fleet sweep across multiple K values
./target/release/bss-optimizer sweep -k 8,10,12,15,18 --iterations 20

# Run historical analytics & strategic hubs report
./target/release/bss-optimizer analyze

# List past runs in SQLite
./target/release/bss-optimizer list

# Run comparative benchmark across algorithms
./target/release/bss-optimizer benchmark --runs 3 --iterations 20
```

---

## 5. Adding New Capabilities (Extension Guide)

### Adding a New Spatial Clustering Strategy
1. Implement the `SpatialClustererPort` trait in `src/infrastructure/clustering/`:
   ```rust
   pub trait SpatialClustererPort: Send + Sync + std::fmt::Debug {
       fn algorithm_name(&self) -> &'static str;
       fn partition(&self, stations: &[Station], k: usize, seed: u64) -> Vec<Vec<StationIndex>>;
   }
   ```
2. Register the new strategy in `MethodArg` in `src/presentation/cli.rs`.
3. Inject the new adapter in `src/main.rs`.

### Adding a New Assignment Solver
1. Implement the `AssignmentSolverPort` trait in `src/infrastructure/solver/`:
   ```rust
   pub trait AssignmentSolverPort: Send + Sync + std::fmt::Debug {
       fn solve(
           &self,
           hubs: &[StationIndex],
           balance_ratio: f64,
           priority_tolerance: i32,
           all_priorities: bool,
       ) -> Option<DistrictingSolution>;
   }
   ```
2. Solvers MUST be thread-safe (`Send + Sync`) to allow parallel evaluations in Rayon.

---

## 6. Git Hygiene for Agents
* **Do NOT commit** `districting.db` or individual `receipts/receipt_run_*.json` files. These are local runtime artifacts.
* Keep commit messages conventional: `feat: ...`, `fix: ...`, `docs: ...`, `refactor: ...`, `test: ...`.
* Preserve all existing comments, docstrings, and architectural boundaries.
