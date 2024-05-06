use std::collections::HashMap;

use anyhow::Result;
use log::{debug, info};
use plotters::prelude::*;

use crate::database::{Database, Job};

pub fn plot_job_metrics(db: &Database, output_path: &str) -> Result<()> {
    let all_job_names = db.get_job_names()?;

    for job_name in all_job_names {
        info!("Starting graph for {}", &job_name);
        let jobs = db.get_jobs_by_name(&job_name)?;
        debug!(
            "Got {} jobs from the database for {}",
            jobs.len(),
            &job_name
        );

        // Group jobs by commit_id and sort them
        let mut jobs_by_commit: HashMap<String, Vec<Job>> = HashMap::new();
        for job in jobs {
            jobs_by_commit
                .entry(job.commit_id.clone())
                .or_default()
                .push(job);
        }

        let commits: Vec<_> = jobs_by_commit.keys().cloned().collect();
        let mut commit_indices = HashMap::new();
        for (index, commit) in commits.iter().enumerate() {
            commit_indices.insert(commit, index);
        }

        // Prepare plot
        let file_path = format!(
            "{}/{}.png",
            output_path,
            job_name.replace("./", "").replace(' ', "_")
        );
        debug!("Using filepath: {:?} for graph", file_path);
        let root = BitMapBackend::new(&file_path, (1920, 1080)).into_drawing_area();
        root.fill(&WHITE)?;

        // Calculate the maximum user time to set the y-axis limit
        let max_y = jobs_by_commit
            .values()
            .flat_map(|jobs| jobs.iter().map(|job| job.result.user_time))
            .fold(0.0, f64::max);

        let max_x = jobs_by_commit.len() as i32 - 1;

        let mut chart = ChartBuilder::on(&root)
            .caption(format!("User Time for {}", job_name), ("sans-serif", 50))
            .x_label_area_size(50)
            .y_label_area_size(80)
            .margin(10)
            .build_cartesian_2d(0..max_x, 0.0..max_y)?;

        chart
            .configure_mesh()
            .x_labels(commits.len())
            .x_label_formatter(&|x| commits[*x as usize].clone())
            .y_desc("User Time (s)")
            .axis_desc_style(("sans-serif", 30))
            .draw()?;

        // Collect data points for the single series
        let data_points: Vec<_> = jobs_by_commit
            .iter()
            .map(|(commit_id, jobs)| {
                let x = *commit_indices.get(commit_id).unwrap() as i32;
                let y =
                    jobs.iter().map(|job| job.result.user_time).sum::<f64>() / jobs.len() as f64; // Average time or sum
                (x, y)
            })
            .collect();

        // Plot the single series
        chart
            .draw_series(LineSeries::new(data_points, &RED))?
            .label(job_name.clone())
            .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], RED));

        chart
            .configure_series_labels()
            .background_style(WHITE.mix(0.8))
            .border_style(BLACK)
            .draw()?;

        root.present()?;
        info!("Plot for {} created at {}", job_name, file_path);
    }

    Ok(())
}
