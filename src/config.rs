use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub settings: Settings,
    pub hyperfine: HyperfineSettings,
    pub benchmarks: Benchmarks,
}

#[derive(Deserialize, Debug)]
pub struct Settings {
    pub binaries: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct HyperfineSettings {
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
    pub args: Vec<String>,
}
