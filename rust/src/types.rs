//! FEEL / DMN types, `instance of` and type coercion.

use crate::feel::ast::UnaryTests;
use crate::feel::eval::{Scope, eval_unary_tests};
use crate::value::{Context, Value};
use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::Rc;

pub enum DmnType {
    Any,
    Null,
    Number,
    String,
    Boolean,
    Date,
    Time,
    DateTime,
    DtDuration,
    YmDuration,
    List(Rc<DmnType>),
    /// Context type; an empty list of entries means "any context".
    Context(Vec<(Rc<str>, Rc<DmnType>)>),
    Range(Rc<DmnType>),
    Function(Vec<Rc<DmnType>>, Rc<DmnType>),
    /// Any function (`function` without signature).
    AnyFunction,
    Named(Rc<NamedType>),
    Unknown(String),
}

pub struct NamedType {
    pub name: String,
    pub base: RefCell<Rc<DmnType>>,
    pub allowed_values: RefCell<Option<Rc<UnaryTests>>>,
    pub type_constraint: RefCell<Option<Rc<UnaryTests>>>,
    /// Whether the base is a collection (allowed values then apply to elements).
    pub collection: Cell<bool>,
    /// Scope used to evaluate constraints.
    pub scope: RefCell<Option<Rc<Scope>>>,
}

impl NamedType {
    pub fn new(name: &str) -> Rc<NamedType> {
        Rc::new(NamedType {
            name: name.to_string(),
            base: RefCell::new(Rc::new(DmnType::Any)),
            allowed_values: RefCell::new(None),
            type_constraint: RefCell::new(None),
            collection: Cell::new(false),
            scope: RefCell::new(None),
        })
    }
}

impl fmt::Debug for DmnType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DmnType::Any => f.write_str("Any"),
            DmnType::Null => f.write_str("Null"),
            DmnType::Number => f.write_str("number"),
            DmnType::String => f.write_str("string"),
            DmnType::Boolean => f.write_str("boolean"),
            DmnType::Date => f.write_str("date"),
            DmnType::Time => f.write_str("time"),
            DmnType::DateTime => f.write_str("date and time"),
            DmnType::DtDuration => f.write_str("days and time duration"),
            DmnType::YmDuration => f.write_str("years and months duration"),
            DmnType::List(t) => write!(f, "list<{:?}>", t),
            DmnType::Context(e) => {
                f.write_str("context<")?;
                for (i, (k, t)) in e.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{}: {:?}", k, t)?;
                }
                f.write_str(">")
            }
            DmnType::Range(t) => write!(f, "range<{:?}>", t),
            DmnType::Function(p, r) => write!(f, "function<{:?}> -> {:?}", p, r),
            DmnType::AnyFunction => f.write_str("function"),
            DmnType::Named(n) => write!(f, "{}", n.name),
            DmnType::Unknown(n) => write!(f, "?{}", n),
        }
    }
}

pub fn builtin_type(name: &str) -> Option<DmnType> {
    let name = name.strip_prefix("feel:").unwrap_or(name);
    Some(match name {
        "number" => DmnType::Number,
        "string" => DmnType::String,
        "boolean" => DmnType::Boolean,
        "date" => DmnType::Date,
        "time" => DmnType::Time,
        "date and time" | "dateTime" => DmnType::DateTime,
        "days and time duration" | "dayTimeDuration" => DmnType::DtDuration,
        "years and months duration" | "yearMonthDuration" => DmnType::YmDuration,
        "Any" => DmnType::Any,
        "Null" => DmnType::Null,
        "context" => DmnType::Context(Vec::new()),
        "list" => DmnType::List(Rc::new(DmnType::Any)),
        "range" => DmnType::Range(Rc::new(DmnType::Any)),
        "function" => DmnType::AnyFunction,
        _ => return None,
    })
}

/// Return type of a function type (after unwrapping named types).
pub fn function_return(t: &DmnType) -> Option<Rc<DmnType>> {
    match &*base_of(t) {
        DmnType::Function(_, r) => Some(r.clone()),
        DmnType::AnyFunction => Some(Rc::new(DmnType::Any)),
        _ => None,
    }
}

/// Unwrap named types to the structural base.
pub fn base_of(t: &DmnType) -> Rc<DmnType> {
    match t {
        DmnType::Named(n) => {
            let b = n.base.borrow().clone();
            base_of(&b)
        }
        _ => Rc::new(clone_shallow(t)),
    }
}

fn clone_shallow(t: &DmnType) -> DmnType {
    match t {
        DmnType::Any => DmnType::Any,
        DmnType::Null => DmnType::Null,
        DmnType::Number => DmnType::Number,
        DmnType::String => DmnType::String,
        DmnType::Boolean => DmnType::Boolean,
        DmnType::Date => DmnType::Date,
        DmnType::Time => DmnType::Time,
        DmnType::DateTime => DmnType::DateTime,
        DmnType::DtDuration => DmnType::DtDuration,
        DmnType::YmDuration => DmnType::YmDuration,
        DmnType::List(t) => DmnType::List(t.clone()),
        DmnType::Context(e) => DmnType::Context(e.clone()),
        DmnType::Range(t) => DmnType::Range(t.clone()),
        DmnType::Function(p, r) => DmnType::Function(p.clone(), r.clone()),
        DmnType::AnyFunction => DmnType::AnyFunction,
        DmnType::Named(n) => DmnType::Named(n.clone()),
        DmnType::Unknown(n) => DmnType::Unknown(n.clone()),
    }
}

fn check_constraints(v: &Value, n: &NamedType) -> bool {
    let scope = n.scope.borrow().clone().unwrap_or_else(Scope::root);
    if let Some(av) = n.allowed_values.borrow().as_ref() {
        if n.collection.get() {
            if let Value::List(items) = v {
                for item in items.iter() {
                    if !matches!(eval_unary_tests(av, item, &scope), Value::Bool(true)) {
                        return false;
                    }
                }
            } else if !matches!(eval_unary_tests(av, v, &scope), Value::Bool(true)) {
                return false;
            }
        } else if !matches!(eval_unary_tests(av, v, &scope), Value::Bool(true)) {
            return false;
        }
    }
    if let Some(tc) = n.type_constraint.borrow().as_ref()
        && !matches!(eval_unary_tests(tc, v, &scope), Value::Bool(true))
    {
        return false;
    }
    true
}

/// `v instance of t`
pub fn is_instance(v: &Value, t: &DmnType) -> bool {
    match t {
        DmnType::Any => !v.is_null(),
        DmnType::Null => v.is_null(),
        DmnType::Unknown(_) => false,
        DmnType::Number => matches!(v, Value::Number(_)),
        DmnType::String => matches!(v, Value::String(_)),
        DmnType::Boolean => matches!(v, Value::Bool(_)),
        DmnType::Date => matches!(v, Value::Date(_)),
        DmnType::Time => matches!(v, Value::Time(_)),
        DmnType::DateTime => matches!(v, Value::DateTime(_)),
        DmnType::DtDuration => matches!(v, Value::DtDuration(_)),
        DmnType::YmDuration => matches!(v, Value::YmDuration(_)),
        DmnType::List(et) => match v {
            Value::List(items) => items.iter().all(|i| i.is_null() || is_instance(i, et)),
            _ => false,
        },
        DmnType::Context(entries) => match v {
            Value::Context(c) => entries.iter().all(|(k, et)| match c.get(k) {
                Some(x) => x.is_null() || is_instance(x, et),
                None => false,
            }),
            _ => false,
        },
        DmnType::Range(et) => match v {
            Value::Range(r) => {
                let ok = |b: Option<&Value>| match b {
                    None => true,
                    Some(x) => is_instance(x, et),
                };
                ok(r.low()) && ok(r.high())
            }
            _ => false,
        },
        DmnType::AnyFunction => matches!(v, Value::Function(_)),
        DmnType::Function(params, ret) => match v {
            Value::Function(f) => f.conforms_to(params, ret),
            _ => false,
        },
        DmnType::Named(n) => {
            let base = n.base.borrow().clone();
            is_instance(v, &base)
        }
    }
}

/// Is type `a` a subtype of (conforms to) type `b`?
pub fn conforms_type(a: &DmnType, b: &DmnType) -> bool {
    let a = base_of(a);
    let b2 = base_of(b);
    match (&*a, &*b2) {
        (_, DmnType::Any) => true,
        (DmnType::Null, _) => true,
        (DmnType::Number, DmnType::Number)
        | (DmnType::String, DmnType::String)
        | (DmnType::Boolean, DmnType::Boolean)
        | (DmnType::Date, DmnType::Date)
        | (DmnType::Time, DmnType::Time)
        | (DmnType::DateTime, DmnType::DateTime)
        | (DmnType::DtDuration, DmnType::DtDuration)
        | (DmnType::YmDuration, DmnType::YmDuration) => true,
        (DmnType::List(x), DmnType::List(y)) => conforms_type(x, y),
        (DmnType::Range(x), DmnType::Range(y)) => conforms_type(x, y),
        (DmnType::Context(x), DmnType::Context(y)) => y
            .iter()
            .all(|(k, ty)| x.iter().any(|(k2, tx)| k2 == k && conforms_type(tx, ty))),
        (DmnType::Function(..), DmnType::AnyFunction)
        | (DmnType::AnyFunction, DmnType::AnyFunction) => true,
        (DmnType::Function(pa, ra), DmnType::Function(pb, rb)) => {
            pa.len() == pb.len()
                && pa.iter().zip(pb.iter()).all(|(x, y)| conforms_type(y, x))
                && conforms_type(ra, rb)
        }
        _ => false,
    }
}

/// Coerce a value to a declared type; returns `Null` if it does not conform.
pub fn coerce(v: &Value, t: &DmnType) -> Value {
    // most values already conform: avoid rebuilding lists and contexts
    if conforms_exactly(v, t) {
        return v.clone();
    }
    coerce_inner(v, t, true)
}

/// Does `v` conform to `t` without any conversion (so that coercion is the identity)?
fn conforms_exactly(v: &Value, t: &DmnType) -> bool {
    if v.is_null() {
        return true;
    }
    match t {
        DmnType::Any | DmnType::Unknown(_) => true,
        DmnType::Null => false,
        DmnType::Named(n) => {
            let base = n.base.borrow().clone();
            conforms_exactly(v, &base) && check_constraints(v, n)
        }
        DmnType::List(et) => match v {
            Value::List(items) => items.iter().all(|i| conforms_exactly(i, et)),
            _ => false,
        },
        DmnType::Context(entries) => match v {
            Value::Context(c) => entries.iter().all(|(k, et)| match c.get(k) {
                Some(x) => conforms_exactly(x, et),
                None => false,
            }),
            _ => false,
        },
        _ => is_instance(v, t),
    }
}

fn coerce_inner(v: &Value, t: &DmnType, unwrap: bool) -> Value {
    let coerce = |v: &Value, t: &DmnType| coerce_inner(v, t, true);
    if v.is_null() {
        return Value::Null;
    }
    match t {
        DmnType::Any | DmnType::Unknown(_) => v.clone(),
        DmnType::Null => Value::Null,
        DmnType::Named(n) => {
            let base = n.base.borrow().clone();
            let r = coerce(v, &base);
            if r.is_null() {
                return Value::Null;
            }
            if !check_constraints(&r, n) {
                return Value::Null;
            }
            r
        }
        DmnType::List(et) => match v {
            Value::List(items) => {
                let mut out = Vec::with_capacity(items.len());
                for i in items.iter() {
                    if i.is_null() {
                        out.push(Value::Null);
                        continue;
                    }
                    let c = coerce(i, et);
                    if c.is_null() {
                        return Value::Null;
                    }
                    out.push(c);
                }
                Value::list(out)
            }
            other => {
                let c = coerce(other, et);
                if c.is_null() {
                    Value::Null
                } else {
                    Value::list(vec![c])
                }
            }
        },
        DmnType::Context(entries) => match v {
            Value::Context(c) => {
                if entries.is_empty() {
                    return v.clone();
                }
                let mut out = Context::new();
                for (k, et) in entries {
                    match c.get(k) {
                        Some(x) => {
                            if x.is_null() {
                                out.put(k.clone(), Value::Null);
                                continue;
                            }
                            let cx = coerce(x, et);
                            if cx.is_null() {
                                return Value::Null;
                            }
                            out.put(k.clone(), cx);
                        }
                        None => return Value::Null,
                    }
                }
                for (k, x) in c.entries() {
                    if !out.contains(k) {
                        out.put(k.clone(), x.clone());
                    }
                }
                Value::context(out)
            }
            Value::List(l) if l.len() == 1 && unwrap => coerce_inner(&l[0], t, false),
            _ => Value::Null,
        },
        DmnType::DateTime => match v {
            Value::DateTime(_) => v.clone(),
            Value::Date(d) => Value::DateTime(crate::temporal::DateTime {
                date: *d,
                time: crate::temporal::Time {
                    hour: 0,
                    minute: 0,
                    second: 0,
                    nanos: 0,
                    zone: Some(crate::temporal::Zone::Offset(0)),
                },
            }),
            Value::List(l) if l.len() == 1 && unwrap => coerce_inner(&l[0], t, false),
            _ => Value::Null,
        },
        _ => {
            if is_instance(v, t) {
                return v.clone();
            }
            if let Value::List(l) = v
                && l.len() == 1
                && unwrap
            {
                return coerce_inner(&l[0], t, false);
            }
            Value::Null
        }
    }
}
