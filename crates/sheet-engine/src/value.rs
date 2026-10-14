//! What a cell can hold once calculated.

use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrCode {
    DivZero,
    Value,
    Name,
    Ref,
    Cycle,
    Num,
}

impl ErrCode {
    pub fn text(self) -> &'static str {
        match self {
            ErrCode::DivZero => "#DIV/0!",
            ErrCode::Value => "#VALUE!",
            ErrCode::Name => "#NAME?",
            ErrCode::Ref => "#REF!",
            ErrCode::Cycle => "#CYCLE!",
            ErrCode::Num => "#NUM!",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Empty,
    Num(f64),
    Text(String),
    Bool(bool),
    Error(ErrCode),
}

impl Value {
    /// Number for arithmetic: empty is 0, booleans are 0/1, text that looks numeric converts.
    pub(crate) fn to_num(&self) -> Result<f64, ErrCode> {
        match self {
            Value::Empty => Ok(0.0),
            Value::Num(n) => Ok(*n),
            Value::Bool(b) => Ok(f64::from(u8::from(*b))),
            Value::Text(t) => t.trim().parse::<f64>().map_err(|_| ErrCode::Value),
            Value::Error(e) => Err(*e),
        }
    }

    pub(crate) fn truthy(&self) -> Result<bool, ErrCode> {
        match self {
            Value::Bool(b) => Ok(*b),
            Value::Error(e) => Err(*e),
            other => other.to_num().map(|n| n != 0.0),
        }
    }

    pub fn is_error(&self) -> bool {
        matches!(self, Value::Error(_))
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Empty => Ok(()),
            Value::Num(n) => {
                if n.fract() == 0.0 && n.abs() < 1e15 {
                    write!(f, "{}", *n as i64)
                } else {
                    // up to 10 decimals, trailing zeros trimmed
                    let s = format!("{n:.10}");
                    write!(f, "{}", s.trim_end_matches('0').trim_end_matches('.'))
                }
            }
            Value::Text(t) => write!(f, "{t}"),
            Value::Bool(b) => write!(f, "{}", if *b { "TRUE" } else { "FALSE" }),
            Value::Error(e) => write!(f, "{}", e.text()),
        }
    }
}
