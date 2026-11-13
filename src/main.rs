mod cli;
mod expression;
mod plot;
mod sampling;
mod viewer;

use cli::{Config, USAGE};
use plot::Plot;
use std::path::Path;
use std::process::ExitCode;
use std::time::Instant;

fn main() -> ExitCode {
    let config = match Config::parse(std::env::args().skip(1), &mut |operation, a, b| {
        sampling::tet(Path::new("tet"), operation, a, b)
    }) {
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
    let workers = std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
        .min(config.points);
    eprintln!(
        "Calculating {} points for f(x) = {} with {workers} workers...",
        config.points,
        config.function.source()
    );
    if config.function.uses_tet() {
        eprintln!(
            "  tet: {} digits, at most {workers} processes, one thread each",
            sampling::PRECISION
        );
    }
    let started = Instant::now();
    let progress_step = (config.points / 20).max(1);
    let samples =
        match sampling::calculate(&config, Path::new("tet"), workers, |completed, missing| {
            if completed == config.points || completed % progress_step == 0 {
                eprintln!(
                    "  {completed}/{} calculated, {missing} skipped",
                    config.points
                );
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
    let plot = match Plot::new(&samples.values) {
        Ok(plot) => plot,
        Err(err) => {
            eprintln!("error: {err}");
            return ExitCode::FAILURE;
        }
    };
    eprintln!(
        "Ready: {}/{} finite points in {:.2}s. Re=[{}, {}], Im=[{}, {}]. No recalculation.",
        plot.finite_count,
        config.points,
        started.elapsed().as_secs_f64(),
        plot.real.min,
        plot.real.max,
        plot.imag.min,
        plot.imag.max
    );
    macroquad::Window::from_config(viewer::window_config(), viewer::run(config, plot));
    ExitCode::SUCCESS
}
