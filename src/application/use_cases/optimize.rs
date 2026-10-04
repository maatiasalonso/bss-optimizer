use crate::application::ports::{AssignmentSolverPort, IterationLog};
use crate::domain::{
    DistanceMatrix, DistrictingSolution, Meters, ReachabilityMap, Station, StationIndex,
};
use rand::prelude::*;
use rand::rngs::StdRng;
use rayon::prelude::*;
use std::collections::HashSet;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct OptimizationConfig {
    pub max_iterations: usize,
    pub neighborhood_structure: Vec<usize>,
    pub neighborhood_size: usize,
    pub balance: f64,
    pub priority: i32,
    pub allprior: bool,
    pub seed: u64,
}

#[derive(Debug, Clone)]
pub struct OptimizationOutput {
    pub solution: DistrictingSolution,
    pub initial_objective: Meters,
    pub num_improvements: usize,
    pub runtime_ms: u64,
    pub iterations: Vec<IterationLog>,
}

pub struct OptimizeDistrictingUseCase<'a> {
    stations: &'a [Station],
    distances: &'a DistanceMatrix,
    reachability: &'a ReachabilityMap,
    dmax: Meters,
    cluster_partitions: &'a [Vec<StationIndex>],
    solver: &'a dyn AssignmentSolverPort,
    config: OptimizationConfig,
}

impl<'a> std::fmt::Debug for OptimizeDistrictingUseCase<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OptimizeDistrictingUseCase")
            .field("stations_count", &self.stations.len())
            .field("dmax", &self.dmax)
            .field("config", &self.config)
            .finish()
    }
}

impl<'a> OptimizeDistrictingUseCase<'a> {
    pub fn new(
        stations: &'a [Station],
        distances: &'a DistanceMatrix,
        reachability: &'a ReachabilityMap,
        dmax: Meters,
        cluster_partitions: &'a [Vec<StationIndex>],
        solver: &'a dyn AssignmentSolverPort,
        config: OptimizationConfig,
    ) -> Self {
        Self {
            stations,
            distances,
            reachability,
            dmax,
            cluster_partitions,
            solver,
            config,
        }
    }

    fn find_initial_solution(&self, rng: &mut StdRng) -> (Vec<StationIndex>, DistrictingSolution) {
        let k = self.cluster_partitions.len();
        let num_stations = self.stations.len();
        let mut attempts = 0;

        loop {
            attempts += 1;
            let mut hubs = Vec::with_capacity(k);
            for members in self.cluster_partitions {
                let chosen = members[rng.gen_range(0..members.len())];
                hubs.push(chosen);
            }
            let mut unique_hubs = hubs.clone();
            unique_hubs.sort_unstable();
            unique_hubs.dedup();

            if unique_hubs.len() == k
                && self
                    .reachability
                    .is_coverable(self.distances, &hubs, self.dmax, num_stations)
            {
                if let Some(sol) = self.solver.solve(
                    &hubs,
                    self.config.balance,
                    self.config.priority,
                    self.config.allprior,
                ) {
                    return (hubs, sol);
                }
            }

            if attempts > 2000 {
                panic!("Failed to find an initial feasible solution. Check tolerance constraints.");
            }
        }
    }

    fn generate_neighbors(
        &self,
        current_hubs: &[StationIndex],
        k: usize,
        num_neighbors: usize,
        visited: &HashSet<Vec<StationIndex>>,
        seed: u64,
    ) -> Vec<Vec<StationIndex>> {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut candidates = Vec::new();
        let num_clusters = self.cluster_partitions.len();
        let num_stations = self.stations.len();

        let mut attempts = 0;
        while candidates.len() < num_neighbors && attempts < num_neighbors * 50 {
            attempts += 1;
            let mut new_hubs = current_hubs.to_vec();

            let mut cluster_order: Vec<usize> = (0..num_clusters).collect();
            cluster_order.shuffle(&mut rng);
            let swap_clusters = &cluster_order[..k.min(num_clusters)];
            let mut selected_hubs = vec![false; num_stations];
            for &hub in &new_hubs {
                selected_hubs[hub.0] = true;
            }

            for &c_idx in swap_clusters {
                let members = &self.cluster_partitions[c_idx];
                if members.len() > 1 {
                    let old_hub = new_hubs[c_idx];
                    let candidates: Vec<StationIndex> = members
                        .iter()
                        .copied()
                        .filter(|candidate| *candidate != old_hub && !selected_hubs[candidate.0])
                        .collect();
                    if let Some(&replacement) = candidates.choose(&mut rng) {
                        selected_hubs[old_hub.0] = false;
                        selected_hubs[replacement.0] = true;
                        new_hubs[c_idx] = replacement;
                    }
                }
            }

            let mut sorted = new_hubs.clone();
            sorted.sort_unstable();

            if !visited.contains(&sorted)
                && sorted.len() == num_clusters
                && sorted.windows(2).all(|pair| pair[0] != pair[1])
                && self.reachability.is_coverable(self.distances, &new_hubs, self.dmax, num_stations)
            {
                candidates.push(new_hubs);
            }
        }

        candidates
    }

    pub fn execute<F>(&self, mut on_progress: F) -> OptimizationOutput
    where
        F: FnMut(usize, usize, Meters, bool),
    {
        let start_time = Instant::now();
        let mut rng = StdRng::seed_from_u64(self.config.seed);

        let (mut current_hubs, current_sol) = self.find_initial_solution(&mut rng);
        let initial_objective = current_sol.total_distance;
        let mut best_solution = current_sol;
        let mut best_objective = initial_objective;

        let mut visited = HashSet::new();
        let mut canonical_init = current_hubs.clone();
        canonical_init.sort_unstable();
        visited.insert(canonical_init);

        let mut iterations_log = Vec::new();
        let mut num_improvements = 0;

        for r in 1..=self.config.max_iterations {
            let iter_start = Instant::now();
            let mut iter_improved = false;
            let mut best_k = self.config.neighborhood_structure[0];

            for &k in &self.config.neighborhood_structure {
                let neighbor_seed = self.config.seed + (r as u64 * 1000) + (k as u64);
                let neighbors = self.generate_neighbors(
                    &current_hubs,
                    k,
                    self.config.neighborhood_size,
                    &visited,
                    neighbor_seed,
                );

                for cand in &neighbors {
                    let mut s = cand.clone();
                    s.sort_unstable();
                    visited.insert(s);
                }

                // Rayon parallel evaluation across cores
                let results: Vec<(Vec<StationIndex>, DistrictingSolution)> = neighbors
                    .into_par_iter()
                    .filter_map(|candidate| {
                        self.solver
                            .solve(
                                &candidate,
                                self.config.balance,
                                self.config.priority,
                                self.config.allprior,
                            )
                            .map(|sol| (candidate, sol))
                    })
                    .collect();

                let mut results = results;
                results.sort_by(|(left_hubs, left), (right_hubs, right)| {
                    left.total_distance
                        .cmp(&right.total_distance)
                        .then_with(|| left_hubs.cmp(right_hubs))
                });

                for (cand_hubs, sol) in results {
                    if sol.total_distance < best_objective {
                        best_objective = sol.total_distance;
                        best_solution = sol;
                        current_hubs = cand_hubs;
                        iter_improved = true;
                        best_k = k;
                        num_improvements += 1;
                    }
                }

                if iter_improved {
                    break;
                }
            }

            let iter_duration = iter_start.elapsed().as_millis() as u64;

            iterations_log.push(IterationLog {
                iteration: r,
                neighborhood_k: best_k,
                objective_value: best_objective.0,
                improved: iter_improved,
                duration_ms: iter_duration,
            });

            on_progress(r, self.config.max_iterations, best_objective, iter_improved);
        }

        let total_runtime_ms = start_time.elapsed().as_millis() as u64;

        OptimizationOutput {
            solution: best_solution,
            initial_objective,
            num_improvements,
            runtime_ms: total_runtime_ms,
            iterations: iterations_log,
        }
    }
}
