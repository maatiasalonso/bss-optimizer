use crate::domain::DistrictingSolution;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IterationLog {
    pub iteration: usize,
    pub neighborhood_k: usize,
    pub objective_value: u32,
    pub improved: bool,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunSummary {
    pub id: i64,
    pub timestamp: String,
    pub method: String,
    pub best_objective_m: u32,
    pub runtime_ms: u64,
    pub avg_radius_m: f64,
}

#[derive(Debug, Clone)]
pub struct RunRecord<'a> {
    pub instance_name: &'a str,
    pub num_stations: usize,
    pub num_clusters: usize,
    pub method_name: &'a str,
    pub balance_ratio: f64,
    pub priority_tolerance: i32,
    pub iterations_limit: usize,
    pub neighborhood_size: usize,
    pub seed: u64,
    pub initial_objective: u32,
    pub best_objective: u32,
    pub num_improvements: usize,
    pub runtime_ms: u64,
    pub solution: &'a DistrictingSolution,
    pub iterations: &'a [IterationLog],
}

pub trait RunRepositoryPort {
    fn save_run(&mut self, record: &RunRecord) -> Result<i64, Box<dyn std::error::Error>>;
    fn list_runs(&self) -> Result<Vec<RunSummary>, Box<dyn std::error::Error>>;
}
