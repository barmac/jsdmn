//! FEEL runtime values.

use crate::feel::eval::Function;
use crate::number::Number;
use crate::temporal::{self, Date, DateTime, DtDuration, Time, YmDuration, Zone};
use std::cmp::Ordering;
use std::fmt;
use std::rc::Rc;

#[derive(Clone)]
pub enum Value {
    Null,
    Bool(bool),
    Number(Number),
    String(Rc<str>),
    Date(Date),
    Time(Time),
    DateTime(DateTime),
    DtDuration(DtDuration),
    YmDuration(YmDuration),
    List(Rc<Vec<Value>>),
    Context(Rc<Context>),
    Range(Rc<Range>),
    Function(Rc<Function>),
}

#[derive(Clone, Default)]
pub struct Context {
    entries: Vec<(Rc<str>, Value)>,
}

impl Context {
    pub fn new() -> Context {
        Context {
            entries: Vec::new(),
        }
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.entries
            .iter()
            .rev()
            .find(|(k, _)| &**k == key)
            .map(|(_, v)| v)
    }

    pub fn contains(&self, key: &str) -> bool {
        self.entries.iter().any(|(k, _)| &**k == key)
    }

    pub fn put(&mut self, key: impl Into<Rc<str>>, value: Value) {
        let key = key.into();
        if let Some(e) = self.entries.iter_mut().find(|(k, _)| *k == key) {
            e.1 = value;
        } else {
            self.entries.push((key, value));
        }
    }

    pub fn entries(&self) -> &[(Rc<str>, Value)] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CmpOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Clone)]
pub struct Range {
    /// `None` = unbounded (only for ranges created from unary comparisons).
    pub start: Option<Value>,
    pub start_incl: bool,
    pub end: Option<Value>,
    pub end_incl: bool,
    /// Set when the range was created from a unary comparison (`< 10`, `!= 3`, ...).
    pub unary: Option<CmpOp>,
}

impl Range {
    pub fn from_unary(op: CmpOp, v: Value) -> Range {
        match op {
            CmpOp::Lt => Range {
                start: None,
                start_incl: false,
                end: Some(v),
                end_incl: false,
                unary: Some(op),
            },
            CmpOp::Le => Range {
                start: None,
                start_incl: false,
                end: Some(v),
                end_incl: true,
                unary: Some(op),
            },
            CmpOp::Gt => Range {
                start: Some(v),
                start_incl: false,
                end: None,
                end_incl: false,
                unary: Some(op),
            },
            CmpOp::Ge => Range {
                start: Some(v),
                start_incl: true,
                end: None,
                end_incl: false,
                unary: Some(op),
            },
            CmpOp::Eq | CmpOp::Ne => Range {
                start: Some(v.clone()),
                start_incl: true,
                end: Some(v),
                end_incl: true,
                unary: Some(op),
            },
        }
    }

    fn bound(v: &Option<Value>) -> Option<&Value> {
        match v {
            None | Some(Value::Null) => None,
            Some(v) => Some(v),
        }
    }

    pub fn low(&self) -> Option<&Value> {
        Self::bound(&self.start)
    }

    pub fn high(&self) -> Option<&Value> {
        Self::bound(&self.end)
    }

    /// Membership test; `None` when not comparable.
    pub fn contains(&self, x: &Value) -> Option<bool> {
        if let Some(CmpOp::Ne) = self.unary {
            return feel_eq(x, self.start.as_ref().unwrap()).map(|b| !b);
        }
        if let Some(CmpOp::Eq) = self.unary {
            return feel_eq(x, self.start.as_ref().unwrap());
        }
        if matches!(self.start, Some(Value::Null)) || matches!(self.end, Some(Value::Null)) {
            return None;
        }
        if let Some(lo) = self.low() {
            let o = feel_cmp(x, lo)?;
            let ok = if self.start_incl {
                o != Ordering::Less
            } else {
                o == Ordering::Greater
            };
            if !ok {
                return Some(false);
            }
        }
        if let Some(hi) = self.high() {
            let o = feel_cmp(x, hi)?;
            let ok = if self.end_incl {
                o != Ordering::Greater
            } else {
                o == Ordering::Less
            };
            if !ok {
                return Some(false);
            }
        }
        if self.low().is_none() && self.high().is_none() {
            return None;
        }
        Some(true)
    }
}

impl Value {
    pub fn str(s: &str) -> Value {
        Value::String(Rc::from(s))
    }

    pub fn num(n: Number) -> Value {
        Value::Number(n)
    }

    pub fn int(i: i64) -> Value {
        Value::Number(Number::from_i64(i))
    }

    pub fn list(v: Vec<Value>) -> Value {
        Value::List(Rc::new(v))
    }

    pub fn context(c: Context) -> Value {
        Value::Context(Rc::new(c))
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    pub fn as_number(&self) -> Option<&Number> {
        match self {
            Value::Number(n) => Some(n),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&Rc<Vec<Value>>> {
        match self {
            Value::List(l) => Some(l),
            _ => None,
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "Null",
            Value::Bool(_) => "boolean",
            Value::Number(_) => "number",
            Value::String(_) => "string",
            Value::Date(_) => "date",
            Value::Time(_) => "time",
            Value::DateTime(_) => "date and time",
            Value::DtDuration(_) => "days and time duration",
            Value::YmDuration(_) => "years and months duration",
            Value::List(_) => "list",
            Value::Context(_) => "context",
            Value::Range(_) => "range",
            Value::Function(_) => "function",
        }
    }

    /// Items of the value viewed as a list (singleton for non-lists).
    pub fn to_items(&self) -> Vec<Value> {
        match self {
            Value::List(l) => l.as_ref().clone(),
            v => vec![v.clone()],
        }
    }
}

fn date_as_utc_datetime(d: &Date) -> DateTime {
    DateTime {
        date: *d,
        time: Time {
            hour: 0,
            minute: 0,
            second: 0,
            nanos: 0,
            zone: Some(Zone::Offset(0)),
        },
    }
}

fn millis_eq(a: i128, b: i128) -> bool {
    a.div_euclid(1_000_000) == b.div_euclid(1_000_000)
}

fn time_eq(a: &Time, b: &Time) -> bool {
    match (&a.zone, &b.zone) {
        (None, None) => millis_eq(a.nanos_of_day(), b.nanos_of_day()),
        (Some(Zone::Offset(o1)), Some(Zone::Offset(o2))) => millis_eq(
            a.nanos_of_day() - *o1 as i128 * temporal::NANOS_PER_SEC,
            b.nanos_of_day() - *o2 as i128 * temporal::NANOS_PER_SEC,
        ),
        (Some(Zone::Id(z1)), Some(Zone::Id(z2))) => {
            z1 == z2 && millis_eq(a.nanos_of_day(), b.nanos_of_day())
        }
        _ => false,
    }
}

fn datetime_eq(a: &DateTime, b: &DateTime) -> bool {
    match (a.has_zone(), b.has_zone()) {
        (false, false) => millis_eq(a.local_nanos(), b.local_nanos()),
        (true, true) => match (a.instant(), b.instant()) {
            (Some(x), Some(y)) => millis_eq(x, y),
            _ => false,
        },
        _ => false,
    }
}

/// FEEL equality: `Some(bool)` or `None` (null) for incomparable values.
pub fn feel_eq(a: &Value, b: &Value) -> Option<bool> {
    use Value::*;
    match (a, b) {
        (Null, Null) => Some(true),
        (Null, _) | (_, Null) => Some(false),
        (Bool(x), Bool(y)) => Some(x == y),
        (Number(x), Number(y)) => Some(x == y),
        (String(x), String(y)) => Some(x == y),
        (Date(x), Date(y)) => Some(x == y),
        (Date(x), DateTime(y)) => Some(datetime_eq(&date_as_utc_datetime(x), y)),
        (DateTime(x), Date(y)) => Some(datetime_eq(x, &date_as_utc_datetime(y))),
        (Time(x), Time(y)) => Some(time_eq(x, y)),
        (DateTime(x), DateTime(y)) => Some(datetime_eq(x, y)),
        (DtDuration(x), DtDuration(y)) => Some(x == y),
        (YmDuration(x), YmDuration(y)) => Some(x == y),
        (DtDuration(x), YmDuration(y)) | (YmDuration(y), DtDuration(x)) => {
            if x.0 == 0 && y.0 == 0 {
                Some(true)
            } else {
                None
            }
        }
        (List(x), List(y)) => {
            if x.len() != y.len() {
                return Some(false);
            }
            let mut result = Some(true);
            for (p, q) in x.iter().zip(y.iter()) {
                match feel_eq(p, q) {
                    Some(true) => {}
                    Some(false) => return Some(false),
                    None => result = None,
                }
            }
            result
        }
        (Context(x), Context(y)) => {
            if x.len() != y.len() {
                return Some(false);
            }
            let mut result = Some(true);
            for (k, v) in x.entries() {
                match y.get(k) {
                    None => return Some(false),
                    Some(w) => match feel_eq(v, w) {
                        Some(true) => {}
                        Some(false) => return Some(false),
                        None => result = None,
                    },
                }
            }
            result
        }
        (Range(x), Range(y)) => {
            let beq = |p: &Option<Value>, q: &Option<Value>| match (p, q) {
                (None, None) => Some(true),
                (Some(p), Some(q)) => feel_eq(p, q),
                _ => Some(false),
            };
            if x.unary != y.unary {
                return Some(false);
            }
            let s = beq(&x.start, &y.start)?;
            let e = beq(&x.end, &y.end)?;
            let incl_eq = (x.low().is_none() || x.start_incl == y.start_incl)
                && (x.high().is_none() || x.end_incl == y.end_incl);
            Some(s && e && incl_eq)
        }
        (Function(x), Function(y)) => Some(x.same_as(y)),
        _ => None,
    }
}

/// FEEL ordering: `None` if not comparable.
pub fn feel_cmp(a: &Value, b: &Value) -> Option<Ordering> {
    use Value::*;
    match (a, b) {
        (Number(x), Number(y)) => Some(x.cmp(y)),
        (String(x), String(y)) => Some(x.cmp(y)),
        (Date(x), Date(y)) => Some(x.days().cmp(&y.days())),
        (Date(x), DateTime(y)) => temporal::datetime_cmp(&date_as_utc_datetime(x), y),
        (DateTime(x), Date(y)) => temporal::datetime_cmp(x, &date_as_utc_datetime(y)),
        (Time(x), Time(y)) => temporal::time_cmp(x, y),
        (DateTime(x), DateTime(y)) => temporal::datetime_cmp(x, y),
        (DtDuration(x), DtDuration(y)) => Some(x.cmp(y)),
        (YmDuration(x), YmDuration(y)) => Some(x.cmp(y)),
        (Bool(x), Bool(y)) => Some(x.cmp(y)),
        _ => None,
    }
}

/// Representation used by the FEEL `string()` function.
pub fn to_feel_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.to_string(),
        other => format!("{}", other),
    }
}

fn fmt_name(f: &mut fmt::Formatter<'_>, k: &str) -> fmt::Result {
    f.write_str(k)
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => f.write_str("null"),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Number(n) => write!(f, "{}", n),
            Value::String(s) => write!(f, "{}", s),
            Value::Date(d) => write!(f, "{}", d),
            Value::Time(t) => write!(f, "{}", t),
            Value::DateTime(d) => write!(f, "{}", d),
            Value::DtDuration(d) => write!(f, "{}", d),
            Value::YmDuration(d) => write!(f, "{}", d),
            Value::List(l) => {
                f.write_str("[")?;
                for (i, v) in l.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{}", v)?;
                }
                f.write_str("]")
            }
            Value::Context(c) => {
                f.write_str("{")?;
                for (i, (k, v)) in c.entries().iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    fmt_name(f, k)?;
                    write!(f, ": {}", v)?;
                }
                f.write_str("}")
            }
            Value::Range(r) => {
                if let Some(op) = r.unary {
                    let s = match op {
                        CmpOp::Eq => "=",
                        CmpOp::Ne => "!=",
                        CmpOp::Lt => "<",
                        CmpOp::Le => "<=",
                        CmpOp::Gt => ">",
                        CmpOp::Ge => ">=",
                    };
                    let v = r.start.as_ref().or(r.end.as_ref()).unwrap();
                    return write!(f, "{} {}", s, v);
                }
                f.write_str(if r.start_incl { "[" } else { "(" })?;
                match &r.start {
                    Some(v) => write!(f, "{}", v)?,
                    None => f.write_str("null")?,
                }
                f.write_str("..")?;
                match &r.end {
                    Some(v) => write!(f, "{}", v)?,
                    None => f.write_str("null")?,
                }
                f.write_str(if r.end_incl { "]" } else { ")" })
            }
            Value::Function(_) => f.write_str("function"),
        }
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::String(s) => write!(f, "{:?}", &**s),
            Value::Date(d) => write!(f, "date(\"{}\")", d),
            Value::Time(t) => write!(f, "time(\"{}\")", t),
            Value::DateTime(d) => write!(f, "date and time(\"{}\")", d),
            Value::DtDuration(d) => write!(f, "duration(\"{}\")", d),
            Value::YmDuration(d) => write!(f, "duration(\"{}\")", d),
            Value::List(l) => {
                f.write_str("[")?;
                for (i, v) in l.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{:?}", v)?;
                }
                f.write_str("]")
            }
            Value::Context(c) => {
                f.write_str("{")?;
                for (i, (k, v)) in c.entries().iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{:?}: {:?}", &**k, v)?;
                }
                f.write_str("}")
            }
            other => write!(f, "{}", other),
        }
    }
}
