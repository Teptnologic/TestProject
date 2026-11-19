//! Formula language used by content files.
//!
//! A formula is parsed once at load time into an [`Expr`] and evaluated many
//! times. The crate knows nothing about games: variables are an opaque type
//! `V` supplied by the caller, who resolves names at parse time ([`Env`]) and
//! values at evaluation time.
//!
//! Grammar (lowest to highest precedence):
//! `or`, `and`, `not`, comparisons, `+ -`, `* /`, unary `-`, indexing.
//! Primaries: numbers (`12`, `0.5`, `40%`), variables (`bonus_ad`,
//! `target.max_hp`), calls (`min(a, b)`), lists (`[60, 85, 110][rank]`,
//! 1-based, clamped) and named constants (`@q_damage[rank]`).

use std::fmt;

/// A named constant supplied by the data layer.
#[derive(Clone, Debug, PartialEq)]
pub enum Const {
    Num(f64),
    List(Vec<f64>),
}

/// Resolves names while parsing.
pub trait Env<V> {
    fn var(&mut self, name: &str) -> Result<V, String>;
    fn constant(&mut self, name: &str) -> Option<Const>;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
    And,
    Or,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Func {
    Min,
    Max,
    Clamp,
    Floor,
    Ceil,
    Abs,
    If,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Expr<V> {
    Num(f64),
    Var(V),
    Index(Vec<f64>, Box<Expr<V>>),
    Neg(Box<Expr<V>>),
    Not(Box<Expr<V>>),
    Bin(BinOp, Box<Expr<V>>, Box<Expr<V>>),
    Call(Func, Vec<Expr<V>>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct FormulaError {
    pub formula: String,
    pub message: String,
}

impl fmt::Display for FormulaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "formula `{}`: {}", self.formula, self.message)
    }
}

impl std::error::Error for FormulaError {}

impl<V> Expr<V> {
    pub fn num(v: f64) -> Self {
        Expr::Num(v)
    }

    /// The value, if the expression is a plain number.
    pub fn as_const(&self) -> Option<f64> {
        match self {
            Expr::Num(v) => Some(*v),
            _ => None,
        }
    }

    pub fn eval(&self, vars: &dyn Fn(&V) -> f64) -> f64 {
        match self {
            Expr::Num(v) => *v,
            Expr::Var(v) => vars(v),
            Expr::Index(list, idx) => {
                // 1-based and clamped, so `[..][rank]` is safe for any rank.
                let i = idx.eval(vars).round().max(1.0) as usize;
                list[(i - 1).min(list.len() - 1)]
            }
            Expr::Neg(e) => -e.eval(vars),
            Expr::Not(e) => bool_f(e.eval(vars) == 0.0),
            Expr::Bin(op, a, b) => {
                let x = a.eval(vars);
                match op {
                    BinOp::And => return bool_f(x != 0.0 && b.eval(vars) != 0.0),
                    BinOp::Or => return bool_f(x != 0.0 || b.eval(vars) != 0.0),
                    _ => {}
                }
                let y = b.eval(vars);
                match op {
                    BinOp::Add => x + y,
                    BinOp::Sub => x - y,
                    BinOp::Mul => x * y,
                    BinOp::Div => {
                        if y == 0.0 {
                            0.0
                        } else {
                            x / y
                        }
                    }
                    BinOp::Lt => bool_f(x < y),
                    BinOp::Le => bool_f(x <= y),
                    BinOp::Gt => bool_f(x > y),
                    BinOp::Ge => bool_f(x >= y),
                    BinOp::Eq => bool_f(x == y),
                    BinOp::Ne => bool_f(x != y),
                    BinOp::And | BinOp::Or => unreachable!(),
                }
            }
            Expr::Call(f, args) => {
                let a = |i: usize| args[i].eval(vars);
                match f {
                    Func::Min => a(0).min(a(1)),
                    Func::Max => a(0).max(a(1)),
                    Func::Clamp => a(0).max(a(1)).min(a(2)),
                    Func::Floor => a(0).floor(),
                    Func::Ceil => a(0).ceil(),
                    Func::Abs => a(0).abs(),
                    Func::If => {
                        if a(0) != 0.0 {
                            a(1)
                        } else {
                            a(2)
                        }
                    }
                }
            }
        }
    }
}

fn bool_f(b: bool) -> f64 {
    if b {
        1.0
    } else {
        0.0
    }
}

/// Parses `src`, resolving every variable and constant through `env`.
pub fn parse<V>(src: &str, env: &mut dyn Env<V>) -> Result<Expr<V>, FormulaError> {
    let err = |message: String| FormulaError {
        formula: src.to_string(),
        message,
    };
    let tokens = tokenize(src).map_err(err)?;
    let mut p = Parser {
        tokens,
        pos: 0,
        env,
    };
    let expr = p.expr().map_err(err)?;
    match p.peek() {
        None => Ok(expr),
        Some(t) => Err(err(format!("unexpected `{t}`"))),
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Num(f64),
    Ident(String),
    Const(String),
    Sym(&'static str),
}

impl fmt::Display for Tok {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Tok::Num(v) => write!(f, "{v}"),
            Tok::Ident(s) => write!(f, "{s}"),
            Tok::Const(s) => write!(f, "@{s}"),
            Tok::Sym(s) => write!(f, "{s}"),
        }
    }
}

const SYMBOLS: [&str; 16] = [
    "<=", ">=", "==", "!=", "<", ">", "+", "-", "*", "/", "(", ")", "[", "]", ",", "%",
];

fn tokenize(src: &str) -> Result<Vec<Tok>, String> {
    let b = src.as_bytes();
    let mut i = 0;
    let mut out = Vec::new();
    let ident_end = |mut j: usize| {
        while j < b.len() && (b[j].is_ascii_alphanumeric() || b[j] == b'_' || b[j] == b'.') {
            j += 1;
        }
        j
    };
    while i < b.len() {
        let c = b[i];
        if c.is_ascii_whitespace() {
            i += 1;
        } else if c.is_ascii_digit() || (c == b'.' && b.get(i + 1).is_some_and(u8::is_ascii_digit))
        {
            let mut j = i;
            while j < b.len() && (b[j].is_ascii_digit() || b[j] == b'.') {
                j += 1;
            }
            let v: f64 = src[i..j]
                .parse()
                .map_err(|_| format!("bad number `{}`", &src[i..j]))?;
            out.push(Tok::Num(v));
            i = j;
        } else if c.is_ascii_alphabetic() || c == b'_' {
            let j = ident_end(i);
            out.push(Tok::Ident(src[i..j].to_string()));
            i = j;
        } else if c == b'@' {
            let j = ident_end(i + 1);
            if j == i + 1 {
                return Err("`@` must be followed by a constant name".into());
            }
            out.push(Tok::Const(src[i + 1..j].to_string()));
            i = j;
        } else if let Some(s) = SYMBOLS.iter().find(|s| src[i..].starts_with(**s)) {
            out.push(Tok::Sym(s));
            i += s.len();
        } else {
            return Err(format!("unexpected character `{}`", &src[i..i + 1]));
        }
    }
    Ok(out)
}

struct Parser<'a, V> {
    tokens: Vec<Tok>,
    pos: usize,
    env: &'a mut dyn Env<V>,
}

type PResult<V> = Result<Expr<V>, String>;

impl<V> Parser<'_, V> {
    fn peek(&self) -> Option<&Tok> {
        self.tokens.get(self.pos)
    }

    fn eat_sym(&mut self, s: &str) -> bool {
        if matches!(self.peek(), Some(Tok::Sym(t)) if *t == s) {
            self.pos += 1;
            return true;
        }
        false
    }

    fn eat_word(&mut self, w: &str) -> bool {
        if matches!(self.peek(), Some(Tok::Ident(t)) if t == w) {
            self.pos += 1;
            return true;
        }
        false
    }

    fn expect(&mut self, s: &str) -> Result<(), String> {
        if self.eat_sym(s) {
            Ok(())
        } else {
            Err(format!("expected `{s}`"))
        }
    }

    fn bin(op: BinOp, a: Expr<V>, b: Expr<V>) -> Expr<V> {
        Expr::Bin(op, Box::new(a), Box::new(b))
    }

    fn expr(&mut self) -> PResult<V> {
        let mut lhs = self.and()?;
        while self.eat_word("or") {
            lhs = Self::bin(BinOp::Or, lhs, self.and()?);
        }
        Ok(lhs)
    }

    fn and(&mut self) -> PResult<V> {
        let mut lhs = self.not()?;
        while self.eat_word("and") {
            lhs = Self::bin(BinOp::And, lhs, self.not()?);
        }
        Ok(lhs)
    }

    fn not(&mut self) -> PResult<V> {
        if self.eat_word("not") {
            return Ok(Expr::Not(Box::new(self.not()?)));
        }
        self.cmp()
    }

    fn cmp(&mut self) -> PResult<V> {
        let lhs = self.add()?;
        let ops = [
            ("<=", BinOp::Le),
            (">=", BinOp::Ge),
            ("==", BinOp::Eq),
            ("!=", BinOp::Ne),
            ("<", BinOp::Lt),
            (">", BinOp::Gt),
        ];
        for (s, op) in ops {
            if self.eat_sym(s) {
                return Ok(Self::bin(op, lhs, self.add()?));
            }
        }
        Ok(lhs)
    }

    fn add(&mut self) -> PResult<V> {
        let mut lhs = self.mul()?;
        loop {
            if self.eat_sym("+") {
                lhs = Self::bin(BinOp::Add, lhs, self.mul()?);
            } else if self.eat_sym("-") {
                lhs = Self::bin(BinOp::Sub, lhs, self.mul()?);
            } else {
                return Ok(lhs);
            }
        }
    }

    fn mul(&mut self) -> PResult<V> {
        let mut lhs = self.unary()?;
        loop {
            if self.eat_sym("*") {
                lhs = Self::bin(BinOp::Mul, lhs, self.unary()?);
            } else if self.eat_sym("/") {
                lhs = Self::bin(BinOp::Div, lhs, self.unary()?);
            } else {
                return Ok(lhs);
            }
        }
    }

    fn unary(&mut self) -> PResult<V> {
        if self.eat_sym("-") {
            return Ok(match self.unary()? {
                Expr::Num(v) => Expr::Num(-v),
                e => Expr::Neg(Box::new(e)),
            });
        }
        self.primary()
    }

    fn primary(&mut self) -> PResult<V> {
        let tok = self.peek().cloned().ok_or("unexpected end of formula")?;
        self.pos += 1;
        match tok {
            Tok::Num(v) => {
                if self.eat_sym("%") {
                    Ok(Expr::Num(v / 100.0))
                } else {
                    Ok(Expr::Num(v))
                }
            }
            Tok::Sym("(") => {
                let e = self.expr()?;
                self.expect(")")?;
                Ok(e)
            }
            Tok::Sym("[") => {
                let mut list = Vec::new();
                loop {
                    match self.unary()? {
                        Expr::Num(v) => list.push(v),
                        _ => return Err("lists may only contain numbers".into()),
                    }
                    if !self.eat_sym(",") {
                        break;
                    }
                }
                self.expect("]")?;
                self.index(list)
            }
            Tok::Const(name) => match self.env.constant(&name) {
                Some(Const::Num(v)) => Ok(Expr::Num(v)),
                Some(Const::List(list)) if list.is_empty() => {
                    Err(format!("constant `@{name}` is an empty list"))
                }
                Some(Const::List(list)) => self.index(list),
                None => Err(format!("unknown constant `@{name}`")),
            },
            Tok::Ident(name) => {
                if self.eat_sym("(") {
                    return self.call(&name);
                }
                Ok(Expr::Var(self.env.var(&name)?))
            }
            t => Err(format!("unexpected `{t}`")),
        }
    }

    fn index(&mut self, list: Vec<f64>) -> PResult<V> {
        if !self.eat_sym("[") {
            return Err("a list must be indexed, e.g. `[1, 2, 3][rank]`".into());
        }
        let idx = self.expr()?;
        self.expect("]")?;
        Ok(Expr::Index(list, Box::new(idx)))
    }

    fn call(&mut self, name: &str) -> PResult<V> {
        let (func, arity) = match name {
            "min" => (Func::Min, 2),
            "max" => (Func::Max, 2),
            "clamp" => (Func::Clamp, 3),
            "floor" => (Func::Floor, 1),
            "ceil" => (Func::Ceil, 1),
            "abs" => (Func::Abs, 1),
            "if" => (Func::If, 3),
            _ => return Err(format!("unknown function `{name}`")),
        };
        let mut args = Vec::new();
        if !self.eat_sym(")") {
            loop {
                args.push(self.expr()?);
                if !self.eat_sym(",") {
                    break;
                }
            }
            self.expect(")")?;
        }
        if args.len() != arity {
            return Err(format!(
                "`{name}` takes {arity} argument(s), got {}",
                args.len()
            ));
        }
        Ok(Expr::Call(func, args))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestEnv;

    impl Env<String> for TestEnv {
        fn var(&mut self, name: &str) -> Result<String, String> {
            if name == "nope" {
                return Err("unknown variable `nope`".into());
            }
            Ok(name.to_string())
        }

        fn constant(&mut self, name: &str) -> Option<Const> {
            match name {
                "dmg" => Some(Const::List(vec![10.0, 20.0, 30.0])),
                "ratio" => Some(Const::Num(0.5)),
                _ => None,
            }
        }
    }

    fn eval(src: &str) -> f64 {
        let vars = |v: &String| match v.as_str() {
            "rank" => 2.0,
            "ad" => 100.0,
            "target.max_hp" => 2000.0,
            _ => 0.0,
        };
        parse(src, &mut TestEnv).unwrap().eval(&vars)
    }

    #[test]
    fn arithmetic_and_precedence() {
        assert_eq!(eval("1 + 2 * 3"), 7.0);
        assert_eq!(eval("(1 + 2) * 3"), 9.0);
        assert_eq!(eval("-2 * -3"), 6.0);
        assert_eq!(eval("10 / 4"), 2.5);
        assert_eq!(eval("1 / 0"), 0.0);
    }

    #[test]
    fn percent_lists_and_constants() {
        assert_eq!(eval("4% * target.max_hp"), 80.0);
        assert_eq!(eval("[60, 85, 110][rank] + 0.5 * ad"), 135.0);
        assert_eq!(eval("@dmg[rank] + @ratio * ad"), 70.0);
        assert_eq!(eval("[1, 2, 3][99]"), 3.0);
        assert_eq!(eval("[1, 2, 3][0]"), 1.0);
    }

    #[test]
    fn logic_and_functions() {
        assert_eq!(eval("ad > 50 and rank == 2"), 1.0);
        assert_eq!(eval("not (ad > 50) or rank != 2"), 0.0);
        assert_eq!(eval("clamp(ad, 0, 60)"), 60.0);
        assert_eq!(eval("if(rank >= 2, max(1, 2), 0)"), 2.0);
    }

    #[test]
    fn errors_name_the_problem() {
        let msg = |s: &str| parse::<String>(s, &mut TestEnv).unwrap_err().message;
        assert!(msg("1 +").contains("end of formula"));
        assert!(msg("nope + 1").contains("nope"));
        assert!(msg("@missing").contains("missing"));
        assert!(msg("[1, 2]").contains("indexed"));
        assert!(msg("min(1)").contains("argument"));
        assert!(msg("1 2").contains("unexpected"));
    }
}
