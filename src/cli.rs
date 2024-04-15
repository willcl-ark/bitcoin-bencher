use std::path::PathBuf;

use anyhow::Result;
use chrono::prelude::*;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Path to config file
    #[arg(long)]
    pub config_file: Option<PathBuf>,

    /// Path to the database directory.
    #[arg(long, env = "XDG_CONFIG_HOME", default_value_t = String::from("~/.config"))]
    pub bench_data_dir: String,

    /// Database name.
    #[arg(long, default_value = "db.sqlite")]
    pub bench_db_name: String,

    /// Path to source code.
    #[arg(required = true)]
    pub bitcoin_src_dir: PathBuf,

    /// bitcoind test data dir. Randomly created when not supplied.
    #[arg(long)]
    pub test_data_dir: Option<PathBuf>,

    /// Optional date in unix time to backdate the tests to.
    /// Will check out git repo to this date too.
    #[arg(long)]
    pub date: Option<i64>,
}

pub fn parse_cli() -> Result<Cli> {
    let mut cli = Cli::parse();
    if cli.test_data_dir.is_none() {
        cli.test_data_dir = Some(std::env::temp_dir());
    }
    if cli.config_file.is_none() {
        cli.config_file = Some(
            std::env::current_dir()
                .map_err(|e| anyhow::anyhow!("Failed to get current working directory: {}", e))?,
        );
    }
    if cli.date.is_none() {
        cli.date = Some(Utc::now().timestamp());
    }
    Ok(cli)
}
