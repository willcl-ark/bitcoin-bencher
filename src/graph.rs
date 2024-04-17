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

        let mut elapsed_times: Vec<(usize, f64)> = Vec::new();

        for (index, job) in jobs.iter().enumerate() {
            elapsed_times.push((index, job.result.elapsed_time));
        }

        let file_path = format!(
            "{}/{}.png",
            output_path,
            job_name.replace("./", "").replace(' ', "_")
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
            .caption(format!("Elapsed Time for {}", job_name), ("sans-serif", 50))
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
            .draw_series(LineSeries::new(elapsed_times, &RED))?
            .label("Elapsed Time")
            .legend(move |(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], RED));

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
