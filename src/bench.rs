use anyhow::{bail, Context, Result};
use log::{debug, error, info};
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::config::{Config, Job};
use crate::database::{Database, Run};
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

        Ok((commit_date, commit_id))
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

    fn run_single_job(&self, job: &Job, run_id: i64) -> Result<()> {
        let output_filename = format!("/tmp/{}-{}-output.log", run_id, &job.name);
        let error_filename = format!("/tmp/{}-{}-error.log", run_id, &job.name);
        let output_file = std::fs::File::create(&output_filename)?;
        let error_file = std::fs::File::create(&error_filename)?;

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
            bail!(
                "Job {} failed, see '{}' for details",
                job.name,
                error_filename
            );
        } else {
            info!(
                "Job {} completed successfully, see '{}' for details",
                job.name, output_filename,
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

    fn run_benchmarks(&mut self, run_date: i64, commit_id: &str, commit_date: i64) -> Result<()> {
        let run = Run {
            id: None,
            run_date,
            commit_id: commit_id.to_string(),
            commit_date,
            was_master: true,
        };

        let run_id = self.db.record_run(run)?;
        let jobs = std::mem::take(&mut self.config.jobs);

        std::env::set_current_dir(self.src_dir)
            .map_err(|e| anyhow::anyhow!("Failed to change directory: {:?}", e))?;
        info!("Changed working directory to {}", self.src_dir.display());

        util::checkout_commit(self.src_dir, commit_id).unwrap_or_else(|e| {
            error!("Error checking out commit: {}", e);
            std::process::exit(exitcode::SOFTWARE);
        });

        debug!(
            "Using date: {:?}, and commit_id: {}",
            util::unix_timestamp_to_hr(commit_date),
            commit_id
        );

        for job in &jobs.jobs {
            self.run_single_job(job, run_id)?;
        }
        self.config.jobs = jobs; // What was this doing again?

        Ok(())
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

        let run_date = chrono::Utc::now().timestamp();
        match self.bench_type {
            BenchType::Single => {
                let (commit_date, commit_id) = self.setup(run_date)?;
                self.run_benchmarks(run_date, &commit_id, commit_date)?;
                if self.config.jobs.cleanup {
                    util::erase_dir_and_contents(&self.config.settings.bitcoin_data_dir)?;
                }
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
                    self.run_benchmarks(run_date, &commit_id, commit_date)?;
                    current_date += 86400; // Increment by one day (86400 seconds)
                    if self.config.jobs.cleanup {
                        util::erase_dir_and_contents(&self.config.settings.bitcoin_data_dir)?;
                    }
                }
            }
        }
        Ok(())
    }
}
