use std::path::Path;

use clap::Parser;
use env_logger::Env;
use log::{debug, error, info, log_enabled, Level};
use rusqlite::{params, Connection, Result};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Path to the database directory
    #[arg(short, long, env = "XDG_CONFIG_HOME", default_value_t = String::from("~/.config"))]
    data_dir: String,

    /// Database name
    #[arg(default_value = "db")]
    db_name: String,
}

fn setup_db(args: &Cli) -> Result<rusqlite::Connection> {
    let data_dir_path = Path::new(&args.data_dir).join("bench_bitcoin");
    std::fs::create_dir_all(&data_dir_path).expect("Unable to create data dir");
    info!("Using data directory: {:?}", &data_dir_path);
    let db_path = data_dir_path.join(&args.db_name);
    let db_path_str = db_path.to_str().expect("Path conversion error");
    let conn = Connection::open(db_path_str)?;

    // Create the tables
    conn.execute(
        "CREATE TABLE IF NOT EXISTS runs (
            run_id INTEGER PRIMARY KEY AUTOINCREMENT,
            date DATETIME NOT NULL
        );",
        params![],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS jobs (
            job_id INTEGER PRIMARY KEY AUTOINCREMENT,
            run_id INTEGER,
            job_name TEXT NOT NULL,
            duration REAL NOT NULL,
            FOREIGN KEY (run_id) REFERENCES Runs(run_id)
        );",
        params![],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS metrics (
            metric_id INTEGER PRIMARY KEY AUTOINCREMENT,
            job_id INTEGER,
            timestamp DATETIME NOT NULL,
            cpu_usage REAL,
            ram_usage REAL,
            other_metrics TEXT,
            FOREIGN KEY (job_id) REFERENCES Jobs(job_id)
        );",
        params![],
    )?;
    info!("All required tables exist in db");
    Ok(conn)
}

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let cli = Cli::parse();
    let connection = match setup_db(&cli) {
        Ok(c) => c,
        Err(e) => {
            panic!("Error setting up data directory of database: {}", e);
        }
    };
}
