use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use log::info;
use std::path::PathBuf;
use tempdir::TempDir;

fn get_default_data_dir() -> PathBuf {
    let mut path = dirs::config_dir().expect("Could not get config dir");
    path.pop();
    path.push(".config/bench_bitcoin");
    path
}

fn get_random_bitcoin_dir() -> PathBuf {
    TempDir::new("bench")
        .expect("Could not create temp dir")
        .into_path()
}

/// Benchmarker which uses /usr/bin/(g)time to benchmark long-running processes, and stores their
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
    #[arg(long, default_value=get_random_bitcoin_dir().into_os_string())]
    pub bitcoin_data_dir: Option<PathBuf>,

    /// Subcommands for bitcoin-bench
    #[clap(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Benchmark-related commands
    #[command(subcommand)]
    Bench(BenchCommands),

    /// Graph-related commands
    #[command(subcommand)]
    Graph(GraphCommands),
}

#[derive(Debug, Subcommand)]
pub enum BenchCommands {
    /// Run benchmarks
    Run {
        #[command(subcommand)]
        run_command: RunCommands,
    },
}

#[derive(Debug, Subcommand)]
pub enum RunCommands {
    /// Run benchmarks once
    Once {
        /// Path to bitcoin source code directory
        src_dir: PathBuf,

        /// git commit hash
        commit: String,
    },

    /// Run benchmarks daily between the start and end dates
    Daily {
        /// Path to bitcoin source code directory
        src_dir: PathBuf,

        /// Start date for daily benchmarks in YYYY-MM-DD format
        start: String,

        /// End date for daily benchmarks in YYYY-MM-DD format
        end: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum GraphCommands {
    /// Generate graphs
    Generate {},
}

impl Cli {
    pub fn init() -> Result<Self> {
        let mut cli = Cli::parse();
        if cli.config_file.is_none() {
            let current_dir = std::env::current_dir()
                .map_err(|e| anyhow!("Failed to get current working directory: {}", e))?;
            cli.config_file = Some(current_dir.join("config.toml"));
        }
        info!(
            "Bitcoin datadir set to: {}",
            cli.bitcoin_data_dir.as_ref().unwrap().to_string_lossy()
        );
        info!(
            "Bitcoin bencher datadir set to: {}",
            cli.bench_data_dir.display()
        );
        Ok(cli)
    }
}
