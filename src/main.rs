use anyhow::Result;
use env_logger::Env;
use log::error;
extern crate exitcode;

mod bench;
mod cli;
mod config;
mod database;
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
    let _connection = database::get_db(&cli).unwrap_or_else(|e| {
        error!("Error getting database: {}", e);
        std::process::exit(exitcode::CANTCREAT);
    });

    // Check source dir appears valid
    let src_dir_path = util::check_source_file(&cli).unwrap_or_else(|e| {
        error!("Error checking for source code: {}", e);
        std::process::exit(exitcode::NOINPUT);
    });

    // Sync the repository by running git fetch --all --tags --prune
    if let Err(e) = util::fetch_repo(&src_dir_path) {
        error!("Error updating repo: {}", e);
        std::process::exit(exitcode::SOFTWARE);
    };

    // TODO: Check out code at commit/day

    // Run benchmarks
    if let Err(e) = bench::run_benchmarks(&cli, &mut config) {
        error!("{}", e);
        std::process::exit(exitcode::SOFTWARE);
    };

    // TODO: Record results
    // Each output file is specified in config.benchmarks.list.[bench].outfile

    std::process::exit(exitcode::OK)
}
