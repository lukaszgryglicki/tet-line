# tet-line

An OpenGL-backed Rust viewer for complex functions along any straight line in the
complex plane. **All tetration (`^^`) is calculated by the external `tet` executable
on PATH.** Other arithmetic and functions are evaluated locally; this project
contains no tetration implementation or approximation.

## Build and run

Install Rust and, for expressions using `^^`, put the `tet` executable from
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
- Functions include complex trig/hyperbolic functions and their inverses,
  `exp`, `exp2`, `ln`, `log(x[,base])`, `log2`, `log10`, `sqrt`, `cbrt`,
  `pow`, `root`, `abs`, `arg`, `re`, `im`, `conj`, `complex`, `polar`,
  rounding, finite-value predicates, `gamma`, `factorial`, and the drawer's aliases.
  Powers, logarithms and roots follow `num-complex`'s principal branches.

Expressions are parsed once before calculation. Every evaluated `base^^height`
subexpression runs:

```text
tet 10 base_re base_im height_re height_im
```

`SILENT=1` suppresses solver progress without requiring a particular quiet flag;
stdout must contain the real and imaginary results as two numbers. The program
uses all available CPU cores (up to the point count), with one `tet` process per
worker at a time. Multiple `^^` operations within one sample run sequentially;
their operands may themselves be expressions. Each child gets `RAYON_NUM_THREADS=1`
to avoid nested oversubscription. Expressions without `^^` never invoke `tet`.
On 16 available CPUs, at most 16 samples are evaluated concurrently.

Calculation happens once, with progress in the terminal, before the window opens.
Changing the camera never invokes `tet` again. Solver failures, malformed output,
and final results outside finite `f64` range leave **gaps**, not connections across
missing points. Nonfinite tetration operands are skipped without starting `tet`.
An unavailable executable when needed, or a line with no finite results, is an error.
The viewer inherits the installed solver's mathematical coverage and accuracy;
10-digit working precision is not a certified error bound.

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

Apache-2.0; see [LICENSE](LICENSE). Graphics follow
[rust-opengl-complex-drawer](https://github.com/lukaszgryglicki/rust-opengl-complex-drawer):
Macroquad 0.4.14 with its vendored Miniquad 0.4.8
[FreeBSD X11/OpenGL patch](https://github.com/not-fl3/miniquad/pull/600).
The copy in `third_party/miniquad` retains its MIT and Apache licenses, so no sibling
checkout is needed to build. Linux and FreeBSD use the same graphics stack.

`Cargo.lock` belongs in version control; `target/`, Rust backups, and all `*.secret`
files (including nested ones) are ignored.
