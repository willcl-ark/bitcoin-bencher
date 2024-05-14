use std::path::PathBuf;

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use log::info;
use tempdir::TempDir;

fn get_default_data_dir() -> PathBuf {
    // Ridiculous this needs a crate...
    let mut path = dirs::config_dir().expect("Could not get config dir");
    path.pop();
    path.push(".config/bench_bitcoin");
    path
}

fn get_random_bitcoin_dir() -> PathBuf {
    // This too
    TempDir::new("bench")
        .expect("Could not create temp dir")
        .into_path()
}

/// Benchmarker which uses /usr/bin/time to benchmark long-running processes, and stores their
/// results in a simple sqlite db.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Path to bitcoin-bench config file (toml)
    #[arg(long)]
    pub config_file: Option<PathBuf>,

    /// Path to the bitcoin-bench database directory.
    #[arg(long, env = "BENCH_BITCOIN_DIR", default_value=get_default_data_dir().into_os_string())]
    pub bench_data_dir: PathBuf,

    /// The bitcoin-bench database name.
    #[arg(long, default_value = "db.sqlite")]
    pub bench_db_name: String,

    /// Data dir to use for bitcoin core during tests.
    /// Randomly created when not supplied.
    #[arg(long, default_value=get_random_bitcoin_dir().into_os_string())]
    pub bitcoin_data_dir: PathBuf,

    /// The subcommands for bitcoin-bench
    #[clap(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Handle benchmark-related commands
    #[command(subcommand)]
    Bench(BenchCommands),

    /// Handle graph-related commands
    #[command(subcommand)]
    Graph(GraphCommands),
}

#[derive(Debug, Subcommand)]
pub enum BenchCommands {
    /// Command to run benchmarks
    Run {
        /// Path to bitcoin source code directory
        #[arg(required = true)]
        src_dir: PathBuf,

        /// Date in unix time to run tests at.
        /// Will check out git repo to this date too.
        /// Useful for backdating tests (hello Craig!)
        #[arg(long)]
        date: Option<i64>,

        /// Commit hash to run tests at.
        /// Will check out git repo at this hash
        #[arg(long)]
        commit: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
pub enum GraphCommands {
    /// Command to generate graphs
    Generate {},
}

impl Cli {
    pub fn init() -> Result<Self> {
        let mut cli = Cli::parse();
        if cli.config_file.is_none() {
            cli.config_file = Some(
                std::env::current_dir()
                    .map_err(|e| anyhow!("Failed to get current working directory: {}", e))?,
            );
        }
        info!(
            "Bitcoin bencher datadir set to: {}",
            cli.bench_data_dir.display()
        );
        info!(
            "Bitcoin datadir set to: {}",
            cli.bitcoin_data_dir.to_string_lossy()
        );
        Ok(cli)
    }
}
