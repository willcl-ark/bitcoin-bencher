use std::fs;

use anyhow::{anyhow, Result};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub settings: Settings,
    pub time: TimeSettings,
    pub benchmarks: Benchmarks,
}

#[derive(Deserialize, Debug)]
pub struct Settings {
    pub binaries: Vec<String>,
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

pub fn read_config_file() -> Result<Config> {
    let config_contents = fs::read_to_string("config.toml")
        .map_err(|e| anyhow!("Error reading config file from disk: {}", e))?;
    let mut config: Config = toml::from_str(&config_contents)
        .map_err(|e| anyhow!("Failed to parse config.toml: {}", e,))?;

    // Initialize or set default values for optional fields
    for benchmark in &mut config.benchmarks.list {
        benchmark.format = Some(String::from("--output"));
        benchmark.outfile = Some(format!("{}-results.txt", benchmark.name));
    }

    Ok(config)
}
