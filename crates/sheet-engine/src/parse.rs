//! Formula text -> syntax tree. Precedence (low to high): comparison, `&`, `+ -`, `* /`, `^`,
//! unary minus, postfix `%`.

use crate::addr::{Addr, parse_addr};
use crate::value::ErrCode;

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Num(f64),
    Str(String),
    Bool(bool),
    Ref(Addr),
    Range(Addr, Addr),
    Neg(Box<Expr>),
    Pos(Box<Expr>),
    Percent(Box<Expr>),
    Bin(Op, Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Concat,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
}

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Num(f64),
    Str(String),
    Ident(String),
    Op(&'static str),
}

const OPS: [&str; 17] = ["<=", ">=", "<>", "+", "-", "*", "/", "^", "&", "=", "<", ">", "%", "(", ")", ",", ":"];

fn tokenize(src: &str) -> Result<Vec<Tok>, ErrCode> {
    let chars: Vec<char> = src.chars().collect();
    let mut toks = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
        } else if c.is_ascii_digit() || (c == '.' && chars.get(i + 1).is_some_and(char::is_ascii_digit)) {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            if i < chars.len() && matches!(chars[i], 'e' | 'E') {
                let mut j = i + 1;
                if matches!(chars.get(j), Some('+' | '-')) {
                    j += 1;
                }
                if chars.get(j).is_some_and(char::is_ascii_digit) {
                    while j < chars.len() && chars[j].is_ascii_digit() {
                        j += 1;
                    }
                    i = j;
                }
            }
            let text: String = chars[start..i].iter().collect();
            toks.push(Tok::Num(text.parse().map_err(|_| ErrCode::Value)?));
        } else if c == '"' {
            let mut s = String::new();
            i += 1;
            loop {
                match chars.get(i) {
                    None => return Err(ErrCode::Value),
                    Some('"') if chars.get(i + 1) == Some(&'"') => {
                        s.push('"');
                        i += 2;
                    }
                    Some('"') => {
                        i += 1;
                        break;
                    }
                    Some(ch) => {
                        s.push(*ch);
                        i += 1;
                    }
                }
            }
            toks.push(Tok::Str(s));
        } else if c.is_ascii_alphabetic() || c == '_' || c == '$' {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || matches!(chars[i], '_' | '.' | '$')) {
                i += 1;
            }
            toks.push(Tok::Ident(chars[start..i].iter().collect()));
        } else {
            let rest: String = chars[i..chars.len().min(i + 2)].iter().collect();
            let op = OPS.iter().find(|o| rest.starts_with(**o)).ok_or(ErrCode::Name)?;
            toks.push(Tok::Op(op));
            i += op.len();
        }
    }
    Ok(toks)
}

struct Parser {
    toks: Vec<Tok>,
    pos: usize,
    depth: usize,
}

const LEVELS: [&[(&str, Op)]; 5] = [
    &[("=", Op::Eq), ("<>", Op::Ne), ("<=", Op::Le), (">=", Op::Ge), ("<", Op::Lt), (">", Op::Gt)],
    &[("&", Op::Concat)],
    &[("+", Op::Add), ("-", Op::Sub)],
    &[("*", Op::Mul), ("/", Op::Div)],
    &[("^", Op::Pow)],
];

/// Formulas nest at most this deep; deeper input is an error, not a stack overflow.
const MAX_DEPTH: usize = 64;

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }

    fn eat_op(&mut self, op: &str) -> bool {
        if matches!(self.peek(), Some(Tok::Op(o)) if *o == op) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn binary(&mut self, level: usize) -> Result<Expr, ErrCode> {
        if level == LEVELS.len() {
            return self.unary();
        }
        let mut left = self.binary(level + 1)?;
        loop {
            let found = match self.peek() {
                Some(Tok::Op(o)) => LEVELS[level].iter().find(|(s, _)| s == o).map(|(_, op)| *op),
                _ => None,
            };
            let Some(op) = found else { return Ok(left) };
            self.pos += 1;
            let right = self.binary(level + 1)?;
            left = Expr::Bin(op, Box::new(left), Box::new(right));
        }
    }

    fn unary(&mut self) -> Result<Expr, ErrCode> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(ErrCode::Value);
        }
        let out = if self.eat_op("-") {
            Expr::Neg(Box::new(self.unary()?))
        } else if self.eat_op("+") {
            Expr::Pos(Box::new(self.unary()?))
        } else {
            let mut e = self.primary()?;
            while self.eat_op("%") {
                e = Expr::Percent(Box::new(e));
            }
            e
        };
        self.depth -= 1;
        Ok(out)
    }

    fn primary(&mut self) -> Result<Expr, ErrCode> {
        let tok = self.peek().cloned().ok_or(ErrCode::Value)?;
        self.pos += 1;
        match tok {
            Tok::Num(n) => Ok(Expr::Num(n)),
            Tok::Str(s) => Ok(Expr::Str(s)),
            Tok::Op("(") => {
                self.depth += 1;
                if self.depth > MAX_DEPTH {
                    return Err(ErrCode::Value);
                }
                let e = self.binary(0)?;
                self.depth -= 1;
                if self.eat_op(")") { Ok(e) } else { Err(ErrCode::Value) }
            }
            Tok::Ident(name) => {
                if self.eat_op("(") {
                    let mut args = Vec::new();
                    if !self.eat_op(")") {
                        loop {
                            args.push(self.binary(0)?);
                            if self.eat_op(",") {
                                continue;
                            }
                            if self.eat_op(")") {
                                break;
                            }
                            return Err(ErrCode::Value);
                        }
                    }
                    return Ok(Expr::Call(name.to_ascii_uppercase(), args));
                }
                match name.to_ascii_uppercase().as_str() {
                    "TRUE" => return Ok(Expr::Bool(true)),
                    "FALSE" => return Ok(Expr::Bool(false)),
                    _ => {}
                }
                let a = parse_addr(&name).ok_or(ErrCode::Name)?;
                if self.eat_op(":") {
                    let Some(Tok::Ident(second)) = self.peek().cloned() else { return Err(ErrCode::Ref) };
                    self.pos += 1;
                    let b = parse_addr(&second).ok_or(ErrCode::Ref)?;
                    return Ok(Expr::Range(a, b));
                }
                Ok(Expr::Ref(a))
            }
            Tok::Op(_) => Err(ErrCode::Value),
        }
    }
}

/// Parses the text after the leading `=`.
pub fn parse(src: &str) -> Result<Expr, ErrCode> {
    let mut p = Parser { toks: tokenize(src)?, pos: 0, depth: 0 };
    let e = p.binary(0)?;
    if p.pos != p.toks.len() {
        return Err(ErrCode::Value);
    }
    Ok(e)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn precedence() {
        let e = parse("1+2*3").unwrap();
        assert_eq!(
            e,
            Expr::Bin(
                Op::Add,
                Box::new(Expr::Num(1.0)),
                Box::new(Expr::Bin(Op::Mul, Box::new(Expr::Num(2.0)), Box::new(Expr::Num(3.0))))
            )
        );
    }

    #[test]
    fn refs_ranges_and_calls() {
        assert_eq!(
            parse("SUM(a1:b2)").unwrap(),
            Expr::Call("SUM".into(), vec![Expr::Range(Addr::new(0, 0), Addr::new(1, 1))])
        );
        assert_eq!(parse("$C$3").unwrap(), Expr::Ref(Addr::new(2, 2)));
        assert_eq!(parse("PI()").unwrap(), Expr::Call("PI".into(), vec![]));
    }

    #[test]
    fn bad_input_is_an_error() {
        for bad in ["", "1+", "(1", "1)", "SUM(", "SUM(1,", "A1:", "\"open", "1 2", "#", "FOO", "A0"] {
            assert!(parse(bad).is_err(), "{bad}");
        }
        let deep = format!("{}1{}", "(".repeat(500), ")".repeat(500));
        assert!(parse(&deep).is_err());
        let minuses = format!("{}1", "-".repeat(5000));
        assert!(parse(&minuses).is_err());
    }
}
