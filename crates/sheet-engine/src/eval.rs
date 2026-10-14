//! The sheet: raw cell text in, calculated values out, computed lazily and cached.

use std::cell::RefCell;
use std::collections::HashMap;

use crate::MAX_RANGE_CELLS;
use crate::addr::Addr;
use crate::parse::{Expr, Op, parse};
use crate::value::{ErrCode, Value};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mark {
    Visiting,
}

#[derive(Default)]
pub struct Sheet {
    raw: HashMap<Addr, String>,
    cache: RefCell<HashMap<Addr, Value>>,
    visiting: RefCell<HashMap<Addr, Mark>>,
}

impl Clone for Sheet {
    fn clone(&self) -> Self {
        Sheet { raw: self.raw.clone(), cache: RefCell::default(), visiting: RefCell::default() }
    }
}

/// A value inside a function call: one cell or a block of them.
enum Arg {
    One(Value),
    Many(Vec<Value>),
}

impl Sheet {
    pub fn new() -> Sheet {
        Sheet::default()
    }

    pub fn from_cells(cells: HashMap<Addr, String>) -> Sheet {
        Sheet { raw: cells, ..Sheet::default() }
    }

    /// Sets (or clears, when `text` is empty) a cell's raw text.
    pub fn set(&mut self, at: Addr, text: &str) {
        if text.is_empty() {
            self.raw.remove(&at);
        } else {
            self.raw.insert(at, text.to_string());
        }
        self.cache.get_mut().clear();
    }

    pub fn raw(&self, at: Addr) -> &str {
        self.raw.get(&at).map_or("", String::as_str)
    }

    pub fn cells(&self) -> &HashMap<Addr, String> {
        &self.raw
    }

    pub fn clear(&mut self) {
        self.raw.clear();
        self.cache.get_mut().clear();
    }

    /// Bottom-right corner of the used area as (columns, rows), counting from 1.
    pub fn extent(&self) -> (u32, u32) {
        self.raw.keys().fold((0, 0), |(c, r), a| (c.max(a.col + 1), r.max(a.row + 1)))
    }

    pub fn value(&self, at: Addr) -> Value {
        if let Some(v) = self.cache.borrow().get(&at) {
            return v.clone();
        }
        if self.visiting.borrow().contains_key(&at) {
            return Value::Error(ErrCode::Cycle);
        }
        self.visiting.borrow_mut().insert(at, Mark::Visiting);
        let v = self.compute(at);
        self.visiting.borrow_mut().remove(&at);
        self.cache.borrow_mut().insert(at, v.clone());
        v
    }

    fn compute(&self, at: Addr) -> Value {
        let raw = self.raw(at);
        if raw.is_empty() {
            return Value::Empty;
        }
        if let Some(f) = raw.strip_prefix('=') {
            return match parse(f).and_then(|e| self.eval(&e)) {
                Ok(Arg::One(v)) => v,
                Ok(Arg::Many(_)) => Value::Error(ErrCode::Value),
                Err(code) => Value::Error(code),
            };
        }
        if let Some(n) = raw.trim().parse::<f64>().ok().filter(|n| n.is_finite()) {
            return Value::Num(n);
        }
        match raw.to_ascii_uppercase().as_str() {
            "TRUE" => Value::Bool(true),
            "FALSE" => Value::Bool(false),
            _ => Value::Text(raw.to_string()),
        }
    }

    fn scalar(&self, e: &Expr) -> Result<Value, ErrCode> {
        match self.eval(e)? {
            Arg::One(v) => match v {
                Value::Error(c) => Err(c),
                v => Ok(v),
            },
            Arg::Many(_) => Err(ErrCode::Value),
        }
    }

    fn num(&self, e: &Expr) -> Result<f64, ErrCode> {
        self.scalar(e)?.to_num()
    }

    fn eval(&self, e: &Expr) -> Result<Arg, ErrCode> {
        Ok(match e {
            Expr::Num(n) => Arg::One(Value::Num(*n)),
            Expr::Str(s) => Arg::One(Value::Text(s.clone())),
            Expr::Bool(b) => Arg::One(Value::Bool(*b)),
            Expr::Ref(a) => Arg::One(self.value(*a)),
            Expr::Range(a, b) => {
                let (c1, c2) = (a.col.min(b.col), a.col.max(b.col));
                let (r1, r2) = (a.row.min(b.row), a.row.max(b.row));
                let cells = (u64::from(c2 - c1) + 1) * (u64::from(r2 - r1) + 1);
                if cells > MAX_RANGE_CELLS as u64 {
                    return Err(ErrCode::Ref);
                }
                let mut out = Vec::with_capacity(cells as usize);
                for r in r1..=r2 {
                    for c in c1..=c2 {
                        out.push(self.value(Addr::new(c, r)));
                    }
                }
                Arg::Many(out)
            }
            Expr::Neg(x) => Arg::One(Value::Num(-self.num(x)?)),
            Expr::Pos(x) => Arg::One(Value::Num(self.num(x)?)),
            Expr::Percent(x) => Arg::One(Value::Num(self.num(x)? / 100.0)),
            Expr::Bin(op, a, b) => Arg::One(self.binary(*op, a, b)?),
            Expr::Call(name, args) => Arg::One(self.call(name, args)?),
        })
    }

    fn binary(&self, op: Op, a: &Expr, b: &Expr) -> Result<Value, ErrCode> {
        let (x, y) = (self.scalar(a)?, self.scalar(b)?);
        Ok(match op {
            Op::Add => Value::Num(x.to_num()? + y.to_num()?),
            Op::Sub => Value::Num(x.to_num()? - y.to_num()?),
            Op::Mul => Value::Num(x.to_num()? * y.to_num()?),
            Op::Div => {
                let d = y.to_num()?;
                if d == 0.0 {
                    return Err(ErrCode::DivZero);
                }
                Value::Num(x.to_num()? / d)
            }
            Op::Pow => {
                let r = x.to_num()?.powf(y.to_num()?);
                if r.is_finite() { Value::Num(r) } else { return Err(ErrCode::Num) }
            }
            Op::Concat => Value::Text(format!("{x}{y}")),
            Op::Eq | Op::Ne | Op::Lt | Op::Gt | Op::Le | Op::Ge => {
                let ord = compare(&x, &y);
                Value::Bool(match op {
                    Op::Eq => ord == Some(std::cmp::Ordering::Equal),
                    Op::Ne => ord != Some(std::cmp::Ordering::Equal),
                    Op::Lt => ord == Some(std::cmp::Ordering::Less),
                    Op::Gt => ord == Some(std::cmp::Ordering::Greater),
                    Op::Le => matches!(ord, Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)),
                    _ => matches!(ord, Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)),
                })
            }
        })
    }

    /// Flatten arguments to a list of values (ranges expand). Errors inside propagate.
    fn flat(&self, args: &[Expr]) -> Result<Vec<Value>, ErrCode> {
        let mut out = Vec::new();
        for a in args {
            match self.eval(a)? {
                Arg::One(v) => out.push(v),
                Arg::Many(vs) => out.extend(vs),
            }
        }
        if let Some(Value::Error(c)) = out.iter().find(|v| v.is_error()) {
            return Err(*c);
        }
        Ok(out)
    }

    fn call(&self, name: &str, args: &[Expr]) -> Result<Value, ErrCode> {
        let nums = |s: &Self| -> Result<Vec<f64>, ErrCode> {
            Ok(s.flat(args)?.into_iter().filter_map(|v| if let Value::Num(n) = v { Some(n) } else { None }).collect())
        };
        let need = |n: usize| if args.len() == n { Ok(()) } else { Err(ErrCode::Value) };
        Ok(match name {
            "SUM" => Value::Num(nums(self)?.iter().sum()),
            "AVERAGE" => {
                let n = nums(self)?;
                if n.is_empty() {
                    return Err(ErrCode::DivZero);
                }
                Value::Num(n.iter().sum::<f64>() / n.len() as f64)
            }
            "MIN" => Value::Num(
                nums(self)?.into_iter().fold(None, |m: Option<f64>, x| Some(m.map_or(x, |m| m.min(x)))).unwrap_or(0.0),
            ),
            "MAX" => Value::Num(
                nums(self)?.into_iter().fold(None, |m: Option<f64>, x| Some(m.map_or(x, |m| m.max(x)))).unwrap_or(0.0),
            ),
            "COUNT" => Value::Num(nums(self)?.len() as f64),
            "COUNTA" => Value::Num(self.flat(args)?.iter().filter(|v| **v != Value::Empty).count() as f64),
            "ABS" => {
                need(1)?;
                Value::Num(self.num(&args[0])?.abs())
            }
            "ROUND" => {
                if args.is_empty() || args.len() > 2 {
                    return Err(ErrCode::Value);
                }
                let digits = if args.len() == 2 { self.num(&args[1])? } else { 0.0 }.clamp(-15.0, 15.0);
                let f = 10f64.powf(digits.trunc());
                Value::Num((self.num(&args[0])? * f).round() / f)
            }
            "INT" => {
                need(1)?;
                Value::Num(self.num(&args[0])?.floor())
            }
            "SQRT" => {
                need(1)?;
                let n = self.num(&args[0])?;
                if n < 0.0 { return Err(ErrCode::Num) } else { Value::Num(n.sqrt()) }
            }
            "POWER" => {
                need(2)?;
                let r = self.num(&args[0])?.powf(self.num(&args[1])?);
                if r.is_finite() { Value::Num(r) } else { return Err(ErrCode::Num) }
            }
            "MOD" => {
                need(2)?;
                let d = self.num(&args[1])?;
                if d == 0.0 { return Err(ErrCode::DivZero) } else { Value::Num(self.num(&args[0])?.rem_euclid(d)) }
            }
            "IF" => {
                if args.len() < 2 || args.len() > 3 {
                    return Err(ErrCode::Value);
                }
                let cond = self.scalar(&args[0])?.truthy()?;
                match (cond, args.get(2)) {
                    (true, _) => self.scalar(&args[1])?,
                    (false, Some(e)) => self.scalar(e)?,
                    (false, None) => Value::Bool(false),
                }
            }
            "AND" => Value::Bool(
                self.flat(args)?.iter().map(Value::truthy).collect::<Result<Vec<_>, _>>()?.into_iter().all(|b| b),
            ),
            "OR" => Value::Bool(
                self.flat(args)?.iter().map(Value::truthy).collect::<Result<Vec<_>, _>>()?.into_iter().any(|b| b),
            ),
            "NOT" => {
                need(1)?;
                Value::Bool(!self.scalar(&args[0])?.truthy()?)
            }
            "CONCAT" => Value::Text(self.flat(args)?.iter().map(ToString::to_string).collect()),
            "LEN" => {
                need(1)?;
                Value::Num(self.scalar(&args[0])?.to_string().chars().count() as f64)
            }
            "UPPER" => {
                need(1)?;
                Value::Text(self.scalar(&args[0])?.to_string().to_uppercase())
            }
            "LOWER" => {
                need(1)?;
                Value::Text(self.scalar(&args[0])?.to_string().to_lowercase())
            }
            "PI" => {
                need(0)?;
                Value::Num(std::f64::consts::PI)
            }
            _ => return Err(ErrCode::Name),
        })
    }
}

/// Numbers compare as numbers, text case-insensitively; mixed kinds order numbers before text.
fn compare(a: &Value, b: &Value) -> Option<std::cmp::Ordering> {
    use Value::*;
    match (a, b) {
        (Num(x), Num(y)) => x.partial_cmp(y),
        (Text(x), Text(y)) => Some(x.to_lowercase().cmp(&y.to_lowercase())),
        (Bool(x), Bool(y)) => Some(x.cmp(y)),
        (Empty, Empty) => Some(std::cmp::Ordering::Equal),
        (Empty, other) => compare(&Num(0.0), other).or_else(|| compare(&Text(String::new()), other)),
        (other, Empty) => compare(other, &Num(0.0)).or_else(|| compare(other, &Text(String::new()))),
        (Num(_), Text(_)) => Some(std::cmp::Ordering::Less),
        (Text(_), Num(_)) => Some(std::cmp::Ordering::Greater),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::addr::parse_addr;

    fn sheet(cells: &[(&str, &str)]) -> Sheet {
        let mut s = Sheet::new();
        for (a, t) in cells {
            s.set(parse_addr(a).unwrap(), t);
        }
        s
    }

    fn v(s: &Sheet, a: &str) -> Value {
        s.value(parse_addr(a).unwrap())
    }

    #[test]
    fn arithmetic() {
        let s = sheet(&[
            ("A1", "=1+2*3"),
            ("A2", "=(1+2)*3"),
            ("A3", "=2^3^2"),
            ("A4", "=-2+5"),
            ("A5", "=50%"),
            ("A6", "=10/4"),
        ]);
        let got: Vec<_> = ["A1", "A2", "A3", "A4", "A5", "A6"].iter().map(|a| v(&s, a)).collect();
        assert_eq!(
            got,
            vec![Value::Num(7.0), Value::Num(9.0), Value::Num(64.0), Value::Num(3.0), Value::Num(0.5), Value::Num(2.5)]
        );
    }

    #[test]
    fn exponent_is_left_associative_like_excel() {
        // like Excel, 2^3^2 is (2^3)^2 = 64: the operator is left-associative
        let s = sheet(&[("A1", "=2^3^2")]);
        assert_eq!(v(&s, "A1"), Value::Num(64.0));
    }

    #[test]
    fn references_ranges_functions() {
        let s = sheet(&[
            ("A1", "10"),
            ("A2", "20"),
            ("A3", "30"),
            ("B1", "=SUM(A1:A3)"),
            ("B2", "=AVERAGE(A1:A3)"),
            ("B3", "=MAX(A1:A3)-MIN(A1:A3)"),
            ("B4", r#"=IF(B1>50,"big","small")"#),
            ("B5", "=COUNT(A1:A3,5)"),
        ]);
        assert_eq!(v(&s, "B1"), Value::Num(60.0));
        assert_eq!(v(&s, "B2"), Value::Num(20.0));
        assert_eq!(v(&s, "B3"), Value::Num(20.0));
        assert_eq!(v(&s, "B4"), Value::Text("big".into()));
        assert_eq!(v(&s, "B5"), Value::Num(4.0));
    }

    #[test]
    fn editing_recalculates() {
        let mut s = sheet(&[("A1", "1"), ("B1", "=A1*2")]);
        assert_eq!(v(&s, "B1"), Value::Num(2.0));
        s.set(parse_addr("A1").unwrap(), "5");
        assert_eq!(v(&s, "B1"), Value::Num(10.0));
    }

    #[test]
    fn errors_are_values() {
        let s = sheet(&[
            ("A1", "=1/0"),
            ("A2", "=A1+1"),
            ("A3", "=NOPE(1)"),
            ("B1", "=B2"),
            ("B2", "=B1"),
            ("C1", "=1+"),
            ("D1", "=SUM("),
            ("E1", "=SQRT(-1)"),
            ("F1", r#"="a"+1"#),
            ("G1", "=A1:A2"),
        ]);
        assert_eq!(v(&s, "A1"), Value::Error(ErrCode::DivZero));
        assert_eq!(v(&s, "A2"), Value::Error(ErrCode::DivZero));
        assert_eq!(v(&s, "A3"), Value::Error(ErrCode::Name));
        assert_eq!(v(&s, "B1"), Value::Error(ErrCode::Cycle));
        assert_eq!(v(&s, "B2"), Value::Error(ErrCode::Cycle));
        assert_eq!(v(&s, "C1"), Value::Error(ErrCode::Value));
        assert_eq!(v(&s, "D1"), Value::Error(ErrCode::Value));
        assert_eq!(v(&s, "E1"), Value::Error(ErrCode::Num));
        assert_eq!(v(&s, "F1"), Value::Error(ErrCode::Value));
        assert_eq!(v(&s, "G1"), Value::Error(ErrCode::Value));
    }

    #[test]
    fn text_and_booleans() {
        let s = sheet(&[
            ("A1", "hello"),
            ("B1", r#"=A1&" "&"world""#),
            ("C1", "=UPPER(A1)"),
            ("D1", "=1<2"),
            ("E1", "true"),
            ("F1", "=LEN(B1)"),
        ]);
        assert_eq!(v(&s, "B1"), Value::Text("hello world".into()));
        assert_eq!(v(&s, "C1"), Value::Text("HELLO".into()));
        assert_eq!(v(&s, "D1"), Value::Bool(true));
        assert_eq!(v(&s, "E1"), Value::Bool(true));
        assert_eq!(v(&s, "F1"), Value::Num(11.0));
    }

    #[test]
    fn huge_ranges_are_refused_not_computed() {
        let s = sheet(&[("A1", "=SUM(A2:ZZ999999)")]);
        assert_eq!(v(&s, "A1"), Value::Error(ErrCode::Ref));
    }

    #[test]
    fn long_dependency_chains_do_not_overflow_the_stack() {
        let mut s = Sheet::new();
        s.set(Addr::new(0, 0), "1");
        for r in 1..400 {
            s.set(Addr::new(0, r), &format!("=A{r}+1"));
        }
        assert_eq!(s.value(Addr::new(0, 399)), Value::Num(400.0));
    }

    #[test]
    fn display_formats_numbers() {
        assert_eq!(Value::Num(16.5).to_string(), "16.5");
        assert_eq!(Value::Num(3.0).to_string(), "3");
        assert_eq!(Value::Num(0.1 + 0.2).to_string(), "0.3");
        assert_eq!(Value::Error(ErrCode::Ref).to_string(), "#REF!");
    }
}
