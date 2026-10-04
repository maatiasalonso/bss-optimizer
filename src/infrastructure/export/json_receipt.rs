use crate::application::ports::RunRecord;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os: String,
    pub arch: String,
    pub cpu_cores: usize,
    pub solver: String,
    pub optimizer_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptParameters {
    pub instance_name: String,
    pub num_stations: usize,
    pub num_clusters: usize,
    pub clustering_method: String,
    pub max_iterations: usize,
    pub neighborhood_size: usize,
    pub balance_ratio: f64,
    pub priority_tolerance: i32,
    pub random_seed: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptResults {
    pub initial_distance_score: u32,
    pub best_distance_score: u32,
    pub percentage_improvement: f64,
    pub num_improvements: usize,
    pub total_runtime_ms: u64,
    pub avg_cluster_radius_m: f64,
    pub max_cluster_radius_m: u32,
    pub best_hubs_station_ids: Vec<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandardRunReceipt {
    pub receipt_id: i64,
    pub generated_at: String,
    pub system_info: SystemInfo,
    pub parameters: ReceiptParameters,
    pub results: ReceiptResults,
    pub districts: Vec<crate::domain::District>,
}

impl StandardRunReceipt {
    pub fn build(run_id: i64, record: &RunRecord) -> Self {
        let improvement_pct = if record.initial_objective > 0 {
            ((record.initial_objective as f64 - record.best_objective as f64)
                / record.initial_objective as f64)
                * 100.0
        } else {
            0.0
        };

        let hub_ids: Vec<usize> = record
            .solution
            .districts
            .iter()
            .map(|d| d.hub_id.0)
            .collect();

        Self {
            receipt_id: run_id,
            generated_at: Utc::now().to_rfc3339(),
            system_info: SystemInfo {
                os: std::env::consts::OS.to_string(),
                arch: std::env::consts::ARCH.to_string(),
                cpu_cores: rayon::current_num_threads(),
                solver: "HiGHS Sparse MIP Solver via good_lp".to_string(),
                optimizer_version: env!("CARGO_PKG_VERSION").to_string(),
            },
            parameters: ReceiptParameters {
                instance_name: record.instance_name.to_string(),
                num_stations: record.num_stations,
                num_clusters: record.num_clusters,
                clustering_method: record.method_name.to_string(),
                max_iterations: record.iterations_limit,
                neighborhood_size: record.neighborhood_size,
                balance_ratio: record.balance_ratio,
                priority_tolerance: record.priority_tolerance,
                random_seed: record.seed,
            },
            results: ReceiptResults {
                initial_distance_score: record.initial_objective,
                best_distance_score: record.best_objective,
                percentage_improvement: improvement_pct,
                num_improvements: record.num_improvements,
                total_runtime_ms: record.runtime_ms,
                avg_cluster_radius_m: record.solution.avg_radius,
                max_cluster_radius_m: record.solution.max_radius.0,
                best_hubs_station_ids: hub_ids,
            },
            districts: record.solution.districts.clone(),
        }
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)
    }
}
