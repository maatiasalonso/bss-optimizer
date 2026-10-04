use bss_optimizer::application::ports::{RunRecord, RunRepositoryPort, SpatialClustererPort};
use bss_optimizer::application::use_cases::{
    BenchmarkConfig, BenchmarkUseCase, OptimizationConfig, OptimizeDistrictingUseCase,
};
use bss_optimizer::infrastructure::clustering::{
    BalancedKMeansClusterer, ManualGridClusterer, NaiveKMeansClusterer,
};
use bss_optimizer::infrastructure::data_loader::load_instance_from_files;
use bss_optimizer::infrastructure::export::{export_districts_geojson, StandardRunReceipt};
use bss_optimizer::infrastructure::persistence::SqliteRunRepository;
use bss_optimizer::infrastructure::HighsAssignmentSolver;
use bss_optimizer::presentation::cli::{Cli, Commands, MethodArg};
use bss_optimizer::presentation::views::{
    print_banner, render_analytics_dashboard, render_benchmark_table, render_districts_table,
    render_runs_list, render_summary_table, render_sweep_table,
};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run {
            dat_path,
            geo_path,
            db_path,
            receipt_dir,
            geojson_path,
            method,
            clusters,
            iterations,
            neighborhood_size,
            solver_time_limit,
            balance,
            priority,
            allprior,
            seed,
        } => {
            print_banner();
            println!("⚡ Loading instance data from {:?}...", dat_path);
            let instance = load_instance_from_files(&dat_path, &geo_path)?;
            let num_clusters = clusters.unwrap_or(instance.num_clusters);
            println!(
                "✓ Loaded {} stations. Target hubs/districts: {} (fleet size K). Max radius: {}.",
                instance.num_stations, num_clusters, instance.dmax
            );
            let balance = balance.unwrap_or(instance.default_balance);
            let priority = priority.unwrap_or(instance.default_priority);
            let allprior = allprior.unwrap_or(instance.default_allprior);

            // Dependency Injection: Select Clusterer Adapter
            let clusterer: Box<dyn SpatialClustererPort> = match method {
                MethodArg::Balanced => Box::new(BalancedKMeansClusterer),
                MethodArg::Kmeans => Box::new(NaiveKMeansClusterer),
                MethodArg::Grid => Box::new(ManualGridClusterer),
            };

            println!("⚡ Running spatial partitioning: {}...", clusterer.algorithm_name());
            let partitions = clusterer.partition(&instance.stations, num_clusters, seed);

            // Dependency Injection: Solver Adapter (Sparse HiGHS without nested loops!)
            let solver = HighsAssignmentSolver::new(
                &instance.stations,
                &instance.distances,
                &instance.reachability,
                &instance.priority_buckets,
                instance.dmax,
            )
            .with_time_limit(solver_time_limit);

            let config = OptimizationConfig {
                max_iterations: iterations,
                neighborhood_structure: vec![1, 2, 3],
                neighborhood_size,
                balance,
                priority,
                allprior,
                seed,
            };

            let pb = ProgressBar::new(iterations as u64);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} iter | Best: {msg}")?
                    .progress_chars("#>-"),
            );

            let pb_clone = pb.clone();
            let use_case = OptimizeDistrictingUseCase::new(
                &instance.stations,
                &instance.distances,
                &instance.reachability,
                instance.dmax,
                &partitions,
                &solver,
                config,
            );

            let output = use_case.execute(move |iter, _total, best_dist, improved| {
                pb_clone.set_position(iter as u64);
                let tag = if improved { " \x1b[32m▲ (Improved!)\x1b[0m" } else { "" };
                pb_clone.set_message(format!("{} m{}", best_dist.0, tag));
            });
            pb.finish_with_message(format!("Final FO: {}", output.solution.total_distance));

            println!("\n\x1b[1;33m>>> OPTIMIZATION COMPLETED SUCCESSFULLY <<<\x1b[0m\n");

            // 1. Render District Breakdown Table
            render_districts_table(&output.solution);

            // 2. Render Overall Summary Table
            render_summary_table(
                output.initial_objective.0,
                output.solution.total_distance.0,
                output.num_improvements,
                output.solution.avg_radius,
                output.solution.max_radius.0,
                output.runtime_ms,
            );

            // 3. Save to SQLite Repository
            let mut repo = SqliteRunRepository::open(&db_path)?;
            let run_record = RunRecord {
                instance_name: &instance.name,
                num_stations: instance.num_stations,
                num_clusters,
                method_name: method.display_name(),
                balance_ratio: balance,
                priority_tolerance: priority,
                iterations_limit: iterations,
                neighborhood_size,
                seed,
                initial_objective: output.initial_objective.0,
                best_objective: output.solution.total_distance.0,
                num_improvements: output.num_improvements,
                runtime_ms: output.runtime_ms,
                solution: &output.solution,
                iterations: &output.iterations,
            };

            let run_id = repo.save_run(&run_record)?;
            println!("\n\x1b[32m✓ Saved execution receipt to SQLite database:\x1b[0m {:?} (Run ID: {})", db_path, run_id);

            // 4. Save JSON Execution Receipt
            let receipt = StandardRunReceipt::build(run_id, &run_record);
            let receipt_filename = format!("receipt_run_{:04}.json", run_id);
            let receipt_path = receipt_dir.join(receipt_filename);
            receipt.save_to_file(&receipt_path)?;
            println!("\x1b[32m✓ Generated JSON execution receipt:\x1b[0m {:?}", receipt_path);

            // 5. Export GeoJSON Interactive Map
            export_districts_geojson(
                &instance.stations,
                &output.solution,
                &instance.distances,
                &geojson_path,
            )?;
            println!("\x1b[32m✓ Exported interactive GeoJSON map:\x1b[0m {:?} (Ready for geojson.io)", geojson_path);
        }

        Commands::List { db_path } => {
            print_banner();
            let repo = SqliteRunRepository::open(&db_path)?;
            let runs = repo.list_runs()?;
            render_runs_list(&runs);
        }

        Commands::Sweep {
            dat_path,
            geo_path,
            db_path,
            method,
            clusters,
            iterations,
            balance,
            priority,
        } => {
            print_banner();
            let k_values: Vec<usize> = clusters
                .split(',')
                .filter_map(|s| s.trim().parse::<usize>().ok())
                .filter(|&k| k >= 2)
                .collect();

            if k_values.is_empty() {
                eprintln!(
                    "\x1b[31mError: No valid cluster counts found in '{}'. Expected format e.g. -k 8,10,12,15\x1b[0m",
                    clusters
                );
                return Ok(());
            }

            println!("⚡ Loading instance data from {:?}...", dat_path);
            let instance = load_instance_from_files(&dat_path, &geo_path)?;
            println!(
                "⚡ Running parametric sweep across K = {:?} districts ({} stations, max radius {})...",
                k_values, instance.num_stations, instance.dmax
            );
            let balance = balance.unwrap_or(instance.default_balance);
            let priority = priority.unwrap_or(instance.default_priority);

            let clusterer: Box<dyn SpatialClustererPort> = match method {
                MethodArg::Balanced => Box::new(BalancedKMeansClusterer),
                MethodArg::Kmeans => Box::new(NaiveKMeansClusterer),
                MethodArg::Grid => Box::new(ManualGridClusterer),
            };

            let solver = HighsAssignmentSolver::new(
                &instance.stations,
                &instance.distances,
                &instance.reachability,
                &instance.priority_buckets,
                instance.dmax,
            );

            let mut repo = SqliteRunRepository::open(&db_path)?;
            let mut sweep_results = Vec::new();

            let total_steps = k_values.len();
            let sweep_pb = ProgressBar::new(total_steps as u64);
            sweep_pb.set_style(
                ProgressStyle::default_bar()
                    .template("{spinner:.green} [{elapsed_precise}] [{bar:40.yellow/blue}] {pos}/{len} K evaluations | Current: {msg}")?
                    .progress_chars("#>-"),
            );

            for &k in &k_values {
                sweep_pb.set_message(format!("K = {} districts", k));
                let partitions = clusterer.partition(&instance.stations, k, 42);

                let config = OptimizationConfig {
                    max_iterations: iterations,
                    neighborhood_structure: vec![1, 2, 3],
                    neighborhood_size: 3,
                    balance,
                    priority,
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

                let output = use_case.execute(|_, _, _, _| {});

                // Persist run in SQLite
                let run_record = RunRecord {
                    instance_name: &instance.name,
                    num_stations: instance.num_stations,
                    num_clusters: k,
                    method_name: method.display_name(),
                    balance_ratio: balance,
                    priority_tolerance: priority,
                    iterations_limit: iterations,
                    neighborhood_size: 3,
                    seed: 42,
                    initial_objective: output.initial_objective.0,
                    best_objective: output.solution.total_distance.0,
                    num_improvements: output.num_improvements,
                    runtime_ms: output.runtime_ms,
                    solution: &output.solution,
                    iterations: &output.iterations,
                };
                let _ = repo.save_run(&run_record)?;

                sweep_results.push((
                    k,
                    output.solution.total_distance.0,
                    output.solution.avg_radius,
                    output.solution.max_radius,
                    output.runtime_ms,
                ));

                sweep_pb.inc(1);
            }
            sweep_pb.finish_with_message("Sweep completed!");

            render_sweep_table(&sweep_results);
            println!("\x1b[32m✓ Recorded all {} sweep runs to SQLite:\x1b[0m {:?}", total_steps, db_path);
            println!("\x1b[36mTip: Run 'bss-optimizer analyze' to view the computed Pareto scaling curve & strategic hubs!\x1b[0m\n");
        }

        Commands::Analyze { db_path } => {
            print_banner();
            let repo = SqliteRunRepository::open(&db_path)?;
            let curve = repo.get_scaling_curve()?;
            let top_hubs = repo.get_top_strategic_hubs(15)?;
            render_analytics_dashboard(&curve, &top_hubs);
        }

        Commands::Benchmark {
            dat_path,
            geo_path,
            iterations,
            runs,
        } => {
            print_banner();
            println!("⚡ Running benchmark across clustering methods ({} runs x {} iters)...", runs, iterations);
            let instance = load_instance_from_files(&dat_path, &geo_path)?;
            let balance = instance.default_balance;
            let priority = instance.default_priority;
            let allprior = instance.default_allprior;

            let solver = HighsAssignmentSolver::new(
                &instance.stations,
                &instance.distances,
                &instance.reachability,
                &instance.priority_buckets,
                instance.dmax,
            );

            let clusterers: Vec<&dyn SpatialClustererPort> = vec![
                &BalancedKMeansClusterer,
                &NaiveKMeansClusterer,
                &ManualGridClusterer,
            ];

            let benchmark_use_case = BenchmarkUseCase::new(
                &instance.stations,
                &instance.distances,
                &instance.reachability,
                instance.dmax,
                instance.num_clusters,
                &solver,
                BenchmarkConfig {
                    balance,
                    priority,
                    allprior,
                },
            );

            let benchmark_rows = benchmark_use_case.execute(&clusterers, runs, iterations);
            render_benchmark_table(&benchmark_rows);
        }
    }

    Ok(())
}
