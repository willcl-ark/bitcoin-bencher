use anyhow::Result;
use log::{debug, info};
use plotters::{prelude::*, style::full_palette::PURPLE};

use crate::database::Database;

pub fn plot_job_metrics(db: &Database, output_path: &str) -> Result<()> {
    let job_name = "IBD".to_string();
    info!("Starting graph for {}", job_name);

    let jobs_with_runs = db.get_jobs_by_name(&job_name)?;
    debug!(
        "Got {} jobs from the database for {}",
        jobs_with_runs.len(),
        job_name
    );

    let file_path = format!(
        "{}/{}.png",
        output_path,
        job_name.replace("./", "").replace(' ', "_")
    );
    debug!("Using filepath: {:?} for graph", file_path);
    let root = BitMapBackend::new(&file_path, (1920, 1080)).into_drawing_area();
    root.fill(&WHITE)?;

    // Calculate the maximum user time to set the y-axis limit
    let max_user_time = jobs_with_runs
        .iter()
        .map(|(job, _)| job.result.user_time)
        .fold(0.0, f64::max);

    // Calculate the maximum RSS to set the y-axis limit
    let max_rss = jobs_with_runs
        .iter()
        .map(|(job, _)| job.result.max_resident_set_size_kb as f64)
        .fold(0.0, f64::max);

    let min_date = jobs_with_runs
        .iter()
        .map(|(_, run)| run.run_date)
        .min()
        .unwrap_or(0);
    let max_date = jobs_with_runs
        .iter()
        .map(|(_, run)| run.run_date)
        .max()
        .unwrap_or(0);

    let mut chart = ChartBuilder::on(&root)
        .caption(
            format!("User Time and Max RSS for {}", job_name),
            ("sans-serif", 50),
        )
        .x_label_area_size(50)
        .y_label_area_size(80)
        .right_y_label_area_size(80)
        .margin(10)
        .build_cartesian_2d(min_date..max_date, 0.0..max_user_time)?
        .set_secondary_coord(min_date..max_date, 0.0..max_rss);

    chart
        .configure_mesh()
        .x_labels(10)
        .x_label_formatter(&|x| format!("{}", x))
        .y_desc("User Time (s)")
        .axis_desc_style(("sans-serif", 30))
        .draw()?;

    chart
        .configure_secondary_axes()
        .y_desc("Max RSS (KB)")
        .axis_desc_style(("sans-serif", 30))
        .draw()?;

    // Collect data points for master and non-master jobs
    let master_points_user_time: Vec<_> = jobs_with_runs
        .iter()
        .filter(|(_, run)| run.was_master)
        .map(|(job, run)| (run.run_date, job.result.user_time))
        .collect();

    let non_master_points_user_time: Vec<_> = jobs_with_runs
        .iter()
        .filter(|(_, run)| !run.was_master)
        .map(|(job, run)| (run.run_date, job.result.user_time))
        .collect();

    let master_points_rss: Vec<_> = jobs_with_runs
        .iter()
        .filter(|(_, run)| run.was_master)
        .map(|(job, run)| (run.run_date, job.result.max_resident_set_size_kb as f64))
        .collect();

    let non_master_points_rss: Vec<_> = jobs_with_runs
        .iter()
        .filter(|(_, run)| !run.was_master)
        .map(|(job, run)| (run.run_date, job.result.max_resident_set_size_kb as f64))
        .collect();

    // Plot master jobs user time
    chart
        .draw_series(LineSeries::new(master_points_user_time.clone(), &RED))?
        .label("Master User Time")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], RED));

    // Plot non-master jobs user time
    chart
        .draw_series(PointSeries::of_element(
            non_master_points_user_time.clone(),
            5,
            &BLUE,
            &|c, _s, _st| {
                return EmptyElement::at(c)
                    + Text::new(format!("{:?}", c), (0, 15), ("sans-serif", 15).into_font());
            },
        ))?
        .label("Non-Master User Time")
        .legend(|(x, y)| Circle::new((x + 10, y), 5, BLUE.filled()));

    // Plot master jobs RSS
    chart
        .draw_secondary_series(LineSeries::new(master_points_rss.clone(), &GREEN))?
        .label("Master Max RSS")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], GREEN));

    // Plot non-master jobs RSS
    chart
        .draw_secondary_series(PointSeries::of_element(
            non_master_points_rss.clone(),
            5,
            &PURPLE,
            &|c, _s, _st| {
                return EmptyElement::at(c)
                    + Text::new(format!("{:?}", c), (0, 15), ("sans-serif", 15).into_font());
            },
        ))?
        .label("Non-Master Max RSS")
        .legend(|(x, y)| Circle::new((x + 10, y), 5, PURPLE.filled()));

    chart
        .configure_series_labels()
        .background_style(WHITE.mix(0.8))
        .border_style(BLACK)
        .draw()?;

    root.present()?;
    info!("Plot for {} created at {}", job_name, file_path);

    Ok(())
}
