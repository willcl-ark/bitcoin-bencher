use anyhow::{bail, Result};
use log::{debug, error, info};
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::config::{Config, Job};
use crate::database::Database;
use crate::result::TimeResult;
use crate::util;

pub struct Bencher<'a> {
    config: &'a mut Config,
    db: &'a Database,
    src_dir: &'a PathBuf,
    commit: &'a Option<String>,
    date: &'a Option<i64>,
}

impl<'a> Bencher<'a> {
    pub fn new(
        config: &'a mut Config,
        db: &'a Database,
        src_dir: &'a PathBuf,
        commit: &'a Option<String>,
        date: &'a Option<i64>,
    ) -> Self {
        Bencher {
            config,
            db,
            src_dir,
            commit,
            date,
        }
    }

    fn setup(&self) -> Result<(i64, String)> {
        // Check source dir appears valid
        let src_dir_path = util::check_source_file(self.src_dir).unwrap_or_else(|e| {
            error!("Error checking for source code: {}", e);
            std::process::exit(exitcode::NOINPUT);
        });

        // Sync the source repository
        if let Err(e) = util::fetch_repo(src_dir_path) {
            error!("Error updating repo: {}", e);
            std::process::exit(exitcode::SOFTWARE);
        }

        // Determine the commit_id and date to use
        let (commit_id, date_to_use): (String, i64) = if let Some(c) = &self.commit {
            // If we have a commit ID, we use it and fetch the date
            let commit_date = util::get_commit_date(self.src_dir, c).unwrap_or_else(|e| {
                error!("Error fetching commit date: {}", e);
                std::process::exit(exitcode::USAGE);
            });
            (c.clone(), commit_date)
        } else if let Some(date) = self.date {
            // If we have a date, we use it to fetch the commit ID
            let fetched_commit_id = util::get_commit_id_from_date(self.src_dir, date)
                .unwrap_or_else(|e| {
                    error!("Error fetching commit ID: {}", e);
                    std::process::exit(exitcode::USAGE);
                });
            (fetched_commit_id, *date)
        } else {
            // If neither is provided, use the current time
            let now = chrono::Utc::now().timestamp();
            let fetched_commit_id = util::get_commit_id_from_date(self.src_dir, &now)
                .unwrap_or_else(|e| {
                    error!("Error fetching commit ID: {}", e);
                    std::process::exit(exitcode::USAGE);
                });
            (fetched_commit_id, now)
        };

        // Check out the commit
        util::checkout_commit(src_dir_path, &commit_id).unwrap_or_else(|e| {
            error!("Error checking out commit: {}", e);
            std::process::exit(exitcode::SOFTWARE);
        });
        debug!(
            "Using date: {:?}, and commit_id: {}",
            util::unix_timestamp_to_hr(date_to_use),
            commit_id
        );

        // Return the date and commit ID
        Ok((date_to_use, commit_id))
    }

    pub fn run(&mut self) -> Result<()> {
        let (date, commit_id) = self.setup()?;
        let run_id = self.db.record_run(date, commit_id)?;
        let jobs = std::mem::take(&mut self.config.jobs);

        std::env::set_current_dir(self.src_dir)
            .map_err(|e| anyhow::anyhow!("Failed to change directory: {:?}", e))?;
        info!("Changed working directory to {}", self.src_dir.display());

        for job in &jobs.jobs {
            self.run_single_job(job, run_id)?;
        }

        self.config.jobs = jobs;
        Ok(())
    }

    fn run_single_job(&self, job: &Job, run_id: i64) -> Result<()> {
        let output_file = std::fs::File::create("/tmp/output.log")?;
        let error_file = std::fs::File::create("/tmp/error.log")?;

        let bench_args = self.process_args(&job.command)?;
        let is_macos = std::env::consts::OS == "macos";
        let mut command = if job.bench {
            let mut cmd = if is_macos {
                Command::new("/usr/local/bin/gtime")
            } else {
                Command::new("/usr/bin/time")
            };
            cmd.args([
                "-v",
                format!("--output={}", job.outfile.as_ref().unwrap()).as_str(),
            ])
            .args(&bench_args)
            .stdout(Stdio::from(output_file))
            .stderr(Stdio::from(error_file));

            cmd
        } else {
            // Split the command and its arguments
            let parts: Vec<&str> = job.command.split_whitespace().collect();
            if parts.is_empty() {
                bail!("Empty command provided for job {}", job.name);
            }
            let (cmd, args) = parts.split_at(1);
            let mut cmd = Command::new(cmd[0]);
            cmd.args(args)
                .stdout(Stdio::from(output_file))
                .stderr(Stdio::from(error_file));
            cmd
        };

        if let Some(envs) = self.process_env_vars(&job.env) {
            command.envs(envs);
        }

        info!("Running command: {:?}", command);
        let status = command.spawn()?.wait()?;

        if !status.success() {
            bail!("Job {} failed, see '/tmp/error.log' for details", job.name);
        } else {
            info!(
                "Job {} completed successfully, see '/tmp/output.log' for details",
                job.name
            );
        }

        if job.bench {
            if let Some(ref outfile_path) = job.outfile {
                let results = TimeResult::from_file(outfile_path)?;
                self.db.record_job(run_id, results)?;
            }
        }

        Ok(())
    }

    fn process_env_vars(&self, env: &Option<Vec<String>>) -> Option<Vec<(OsString, OsString)>> {
        env.as_ref().map(|env_vars| {
            env_vars
                .iter()
                .filter_map(|var| {
                    var.split_once('=')
                        .map(|(key, value)| (OsString::from(key), OsString::from(value)))
                })
                .collect()
        })
    }

    fn process_args(&'a self, args: &'a str) -> Result<Vec<&str>> {
        let parts: Vec<&str> = args.split_whitespace().collect();
        if parts.is_empty() {
            bail!("Empty command provided");
        }
        Ok(parts)
    }
}
