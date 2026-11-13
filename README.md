# tet-line

An OpenGL-backed Rust viewer for complex functions along any straight line in the
complex plane. **All tetration (`^^`) is calculated by the external `tet` executable
on PATH**, and so are its inverses `slog` and `ssrt`. Other arithmetic and functions
are evaluated locally; this project contains no tetration implementation or
approximation.

## Build and run

Install Rust and, for expressions that need it, put the `tet` executable from
[claude-tetration](https://github.com/lukaszgryglicki/claude-tetration) on PATH.
A working desktop OpenGL display is required for the viewer.

```sh
cargo build --release --locked
./target/release/tet-line 'x^^x-x^x+x' 1 0 2 0
```

```text
tet-line 'expression' plot_from_re plot_from_im plot_to_re plot_to_im [number_of_points]
tet-line --expr=EXPRESSION plot_from_re plot_from_im plot_to_re plot_to_im [number_of_points]
tet-line base_re base_im plot_from_re plot_from_im plot_to_re plot_to_im [number_of_points]
```

The point count defaults to **512**, must be at least 2, and includes both endpoints.
All line/base coordinates must be finite numbers. Negative arguments work directly.
The original numeric form still plots `base^^x`. Six/seven numeric positionals
retain that meaning; use `--expr=2` to plot a numeric constant with a point count.

```sh
# Complex functions without tet:
./target/release/tet-line 'sin(x)+exp(i*x)' -2 -1 2 1

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
```

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

Calculation happens once, with progress in the terminal, before the window opens.
Changing the camera never invokes `tet` again. Solver failures, malformed output,
and final results outside finite `f64` range leave **gaps**, not connections across
missing points. Nonfinite tetration operands are skipped without starting `tet`.
An unavailable executable when needed, or a line with no finite results, is an error.
The viewer inherits the installed solver's mathematical coverage and accuracy;
13-digit working precision is not a certified error bound.

## Plot and controls

The x axis follows `x(t) = (1-t) * from + t * to`, for `0 <= t <= 1`; its ticks
show the actual complex arguments. The y axis is `Re(f(x))` and the z axis is
`Im(f(x))`. Result axes are fitted independently to their finite bounds, which are
also shown on screen. Constant components are centered. Cyan marks the start of
the line, pink the end; small markers keep isolated valid samples visible.

- Left mouse drag: orbit; right/middle drag: pan; wheel or `Z`/`X`: zoom.
- `A`/`D`: yaw; `W`/`S`: pitch; `Q`/`E`: roll; `R`: toggle automatic rotation.
- `0` or `Home`: reset the view; `P`: save the current view as `tet-line-<timestamp>.png`.
- `F1`: toggle help; `F11`: fullscreen; `Esc`: quit.

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

`Cargo.lock` belongs in version control; `target/`, Rust backups, and all `*.secret`
files (including nested ones) are ignored.
