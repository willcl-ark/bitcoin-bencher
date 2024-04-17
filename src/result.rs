use anyhow::{bail, Context, Result};
use log::debug;

extern crate exitcode;

use std::fs::File;
use std::io::BufRead;

#[derive(Debug, Default)]
pub struct TimeResult {
    pub command: String,
    pub user_time: f64,
    pub system_time: f64,
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

impl TimeResult {
    fn parse_time_to_seconds(input: &str) -> Result<f64> {
        let parts: Vec<&str> = input.split(':').collect();
        let (hours, minutes, seconds_parts) = match parts.len() {
            2 => (
                0,
                parts[0].parse::<i32>()?,
                parts[1].split('.').collect::<Vec<&str>>(),
            ),
            3 => (
                parts[0].parse::<i32>()?,
                parts[1].parse::<i32>()?,
                parts[2].split('.').collect::<Vec<&str>>(),
            ),
            _ => bail!("Invalid time format. Expected HH:MM:SS.ss or MM:SS.ss"),
        };

        if seconds_parts.len() != 2 {
            bail!("Invalid seconds format. Expected SS.ss");
        }
        let seconds: i32 = seconds_parts[0].parse()?;
        let fractional: f64 = format!("0.{}", seconds_parts[1]).parse()?;
        Ok(hours as f64 * 3600.0 + minutes as f64 * 60.0 + seconds as f64 + fractional)
    }

    fn parse_line(&mut self, line: &str) -> Result<()> {
        let parts: Vec<&str> = line.rsplitn(2, ": ").collect();
        if parts.len() == 2 {
            let value = parts[0].trim();
            let key = parts[1].trim();
            match key {
                // This removes quote marks from the time -v output
                "Command being timed" => self.command = value.to_string().replace('"', ""),
                "User time (seconds)" => self.user_time = value.parse()?,
                "System time (seconds)" => self.system_time = value.parse()?,
                "Percent of CPU this job got" => {
                    self.percent_of_cpu = value.trim_end_matches('%').parse()?
                }
                "Elapsed (wall clock) time (h:mm:ss or m:ss)" => {
                    self.elapsed_time = Self::parse_time_to_seconds(value)?
                }
                "Maximum resident set size (kbytes)" => {
                    self.max_resident_set_size_kb = value.parse()?
                }
                "Major (requiring I/O) page faults" => self.major_page_faults = value.parse()?,
                "Minor (reclaiming a frame) page faults" => {
                    self.minor_page_faults = value.parse()?
                }
                "Voluntary context switches" => self.voluntary_context_switches = value.parse()?,
                "Involuntary context switches" => {
                    self.involuntary_context_switches = value.parse()?
                }
                "File system outputs" => self.file_system_outputs = value.parse()?,
                "Exit status" => self.exit_status = value.parse()?,
                _ => {
                    debug!("Failed to match key: {} against Result struct", key);
                }
            }
        }
        Ok(())
    }

    pub fn from_file(file_path: &str) -> Result<Self> {
        let file =
            File::open(file_path).with_context(|| format!("Failed to open file: {}", file_path))?;
        let reader = std::io::BufReader::new(file);
        let mut result = TimeResult::default();

        for line in reader.lines() {
            let line = line.with_context(|| "Failed to read line from file")?;
            result.parse_line(&line)?;
        }
        Ok(result)
    }
}
