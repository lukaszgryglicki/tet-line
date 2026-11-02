use num_complex::Complex64;

pub const USAGE: &str = "\
Usage: tet-line [--screenshot=FILE.png] <base_re> <base_im> \
<plot_from_re> <plot_from_im> <plot_to_re> <plot_to_im> [number_of_points]

Draw complex tetration along the inclusive line from plot_from to plot_to.
The number of points defaults to 512 and must be at least 2.
All numeric arguments must be finite; negative arguments need no extra flags.

Runs PATH's tet at 10-digit precision, once per point, using all available CPUs.
Failed/nonfinite samples leave gaps. Exploration never recalculates values.
x = position along the complex argument line, y = Re(result), z = Im(result).

Mouse: left-drag orbit, right/middle-drag pan, wheel zoom.
Keys: A/D W/S Q/E rotate, Z/X zoom, R auto-rotate, 0 reset, P save PNG, F1 help, Esc quit.
--screenshot=FILE.png saves the initial view and exits.
-h, --help prints this help without opening a window.

Example: tet-line 1.333 -0.06 -2 0 2 0
";

#[derive(Clone, Debug)]
pub struct Config {
    pub base: Complex64,
    pub from: Complex64,
    pub to: Complex64,
    pub points: usize,
    pub screenshot: Option<String>,
}

impl Config {
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Option<Self>, String> {
        let mut positional = Vec::new();
        let mut screenshot = None;
        let mut options = true;
        for arg in args {
            if options {
                if matches!(arg.as_str(), "-h" | "--help") {
                    return Ok(None);
                }
                if arg == "--" {
                    options = false;
                    continue;
                }
                if let Some(path) = arg.strip_prefix("--screenshot=") {
                    if path.is_empty() {
                        return Err("--screenshot requires a file name".to_owned());
                    }
                    if screenshot.replace(path.to_owned()).is_some() {
                        return Err("--screenshot may only be specified once".to_owned());
                    }
                    continue;
                }
                if arg.starts_with("--") {
                    return Err(format!("unknown option: {arg}"));
                }
            }
            positional.push(arg);
        }

        if !(6..=7).contains(&positional.len()) {
            return Err("expected six numeric arguments and an optional point count".to_owned());
        }
        let mut numbers = [0.0; 6];
        for ((number, arg), name) in numbers.iter_mut().zip(&positional).zip([
            "base_re",
            "base_im",
            "plot_from_re",
            "plot_from_im",
            "plot_to_re",
            "plot_to_im",
        ]) {
            *number = arg
                .parse::<f64>()
                .ok()
                .filter(|value| value.is_finite())
                .ok_or_else(|| format!("{name} must be a finite number, got '{arg}'"))?;
        }
        let points = match positional.get(6) {
            Some(arg) => arg
                .parse::<usize>()
                .ok()
                .filter(|&count| count >= 2)
                .ok_or_else(|| "number_of_points must be an integer of at least 2".to_owned())?,
            None => 512,
        };
        Ok(Some(Self {
            base: Complex64::new(numbers[0], numbers[1]),
            from: Complex64::new(numbers[2], numbers[3]),
            to: Complex64::new(numbers[4], numbers[5]),
            points,
            screenshot,
        }))
    }

    pub fn height_at(&self, t: f64) -> Complex64 {
        Complex64::new(
            interpolate(self.from.re, self.to.re, t),
            interpolate(self.from.im, self.to.im, t),
        )
    }

    pub fn height(&self, index: usize) -> Complex64 {
        self.height_at(index as f64 / (self.points - 1) as f64)
    }
}

pub fn interpolate(from: f64, to: f64, t: f64) -> f64 {
    if t == 0.0 {
        from
    } else if t == 1.0 {
        to
    } else {
        let span = to - from;
        if span.is_finite() {
            from + span * t
        } else {
            // Opposite-sign extreme endpoints can overflow in to - from.
            from * (1.0 - t) + to * t
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Option<Config>, String> {
        Config::parse(args.iter().map(|arg| arg.to_string()))
    }

    #[test]
    fn accepts_negative_arguments_and_defaults_to_512_inclusive_samples() {
        let config = parse(&["1.333", "-0.06", "-2", "0", "2", "0"])
            .unwrap()
            .unwrap();
        assert_eq!(config.base, Complex64::new(1.333, -0.06));
        assert_eq!(config.points, 512);
        assert_eq!(config.height(0), config.from);
        assert_eq!(config.height(511), config.to);
    }

    #[test]
    fn interpolates_imaginary_diagonal_reversed_and_degenerate_lines() {
        for (from, to, middle) in [
            ((0.0, -3.0), (0.0, 3.0), (0.0, 0.0)),
            ((-2.0, 1.0), (4.0, -3.0), (1.0, -1.0)),
            ((4.0, -3.0), (-2.0, 1.0), (1.0, -1.0)),
            ((2.0, 3.0), (2.0, 3.0), (2.0, 3.0)),
        ] {
            let config = Config {
                base: Complex64::new(2.0, 0.0),
                from: Complex64::new(from.0, from.1),
                to: Complex64::new(to.0, to.1),
                points: 3,
                screenshot: None,
            };
            assert_eq!(config.height(0), config.from);
            assert_eq!(config.height(1), Complex64::new(middle.0, middle.1));
            assert_eq!(config.height(2), config.to);
        }
        assert_eq!(interpolate(-f64::MAX, f64::MAX, 0.5), 0.0);
        assert!(interpolate(f64::MAX / 2.0, f64::MAX, 0.5).is_finite());
    }

    #[test]
    fn preserves_integer_grid_heights_without_snapping_nearby_fractional_inputs() {
        let config = parse(&["2", "0", "-2", "0", "3", "0", "6"])
            .unwrap()
            .unwrap();
        let heights: Vec<_> = (0..config.points)
            .map(|index| config.height(index))
            .collect();
        let expected: Vec<_> = (-2..=3)
            .map(|height| Complex64::new(f64::from(height), 0.0))
            .collect();
        assert_eq!(heights, expected);
        assert!(interpolate(1.0 - 1e-12, 1.0 - 2e-12, 0.5) < 1.0);
    }

    #[test]
    fn rejects_invalid_numbers_counts_and_options() {
        for bad in ["NaN", "inf", "-inf", "1e999", "not-a-number"] {
            for index in 0..6 {
                let mut args = ["2", "0", "-2", "0", "2", "0"];
                args[index] = bad;
                assert!(parse(&args).is_err(), "{args:?}");
            }
        }
        for count in ["0", "1", "-2", "2.5", "18446744073709551616"] {
            assert!(parse(&["2", "0", "-2", "0", "2", "0", count]).is_err());
        }
        assert!(parse(&[]).is_err());
        assert!(parse(&["2", "0", "-2", "0", "2", "0", "3", "4"]).is_err());
        assert!(parse(&["--unknown"]).is_err());
        assert!(parse(&["--screenshot="]).is_err());
        assert!(parse(&["--screenshot=a.png", "--screenshot=b.png"]).is_err());
    }

    #[test]
    fn accepts_help_and_screenshot_without_changing_positionals() {
        assert!(parse(&["--help"]).unwrap().is_none());
        let config = parse(&["2", "0", "--screenshot=a.png", "0", "-3", "0", "3", "17"])
            .unwrap()
            .unwrap();
        assert_eq!(config.points, 17);
        assert_eq!(config.screenshot.as_deref(), Some("a.png"));
        let config = parse(&[
            "--screenshot=--screenshot=a.png",
            "2",
            "0",
            "0",
            "0",
            "1",
            "0",
        ])
        .unwrap()
        .unwrap();
        assert_eq!(config.screenshot.as_deref(), Some("--screenshot=a.png"));
    }
}
