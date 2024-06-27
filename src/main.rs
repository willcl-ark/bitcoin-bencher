use anyhow::Result;
use cli::{BenchCommands, Cli, Commands, RunCommands};
use config::Config;
use database::Database;
use env_logger::Env;
use graph::plot_job_metrics;
use log::{error, info};

use crate::bench::{BenchOptions, BenchType, Bencher, Multi, Single};

mod bench;
mod cli;
mod config;
mod database;
mod graph;
mod result;
mod util;

fn main() -> Result<()> {
    setup_logging();

    let cli = Cli::init().map_err(|e| {
        error!("Error initializing CLI: {}", e);
        e
    })?;

    let mut config = Config::load_from_file(&cli, &cli.bitcoin_data_dir).map_err(|e| {
        error!("Error reading config.toml: {}", e);
        e
    })?;

    util::check_binaries_exist(&config).map_err(|e| {
        error!("Error checking binaries: {}", e);
        e
    })?;

    let database =
        Database::create_or_load(&cli.bench_data_dir.to_string_lossy(), &cli.bench_db_name)
            .map_err(|e| {
                error!("Error getting database: {}", e);
                e
            })?;

    match &cli.command {
        Some(Commands::Bench(BenchCommands::Run { run_command })) => {
            handle_bench_command(run_command, &mut config, &database)?;
        }
        Some(Commands::Graph(_)) => {
            plot_job_metrics(&database, &cli.bench_data_dir.to_string_lossy())?;
        }
        None => {
            info!("No command specified. Use --help for usage information.");
        }
    }

    Ok(())
}

fn setup_logging() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
}

fn handle_bench_command(
    run_command: &RunCommands,
    config: &mut Config,
    database: &Database,
) -> Result<()> {
    match run_command {
        RunCommands::Once { src_dir, commit } => {
            let options = BenchOptions::Single(Single {
                commit: commit.clone(),
            });
            run_bencher(config, database, src_dir, BenchType::Single, options)?;
        }
        RunCommands::Daily {
            start,
            end,
            src_dir,
        } => {
            let options = BenchOptions::Multi(Multi { start, end });
            run_bencher(config, database, src_dir, BenchType::Multi, options)?;
        }
    }
    Ok(())
}

fn run_bencher(
    config: &mut Config,
    database: &Database,
    src_dir: &std::path::PathBuf,
    bench_type: BenchType,
    options: BenchOptions,
) -> Result<()> {
    let mut bencher = Bencher::new(config, database, src_dir, bench_type, options)?;
    bencher.run().map_err(|e| {
        error!("Error running benchmarks: {}", e);
        e
    })?;
    info!("Finished running benchmarks");
    Ok(())
}
