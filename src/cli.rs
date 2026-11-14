use crate::expression::{Expression, Operation};
use num_complex::Complex64;

pub const USAGE: &str = "\
Usage: tet-line [--screenshot=FILE.png] <expression> \
<plot_from_re> <plot_from_im> <plot_to_re> <plot_to_im> [number_of_points]
       tet-line [--screenshot=FILE.png] <expression> <plot_from> <plot_to> [number_of_points]
       tet-line [--screenshot=FILE.png] --expr=EXPRESSION \
<plot_from_re> <plot_from_im> <plot_to_re> <plot_to_im> [number_of_points]
       tet-line [--screenshot=FILE.png] --expr=EXPRESSION <plot_from> <plot_to> [number_of_points]
       tet-line [--screenshot=FILE.png] <base_re> <base_im> \
<plot_from_re> <plot_from_im> <plot_to_re> <plot_to_im> [number_of_points]

Draw a complex function along the inclusive line from plot_from to plot_to.
The two ends of the line are four real coordinates, or two complex points:
-2 -1 2 1 and -2-i 2+i are the same line.
The number of points defaults to 512 and must be at least 2.
Line/base coordinates must be finite; negative arguments need no extra flags.
Quote expressions. x (or z) is the complex point on the argument line.
Operators: + - * / ^ ^^, parentheses and implicit multiplication (2x, 3i).
^^ is tetration and binds tighter than ^; both associate right, before unary -.
tet(b,h) is b^^h written as a function.
Constants: i, j, pi, tau, e. Functions match rust-opengl-complex-drawer, including
sin, cos, tan, exp, ln, log, sqrt, abs, arg, re, im, conj, pow, gamma and factorial.
A zero has no sign, as in tet: sqrt(-4) is 2i and ln(-1) is pi i.
slog(b,z) (or ate) is the height h and ssrt(z,h) (or sroot) the base b in b^^h = z.
Named real constants (aliases after /): pi, e, tau, phi/golden, gamma/eulergamma
(gamma( is still the function), catalan, apery/zeta3, zeta2/basel, ln2, ln10, sqrt2,
sqrt3, sqrt5, sqrtpi, halfpi, eta/towermax = e^(1/e), towermin = e^(-e), einv,
omega, dottie, gelfond, gelfondschneider, ramanujan, silver, plastic, supergolden,
tribonacci, lemniscate, gauss, levy, khinchin, erdosborwein, cahen, recfib,
gompertz, soldner, laplace, parabolic, deg/degree, niven, twinprime, mertens.
Unicode aliases: \u{3c0}, \u{3c4}, \u{3c6}, \u{3b3}, \u{3b7}, \u{3a9}, \u{3d6} (lemniscate), \
\u{221a}2, \u{221a}3, \u{221a}5, \u{221a}\u{3c0}.
A coordinate that is not a plain number is a constant expression without x or z,
evaluated once (^^, tet, slog and ssrt run tet) to a finite real value: -pi 0 tau/3 0,
'pi*e^e-pow(2, 0.5)' or 1.5^^2; as base_re it needs number_of_points.
plot_from and plot_to are such expressions with a finite complex value:
'e^-e' e, -2-i 2+i or 'exp(i*pi/4)' '2^^0.5'.
Use --expr=EXPRESSION for numeric constants with a point count; six/seven
numeric positionals retain the original fixed-base tetration format.

Each ^^ or tet(b,h) runs PATH's tet at 13-digit precision; other math is local.
slog/ssrt run tet's --slog/--sroot modes; ssrt at non-integer heights can be slow.
Uses all available CPUs. Expressions without ^^, tet, slog or ssrt do not need tet.
Failed/nonfinite samples leave gaps. Exploration never recalculates values.
x = position along the complex argument line, y = Re(result), z = Im(result).

Mouse: left-drag orbit, right/middle-drag pan, wheel zoom.
Keys: A/D W/S Q/E rotate, Z/X zoom, R auto-rotate, 0 reset, P save PNG, F1 help, Esc quit.
--screenshot=FILE.png saves the initial view and exits.
-h, --help prints this help without opening a window.

Examples: tet-line 'x^^x-x^x+x' 1 0 2 0
          tet-line 'x^^x-x^x+x' 1 2
          tet-line 'sin(x)+exp(i*x)' -2 0 2 0
          tet-line 'sin(x)+exp(i*x)' -2-i 2+i 1024
          tet-line 1.333 -0.06 -2 0 2 0
";

#[derive(Clone, Debug)]
pub struct Config {
    pub function: Expression,
    pub from: Complex64,
    pub to: Complex64,
    pub points: usize,
    pub screenshot: Option<String>,
}

impl Config {
    pub fn parse(
        args: impl IntoIterator<Item = String>,
        tet: &mut impl FnMut(Operation, Complex64, Complex64) -> Result<Complex64, String>,
    ) -> Result<Option<Self>, String> {
        let mut positional = Vec::new();
        let mut screenshot = None;
        let mut expression = None;
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
                if let Some(source) = arg.strip_prefix("--expr=") {
                    if expression.replace(source.to_owned()).is_some() {
                        return Err("--expr may only be specified once".to_owned());
                    }
                    continue;
                }
                if arg.starts_with("--") {
                    return Err(format!("unknown option: {arg}"));
                }
            }
            positional.push(arg);
        }

        if expression.is_none()
            && ((3..=5).contains(&positional.len())
                || (positional.len() == 6 && positional[0].parse::<f64>().is_err()))
        {
            expression = Some(positional.remove(0));
        }
        let coordinate_count = match (expression.is_some(), positional.len()) {
            (true, 2 | 3) => 2,
            (true, 4 | 5) => 4,
            (false, 6 | 7) => 6,
            _ => {
                return Err(
                    "expected an expression and two complex points or four real coordinates, \
                     or six legacy coordinates, followed by an optional point count"
                        .to_owned(),
                )
            }
        };
        let mut numbers = [0.0; 6];
        if coordinate_count == 2 {
            for ((point, arg), name) in numbers[2..]
                .chunks_mut(2)
                .zip(&positional)
                .zip(["plot_from", "plot_to"])
            {
                let value = match arg.parse::<f64>() {
                    Ok(value) => Complex64::new(value, 0.0),
                    Err(_) => constant_value(arg, tet).map_err(|reason| {
                        format!(
                            "{name} must be a finite complex number or a constant expression \
                             without variables, got '{arg}': {reason}"
                        )
                    })?,
                };
                if !value.is_finite() {
                    return Err(format!(
                        "{name} must be a finite complex number, got '{arg}' = {value}"
                    ));
                }
                point.copy_from_slice(&[value.re, value.im]);
            }
        } else {
            let offset = 6 - coordinate_count;
            let names = [
                "base_re",
                "base_im",
                "plot_from_re",
                "plot_from_im",
                "plot_to_re",
                "plot_to_im",
            ];
            for ((number, arg), name) in numbers[offset..]
                .iter_mut()
                .zip(&positional)
                .zip(&names[offset..])
            {
                if arg.parse::<f64>().is_err() {
                    *number = constant_expression(arg, tet).map_err(|reason| {
                        format!(
                            "{name} must be a finite number or a constant expression without \
                             variables, got '{arg}': {reason}"
                        )
                    })?;
                    continue;
                }
                *number = arg
                    .parse::<f64>()
                    .ok()
                    .filter(|value| value.is_finite())
                    .ok_or_else(|| format!("{name} must be a finite number, got '{arg}'"))?;
            }
        }
        let points = match positional.get(coordinate_count) {
            Some(arg) => arg
                .parse::<usize>()
                .ok()
                .filter(|&count| count >= 2)
                .ok_or_else(|| "number_of_points must be an integer of at least 2".to_owned())?,
            None => 512,
        };
        let function = match expression {
            Some(source) => {
                Expression::parse(&source).map_err(|err| format!("invalid expression: {err}"))?
            }
            None => Expression::tetration(Complex64::new(numbers[0], numbers[1])),
        };
        Ok(Some(Self {
            function,
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

fn constant_value(
    source: &str,
    tet: &mut impl FnMut(Operation, Complex64, Complex64) -> Result<Complex64, String>,
) -> Result<Complex64, String> {
    let expression = Expression::parse(source)?;
    if expression.uses_variable() {
        return Err("it contains a variable".to_owned());
    }
    expression.evaluate(Complex64::new(0.0, 0.0), tet)
}

fn constant_expression(
    source: &str,
    tet: &mut impl FnMut(Operation, Complex64, Complex64) -> Result<Complex64, String>,
) -> Result<f64, String> {
    let value = constant_value(source, tet)?;
    if value.is_finite() && value.im.abs() <= 1e-12 * value.re.abs().max(1.0) {
        Ok(value.re)
    } else {
        Err(format!("its value {value} is not a finite real number"))
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
        Config::parse(args.iter().map(|arg| arg.to_string()), &mut |_, _, _| {
            Err("unexpected tet".to_owned())
        })
    }

    #[test]
    fn accepts_negative_arguments_and_defaults_to_512_inclusive_samples() {
        let config = parse(&["1.333", "-0.06", "-2", "0", "2", "0"])
            .unwrap()
            .unwrap();
        let mut operands = None;
        config
            .function
            .evaluate(config.height(0), &mut |_, base, height| {
                operands = Some((base, height));
                Ok::<_, ()>(height)
            })
            .unwrap();
        assert_eq!(operands, Some((Complex64::new(1.333, -0.06), config.from)));
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
                function: Expression::tetration(Complex64::new(2.0, 0.0)),
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

    #[test]
    fn accepts_expressions_with_default_or_explicit_sample_counts() {
        for args in [
            vec!["x^^x-x^x+x", "-2", "-1", "2", "1"],
            vec!["--expr=x^^x-x^x+x", "-2", "-1", "2", "1"],
        ] {
            let config = parse(&args).unwrap().unwrap();
            assert_eq!(config.function.source(), "x^^x-x^x+x");
            assert_eq!(config.points, 512);
            assert_eq!(config.height(0), Complex64::new(-2.0, -1.0));
            assert_eq!(config.height(511), Complex64::new(2.0, 1.0));
        }
        let config = parse(&[
            "sin(x)+3i",
            "-2",
            "-1",
            "--screenshot=expression.png",
            "2",
            "1",
            "3",
        ])
        .unwrap()
        .unwrap();
        assert_eq!(config.points, 3);
        assert_eq!(config.height(1), Complex64::new(0.0, 0.0));
        assert_eq!(config.screenshot.as_deref(), Some("expression.png"));
        let value = config
            .function
            .evaluate(config.height(1), &mut |_, _, _| Err("unexpected tet"))
            .unwrap();
        assert_eq!(value, Complex64::new(0.0, 3.0));
    }

    #[test]
    fn disambiguates_numeric_constants_and_accepts_leading_minus_expressions() {
        for args in [
            vec!["2", "-1", "0", "1", "0"],
            vec!["--expr=2", "-1", "0", "1", "0", "9"],
        ] {
            let config = parse(&args).unwrap().unwrap();
            let value = config
                .function
                .evaluate(config.from, &mut |_, _, _| Err("unexpected tet"))
                .unwrap();
            assert_eq!(value, Complex64::new(2.0, 0.0));
            assert_eq!(config.points, if args.len() == 5 { 512 } else { 9 });
        }
        let legacy = parse(&["2", "0", "-1", "0", "1", "0"]).unwrap().unwrap();
        assert!(legacy.function.uses_tet());
        assert_eq!(legacy.points, 512);
        for (source, expected) in [("-x^2", -4.0), ("--x", 2.0)] {
            let config = parse(&["--", source, "-2", "0", "2", "0", "3"])
                .unwrap()
                .unwrap();
            let value = config
                .function
                .evaluate(config.to, &mut |_, _, _| Err("unexpected tet"))
                .unwrap();
            assert!((value - Complex64::new(expected, 0.0)).norm() < 1e-12);
        }
    }

    #[test]
    fn validates_expression_syntax_coordinates_counts_and_duplicate_options() {
        for source in ["", "x^^", "unknown(x)", "pow(x)", "sin(x,2)"] {
            assert!(parse(&[source, "0", "0", "1", "0"]).is_err());
        }
        for index in 1..=4 {
            let mut args = ["x", "0", "0", "1", "0"];
            args[index] = "NaN";
            assert!(parse(&args).is_err());
        }
        for count in ["0", "1", "-2", "2.5"] {
            assert!(parse(&["x", "0", "0", "1", "0", count]).is_err());
        }
        assert!(parse(&["--expr=x", "--expr=x", "0", "0", "1", "0"]).is_err());
        assert!(parse(&["--expr=", "0", "0", "1", "0"]).is_err());
        assert!(parse(&["x", "0", "0", "1"]).is_err());
    }

    #[test]
    fn coordinates_accept_constant_expressions_and_plain_numbers_parse_as_before() {
        let config = parse(&["x", "-2.5", "-pi", "pi*e^e-pow(2, 0.5)", "sqrt(2)+ETA", "3"])
            .unwrap()
            .unwrap();
        assert_eq!(config.from, Complex64::new(-2.5, -std::f64::consts::PI));
        assert!((config.to.re - 46.19430536603135).abs() < 1e-12);
        assert!((config.to.im - (std::f64::consts::SQRT_2 + 1.444667861009766)).abs() < 1e-12);
        assert_eq!(config.points, 3);
        let config = parse(&["--expr=x", "exp(i*pi)", "1e-3", "tau/3", "-2*e"])
            .unwrap()
            .unwrap();
        assert_eq!(config.from, Complex64::new(-1.0, 0.001));
        assert!((config.to.re - std::f64::consts::TAU / 3.0).abs() < 1e-12);
        assert_eq!(config.to.im, -2.0 * std::f64::consts::E);
        let config = parse(&["x", "1+1e-13i", "1e6+1e-7i", "1", "0"])
            .unwrap()
            .unwrap();
        assert_eq!(config.from, Complex64::new(1.0, 1e6));
        for bad in ["inf", "-Infinity", "nan", "1e999"] {
            assert_eq!(
                parse(&["x", "0", "0", bad, "0"]).unwrap_err(),
                format!("plot_to_re must be a finite number, got '{bad}'")
            );
        }
        assert_eq!(
            parse(&["x", "0", "0", "1", "0", "pi"]).unwrap_err(),
            "number_of_points must be an integer of at least 2"
        );
    }

    #[test]
    fn coordinates_reject_variables_nonreal_values_and_failed_evaluations() {
        for (coordinate, reason) in [
            ("x+1", "it contains a variable"),
            ("2sin(Z)", "it contains a variable"),
            ("sqrt(-4)", "its value 0+2i is not a finite real number"),
            ("1+1e-11i", "is not a finite real number"),
            ("2*inf", "is not a finite real number"),
            ("(pi", "expected `)` at byte 3"),
            ("towermax2", "unknown identifier `towermax2` at byte 0"),
            ("2^^3", "unexpected tet"),
        ] {
            let error = parse(&["x", "0", coordinate, "1", "0"]).unwrap_err();
            let start = format!(
                "plot_from_im must be a finite number or a constant expression without \
                 variables, got '{coordinate}': "
            );
            assert!(
                error.starts_with(&start) && error.ends_with(reason),
                "{error}"
            );
        }
    }

    #[test]
    fn two_complex_points_are_the_ends_of_the_line() {
        let ends = |args: &[&str]| {
            let config = parse(args).unwrap().unwrap();
            (config.from, config.to, config.points)
        };
        let line = (Complex64::new(-2.0, -1.0), Complex64::new(2.0, 1.0));
        assert_eq!(ends(&["x", "-2", "-1", "2", "1"]), (line.0, line.1, 512));
        for (args, points) in [
            (&["x", "-2-i", "2+i"][..], 512),
            (&["x", "-2-i", "2+i", "3"][..], 3),
            (&["--expr=x", "-2-i", "2+i"][..], 512),
            (&["--expr=x", "-i-2", "2+1i", "9"][..], 9),
            (&["x", "conj(i-2)", "abs(2)+sqrt(-4)/2"][..], 512),
        ] {
            assert_eq!(ends(args), (line.0, line.1, points), "{args:?}");
        }
        let real = |re| Complex64::new(re, 0.0);
        assert_eq!(ends(&["x", "1", "2.5"]), (real(1.0), real(2.5), 512));
        assert_eq!(
            ends(&["x", "-1e-3", "+.5", "7"]),
            (real(-0.001), real(0.5), 7)
        );
        let (from, to, _) = ends(&["x", "exp(i*pi/2)", "towermin - pi*i"]);
        assert!((from - Complex64::new(0.0, 1.0)).norm() < 1e-15);
        let pi = std::f64::consts::PI;
        assert_eq!(to, Complex64::new(0.06598803584531254, -pi));
        // A number in front of two points is the expression, not a legacy base.
        let config = parse(&["2", "0", "1"]).unwrap().unwrap();
        assert!(!config.function.uses_tet());
        assert_eq!((config.from, config.to), (real(0.0), real(1.0)));
    }

    #[test]
    fn complex_points_reject_variables_nonfinite_values_and_wrong_counts() {
        for (point, reason) in [
            ("x+1", "it contains a variable"),
            ("(pi", "expected `)` at byte 3"),
            ("2^^3", "unexpected tet"),
        ] {
            let error = parse(&["x", "0", point]).unwrap_err();
            let start = format!(
                "plot_to must be a finite complex number or a constant expression without \
                 variables, got '{point}': "
            );
            assert!(
                error.starts_with(&start) && error.ends_with(reason),
                "{error}"
            );
        }
        for (point, value) in [("inf", "inf+0i"), ("1e999", "inf+0i"), ("1/0", "NaN+NaNi")] {
            assert_eq!(
                parse(&["--expr=x", point, "1"]).unwrap_err(),
                format!("plot_from must be a finite complex number, got '{point}' = {value}")
            );
        }
        for args in [
            &["x", "0"][..],
            &["--expr=x", "0"][..],
            &["--expr=x"][..],
            &["x"][..],
        ] {
            let error = parse(args).unwrap_err();
            assert!(error.starts_with("expected an expression and two complex points"));
        }
        for count in ["1", "2.5", "x", "2+i"] {
            assert_eq!(
                parse(&["x", "0", "1", count]).unwrap_err(),
                "number_of_points must be an integer of at least 2"
            );
        }
    }

    #[test]
    fn coordinate_expressions_run_tet_and_keep_the_argument_forms() {
        let mut calls = Vec::new();
        let config = Config::parse(
            ["x", "1.5^^2", "0", "ate(2, 16) + 1", "0"].map(str::to_owned),
            &mut |operation, a, b| {
                calls.push((operation, a, b));
                Ok(Complex64::new(1.8371173070873836, 0.0))
            },
        )
        .unwrap()
        .unwrap();
        assert_eq!(config.from, Complex64::new(1.8371173070873836, 0.0));
        assert_eq!(config.to, Complex64::new(1.8371173070873836 + 1.0, 0.0));
        let expected = [
            (
                Operation::Tetration,
                Complex64::new(1.5, 0.0),
                Complex64::new(2.0, 0.0),
            ),
            (
                Operation::SuperLog,
                Complex64::new(2.0, 0.0),
                Complex64::new(16.0, 0.0),
            ),
        ];
        assert_eq!(calls, expected);
        calls.clear();
        let config = Config::parse(
            ["x", "1.5^^2 - i", "ate(2, 16)i + 1", "4"].map(str::to_owned),
            &mut |operation, a, b| {
                calls.push((operation, a, b));
                Ok(Complex64::new(1.8371173070873836, 0.0))
            },
        )
        .unwrap()
        .unwrap();
        assert_eq!(config.from, Complex64::new(1.8371173070873836, -1.0));
        assert_eq!(config.to, Complex64::new(1.0, 1.8371173070873836));
        assert_eq!((calls, config.points), (expected.to_vec(), 4));
        let base = |args: &[&str]| {
            let config = parse(args).unwrap().unwrap();
            let mut base = None;
            config
                .function
                .evaluate(config.from, &mut |_, a, _| {
                    base = Some(a);
                    Ok::<_, ()>(a)
                })
                .unwrap();
            (base, config.from, config.points)
        };
        let eta = 1.444667861009766;
        assert_eq!(
            base(&["2", "eta", "-pi", "0", "pi", "0"]),
            (
                Some(Complex64::new(2.0, eta)),
                Complex64::new(-std::f64::consts::PI, 0.0),
                512
            )
        );
        assert_eq!(
            base(&["eta", "0", "-2", "0", "2", "0", "5"]),
            (Some(Complex64::new(eta, 0.0)), Complex64::new(-2.0, 0.0), 5)
        );
        assert_eq!(
            base(&["eta", "0", "-2", "0", "2", "5"]),
            (None, Complex64::new(0.0, -2.0), 5)
        );
    }
}
