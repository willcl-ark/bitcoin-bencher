use log::{error, info};
use std::process::{Command, Stdio};

use crate::cli::Cli;
use crate::config::Config;
use crate::util;
extern crate exitcode;

pub fn make_subs(config: &mut Config, cli: &Cli) {
    let nproc = util::get_nproc().trim().to_string();

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
}

pub fn run_benchmarks(cli: &Cli, config: &mut Config) {
    make_subs(config, cli);

    assert!(std::env::set_current_dir(&cli.bitcoin_src_dir).is_ok());
    info!(
        "Changed working directory to {}",
        &cli.bitcoin_src_dir.display()
    );

    for benchmark in &config.benchmarks.list {
        info!(
            "Running benchmark: {} using {}",
            benchmark.name, benchmark.command
        );
        let command = Command::new(benchmark.command.trim())
            .args(&config.hyperfine.args)
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
            error!("Benchmark {} failed: {}", benchmark.name, stderr);
            std::process::exit(exitcode::SOFTWARE);
        } else {
            let stdout = String::from_utf8_lossy(&output.stdout);
            info!(
                "Benchmark {} completed successfully: {}",
                benchmark.name, stdout
            );
        }
    }
}
