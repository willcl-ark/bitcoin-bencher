use anyhow::{bail, Result};
use log::info;

use std::ffi::OsString;
use std::process::{Command, Stdio};

use crate::cli::Cli;
use crate::config::{Benchmark, Config};
use crate::database::{record_job, record_run};
use crate::result::TimeResult;
use crate::util;

pub struct Bencher<'a> {
    cli: &'a Cli,
    config: &'a mut Config,
    db_conn: &'a rusqlite::Connection,
}

impl<'a> Bencher<'a> {
    pub fn new(cli: &'a Cli, config: &'a mut Config, db_conn: &'a rusqlite::Connection) -> Self {
        Bencher {
            cli,
            config,
            db_conn,
        }
    }

    pub fn run(&mut self, date: &i64, commit_id: String) -> Result<()> {
        self.make_subs()?;
        assert!(std::env::set_current_dir(&self.cli.bitcoin_src_dir).is_ok());
        info!(
            "Changed working directory to {}",
            &self.cli.bitcoin_src_dir.display()
        );

        let run_id = record_run(self.db_conn, *date, commit_id)?;

        let mut benchmarks = std::mem::take(&mut self.config.benchmarks.list);
        for benchmark in &mut benchmarks {
            self.run_single_benchmark(benchmark, run_id)?;
        }
        self.config.benchmarks.list = benchmarks;

        Ok(())
    }

    fn make_subs(&mut self) -> Result<()> {
        let nproc = util::get_nproc().unwrap_or_else(|e| {
            log::error!("{}", e);
            std::process::exit(exitcode::OSERR);
        });

        for benchmark in &mut self.config.benchmarks.list {
            benchmark.args = Some(util::make_substitutions(
                &benchmark.args,
                &nproc,
                &self.cli.test_data_dir.as_ref().unwrap().to_string_lossy(),
            ));
        }
        Ok(())
    }

    fn run_single_benchmark(&self, benchmark: &mut Benchmark, run_id: i64) -> Result<()> {
        let mut command = Command::new(benchmark.command.trim());
        command
            .args(&self.config.time.args)
            .args(&benchmark.format)
            .args(&benchmark.outfile)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let bench_args = self.process_args(&benchmark.args)?;
        command.args(&bench_args);

        if let Some(envs) = self.process_env_vars(&benchmark.env) {
            command.envs(envs);
        }

        let child = command.spawn().expect("Failed to start benchmark command");
        let output = child
            .wait_with_output()
            .expect("Failed to read benchmark command output");

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("Benchmark {} failed: {}", benchmark.name, stderr);
        } else {
            let stdout = String::from_utf8_lossy(&output.stdout);
            info!(
                "Benchmark {} completed successfully: {}",
                benchmark.name, stdout
            );
        }

        if let Some(ref outfile_path) = benchmark.outfile {
            let results = TimeResult::from_file(outfile_path)?;
            record_job(self.db_conn, run_id, results)?;
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
