use anyhow::{anyhow, Context, Result};
use log::{debug, info};
use rusqlite::{params, Connection};
use std::path::Path;

use crate::result::TimeResult;

#[derive(Debug)]
pub struct Run {
    pub run_id: i32,
    date: i64,
    commit_id: String,
}

#[derive(Debug)]
pub struct Job {
    pub job_id: i64,
    pub run_id: i64,
    pub result: TimeResult,
    pub commit_id: String,
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

        std::fs::create_dir_all(data_dir_path).map_err(|e| {
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
                date INTEGER NOT NULL,
                commit_id TEXT NOT NULL
            );",
            params![],
        )?;
        debug!("runs table exists");

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
        debug!("jobs table exists");

        info!("All required tables exist in db");
        Ok(())
    }

    pub fn record_run(&self, date: i64, commit_id: String) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO runs (date, commit_id) VALUES (?, ?)",
            params![date, commit_id],
        )?;
        debug!(
            "Recorded run on date: {:?} with commit_id: {}",
            date, commit_id
        );
        Ok(self.conn.last_insert_rowid())
    }

    pub fn record_job(&self, run_id: i64, result: TimeResult) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO jobs (
                run_id,
                job_name,
                user_time,
                system_time,
                percent_of_cpu,
                elapsed_time,
                max_resident_set_size_kb,
                major_page_faults,
                minor_page_faults,
                voluntary_context_switches,
                involuntary_context_switches,
                file_system_outputs,
                exit_status
            ) VALUES
            (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                run_id,
                result.command,
                result.user_time,
                result.system_time,
                result.percent_of_cpu,
                result.elapsed_time,
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

    pub fn get_job_names(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare("SELECT DISTINCT job_name FROM jobs")?;
        let job_names = stmt
            .query_map([], |row| row.get(0))
            .with_context(|| "Failed to map query results")?
            .map(|result| result.map_err(anyhow::Error::from))
            .collect::<Result<Vec<String>>>()?;
        debug!("Got job names: {:?}", job_names);
        Ok(job_names)
    }

    pub fn get_jobs_by_name(&self, job_name: &String) -> Result<Vec<Job>> {
        let mut stmt = self
        .conn
        .prepare("SELECT jobs.*, runs.commit_id FROM jobs INNER JOIN runs ON jobs.run_id = runs.run_id WHERE job_name = ? ORDER BY jobs.run_id ASC")?;
        let job_iter = stmt.query_map([job_name], |row| {
            Ok(Job {
                job_id: row.get(0)?,
                run_id: row.get(1)?,
                result: TimeResult {
                    command: row.get(2)?,
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
                },
                commit_id: row.get(14)?,
            })
        })?;

        let mut jobs = Vec::new();
        for job in job_iter {
            debug!("Got job: {:?}", job);
            jobs.push(job?);
        }

        Ok(jobs)
    }
}
