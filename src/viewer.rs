use crate::cli::{Angles, Animation, Config};
use crate::plot::{Plot, Range};
use crate::video::{Recorder, FFMPEG};
use macroquad::camera::Camera;
use macroquad::prelude::*;
use num_complex::Complex64;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const BACKGROUND: Color = Color::new(0.035, 0.045, 0.07, 1.0);
const X_COLOR: Color = Color::new(1.0, 0.65, 0.35, 1.0);
const Y_COLOR: Color = Color::new(0.45, 0.95, 0.6, 1.0);
const Z_COLOR: Color = Color::new(0.55, 0.7, 1.0, 1.0);

const SAMPLES: i32 = 4;

/// The sizes of the text: of the labels, of the status lines and of the lines of keys.
const LABEL_TEXT: u16 = 17;
const STATUS_TEXT: u16 = 20;
const KEYS_TEXT: u16 = 18;

/// The automatic rotation of `R`: radians of yaw in a second.
const ROTATION: f32 = 0.25;

/// The automatic rotation of a window in radians a second: as fast as `--auto-rotation` says,
/// otherwise as fast as it always was.
fn rotation(config: &Config) -> f32 {
    config
        .auto_rotation
        .flatten()
        .map_or(ROTATION, f32::to_radians)
}

/// What `--auto-rotation` turns the view by between two frames of a video that is made without
/// a window: the degrees a second that it gives or, without them, a full turn over the video.
fn turn(config: &Config, animation: &Animation) -> f32 {
    match config.auto_rotation {
        None => 0.0,
        Some(None) => std::f32::consts::TAU / animation.frames as f32,
        Some(Some(degrees)) => degrees.to_radians() * Pace::new(animation).step,
    }
}

/// The window of the viewer. A `hidden` one is never shown and has the size of the screen: what
/// is drawn for it goes to a video.
pub fn window_config(hidden: bool) -> macroquad::conf::Conf {
    macroquad::conf::Conf {
        miniquad_conf: macroquad::miniquad::conf::Conf {
            window_title: "tet-line - complex functions".to_owned(),
            window_width: 1280,
            window_height: 900,
            high_dpi: true,
            sample_count: SAMPLES,
            fullscreen: hidden,
            platform: macroquad::miniquad::conf::Platform {
                linux_x11_hidden: hidden,
                // Nothing that is hidden waits for the monitor.
                swap_interval: hidden.then_some(0),
                ..Default::default()
            },
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
    /// The first view: the default one with the angles that were given, each in the range
    /// that the keys keep it in.
    fn initial(angles: &Angles) -> Self {
        let mut view = Self::default();
        for (angle, degrees) in [
            (&mut view.yaw, angles.yaw),
            (&mut view.pitch, angles.pitch),
            (&mut view.roll, angles.roll),
        ] {
            if let Some(degrees) = degrees {
                *angle = degrees.to_radians();
            }
        }
        view.orbit(0.0, 0.0, 0.0);
        view
    }

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

/// Makes macroquad know every character that the viewer writes, before a video starts.
/// It learns a character when it first draws it, and now and then it moves the ones that
/// it knows as it does: the text that it drew before in that picture comes out as dots,
/// and the text after it looks a little different. A window shows that for one frame; a
/// video would keep it.
fn learn_text(config: &Config) {
    let characters: String = (' '..='~')
        .chain(config.function.source().chars())
        .collect();
    for size in [LABEL_TEXT, STATUS_TEXT, KEYS_TEXT] {
        measure_text(&characters, None, size, 1.0);
    }
}

/// The clock in the middle of the frame after the one that is shown at `clock`. A recording
/// goes from picture to picture like that, so its video has every frame of the animation once.
fn following(clock: f64, animation: &Animation) -> f64 {
    let next = (frame_at(clock, animation) + 1) % animation.frames;
    (next as f64 + 0.5) / animation.frames as f64 * animation.seconds
}

/// The pace of a recording: a picture is due when the time of a frame of the video has passed,
/// and not more than one for every frame that the window draws.
struct Pace {
    /// The time of a frame of the video in seconds.
    step: f32,
    waited: f32,
}

impl Pace {
    /// The first picture is due at once.
    fn new(animation: &Animation) -> Self {
        let step = (animation.seconds / animation.frames as f64) as f32;
        Self { step, waited: step }
    }

    /// Whether the frame that the window draws `elapsed` seconds after its last one is a
    /// picture of the video.
    fn due(&mut self, elapsed: f32) -> bool {
        self.waited += elapsed;
        if self.waited < self.step {
            return false;
        }
        // A window that is slower than the video does not owe the pictures that it missed.
        self.waited = (self.waited - self.step).min(self.step);
        true
    }
}

/// An MP4 of the window that is being recorded: `M` starts it and `N` saves it.
struct Recording {
    recorder: Recorder,
    name: String,
    pace: Pace,
}

impl Recording {
    /// Starts a video of the window as large as it is now. A failure is reported and leaves
    /// no recording.
    fn start(animation: &Animation, lossy: bool) -> Option<Self> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let name = format!("tet-line-{timestamp}.mp4");
        let (width, height) = miniquad::window::screen_size();
        let (width, height) = (width as u32, height as u32);
        match Recorder::start(
            Path::new(FFMPEG),
            Path::new(&name),
            width,
            height,
            &animation.rate(),
            lossy,
        ) {
            Ok(recorder) => {
                println!(
                    "Recording {name}: {width} x {height} pixels, {} frames a second; N saves it",
                    number(animation.frames as f64 / animation.seconds)
                );
                // Closing the window saves the video as well.
                prevent_quit();
                Some(Self {
                    recorder,
                    name,
                    pace: Pace::new(animation),
                })
            }
            Err(err) => {
                eprintln!("error: {err}");
                None
            }
        }
    }

    /// Adds the picture of the window. A window that has another size now ends the video,
    /// which is saved, and a failure ends it without a file: both give `None`.
    fn picture(mut self, image: &Image) -> Option<Self> {
        if (u32::from(image.width), u32::from(image.height)) != self.recorder.size() {
            println!("The window has another size: the recording ends.");
            self.save();
            return None;
        }
        match self.recorder.frame(&image.bytes) {
            Ok(()) => Some(self),
            Err(err) => {
                eprintln!("error: {} is not saved: {err}", self.name);
                None
            }
        }
    }

    fn save(self) {
        let seconds = self.recorder.frames() as f32 * self.pace.step;
        match self.recorder.finish() {
            Ok(frames) => println!(
                "Saved video to {}: {frames} frames, {} s",
                self.name,
                number(f64::from(seconds))
            ),
            Err(err) => eprintln!("error: {} is not saved: {err}", self.name),
        }
    }

    /// Marks the window as recorded with a red dot. It is drawn after the picture is taken, so
    /// the video does not have it; text cannot be: after `get_screen_data` macroquad loses the
    /// glyphs that it draws for the first time.
    fn draw(&self) {
        draw_circle(screen_width() - 28.0, 26.0, 9.0, RED);
    }
}

pub async fn run(config: Config, plots: Vec<Plot>) -> Result<(), String> {
    if let Some(file) = &config.mp4 {
        return export(&config, &plots, file).await;
    }
    let mut frame = 0;
    let mut labels = axis_labels(&config, &plots[frame], frame);
    let mut status = status_lines(&config, &plots[frame], frame);
    // The animation plays from the start; Space pauses and resumes it.
    let mut playing = config.animation.is_some();
    let mut clock = 0.0_f64;
    let home = View::initial(&config.angles);
    let mut view = home.clone();
    let mut previous_mouse = Vec2::from(mouse_position());
    // A screenshot is the first view.
    let mut auto_rotate = config.auto_rotation.is_some() && config.screenshot.is_none();
    let rotation = rotation(&config);
    let mut help = true;
    let mut fullscreen = false;
    let mut rendered = 0;
    let mut recording: Option<Recording> = None;

    loop {
        if is_key_pressed(KeyCode::Escape) || is_quit_requested() {
            break;
        }
        // The time that this frame adds. A recording has the time of its video: each of its
        // pictures is one frame later, and what the window draws between two of them adds none.
        let mut dt = get_frame_time().min(0.1);
        let mut picture = false;
        if let Some(animation) = &config.animation {
            if is_key_pressed(KeyCode::Space) {
                playing = !playing;
            }
            if is_key_pressed(KeyCode::M) && recording.is_none() {
                learn_text(&config);
                recording = Recording::start(animation, config.lossy);
            }
            if is_key_pressed(KeyCode::N) {
                if let Some(recording) = recording.take() {
                    recording.save();
                }
            }
            if let Some(recording) = &mut recording {
                picture = recording.pace.due(get_frame_time());
                dt = if picture { recording.pace.step } else { 0.0 };
                if playing && picture {
                    clock = following(clock, animation);
                }
            } else if playing {
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
        let aspect = screen_width().max(1.0) / screen_height().max(1.0);
        let mouse = Vec2::from(mouse_position());
        let delta = mouse - previous_mouse;
        previous_mouse = mouse;
        if is_key_pressed(KeyCode::Key0) || is_key_pressed(KeyCode::Home) {
            view = home.clone();
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
            view.orbit(dt * rotation, 0.0, 0.0);
        }
        view.zoom((-mouse_wheel().1 * 0.12 + key_axis(KeyCode::Z, KeyCode::X) * dt).exp());

        draw_plot(&config, plot, &labels, &view, None);
        let keys = config
            .animation
            .as_ref()
            .map(|_| (playing, recording.is_some()));
        draw_hud(&status, help, &hint(auto_rotate, keys));
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
        if picture {
            let image = get_screen_data();
            recording = recording.and_then(|recording| recording.picture(&image));
        }
        if let Some(recording) = &recording {
            recording.draw();
        }
        next_frame().await;
    }
    if let Some(recording) = recording {
        recording.save();
    }
    Ok(())
}

/// `--mp4`: saves the animation as a video, every frame once from the first to the last, seen
/// from the first view, which `--auto-rotation` turns from frame to frame. No window is shown;
/// the pictures have the size of the hidden one, which is that of the screen.
async fn export(config: &Config, plots: &[Plot], file: &str) -> Result<(), String> {
    let animation = config.animation.as_ref().ok_or("--mp4 requires --anim")?;
    let (width, height) = miniquad::window::screen_size();
    let (width, height) = (width as u32, height as u32);
    let target = render_target_ex(
        width,
        height,
        RenderTargetParams {
            sample_count: SAMPLES,
            depth: true,
        },
    );
    let mut view = View::initial(&config.angles);
    learn_text(config);
    let mut recorder = Recorder::start(
        Path::new(FFMPEG),
        Path::new(file),
        width,
        height,
        &animation.rate(),
        config.lossy,
    )?;
    eprintln!(
        "Saving {} frames of {width} x {height} pixels, {} a second, to {file}...",
        animation.frames,
        number(animation.frames as f64 / animation.seconds)
    );
    let progress_step = (animation.frames / 20).max(1);
    for (frame, plot) in plots.iter().enumerate() {
        draw_plot(
            config,
            plot,
            &axis_labels(config, plot, frame),
            &view,
            Some(&target),
        );
        draw_hud(&status_lines(config, plot, frame), false, LEGEND);
        // Draws what is left into the picture.
        set_default_camera();
        recorder.frame(&target.texture.get_texture_data().bytes)?;
        if (frame + 1) % progress_step == 0 || frame + 1 == plots.len() {
            eprintln!("  {}/{} saved", frame + 1, plots.len());
        }
        view.orbit(turn(config, animation), 0.0, 0.0);
        next_frame().await;
    }
    let frames = recorder.finish()?;
    println!(
        "Saved video to {file}: {frames} frames, {} s",
        number(animation.seconds)
    );
    Ok(())
}

/// Draws the plot seen from `view` and its labels: into `target` when there is one, otherwise
/// into the window. The text that is drawn next goes the same way.
fn draw_plot(
    config: &Config,
    plot: &Plot,
    labels: &[Label],
    view: &View,
    target: Option<&RenderTarget>,
) {
    let aspect = screen_width().max(1.0) / screen_height().max(1.0);
    let mut camera = view.camera(aspect);
    camera.render_target = target.cloned();
    set_camera(&camera);
    clear_background(BACKGROUND);
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

    match target {
        Some(target) => set_camera(&Pixels(target)),
        None => set_default_camera(),
    }
    for label in labels {
        draw_label(&camera, label);
    }
}

/// The camera that the window has for its text, which measures in pixels from the top left
/// corner, for a picture that is drawn elsewhere.
struct Pixels<'a>(&'a RenderTarget);

impl Camera for Pixels<'_> {
    fn matrix(&self) -> Mat4 {
        Mat4::orthographic_rh_gl(0.0, screen_width(), screen_height(), 0.0, -1.0, 1.0)
    }

    fn depth_enabled(&self) -> bool {
        false
    }

    fn render_pass(&self) -> Option<RenderPass> {
        Some(self.0.render_pass.clone())
    }

    fn viewport(&self) -> Option<(i32, i32, i32, i32)> {
        None
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
    let dimensions = measure_text(&label.text, None, LABEL_TEXT, 1.0);
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
        f32::from(LABEL_TEXT),
        label.color,
    );
}

const LEGEND: &str = "cyan: start, pink: end";

/// The line of keys under the plot. `animation` is the state of one, whether it plays and
/// whether it is recorded, `None` without one.
fn hint(auto_rotate: bool, animation: Option<(bool, bool)>) -> String {
    format!(
        "{}{}",
        match animation {
            Some((true, false)) => "Space: pause the animation | M: record an MP4 | ",
            Some((false, false)) => "Space: play the animation | M: record an MP4 | ",
            Some((true, true)) => "Space: pause the animation | N: save the MP4 | ",
            Some((false, true)) => "Space: play the animation | N: save the MP4 | ",
            None => "",
        },
        if auto_rotate {
            "R: stop rotation | F1: help".to_owned()
        } else {
            format!("F1: help | {LEGEND}")
        }
    )
}

fn draw_hud(status: &[String], help: bool, hint: &str) {
    draw_rectangle(
        0.0,
        0.0,
        screen_width(),
        114.0,
        Color::new(0.02, 0.03, 0.05, 0.92),
    );
    for (index, line) in status.iter().enumerate() {
        let y = 28.0 + index as f32 * 24.0;
        draw_text(line, 18.0, y, f32::from(STATUS_TEXT), WHITE);
    }
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
                f32::from(KEYS_TEXT),
                LIGHTGRAY,
            );
        }
    } else {
        let y = screen_height() - 18.0;
        draw_text(hint, 18.0, y, f32::from(KEYS_TEXT), LIGHTGRAY);
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
            angles: crate::cli::Angles::default(),
            mp4: None,
            lossy: false,
            auto_rotation: None,
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
    fn the_first_view_has_the_given_angles_in_degrees_and_the_old_ones_otherwise() {
        let view = View::initial(&Angles::default());
        assert_eq!((view.yaw, view.pitch, view.roll), (0.65, 0.4, 0.0));
        let view = View::initial(&Angles {
            yaw: Some(0.0),
            pitch: Some(0.0),
            roll: None,
        });
        // Along the z axis: x to the right and y up.
        let camera = view.camera(1.5);
        assert!((camera.position - vec3(0.0, 0.0, view.distance)).length() < 1e-5);
        assert!((camera.up - Vec3::Y).length() < 1e-5);
        let view = View::initial(&Angles {
            yaw: Some(90.0),
            pitch: Some(30.0),
            roll: Some(-45.0),
        });
        assert!((view.yaw - std::f32::consts::FRAC_PI_2).abs() < 1e-6);
        assert!((view.pitch - std::f32::consts::FRAC_PI_6).abs() < 1e-6);
        assert!((view.roll - 7.0 * std::f32::consts::FRAC_PI_4).abs() < 1e-5);
        // Turns wrap; the pitch stops where W and S stop.
        for (degrees, same) in [(-270.0, 90.0), (810.0, 90.0), (-30.0, 330.0)] {
            let view = |yaw| {
                View::initial(&Angles {
                    yaw: Some(yaw),
                    ..Angles::default()
                })
                .camera(1.5)
                .position
            };
            assert!((view(degrees) - view(same)).length() < 1e-4, "{degrees}");
        }
        let mut limit = View::default();
        for (degrees, sign) in [(90.0, 1.0), (1e30, 1.0), (-88.0, -1.0), (f32::MIN, -1.0)] {
            limit.orbit(0.0, sign * 100.0, 0.0);
            let view = View::initial(&Angles {
                pitch: Some(degrees),
                ..Angles::default()
            });
            assert_eq!(view.pitch, limit.pitch, "{degrees}");
            assert!(view.camera(1.5).up.is_finite());
        }
    }

    #[test]
    fn a_recording_takes_every_frame_once_at_the_pace_of_the_video() {
        let animation = Animation {
            from: Complex64::new(2.0, 1.0),
            to: Complex64::new(6.0, 1.0),
            frames: 7,
            seconds: 0.35,
        };
        // From any moment of the animation the pictures are the frames that follow, in order.
        for start in [0.0, 0.0499, 0.05, 0.123, 0.3499] {
            let mut clock = start;
            let mut frame = frame_at(clock, &animation);
            for _ in 0..30 {
                clock = following(clock, &animation);
                frame = (frame + 1) % 7;
                assert_eq!(frame_at(clock, &animation), frame, "{start}");
            }
        }
        // The view of a video without a window turns once in all its frames, or as fast as
        // `--auto-rotation` says; that of a window as fast as `R` always turned it.
        let mut config = Config {
            function: crate::expression::Expression::parse("x").unwrap(),
            from: Complex64::new(0.0, 0.0),
            to: Complex64::new(1.0, 0.0),
            points: 2,
            screenshot: None,
            animation: None,
            angles: Angles::default(),
            mp4: None,
            lossy: false,
            auto_rotation: None,
        };
        assert_eq!((turn(&config, &animation), rotation(&config)), (0.0, 0.25));
        config.auto_rotation = Some(None);
        assert_eq!(turn(&config, &animation), std::f32::consts::TAU / 7.0);
        assert_eq!(rotation(&config), 0.25);
        config.auto_rotation = Some(Some(-30.0));
        assert!((turn(&config, &animation) + 1.5_f32.to_radians()).abs() < 1e-7);
        assert_eq!(rotation(&config), -30.0_f32.to_radians());
        // A window that draws 60 frames a second takes 20 pictures a second, the first at once.
        let mut pace = Pace::new(&animation);
        assert_eq!(pace.step, 0.05);
        let pictures: Vec<bool> = (0..600).map(|_| pace.due(1.0 / 60.0)).collect();
        assert!(pictures[0]);
        let count = pictures.iter().filter(|&&due| due).count();
        assert!((200..=201).contains(&count), "{count}");
        assert!(pictures.windows(2).all(|pair| !(pair[0] && pair[1])));
        // A window that is slower than the video takes a picture of every frame that it draws
        // and does not hurry after a pause.
        let mut pace = Pace::new(&animation);
        assert!((0..50).all(|_| pace.due(0.2)));
        assert!(pace.due(30.0));
        assert!(pace.due(0.01));
        assert!(!pace.due(0.01));
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
