use super::optimize::{OptimizationConfig, OptimizeDistrictingUseCase};
use crate::application::ports::{AssignmentSolverPort, SpatialClustererPort};
use crate::domain::{DistanceMatrix, Meters, ReachabilityMap, Station};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkRow {
    pub algorithm_name: String,
    pub best_distance_m: u32,
    pub avg_distance_m: f64,
    pub avg_radius_m: f64,
    pub avg_runtime_s: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct BenchmarkConfig {
    pub balance: f64,
    pub priority: i32,
    pub allprior: bool,
}

pub struct BenchmarkUseCase<'a> {
    stations: &'a [Station],
    distances: &'a DistanceMatrix,
    reachability: &'a ReachabilityMap,
    dmax: Meters,
    num_clusters: usize,
    solver: &'a dyn AssignmentSolverPort,
    config: BenchmarkConfig,
}

impl<'a> std::fmt::Debug for BenchmarkUseCase<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BenchmarkUseCase")
            .field("stations_count", &self.stations.len())
            .field("dmax", &self.dmax)
            .field("num_clusters", &self.num_clusters)
            .finish()
    }
}

impl<'a> BenchmarkUseCase<'a> {
    pub fn new(
        stations: &'a [Station],
        distances: &'a DistanceMatrix,
        reachability: &'a ReachabilityMap,
        dmax: Meters,
        num_clusters: usize,
        solver: &'a dyn AssignmentSolverPort,
        config: BenchmarkConfig,
    ) -> Self {
        Self {
            stations,
            distances,
            reachability,
            dmax,
            num_clusters,
            solver,
            config,
        }
    }

    pub fn execute(
        &self,
        clusterers: &[&dyn SpatialClustererPort],
        runs_per_clusterer: usize,
        iterations_per_run: usize,
    ) -> Vec<BenchmarkRow> {
        let mut results = Vec::new();

        for &clusterer in clusterers {
            let mut best_scores = Vec::new();
            let mut avg_radii = Vec::new();
            let mut runtimes = Vec::new();

            for r in 0..runs_per_clusterer {
                let seed = 42 + (r as u64 * 100);
                let partitions = clusterer.partition(self.stations, self.num_clusters, seed);

                let config = OptimizationConfig {
                    max_iterations: iterations_per_run,
                    neighborhood_structure: vec![1, 2, 3],
                    neighborhood_size: 3,
                    balance: self.config.balance,
                    priority: self.config.priority,
                    allprior: self.config.allprior,
                    seed,
                };

                let use_case = OptimizeDistrictingUseCase::new(
                    self.stations,
                    self.distances,
                    self.reachability,
                    self.dmax,
                    &partitions,
                    self.solver,
                    config,
                );

                let out = use_case.execute(|_, _, _, _| {});
                best_scores.push(out.solution.total_distance.0);
                avg_radii.push(out.solution.avg_radius);
                runtimes.push(out.runtime_ms as f64 / 1000.0);
            }

            let min_dist = *best_scores.iter().min().unwrap_or(&0);
            let avg_dist = best_scores.iter().sum::<u32>() as f64 / best_scores.len() as f64;
            let avg_radius = avg_radii.iter().sum::<f64>() / avg_radii.len() as f64;
            let avg_runtime = runtimes.iter().sum::<f64>() / runtimes.len() as f64;

            results.push(BenchmarkRow {
                algorithm_name: clusterer.algorithm_name().to_string(),
                best_distance_m: min_dist,
                avg_distance_m: avg_dist,
                avg_radius_m: avg_radius,
                avg_runtime_s: avg_runtime,
            });
        }

        results
    }
}
