use std::path::PathBuf;

use anyhow::{anyhow, Result};
use chrono::prelude::*;
use clap::Parser;

/// Benchmarker which uses /usr/bin/time to benchmark long-running processes, and stores their
/// results in a simple sqlite db.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Path to bitcoin-bench config file (toml)
    #[arg(long)]
    pub config_file: Option<PathBuf>,

    /// Path to the bitcoin-bench database directory.
    #[arg(long, env = "XDG_CONFIG_HOME", default_value_t = String::from("~/.config"))]
    pub bench_data_dir: String,

    /// The bitcoin-bench database name.
    #[arg(long, default_value = "db.sqlite")]
    pub bench_db_name: String,

    /// Path to bitcoin source code.
    #[arg(required = true)]
    pub src_dir: PathBuf,

    /// Data dir to use for bitcoin core during tests.
    /// Randomly created when not supplied.
    #[arg(long)]
    pub bitcoin_data_dir: Option<PathBuf>,

    /// Date in unix time to run tests at.
    /// Will check out git repo to this date too.
    /// Useful for backdating tests (hello Craig!)
    #[arg(long)]
    pub date: Option<i64>,
}

impl Cli {
    pub fn init() -> Result<Self> {
        let mut cli = Cli::parse();
        if cli.bitcoin_data_dir.is_none() {
            cli.bitcoin_data_dir = Some(std::env::temp_dir());
        }
        if cli.config_file.is_none() {
            cli.config_file = Some(
                std::env::current_dir()
                    .map_err(|e| anyhow!("Failed to get current working directory: {}", e))?,
            );
        }
        if cli.date.is_none() {
            cli.date = Some(Utc::now().timestamp());
        }
        Ok(cli)
    }
}
