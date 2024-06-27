use anyhow::{bail, Context, Result};
use log::{debug, info};
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
        Self::validate_options(&options)?;
        Ok(Bencher {
            config,
            db,
            src_dir,
            bench_type,
            options,
        })
    }

    fn validate_options(options: &BenchOptions) -> Result<()> {
        match options {
            BenchOptions::Single(single) if single.commit.is_empty() => {
                bail!("Commit must be provided for Single bench type")
            }
            BenchOptions::Multi(multi) if multi.start.is_empty() || multi.end.is_empty() => {
                bail!("Start and end dates must be provided for Multi bench type")
            }
            _ => Ok(()),
        }
    }

    pub fn setup(&self, date_to_use: i64) -> Result<(i64, String)> {
        match &self.options {
            BenchOptions::Single(single) => {
                let commit_date = util::get_commit_date(self.src_dir, &single.commit)
                    .context("Error fetching commit date")?;
                Ok((commit_date, single.commit.clone()))
            }
            BenchOptions::Multi(_) => {
                let commit_id = util::get_commit_id_from_date(self.src_dir, &date_to_use)
                    .context("Error fetching commit ID")?;
                let commit_date = util::get_commit_date(self.src_dir, &commit_id)
                    .context("Error fetching commit date")?;
                Ok((commit_date, commit_id))
            }
        }
    }

    fn process_env_vars(&self, env: &Option<Vec<String>>) -> Vec<(OsString, OsString)> {
        env.iter()
            .flat_map(|env_vars| env_vars.iter())
            .filter_map(|var| {
                var.split_once('=')
                    .map(|(key, value)| (OsString::from(key), OsString::from(value)))
            })
            .collect()
    }

    fn process_args<'b>(&self, args: &'b str) -> Result<Vec<&'b str>> {
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
        let mut command = self.create_command(job, &bench_args, output_file, error_file)?;

        let envs = self.process_env_vars(&job.env);
        command.envs(envs);

        info!("Running command: {:?}", command);
        let status = command.spawn()?.wait()?;

        self.handle_job_result(status, job, run_id, &output_filename, &error_filename)?;

        Ok(())
    }

    fn create_command(
        &self,
        job: &Job,
        bench_args: &[&str],
        output_file: std::fs::File,
        error_file: std::fs::File,
    ) -> Result<Command> {
        if job.bench {
            let mut cmd = if cfg!(target_os = "macos") {
                Command::new("/usr/local/bin/gtime")
            } else {
                Command::new("/usr/bin/time")
            };
            cmd.args(["-v", &format!("--output={}", job.outfile.as_ref().unwrap())])
                .args(bench_args)
                .stdout(Stdio::from(output_file))
                .stderr(Stdio::from(error_file));
            Ok(cmd)
        } else {
            let (cmd_name, args) = bench_args
                .split_first()
                .ok_or_else(|| anyhow::anyhow!("Empty command provided for job {}", job.name))?;
            let mut cmd = Command::new(cmd_name);
            cmd.args(args)
                .stdout(Stdio::from(output_file))
                .stderr(Stdio::from(error_file));
            Ok(cmd)
        }
    }

    fn handle_job_result(
        &self,
        status: std::process::ExitStatus,
        job: &Job,
        run_id: i64,
        output_filename: &str,
        error_filename: &str,
    ) -> Result<()> {
        if !status.success() {
            bail!(
                "Job {} failed, see '{}' for details",
                job.name,
                error_filename
            );
        } else {
            info!(
                "Job {} completed successfully, see '{}' for details",
                job.name, output_filename
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

        std::env::set_current_dir(self.src_dir).context("Failed to change directory")?;
        info!("Changed working directory to {}", self.src_dir.display());

        util::checkout_commit(self.src_dir, commit_id).context("Error checking out commit")?;

        debug!(
            "Using date: {:?}, and commit_id: {}",
            util::unix_timestamp_to_hr(commit_date),
            commit_id
        );

        for job in &jobs.jobs {
            self.run_single_job(job, run_id)?;
        }
        self.config.jobs = jobs;

        Ok(())
    }

    pub fn run(&mut self) -> Result<()> {
        util::check_source_file(self.src_dir).context("Error checking for source code")?;

        util::fetch_repo(self.src_dir).context("Error updating repo")?;

        let run_date = chrono::Utc::now().timestamp();
        match self.bench_type {
            BenchType::Single => self.run_single_bench(run_date)?,
            BenchType::Multi => self.run_multi_bench(run_date)?,
        }
        Ok(())
    }

    fn run_single_bench(&mut self, run_date: i64) -> Result<()> {
        let (commit_date, commit_id) = self.setup(run_date)?;
        self.run_benchmarks(run_date, &commit_id, commit_date)?;
        self.cleanup_if_needed()
    }

    fn run_multi_bench(&mut self, run_date: i64) -> Result<()> {
        let options = match &self.options {
            BenchOptions::Multi(multi) => multi,
            _ => bail!("Invalid options for Multi bench type"),
        };
        let start_date = util::parse_date(options.start).context("Failed to parse start date")?;
        let end_date = util::parse_date(options.end).context("Failed to parse end date")?;

        let mut current_date = start_date;
        while current_date <= end_date {
            let (commit_date, commit_id) = self.setup(current_date)?;
            self.run_benchmarks(run_date, &commit_id, commit_date)?;
            current_date += 86400; // Increment by one day (86400 seconds)
            self.cleanup_if_needed()?;
        }
        Ok(())
    }

    fn cleanup_if_needed(&self) -> Result<()> {
        if self.config.jobs.cleanup {
            util::erase_dir_and_contents(&self.config.settings.bitcoin_data_dir)?;
        }
        Ok(())
    }
}
