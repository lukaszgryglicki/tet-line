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
    /// The plot of one line.
    #[cfg(test)]
    pub fn new(values: &[Option<Complex64>]) -> Result<Self, String> {
        let mut frames = Self::frames(values, values.len())?;
        frames
            .pop()
            .ok_or_else(|| "function returned no finite values for this line".to_owned())
    }

    /// One plot for every `points` values, all in the same box: the two result axes are fitted
    /// to the finite values of all the frames, so that the curve moves and the box does not.
    pub fn frames(values: &[Option<Complex64>], points: usize) -> Result<Vec<Self>, String> {
        let is_finite = |value: &&Complex64| value.re.is_finite() && value.im.is_finite();
        let mut finite = values.iter().flatten().filter(is_finite);
        let first = finite
            .next()
            .ok_or_else(|| "function returned no finite values for this line".to_owned())?;
        let mut real = Range {
            min: first.re,
            max: first.re,
        };
        let mut imag = Range {
            min: first.im,
            max: first.im,
        };
        for value in finite {
            real.min = real.min.min(value.re);
            real.max = real.max.max(value.re);
            imag.min = imag.min.min(value.im);
            imag.max = imag.max.max(value.im);
        }
        Ok(values
            .chunks(points.max(1))
            .map(|frame| {
                let points: Vec<_> = frame
                    .iter()
                    .enumerate()
                    .map(|(index, value)| {
                        value
                            .filter(|value| value.re.is_finite() && value.im.is_finite())
                            .map(|value| {
                                vec3(
                                    (2.0 * index as f64 / (frame.len() - 1).max(1) as f64 - 1.0)
                                        as f32,
                                    real.normalize(value.re),
                                    imag.normalize(value.im),
                                )
                            })
                    })
                    .collect();
                Self {
                    finite_count: points.iter().flatten().count(),
                    points,
                    real,
                    imag,
                }
            })
            .collect())
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
    fn frames_share_one_box_and_keep_their_own_gaps() {
        let value = |re, im| Some(Complex64::new(re, im));
        let frames = Plot::frames(
            &[
                value(0.0, 0.0),
                value(1.0, 10.0),
                None,
                None,
                value(4.0, -10.0),
                value(2.0, 0.0),
            ],
            2,
        )
        .unwrap();
        assert_eq!(frames.len(), 3);
        for frame in &frames {
            assert_eq!((frame.real.min, frame.real.max), (0.0, 4.0));
            assert_eq!((frame.imag.min, frame.imag.max), (-10.0, 10.0));
        }
        let counts: Vec<_> = frames.iter().map(|frame| frame.finite_count).collect();
        assert_eq!(counts, [2, 0, 2]);
        assert_eq!(frames[0].points[0], Some(vec3(-1.0, -1.0, 0.0)));
        assert_eq!(frames[0].points[1], Some(vec3(1.0, -0.5, 1.0)));
        assert_eq!(frames[1].points, [None, None]);
        assert_eq!(frames[2].points[0], Some(vec3(-1.0, 1.0, -1.0)));
        assert_eq!(frames[2].points[1], Some(vec3(1.0, 0.0, 0.0)));
        assert_eq!(frames[1].segments().count(), 0);
        assert!(Plot::frames(&[None, None, None, None], 2).is_err());
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
