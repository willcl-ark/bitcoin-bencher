use anyhow::Result;
use env_logger::Env;
use log::error;
extern crate exitcode;

mod bench;
mod cli;
mod config;
mod database;
mod result;
mod util;

fn main() -> Result<()> {
    // Setup logging
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    // Parse CLI args
    let cli = cli::parse_cli().unwrap_or_else(|e| {
        error!("Error parsing cli: {}", e);
        std::process::exit(exitcode::CONFIG);
    });

    // Load configuration from TOML
    let mut config = config::read_config_file().unwrap_or_else(|e| {
        error!("Error reading config.toml: {}", e);
        std::process::exit(exitcode::CONFIG);
    });

    // Check required binaries exist on PATH
    if let Err(e) = util::check_binaries_exist(&config) {
        error!("Error checking binaries: {}", e);
        std::process::exit(exitcode::UNAVAILABLE);
    }

    // Setup db
    let db_connection = database::get_db(&cli).unwrap_or_else(|e| {
        error!("Error getting database: {}", e);
        std::process::exit(exitcode::CANTCREAT);
    });

    // Check source dir appears valid
    let src_dir_path = util::check_source_file(&cli).unwrap_or_else(|e| {
        error!("Error checking for source code: {}", e);
        std::process::exit(exitcode::NOINPUT);
    });

    // Sync the source repository
    if let Err(e) = util::fetch_repo(&src_dir_path) {
        error!("Error updating repo: {}", e);
        std::process::exit(exitcode::SOFTWARE);
    };

    // Check out code at specified unix time
    let commit_id = util::checkout_commit(&src_dir_path, &cli.date.unwrap()).unwrap_or_else(|e| {
        error!("Error checking for source code: {}", e);
        std::process::exit(exitcode::SOFTWARE);
    });

    // Run benchmarks
    if let Err(e) = bench::run_benchmarks(
        &cli,
        &mut config,
        &cli.date.unwrap(),
        commit_id,
        &db_connection,
    ) {
        error!("{}", e);
        std::process::exit(exitcode::SOFTWARE);
    };

    std::process::exit(exitcode::OK)
}
