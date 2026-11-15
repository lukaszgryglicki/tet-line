# tet-line

An OpenGL-backed Rust viewer for complex functions along any straight line in the
complex plane. **All tetration (`^^`) is calculated by the external `tet` executable
on PATH**, and so are its inverses `slog` and `ssrt`. Other arithmetic and functions
are evaluated locally; this project contains no tetration implementation or
approximation.

## Build and run

Install Rust and, for expressions that need it, put the `tet` executable from
[claude-tetration](https://github.com/lukaszgryglicki/claude-tetration) on PATH.
A working desktop OpenGL display is required for the viewer, and the `ffmpeg`
executable on PATH for [videos](#video).

```sh
cargo build --release --locked
./target/release/tet-line 'x^^x-x^x+x' 1 0 2 0
```

```text
tet-line 'expression' plot_from_re plot_from_im plot_to_re plot_to_im [number_of_points]
tet-line 'expression' plot_from plot_to [number_of_points]
tet-line --expr=EXPRESSION plot_from_re plot_from_im plot_to_re plot_to_im [number_of_points]
tet-line --expr=EXPRESSION plot_from plot_to [number_of_points]
tet-line base_re base_im plot_from_re plot_from_im plot_to_re plot_to_im [number_of_points]
tet-line ... --anim 'anim_from;anim_to;frames;seconds' [--auto-scale-frames]
tet-line ... --anim 'anim_from;anim_to;frames;seconds' --mp4=FILE.mp4 [--lossy]
tet-line ... --yaw degrees --pitch degrees --roll degrees --auto-rotation[=degrees]
tet-line ... --tet-prec digits
```

The line runs from the point `plot_from` to the point `plot_to`. Its two ends are
four real coordinates or two complex points, told apart by the number of arguments:
`-2 -1 2 1` and `-2-i 2+i` are the same line.
The point count defaults to **512**, must be at least 2, and includes both endpoints.
All line/base coordinates must be finite numbers. Negative arguments work directly.
The original numeric form still plots `base^^x`. Six/seven numeric positionals
retain that meaning; use `--expr=2` to plot a numeric constant with four coordinates
and a point count.

```sh
# Complex functions without tet:
./target/release/tet-line 'sin(x)+exp(i*x)' -2 -1 2 1

# The same line by its two complex end points, with 1024 samples:
./target/release/tet-line 'sin(x)+exp(i*x)' -2-i 2+i 1024

# An explicit complex tetration base:
./target/release/tet-line '(1.333-0.06i)^^x' -2 0 2 0

# Inverse tetration: the super-logarithm to base 2 and the super square root:
./target/release/tet-line 'ate(2, x) - ssrt(x, 2)' 1 0 2 0

# Imaginary height line, -3i to 3i:
./target/release/tet-line 1.333 -0.06 0 -3 0 3

# Diagonal height line with 1024 samples:
./target/release/tet-line 1.5 0.1 -2 -1 2 1 1024

# Save the initial view as a PNG and exit:
./target/release/tet-line --screenshot=line.png 1.333 -0.06 -2 0 2 0

# Ask tet for 6 significant digits instead of 13:
./target/release/tet-line --tet-prec 6 '2^^x' -1 3

# An animation: the line -2+i .. 6-i turns into 2+i .. 6+i in 100 frames that take 5 seconds:
./target/release/tet-line 'slog(2,x)' -2+i 6-i --anim '2+i;6+i;100;5'

# The same animation as a video, made without a window, seen from yaw 20 and pitch 30 degrees:
./target/release/tet-line 'slog(2,x)' -2+i 6-i --anim '2+i;6+i;100;5' --mp4=slog.mp4 --yaw 20 --pitch 30

# A video in which the view turns as well: once around in its 16 seconds, or 10 degrees a second:
./target/release/tet-line 'x^x' -3+3i 2-3i --anim '-3-3i;2+3i;512;16' --mp4=turn.mp4 --auto-rotation
./target/release/tet-line 'x^x' -3+3i 2-3i --anim '-3-3i;2+3i;512;16' --mp4=slow.mp4 --auto-rotation=10

# An animation whose Re and Im axes are fitted to every frame by itself, not once to all of them:
./target/release/tet-line 'x^x' -3+3i 2-3i --anim '-3-3i;2+3i;512;16' --auto-scale-frames
```

## Animation

`--anim 'anim_from;anim_to;frames;seconds'` (or `--anim=...`, anywhere among the arguments
of every form above) moves the line while it is drawn: its start goes from `plot_from` to
`anim_from` and its end from `plot_to` to `anim_to`, in `frames` equal steps. The first
frame is the line of the arguments and the last one the line `anim_from .. anim_to`, so
`frames` is at least 2. The frames are shown one after another in `seconds` (100 frames in
5 seconds are 20 a second), and after the last one the animation starts again from the
first. `Space` pauses and resumes it; rotation (`R`), the camera, `P` and everything else
work while it plays, and `P` saves the frame that is shown.

`anim_from` and `anim_to` are points like `plot_from` and `plot_to`: plain numbers or
constant expressions with a finite complex value, so `--anim '2^^0.5;-e^^i;600;20'` runs
`tet` twice for its two points. `seconds` is a positive number, for example `2.5`.

Every frame is calculated before the window opens, `frames` times `number_of_points`
values on all CPU cores, with the progress in the terminal; while the animation plays
nothing is calculated. The two result axes are fitted once, to the values of all frames,
so the curve moves in a box that stays; the ticks of the x axis and the status lines
follow the frame that is shown. A frame without any finite value is an empty box; an
animation without any finite value at all is an error.

`--auto-scale-frames` fits the two result axes to every frame by itself instead: each
frame is the most detailed view of its line, with the curve from side to side of the box
in `Re(f(x))` and in `Im(f(x))`, and the ticks of those axes and their bounds on screen
change from frame to frame. It holds wherever the animation is shown: in the window, in a
recording of it and in a video of `--mp4`. A frame without any finite value keeps the axes
of all frames. The terminal still reports the bounds of all values.

`--screenshot` saves the first frame, the line of the arguments, and calculates only that
one.

### Video

An animation can be saved as an MP4 video, written by the `ffmpeg` executable on PATH. The
video has the frame rate of the animation, `frames / seconds` pictures a second.

- **A recording of the window.** `M` starts it; `N` ends it and saves it as
  `tet-line-<timestamp>.mp4` in the current directory. The pictures are those of the window,
  as large as it is and with its text (`F1` hides the help). While a recording runs, time is
  the time of the video: every picture is one frame later than the picture before it, also
  on a computer that is too slow to show them at that pace. So the video has the frames of
  the animation one after another, each once, and automatic rotation (`R`) and the rotation
  and zoom keys move evenly in it. Everything else works as usual: `Space` pauses the
  animation, the mouse and the keys move the view, and the video shows what the window
  showed. A red dot on the window, which is not in the video, marks a recording. `Esc` and
  closing the window save it as well. A window that changes its size (`F11` included) ends
  and saves the recording, because a video has one size.
- **The whole animation, without a window.** `--mp4=FILE.mp4` draws every frame once, from
  the first to the last, and exits. No window is shown. The pictures have the size of the
  screen, as if the window was full screen, the view of `--yaw`, `--pitch` and `--roll`
  ([Plot and controls](#plot-and-controls)) and the status lines, but not the help.
  With `--auto-rotation` the view turns from frame to frame: once around in the whole
  video, so that its end meets its start, or with `--auto-rotation=degrees` by that many
  degrees of yaw in a second of the video.
  `ffmpeg` is tried before the calculation starts, so a long calculation is not lost to a
  video that cannot be written. A display is still needed, because the pictures are drawn
  by OpenGL; with several monitors the pictures have the size of the primary one.

Without `--lossy` the video is lossless: every frame is a PNG picture of its own, with the
highest compression of that format, and comes back exactly as it was drawn:

```sh
ffmpeg -i FILE.mp4 frame-%04d.png            # every frame as a PNG file
ffmpeg -i FILE.mp4 -c copy frame-%04d.png    # the same, the stored pictures as they are
```

Players built on FFmpeg, such as mpv and VLC, show such a video; browsers do not.
`--lossy`, for recordings and for `--mp4`, makes an H.265 video instead (libx265, CRF 20,
4:2:0, `hvc1`), which plays nearly everywhere and is about ten times smaller: 100 frames of
2880 x 1800 pixels took 7.1 MB lossless and 0.7 MB with `--lossy`.

## Expressions and calculation

Expressions use the same syntax and functions as
[rust-opengl-complex-drawer](https://github.com/lukaszgryglicki/rust-opengl-complex-drawer),
with `^^` added for tetration. `x` and `z` both denote the complex point along the
chosen line. Quote expressions to protect operators and parentheses from the shell.

- Operators: `+`, `-`, `*`, `/`, `^`, `^^`, unary signs and parentheses.
  `^^` binds more tightly than `^`; both associate right and bind before unary
  signs: `a^^b^^c` means `a^^(b^^c)`, `a^^b^c` means `(a^^b)^c`, and `-x^2`
  means `-(x^2)`. Use parentheses when mixing powers and tetration.
- Implicit multiplication: `2x`, `3i`, `2sin(x)`, `(x+1)(x-1)`.
  Constants: `i`/`j`, `pi`, `tau`, `e`, `inf`/`infinity`, `nan`; names are case-insensitive.
- Named real constants (aliases after `/`): `pi`, `e`, `tau` (2 pi), `phi`/`golden`
  (golden ratio), `gamma`/`eulergamma` (Euler-Mascheroni; `gamma(` is still the function),
  `catalan`, `apery`/`zeta3` (zeta(3)), `zeta2`/`basel` (pi^2/6), `ln2`, `ln10`, `sqrt2`,
  `sqrt3`, `sqrt5`, `sqrtpi`, `halfpi`, `eta`/`towermax` (e^(1/e)) and `towermin` (e^(-e)),
  the largest and smallest base whose infinite tower converges, `einv` (1/e), `omega`
  (W(1)), `dottie` (cos x = x), `gelfond` (e^pi), `gelfondschneider` (2^sqrt(2)),
  `ramanujan` (e^(pi sqrt 163)), `silver` (1 + sqrt 2), `plastic` (x^3 = x + 1),
  `supergolden` (x^3 = x^2 + 1), `tribonacci` (x^3 = x^2 + x + 1), `lemniscate`, `gauss`
  (1/agm(1, sqrt 2)), `levy`, `khinchin`, `erdosborwein` (sum of 1/(2^n - 1)), `cahen`,
  `recfib` (sum of 1/F(n)), `gompertz`, `soldner` (zero of the logarithmic integral),
  `laplace` (Laplace limit), `parabolic` (sqrt 2 + ln(1 + sqrt 2)), `deg`/`degree`
  (pi/180), `niven`, `twinprime`, `mertens`. Unicode aliases: `π`, `τ`, `φ`, `γ`, `η`,
  `Ω`, `ϖ` (lemniscate), `√2`, `√3`, `√5`, `√π`.
- Functions include complex trig/hyperbolic functions and their inverses,
  `exp`, `exp2`, `ln`, `log(x[,base])`, `log2`, `log10`, `sqrt`, `cbrt`,
  `pow`, `root`, `abs`, `arg`, `re`, `im`, `conj`, `complex`, `polar`,
  rounding, finite-value predicates, `gamma`, `factorial`, and the drawer's aliases.
  Powers, logarithms and roots follow `num-complex`'s principal branches.
  A zero real or imaginary part has no sign, as in `tet`, so a negative real number
  is on the upper side of a branch cut: `sqrt(-4)` is `2i`, `ln(-1)` is `pi i`,
  `(-8)^(1/3)` and `cbrt(-8)` are `1 + 1.732i`.
- `tet(b, h)` is `b^^h` written as a function: `tet(x, x)` is `x^^x`.
- Inverse tetration, solving `b^^h = z` through `tet`: `slog(b, z)` (alias `ate`) is
  the super-logarithm, the height `h`; `ssrt(z, h)` (alias `sroot`) is the super-root,
  the base `b`. Both arguments can be expressions: `x^^x + ate(2+i, x) - ssrt(x, 1.5-i)`.
- A line or base coordinate that is not a plain number is a constant expression in this
  syntax without `x`/`z`, evaluated once to a finite real value (`^^`, `tet`, `slog` and
  `ssrt` run `tet`): `tet-line 'sin(x)' -pi 0 tau/3 0`, `'pi*e^e-pow(2, 0.5)'`, `1.5^^2`.
  As `base_re` it needs the point count, because a first argument that is not a plain
  number is otherwise the expression; the point count itself stays an integer.
- The two points `plot_from` and `plot_to` are constant expressions of the same kind
  with a finite complex value: `tet-line 'x^^x' 'e^-e' e`, `-2-i 2+i`,
  `'exp(i*pi/4)' '2^^0.5'`. Each of four coordinates has to be real instead;
  `re(...)` and `im(...)` take the parts of any expression.

Expressions are parsed once before calculation. Every evaluated `base^^height`
or `tet(base, height)` subexpression runs:

```text
tet 13 base_re base_im height_re height_im
```

Every evaluated `slog(b, z)` and `ssrt(z, h)` runs one of the solver's inverse modes
(a `tet` build that has them is required), with the arguments in the order written:

```text
tet -q --slog 13 b_re b_im z_re z_im
tet -q --sroot 13 z_re z_im h_re h_im
```

The `13` of those three commands is the number of significant digits that `tet` is asked
for. `--tet-prec N` (or `--tet-prec=N`) asks for `N` instead, an integer from 1 to 15, in
every run of `tet`: for the samples and for constant coordinates such as `1.5^^2`,
wherever the option stands among the arguments. More than 15 digits would be lost: the
results are kept as `f64` numbers, which hold 15 significant digits. Fewer digits make
`tet` faster for some bases (base 2 at 3 digits takes about a quarter of the time of 13)
and change nothing for others. The terminal and the status lines of the window name the
digits.

`SILENT=1` suppresses solver progress without requiring a particular quiet flag;
stdout must contain the real and imaginary results as two numbers. The program
uses all available CPU cores (up to the point count), with one `tet` process per
worker at a time. Multiple `tet` operations within one sample run sequentially;
their operands may themselves be expressions. Each child gets `RAYON_NUM_THREADS=1`
to avoid nested oversubscription. Expressions without `^^`, `tet`, `slog` or `ssrt`
never invoke `tet`. On 16 available CPUs, at most 16 samples are evaluated concurrently.

A super-root at a non-integer height can take seconds to minutes per sample, so
start with a small point count. An inverse that `tet` cannot find and verify, such
as a value the base's tetration never takes, is a solver failure.

Calculation happens once, with progress in the terminal, before the window opens
(for an [animation](#animation): of every frame).
Changing the camera never invokes `tet` again. Solver failures, malformed output,
and final results outside finite `f64` range leave **gaps**, not connections across
missing points. Nonfinite tetration operands are skipped without starting `tet`.
An unavailable executable when needed, or a line with no finite results, is an error.
The viewer inherits the installed solver's mathematical coverage and accuracy;
the digits that it is asked for, 13 or those of `--tet-prec`, are not a certified error
bound.

## Plot and controls

The x axis follows `x(t) = (1-t) * from + t * to`, for `0 <= t <= 1`; its ticks
show the actual complex arguments. The y axis is `Re(f(x))` and the z axis is
`Im(f(x))`. Result axes are fitted independently to their finite bounds, which are
also shown on screen. Constant components are centered. Cyan marks the start of
the line, pink the end; small markers keep isolated valid samples visible.

- Left mouse drag: orbit; right/middle drag: pan; wheel or `Z`/`X`: zoom.
- `A`/`D`: yaw; `W`/`S`: pitch; `Q`/`E`: roll; `R`: toggle automatic rotation.
- `0` or `Home`: reset the view; `P`: save the current view as `tet-line-<timestamp>.png`.
- `Space`: pause or resume an [animation](#animation); `M`: start a [video](#video) of
  it; `N`: end and save the video.
- `F1`: toggle help; `F11`: fullscreen; `Esc`: quit.

`--yaw degrees`, `--pitch degrees` and `--roll degrees` (or `--yaw=degrees`, ...; negative
values work directly) turn the first view, the one that `0` and `Home` return to. Without
them it is as before: yaw 37.24, pitch 22.92 and roll 0 degrees. Yaw 0 with pitch 0 looks
along the z axis, with x to the right and y up. A larger yaw moves the camera to the right
around the plot (at 90 it looks along the x axis), a larger pitch lifts it (towards a view
from above; it stays within 87.09 degrees, where `W` and `S` stop too), and a larger roll
turns the picture counter-clockwise.

`--auto-rotation` starts with the automatic rotation of `R` on, and
`--auto-rotation=degrees` also sets its speed, in degrees of yaw a second; a negative
speed turns the other way. Without a speed the window turns as it always did after `R`,
14.32 degrees a second, and a [video](#video) made by `--mp4` turns once in its whole
length. `--screenshot` still saves the first view.

`--help` works without `tet` or a display.

## Development and license

```sh
cargo fmt -- --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

AGPL-3.0-only; see [LICENSE](LICENSE). Graphics follow
[rust-opengl-complex-drawer](https://github.com/lukaszgryglicki/rust-opengl-complex-drawer):
Macroquad 0.4.14 with its vendored Miniquad 0.4.8
[FreeBSD X11/OpenGL patch](https://github.com/not-fl3/miniquad/pull/600).
The copy in `third_party/miniquad` retains its MIT and Apache licenses, so no sibling
checkout is needed to build. Linux and FreeBSD use the same graphics stack.
For `--mp4` the copy has three more changes: `Platform::linux_x11_hidden` (an X11 window
that is never shown and, with `fullscreen`, has the size of the primary monitor, which
Xrandr tells), the depth buffer of a multisampled render target attached as the
renderbuffer that it is (as in upstream `master`), and that buffer 24 bits deep like the
one of a window instead of 16.

`Cargo.lock` belongs in version control; `target/`, Rust backups, and all `*.secret`
files (including nested ones) are ignored.
