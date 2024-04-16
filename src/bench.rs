use std::collections::HashMap;
use std::ffi::OsString;
use std::process::{Command, Stdio};

use anyhow::{bail, Context, Result};
use log::{error, info};

use crate::cli::Cli;
use crate::config::Config;
use crate::database::{record_job, record_run};
use crate::util;

extern crate exitcode;

use std::fs::File;
use std::io::BufRead;

#[derive(Debug, Default)]
pub struct TimeResult {
    pub command: String,
    pub user_time_seconds: f64,
    pub system_time_seconds: f64,
    pub percent_of_cpu: i32,
    pub elapsed_time: f64,
    pub max_resident_set_size_kb: i64,
    pub major_page_faults: i64,
    pub minor_page_faults: i64,
    pub voluntary_context_switches: i64,
    pub involuntary_context_switches: i64,
    pub file_system_outputs: i64,
    pub exit_status: i32,
}

fn parse_time_to_seconds(input: &str) -> Result<f64> {
    let parts: Vec<&str> = input.split(':').collect();
    let hours: i32;
    let minutes: i32;
    let seconds_parts: Vec<&str>;
    log::warn!("Got input {} for conversion", input);

    match parts.len() {
        2 => {
            // Assuming MM:SS.ss
            hours = 0;
            minutes = parts[0].parse::<i32>()?;
            seconds_parts = parts[1].split('.').collect::<Vec<&str>>();
        }
        3 => {
            // Assuming HH:MM:SS.ss
            hours = parts[0].parse::<i32>()?;
            minutes = parts[1].parse::<i32>()?;
            seconds_parts = parts[2].split('.').collect::<Vec<&str>>();
        }
        _ => bail!("Invalid time format. Expected HH:MM:SS.ss or MM:SS.ss"),
    }

    if seconds_parts.len() != 2 {
        bail!("Invalid seconds format. Expected SS.ss");
    }
    let seconds: i32 = seconds_parts[0].parse()?;
    let fractional: f64 = format!("0.{}", seconds_parts[1]).parse()?;
    let total_seconds: f64 =
        hours as f64 * 3600.0 + minutes as f64 * 60.0 + seconds as f64 + fractional;

    Ok(total_seconds)
}

fn parse_time_result(file_path: &str) -> Result<TimeResult> {
    let file =
        File::open(file_path).with_context(|| format!("Failed to open file: {}", file_path))?;
    let reader = std::io::BufReader::new(file);
    let mut stats = TimeResult::default();

    for line in reader.lines() {
        let line = line.with_context(|| "Failed to read line from file")?;
        // Use rsplitn to ensure only one split, giving us two parts at most
        let parts: Vec<&str> = line.rsplitn(2, ": ").collect();
        if parts.len() == 2 {
            let value = parts[0].trim();
            let key = parts[1].trim();
            info!("Matching key: {:?}", key);
            match key {
                "Command being timed" => stats.command = value.to_string(),
                "User time (seconds)" => stats.user_time_seconds = value.parse()?,
                "System time (seconds)" => stats.system_time_seconds = value.parse()?,
                "Percent of CPU this job got" => {
                    stats.percent_of_cpu = value.trim_end_matches('%').parse()?
                }
                "Elapsed (wall clock) time (h:mm:ss or m:ss)" => {
                    stats.elapsed_time = parse_time_to_seconds(value)?
                }
                "Maximum resident set size (kbytes)" => {
                    stats.max_resident_set_size_kb = value.parse()?
                }
                "Major (requiring I/O) page faults" => stats.major_page_faults = value.parse()?,
                "Minor (reclaiming a frame) page faults" => {
                    stats.minor_page_faults = value.parse()?
                }
                "Voluntary context switches" => stats.voluntary_context_switches = value.parse()?,
                "Involuntary context switches" => {
                    stats.involuntary_context_switches = value.parse()?
                }
                "File system outputs" => stats.file_system_outputs = value.parse()?,
                "Exit status" => stats.exit_status = value.parse()?,
                _ => {} // Ignore unknown keys
            }
        }
    }

    Ok(stats)
}

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
        // Set env vars sperately as they need to be OsString formatted
        if let Some(env_vars) = &benchmark.env {
            let mut envs = HashMap::new();
            for var in env_vars {
                if let Some((key, value)) = var.split_once('=') {
                    envs.insert(OsString::from(key), OsString::from(value));
                } else {
                    eprintln!("Invalid environment variable format: {}", var);
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
            bail!("Can't run a non-existant benchmark!");
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
            let results = parse_time_result(outfile_path)?;
            record_job(db_conn, run_id, results)?;
        }
    }
    Ok(())
}
