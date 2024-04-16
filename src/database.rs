use anyhow::{anyhow, Result};
use log::info;
use rusqlite::{params, Connection};
use std::path::Path;

use crate::result::TimeResult;

#[derive(Debug)]
pub struct Run {
    run_id: i32,
    date: String,
    commit_id: String,
}

#[derive(Debug)]
pub struct Job {
    pub job_id: i64,
    pub run_id: i64,
    pub job_name: String,
    pub user_time: Option<f64>,
    pub system_time: Option<f64>,
    pub percent_of_cpu: Option<i32>,
    pub elapsed_time: f64,
    pub max_resident_set_size_kb: Option<i32>,
    pub major_page_faults: Option<i32>,
    pub minor_page_faults: Option<i32>,
    pub voluntary_context_switches: Option<i32>,
    pub involuntary_context_switches: Option<i32>,
    pub file_system_outputs: Option<i32>,
    pub exit_status: Option<i32>,
}

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new(data_dir: &str, db_name: &str) -> Result<Self> {
        let data_dir_path = Path::new(data_dir).join("bench_bitcoin");
        info!(
            "Using data directory: {:?} with db name: {:?}",
            data_dir_path, db_name
        );

        std::fs::create_dir_all(&data_dir_path).map_err(|e| {
            anyhow!(
                "Failed to create data directory '{}': {}",
                data_dir_path.display(),
                e
            )
        })?;

        let db_path = data_dir_path.join(db_name);
        let db_path_str = db_path
            .to_str()
            .ok_or_else(|| anyhow!("Failed to convert database path to string"))?;

        let conn = Connection::open(db_path_str)
            .map_err(|e| anyhow!("Failed to open database at '{}': {}", db_path_str, e))?;

        let db = Database { conn };
        db.create_tables()?;
        Ok(db)
    }

    fn create_tables(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS runs (
                run_id INTEGER PRIMARY KEY AUTOINCREMENT,
                date DATETIME NOT NULL,
                commit_id TEXT NOT NULL
            );",
            params![],
        )?;

        self.conn.execute(
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

        info!("All required tables exist in db");
        Ok(())
    }

    pub fn record_run(&self, date: i64, commit_id: String) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO runs (date, commit_id) VALUES (?, ?)",
            params![date, commit_id],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn record_job(&self, run_id: i64, stats: TimeResult) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO jobs (run_id, job_name, user_time, system_time, percent_of_cpu, elapsed_time, max_resident_set_size_kb, major_page_faults, minor_page_faults, voluntary_context_switches, file_system_outputs, exit_status) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![run_id, stats.command, stats.user_time_seconds, stats.system_time_seconds, stats.percent_of_cpu, stats.elapsed_time, stats.max_resident_set_size_kb, stats.major_page_faults, stats.minor_page_faults, stats.voluntary_context_switches, stats.file_system_outputs, stats.exit_status],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    fn get_all_runs(&self) -> Result<Vec<Run>> {
        let mut stmt = self
            .conn
            .prepare("SELECT run_id, date, commit_id FROM runs ORDER BY run_id ASC")?;
        let run_iter = stmt.query_map([], |row| {
            Ok(Run {
                run_id: row.get(0)?,
                date: row.get(1)?,
                commit_id: row.get(2)?,
            })
        })?;

        let mut runs = Vec::new();
        for run in run_iter {
            runs.push(run?);
        }

        Ok(runs)
    }

    pub fn get_jobs_by_run_id(&self, run_id: i64) -> Result<Vec<Job>> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM jobs WHERE run_id = ? ORDER BY job_id ASC")?;
        let job_iter = stmt.query_map([run_id], |row| {
            Ok(Job {
                job_id: row.get(0)?,
                run_id: row.get(1)?,
                job_name: row.get(2)?,
                user_time: row.get(3)?,
                system_time: row.get(4)?,
                percent_of_cpu: row.get(5)?,
                elapsed_time: row.get(6)?,
                max_resident_set_size_kb: row.get(7)?,
                major_page_faults: row.get(8)?,
                minor_page_faults: row.get(9)?,
                voluntary_context_switches: row.get(10)?,
                involuntary_context_switches: row.get(11)?,
                file_system_outputs: row.get(12)?,
                exit_status: row.get(13)?,
            })
        })?;

        let mut jobs = Vec::new();
        for job in job_iter {
            jobs.push(job?);
        }

        Ok(jobs)
    }
}
