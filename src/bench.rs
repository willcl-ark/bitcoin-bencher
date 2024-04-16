use std::collections::HashMap;
use std::ffi::OsString;
use std::process::{Command, Stdio};

use anyhow::{bail, Result};
use log::{error, info};

use crate::cli::Cli;
use crate::config::Config;
use crate::database::{record_job, record_run};
use crate::result::TimeResult;
use crate::util;

extern crate exitcode;

fn make_subs(config: &mut Config, cli: &Cli) -> Result<()> {
    let nproc = util::get_nproc().unwrap_or_else(|e| {
        error!("{}", e);
        std::process::exit(exitcode::OSERR);
    });

    for benchmark in &mut config.benchmarks.list {
        // Apply substitutions directly to the entire args string
        benchmark.args = Some(util::make_substitutions(
            &benchmark.args,
            &nproc,
            &cli.test_data_dir.as_ref().unwrap().to_string_lossy(),
        ));
    }

    Ok(())
}

pub fn run_benchmarks(
    cli: &Cli,
    config: &mut Config,
    date: &i64,
    commit_id: String,
    db_conn: &rusqlite::Connection,
) -> Result<()> {
    make_subs(config, cli)?;

    assert!(std::env::set_current_dir(&cli.bitcoin_src_dir).is_ok());
    info!(
        "Changed working directory to {}",
        &cli.bitcoin_src_dir.display()
    );

    let run_id = record_run(db_conn, *date, commit_id)?;
    // TODO: Monitor with procfs while benchmark is running
    // Perhaps just use /usr/bin/time -v for now?
    for benchmark in &mut config.benchmarks.list {
        info!(
            "Running benchmark: {} using {}",
            benchmark.name, benchmark.command
        );
        let mut command = Command::new(benchmark.command.trim());
        // Set env vars sperately as they need to be OsStrings
        if let Some(env_vars) = &benchmark.env {
            let mut envs = HashMap::new();
            for var in env_vars {
                if let Some((key, value)) = var.split_once('=') {
                    envs.insert(OsString::from(key), OsString::from(value));
                } else {
                    bail!("Invalid environment variable format: {}", var);
                }
            }
        }
        let args;
        if let Some(args_single) = &benchmark.args {
            args = args_single
                .split_whitespace()
                .map(String::from)
                .collect::<Vec<String>>();
        } else {
            bail!("Can't run an empty benchmark!");
        }

        command
            .args(&config.time.args)
            .args(&benchmark.format)
            .args(&benchmark.outfile)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

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

        // Read mean, user and system values out
        if let Some(ref outfile_path) = benchmark.outfile {
            let results = TimeResult::from_file(outfile_path)?;
            record_job(db_conn, run_id, results)?;
        }
    }
    Ok(())
}
