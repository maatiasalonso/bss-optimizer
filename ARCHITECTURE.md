# System Architecture & Technical Specification

`bss-optimizer` is designed around **Hexagonal Architecture (Ports & Adapters)** and **Domain-Driven Design (DDD)** principles to achieve modularity, strict testability, and microsecond-level algorithmic performance.

---

## 1. High-Level Architectural Blueprint

```
┌────────────────────────────────────────────────────────────────────────┐
│                        PRESENTATION LAYER                              │
│         CLI Subcommands (`clap`)  │ Terminal Views (`comfy-table`)     │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │
                                    ▼
┌────────────────────────────────────────────────────────────────────────┐
│                        APPLICATION LAYER                               │
│  Use Cases:                                                            │
│   • `OptimizeDistrictingUseCase` (Parallel RVNS Metaheuristic)         │
│   • `BenchmarkUseCase` (Cross-algorithm comparative runner)            │
│                                                                        │
│  Port Interfaces:                                                      │
│   • `AssignmentSolverPort`                                             │
│   • `SpatialClustererPort`                                             │
│   • `RunRepositoryPort`                                                │
└──────────────────┬─────────────────────────────────┬───────────────────┘
                   │                                 │
                   ▼                                 ▼
┌─────────────────────────────────────┐  ┌───────────────────────────────┐
│            DOMAIN LAYER             │  │     INFRASTRUCTURE LAYER      │
│  (Zero External Dependencies)       │  │  (Concrete Technology)        │
│                                     │  │                               │
│  • Primitives: `StationIndex`,      │  │  • Solver: `HighsAdapter`     │
│    `StationId`, `Meters`, `NetFlow`,│  │    (Sparse MIP via HiGHS)      │
│    `PriorityTier`                   │  │  • Clustering: BalancedKMeans │
│  • Entities: `Station`, `District`  │  │    NaiveKMeans, ManualGrid    │
│  • Aggregates: `DistrictingSolution`│  │  • Persistence: SqliteRepo    │
│  • Spatial: `DistanceMatrix`,       │  │    (`districting.db`)         │
│    `ReachabilityMap`,               │  │  • Export: GeoJSON Exporter,  │
│    `PriorityBuckets`                │  │    StandardRunReceipt JSON    │
└─────────────────────────────────────┘  └───────────────────────────────┘
```

---

## 2. Domain-Driven Design (DDD) Layer

The core domain (`src/domain/`) encapsulates the fundamental invariants of the bike-sharing districting domain without knowing anything about linear programming solvers, SQL databases, or terminal formatters.

### Strongly-Typed Domain Primitives (`types.rs`)
To prevent the **Primitive Obsession** code smell:
* **`StationIndex(usize)`**: Internal 0-based vector index ($0 \le i < 452$). Used for contiguous array indexing and fast distance matrix lookups.
* **`StationId(usize)`**: Real-world 1-based ECOBICI station ID ($1 \le \text{id} \le 452$). Used for external reporting, receipts, and GeoJSON maps.
* **`Meters(u32)`**: Physical distance in meters. Distinct type preventing accidental arithmetic with counts or percentages.
* **`NetFlow(i32)`**: Net bike balance ($\text{trips}_{\text{in}} - \text{trips}_{\text{out}}$). Self-formats with arithmetic signs (`+14`, `-52`).
* **`PriorityTier`**: Categorical classification based on demand urgency ($1 \to \text{Tier1}, \dots, 4 \to \text{Tier4}$).

### Sparse Spatial Indexing (`spatial.rs`)
The critical algorithmic breakthrough that allows the optimizer to run in **0.6 seconds** instead of **40 minutes**:
* **`ReachabilityMap`**: An inverted index mapping each station to only those stations within maximum radius $d_{\max}$ ($~25$ to $40$ neighbors instead of 452).
* **`PriorityBuckets`**: Pre-indexed arrays partitioning station indices by priority tier. Allows the solver to iterate directly over Tier 1 stations in $O(1)$ without scanning the whole dataset.

---

## 3. Mathematical Formulation (Mixed-Integer Linear Programming)

Given a set of $N$ bike stations $S = \{1, \dots, N\}$, a candidate set of $K$ district hubs $H \subset S$, and a distance metric $d_{ij}$:

### Objective Function
Minimize the total commuter travel distance connecting all stations to their respective district hubs:
$$\min \sum_{j \in H} \sum_{i \in S} d_{ij} \cdot x_{ij}$$

Where $x_{ij} \in \{0, 1\}$ is a binary decision variable indicating whether station $i$ is assigned to hub $j$.

### Operational Constraints
1. **Unambiguous Assignment**:
   $$\sum_{j \in H} x_{ij} = 1 \quad \forall i \in S$$
   *Every station must belong to exactly one district.*

2. **Maximum District Radius ($d_{\max}$)**:
   $$x_{ij} = 0 \quad \forall (i, j) \text{ such that } d_{ij} > d_{\max}$$
   *Enforced at compile-time/graph-construction time via `ReachabilityMap`: pairs exceeding $d_{\max}$ are never added as decision variables.*

3. **Priority Distribution Balance**:
   $$\left| \sum_{i \in P_l} x_{ij} - \left\lfloor \frac{|P_l|}{K} \right\rfloor \right| \le \text{tol}_p \quad \forall j \in H, \forall l \in \{1, \dots, L\}$$
   *Critical stations (e.g. high-turnover commuter hubs) are evenly distributed across all $K$ districts so no single van is overwhelmed.*

4. **Rebalancing Net-Flow Balance**:
   $$\left| \sum_{i \in S} \text{NetFlow}_i \cdot x_{ij} \right| \le \beta \cdot \text{TotalFlow}_j \quad \forall j \in H$$
   *Ensures that each district's bike deficit is roughly offset by its bike surplus, minimizing the need for inter-district truck transfers.*

---

## 4. Algorithmic Optimization Pipeline

The application optimizes the problem in two coordinated phases:

```
Step 1: Spatial Partitioning (K-Means / Grid)
               │
               ▼
Step 2: Initial Hub Selection (Centroid projection)
               │
               ▼
Step 3: Parallel RVNS Loop (Rayon Concurrency)
   ├── Neighborhood 1: Random 1-Hub Swap
   ├── Neighborhood 2: Boundary Station Swap
   └── Neighborhood 3: High-Imbalance Hub Swap
               │
               ▼
Step 4: Microsecond Sub-Problem Solves (HiGHS)
               │
               ▼
Step 5: Persistence & Industry Receipts (SQLite + JSON + GeoJSON)
```

### Rayon Work-Stealing Parallelism
During the RVNS search, candidate neighborhood solutions are evaluated concurrently across all available CPU cores using Rayon. Because the domain structures are lock-free and stack-allocated, scaling is near-linear with zero thread contention.

### Literature Baseline Comparison (Cabrera-Guerrero et al., 2022)
In Section 4 and Table 6 of the original paper, the authors reported:
* **Hardware Environment**: Intel Core i7-10700 @ 2.9 GHz (8 cores, 16 threads), 16 GB RAM, Ubuntu 20.04.
* **Solver & Platform**: Gurobi 7.5 called via the JuMP interface in Julia v1.2.
* **Execution Times**: Exact MIP solver capped at **10,000 seconds** (~2.77 hours); rVNS matheuristic executions generally required up to **2,500 seconds (~41.6 minutes)**, with multi-step neighborhood configurations exceeding **10,000 seconds**.
* **Modern Rust Re-implementation**: By replacing the $O(N \times K \times L)$ dense loops and inner-loop Gurobi allocations with `ReachabilityMap` and `PriorityBuckets`, the exact same 3-step rVNS neighborhood exploration executes in **0.62 seconds (30 iterations)** and **18.31 seconds (600 iterations)** on standard 8-core CPUs using the open-source HiGHS solver.
* **Bounded evidence runs**: `run --solver-time-limit <seconds>` optionally applies a per-assignment-MILP HiGHS limit. It defaults to unlimited and preserves the formulation, allowing difficult instance configurations to be characterized without an unbounded candidate solve.

### Related 2024 Clustering Study

Cabrera-Guerrero et al. (2024), *A Clustering Algorithm to Improve Local Search's Performance for a Public Bicycle Sharing System*, Springer, pp. 19–32, DOI [10.1007/978-3-031-77426-3_2](https://doi.org/10.1007/978-3-031-77426-3_2), studies k-means-generated spatial grids as a way to guide and improve local-search performance. The repository keeps this as an extension surface: geographic k-means partitions guide initial hub selection and restricted hub-swap neighborhoods, while the predefined grid remains the 2022-style comparator. Because the full chapter is not openly accessible, this implementation is an independent adaptation rather than an exact reproduction.

---

## 5. Persistence & Relational Schema (`districting.db`)

All optimization runs are persisted into an embedded SQLite database using a fully normalized relational schema:

* **`runs`**: High-level execution metadata (run ID, timestamp, algorithm, seed, fleet size $K$, best objective, runtime ms).
* **`iterations`**: Granular progress log recording every single neighborhood exploration step and objective improvement.
* **`clusters_summary`**: Aggregate metrics per district (hub station ID, station count, net flow, total flow, max radius, priority distribution).
* **`assignments`**: Complete mapping of every station to its designated center and the exact travel distance.

---

## 6. Industry Execution Receipts & GeoJSON Pipeline

To meet enterprise and academic reproducibility standards:
* **Standard JSON Receipt (`receipts/receipt_run_XXXX.json`)**: Contains full execution telemetry, hardware thread count, parameter configurations, and district metrics.
* **Interactive GeoJSON (`districts.geojson`)**: Encodes district convex hulls, station points with metadata (priority tier, capacity, flow), and star-network radial lines ready for direct visualization in [geojson.io](https://geojson.io) or GIS tools.
