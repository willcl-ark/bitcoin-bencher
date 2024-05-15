use anyhow::Result;
use cli::{BenchCommands, Cli, Commands, RunCommands};
use config::Config;
use database::Database;
use env_logger::Env;
use graph::plot_job_metrics;
use log::{error, info};

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
                RunCommands::Once {
                    src_dir,
                    commit,
                    date,
                } => {
                    // Run benchmarks once
                    let mut bencher = bench::Bencher::new(
                        &mut config,
                        &database,
                        src_dir,
                        commit,
                        date,
                        false,
                        None,
                        None,
                    );
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
                    // Parse start and end dates
                    let start_timestamp = util::parse_date(start).unwrap_or_else(|e| {
                        error!("Invalid start date format: {}", e);
                        std::process::exit(exitcode::USAGE);
                    });
                    let end_timestamp = util::parse_date(end).unwrap_or_else(|e| {
                        error!("Invalid end date format: {}", e);
                        std::process::exit(exitcode::USAGE);
                    });

                    // Run benchmarks daily
                    let mut bencher = bench::Bencher::new(
                        &mut config,
                        &database,
                        src_dir,
                        &None,
                        &None,
                        true,
                        Some(start_timestamp),
                        Some(end_timestamp),
                    );
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
