use anyhow::{bail, Context, Result};
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
    bench_type: BenchType,
    options: BenchOptions<'a>,
}

pub enum BenchType {
    Single,
    Multi,
}

pub enum BenchOptions<'a> {
    Single(Single),
    Multi(Multi<'a>),
}

pub struct Single {
    pub commit: String,
}

pub struct Multi<'a> {
    pub start: &'a String,
    pub end: &'a String,
}

impl<'a> Bencher<'a> {
    pub fn new(
        config: &'a mut Config,
        db: &'a Database,
        src_dir: &'a PathBuf,
        bench_type: BenchType,
        options: BenchOptions<'a>,
    ) -> Result<Self> {
        match &options {
            BenchOptions::Single(single) => {
                if single.commit.is_empty() {
                    bail!("Commit must be provided for Single bench type");
                }
            }
            BenchOptions::Multi(multi) => {
                if multi.start.is_empty() || multi.end.is_empty() {
                    bail!("Start and end dates must be provided for Multi bench type");
                }
            }
        }

        Ok(Bencher {
            config,
            db,
            src_dir,
            bench_type,
            options,
        })
    }

    pub fn setup(&self, date_to_use: i64) -> Result<(i64, String)> {
        let (commit_id, commit_date) = match &self.options {
            BenchOptions::Single(single) => {
                let commit_date = util::get_commit_date(self.src_dir, &single.commit)
                    .unwrap_or_else(|e| {
                        error!("Error fetching commit date: {}", e);
                        std::process::exit(exitcode::USAGE);
                    });
                (single.commit.clone(), commit_date)
            }
            BenchOptions::Multi(_) => {
                let fetched_commit_id = util::get_commit_id_from_date(self.src_dir, &date_to_use)
                    .unwrap_or_else(|e| {
                        error!("Error fetching commit ID: {}", e);
                        std::process::exit(exitcode::USAGE);
                    });
                let commit_date = util::get_commit_date(self.src_dir, &fetched_commit_id)
                    .unwrap_or_else(|e| {
                        error!("Error fetching commit date: {}", e);
                        std::process::exit(exitcode::USAGE);
                    });
                (fetched_commit_id, commit_date)
            }
        };

        util::checkout_commit(self.src_dir, &commit_id).unwrap_or_else(|e| {
            error!("Error checking out commit: {}", e);
            std::process::exit(exitcode::SOFTWARE);
        });

        debug!(
            "Using date: {:?}, and commit_id: {}",
            util::unix_timestamp_to_hr(commit_date),
            commit_id
        );

        Ok((commit_date, commit_id))
    }

    pub fn run(&mut self) -> Result<()> {
        let src_dir_path = util::check_source_file(self.src_dir).unwrap_or_else(|e| {
            error!("Error checking for source code: {}", e);
            std::process::exit(exitcode::NOINPUT);
        });

        if let Err(e) = util::fetch_repo(src_dir_path) {
            error!("Error updating repo: {}", e);
            std::process::exit(exitcode::SOFTWARE);
        }

        match self.bench_type {
            BenchType::Single => {
                let date = chrono::Utc::now().timestamp();
                let (commit_date, commit_id) = self.setup(date)?;
                self.run_benchmarks(commit_date, &commit_id)?;
            }
            BenchType::Multi => {
                let options = match &self.options {
                    BenchOptions::Multi(multi) => multi,
                    _ => bail!("Invalid options for Multi bench type"),
                };
                let start_date =
                    util::parse_date(options.start).context("Failed to parse start date")?;
                let end_date = util::parse_date(options.end).context("Failed to parse end date")?;

                let mut current_date = start_date;
                while current_date <= end_date {
                    let (commit_date, commit_id) = self.setup(current_date)?;
                    self.run_benchmarks(commit_date, &commit_id)?;
                    current_date += 86400; // Increment by one day (86400 seconds)
                }
            }
        }
        Ok(())
    }

    fn run_benchmarks(&mut self, date: i64, commit_id: &str) -> Result<()> {
        let run_id = self.db.record_run(date, commit_id.to_string())?;
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
