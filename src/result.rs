use anyhow::{Context, Result};
use log::debug;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::str::FromStr;

#[derive(Debug, Default)]
pub struct TimeResult {
    pub command: String,
    pub user_time: f64,
    pub system_time: f64,
    pub percent_of_cpu: i32,
    pub max_resident_set_size_kb: i64,
    pub major_page_faults: i64,
    pub minor_page_faults: i64,
    pub voluntary_context_switches: i64,
    pub involuntary_context_switches: i64,
    pub file_system_outputs: i64,
    pub exit_status: i32,
}

impl TimeResult {
    pub fn from_file(file_path: &str) -> Result<Self> {
        let file =
            File::open(file_path).with_context(|| format!("Failed to open file: {}", file_path))?;
        let reader = BufReader::new(file);
        let mut result = TimeResult::default();

        for line in reader.lines() {
            let line = line.with_context(|| "Failed to read line from file")?;
            result.parse_line(&line)?;
        }
        Ok(result)
    }

    fn parse_line(&mut self, line: &str) -> Result<()> {
        let parts: Vec<&str> = line.rsplitn(2, ": ").collect();
        if parts.len() == 2 {
            let value = parts[0].trim();
            let key = parts[1].trim();
            self.update_field(key, value)
                .with_context(|| format!("Failed to parse line: {}", line))?;
        }
        Ok(())
    }

    fn update_field(&mut self, key: &str, value: &str) -> Result<()> {
        match key {
            "Command being timed" => self.command = value.trim_matches('"').to_string(),
            "User time (seconds)" => self.user_time = parse_value(value)?,
            "System time (seconds)" => self.system_time = parse_value(value)?,
            "Percent of CPU this job got" => {
                self.percent_of_cpu = parse_value(value.trim_end_matches('%'))?
            }
            "Maximum resident set size (kbytes)" => {
                self.max_resident_set_size_kb = parse_value(value)?
            }
            "Major (requiring I/O) page faults" => self.major_page_faults = parse_value(value)?,
            "Minor (reclaiming a frame) page faults" => {
                self.minor_page_faults = parse_value(value)?
            }
            "Voluntary context switches" => self.voluntary_context_switches = parse_value(value)?,
            "Involuntary context switches" => {
                self.involuntary_context_switches = parse_value(value)?
            }
            "File system outputs" => self.file_system_outputs = parse_value(value)?,
            "Exit status" => self.exit_status = parse_value(value)?,
            _ => debug!("Unrecognized key: {} with value: {}", key, value),
        }
        Ok(())
    }
}

fn parse_value<T: FromStr>(value: &str) -> Result<T>
where
    <T as FromStr>::Err: std::error::Error + Send + Sync + 'static,
{
    value
        .parse()
        .with_context(|| format!("Failed to parse value: {}", value))
}
