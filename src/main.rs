use anyhow::Result;
use cli::{BenchCommands, Cli, Commands, RunCommands};
use config::Config;
use database::Database;
use env_logger::Env;
use graph::plot_job_metrics;
use log::{error, info};

use crate::bench::{BenchOptions, Multi, Single};

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
        Some(Commands::Bench(BenchCommands::Run { run_command })) => {
            match run_command {
                RunCommands::Once { src_dir, commit } => {
                    let single_options = BenchOptions::Single(Single {
                        commit: commit.clone(),
                    });
                    let mut bencher = bench::Bencher::new(
                        &mut config,
                        &database,
                        src_dir,
                        bench::BenchType::Single,
                        single_options,
                    )?;
                    if let Err(e) = bencher.run() {
                        error!("{}", e);
                        std::process::exit(exitcode::SOFTWARE);
                    }
                    info!("Finished running benchmarks");
                }
                RunCommands::Daily {
                    start,
                    end,
                    src_dir,
                } => {
                    // Run benchmarks daily
                    let multi_options = BenchOptions::Multi(Multi { start, end });
                    let mut bencher = bench::Bencher::new(
                        &mut config,
                        &database,
                        src_dir,
                        bench::BenchType::Multi,
                        multi_options,
                    )?;
                    if let Err(e) = bencher.run() {
                        error!("{}", e);
                        std::process::exit(exitcode::SOFTWARE);
                    }
                    info!("Finished running daily benchmarks");
                }
            }
        }
        Some(Commands::Graph(_)) => {
            plot_job_metrics(&database, &cli.bench_data_dir.to_string_lossy())?;
        }
        None => {}
    }
    std::process::exit(exitcode::OK);
}
