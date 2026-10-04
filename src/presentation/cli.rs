use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "bss-optimizer")]
#[command(about = "SOTA Clean Architecture Districting & Rebalancing Optimizer for Bike-Sharing Systems", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum MethodArg {
    Kmeans,
    Balanced,
    Grid,
}

impl MethodArg {
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Kmeans => "Naive K-Means",
            Self::Balanced => "Balanced / Constrained K-Means",
            Self::Grid => "Manual Geographic Grid (Legacy)",
        }
    }
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Run an optimization process with rich output, SQLite recording, and JSON receipt
    Run {
        #[arg(long, default_value = "data/Ins_PT_452_15.dat")]
        dat_path: PathBuf,

        #[arg(long, default_value = "data/geodata.txt")]
        geo_path: PathBuf,

        #[arg(long, default_value = "districting.db")]
        db_path: PathBuf,

        #[arg(long, default_value = "receipts")]
        receipt_dir: PathBuf,

        #[arg(long, default_value = "districts.geojson")]
        geojson_path: PathBuf,

        #[arg(long, value_enum, default_value_t = MethodArg::Balanced)]
        method: MethodArg,

        #[arg(short = 'k', long, help = "Number of districts / hubs (overrides instance default)")]
        clusters: Option<usize>,

        #[arg(short, long, default_value_t = 30)]
        iterations: usize,

        #[arg(long, default_value_t = 3)]
        neighborhood_size: usize,

        #[arg(long, help = "Optional HiGHS time limit per assignment model, in seconds")]
        solver_time_limit: Option<f64>,

        #[arg(short, long)]
        balance: Option<f64>,

        #[arg(short, long)]
        priority: Option<i32>,

        #[arg(long)]
        allprior: Option<bool>,

        #[arg(short, long, default_value_t = 42)]
        seed: u64,
    },

    /// List previous optimization runs stored in SQLite
    List {
        #[arg(long, default_value = "districting.db")]
        db_path: PathBuf,
    },

    /// Run a parameter sweep across multiple district counts (K) to study trade-offs
    Sweep {
        #[arg(long, default_value = "data/Ins_PT_452_15.dat")]
        dat_path: PathBuf,

        #[arg(long, default_value = "data/geodata.txt")]
        geo_path: PathBuf,

        #[arg(long, default_value = "districting.db")]
        db_path: PathBuf,

        #[arg(long, value_enum, default_value_t = MethodArg::Balanced)]
        method: MethodArg,

        #[arg(
            short = 'k',
            long,
            default_value = "8,10,12,14,16,18,20",
            help = "Comma-separated list of district counts to test"
        )]
        clusters: String,

        #[arg(short, long, default_value_t = 30)]
        iterations: usize,

        #[arg(short, long)]
        balance: Option<f64>,

        #[arg(short, long)]
        priority: Option<i32>,
    },

    /// Analyze historical runs in SQLite to identify strategic hubs and fleet scaling
    Analyze {
        #[arg(long, default_value = "districting.db")]
        db_path: PathBuf,
    },

    /// Run a comparative benchmark across clustering methods
    Benchmark {
        #[arg(long, default_value = "data/Ins_PT_452_15.dat")]
        dat_path: PathBuf,

        #[arg(long, default_value = "data/geodata.txt")]
        geo_path: PathBuf,

        #[arg(short, long, default_value_t = 20)]
        iterations: usize,

        #[arg(short, long, default_value_t = 3)]
        runs: usize,
    },
}

#[cfg(test)]
mod tests {
    use super::{Cli, Commands};
    use clap::Parser;

    #[test]
    fn run_accepts_solver_time_limit() {
        let cli = Cli::try_parse_from([
            "bss-optimizer",
            "run",
            "--solver-time-limit",
            "1.5",
        ])
        .expect("valid solver time limit");

        match cli.command {
            Commands::Run {
                solver_time_limit, ..
            } => assert_eq!(solver_time_limit, Some(1.5)),
            _ => panic!("expected run command"),
        }
    }
}
