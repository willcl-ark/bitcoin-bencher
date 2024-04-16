use std::path::Path;

use anyhow::{anyhow, Result};
use log::info;
use rusqlite::{params, Connection};

use crate::cli;
use crate::result::TimeResult;

fn create_tables(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS runs (
            run_id INTEGER PRIMARY KEY AUTOINCREMENT,
            date DATETIME NOT NULL,
            commit_id TEXT NOT NULL
        );",
        params![],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS jobs (
            job_id INTEGER PRIMARY KEY AUTOINCREMENT,
            run_id INTEGER,
            job_name TEXT NOT NULL,
            user_time REAL,
            system_time REAL,
            percent_of_cpu INTEGER,
            elapsed_time REAL NOT NULL,
            max_resident_set_size_kb INTEGER,
            major_page_faults INTEGER,
            minor_page_faults INTEGER,
            voluntary_context_switches INTEGER,
            involuntary_context_switches INTEGER,
            file_system_outputs INTEGER,
            exit_status INTEGER,
            FOREIGN KEY (run_id) REFERENCES runs(run_id)
        );",
        params![],
    )?;

    // conn.execute(
    //     "CREATE TABLE IF NOT EXISTS metrics (
    //         metric_id INTEGER PRIMARY KEY AUTOINCREMENT,
    //         job_id INTEGER,
    //         timestamp DATETIME NOT NULL,
    //         cpu_usage REAL,
    //         ram_usage REAL,
    //         other_metrics TEXT,
    //         FOREIGN KEY (job_id) REFERENCES jobs(job_id)
    //     );",
    //     params![],
    // )?;
    info!("All required tables exist in db");

    Ok(())
}

pub fn record_run(conn: &Connection, date: i64, commit_id: String) -> Result<i64> {
    conn.execute(
        "INSERT INTO runs (date, commit_id) VALUES (?, ?)",
        params![date, commit_id],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn record_job(conn: &Connection, run_id: i64, stats: TimeResult) -> Result<i64> {
    conn.execute(
        "INSERT INTO jobs (run_id, job_name, user_time, system_time, percent_of_cpu, elapsed_time, max_resident_set_size_kb, major_page_faults, minor_page_faults, voluntary_context_switches, file_system_outputs, exit_status) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        params![run_id, stats.command, stats.user_time_seconds, stats.system_time_seconds, stats.percent_of_cpu, stats.elapsed_time, stats.max_resident_set_size_kb, stats.major_page_faults, stats.minor_page_faults, stats.voluntary_context_switches, stats.file_system_outputs, stats.exit_status],
    )?;
    Ok(conn.last_insert_rowid())
}

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
    create_tables(&conn)?;

    Ok(conn)
}
