use std::process::{Command, Stdio};

use anyhow::{Context, Result};
use log::{error, info};

use crate::cli::Cli;
use crate::config::Config;
use crate::database::{record_job, record_run};
use crate::util;

extern crate exitcode;

use serde::Deserialize;
use std::fs::File;
use std::io::Read;

#[derive(Debug, Deserialize)]
struct BenchmarkResults {
    results: Vec<BenchResult>,
}

#[derive(Debug, Deserialize)]
struct BenchResult {
    command: String,
    mean: f64,
    user: f64,
    system: f64,
}
fn read_benchmark_results(file_path: &String) -> Result<BenchmarkResults> {
    let mut file =
        File::open(file_path).with_context(|| format!("Failed to open the file: {}", file_path))?;

    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .with_context(|| format!("Failed to read the file: {}", file_path))?;

    let results: BenchmarkResults =
        serde_json::from_str(&contents).with_context(|| "Failed to parse JSON data")?;

    Ok(results)
}

pub fn make_subs(config: &mut Config, cli: &Cli) -> Result<()> {
    let nproc = util::get_nproc().unwrap_or_else(|e| {
        error!("{}", e);
        std::process::exit(exitcode::OSERR);
    });

    for benchmark in &mut config.benchmarks.list {
        benchmark.args = benchmark
            .args
            .iter()
            .map(|arg| {
                util::make_substitutions(
                    arg,
                    &nproc,
                    &cli.test_data_dir.as_ref().unwrap().display().to_string(),
                )
            })
            .collect();
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
    for benchmark in &mut config.benchmarks.list {
        info!(
            "Running benchmark: {} using {}",
            benchmark.name, benchmark.command
        );
        let command = Command::new(benchmark.command.trim())
            .args(&config.hyperfine.args)
            .args(&benchmark.format)
            .args(&benchmark.outfile)
            .args(&benchmark.args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to start benchmark command");

        let output = command
            .wait_with_output()
            .expect("Failed to read benchmark command output");

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Benchmark {} failed: {}", benchmark.name, stderr);
        } else {
            let stdout = String::from_utf8_lossy(&output.stdout);
            info!(
                "Benchmark {} completed successfully: {}",
                benchmark.name, stdout
            );
        }

        // Read mean, user and system values out
        if let Some(ref outfile_path) = benchmark.outfile {
            let results = read_benchmark_results(outfile_path)?;

            // TODO: added a loop here, but I think we'll always only have a single run. Remove?
            for result in results.results {
                record_job(
                    db_conn,
                    run_id,
                    result.command,
                    result.mean,
                    result.user,
                    result.system,
                )?;
            }
        }
    }
    Ok(())
}
