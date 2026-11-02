use crate::cli::interpolate;
use macroquad::prelude::{vec3, Vec3};
use num_complex::Complex64;

#[derive(Clone, Copy, Debug)]
pub struct Range {
    pub min: f64,
    pub max: f64,
}

impl Range {
    pub fn normalize(self, value: f64) -> f32 {
        if self.min == self.max {
            return 0.0;
        }
        let span = self.max - self.min;
        let fraction = if span.is_finite() {
            (value - self.min) / span
        } else {
            (value / 2.0 - self.min / 2.0) / (self.max / 2.0 - self.min / 2.0)
        };
        (2.0 * fraction - 1.0).clamp(-1.0, 1.0) as f32
    }

    pub fn at(self, t: f64) -> f64 {
        interpolate(self.min, self.max, t)
    }
}

pub struct Plot {
    pub points: Vec<Option<Vec3>>,
    pub real: Range,
    pub imag: Range,
    pub finite_count: usize,
}

impl Plot {
    pub fn new(values: &[Option<Complex64>]) -> Result<Self, String> {
        let is_finite = |value: &&Complex64| value.re.is_finite() && value.im.is_finite();
        let mut finite = values.iter().flatten().filter(is_finite);
        let first = finite
            .next()
            .ok_or_else(|| "tet returned no finite values for this line".to_owned())?;
        let mut real = Range {
            min: first.re,
            max: first.re,
        };
        let mut imag = Range {
            min: first.im,
            max: first.im,
        };
        let mut finite_count = 1;
        for value in finite {
            real.min = real.min.min(value.re);
            real.max = real.max.max(value.re);
            imag.min = imag.min.min(value.im);
            imag.max = imag.max.max(value.im);
            finite_count += 1;
        }
        let points = values
            .iter()
            .enumerate()
            .map(|(index, value)| {
                value
                    .filter(|value| value.re.is_finite() && value.im.is_finite())
                    .map(|value| {
                        vec3(
                            (2.0 * index as f64 / (values.len() - 1).max(1) as f64 - 1.0) as f32,
                            real.normalize(value.re),
                            imag.normalize(value.im),
                        )
                    })
            })
            .collect();
        Ok(Self {
            points,
            real,
            imag,
            finite_count,
        })
    }

    pub fn segments(&self) -> impl Iterator<Item = (usize, Vec3, Vec3)> + '_ {
        self.points
            .windows(2)
            .enumerate()
            .filter_map(|(index, pair)| Some((index, pair[0]?, pair[1]?)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_gaps_original_x_positions_and_independent_bounds() {
        let plot = Plot::new(&[
            Some(Complex64::new(-2.0, 10.0)),
            None,
            Some(Complex64::new(0.0, 20.0)),
            Some(Complex64::new(2.0, 30.0)),
            None,
        ])
        .unwrap();
        assert_eq!(plot.finite_count, 3);
        assert_eq!(plot.points[0], Some(vec3(-1.0, -1.0, -1.0)));
        assert_eq!(plot.points[2], Some(vec3(0.0, 0.0, 0.0)));
        assert_eq!(plot.points[3], Some(vec3(0.5, 1.0, 1.0)));
        let segments: Vec<_> = plot.segments().collect();
        assert_eq!(segments, [(2, vec3(0.0, 0.0, 0.0), vec3(0.5, 1.0, 1.0))]);
    }

    #[test]
    fn handles_constant_huge_and_subnormal_values_without_invalid_vertices() {
        assert_eq!(
            Range {
                min: 1.0e16,
                max: 1.0e16 + 4.0,
            }
            .normalize(1.0e16 + 2.0),
            0.0
        );
        for (low, high) in [
            (-f64::MAX, f64::MAX),
            (f64::MAX / 2.0, f64::MAX),
            (f64::from_bits(1), f64::from_bits(2)),
            (0.0, 0.0),
            (f64::MAX, f64::MAX),
        ] {
            let plot = Plot::new(&[
                Some(Complex64::new(low, 0.0)),
                Some(Complex64::new(high, 0.0)),
            ])
            .unwrap();
            for point in plot.points.iter().flatten() {
                assert!(point.is_finite());
                assert_eq!(point.z, 0.0);
            }
            assert_eq!(plot.real.at(0.0), low);
            assert_eq!(plot.real.at(1.0), high);
            assert!(plot.real.at(0.5).is_finite());
            if low == high {
                assert_eq!(plot.points[0].unwrap().y, 0.0);
            } else {
                assert_eq!(plot.points[0].unwrap().y, -1.0);
                assert_eq!(plot.points[1].unwrap().y, 1.0);
            }
        }
    }

    #[test]
    fn rejects_empty_plots_but_keeps_an_isolated_finite_point() {
        assert!(Plot::new(&[]).is_err());
        assert!(Plot::new(&[None, Some(Complex64::new(f64::NAN, 0.0))]).is_err());
        let plot = Plot::new(&[None, Some(Complex64::new(1.0, 2.0)), None]).unwrap();
        assert_eq!(plot.finite_count, 1);
        assert_eq!(plot.points[1], Some(Vec3::ZERO));
        assert_eq!(plot.segments().count(), 0);
    }
}
