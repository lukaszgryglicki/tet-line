mod cli;
mod expression;
mod plot;
mod sampling;
mod video;
mod viewer;

use cli::{Config, USAGE};
use plot::Plot;
use std::path::Path;
use std::process::ExitCode;
use std::sync::{Arc, Mutex};
use std::time::Instant;

fn main() -> ExitCode {
    let mut config = match Config::parse(
        std::env::args().skip(1),
        &mut |precision, operation, a, b| {
            sampling::tet(Path::new("tet"), precision, operation, a, b)
        },
    ) {
        Ok(Some(config)) => config,
        Ok(None) => {
            print!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Err(err) => {
            eprintln!("error: {err}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    if config.screenshot.is_some() && config.animation.take().is_some() {
        eprintln!(
            "--screenshot saves the first line of the animation; the others are not calculated."
        );
    }
    if let (Some(file), Some(animation)) = (&config.mp4, &config.animation) {
        // Before the calculation, which can be long.
        if let Err(err) = video::check(
            Path::new(video::FFMPEG),
            Path::new(file),
            &animation.rate(),
            config.lossy,
        ) {
            eprintln!("error: {err}");
            return ExitCode::FAILURE;
        }
    }
    let total = config.frames().saturating_mul(config.points);
    let workers = std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
        .min(total);
    if config.animation.is_some() {
        eprintln!(
            "Calculating {} frames of {} points ({total} values) for f(x) = {} with {workers} workers...",
            config.frames(),
            config.points,
            config.function.source()
        );
    } else {
        eprintln!(
            "Calculating {} points for f(x) = {} with {workers} workers...",
            config.points,
            config.function.source()
        );
    }
    if config.function.uses_tet() {
        eprintln!(
            "  tet: {}, at most {workers} processes, one thread each",
            config.tet_digits()
        );
    }
    let started = Instant::now();
    let progress_step = (total / 20).max(1);
    let samples =
        match sampling::calculate(&config, Path::new("tet"), workers, |completed, missing| {
            if completed == total || completed % progress_step == 0 {
                eprintln!("  {completed}/{total} calculated, {missing} skipped");
            }
        }) {
            Ok(samples) => samples,
            Err(err) => {
                eprintln!("error: {err}");
                return ExitCode::FAILURE;
            }
        };
    for reason in &samples.failure_examples {
        eprintln!("  Skipped {reason}");
    }
    let plots = match Plot::frames(&samples.values, config.points) {
        Ok(plots) => plots,
        Err(err) => {
            eprintln!("error: {err}");
            return ExitCode::FAILURE;
        }
    };
    eprintln!(
        "Ready: {}/{total} finite points in {:.2}s. Re=[{}, {}], Im=[{}, {}]. No recalculation.",
        plots.iter().map(|plot| plot.finite_count).sum::<usize>(),
        started.elapsed().as_secs_f64(),
        plots[0].real.min,
        plots[0].real.max,
        plots[0].imag.min,
        plots[0].imag.max
    );
    let plots = if config.auto_scale_frames {
        Plot::auto_scaled(plots, &samples.values, config.points)
    } else {
        plots
    };
    let failure = Arc::new(Mutex::new(None));
    let result = failure.clone();
    macroquad::Window::from_config(viewer::window_config(config.mp4.is_some()), async move {
        *result.lock().unwrap() = viewer::run(config, plots).await.err();
    });
    let failure = failure.lock().unwrap().take();
    if let Some(err) = failure {
        eprintln!("error: {err}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
