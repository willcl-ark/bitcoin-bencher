use std::{
    fs,
    path::PathBuf,
    process::{Command, Stdio},
    time::{Duration, UNIX_EPOCH},
};

use chrono::prelude::*;
use log::{debug, info, warn};
use which::which;

use crate::config;
extern crate exitcode;

use anyhow::{bail, Context, Result};

pub fn check_binaries_exist(config: &config::Config) -> Result<()> {
    // Check required binaries exist on PATH
    let mut all_exist: bool = true;
    for prog in &config.settings.binaries {
        if which(prog).is_err() {
            warn!("{} not found on PATH", prog);
            all_exist = false;
        } else {
            info!("Found {} binary on $PATH", prog);
        }
    }

    if !all_exist {
        anyhow::bail!("Could not find all required binaries on $PATH");
    }

    Ok(())
}

pub fn check_source_file(src_dir_path: &PathBuf) -> Result<&PathBuf> {
    let init_cpp_path = src_dir_path.join("src/init.cpp");
    if !init_cpp_path.exists() {
        anyhow::bail!(
            "Expected file init.cpp not found in provided src directory: {}",
            init_cpp_path.display()
        );
    } else {
        info!(
            "Found init.cpp in src directory: {}",
            init_cpp_path.display()
        );
    }

    Ok(src_dir_path)
}

pub fn get_commit_id_from_date(src_dir_path: &PathBuf, date: &i64) -> Result<String> {
    let date = Utc.timestamp_opt(*date, 0).unwrap();
    let formatted_date = date.format("%Y-%m-%d %H:%M").to_string();
    debug!(
        "Checking out commit closest in date to {:?}",
        formatted_date
    );

    let commit_id_output = Command::new("git")
        .args(["rev-list", "-n", "1", "--before", &formatted_date, "master"])
        .current_dir(src_dir_path)
        .output()
        .with_context(|| "Failed to execute git rev-list")?;

    if !commit_id_output.status.success() {
        let stderr = String::from_utf8_lossy(&commit_id_output.stderr);
        anyhow::bail!("git rev-list failed: {}", stderr);
    }

    let commit_id = String::from_utf8_lossy(&commit_id_output.stdout)
        .trim()
        .to_string();

    Ok(commit_id)
}

pub fn get_commit_date(repo_path: &PathBuf, commit_id: &str) -> Result<i64> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .arg("show")
        .arg("-s")
        .arg("--format=%ct")
        .arg(commit_id)
        .stdout(Stdio::piped())
        .output()
        .with_context(|| format!("Failed to execute git command for commit ID: {}", commit_id))?;

    if !output.status.success() {
        bail!(
            "Git command failed with status: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let output_str = String::from_utf8(output.stdout)
        .context("Failed to convert git command output to string")?;
    let commit_timestamp = output_str
        .trim()
        .parse::<i64>()
        .context("Failed to parse commit date as i64")?;

    Ok(commit_timestamp)
}

pub fn unix_timestamp_to_hr(timestamp: i64) -> String {
    // Convert the timestamp to a SystemTime
    let duration_since_epoch = Duration::from_secs(timestamp as u64);
    let datetime = UNIX_EPOCH + duration_since_epoch;

    // Format the SystemTime to a human-readable date
    let datetime: chrono::DateTime<chrono::Utc> = datetime.into();
    datetime.format("%Y-%m-%d %H:%M:%S").to_string()
}

pub fn checkout_commit(src_dir_path: &PathBuf, commit_id: &str) -> Result<()> {
    let checkout_output = Command::new("git")
        .args(["checkout", &commit_id, "--detach"])
        .current_dir(src_dir_path)
        .output()
        .with_context(|| "Failed to execute git checkout")?;

    if !checkout_output.status.success() {
        let stderr = String::from_utf8_lossy(&checkout_output.stderr);
        anyhow::bail!("git checkout failed: {}", stderr);
    }

    info!("Successfully checked out commit {}", commit_id);
    Ok(())
}

pub fn fetch_repo(src_dir_path: &PathBuf) -> Result<()> {
    // Sync the repository by running git fetch --all --tags --prune
    let output = Command::new("git")
        .args(["fetch", "--all", "--tags", "--prune"])
        .current_dir(src_dir_path)
        .output()
        .with_context(|| {
            format!(
                "Failed to execute git command in directory '{}'",
                src_dir_path.display()
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to fetch git repository: {}", stderr);
    } else {
        info!("Successfully synced the git repository.");
    }

    Ok(())
}

pub fn get_nproc() -> Result<String> {
    let nproc_output = Command::new("nproc")
        .output()
        .context("Failed to execute nproc command")?;

    if !nproc_output.status.success() {
        anyhow::bail!("nproc command execution failed");
    }

    Ok(String::from_utf8_lossy(&nproc_output.stdout)
        .trim()
        .to_string())
}

pub fn parse_date(date_str: &str) -> Result<i64> {
    let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")?;
    Ok(date.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp())
}

pub fn erase_datadir_except_debug_log(data_dir: &Option<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(data_dir.clone().unwrap())? {
        let entry = entry?;
        let path = entry.path();
        if let Some(file_name) = path.file_name() {
            if file_name != "debug.log" {
                if path.is_dir() {
                    fs::remove_dir_all(&path)?;
                } else {
                    fs::remove_file(&path)?;
                }
            }
        }
    }
    Ok(())
}
