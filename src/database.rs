use std::path::Path;

use anyhow::{anyhow, Result};
use log::info;
use rusqlite::{params, Connection};

use crate::cli;

pub fn get_db(args: &cli::Cli) -> Result<rusqlite::Connection> {
    let data_dir_path = Path::new(&args.bench_data_dir).join("bench_bitcoin");
    info!(
        "Using data directory: {:?} with db name: {:?}",
        data_dir_path, args.bench_db_name
    );

    std::fs::create_dir_all(&data_dir_path).map_err(|e| {
        anyhow!(
            "Failed to create data directory '{}': {}",
            data_dir_path.display(),
            e
        )
    })?;

    let db_path = data_dir_path.join(&args.bench_db_name);
    let db_path_str = db_path
        .to_str()
        .ok_or_else(|| anyhow!("Failed to convert database path to string"))?;

    let conn = Connection::open(db_path_str)
        .map_err(|e| anyhow!("Failed to open database at '{}': {}", db_path_str, e))?;

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
