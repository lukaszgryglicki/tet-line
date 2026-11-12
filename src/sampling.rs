use crate::cli::Config;
use crate::expression::Operation;
use num_complex::Complex64;
use std::io;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc;
use std::thread;

pub const PRECISION: usize = 10;

pub struct Samples {
    pub values: Vec<Option<Complex64>>,
    pub failure_examples: Vec<String>,
}

#[derive(Debug)]
enum SampleError {
    Launch(io::Error),
    Missing(String),
}

pub fn calculate(
    config: &Config,
    program: &Path,
    workers: usize,
    progress: impl FnMut(usize, usize),
) -> Result<Samples, String> {
    calculate_with(
        config,
        workers,
        |index| {
            let value = config
                .function
                .evaluate(config.height(index), &mut |operation, a, b| {
                    evaluate(program, operation, a, b)
                })?;
            if value.is_finite() {
                Ok(value)
            } else {
                Err(SampleError::Missing(
                    "expression returned a nonfinite value".to_owned(),
                ))
            }
        },
        progress,
    )
}

fn calculate_with(
    config: &Config,
    workers: usize,
    evaluate: impl Fn(usize) -> Result<Complex64, SampleError> + Sync,
    mut progress: impl FnMut(usize, usize),
) -> Result<Samples, String> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(config.points)
        .map_err(|err| format!("cannot allocate {} samples: {err}", config.points))?;
    values.resize(config.points, None);
    let next = AtomicUsize::new(0);
    let stopped = AtomicBool::new(false);
    let (sender, receiver) = mpsc::channel();

    thread::scope(|scope| {
        for _ in 0..workers.max(1).min(config.points) {
            let sender = sender.clone();
            let (next, stopped, evaluate) = (&next, &stopped, &evaluate);
            scope.spawn(move || {
                while !stopped.load(Ordering::Relaxed) {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    if index >= config.points {
                        break;
                    }
                    let result = evaluate(index);
                    if matches!(result, Err(SampleError::Launch(_))) {
                        stopped.store(true, Ordering::Relaxed);
                    }
                    if sender.send((index, result)).is_err() {
                        break;
                    }
                }
            });
        }
        drop(sender);

        let mut fatal = None;
        let mut failure_examples = Vec::new();
        let (mut completed, mut missing) = (0, 0);
        for (index, result) in receiver {
            completed += 1;
            match result {
                Ok(value) => values[index] = Some(value),
                Err(SampleError::Missing(reason)) => {
                    missing += 1;
                    if failure_examples.len() < 3 {
                        failure_examples.push(format!(
                            "sample {} at x={}: {reason}",
                            index + 1,
                            config.height(index)
                        ));
                    }
                }
                Err(SampleError::Launch(err)) => {
                    fatal.get_or_insert_with(|| {
                        format!("could not run tet: {err}; ensure the tet executable is on PATH")
                    });
                }
            }
            progress(completed, missing);
        }
        match fatal {
            Some(err) => Err(err),
            None => Ok(Samples {
                values,
                failure_examples,
            }),
        }
    })
}

fn evaluate(
    program: &Path,
    operation: Operation,
    a: Complex64,
    b: Complex64,
) -> Result<Complex64, SampleError> {
    if !a.is_finite() || !b.is_finite() {
        return Err(SampleError::Missing(
            "tetration operands must be finite".to_owned(),
        ));
    }
    let mode: &[&str] = match operation {
        Operation::Tetration => &[],
        Operation::SuperLog => &["-q", "--slog"],
        Operation::SuperRoot => &["-q", "--sroot"],
    };
    let output = Command::new(program)
        .args(mode)
        .args([
            PRECISION.to_string(),
            a.re.to_string(),
            a.im.to_string(),
            b.re.to_string(),
            b.im.to_string(),
        ])
        .env("SILENT", "1")
        .env("RAYON_NUM_THREADS", "1")
        .stdin(Stdio::null())
        .output()
        .map_err(SampleError::Launch)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let reason: String = stderr.trim().chars().take(200).collect();
        return Err(SampleError::Missing(if reason.is_empty() {
            format!("tet exited with {}", output.status)
        } else {
            format!("tet exited with {}: {reason}", output.status)
        }));
    }
    parse_output(&output.stdout).map_err(SampleError::Missing)
}

fn parse_output(output: &[u8]) -> Result<Complex64, String> {
    let text = std::str::from_utf8(output).map_err(|_| "tet output is not UTF-8".to_owned())?;
    let mut parts = text.split_whitespace();
    let mut number = || {
        parts
            .next()
            .and_then(|part| part.parse::<f64>().ok())
            .filter(|value| value.is_finite())
            .ok_or_else(|| "tet did not return two finite f64 numbers".to_owned())
    };
    let value = Complex64::new(number()?, number()?);
    if parts.next().is_some() {
        return Err("tet returned unexpected extra output".to_owned());
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Barrier;
    use std::time::Duration;

    fn config(points: usize) -> Config {
        Config {
            function: crate::expression::Expression::tetration(Complex64::new(1.333, -0.06)),
            from: Complex64::new(-2.0, 1.0),
            to: Complex64::new(2.0, -1.0),
            points,
            screenshot: None,
        }
    }

    #[test]
    fn parses_real_tet_format_and_rejects_missing_or_nonfinite_values() {
        assert_eq!(
            parse_output(b"1.835609672\n5.827999776e-1\n").unwrap(),
            Complex64::new(1.835609672, 0.5827999776)
        );
        assert_eq!(
            parse_output(b"  -1.2e+12\r\n-0\r\n").unwrap(),
            Complex64::new(-1.2e12, -0.0)
        );
        for output in ["", "1", "1\nbad", "NaN\n0", "0\ninf", "1e999\n0", "1\n2\n3"] {
            assert!(parse_output(output.as_bytes()).is_err(), "{output}");
        }
        assert!(parse_output(&[0xff, b'\n', b'0']).is_err());
    }

    #[test]
    fn uses_every_worker_once_per_sample_and_preserves_order_and_gaps() {
        let config = config(17);
        let workers = 4;
        let barrier = Barrier::new(workers);
        let active = AtomicUsize::new(0);
        let peak = AtomicUsize::new(0);
        let calls: Vec<_> = (0..config.points).map(|_| AtomicUsize::new(0)).collect();
        let mut progress = Vec::new();
        let samples = calculate_with(
            &config,
            workers,
            |index| {
                calls[index].fetch_add(1, Ordering::SeqCst);
                let count = active.fetch_add(1, Ordering::SeqCst) + 1;
                peak.fetch_max(count, Ordering::SeqCst);
                if index < workers {
                    barrier.wait();
                }
                thread::sleep(Duration::from_millis((3 - index % 4) as u64));
                active.fetch_sub(1, Ordering::SeqCst);
                if index == 5 {
                    Err(SampleError::Missing("unsupported height".to_owned()))
                } else {
                    Ok(config.height(index))
                }
            },
            |completed, missing| progress.push((completed, missing)),
        )
        .unwrap();

        assert_eq!(peak.load(Ordering::SeqCst), workers);
        assert!(calls.iter().all(|count| count.load(Ordering::SeqCst) == 1));
        for (index, value) in samples.values.iter().enumerate() {
            assert_eq!(*value, (index != 5).then(|| config.height(index)));
        }
        assert_eq!(samples.failure_examples.len(), 1);
        assert_eq!(progress.len(), config.points);
        assert_eq!(progress.last(), Some(&(config.points, 1)));
    }

    #[test]
    fn missing_executable_is_fatal_rather_than_a_plot_gap() {
        let directory = tempfile::tempdir().unwrap();
        let result = calculate(
            &config(3),
            &directory.path().join("missing-tet"),
            2,
            |_, _| {},
        );
        assert!(result
            .err()
            .unwrap()
            .contains("ensure the tet executable is on PATH"));
    }

    #[cfg(unix)]
    #[test]
    fn invokes_the_tet_protocol_and_does_not_accept_failed_process_output() {
        use std::os::unix::fs::PermissionsExt;

        let directory = tempfile::tempdir().unwrap();
        let program = directory.path().join("tet");
        std::fs::write(
            &program,
            r#"#!/bin/sh
if [ "$#" -ne 5 ] || [ "$1" != 10 ] || [ "$2" != 1.333 ] || [ "$3" != -0.06 ] ||
   [ "$SILENT" != 1 ] || [ "$RAYON_NUM_THREADS" != 1 ]; then
    echo 'incorrect invocation' >&2
    exit 2
fi
printf '%s\n' "$4" >> "$0.calls"
printf '%s\n%s\n' "$4" "$5"
if [ "$4" = 0 ]; then
    echo 'unsupported height' >&2
    exit 1
fi
"#,
        )
        .unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
        let config = config(5);
        let samples = calculate(&config, &program, 3, |_, _| {}).unwrap();
        for (index, value) in samples.values.iter().enumerate() {
            assert_eq!(*value, (index != 2).then(|| config.height(index)));
        }
        assert!(samples.failure_examples[0].contains("unsupported height"));
        let calls = std::fs::read_to_string(directory.path().join("tet.calls")).unwrap();
        let mut calls: Vec<f64> = calls.lines().map(|line| line.parse().unwrap()).collect();
        calls.sort_by(f64::total_cmp);
        assert_eq!(calls, [-2.0, -1.0, 0.0, 1.0, 2.0]);
    }

    #[test]
    fn expressions_without_tetration_need_no_executable() {
        let mut config = config(5);
        config.function = crate::expression::Expression::parse("sin(x)+x^2+3i").unwrap();
        let directory = tempfile::tempdir().unwrap();
        let samples =
            calculate(&config, &directory.path().join("missing-tet"), 3, |_, _| {}).unwrap();
        assert!(samples.failure_examples.is_empty());
        for (index, value) in samples.values.iter().enumerate() {
            let x = config.height(index);
            let expected = x.sin() + x * x + Complex64::new(0.0, 3.0);
            assert!((value.unwrap() - expected).norm() < 1e-12);
        }
    }

    #[test]
    fn nonfinite_expression_results_leave_gaps_but_predicates_can_handle_them() {
        let mut config = config(3);
        config.from = Complex64::new(-1.0, 0.0);
        config.to = Complex64::new(1.0, 0.0);
        let directory = tempfile::tempdir().unwrap();
        let program = directory.path().join("missing-tet");
        config.function = crate::expression::Expression::parse("1/x").unwrap();
        let samples = calculate(&config, &program, 2, |_, _| {}).unwrap();
        assert_eq!(
            samples.values,
            [
                Some(Complex64::new(-1.0, 0.0)),
                None,
                Some(Complex64::new(1.0, 0.0)),
            ]
        );
        assert_eq!(samples.failure_examples.len(), 1);
        assert!(samples.failure_examples[0].contains("nonfinite"));
        config.function = crate::expression::Expression::parse("isfinite(1/x)").unwrap();
        let samples = calculate(&config, &program, 2, |_, _| {}).unwrap();
        assert_eq!(
            samples.values,
            [
                Some(Complex64::new(1.0, 0.0)),
                Some(Complex64::new(0.0, 0.0)),
                Some(Complex64::new(1.0, 0.0)),
            ]
        );
    }

    #[test]
    fn nonfinite_tetration_operands_do_not_launch_a_process() {
        let mut config = config(2);
        config.function = crate::expression::Expression::parse("(1/x)^^x").unwrap();
        config.from = Complex64::new(0.0, 0.0);
        config.to = config.from;
        let directory = tempfile::tempdir().unwrap();
        let samples =
            calculate(&config, &directory.path().join("missing-tet"), 2, |_, _| {}).unwrap();
        assert_eq!(samples.values, [None, None]);
        assert!(samples
            .failure_examples
            .iter()
            .all(|reason| reason.contains("tetration operands must be finite")));
    }

    #[cfg(unix)]
    #[test]
    fn expression_tetration_passes_computed_base_and_height_through_the_protocol() {
        use std::os::unix::fs::PermissionsExt;

        let directory = tempfile::tempdir().unwrap();
        let program = directory.path().join("tet");
        std::fs::write(
            &program,
            r#"#!/bin/sh
if [ "$#" -ne 5 ] || [ "$1" != 10 ] || [ "$SILENT" != 1 ] || [ "$RAYON_NUM_THREADS" != 1 ]; then
    echo 'incorrect invocation' >&2
    exit 2
fi
printf '%s %s %s %s\n' "$2" "$3" "$4" "$5" >> "$0.calls"
printf '%s\n%s\n' "$2" "$4"
"#,
        )
        .unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
        let mut config = config(3);
        config.from = Complex64::new(0.0, 0.0);
        config.to = Complex64::new(2.0, 0.0);
        config.function =
            crate::expression::Expression::parse("(x+1+2i)^^(2*x-i)-x^2+sin(x)").unwrap();
        let samples = calculate(&config, &program, 3, |_, _| {}).unwrap();
        for (index, value) in samples.values.iter().enumerate() {
            let x = config.height(index);
            let expected = Complex64::new(x.re + 1.0, 2.0 * x.re) - x * x + x.sin();
            assert!((value.unwrap() - expected).norm() < 1e-12);
        }
        let calls = std::fs::read_to_string(directory.path().join("tet.calls")).unwrap();
        let mut calls: Vec<_> = calls.lines().collect();
        calls.sort_unstable();
        assert_eq!(calls, ["1 2 0 -1", "2 2 2 -1", "3 2 4 -1"]);
    }

    #[cfg(unix)]
    #[test]
    fn inverse_functions_run_the_quiet_slog_and_sroot_modes_with_operands_in_order() {
        use std::os::unix::fs::PermissionsExt;

        let directory = tempfile::tempdir().unwrap();
        let program = directory.path().join("tet");
        std::fs::write(
            &program,
            r#"#!/bin/sh
if [ "$SILENT" != 1 ] || [ "$RAYON_NUM_THREADS" != 1 ]; then
    echo 'incorrect environment' >&2
    exit 2
fi
case "$#,$1,$2,$3" in
    5,10,*) mode=tet; result=1 ;;
    7,-q,--slog,10) mode=slog; result=10; shift 2 ;;
    7,-q,--sroot,10) mode=sroot; result=100; shift 2 ;;
    *) echo 'incorrect invocation' >&2; exit 2 ;;
esac
printf '%s %s %s %s %s\n' "$mode" "$2" "$3" "$4" "$5" >> "$0.calls"
printf '%s\n0\n' "$result"
"#,
        )
        .unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
        let mut config = config(2);
        config.from = Complex64::new(1.25, -0.5);
        config.to = config.from;
        config.function =
            crate::expression::Expression::parse("x^^x + ate(2+i, x) - ssrt(x, 1.5-i)").unwrap();
        let samples = calculate(&config, &program, 1, |_, _| {}).unwrap();
        assert_eq!(samples.values, [Some(Complex64::new(-89.0, 0.0)); 2]);
        let calls = std::fs::read_to_string(directory.path().join("tet.calls")).unwrap();
        assert_eq!(
            calls,
            "tet 1.25 -0.5 1.25 -0.5\nslog 2 1 1.25 -0.5\nsroot 1.25 -0.5 1.5 -1\n".repeat(2)
        );
    }
}
