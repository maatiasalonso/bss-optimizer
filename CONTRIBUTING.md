# Contributing to BSS Districting Optimizer

Thank you for your interest in contributing to **BSS Districting Optimizer**! This project is an open-source, State-of-the-Art (SOTA) logistics and districting optimization engine for bike-sharing networks, written in pure Rust with Clean Architecture.

Whether you are fixing a bug, adding a new heuristic/metaheuristic, enhancing spatial clustering, or expanding documentation, we welcome your contributions.

---

## 1. Development Principles & Code Standards

To keep the codebase maintainable, highly concurrent, and production-grade, all contributions must respect our core engineering standards:

1. **Clean / Hexagonal Architecture**:
   - Keep business logic in `domain/` completely isolated from external dependencies.
   - Use `application/ports/` to decouple use cases from specific algorithms, solvers, and databases.
   - New solvers or clusterers must be implemented as infrastructure adapters.
2. **Strict Compiler Governance**:
   - `unsafe_code = "forbid"` is non-negotiable. 100% memory safety.
   - Zero dead code or unhandled warnings allowed (`unused_variables = "deny"`).
   - All code must pass `cargo clippy` and `cargo test`.
3. **Mechanical Sympathy & Sparsity**:
   - Avoid dense matrix allocations in inner search loops.
   - Use sparse lookups (`ReachabilityMap`, `PriorityBuckets`) to keep solver evaluations in the microsecond range.
4. **Open-Source Solver Policy**:
   - We use the open-source **HiGHS** solver (via `good_lp`). Contributions must not introduce mandatory dependencies on proprietary commercial solvers (such as Gurobi or CPLEX).

---

## 2. Setting Up Your Development Environment

### Prerequisites
* **Rust**: 1.80 or later (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
* **Cargo**: Included with Rust
* **Git**: Version control

### Getting the Code
```bash
git clone https://github.com/maatiasalonso/bss-optimizer.git
cd bss-optimizer
cargo check
cargo test
```

---

## 3. Contribution Workflow

1. **Fork and Branch**:
   Create a descriptive branch for your work:
   ```bash
   git checkout -b feat/quantum-annealing-solver
   # or
   git checkout -b fix/geojson-coordinate-ordering
   ```

2. **Develop and Test**:
   Make sure all tests pass and your code is properly formatted:
   ```bash
   cargo fmt --check
   cargo clippy -- -D warnings
   cargo test
   ```

3. **Verify Locally**:
   Run an optimization and check that standard receipts and SQLite tracking behave as expected:
   ```bash
   cargo run --release -- run -k 15 --iterations 30
   cargo run --release -- analyze
   ```

4. **Commit Conventions**:
   Follow Conventional Commits:
   * `feat: add Genetic Algorithm spatial clusterer adapter`
   * `fix: correct bounding box calculation in GeoJSON exporter`
   * `docs: update ARCHITECTURE.md with sequence diagram`
   * `perf: optimize ReachabilityMap bitset lookup`

5. **Submit a Pull Request**:
   Push your branch to your fork and open a Pull Request with a clear description of:
   * What problem this solves.
   * How you tested it.
   * Benchmark metrics (runtime, distance objective, memory impact).

---

## 4. Areas for Contribution

We are actively seeking contributions in the following areas:
* **Interactive Web UI**: Lightweight visualizer using Leaflet or MapLibre to display district polygons and hub star-routes.
* **Additional Metaheuristics**: Adaptive Large Neighborhood Search (ALNS), Simulated Annealing, or Tabu Search adapters.
* **Vehicle Routing (VRP / TSP)**: Intra-district TSP van routing to compute actual rebalancing vehicle driving tours.
* **Dynamic Time-of-Day Districting**: Modeling morning vs evening rush hour demand matrices.
* **New Benchmark Datasets**: Incorporating public Citi Bike (NYC), Divvy (Chicago), or BiciMAD (Madrid) station data.

---

## 5. Code of Conduct

We are committed to providing a welcoming, inclusive, and professional environment for everyone. Please be respectful, constructive, and collaborative in all discussions and pull requests.
