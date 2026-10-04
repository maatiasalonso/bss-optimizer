use crate::application::ports::RunSummary;
use crate::application::use_cases::BenchmarkRow;
use crate::domain::{DistrictingSolution, Meters};
use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::UTF8_FULL;
use comfy_table::{Cell, Color, Row, Table};

pub fn print_banner() {
    println!("\x1b[1;36m========================================================================\x1b[0m");
    println!("\x1b[1;32m   BICYCLE-SHARING SYSTEM DISTRICTING OPTIMIZER (SOTA CLEAN ARCH)       \x1b[0m");
    println!("\x1b[1;36m========================================================================\x1b[0m");
}

pub fn render_districts_table(solution: &DistrictingSolution) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_header(vec![
            "District",
            "Hub Station",
            "Stations",
            "Net Flow",
            "Total Flow",
            "Max Radius",
            "Avg Dist (m)",
            "Priority (P1/P2/P3/P4)",
        ]);

    for d in &solution.districts {
        let p_str = format!(
            "{}/{}/{}/{}",
            d.priority_distribution[0],
            d.priority_distribution[1],
            d.priority_distribution[2],
            d.priority_distribution[3]
        );
        let flow_color = if d.net_flow.0.abs() <= 2 {
            Color::Green
        } else {
            Color::Yellow
        };

        table.add_row(Row::from(vec![
            Cell::new(format!("#{}", d.cluster_idx + 1)),
            Cell::new(format!("{}", d.hub_id)).fg(Color::Cyan),
            Cell::new(d.station_count()),
            Cell::new(format!("{}", d.net_flow)).fg(flow_color),
            Cell::new(d.total_flow),
            Cell::new(format!("{}", d.max_radius)),
            Cell::new(format!("{:.0}", d.avg_distance)),
            Cell::new(p_str),
        ]));
    }
    println!("{}", table);
}

pub fn render_summary_table(
    initial_obj_m: u32,
    best_obj_m: u32,
    num_improvements: usize,
    avg_radius_m: f64,
    max_radius_m: u32,
    runtime_ms: u64,
) {
    let mut summary_table = Table::new();
    summary_table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_header(vec!["Optimization Metric", "Value"]);

    let imp_pct = if initial_obj_m > 0 {
        ((initial_obj_m as f64 - best_obj_m as f64) / initial_obj_m as f64) * 100.0
    } else {
        0.0
    };

    summary_table.add_row(vec!["Initial Distance (FO)", &format!("{} m", initial_obj_m)]);
    summary_table.add_row(vec!["Best Distance (FO)", &format!("{} m", best_obj_m)]);
    summary_table.add_row(vec![
        "Total Improvement",
        &format!("{:.2}% ({} upgrades)", imp_pct, num_improvements),
    ]);
    summary_table.add_row(vec!["Average District Radius", &format!("{:.1} m", avg_radius_m)]);
    summary_table.add_row(vec!["Maximum District Radius", &format!("{} m", max_radius_m)]);
    summary_table.add_row(vec![
        "Execution Time",
        &format!("{} ms ({:.2} s)", runtime_ms, runtime_ms as f64 / 1000.0),
    ]);
    summary_table.add_row(vec!["Parallel Worker Threads", &format!("{}", rayon::current_num_threads())]);
    println!("\n{}", summary_table);
}

pub fn render_runs_list(runs: &[RunSummary]) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_header(vec![
            "Run ID",
            "Timestamp",
            "Method",
            "Best FO (m)",
            "Runtime (s)",
            "Avg Radius (m)",
        ]);

    for r in runs {
        table.add_row(vec![
            r.id.to_string(),
            r.timestamp.clone(),
            r.method.clone(),
            r.best_objective_m.to_string(),
            format!("{:.1}", r.runtime_ms as f64 / 1000.0),
            format!("{:.1}", r.avg_radius_m),
        ]);
    }
    println!("{table}");
}

pub fn render_benchmark_table(rows: &[BenchmarkRow]) {
    let mut summary_table = Table::new();
    summary_table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_header(vec![
            "Method",
            "Best Distance (m)",
            "Avg Distance (m)",
            "Avg Radius (m)",
            "Avg Time (s)",
        ]);

    for r in rows {
        summary_table.add_row(vec![
            r.algorithm_name.clone(),
            format!("{} m", r.best_distance_m),
            format!("{:.1} m", r.avg_distance_m),
            format!("{:.1} m", r.avg_radius_m),
            format!("{:.2} s", r.avg_runtime_s),
        ]);
    }
    println!("\n{summary_table}");
}

pub fn render_sweep_table(rows: &[(usize, u32, f64, Meters, u64)]) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_header(vec![
            "Districts (K)",
            "Best Distance",
            "Avg Radius",
            "Max Radius",
            "Runtime",
        ]);

    for &(k, best_dist, avg_r, max_r, time_ms) in rows {
        table.add_row(vec![
            format!("{} hubs", k),
            format!("{} m", best_dist),
            format!("{:.1} m", avg_r),
            format!("{}", max_r),
            format!("{:.2} s", time_ms as f64 / 1000.0),
        ]);
    }
    println!("\n\x1b[1;33m>>> PARAMETRIC K-DISTRICTS SWEEP RESULTS <<<\x1b[0m\n");
    println!("{table}");
}

pub fn render_analytics_dashboard(
    curve: &[(usize, u32, f64, f64, usize)],
    top_hubs: &[(usize, usize, f64)],
) {
    println!("\n\x1b[1;36m========================================================================\x1b[0m");
    println!("\x1b[1;32m      SQLITE HISTORICAL ANALYTICS & STRATEGIC HUB INTELLIGENCE         \x1b[0m");
    println!("\x1b[1;36m========================================================================\x1b[0m\n");

    // 1. Fleet Scaling Table
    println!("\x1b[1;33m1. FLEET SCALING CURVE (Fleet Size K vs Commuter Travel Distance):\x1b[0m");
    let mut curve_table = Table::new();
    curve_table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_header(vec![
            "Fleet Size (K)",
            "Min Total Dist (m)",
            "Avg Total Dist (m)",
            "Avg District Radius (m)",
            "Sample Runs",
        ]);

    for &(k, min_d, avg_d, avg_r, count) in curve {
        curve_table.add_row(vec![
            format!("{k} vans/hubs"),
            format!("{min_d} m"),
            format!("{avg_d:.1} m"),
            format!("{avg_r:.1} m"),
            count.to_string(),
        ]);
    }
    println!("{curve_table}");

    // 2. Top Strategic Hubs Table
    println!("\n\x1b[1;33m2. MOST STRATEGIC HUBS (Top Hub Stations in Mexico City across Runs):\x1b[0m");
    let mut hubs_table = Table::new();
    hubs_table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_header(vec![
            "Rank",
            "ECOBICI Station ID",
            "Selection Frequency",
            "Avg Stations in District",
        ]);

    for (rank, &(st_id, count, avg_st)) in top_hubs.iter().enumerate() {
        hubs_table.add_row(vec![
            format!("#{}", rank + 1),
            format!("Station #{st_id}"),
            format!("{count} times"),
            format!("{avg_st:.1} stations"),
        ]);
    }
    println!("{hubs_table}");
}
