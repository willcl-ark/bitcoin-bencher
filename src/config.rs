use anyhow::{Context, Result};
use serde::Deserialize;
use std::{fs, path::PathBuf};

use crate::util;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub settings: Settings,
    pub time: TimeSettings,
    pub benchmarks: Benchmarks,
}

#[derive(Deserialize, Debug)]
pub struct Settings {
    pub binaries: Vec<String>,
    pub bitcoin_data_dir: Option<PathBuf>,
}

#[derive(Deserialize, Debug)]
pub struct TimeSettings {
    pub args: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct Benchmarks {
    pub list: Vec<Benchmark>,
}

#[derive(Deserialize, Debug)]
pub struct Benchmark {
    pub name: String,
    pub command: String,
    pub env: Option<Vec<String>>,
    pub format: Option<String>,
    pub outfile: Option<String>,
    pub args: Option<String>,
}

impl Config {
    pub fn load_from_file(
        filename: &Option<PathBuf>,
        bitcoin_data_dir: &Option<PathBuf>,
    ) -> Result<Self> {
        let config_contents = fs::read_to_string(filename.clone().unwrap())
            .with_context(|| format!("Failed to read config file from {:?}", filename.as_ref()))?;
        let mut config: Config =
            toml::from_str(&config_contents).context("Failed to parse config from TOML")?;
        config.settings.bitcoin_data_dir = bitcoin_data_dir.clone();

        config.substitute_defaults();
        config.substitute_vars()?;

        Ok(config)
    }

    fn substitute_defaults(&mut self) {
        for benchmark in &mut self.benchmarks.list {
            benchmark
                .format
                .get_or_insert_with(|| "--output".to_string());
            benchmark
                .outfile
                .get_or_insert_with(|| format!("{}-results.txt", benchmark.name));
        }
    }

    fn substitute_vars(&mut self) -> Result<()> {
        let nproc = util::get_nproc().context("Failed to get number of processors")?;

        for benchmark in &mut self.benchmarks.list {
            if let Some(args) = &mut benchmark.args {
                let bitcoin_data_dir = self
                    .settings
                    .bitcoin_data_dir
                    .as_ref()
                    .map(|path| path.to_string_lossy().to_string())
                    .unwrap();

                *args = args.replace("{cores}", &nproc.to_string());
                *args = args.replace("{datadir}", &bitcoin_data_dir);
            }
        }
        Ok(())
    }
}
