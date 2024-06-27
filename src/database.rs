use anyhow::{Context, Result};
use log::{debug, info};
use rusqlite::{params, Connection};
use std::path::Path;

use crate::result::TimeResult;

#[derive(Debug)]
pub struct Run {
    pub id: Option<i32>,
    pub run_date: i64,
    pub commit_id: String,
    pub commit_date: i64,
    pub was_master: bool,
}

#[derive(Debug)]
pub struct Job {
    pub job_id: i64,
    pub run_id: i64,
    pub result: TimeResult,
}

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn create_or_load(data_dir: &str, db_name: &str) -> Result<Self> {
        let data_dir_path = Path::new(data_dir);
        info!(
            "Using data directory: {:?} with db name: {:?}",
            data_dir_path, db_name
        );

        std::fs::create_dir_all(data_dir_path).with_context(|| {
            format!(
                "Failed to create data directory '{}'",
                data_dir_path.display()
            )
        })?;

        let db_path = data_dir_path.join(db_name);
        let db_path_str = db_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Failed to convert database path to string"))?;

        let conn = Connection::open(db_path_str)
            .with_context(|| format!("Failed to open database at '{}'", db_path_str))?;

        let db = Database { conn };
        db.create_tables()?;
        Ok(db)
    }

    fn create_tables(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS runs (
                run_id INTEGER PRIMARY KEY AUTOINCREMENT,
                run_date INTEGER NOT NULL,
                was_master INTEGER NOT NULL,
                commit_id TEXT NOT NULL,
                commit_date TEXT NOT NULL
            );",
            params![],
        )?;
        debug!("runs table exists");

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS jobs (
                job_id INTEGER PRIMARY KEY AUTOINCREMENT,
                run_id INTEGER,
                job_name TEXT NOT NULL,
                user_time REAL NOT NULL,
                system_time REAL,
                percent_of_cpu INTEGER,
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
        debug!("jobs table exists");

        info!("All required tables exist in db");
        Ok(())
    }

    pub fn record_run(&self, run: Run) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO runs (run_date, was_master, commit_id, commit_date) VALUES (?, ?, ?, ?)",
            params![run.run_date, run.was_master, run.commit_id, run.commit_date],
        )?;
        debug!(
            "Recorded run on date: {:?} with commit_id: {}, commit_date: {} and was_master: {}",
            run.run_date, run.commit_id, run.commit_date, run.was_master
        );
        Ok(self.conn.last_insert_rowid())
    }

    pub fn record_job(&self, run_id: i64, result: TimeResult) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO jobs (
                run_id, job_name, user_time, system_time, percent_of_cpu,
                max_resident_set_size_kb, major_page_faults, minor_page_faults,
                voluntary_context_switches, involuntary_context_switches,
                file_system_outputs, exit_status
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                run_id,
                result.command,
                result.user_time,
                result.system_time,
                result.percent_of_cpu,
                result.max_resident_set_size_kb,
                result.major_page_faults,
                result.minor_page_faults,
                result.voluntary_context_switches,
                result.involuntary_context_switches,
                result.file_system_outputs,
                result.exit_status
            ],
        )?;
        debug!("Recorded job: {:?}", result);
        Ok(self.conn.last_insert_rowid())
    }

    pub fn get_jobs_by_name(&self, job_name: &str) -> Result<Vec<(Job, Run)>> {
        let mut stmt = self.conn.prepare(
            "SELECT jobs.*, runs.commit_id, runs.run_date, runs.was_master
            FROM jobs
            INNER JOIN runs ON jobs.run_id = runs.run_id
            WHERE job_name = ?
            ORDER BY jobs.run_id ASC",
        )?;

        let job_iter = stmt.query_map([job_name], |row| {
            Ok((
                Job {
                    job_id: row.get(0)?,
                    run_id: row.get(1)?,
                    result: TimeResult {
                        command: row.get(2)?,
                        user_time: row.get(3)?,
                        system_time: row.get(4)?,
                        percent_of_cpu: row.get(5)?,
                        max_resident_set_size_kb: row.get(6)?,
                        major_page_faults: row.get(7)?,
                        minor_page_faults: row.get(8)?,
                        voluntary_context_switches: row.get(9)?,
                        involuntary_context_switches: row.get(10)?,
                        file_system_outputs: row.get(11)?,
                        exit_status: row.get(12)?,
                    },
                },
                Run {
                    id: Some(row.get(1)?),
                    run_date: row.get(14)?,
                    commit_id: row.get(13)?,
                    commit_date: row.get(15)?,
                    was_master: row.get(16)?,
                },
            ))
        })?;

        let jobs_with_runs: Result<Vec<_>, _> = job_iter.collect();
        jobs_with_runs.map_err(anyhow::Error::from)
    }
}
