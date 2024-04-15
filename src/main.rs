use log::{error, info, warn};

use clap::Parser;
use env_logger::Env;
use which::which;
extern crate exitcode;

mod cli;
mod database;

fn main() {
    // Setup logging
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    // Parse CLI args
    let cli = cli::Cli::parse();

    // Check required binaries exist on PATH
    let programs = vec!["bitcoind", "hyperfine"];
    let mut all_exist: bool = true;
    for prog in &programs {
        match which(prog) {
            Ok(_) => {
                info!("Found {} binary on PATH", prog)
            }
            Err(error) => {
                warn!("{} not found on PATH: {}", prog, error);
                all_exist = false;
            }
        };
    }
    if !all_exist {
        error!("Could not find required binaries on PATH");
        std::process::exit(exitcode::UNAVAILABLE);
    };

    // Setup db
    let connection = match database::setup_db(&cli) {
        Ok(c) => c,
        Err(e) => {
            error!("Error setting up data directory of database: {}", e);
            std::process::exit(exitcode::CANTCREAT);
        }
    };
}
