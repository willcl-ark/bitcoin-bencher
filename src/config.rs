use anyhow::{bail, Context, Result};
use log::debug;
use serde::Deserialize;
use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{cli::Cli, util};

#[derive(Deserialize, Debug)]
pub struct Config {
    pub settings: Settings,
    pub jobs: Jobs,
}

#[derive(Deserialize, Debug)]
pub struct Settings {
    pub binaries: Vec<String>,
    pub bitcoin_data_dir: Option<PathBuf>,
}

#[derive(Deserialize, Debug, Default)]
pub struct Jobs {
    pub jobs: Vec<Job>,
}

fn default_bench() -> bool {
    true
}

#[derive(Deserialize, Debug)]
pub struct Job {
    pub name: String,
    pub env: Option<Vec<String>>,
    pub command: String,
    #[serde(default = "default_bench")]
    pub bench: bool,
    pub outfile: Option<String>,
}

impl Config {
    pub fn load_from_file(cli: &Cli, bitcoin_data_dir: &Path) -> Result<Self> {
        let config_contents = fs::read_to_string(cli.config_file.as_ref().unwrap())?;
        let mut config: Config = toml::from_str(&config_contents)?;
        config.settings.bitcoin_data_dir = Some(bitcoin_data_dir.to_path_buf());
        debug!("Using configuration: {:?}", config);

        config.substitute_defaults(cli);
        config.substitute_vars()?;

        Ok(config)
    }

    fn substitute_defaults(&mut self, cli: &Cli) {
        for job in &mut self.jobs.jobs {
            job.outfile.get_or_insert_with(|| {
                format!(
                    "{}/{}-results.txt",
                    cli.bench_data_dir.to_str().unwrap(),
                    job.name
                )
            });
        }
    }

    fn substitute_vars(&mut self) -> Result<()> {
        let nproc = util::get_nproc().context("Failed to get number of processors")?;

        for job in &mut self.jobs.jobs {
            if let Some(bitcoin_data_dir) = &self.settings.bitcoin_data_dir {
                if let Some(bitcoin_data_dir_str) = bitcoin_data_dir.to_str() {
                    job.command = job.command.replace("{cores}", &nproc.to_string());
                    job.command = job
                        .command
                        .replace("{bitcoin_data_dir}", bitcoin_data_dir_str);
                } else {
                    bail!("Failed to convert bitcoin_data_dir to string");
                }
            } else {
                bail!("bitcoin_data_dir is not set");
            }
        }

        Ok(())
    }
}
