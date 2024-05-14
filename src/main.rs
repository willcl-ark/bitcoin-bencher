use anyhow::Result;
use env_logger::Env;
use log::{error, info};

use cli::{BenchCommands, Cli, Commands};
use config::Config;
use database::Database;
use graph::plot_job_metrics;

extern crate exitcode;

mod bench;
mod cli;
mod config;
mod database;
mod graph;
mod result;
mod util;

fn main() -> Result<()> {
    // Setup logging
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    // Parse CLI args
    let cli = Cli::init().unwrap_or_else(|e| {
        error!("Error initialising cli: {}", e);
        std::process::exit(exitcode::CONFIG);
    });

    // Load configuration from TOML
    let mut config = Config::load_from_file(&cli, &cli.bitcoin_data_dir).unwrap_or_else(|e| {
        error!("Error reading config.toml: {}", e);
        std::process::exit(exitcode::CONFIG);
    });

    // Check required binaries exist on PATH
    if let Err(e) = util::check_binaries_exist(&config) {
        error!("Error checking binaries: {}", e);
        std::process::exit(exitcode::UNAVAILABLE);
    }

    // Setup db
    let database =
        Database::create_or_load(&cli.bench_data_dir.to_string_lossy(), &cli.bench_db_name)
            .unwrap_or_else(|e| {
                error!("Error getting database: {}", e);
                std::process::exit(exitcode::CANTCREAT);
            });

    // Handle CLI commands
    match &cli.command {
        Some(Commands::Bench(BenchCommands::Run {
            src_dir,
            commit,
            date,
        })) => {
            // Run benchmarks
            let mut bencher = bench::Bencher::new(&mut config, &database, src_dir, commit, date);
            if let Err(e) = bencher.run() {
                error!("{}", e);
                std::process::exit(exitcode::SOFTWARE);
            }
            info!("Finished running benchmarks");
        }
        Some(Commands::Graph(_)) => {
            plot_job_metrics(&database, &cli.bench_data_dir.to_string_lossy())?;
        }
        None => {}
    }
    std::process::exit(exitcode::OK);
}
