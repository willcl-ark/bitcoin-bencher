use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Path to the database directory
    #[arg(long, env = "XDG_CONFIG_HOME", default_value_t = String::from("~/.config"))]
    pub bench_data_dir: String,

    /// Database name
    #[arg(long, default_value = "db.sqlite")]
    pub bench_db_name: String,

    /// Path to source code
    #[arg(required = true)]
    pub bitcoin_src_dir: PathBuf,

    /// bitcoind test data dir. Randomly created when empty
    #[arg(long)]
    pub test_data_dir: Option<PathBuf>,
}
