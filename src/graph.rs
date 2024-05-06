use std::collections::HashMap;

use anyhow::Result;
use log::{debug, info};
use plotters::prelude::*;

use crate::database::Database;

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

        // Group jobs by commit_id
        let mut elapsed_times_by_commit: HashMap<String, Vec<(usize, f64)>> = HashMap::new();
        for (index, job) in jobs.iter().enumerate() {
            let commit_group = elapsed_times_by_commit
                .entry(job.commit_id.clone())
                .or_default();
            commit_group.push((index, job.result.elapsed_time));
        }

        // Plot for each commit_id
        for (commit_id, elapsed_times) in &elapsed_times_by_commit {
            let file_path = format!(
                "{}/{}_{}.png",
                output_path,
                job_name.replace("./", "").replace(' ', "_"),
                commit_id
            );
            debug!("Using filepath: {:?} for graph", file_path);
            let root = BitMapBackend::new(&file_path, (1920, 1080)).into_drawing_area();
            root.fill(&WHITE)?;

            let max_x = elapsed_times.len() - 1;
            let max_y = elapsed_times
                .iter()
                .map(|&(_, time)| time)
                .fold(0.0, f64::max);

            let mut chart = ChartBuilder::on(&root)
                .caption(
                    format!("Elapsed Time for {} [Commit: {}]", job_name, commit_id),
                    ("sans-serif", 50),
                )
                .x_label_area_size(50)
                .y_label_area_size(80)
                .margin(10)
                .build_cartesian_2d(0..max_x, 0.0..max_y)?;

            chart
                .configure_mesh()
                .x_desc("Run Index")
                .y_desc("Elapsed Time (s)")
                .axis_desc_style(("sans-serif", 30))
                .draw()?;

            chart
                .draw_series(LineSeries::new(elapsed_times.to_vec(), &RED))?
                .label("Elapsed Time")
                .legend(move |(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], RED));

            chart
                .configure_series_labels()
                .background_style(WHITE.mix(0.8))
                .border_style(BLACK)
                .draw()?;

            root.present()?;
            info!(
                "Plot for {} [Commit: {}] created at {}",
                job_name, commit_id, file_path
            );
        }
    }

    Ok(())
}
