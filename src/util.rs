use std::{
    path::{Path, PathBuf},
    process::Command,
};

use log::{error, info, warn};
use which::which;

use crate::{cli::Cli, config};
extern crate exitcode;

pub fn check_binaries_exist(config: &config::Config) {
    // Check required binaries exist on PATH
    let mut all_exist: bool = true;
    for prog in &config.settings.binaries {
        match which(prog) {
            Ok(_) => {
                info!("Found {} binary on PATH", prog)
            }
            Err(error) => {
                warn!("{} not found on PATH: {}", prog, error);
                all_exist = false;
            }
        };
    }
    if !all_exist {
        error!("Could not find required binaries on PATH");
        std::process::exit(exitcode::UNAVAILABLE);
    };
}

pub fn check_source_file(cli: &Cli) -> PathBuf {
    let src_dir_path = Path::new(&cli.bitcoin_src_dir).join("src");
    let init_cpp_path = src_dir_path.join("init.cpp");

    if !init_cpp_path.exists() {
        error!(
            "Expected file init.cpp not found in provided src directory: {}",
            init_cpp_path.display()
        );
        std::process::exit(exitcode::NOINPUT);
    } else {
        info!(
            "Found init.cpp in src directory: {}",
            init_cpp_path.display()
        );
    }

    src_dir_path
}

pub fn fetch_repo(src_dir_path: &PathBuf) {
    // Sync the repository by running git fetch --all --tags --prune
    let output = Command::new("git")
        .args(["fetch", "--all", "--tags", "--prune"])
        .current_dir(src_dir_path)
        .output()
        .expect("Failed to execute git command");

    if !output.status.success() {
        error!(
            "Failed to fetch git repository: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        std::process::exit(exitcode::SOFTWARE);
    } else {
        info!("Successfully synced the git repository.");
    }
}

pub fn get_nproc() -> String {
    let nproc_output = Command::new("nproc")
        .output()
        .expect("failed to execute nproc");
    String::from_utf8_lossy(&nproc_output.stdout)
        .trim()
        .to_string()
}

pub fn make_substitutions(input: &str, nproc: &str, datadir: &str) -> String {
    let input = input.replace("{cores}", nproc);
    input.replace("{datadir}", datadir)
}
