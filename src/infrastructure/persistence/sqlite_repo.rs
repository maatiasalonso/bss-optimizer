use crate::application::ports::{RunRecord, RunRepositoryPort, RunSummary};
use chrono::Utc;
use rusqlite::{params, Connection, Result};
use std::path::Path;

pub struct SqliteRunRepository {
    conn: Connection,
}

impl std::fmt::Debug for SqliteRunRepository {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SqliteRunRepository").finish()
    }
}

pub type ScalingCurvePoint = (usize, u32, f64, f64, usize);
pub type StrategicHubPoint = (usize, usize, f64);

impl SqliteRunRepository {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let conn = Connection::open(path)?;
        let repo = Self { conn };
        repo.init_schema()?;
        Ok(repo)
    }

    fn init_schema(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS runs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                created_at TEXT NOT NULL,
                instance_name TEXT NOT NULL,
                num_stations INTEGER NOT NULL,
                num_clusters INTEGER NOT NULL,
                clustering_method TEXT NOT NULL,
                balance_ratio REAL NOT NULL,
                priority_tolerance INTEGER NOT NULL,
                iterations_limit INTEGER NOT NULL,
                neighborhood_size INTEGER NOT NULL,
                seed INTEGER NOT NULL,
                initial_objective INTEGER NOT NULL,
                best_objective INTEGER NOT NULL,
                num_improvements INTEGER NOT NULL,
                runtime_ms INTEGER NOT NULL,
                avg_cluster_diameter REAL NOT NULL,
                max_cluster_diameter REAL NOT NULL,
                best_centers_json TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS iterations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                run_id INTEGER NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
                iteration INTEGER NOT NULL,
                neighborhood_k INTEGER NOT NULL,
                objective_value INTEGER NOT NULL,
                improved INTEGER NOT NULL,
                duration_ms INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS clusters_summary (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                run_id INTEGER NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
                cluster_idx INTEGER NOT NULL,
                center_station_id INTEGER NOT NULL,
                station_count INTEGER NOT NULL,
                net_flow INTEGER NOT NULL,
                total_flow INTEGER NOT NULL,
                max_diameter REAL NOT NULL,
                avg_distance REAL NOT NULL,
                p1_count INTEGER NOT NULL,
                p2_count INTEGER NOT NULL,
                p3_count INTEGER NOT NULL,
                p4_count INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS assignments (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                run_id INTEGER NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
                station_id INTEGER NOT NULL,
                assigned_center_id INTEGER NOT NULL,
                distance_meters INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_iterations_run ON iterations(run_id);
            CREATE INDEX IF NOT EXISTS idx_clusters_run ON clusters_summary(run_id);
            CREATE INDEX IF NOT EXISTS idx_assignments_run ON assignments(run_id);
            ",
        )?;
        Ok(())
    }

    /// Queries fleet scaling curve (Pareto frontier: K districts vs distance & radius)
    pub fn get_scaling_curve(&self) -> Result<Vec<ScalingCurvePoint>, Box<dyn std::error::Error>> {
        let mut stmt = self.conn.prepare(
            "SELECT num_clusters, MIN(best_objective), AVG(best_objective), AVG(avg_cluster_diameter), COUNT(*)
             FROM runs
             GROUP BY num_clusters
             ORDER BY num_clusters ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)? as usize,
                row.get::<_, i64>(1)? as u32,
                row.get::<_, f64>(2)?,
                row.get::<_, f64>(3)?,
                row.get::<_, i64>(4)? as usize,
            ))
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    /// Queries the most frequently selected strategic hub stations across all historical runs
    pub fn get_top_strategic_hubs(&self, limit: usize) -> Result<Vec<StrategicHubPoint>, Box<dyn std::error::Error>> {
        let mut stmt = self.conn.prepare(
            "SELECT center_station_id, COUNT(*) as selection_count, AVG(station_count)
             FROM clusters_summary
             GROUP BY center_station_id
             ORDER BY selection_count DESC
             LIMIT ?1",
        )?;
        let rows = stmt.query_map([limit as i64], |row| {
            Ok((
                row.get::<_, i64>(0)? as usize,
                row.get::<_, i64>(1)? as usize,
                row.get::<_, f64>(2)?,
            ))
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }
}

impl RunRepositoryPort for SqliteRunRepository {
    fn save_run(&mut self, record: &RunRecord) -> Result<i64, Box<dyn std::error::Error>> {
        let tx = self.conn.transaction()?;

        let created_at = Utc::now().to_rfc3339();
        let hubs_raw: Vec<usize> = record.solution.hubs.iter().map(|h| h.0).collect();
        let centers_json = serde_json::to_string(&hubs_raw)?;

        tx.execute(
            "INSERT INTO runs (
                created_at, instance_name, num_stations, num_clusters, clustering_method,
                balance_ratio, priority_tolerance, iterations_limit, neighborhood_size,
                seed, initial_objective, best_objective, num_improvements, runtime_ms,
                avg_cluster_diameter, max_cluster_diameter, best_centers_json
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
            params![
                created_at,
                record.instance_name,
                record.num_stations as i64,
                record.num_clusters as i64,
                record.method_name,
                record.balance_ratio,
                record.priority_tolerance,
                record.iterations_limit as i64,
                record.neighborhood_size as i64,
                record.seed as i64,
                record.initial_objective as i64,
                record.best_objective as i64,
                record.num_improvements as i64,
                record.runtime_ms as i64,
                record.solution.avg_radius,
                record.solution.max_radius.0 as f64,
                centers_json,
            ],
        )?;

        let run_id = tx.last_insert_rowid();

        for it in record.iterations {
            tx.execute(
                "INSERT INTO iterations (run_id, iteration, neighborhood_k, objective_value, improved, duration_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    run_id,
                    it.iteration as i64,
                    it.neighborhood_k as i64,
                    it.objective_value as i64,
                    if it.improved { 1 } else { 0 },
                    it.duration_ms as i64,
                ],
            )?;
        }

        for d in &record.solution.districts {
            tx.execute(
                "INSERT INTO clusters_summary (
                    run_id, cluster_idx, center_station_id, station_count, net_flow, total_flow,
                    max_diameter, avg_distance, p1_count, p2_count, p3_count, p4_count
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    run_id,
                    d.cluster_idx as i64,
                    d.hub_id.0 as i64,
                    d.station_count() as i64,
                    d.net_flow.0,
                    d.total_flow,
                    d.max_radius.0 as f64,
                    d.avg_distance,
                    d.priority_distribution[0] as i64,
                    d.priority_distribution[1] as i64,
                    d.priority_distribution[2] as i64,
                    d.priority_distribution[3] as i64,
                ],
            )?;
        }

        for d in &record.solution.districts {
            for &m in &d.member_stations {
                tx.execute(
                    "INSERT INTO assignments (run_id, station_id, assigned_center_id, distance_meters)
                     VALUES (?1, ?2, ?3, ?4)",
                    params![run_id, m.0 as i64, d.hub_station.0 as i64, 0i64],
                )?;
            }
        }

        tx.commit()?;
        Ok(run_id)
    }

    fn list_runs(&self) -> Result<Vec<RunSummary>, Box<dyn std::error::Error>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, created_at, clustering_method, best_objective, runtime_ms, avg_cluster_diameter
             FROM runs ORDER BY id DESC LIMIT 50",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(RunSummary {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                method: row.get(2)?,
                best_objective_m: row.get::<_, i64>(3)? as u32,
                runtime_ms: row.get::<_, i64>(4)? as u64,
                avg_radius_m: row.get(5)?,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }
}

