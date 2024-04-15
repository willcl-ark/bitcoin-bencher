use clap::Parser;
use env_logger::Env;
use log::error;
use std::fs;
extern crate exitcode;

mod bench;
mod cli;
mod config;
mod database;
mod util;

fn main() {
    // Setup logging
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    // Parse CLI args
    let mut cli = cli::Cli::parse();
    if cli.test_data_dir.is_none() {
        cli.test_data_dir = Some(std::env::temp_dir());
    }

    // Load configuration from TOML
    let config_contents = fs::read_to_string("config.toml").expect("Failed to read config.toml");
    let mut config: config::Config =
        toml::from_str(&config_contents).expect("Failed to parse config.toml");

    // Check required binaries exist on PATH
    util::check_binaries_exist(&config);

    // Setup db
    let _connection = match database::setup_db(&cli) {
        Ok(c) => c,
        Err(e) => {
            error!("Error setting up data directory of database: {}", e);
            std::process::exit(exitcode::CANTCREAT);
        }
    };

    // Check source dir appears valid
    let src_dir_path = util::check_source_file(&cli);

    // Sync the repository by running git fetch --all --tags --prune
    util::fetch_repo(&src_dir_path);

    // TODO: Check out code at commit/day

    // Run benchmarks
    bench::run_benchmarks(&cli, &mut config);

    // TODO: monitor with procfs
    // Could be hard using hyperfine as we want a child PID I think ...

    // TODO: Record results
}
