# Bicycle-Sharing System (BSS) Districting Optimizer

[![Rust](https://img.shields.io/badge/rust-1.80%2B-orange.svg?logo=rust)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Architecture: Hexagonal](https://img.shields.io/badge/Architecture-Hexagonal%20%2F%20DDD-green.svg)](ARCHITECTURE.md)
[![Solver: HiGHS](https://img.shields.io/badge/Solver-HiGHS%20(Open%20Source)-purple.svg)](https://highs.dev/)
[![Agents: SOTA](https://img.shields.io/badge/Agents-AI%20Ready-blueviolet.svg)](AGENTS.md)

A production-grade, State-of-the-Art (SOTA) Operations Research and Logistics Districting Engine written in **Rust**. It solves the districting, hub-location, and rebalancing problem for large-scale bicycle-sharing systems using real telemetry from **Mexico City's ECOBICI (452 stations)**.

---

## ⚡ Performance Benchmark: Peer-Reviewed Paper vs Modern Rust

In the peer-reviewed paper **Cabrera-Guerrero et al. (2022)** (*Mathematics* 10(22), 4175), all experiments were benchmarked on an **Intel Core i7-10700 CPU @ 2.9 GHz (8 cores), 16 GB RAM, Ubuntu 20.04**, executing the rVNS matheuristic in **Julia v1.2 + JuMP + Gurobi 7.5**. 

The published paper reports that rVNS execution time generally required up to **2,500 seconds (~41.6 minutes)** per experiment, with difficult parameter configurations exceeding the **10,000-second (~2.77 hours)** time-limit threshold.

| Implementation | Environment & Hardware | Solver & Licensing | Concurrency Model | Published / Verified Runtime | Best FO (Distance) |
|---|---|---|---|---|---|
| **Exact MIP Model (Paper 2022)** | Intel i7-10700 (8 cores, Ubuntu) | Gurobi 7.5 (Proprietary) | Solver branch-and-cut | **Up to 10,000 s (~2.77 hrs)** | Optimal on small instances |
| **rVNS Matheuristic (Paper 2022)** | Intel i7-10700 (8 cores, Ubuntu) | Julia v1.2 + Gurobi 7.5 | Julia Threads + Locks | **~2,500 s (~41.6 mins)** | ~280,000 m |
| **`bss-optimizer` (Ours in Rust)** | Modern 8-core CPU | **HiGHS (100% Open Source)** | **Rayon Work-Stealing** | **0.62 s** (30 iters) / **18.31 s** (600 iters) | **270,069 m** 🏆 |

> **Empirical Speedup**: **~4,000x faster** on standard 30-iteration runs and **~136x faster** on full 600-iteration deep searches, while beating the best reported objective value with zero proprietary solver licenses.

---

## 🏗️ Architecture & Governance

The codebase adheres strictly to **Hexagonal Architecture (Ports & Adapters)**, **Domain-Driven Design (DDD)**, and **Strict Compiler Governance**:

* **Domain Layer (`src/domain/`)**: Pure domain invariants, strongly-typed domain primitives (`StationIndex`, `StationId`, `Meters`, `PriorityTier`, `NetFlow`), and sparse spatial structures. Zero external dependencies.
* **Application Layer (`src/application/`)**: Use cases and decoupled ports (`AssignmentSolverPort`, `SpatialClustererPort`, `RunRepositoryPort`).
* **Infrastructure Layer (`src/infrastructure/`)**: Concrete adapters for the open-source **HiGHS** solver, Balanced K-Means, SQLite persistence (`districting.db`), and GeoJSON/Receipt exporters.
* **Presentation Layer (`src/presentation/`)**: Typed CLI via `clap` and rich terminal dashboards via `comfy-table`.
* **Governance**: `unsafe_code = "forbid"`, `dead_code = "deny"`, `unused_variables = "deny"`, and pedantic clippy lints.

For in-depth details, see:
* 📐 **[ARCHITECTURE.md](ARCHITECTURE.md)**: Mathematical MILP formulation, sparse reachability indexing, and component diagrams.
* 🤖 **[AGENTS.md](AGENTS.md)**: Invariants, commands cheatsheet, and guidelines for AI coding agents.
* 🤝 **[CONTRIBUTING.md](CONTRIBUTING.md)**: Developer setup, contribution workflow, and PR standards.

---

## 🚀 Quick Start

### Build
```bash
cargo build --release
```
The optimized binary is located at `target/release/bss-optimizer`.

---

## 💻 CLI Usage

### 1. Run Optimization (`run`)
Solve the districting problem for Mexico City's 452 stations in under a second:
```bash
./target/release/bss-optimizer run -k 15 --iterations 30 --method balanced
```

**Options:**
- `-k, --clusters <K>`: Dynamic number of districts/hubs (default: instance default 15).
- `--method <balanced|kmeans|grid>`: Spatial partitioning strategy (default: `balanced`).
- `--iterations <N>`: RVNS metaheuristic iterations (default: 30).
- `--balance <F>`: Rebalancing flow tolerance ratio (default: 1.0).
- `--priority <N>`: Maximum priority imbalance tolerance per cluster (default: 15).
- `--seed <N>`: Random seed for reproducibility (default: 42).
- `--solver-time-limit <SECONDS>`: Optional HiGHS time limit for each assignment MILP; unlimited by default.
- `--geojson-path <PATH>`: Interactive GeoJSON map export path (default: `districts.geojson`).

For difficult instance configurations, bounded evidence runs can limit each
assignment model independently:

```bash
./target/release/bss-optimizer run -k 15 --iterations 1 --solver-time-limit 1
```

The limit does not change the mathematical constraints. It bounds solver work
per candidate, so report feasibility, objective quality, and runtime together.

### 2. Parametric Fleet Sizing Sweep (`sweep`)
Evaluate fleet sizing behaviors across multiple district counts ($K$) in seconds to compute the Pareto frontier:
```bash
./target/release/bss-optimizer sweep -k 8,10,12,14,15,16,18,20 --iterations 20
```

### 3. Historical Analytics & Strategic Hub Intelligence (`analyze`)
Extract the Pareto scaling curve and discover the top strategic hubs chosen across all runs in `districting.db`:
```bash
./target/release/bss-optimizer analyze
```

### 4. View Historical Runs (`list`)
List historical optimization runs stored in SQLite:
```bash
./target/release/bss-optimizer list
```

### 5. Run Comparative Benchmark (`benchmark`)
Compare spatial partitioning algorithms (Balanced K-Means vs Naive K-Means vs Manual Grid) across multi-seed runs:
```bash
./target/release/bss-optimizer benchmark --runs 3 --iterations 20
```

### 6. Run Automated Tests
```bash
cargo test
```

---

## 📊 Industry Standard Receipts & Map Exports

Every optimization run generates three standard artifacts:

1. **Terminal Dashboard**: Colored table detailing all districts (Hub Station ID, Station Count, Net Flow, Total Flow, Max Radius, Avg Distance, and Priority Breakdown).
2. **SQLite Database (`districting.db`)**: Fully relational persistence tracking runs, iterations, cluster metrics, and station-hub assignments.
3. **Structured JSON Receipt (`receipts/receipt_run_XXXX.json`)**: Contains hardware specs, exact runtime in milliseconds, Pareto improvements, and reproducible configurations.
4. **GeoJSON Map Export (`districts.geojson`)**: Open directly in [geojson.io](https://geojson.io) or Kepler.gl to visualize the colored districts and star-network rebalancing routes across Mexico City.

---

## 📚 Academic Attribution & Citations

This software builds upon and modernizes the mathematical model and instances introduced in:

> **Cabrera-Guerrero, G., Álvarez, A., Vásquez, J., Maya Duque, P. A., & Villavicencio, L. (2022).**  
> *A VNS-Based Matheuristic to Solve the Districting Problem in Bicycle-Sharing Systems.*  
> **Mathematics**, 10(22), 4175. https://doi.org/10.3390/math10224175  
> *(Published under Creative Commons Attribution 4.0 International License - CC BY 4.0)*

### BibTeX
```bibtex
@article{cabrera2022vns,
  title={A VNS-Based Matheuristic to Solve the Districting Problem in Bicycle-Sharing Systems},
  author={Cabrera-Guerrero, Guillermo and {\'A}lvarez, An{\'\i}bal and V{\'a}squez, Joaqu{\'\i}n and Maya Duque, Pablo A. and Villavicencio, Lucas},
  journal={Mathematics},
  volume={10},
  number={22},
  pages={4175},
  year={2022},
  publisher={MDPI},
  doi={10.3390/math10224175}
}
```

### Related 2024 Work

The related 2024 publication is:

> **Cabrera-Guerrero, G., Maya-Duque, P. A., Fernandez, I., Beltran, M., & Lagos, C. (2024).**
> *A Clustering Algorithm to Improve Local Search's Performance for a Public Bicycle Sharing System.*
> In *Optimization, Learning Algorithms and Applications*, pp. 19–32. Springer.
> https://doi.org/10.1007/978-3-031-77426-3_2

The chapter studies how k-means-generated spatial grids affect a local-search matheuristic. This project treats that work as motivation for its k-means strategies, while the 2022 paper remains the verified methodological baseline. The predefined grid is retained as a 2022-style comparator; `kmeans` is the closest 2024-inspired strategy; and `balanced` is an independent project adapter.

The full 2024 chapter is access-controlled. This repository therefore does not claim exact reproduction of its grid construction, dataset, parameters, numerical results, or runtime. Benchmark values produced here are independent measurements on the ECOBICI instance.

---

## 🚲 Data Acknowledgments

The station coordinates and trip telemetry are derived from **ECOBICI Mexico City**, published by the **Secretaría de Movilidad de la Ciudad de México (SEMOVI)** under the **CDMX Datos Abiertos** open data initiative.

---

## 📄 License

This project is licensed under the [MIT License](LICENSE) © 2026 Matias Alonso San Martin.
