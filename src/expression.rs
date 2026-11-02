use num_complex::{Complex64 as C, ComplexFloat};

#[derive(Clone, Debug)]
pub struct Expression {
    source: String,
    root: Expr,
}

impl Expression {
    pub fn parse(source: &str) -> Result<Self, String> {
        let mut parser = Parser {
            tokens: tokenize(source)?,
            pos: 0,
        };
        let root = parser.expression(0)?;
        if !matches!(parser.current(), TokenKind::End) {
            return Err(parser.error("expected end of expression"));
        }
        Ok(Self {
            source: source.to_owned(),
            root,
        })
    }

    pub fn tetration(base: C) -> Self {
        Self {
            source: format!("({base})^^x"),
            root: Expr::Binary(
                BinaryOp::Tet,
                Box::new(Expr::Constant(base)),
                Box::new(Expr::Variable),
            ),
        }
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn uses_tet(&self) -> bool {
        self.root.uses_tet()
    }

    pub fn evaluate<E>(&self, x: C, tet: &mut impl FnMut(C, C) -> Result<C, E>) -> Result<C, E> {
        self.root.evaluate(x, tet)
    }
}

#[derive(Clone, Debug)]
enum Expr {
    Constant(C),
    Variable,
    Negate(Box<Expr>),
    Binary(BinaryOp, Box<Expr>, Box<Expr>),
    Function(String, Vec<Expr>),
}

impl Expr {
    fn uses_tet(&self) -> bool {
        match self {
            Self::Constant(_) | Self::Variable => false,
            Self::Negate(arg) => arg.uses_tet(),
            Self::Binary(op, left, right) => {
                matches!(op, BinaryOp::Tet) || left.uses_tet() || right.uses_tet()
            }
            Self::Function(_, args) => args.iter().any(Self::uses_tet),
        }
    }

    fn evaluate<E>(&self, x: C, tet: &mut impl FnMut(C, C) -> Result<C, E>) -> Result<C, E> {
        Ok(match self {
            Self::Constant(value) => *value,
            Self::Variable => x,
            Self::Negate(arg) => -arg.evaluate(x, tet)?,
            Self::Binary(op, left, right) => {
                let a = left.evaluate(x, tet)?;
                let b = right.evaluate(x, tet)?;
                match op {
                    BinaryOp::Add => a + b,
                    BinaryOp::Sub => a - b,
                    BinaryOp::Mul => a * b,
                    BinaryOp::Div => a / b,
                    BinaryOp::Pow => a.powc(b),
                    BinaryOp::Tet => tet(a, b)?,
                }
            }
            Self::Function(name, args) => {
                let mut values = [C::new(0.0, 0.0); 2];
                for (value, arg) in values.iter_mut().zip(args) {
                    *value = arg.evaluate(x, tet)?;
                }
                eval_function(name, &values[..args.len()])
            }
        })
    }
}

fn eval_function(name: &str, args: &[C]) -> C {
    let a = args[0];
    let one = C::new(1.0, 0.0);
    match name {
        "abs" | "mag" | "mod" | "norm" | "polar_r" | "radius" => C::new(a.norm(), 0.0),
        "abs2" | "mag2" | "norm_sqr" | "normsq" => C::new(a.norm_sqr(), 0.0),
        "l1_norm" | "l1" | "manhattan" | "taxicab" => C::new(a.l1_norm(), 0.0),
        "arg" | "phase" | "polar_theta" | "theta" => C::new(a.arg(), 0.0),
        "to_polar" => {
            let (r, theta) = a.to_polar();
            C::new(r, theta)
        }
        "re" | "real" => C::new(a.re, 0.0),
        "im" | "imag" => C::new(a.im, 0.0),
        "conj" | "conjugate" => a.conj(),
        "inv" => a.inv(),
        "recip" | "inverse" => a.recip(),
        "finv" => a.finv(),
        "is_nan" | "isnan" => bool_to_complex(a.is_nan()),
        "is_infinite" | "isinf" | "isinfinite" => bool_to_complex(a.is_infinite()),
        "is_finite" | "isfinite" => bool_to_complex(a.is_finite()),
        "is_normal" | "isnormal" => bool_to_complex(a.is_normal()),
        "sgn" | "sign" | "signum" => {
            let norm = a.norm();
            if norm == 0.0 {
                C::new(0.0, 0.0)
            } else {
                a / norm
            }
        }
        "cis" => C::cis(a.re),
        "exp" => a.exp(),
        "exp2" => a.exp2(),
        "expf" => a.expf(args[1].re),
        "ln" => a.ln(),
        "log" => {
            if args.len() == 1 {
                a.ln()
            } else {
                a.ln() / args[1].ln()
            }
        }
        "log2" => a.log2(),
        "log10" => a.log10(),
        "sqrt" => a.sqrt(),
        "cbrt" => a.cbrt(),
        "sqr" | "square" => a * a,
        "sin" => a.sin(),
        "cos" => a.cos(),
        "tan" => a.tan(),
        "asin" | "arcsin" => a.asin(),
        "acos" | "arccos" => a.acos(),
        "atan" | "arctan" => a.atan(),
        "sinh" => a.sinh(),
        "cosh" => a.cosh(),
        "tanh" => a.tanh(),
        "asinh" | "arcsinh" => a.asinh(),
        "acosh" | "arccosh" => a.acosh(),
        "atanh" | "arctanh" => a.atanh(),
        "sec" => one / a.cos(),
        "csc" => one / a.sin(),
        "cot" => one / a.tan(),
        "sech" => one / a.cosh(),
        "csch" => one / a.sinh(),
        "coth" => one / a.tanh(),
        "pow" | "powc" => a.powc(args[1]),
        "powf" => a.powf(args[1].re),
        "powi" => a.powi(complex_to_i32(args[1])),
        "powu" => a.powu(complex_to_u32(args[1])),
        "root" => a.powc(one / args[1]),
        "scale" => a.scale(args[1].re),
        "unscale" => a.unscale(args[1].re),
        "fdiv" => a.fdiv(args[1]),
        "complex" | "rect" | "new" => C::new(a.re, args[1].re),
        "polar" | "from_polar" => C::from_polar(a.re, args[1].re),
        "floor" => C::new(a.re.floor(), a.im.floor()),
        "ceil" => C::new(a.re.ceil(), a.im.ceil()),
        "round" => C::new(a.re.round(), a.im.round()),
        "trunc" => C::new(a.re.trunc(), a.im.trunc()),
        "frac" | "fract" => C::new(a.re.fract(), a.im.fract()),
        "minabs" => {
            if a.norm() <= args[1].norm() {
                a
            } else {
                args[1]
            }
        }
        "maxabs" => {
            if a.norm() >= args[1].norm() {
                a
            } else {
                args[1]
            }
        }
        "avg" => (a + args[1]) / 2.0,
        "gamma" | "tgamma" => complex_gamma(a),
        "factorial" | "fact" => complex_gamma(a + one),
        _ => unreachable!("function names are validated while parsing"),
    }
}

fn complex_gamma(z: C) -> C {
    // Lanczos (g = 7), with reflection for Re(z) < 0.5.
    #[allow(clippy::excessive_precision)]
    const COEFFICIENTS: [f64; 9] = [
        0.99999999999980993,
        676.5203681218851,
        -1259.1392167224028,
        771.32342877765313,
        -176.61502916214059,
        12.507343278686905,
        -0.13857109526572012,
        9.9843695780195716e-6,
        1.5056327351493116e-7,
    ];
    let pi = std::f64::consts::PI;
    if z.re < 0.5 {
        pi / ((z * pi).sin() * complex_gamma(C::new(1.0, 0.0) - z))
    } else {
        let z = z - 1.0;
        let mut sum = C::new(COEFFICIENTS[0], 0.0);
        for (i, coefficient) in COEFFICIENTS.iter().enumerate().skip(1) {
            sum += coefficient / (z + i as f64);
        }
        let t = z + 7.5;
        (2.0 * pi).sqrt() * t.powc(z + 0.5) * (-t).exp() * sum
    }
}

fn bool_to_complex(value: bool) -> C {
    C::new(if value { 1.0 } else { 0.0 }, 0.0)
}

fn complex_to_i32(value: C) -> i32 {
    if value.re.is_finite() {
        value.re.round() as i32
    } else {
        0
    }
}

fn complex_to_u32(value: C) -> u32 {
    if value.re.is_finite() {
        value.re.round() as u32
    } else {
        0
    }
}

fn function_arity(name: &str) -> Option<(usize, usize)> {
    match name {
        "abs" | "mag" | "mod" | "norm" | "abs2" | "mag2" | "norm_sqr" | "normsq" | "l1_norm"
        | "l1" | "manhattan" | "taxicab" | "arg" | "phase" | "to_polar" | "polar_r" | "radius"
        | "polar_theta" | "theta" | "re" | "real" | "im" | "imag" | "conj" | "conjugate"
        | "inv" | "recip" | "inverse" | "finv" | "is_nan" | "isnan" | "is_infinite" | "isinf"
        | "isinfinite" | "is_finite" | "isfinite" | "is_normal" | "isnormal" | "sgn" | "sign"
        | "signum" | "cis" | "exp" | "exp2" | "ln" | "log2" | "log10" | "sqrt" | "cbrt" | "sqr"
        | "square" | "sin" | "cos" | "tan" | "asin" | "arcsin" | "acos" | "arccos" | "atan"
        | "arctan" | "sinh" | "cosh" | "tanh" | "asinh" | "arcsinh" | "acosh" | "arccosh"
        | "atanh" | "arctanh" | "sec" | "csc" | "cot" | "sech" | "csch" | "coth" | "floor"
        | "ceil" | "round" | "trunc" | "frac" | "fract" | "gamma" | "tgamma" | "factorial"
        | "fact" => Some((1, 1)),
        "pow" | "powc" | "powf" | "powi" | "powu" | "root" | "scale" | "unscale" | "fdiv"
        | "complex" | "rect" | "new" | "polar" | "from_polar" | "minabs" | "maxabs" | "avg"
        | "expf" => Some((2, 2)),
        "log" => Some((1, 2)),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug)]
enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Tet,
}

#[derive(Debug)]
struct Token {
    kind: TokenKind,
    pos: usize,
}

#[derive(Clone, Debug)]
enum TokenKind {
    Number(f64),
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    Tet,
    LParen,
    RParen,
    Comma,
    End,
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn expression(&mut self, min_precedence: u8) -> Result<Expr, String> {
        let mut left = match self.current() {
            TokenKind::Plus => {
                self.bump();
                self.expression(3)?
            }
            TokenKind::Minus => {
                self.bump();
                Expr::Negate(Box::new(self.expression(3)?))
            }
            _ => self.primary()?,
        };
        loop {
            let (op, precedence, right_associative, implicit) = match self.current() {
                TokenKind::Plus => (BinaryOp::Add, 1, false, false),
                TokenKind::Minus => (BinaryOp::Sub, 1, false, false),
                TokenKind::Star => (BinaryOp::Mul, 2, false, false),
                TokenKind::Slash => (BinaryOp::Div, 2, false, false),
                TokenKind::Caret => (BinaryOp::Pow, 4, true, false),
                TokenKind::Tet => (BinaryOp::Tet, 5, true, false),
                TokenKind::Number(_) | TokenKind::Ident(_) | TokenKind::LParen => {
                    (BinaryOp::Mul, 2, false, true)
                }
                _ => break,
            };
            if precedence < min_precedence {
                break;
            }
            if !implicit {
                self.bump();
            }
            let right = self.expression(precedence + u8::from(!right_associative))?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn primary(&mut self) -> Result<Expr, String> {
        match self.current().clone() {
            TokenKind::Number(value) => {
                self.bump();
                Ok(Expr::Constant(C::new(value, 0.0)))
            }
            TokenKind::Ident(name) => {
                let name = if name == "\u{3c0}" {
                    "pi".to_owned()
                } else {
                    name.to_ascii_lowercase()
                };
                if let Some((min, max)) = function_arity(&name) {
                    self.bump();
                    if !matches!(self.current(), TokenKind::LParen) {
                        return Err(self.error("expected `(` after function name"));
                    }
                    self.bump();
                    let mut args = Vec::new();
                    if !matches!(self.current(), TokenKind::RParen) {
                        loop {
                            args.push(self.expression(0)?);
                            if !matches!(self.current(), TokenKind::Comma) {
                                break;
                            }
                            self.bump();
                        }
                    }
                    self.close_parenthesis()?;
                    if !(min..=max).contains(&args.len()) {
                        return Err(self.error(&format!(
                            "wrong number of arguments for `{name}`: got {}",
                            args.len()
                        )));
                    }
                    Ok(Expr::Function(name, args))
                } else {
                    let expr = match name.as_str() {
                        "x" | "z" => Expr::Variable,
                        "i" | "j" => Expr::Constant(C::new(0.0, 1.0)),
                        "pi" => Expr::Constant(C::new(std::f64::consts::PI, 0.0)),
                        "tau" => Expr::Constant(C::new(std::f64::consts::TAU, 0.0)),
                        "e" => Expr::Constant(C::new(std::f64::consts::E, 0.0)),
                        "inf" | "infinity" => Expr::Constant(C::new(f64::INFINITY, 0.0)),
                        "nan" => Expr::Constant(C::new(f64::NAN, 0.0)),
                        _ => return Err(self.error(&format!("unknown identifier `{name}`"))),
                    };
                    self.bump();
                    Ok(expr)
                }
            }
            TokenKind::LParen => {
                self.bump();
                let expr = self.expression(0)?;
                self.close_parenthesis()?;
                Ok(expr)
            }
            _ => Err(self.error("expected a number, variable, function call, or `(`")),
        }
    }

    fn close_parenthesis(&mut self) -> Result<(), String> {
        if !matches!(self.current(), TokenKind::RParen) {
            return Err(self.error("expected `)`"));
        }
        self.bump();
        Ok(())
    }

    fn current(&self) -> &TokenKind {
        &self.tokens[self.pos].kind
    }

    fn bump(&mut self) {
        if !matches!(self.current(), TokenKind::End) {
            self.pos += 1;
        }
    }

    fn error(&self, message: &str) -> String {
        format!("{message} at byte {}", self.tokens[self.pos].pos)
    }
}

fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let chars: Vec<_> = input.char_indices().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let (pos, ch) = chars[i];
        if ch.is_whitespace() {
            i += 1;
            continue;
        }
        let kind = match ch {
            '+' => TokenKind::Plus,
            '-' | '\u{2212}' => TokenKind::Minus,
            '*' | '\u{b7}' => TokenKind::Star,
            '/' | '\u{f7}' => TokenKind::Slash,
            '^' => {
                if chars.get(i + 1).is_some_and(|&(_, next)| next == '^') {
                    i += 1;
                    TokenKind::Tet
                } else {
                    TokenKind::Caret
                }
            }
            '(' => TokenKind::LParen,
            ')' => TokenKind::RParen,
            ',' => TokenKind::Comma,
            _ if ch.is_ascii_digit()
                || (ch == '.' && chars.get(i + 1).is_some_and(|&(_, c)| c.is_ascii_digit())) =>
            {
                let end = consume_number(&chars, i);
                let end_byte = chars.get(end).map_or(input.len(), |&(byte, _)| byte);
                let text = &input[pos..end_byte];
                i = end - 1;
                TokenKind::Number(
                    text.parse()
                        .map_err(|_| format!("invalid number `{text}` at byte {pos}"))?,
                )
            }
            _ if ch == '_' || ch.is_alphabetic() => {
                let mut end = i + 1;
                while chars
                    .get(end)
                    .is_some_and(|&(_, c)| c == '_' || c.is_alphanumeric())
                {
                    end += 1;
                }
                let end_byte = chars.get(end).map_or(input.len(), |&(byte, _)| byte);
                i = end - 1;
                TokenKind::Ident(input[pos..end_byte].to_owned())
            }
            _ => return Err(format!("unexpected character `{ch}` at byte {pos}")),
        };
        tokens.push(Token { kind, pos });
        i += 1;
    }
    tokens.push(Token {
        kind: TokenKind::End,
        pos: input.len(),
    });
    Ok(tokens)
}

fn consume_number(chars: &[(usize, char)], mut i: usize) -> usize {
    while chars.get(i).is_some_and(|&(_, c)| c.is_ascii_digit()) {
        i += 1;
    }
    if chars.get(i).is_some_and(|&(_, c)| c == '.') {
        i += 1;
        while chars.get(i).is_some_and(|&(_, c)| c.is_ascii_digit()) {
            i += 1;
        }
    }
    if chars.get(i).is_some_and(|&(_, c)| matches!(c, 'e' | 'E')) {
        let mut exponent = i + 1;
        if chars
            .get(exponent)
            .is_some_and(|&(_, c)| matches!(c, '+' | '-'))
        {
            exponent += 1;
        }
        if chars
            .get(exponent)
            .is_some_and(|&(_, c)| c.is_ascii_digit())
        {
            i = exponent + 1;
            while chars.get(i).is_some_and(|&(_, c)| c.is_ascii_digit()) {
                i += 1;
            }
        }
    }
    i
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eval(source: &str, x: C) -> C {
        Expression::parse(source)
            .unwrap()
            .evaluate(x, &mut |_, _| Err("unexpected tetration"))
            .unwrap()
    }

    fn assert_close(actual: C, expected: C) {
        assert!(
            (actual - expected).norm() < 1e-10,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn arithmetic_matches_drawer_precedence_and_implicit_multiplication() {
        for (source, expected) in [
            ("2+3*4", 14.0),
            ("(2+3)*4", 20.0),
            ("2^3^2", 512.0),
            ("-2^2", -4.0),
            ("(-2)^2", 4.0),
            ("2^-2^2", 0.0625),
            ("6/3/2", 1.0),
            ("2x", 6.0),
            ("(x+1)(x-1)", 8.0),
            ("2cos(0)x", 6.0),
            ("x(x-1)", 6.0),
            ("3(2)", 6.0),
            ("1e-3 + .5E+1", 5.001),
            ("2e", 2.0 * std::f64::consts::E),
            ("3\u{b7}4\u{2212}6\u{f7}2", 9.0),
        ] {
            assert_close(eval(source, C::new(3.0, 0.0)), C::new(expected, 0.0));
        }
    }

    #[test]
    fn complex_constants_variables_and_functions_match_the_drawer() {
        let x = C::new(0.7, 0.3);
        for (source, expected) in [
            ("x", x),
            ("Z", x),
            ("i*j", C::new(-1.0, 0.0)),
            ("tau/2-\u{3c0}", C::new(0.0, 0.0)),
            ("exp(i*pi)", C::new(-1.0, 0.0)),
            ("sin(x)^2+cos(x)^2", C::new(1.0, 0.0)),
            ("ln(exp(x))", x),
            ("conj(x)", x.conj()),
            ("REAL(x)+i*IMAG(x)", x),
            ("abs(3+4i)", C::new(5.0, 0.0)),
            ("log(8,2)", C::new(3.0, 0.0)),
            ("pow(i,2)", C::new(-1.0, 0.0)),
            ("root(-1,2)", C::new(0.0, -1.0)),
            ("root(complex(-1,0),2)", C::new(0.0, 1.0)),
            ("polar(2,pi/2)", C::new(0.0, 2.0)),
            ("complex(2,-3)", C::new(2.0, -3.0)),
            ("sqrt(-4)", C::new(0.0, -2.0)),
            ("sqrt(complex(-4,0))", C::new(0.0, 2.0)),
            ("cbrt(8)", C::new(2.0, 0.0)),
        ] {
            assert_close(eval(source, x), expected);
        }
    }

    #[test]
    fn supports_function_aliases_and_the_remaining_complex_operations() {
        let x = C::new(0.7, 0.3);
        for (names, args, expected) in [
            (
                &["abs", "mag", "mod", "norm", "polar_r", "radius"][..],
                "3+4i",
                C::new(5.0, 0.0),
            ),
            (
                &["abs2", "mag2", "norm_sqr", "normsq"][..],
                "3+4i",
                C::new(25.0, 0.0),
            ),
            (
                &["l1_norm", "l1", "manhattan", "taxicab"][..],
                "-3+4i",
                C::new(7.0, 0.0),
            ),
            (
                &["arg", "phase", "polar_theta", "theta"][..],
                "i",
                C::new(std::f64::consts::FRAC_PI_2, 0.0),
            ),
            (&["re", "real"][..], "x", C::new(x.re, 0.0)),
            (&["im", "imag"][..], "x", C::new(x.im, 0.0)),
            (&["conj", "conjugate"][..], "x", x.conj()),
            (
                &["inv", "recip", "inverse", "finv"][..],
                "x",
                C::new(1.0, 0.0) / x,
            ),
            (&["sgn", "sign", "signum"][..], "3+4i", C::new(0.6, 0.8)),
            (&["ln", "log"][..], "x", x.ln()),
            (&["sqr", "square"][..], "x", x * x),
            (&["asin", "arcsin"][..], "x", x.asin()),
            (&["acos", "arccos"][..], "x", x.acos()),
            (&["atan", "arctan"][..], "x", x.atan()),
            (&["asinh", "arcsinh"][..], "x", x.asinh()),
            (&["acosh", "arccosh"][..], "x", x.acosh()),
            (&["atanh", "arctanh"][..], "x", x.atanh()),
            (&["pow", "powc"][..], "x,1+i", x.powc(C::new(1.0, 1.0))),
            (&["complex", "rect", "new"][..], "2,3", C::new(2.0, 3.0)),
            (&["polar", "from_polar"][..], "2,0", C::new(2.0, 0.0)),
            (&["frac", "fract"][..], "1.25-2.5i", C::new(0.25, -0.5)),
            (&["gamma", "tgamma"][..], "5", C::new(24.0, 0.0)),
            (&["factorial", "fact"][..], "4", C::new(24.0, 0.0)),
        ] {
            for name in names {
                assert_close(eval(&format!("{name}({args})"), x), expected);
            }
        }
        for (source, expected) in [
            ("to_polar(3+4i)", C::new(5.0, 4.0_f64.atan2(3.0))),
            ("sign(0)", C::new(0.0, 0.0)),
            ("cis(pi)", C::new(-1.0, 0.0)),
            ("exp2(x)", x.exp2()),
            ("expf(x,3)", x.expf(3.0)),
            ("log2(x)", x.log2()),
            ("log10(x)", x.log10()),
            ("tan(x)", x.tan()),
            ("sinh(x)", x.sinh()),
            ("cosh(x)", x.cosh()),
            ("tanh(x)", x.tanh()),
            ("sec(x)", C::new(1.0, 0.0) / x.cos()),
            ("csc(x)", C::new(1.0, 0.0) / x.sin()),
            ("cot(x)", C::new(1.0, 0.0) / x.tan()),
            ("sech(x)", C::new(1.0, 0.0) / x.cosh()),
            ("csch(x)", C::new(1.0, 0.0) / x.sinh()),
            ("coth(x)", C::new(1.0, 0.0) / x.tanh()),
            ("powf(x,0.5)", x.powf(0.5)),
            ("powi(x,-2)", x.powi(-2)),
            ("powu(x,2)", x.powu(2)),
            ("scale(x,2)", x * 2.0),
            ("unscale(x,2)", x / 2.0),
            ("fdiv(x,1+i)", x / C::new(1.0, 1.0)),
            ("floor(1.25-2.5i)", C::new(1.0, -3.0)),
            ("ceil(1.25-2.5i)", C::new(2.0, -2.0)),
            ("round(1.25-2.5i)", C::new(1.0, -3.0)),
            ("trunc(1.25-2.5i)", C::new(1.0, -2.0)),
            ("minabs(3,1+i)", C::new(1.0, 1.0)),
            ("maxabs(3,1+i)", C::new(3.0, 0.0)),
            ("avg(3,1+i)", C::new(2.0, 0.5)),
        ] {
            assert_close(eval(source, x), expected);
        }
    }

    #[test]
    fn nonfinite_predicates_can_inspect_intermediate_values() {
        for (names, arg) in [
            (&["is_nan", "isnan"][..], "0/0"),
            (&["is_infinite", "isinf", "isinfinite"][..], "inf"),
            (&["is_finite", "isfinite"][..], "x"),
            (&["is_normal", "isnormal"][..], "1+i"),
        ] {
            for name in names {
                assert_close(
                    eval(&format!("{name}({arg})"), C::new(1.0, 2.0)),
                    C::new(1.0, 0.0),
                );
            }
        }
        assert_close(eval("isfinite(1/0)", C::new(0.0, 0.0)), C::new(0.0, 0.0));
        assert_close(eval("isnan(nan)", C::new(0.0, 0.0)), C::new(1.0, 0.0));
    }

    #[test]
    fn gamma_supports_reflection_and_complex_arguments() {
        let x = C::new(0.0, 0.0);
        assert_close(
            eval("gamma(0.5)", x),
            C::new(std::f64::consts::PI.sqrt(), 0.0),
        );
        assert_close(
            eval("gamma(-0.5)", x),
            C::new(-2.0 * std::f64::consts::PI.sqrt(), 0.0),
        );
        let expected = std::f64::consts::PI / std::f64::consts::PI.sinh();
        assert!((eval("gamma(i)", x).norm_sqr() - expected).abs() < 1e-10);
    }

    #[test]
    fn tetration_has_higher_precedence_than_power_and_associates_right() {
        for (source, expected, arguments) in [
            ("2^^3^2", 25.0, vec![(2.0, 3.0)]),
            ("2^3^^2", 32.0, vec![(3.0, 2.0)]),
            ("2^^3^^2", 7.0, vec![(3.0, 2.0), (2.0, 5.0)]),
            ("-2^^2", -4.0, vec![(2.0, 2.0)]),
            ("(-2)^^2", 0.0, vec![(-2.0, 2.0)]),
            ("2^^-3", -1.0, vec![(2.0, -3.0)]),
            ("(2^3)^^2", 10.0, vec![(8.0, 2.0)]),
        ] {
            let mut calls = Vec::new();
            let value = Expression::parse(source)
                .unwrap()
                .evaluate(C::new(0.0, 0.0), &mut |a, b| {
                    calls.push((a, b));
                    Ok::<_, ()>(a + b)
                })
                .unwrap();
            assert_close(value, C::new(expected, 0.0));
            assert_eq!(calls.len(), arguments.len(), "{source}");
            for ((a, b), (expected_a, expected_b)) in calls.into_iter().zip(arguments) {
                assert_close(a, C::new(expected_a, 0.0));
                assert_close(b, C::new(expected_b, 0.0));
            }
        }
    }

    #[test]
    fn requested_expression_passes_both_complex_operands_to_tet_once() {
        let x = C::new(1.3, -0.2);
        let tet_value = C::new(4.0, 5.0);
        let mut calls = Vec::new();
        let value = Expression::parse("x^^x-x^x+x")
            .unwrap()
            .evaluate(x, &mut |a, b| {
                calls.push((a, b));
                Ok::<_, ()>(tet_value)
            })
            .unwrap();
        assert_close(value, tet_value - x.powc(x) + x);
        assert_eq!(calls, [(x, x)]);
    }

    #[test]
    fn nested_tetration_and_functions_preserve_errors_without_retries() {
        let expression = Expression::parse("sin((x+1)^^(x-i)) + x^^2").unwrap();
        assert!(expression.uses_tet());
        assert!(!Expression::parse("sin(x)^2+x").unwrap().uses_tet());
        let mut calls = Vec::new();
        let result = expression.evaluate(C::new(2.0, 3.0), &mut |a, b| {
            calls.push((a, b));
            Err::<C, _>("solver failed")
        });
        assert_eq!(result, Err("solver failed"));
        assert_eq!(calls, [(C::new(3.0, 3.0), C::new(2.0, 2.0))]);
    }

    #[test]
    fn rejects_invalid_syntax_unknown_functions_and_wrong_arities() {
        for source in [
            "",
            "2+",
            "x^^",
            "^^x",
            "2^^^3",
            "2^ ^3",
            "sin()",
            "sin(x,x)",
            "sin",
            "log()",
            "log(x,2,3)",
            "pow(x)",
            "complex(x)",
            "unknownfn(x)",
            "bogus",
            "(x",
            "x)",
            "x,2",
            "sin(x,)",
            "x**2",
            "x!",
            "x[0]",
        ] {
            let error = Expression::parse(source).unwrap_err();
            assert!(error.contains("byte"), "{source}: {error}");
        }
    }
}
