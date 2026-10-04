use bss_optimizer::application::ports::{RunRecord, RunRepositoryPort, SpatialClustererPort};
use bss_optimizer::application::use_cases::{OptimizationConfig, OptimizeDistrictingUseCase};
use bss_optimizer::domain::{Meters, PriorityTier, StationId, StationIndex};
use bss_optimizer::infrastructure::clustering::{
    BalancedKMeansClusterer, ManualGridClusterer, NaiveKMeansClusterer,
};
use bss_optimizer::infrastructure::data_loader::load_instance_from_files;
use bss_optimizer::infrastructure::persistence::SqliteRunRepository;
use bss_optimizer::infrastructure::solver::HighsAssignmentSolver;
use std::path::Path;

#[test]
fn test_data_loading_and_sparse_structures() {
    let dat_path = Path::new("data/Ins_PT_452_15.dat");
    let geo_path = Path::new("data/geodata.txt");

    let instance = load_instance_from_files(dat_path, geo_path).expect("Failed to load instance");

    assert_eq!(instance.num_stations, 452);
    assert_eq!(instance.num_clusters, 15);
    assert_eq!(instance.dmax, Meters(2500));
    assert_eq!(instance.stations.len(), 452);

    // Verify strong types
    assert_eq!(instance.stations[0].index, StationIndex(0));
    assert_eq!(instance.stations[0].id, StationId(1));
    assert_eq!(instance.stations[0].priority, PriorityTier::Tier2);

    // Verify sparse reachability map
    let reachable_from_0 = instance.reachability.reachable_from(StationIndex(0));
    assert!(!reachable_from_0.is_empty());
    for &st_idx in reachable_from_0 {
        assert!(instance.distances.dist(StationIndex(0), st_idx) <= Meters(2500));
    }

    // Verify priority buckets
    let p1_stations = instance.priority_buckets.get_stations(0);
    assert!(!p1_stations.is_empty());
    for &st_idx in p1_stations {
        assert_eq!(instance.stations[st_idx.0].priority, PriorityTier::Tier1);
    }

}

#[test]
fn test_solver_time_limit_configuration() {
    let dat_path = Path::new("data/Ins_PT_452_15.dat");
    let geo_path = Path::new("data/geodata.txt");
    let instance = load_instance_from_files(dat_path, geo_path).unwrap();
    let solver = HighsAssignmentSolver::new(
        &instance.stations,
        &instance.distances,
        &instance.reachability,
        &instance.priority_buckets,
        instance.dmax,
    )
    .with_time_limit(Some(1.5));

    assert!(format!("{solver:?}").contains("time_limit_seconds: Some(1.5)"));
}

#[test]
fn test_spatial_clusterers() {
    let dat_path = Path::new("data/Ins_PT_452_15.dat");
    let geo_path = Path::new("data/geodata.txt");
    let instance = load_instance_from_files(dat_path, geo_path).unwrap();

    let clusterers: Vec<Box<dyn SpatialClustererPort>> = vec![
        Box::new(BalancedKMeansClusterer),
        Box::new(NaiveKMeansClusterer),
        Box::new(ManualGridClusterer),
    ];

    for c in clusterers {
        let partitions = c.partition(&instance.stations, instance.num_clusters, 42);
        assert_eq!(partitions.len(), 15, "Algorithm: {}", c.algorithm_name());
        let mut seen = vec![0usize; instance.num_stations];
        for (idx, p) in partitions.iter().enumerate() {
            assert!(
                !p.is_empty(),
                "Cluster {} was empty for {}",
                idx,
                c.algorithm_name()
            );
            for &station in p {
                seen[station.0] += 1;
            }
        }
        assert!(
            seen.iter().all(|&count| count == 1),
            "Algorithm {} must return a disjoint complete partition",
            c.algorithm_name()
        );
    }
}

#[test]
fn test_manual_grid_supports_dynamic_cluster_counts() {
    let dat_path = Path::new("data/Ins_PT_452_15.dat");
    let geo_path = Path::new("data/geodata.txt");
    let instance = load_instance_from_files(dat_path, geo_path).unwrap();
    let clusterer = ManualGridClusterer;

    for k in [8, 15, 20] {
        let partitions = clusterer.partition(&instance.stations, k, 42);
        assert_eq!(partitions.len(), k);
        assert!(partitions.iter().all(|partition| !partition.is_empty()));
        let mut seen = vec![0usize; instance.num_stations];
        for partition in partitions {
            for station in partition {
                seen[station.0] += 1;
            }
        }
        assert!(seen.iter().all(|&count| count == 1));
    }
}

#[test]
fn test_sqlite_repository_persistence() {
    let dat_path = Path::new("data/Ins_PT_452_15.dat");
    let geo_path = Path::new("data/geodata.txt");
    let instance = load_instance_from_files(dat_path, geo_path).unwrap();

    let temp_db = format!(
        "/tmp/test_clean_arch_{}.db",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let mut repo = SqliteRunRepository::open(&temp_db).expect("Failed to open SQLite db");

    let clusterer = BalancedKMeansClusterer;
    let partitions = clusterer.partition(&instance.stations, instance.num_clusters, 42);

    let solver = HighsAssignmentSolver::new(
        &instance.stations,
        &instance.distances,
        &instance.reachability,
        &instance.priority_buckets,
        instance.dmax,
    );

    let config = OptimizationConfig {
        max_iterations: 2,
        neighborhood_structure: vec![1],
        neighborhood_size: 1,
        balance: 1.0,
        priority: 15,
        allprior: false,
        seed: 42,
    };

    let use_case = OptimizeDistrictingUseCase::new(
        &instance.stations,
        &instance.distances,
        &instance.reachability,
        instance.dmax,
        &partitions,
        &solver,
        config,
    );

    let out = use_case.execute(|_, _, _, _| {});
    assert_eq!(out.solution.assignments.len(), instance.num_stations);
    assert_eq!(
        out.solution.total_distance.0,
        out.solution
            .assignments
            .iter()
            .enumerate()
            .map(|(station, &hub)| instance.distances.dist(StationIndex(station), hub).0)
            .sum::<u32>()
    );
    assert!(out
        .solution
        .assignments
        .iter()
        .all(|&hub| out.solution.hubs.contains(&hub)));
    assert!(out
        .solution
        .assignments
        .iter()
        .enumerate()
        .all(|(station, &hub)| instance.distances.dist(StationIndex(station), hub) <= instance.dmax));

    let run_record = RunRecord {
        instance_name: &instance.name,
        num_stations: instance.num_stations,
        num_clusters: instance.num_clusters,
        method_name: "Balanced K-Means",
        balance_ratio: 1.0,
        priority_tolerance: 15,
        iterations_limit: 2,
        neighborhood_size: 1,
        seed: 42,
        initial_objective: out.initial_objective.0,
        best_objective: out.solution.total_distance.0,
        num_improvements: out.num_improvements,
        runtime_ms: out.runtime_ms,
        solution: &out.solution,
        iterations: &out.iterations,
    };

    let run_id = repo.save_run(&run_record).expect("Failed to save run");
    assert!(run_id > 0);

    let runs = repo.list_runs().expect("Failed to list runs");
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].id, run_id);
    assert_eq!(runs[0].best_objective_m, out.solution.total_distance.0);

    let _ = std::fs::remove_file(&temp_db);
}
