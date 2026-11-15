use crate::cli::{Animation, Config};
use crate::plot::{Plot, Range};
use macroquad::camera::Camera;
use macroquad::prelude::*;
use num_complex::Complex64;
use std::time::{SystemTime, UNIX_EPOCH};

const BACKGROUND: Color = Color::new(0.035, 0.045, 0.07, 1.0);
const X_COLOR: Color = Color::new(1.0, 0.65, 0.35, 1.0);
const Y_COLOR: Color = Color::new(0.45, 0.95, 0.6, 1.0);
const Z_COLOR: Color = Color::new(0.55, 0.7, 1.0, 1.0);

pub fn window_config() -> macroquad::conf::Conf {
    macroquad::conf::Conf {
        miniquad_conf: macroquad::miniquad::conf::Conf {
            window_title: "tet-line - complex functions".to_owned(),
            window_width: 1280,
            window_height: 900,
            high_dpi: true,
            sample_count: 4,
            ..Default::default()
        },
        ..Default::default()
    }
}

#[derive(Clone)]
struct View {
    yaw: f32,
    pitch: f32,
    roll: f32,
    distance: f32,
    target: Vec3,
}

impl Default for View {
    fn default() -> Self {
        Self {
            yaw: 0.65,
            pitch: 0.4,
            roll: 0.0,
            distance: 6.2,
            target: Vec3::ZERO,
        }
    }
}

impl View {
    fn orbit(&mut self, yaw: f32, pitch: f32, roll: f32) {
        self.yaw = (self.yaw + yaw).rem_euclid(std::f32::consts::TAU);
        self.pitch = (self.pitch + pitch).clamp(-1.52, 1.52);
        self.roll = (self.roll + roll).rem_euclid(std::f32::consts::TAU);
    }

    fn zoom(&mut self, factor: f32) {
        self.distance = (self.distance * factor).clamp(0.5, 60.0);
    }

    fn camera(&self, aspect: f32) -> Camera3D {
        let offset = self.distance
            * vec3(
                self.pitch.cos() * self.yaw.sin(),
                self.pitch.sin(),
                self.pitch.cos() * self.yaw.cos(),
            );
        let forward = -offset.normalize();
        let right = forward.cross(Vec3::Y).normalize();
        let up = Quat::from_axis_angle(forward, self.roll) * right.cross(forward).normalize();
        Camera3D {
            position: self.target + offset,
            target: self.target,
            up,
            aspect: Some(aspect),
            fovy: 45.0_f32.to_radians(),
            z_near: 0.01,
            z_far: 200.0,
            ..Default::default()
        }
    }

    fn pan(&mut self, delta: Vec2, viewport_height: f32, aspect: f32) {
        let camera = self.camera(aspect);
        let forward = (camera.target - camera.position).normalize();
        let right = forward.cross(camera.up).normalize();
        let units_per_pixel =
            2.0 * self.distance * (camera.fovy / 2.0).tan() / viewport_height.max(1.0);
        self.target += (-right * delta.x + camera.up * delta.y) * units_per_pixel;
    }
}

/// The frame that is shown after `clock` seconds of play: every frame lasts as long as the
/// others, and after the last one the animation starts again.
fn frame_at(clock: f64, animation: &Animation) -> usize {
    let position = (clock / animation.seconds).rem_euclid(1.0);
    ((position * animation.frames as f64) as usize).min(animation.frames - 1)
}

fn status_lines(config: &Config, plot: &Plot, frame: usize) -> [String; 4] {
    let (from, to) = config.line(frame);
    let (place, fitted) = match &config.animation {
        Some(animation) => (
            format!(
                "frame {} / {} in {} s   |   ",
                frame + 1,
                animation.frames,
                number(animation.seconds)
            ),
            ", over all frames",
        ),
        None => (String::new(), ""),
    };
    [
        format!("tet-line   f(x) = {}", config.function.source()),
        format!(
            "x: {} -> {}   |   y: Re(f(x))   |   z: Im(f(x))",
            complex(from),
            complex(to)
        ),
        format!(
            "{place}{} / {} finite samples, {} skipped   |   calculated once{}",
            plot.finite_count,
            config.points,
            config.points - plot.finite_count,
            if config.function.uses_tet() {
                " (tet: 13 digits)"
            } else {
                ""
            }
        ),
        format!(
            "Re: [{}, {}]   Im: [{}, {}]   |   axes fitted independently{fitted}",
            number(plot.real.min),
            number(plot.real.max),
            number(plot.imag.min),
            number(plot.imag.max)
        ),
    ]
}

pub async fn run(config: Config, plots: Vec<Plot>) {
    let mut frame = 0;
    let mut labels = axis_labels(&config, &plots[frame], frame);
    let mut status = status_lines(&config, &plots[frame], frame);
    // The animation plays from the start; Space pauses and resumes it.
    let mut playing = config.animation.is_some();
    let mut clock = 0.0_f64;
    let mut view = View::default();
    let mut previous_mouse = Vec2::from(mouse_position());
    let mut auto_rotate = false;
    let mut help = true;
    let mut fullscreen = false;
    let mut rendered = 0;

    loop {
        if is_key_pressed(KeyCode::Escape) || is_quit_requested() {
            break;
        }
        if let Some(animation) = &config.animation {
            if is_key_pressed(KeyCode::Space) {
                playing = !playing;
            }
            if playing {
                clock = (clock + f64::from(get_frame_time())).rem_euclid(animation.seconds);
            }
            let shown = frame_at(clock, animation);
            if shown != frame {
                frame = shown;
                labels = axis_labels(&config, &plots[frame], frame);
                status = status_lines(&config, &plots[frame], frame);
            }
        }
        let plot = &plots[frame];
        let dt = get_frame_time().min(0.1);
        let aspect = screen_width().max(1.0) / screen_height().max(1.0);
        let mouse = Vec2::from(mouse_position());
        let delta = mouse - previous_mouse;
        previous_mouse = mouse;
        if is_key_pressed(KeyCode::Key0) || is_key_pressed(KeyCode::Home) {
            view = View::default();
            auto_rotate = false;
        }
        if is_key_pressed(KeyCode::R) {
            auto_rotate = !auto_rotate;
        }
        if is_key_pressed(KeyCode::F1) {
            help = !help;
        }
        if is_key_pressed(KeyCode::F11) {
            fullscreen = !fullscreen;
            set_fullscreen(fullscreen);
        }
        if is_mouse_button_down(MouseButton::Left) {
            view.orbit(-delta.x * 0.007, delta.y * 0.007, 0.0);
            auto_rotate = false;
        }
        if is_mouse_button_down(MouseButton::Right) || is_mouse_button_down(MouseButton::Middle) {
            view.pan(delta, screen_height(), aspect);
        }
        view.orbit(
            key_axis(KeyCode::A, KeyCode::D) * dt,
            key_axis(KeyCode::S, KeyCode::W) * dt,
            key_axis(KeyCode::Q, KeyCode::E) * dt,
        );
        if auto_rotate {
            view.orbit(dt * 0.25, 0.0, 0.0);
        }
        view.zoom((-mouse_wheel().1 * 0.12 + key_axis(KeyCode::Z, KeyCode::X) * dt).exp());

        clear_background(BACKGROUND);
        let camera = view.camera(aspect);
        set_camera(&camera);
        draw_axes();
        for (index, start, end) in plot.segments() {
            let t = (index as f32 + 0.5) / (config.points - 1) as f32;
            draw_line_3d(start, end, curve_color(t));
        }
        for (index, point) in plot.points.iter().enumerate() {
            if let Some(point) = point {
                let t = index as f32 / (config.points - 1) as f32;
                let size = if index == 0 || index + 1 == config.points {
                    0.027
                } else {
                    0.012
                };
                draw_cube(*point, Vec3::splat(size), None, curve_color(t));
            }
        }

        set_default_camera();
        for label in &labels {
            draw_label(&camera, label);
        }
        let play = config.animation.as_ref().map(|_| playing);
        draw_hud(&status, help, auto_rotate, play);
        rendered += 1;
        if rendered >= 3 {
            if let Some(path) = &config.screenshot {
                save_screenshot(path);
                break;
            }
        }
        if is_key_pressed(KeyCode::P) {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            save_screenshot(&format!("tet-line-{timestamp}.png"));
        }
        next_frame().await;
    }
}

fn save_screenshot(path: &str) {
    get_screen_data().export_png(path);
    println!("Saved screenshot to {path}");
}

fn key_axis(negative: KeyCode, positive: KeyCode) -> f32 {
    i32::from(is_key_down(positive)) as f32 - i32::from(is_key_down(negative)) as f32
}

fn curve_color(t: f32) -> Color {
    Color::new(0.15 + 0.85 * t, 0.9 - 0.5 * t, 1.0 - 0.15 * t, 1.0)
}

fn draw_axes() {
    let edge = Color::new(0.25, 0.3, 0.4, 1.0);
    let grid = Color::new(0.12, 0.16, 0.23, 1.0);
    for a in [-1.0, 1.0] {
        for b in [-1.0, 1.0] {
            draw_line_3d(vec3(-1.0, a, b), vec3(1.0, a, b), edge);
            draw_line_3d(vec3(a, -1.0, b), vec3(a, 1.0, b), edge);
            draw_line_3d(vec3(a, b, -1.0), vec3(a, b, 1.0), edge);
        }
    }
    for tick in [-0.5, 0.0, 0.5] {
        draw_line_3d(vec3(tick, -1.0, -1.0), vec3(tick, -1.0, 1.0), grid);
        draw_line_3d(vec3(-1.0, -1.0, tick), vec3(1.0, -1.0, tick), grid);
        draw_line_3d(vec3(-1.0, tick, -1.0), vec3(1.0, tick, -1.0), grid);
        draw_line_3d(vec3(tick, -1.0, -1.0), vec3(tick, 1.0, -1.0), grid);
    }
    draw_line_3d(vec3(-1.0, -1.0, 1.0), vec3(1.12, -1.0, 1.0), X_COLOR);
    draw_line_3d(vec3(-1.0, -1.0, 1.0), vec3(-1.0, 1.12, 1.0), Y_COLOR);
    draw_line_3d(vec3(-1.0, -1.0, -1.0), vec3(-1.0, -1.0, 1.12), Z_COLOR);
}

struct Label {
    position: Vec3,
    text: String,
    color: Color,
}

fn axis_labels(config: &Config, plot: &Plot, frame: usize) -> Vec<Label> {
    let mut labels = Vec::new();
    for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
        labels.push(Label {
            position: vec3(2.0 * t as f32 - 1.0, -1.1, 1.1),
            text: complex(config.frame_height_at(frame, t)),
            color: X_COLOR,
        });
    }
    for (range, real) in [(plot.real, true), (plot.imag, false)] {
        add_value_labels(&mut labels, range, real);
    }
    labels.extend([
        Label {
            position: vec3(1.25, -1.0, 1.1),
            text: "x: argument".to_owned(),
            color: X_COLOR,
        },
        Label {
            position: vec3(-1.0, 1.25, 1.0),
            text: "y: Re(f(x))".to_owned(),
            color: Y_COLOR,
        },
        Label {
            position: vec3(-1.45, -1.3, 1.5),
            text: "z: Im(f(x))".to_owned(),
            color: Z_COLOR,
        },
    ]);
    labels
}

fn add_value_labels(labels: &mut Vec<Label>, range: Range, real: bool) {
    for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
        if range.min == range.max && t != 0.5 {
            continue;
        }
        let coordinate = 2.0 * t as f32 - 1.0;
        labels.push(Label {
            position: if real {
                vec3(-1.13, coordinate, 1.1)
            } else {
                vec3(-1.4, -1.24, coordinate)
            },
            text: number(range.at(t)),
            color: if real { Y_COLOR } else { Z_COLOR },
        });
    }
}

fn draw_label(camera: &Camera3D, label: &Label) {
    let clip = camera.matrix() * label.position.extend(1.0);
    if clip.w <= 0.0 {
        return;
    }
    let ndc = clip.truncate() / clip.w;
    if !(-1.0..=1.0).contains(&ndc.z) {
        return;
    }
    let x = (ndc.x + 1.0) * 0.5 * screen_width();
    let y = (1.0 - ndc.y) * 0.5 * screen_height();
    let dimensions = measure_text(&label.text, None, 17, 1.0);
    draw_rectangle(
        x - dimensions.width / 2.0 - 3.0,
        y - dimensions.height - 2.0,
        dimensions.width + 6.0,
        dimensions.height + 6.0,
        Color::new(BACKGROUND.r, BACKGROUND.g, BACKGROUND.b, 0.8),
    );
    draw_text(
        &label.text,
        x - dimensions.width / 2.0,
        y,
        17.0,
        label.color,
    );
}

/// `playing` is the state of an animation, `None` without one.
fn draw_hud(status: &[String], help: bool, auto_rotate: bool, playing: Option<bool>) {
    draw_rectangle(
        0.0,
        0.0,
        screen_width(),
        114.0,
        Color::new(0.02, 0.03, 0.05, 0.92),
    );
    for (index, line) in status.iter().enumerate() {
        draw_text(line, 18.0, 28.0 + index as f32 * 24.0, 20.0, WHITE);
    }
    let hint = format!(
        "{}{}",
        match playing {
            Some(true) => "Space: pause the animation | ",
            Some(false) => "Space: play the animation | ",
            None => "",
        },
        if auto_rotate {
            "R: stop rotation | F1: help"
        } else {
            "F1: help | cyan: start, pink: end"
        }
    );
    let hint = hint.as_str();
    if help {
        let top = screen_height() - 98.0;
        draw_rectangle(
            0.0,
            top,
            screen_width(),
            98.0,
            Color::new(0.02, 0.03, 0.05, 0.92),
        );
        for (index, text) in [
            "Left drag: orbit | Right/middle drag: pan | Wheel or Z/X: zoom",
            "A/D: yaw | W/S: pitch | Q/E: roll | R: auto-rotate | 0/Home: reset",
            "P: save PNG | F1: help | F11: fullscreen | Esc: quit | Values never recalculated",
            hint,
        ]
        .iter()
        .enumerate()
        {
            draw_text(
                text,
                18.0,
                top + 21.0 + index as f32 * 22.0,
                18.0,
                LIGHTGRAY,
            );
        }
    } else {
        draw_text(hint, 18.0, screen_height() - 18.0, 18.0, LIGHTGRAY);
    }
}

fn number(value: f64) -> String {
    if value == 0.0 {
        "0".to_owned()
    } else if !(0.001..10000.0).contains(&value.abs()) {
        format!("{value:.3e}")
    } else {
        format!("{value:.4}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_owned()
    }
}

fn complex(value: Complex64) -> String {
    if value.im == 0.0 {
        number(value.re)
    } else if value.re == 0.0 {
        format!("{}i", number(value.im))
    } else {
        format!(
            "{}{}{}i",
            number(value.re),
            if value.im < 0.0 { "-" } else { "+" },
            number(value.im.abs())
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_last_equally_long_and_the_animation_starts_again() {
        let animation = Animation {
            from: Complex64::new(2.0, 1.0),
            to: Complex64::new(6.0, 1.0),
            frames: 100,
            seconds: 5.0,
        };
        for (clock, frame) in [
            (0.0, 0),
            (0.049, 0),
            (0.051, 1),
            (2.5, 50),
            (4.99, 99),
            (5.0, 0),
            (7.5, 50),
            (-0.01, 99),
        ] {
            assert_eq!(frame_at(clock, &animation), frame, "{clock}");
        }
        let seen: std::collections::BTreeSet<_> = (0..1000)
            .map(|step| frame_at(f64::from(step) * 0.005, &animation))
            .collect();
        assert_eq!(seen.len(), 100);
        for clock in [f64::MAX, -f64::MAX, 1e300, f64::MIN_POSITIVE] {
            assert!(frame_at(clock, &animation) < 100);
        }
    }

    #[test]
    fn status_and_ticks_follow_the_frame() {
        let mut config = Config {
            function: crate::expression::Expression::parse("x").unwrap(),
            from: Complex64::new(-2.0, 1.0),
            to: Complex64::new(6.0, -1.0),
            points: 3,
            screenshot: None,
            animation: None,
        };
        let value = |re, im| Some(Complex64::new(re, im));
        let values = [
            value(-2.0, 1.0),
            value(2.0, 0.0),
            value(6.0, -1.0),
            value(2.0, 1.0),
            None,
            value(6.0, 1.0),
        ];
        let single = Plot::frames(&values[..3], 3).unwrap();
        let status = status_lines(&config, &single[0], 0);
        assert_eq!(
            status[1],
            "x: -2+1i -> 6-1i   |   y: Re(f(x))   |   z: Im(f(x))"
        );
        assert_eq!(
            status[2],
            "3 / 3 finite samples, 0 skipped   |   calculated once"
        );
        assert!(status[3].ends_with("axes fitted independently"));

        config.animation = Some(Animation {
            from: Complex64::new(2.0, 1.0),
            to: Complex64::new(6.0, 1.0),
            frames: 2,
            seconds: 2.5,
        });
        let frames = Plot::frames(&values, 3).unwrap();
        let status = status_lines(&config, &frames[1], 1);
        assert_eq!(
            status[1],
            "x: 2+1i -> 6+1i   |   y: Re(f(x))   |   z: Im(f(x))"
        );
        assert_eq!(
            status[2],
            "frame 2 / 2 in 2.5 s   |   2 / 3 finite samples, 1 skipped   |   calculated once"
        );
        assert!(status[3].ends_with("axes fitted independently, over all frames"));
        let ticks = |frame: usize| -> Vec<String> {
            axis_labels(&config, &frames[frame], frame)
                .into_iter()
                .take(5)
                .map(|label| label.text)
                .collect()
        };
        assert_eq!(ticks(0), ["-2+1i", "0.5i", "2", "4-0.5i", "6-1i"]);
        assert_eq!(ticks(1), ["2+1i", "3+1i", "4+1i", "5+1i", "6+1i"]);
    }

    #[test]
    fn camera_stays_finite_and_orthogonal_when_rotated_zoomed_and_panned() {
        let mut view = View::default();
        for (yaw, pitch, roll, zoom) in [
            (100.0, 100.0, 0.5, 0.001),
            (-300.0, -300.0, 10.0, 1e6),
            (0.4, 0.3, -0.7, 0.8),
        ] {
            view.orbit(yaw, pitch, roll);
            view.zoom(zoom);
            view.pan(vec2(50.0, -30.0), 900.0, 1.5);
            let camera = view.camera(1.5);
            let forward = (camera.target - camera.position).normalize();
            assert!(camera.position.is_finite());
            assert!(camera.up.is_finite());
            assert!((camera.up.length() - 1.0).abs() < 1e-5);
            assert!(forward.dot(camera.up).abs() < 1e-4);
        }
    }
}
