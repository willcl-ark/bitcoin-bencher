use anyhow::{bail, Result};
use log::info;

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::config::{Benchmark, Config};
use crate::database::Database;
use crate::result::TimeResult;

pub struct Bencher<'a> {
    config: &'a mut Config,
    db: &'a Database,
    src_dir: &'a PathBuf,
}

impl<'a> Bencher<'a> {
    pub fn new(config: &'a mut Config, db: &'a Database, src_dir: &'a PathBuf) -> Self {
        Bencher {
            config,
            db,
            src_dir,
        }
    }

    pub fn run(&mut self, date: &i64, commit_id: String) -> Result<()> {
        assert!(std::env::set_current_dir(self.src_dir).is_ok());
        info!("Changed working directory to {}", &self.src_dir.display());

        let run_id = self.db.record_run(*date, commit_id)?;

        let mut benchmarks = std::mem::take(&mut self.config.benchmarks.list);
        for benchmark in &mut benchmarks {
            self.run_single_benchmark(benchmark, run_id)?;
        }
        self.config.benchmarks.list = benchmarks;

        Ok(())
    }

    fn run_single_benchmark(&self, benchmark: &mut Benchmark, run_id: i64) -> Result<()> {
        let output_file = std::fs::File::create("output.log")?;
        let error_file = std::fs::File::create("error.log")?;

        let mut command = Command::new(benchmark.command.trim());
        command
            .args(&self.config.time.args)
            .args(&benchmark.format)
            .args(&benchmark.outfile)
            .stdout(Stdio::from(output_file))
            .stderr(Stdio::from(error_file));

        let bench_args = self.process_args(&benchmark.args)?;
        command.args(&bench_args);

        // Set environment variables if any
        if let Some(envs) = self.process_env_vars(&benchmark.env) {
            command.envs(envs);
        }

        info!("Running benchmark command: {:?}", command);
        let mut child = command.spawn().expect("Failed to start benchmark command");
        let status = child
            .wait()
            .expect("Failed to wait for benchmark command to complete");

        if !status.success() {
            bail!(
                "Benchmark {} failed, see 'error.log' for details",
                benchmark.name
            );
        } else {
            info!(
                "Benchmark {} completed successfully, see 'output.log' for details",
                benchmark.name
            );
        }

        if let Some(ref outfile_path) = benchmark.outfile {
            let results = TimeResult::from_file(outfile_path)?;
            self.db.record_job(run_id, results)?;
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
                .collect::<Vec<(OsString, OsString)>>()
        })
    }

    fn process_args(&self, args: &Option<String>) -> Result<Vec<String>> {
        args.as_ref().map_or_else(
            || bail!("Can't run an empty benchmark!"),
            |args_single| {
                Ok(args_single
                    .split_whitespace()
                    .map(String::from)
                    .collect::<Vec<String>>())
            },
        )
    }
}
