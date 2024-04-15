use std::process::{Command, Stdio};

use anyhow::Result;
use log::{error, info};

use crate::cli::Cli;
use crate::config::Config;
use crate::util;

extern crate exitcode;

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

pub fn run_benchmarks(cli: &Cli, config: &mut Config) -> Result<()> {
    make_subs(config, cli)?;

    assert!(std::env::set_current_dir(&cli.bitcoin_src_dir).is_ok());
    info!(
        "Changed working directory to {}",
        &cli.bitcoin_src_dir.display()
    );

    // TODO: Generate ID for each benchmark
    // TODO: Monitor with procfs while benchmark is running
    for benchmark in &config.benchmarks.list {
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
    }
    Ok(())
}
