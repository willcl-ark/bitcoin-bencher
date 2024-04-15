use std::{
    path::{Path, PathBuf},
    process::Command,
};

use chrono::prelude::*;
use log::{info, warn};
use which::which;

use crate::{cli::Cli, config};
extern crate exitcode;

use anyhow::{Context, Result};

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

pub fn check_source_file(cli: &Cli) -> Result<PathBuf> {
    let src_dir_path = Path::new(&cli.bitcoin_src_dir).join("src");
    let init_cpp_path = src_dir_path.join("init.cpp");

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

pub fn checkout_commit(src_dir_path: &PathBuf, date: DateTime<Utc>) -> Result<String> {
    let formatted_date = date.format("%Y-%m-%d %H:%M").to_string();

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
    Ok(commit_id)
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

pub fn make_substitutions(input: &str, nproc: &str, datadir: &str) -> String {
    let input = input.replace("{cores}", nproc);
    input.replace("{datadir}", datadir)
}
