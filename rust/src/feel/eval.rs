//! FEEL evaluator.

use super::ast::*;
use super::builtins;
use crate::number::Number;
use crate::temporal::{
    Date, DateTime, DtDuration, NANOS_PER_DAY, NANOS_PER_SEC, Time, YmDuration, Zone,
};
use crate::types::{self, DmnType};
use crate::value::{CmpOp, Context, Range, Value, feel_cmp, feel_eq};
use std::cell::RefCell;
use std::cmp::Ordering;
use std::rc::Rc;

enum Frame {
    /// Mutable set of variables.
    Vars(RefCell<Vec<(Rc<str>, Value)>>),
    /// A single variable (iteration variables, `?`, `partial`).
    One(Rc<str>, Value),
    /// A filter item: the entries of a context item shadow `item`.
    Item(Value),
}

/// Lexical scope (linked list of frames).
pub struct Scope {
    frame: Frame,
    parent: Option<Rc<Scope>>,
}

thread_local! {
    static QMARK: Rc<str> = Rc::from("?");
    static PARTIAL: Rc<str> = Rc::from("partial");
}

impl Scope {
    pub fn root() -> Rc<Scope> {
        Rc::new(Scope {
            frame: Frame::Vars(RefCell::new(Vec::new())),
            parent: None,
        })
    }

    pub fn child(parent: &Rc<Scope>) -> Rc<Scope> {
        Rc::new(Scope {
            frame: Frame::Vars(RefCell::new(Vec::new())),
            parent: Some(parent.clone()),
        })
    }

    /// Child scope binding a single name.
    pub fn with(parent: &Rc<Scope>, name: &str, v: Value) -> Rc<Scope> {
        Scope::with_rc(parent, Rc::from(name), v)
    }

    pub fn with_rc(parent: &Rc<Scope>, name: Rc<str>, v: Value) -> Rc<Scope> {
        Rc::new(Scope {
            frame: Frame::One(name, v),
            parent: Some(parent.clone()),
        })
    }

    /// Child scope binding `?`.
    pub fn with_qmark(parent: &Rc<Scope>, v: Value) -> Rc<Scope> {
        Scope::with_rc(parent, QMARK.with(|q| q.clone()), v)
    }

    /// Child scope binding `partial`.
    pub fn with_partial(parent: &Rc<Scope>, v: Value) -> Rc<Scope> {
        Scope::with_rc(parent, PARTIAL.with(|q| q.clone()), v)
    }

    /// Child scope for a filter item.
    pub fn item(parent: &Rc<Scope>, item: Value) -> Rc<Scope> {
        Rc::new(Scope {
            frame: Frame::Item(item),
            parent: Some(parent.clone()),
        })
    }

    /// Define (or redefine) a variable; only valid on scopes created by `root` / `child`.
    pub fn set(&self, name: &str, v: Value) {
        match &self.frame {
            Frame::Vars(vars) => {
                let mut vars = vars.borrow_mut();
                if let Some(e) = vars.iter_mut().find(|(k, _)| &**k == name) {
                    e.1 = v;
                } else {
                    vars.push((Rc::from(name), v));
                }
            }
            _ => panic!("cannot define variables in a single-variable scope"),
        }
    }

    /// Remove all variables (used to break reference cycles).
    pub fn clear(&self) {
        if let Frame::Vars(vars) = &self.frame {
            vars.borrow_mut().clear();
        }
    }

    pub fn lookup(&self, name: &str) -> Option<Value> {
        let mut s = self;
        loop {
            match &s.frame {
                Frame::Vars(vars) => {
                    if let Some((_, v)) = vars.borrow().iter().rev().find(|(k, _)| &**k == name) {
                        return Some(v.clone());
                    }
                }
                Frame::One(n, v) => {
                    if &**n == name {
                        return Some(v.clone());
                    }
                }
                Frame::Item(v) => {
                    if let Value::Context(c) = v
                        && let Some(x) = c.get(name)
                    {
                        return Some(x.clone());
                    }
                    if name == "item" {
                        return Some(v.clone());
                    }
                }
            }
            s = s.parent.as_ref()?;
        }
    }
}

/// Function implemented natively (decision services, ...).
pub trait NativeFunction {
    fn param_names(&self) -> Vec<Rc<str>>;
    fn invoke(&self, args: Vec<Value>) -> Value;
}

pub struct Closure {
    pub def: Rc<FunctionDef>,
    pub scope: Rc<Scope>,
    /// Name the function is bound to (for recursion).
    pub name: RefCell<Option<Rc<str>>>,
}

pub enum Function {
    Builtin(Rc<str>),
    Closure(Closure),
    Native(Rc<dyn NativeFunction>),
}

impl Function {
    pub fn same_as(&self, other: &Function) -> bool {
        match (self, other) {
            (Function::Builtin(a), Function::Builtin(b)) => a == b,
            _ => std::ptr::eq(self, other),
        }
    }

    pub fn param_count(&self) -> Option<usize> {
        match self {
            Function::Builtin(_) => None,
            Function::Closure(c) => Some(c.def.params.len()),
            Function::Native(n) => Some(n.param_names().len()),
        }
    }

    pub fn conforms_to(&self, params: &[Rc<DmnType>], ret: &Rc<DmnType>) -> bool {
        match self {
            Function::Closure(c) => {
                if c.def.params.len() != params.len() {
                    return false;
                }
                for ((_, pt), t) in c.def.params.iter().zip(params.iter()) {
                    let pt = pt.clone().unwrap_or_else(|| Rc::new(DmnType::Any));
                    if !types::conforms_type(t, &pt) {
                        return false;
                    }
                }
                match &c.def.result_type {
                    Some(rt) => types::conforms_type(rt, ret),
                    None => true,
                }
            }
            Function::Builtin(_) => true,
            Function::Native(n) => n.param_names().len() == params.len(),
        }
    }
}

pub fn null() -> Value {
    Value::Null
}

pub fn eval(e: &Expr, scope: &Rc<Scope>) -> Value {
    match e {
        Expr::Const(v) => v.clone(),
        Expr::Error(_) => Value::Null,
        Expr::Name(n) => lookup_name(n, scope),
        Expr::Neg(a) => negate(&eval(a, scope)),
        Expr::Arith(op, a, b) => arith(*op, &eval(a, scope), &eval(b, scope)),
        Expr::Cmp(op, a, b) => compare(*op, &eval(a, scope), &eval(b, scope)),
        Expr::And(a, b) => {
            let x = eval(a, scope);
            if let Value::Bool(false) = x {
                return Value::Bool(false);
            }
            let y = eval(b, scope);
            match (x, y) {
                (Value::Bool(true), Value::Bool(true)) => Value::Bool(true),
                (_, Value::Bool(false)) => Value::Bool(false),
                _ => Value::Null,
            }
        }
        Expr::Or(a, b) => {
            let x = eval(a, scope);
            if let Value::Bool(true) = x {
                return Value::Bool(true);
            }
            let y = eval(b, scope);
            match (x, y) {
                (Value::Bool(false), Value::Bool(false)) => Value::Bool(false),
                (_, Value::Bool(true)) => Value::Bool(true),
                _ => Value::Null,
            }
        }
        Expr::Between(x, lo, hi) => {
            let x = eval(x, scope);
            let lo = eval(lo, scope);
            let hi = eval(hi, scope);
            let a = compare(CmpOp::Ge, &x, &lo);
            let b = compare(CmpOp::Le, &x, &hi);
            match (a, b) {
                (Value::Bool(true), Value::Bool(true)) => Value::Bool(true),
                (Value::Bool(false), _) | (_, Value::Bool(false)) => Value::Bool(false),
                _ => Value::Null,
            }
        }
        Expr::In(x, tests) => {
            let x = eval(x, scope);
            eval_unary_tests(tests, &x, scope)
        }
        Expr::InstanceOf(x, t) => {
            let x = eval(x, scope);
            Value::Bool(types::is_instance(&x, t))
        }
        Expr::If(c, a, b) => match eval(c, scope) {
            Value::Bool(true) => eval(a, scope),
            _ => eval(b, scope),
        },
        Expr::For(iters, body) => eval_for(iters, body, scope),
        Expr::Quantified { every, iters, cond } => eval_quantified(*every, iters, cond, scope),
        Expr::List(items) => Value::list(items.iter().map(|i| eval(i, scope)).collect()),
        Expr::Context(entries) => eval_context(entries, scope),
        Expr::Range {
            start,
            start_incl,
            end,
            end_incl,
        } => {
            let s = eval(start, scope);
            let e = eval(end, scope);
            make_range(s, *start_incl, e, *end_incl)
        }
        Expr::UnaryRange(op, e) => {
            let v = eval(e, scope);
            if v.is_null() && !matches!(op, CmpOp::Eq | CmpOp::Ne) {
                return Value::Null;
            }
            Value::Range(Rc::new(Range::from_unary(*op, v)))
        }
        Expr::FunctionDef(def) => Value::Function(Rc::new(Function::Closure(Closure {
            def: def.clone(),
            scope: scope.clone(),
            name: RefCell::new(None),
        }))),
        Expr::Call(f, args) => eval_call(f, args, scope),
        Expr::Path(base, name) => {
            let b = eval(base, scope);
            path(&b, name)
        }
        Expr::Filter(base, f) => eval_filter(base, f, scope),
        Expr::DecisionTable(dt) => dt.evaluate(scope),
        Expr::Coerce(e, t) => {
            let v = eval(e, scope);
            types::coerce(&v, t)
        }
        Expr::Strict(inner) => eval_strict(inner, scope),
    }
}

fn eval_strict(e: &Expr, scope: &Rc<Scope>) -> Value {
    match e {
        Expr::If(c, a, b) => match eval(c, scope) {
            Value::Bool(true) => eval(a, scope),
            Value::Bool(false) | Value::Null => eval(b, scope),
            _ => Value::Null,
        },
        Expr::Filter(base, f) => {
            let items = match eval(base, scope) {
                Value::List(l) => l,
                _ => return Value::Null,
            };
            let mut out = Vec::new();
            for item in items.iter() {
                let s = item_scope(scope, item);
                match eval(f, &s) {
                    Value::Bool(true) => out.push(item.clone()),
                    Value::Bool(false) | Value::Null => {}
                    _ => return Value::Null,
                }
            }
            Value::list(out)
        }
        Expr::Quantified { every, iters, cond } => {
            for (_, e) in iters {
                if !matches!(eval(e, scope), Value::List(_)) {
                    return Value::Null;
                }
            }
            eval_quantified_strict(*every, iters, cond, scope)
        }
        Expr::For(iters, _) => {
            for (_, it) in iters {
                if let Iteration::Expr(e) = it
                    && !matches!(eval(e, scope), Value::List(_))
                {
                    return Value::Null;
                }
            }
            eval(e, scope)
        }
        other => eval(other, scope),
    }
}

fn eval_quantified_strict(
    every: bool,
    iters: &[(Rc<str>, Expr)],
    cond: &Expr,
    scope: &Rc<Scope>,
) -> Value {
    let mut results = Vec::new();
    fn rec(
        i: usize,
        iters: &[(Rc<str>, Expr)],
        cond: &Expr,
        scope: &Rc<Scope>,
        results: &mut Vec<Value>,
    ) {
        if i == iters.len() {
            results.push(eval(cond, scope));
            return;
        }
        let (name, e) = &iters[i];
        if let Value::List(l) = eval(e, scope) {
            for v in l.iter() {
                let s = Scope::with_rc(scope, name.clone(), v.clone());
                rec(i + 1, iters, cond, &s, results);
            }
        }
    }
    rec(0, iters, cond, scope, &mut results);
    let mut acc = every;
    for r in &results {
        match r {
            Value::Bool(b) => {
                if every {
                    acc = acc && *b;
                } else {
                    acc = acc || *b;
                }
            }
            _ => return Value::Null,
        }
    }
    Value::Bool(acc)
}

fn lookup_name(n: &str, scope: &Rc<Scope>) -> Value {
    if let Some(v) = scope.lookup(n) {
        return v;
    }
    if builtins::is_builtin_name(n) {
        return Value::Function(Rc::new(Function::Builtin(Rc::from(n))));
    }
    // a name containing dots that is not in scope: interpret as a path
    if n.contains('.') {
        let parts: Vec<&str> = n.split('.').map(|p| p.trim()).collect();
        for split in (1..parts.len()).rev() {
            let head = parts[..split].join(".");
            if let Some(mut v) = scope.lookup(&head) {
                for p in &parts[split..] {
                    v = path(&v, p);
                }
                return v;
            }
        }
    }
    Value::Null
}

fn make_range(s: Value, si: bool, e: Value, ei: bool) -> Value {
    // endpoints must be comparable values of the same kind
    let ok = match (&s, &e) {
        (Value::Null, Value::Null) => false,
        (Value::Null, x) | (x, Value::Null) => is_rangeable(x),
        (a, b) => {
            is_rangeable(a)
                && std::mem::discriminant(a) == std::mem::discriminant(b)
                && matches!(feel_cmp(a, b), Some(Ordering::Less) | Some(Ordering::Equal))
        }
    };
    if !ok {
        return Value::Null;
    }
    Value::Range(Rc::new(Range {
        start: Some(s),
        start_incl: si,
        end: Some(e),
        end_incl: ei,
        unary: None,
    }))
}

fn is_rangeable(v: &Value) -> bool {
    matches!(
        v,
        Value::Number(_)
            | Value::String(_)
            | Value::Date(_)
            | Value::Time(_)
            | Value::DateTime(_)
            | Value::DtDuration(_)
            | Value::YmDuration(_)
    )
}

fn eval_context(entries: &[ContextEntry], scope: &Rc<Scope>) -> Value {
    let local = Scope::child(scope);
    let mut ctx = Context::new();
    for entry in entries {
        let v = eval(&entry.expr, &local);
        match &entry.name {
            Some(name) => {
                if ctx.contains(name) {
                    return Value::Null;
                }
                if let Value::Function(f) = &v
                    && let Function::Closure(c) = &**f
                    && c.name.borrow().is_none()
                {
                    *c.name.borrow_mut() = Some(name.clone());
                }
                local.set(name, v.clone());
                ctx.put(name.clone(), v);
            }
            None => return v,
        }
    }
    Value::context(ctx)
}

fn iteration_values(v: Value) -> Option<Vec<Value>> {
    match v {
        Value::List(l) => Some(l.as_ref().clone()),
        Value::Range(r) => {
            let lo = *r.low()?.as_number()?;
            let hi = *r.high()?.as_number()?;
            let mut start = lo.to_i64()?;
            let mut end = hi.to_i64()?;
            if !r.start_incl {
                start += 1;
            }
            if !r.end_incl {
                end -= 1;
            }
            Some((start..=end).map(Value::int).collect())
        }
        Value::Null => None,
        other => Some(vec![other]),
    }
}

fn range_values(a: Value, b: Value) -> Option<Vec<Value>> {
    match (&a, &b) {
        (Value::Number(x), Value::Number(y)) => {
            let x = x.to_i64()?;
            let y = y.to_i64()?;
            if (x - y).abs() > 10_000_000 {
                return None;
            }
            Some(if x <= y {
                (x..=y).map(Value::int).collect()
            } else {
                (y..=x).rev().map(Value::int).collect()
            })
        }
        (Value::Date(x), Value::Date(y)) => {
            let (dx, dy) = (x.days(), y.days());
            let it: Vec<i64> = if dx <= dy {
                (dx..=dy).collect()
            } else {
                (dy..=dx).rev().collect()
            };
            Some(
                it.into_iter()
                    .filter_map(|d| Date::from_days(d).map(Value::Date))
                    .collect(),
            )
        }
        _ => None,
    }
}

fn eval_for(iters: &[(Rc<str>, Iteration)], body: &Expr, scope: &Rc<Scope>) -> Value {
    let mut results: Vec<Value> = Vec::new();
    // `partial` (the results so far) is only materialized when the body uses it
    let partial = body.uses_name("partial");
    #[allow(clippy::too_many_arguments)]
    fn rec(
        i: usize,
        iters: &[(Rc<str>, Iteration)],
        body: &Expr,
        scope: &Rc<Scope>,
        results: &mut Vec<Value>,
        partial: bool,
    ) -> bool {
        if i == iters.len() {
            let v = if partial {
                let s = Scope::with_partial(scope, Value::list(results.clone()));
                eval(body, &s)
            } else {
                eval(body, scope)
            };
            results.push(v);
            return true;
        }
        let (name, it) = &iters[i];
        let values = match it {
            Iteration::Expr(e) => iteration_values(eval(e, scope)),
            Iteration::Range(a, b) => range_values(eval(a, scope), eval(b, scope)),
        };
        let values = match values {
            Some(v) => v,
            None => return false,
        };
        for v in values {
            let s = Scope::with_rc(scope, name.clone(), v);
            if !rec(i + 1, iters, body, &s, results, partial) {
                return false;
            }
        }
        true
    }
    if !rec(0, iters, body, scope, &mut results, partial) {
        return Value::Null;
    }
    Value::list(results)
}

fn eval_quantified(
    every: bool,
    iters: &[(Rc<str>, Expr)],
    cond: &Expr,
    scope: &Rc<Scope>,
) -> Value {
    let mut results: Vec<Value> = Vec::new();
    fn rec(
        i: usize,
        iters: &[(Rc<str>, Expr)],
        cond: &Expr,
        scope: &Rc<Scope>,
        results: &mut Vec<Value>,
    ) -> bool {
        if i == iters.len() {
            results.push(eval(cond, scope));
            return true;
        }
        let (name, e) = &iters[i];
        let values = match iteration_values(eval(e, scope)) {
            Some(v) => v,
            None => return false,
        };
        for v in values {
            let s = Scope::with_rc(scope, name.clone(), v);
            if !rec(i + 1, iters, cond, &s, results) {
                return false;
            }
        }
        true
    }
    if !rec(0, iters, cond, scope, &mut results) {
        return Value::Null;
    }
    if every {
        let mut all = true;
        for r in &results {
            match r {
                Value::Bool(true) => {}
                Value::Bool(false) => return Value::Bool(false),
                _ => all = false,
            }
        }
        if all { Value::Bool(true) } else { Value::Null }
    } else {
        let mut any_null = false;
        for r in &results {
            match r {
                Value::Bool(true) => return Value::Bool(true),
                Value::Bool(false) => {}
                _ => any_null = true,
            }
        }
        if any_null {
            Value::Null
        } else {
            Value::Bool(false)
        }
    }
}

fn index_list(items: &[Value], n: &Number) -> Value {
    let i = match n.to_i64() {
        Some(i) => i,
        None => return Value::Null,
    };
    let len = items.len() as i64;
    let idx = if i > 0 {
        i - 1
    } else if i < 0 {
        len + i
    } else {
        return Value::Null;
    };
    if idx < 0 || idx >= len {
        return Value::Null;
    }
    items[idx as usize].clone()
}

fn item_scope(scope: &Rc<Scope>, item: &Value) -> Rc<Scope> {
    Scope::item(scope, item.clone())
}

fn eval_filter(base: &Expr, f: &Expr, scope: &Rc<Scope>) -> Value {
    let b = eval(base, scope);
    let items: Vec<Value> = match &b {
        Value::List(l) => l.as_ref().clone(),
        Value::Null => return Value::Null,
        other => vec![other.clone()],
    };
    // numeric index
    if !f.uses_name("item") {
        let probe = eval(f, scope);
        if let Value::Number(n) = &probe {
            return index_list(&items, n);
        }
    }
    let mut out = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let s = item_scope(scope, item);
        let r = eval(f, &s);
        match r {
            Value::Bool(true) => out.push(item.clone()),
            Value::Number(n) if i == 0 => return index_list(&items, &n),
            _ => {}
        }
    }
    Value::list(out)
}

fn eval_args(args: &Args, scope: &Rc<Scope>) -> CallArgs {
    match args {
        Args::Positional(a) => CallArgs::Positional(a.iter().map(|e| eval(e, scope)).collect()),
        Args::Named(a) => {
            CallArgs::Named(a.iter().map(|(n, e)| (n.clone(), eval(e, scope))).collect())
        }
    }
}

pub enum CallArgs {
    Positional(Vec<Value>),
    Named(Vec<(Rc<str>, Value)>),
}

fn eval_call(f: &Expr, args: &Args, scope: &Rc<Scope>) -> Value {
    let fv = match f {
        Expr::Name(n) => match scope.lookup(n) {
            Some(v) => v,
            None => {
                if builtins::is_builtin_name(n) {
                    let a = eval_args(args, scope);
                    return builtins::call_builtin(n, a, scope);
                }
                return Value::Null;
            }
        },
        other => eval(other, scope),
    };
    let a = eval_args(args, scope);
    match fv {
        Value::Function(func) => invoke(&func, a, scope),
        _ => Value::Null,
    }
}

/// Invoke a function value.
pub fn invoke(f: &Rc<Function>, args: CallArgs, scope: &Rc<Scope>) -> Value {
    match &**f {
        Function::Builtin(name) => builtins::call_builtin(name, args, scope),
        Function::Closure(c) => {
            let params = &c.def.params;
            let values: Vec<Value> = match args {
                CallArgs::Positional(v) => {
                    if v.len() != params.len() {
                        return Value::Null;
                    }
                    v
                }
                CallArgs::Named(named) => {
                    let mut vals = vec![Value::Null; params.len()];
                    for (n, v) in named {
                        match params.iter().position(|(p, _)| *p == n) {
                            Some(i) => vals[i] = v,
                            None => return Value::Null,
                        }
                    }
                    vals
                }
            };
            let s = Scope::child(&c.scope);
            if let Some(name) = c.name.borrow().as_ref()
                && c.scope.lookup(name).is_none()
            {
                s.set(name, Value::Function(f.clone()));
            }
            for ((p, ty), v) in params.iter().zip(values) {
                let v = match ty {
                    Some(t) => {
                        if v.is_null() {
                            v
                        } else {
                            let cv = types::coerce(&v, t);
                            if cv.is_null() {
                                return Value::Null;
                            }
                            cv
                        }
                    }
                    None => v,
                };
                s.set(p, v);
            }
            let r = match c.def.kind {
                FnKind::Feel => eval(&c.def.body, &s),
                FnKind::External => {
                    let spec = eval(&c.def.body, &s);
                    let args: Vec<Value> = params
                        .iter()
                        .map(|(p, _)| s.lookup(p).unwrap_or(Value::Null))
                        .collect();
                    builtins::call_external(&spec, args)
                }
            };
            match &c.def.result_type {
                Some(t) => types::coerce(&r, t),
                None => r,
            }
        }
        Function::Native(n) => {
            let names = n.param_names();
            let values: Vec<Value> = match args {
                CallArgs::Positional(v) => {
                    if v.len() != names.len() {
                        return Value::Null;
                    }
                    v
                }
                CallArgs::Named(named) => {
                    let mut vals = vec![Value::Null; names.len()];
                    for (k, v) in named {
                        match names.iter().position(|p| *p == k) {
                            Some(i) => vals[i] = v,
                            None => return Value::Null,
                        }
                    }
                    vals
                }
            };
            n.invoke(values)
        }
    }
}

/// Invoke with positional arguments.
pub fn call(f: &Rc<Function>, args: Vec<Value>, scope: &Rc<Scope>) -> Value {
    invoke(f, CallArgs::Positional(args), scope)
}

pub fn negate(v: &Value) -> Value {
    match v {
        Value::Number(n) => Value::Number(n.neg()),
        Value::DtDuration(d) => Value::DtDuration(DtDuration(-d.0)),
        Value::YmDuration(d) => Value::YmDuration(YmDuration(-d.0)),
        _ => Value::Null,
    }
}

pub fn compare(op: CmpOp, a: &Value, b: &Value) -> Value {
    match op {
        CmpOp::Eq => feel_eq(a, b).map(Value::Bool).unwrap_or(Value::Null),
        CmpOp::Ne => feel_eq(a, b)
            .map(|x| Value::Bool(!x))
            .unwrap_or(Value::Null),
        _ => {
            let o = match feel_cmp(a, b) {
                Some(o) => o,
                None => return Value::Null,
            };
            Value::Bool(match op {
                CmpOp::Lt => o == Ordering::Less,
                CmpOp::Le => o != Ordering::Greater,
                CmpOp::Gt => o == Ordering::Greater,
                CmpOp::Ge => o != Ordering::Less,
                _ => unreachable!(),
            })
        }
    }
}

fn mul_dt(d: &DtDuration, n: &Number) -> Value {
    // d.0 nanos * n
    let r = Number::from_i128(d.0)
        .mul(n)
        .round_scale(0, crate::number::Rounding::HalfEven);
    match r.to_i128() {
        Some(v) => Value::DtDuration(DtDuration(v)),
        None => Value::Null,
    }
}

fn mul_ym(d: &YmDuration, n: &Number) -> Value {
    let r = Number::from_i64(d.0)
        .mul(n)
        .round_scale(0, crate::number::Rounding::Down);
    match r.to_i64() {
        Some(v) => Value::YmDuration(YmDuration(v)),
        None => Value::Null,
    }
}

fn div_dt(d: &DtDuration, n: &Number) -> Value {
    match Number::from_i128(d.0).div(n) {
        Some(r) => match r
            .round_scale(0, crate::number::Rounding::HalfEven)
            .to_i128()
        {
            Some(v) => Value::DtDuration(DtDuration(v)),
            None => Value::Null,
        },
        None => Value::Null,
    }
}

fn div_ym(d: &YmDuration, n: &Number) -> Value {
    match Number::from_i64(d.0).div(n) {
        Some(r) => match r.round_scale(0, crate::number::Rounding::Down).to_i64() {
            Some(v) => Value::YmDuration(YmDuration(v)),
            None => Value::Null,
        },
        None => Value::Null,
    }
}

fn datetime_sub(a: &DateTime, b: &DateTime) -> Value {
    let d = match (a.has_zone(), b.has_zone()) {
        (true, true) => match (a.instant(), b.instant()) {
            (Some(x), Some(y)) => x - y,
            _ => return Value::Null,
        },
        (false, false) => a.local_nanos() - b.local_nanos(),
        _ => return Value::Null,
    };
    Value::DtDuration(DtDuration(d))
}

fn time_value(t: &Time) -> i128 {
    match t.fixed_offset() {
        Some(o) => t.nanos_of_day() - o as i128 * NANOS_PER_SEC,
        None => t.nanos_of_day(),
    }
}

fn date_add_nanos(d: &Date, n: i128) -> Value {
    let days = n.div_euclid(NANOS_PER_DAY);
    match i64::try_from(days)
        .ok()
        .and_then(|x| Date::from_days(d.days() + x))
    {
        Some(d) => Value::Date(d),
        None => Value::Null,
    }
}

pub fn arith(op: ArithOp, a: &Value, b: &Value) -> Value {
    use Value::*;
    match op {
        ArithOp::Add => match (a, b) {
            (Number(x), Number(y)) => Number(x.add(y)),
            (String(x), String(y)) => {
                let mut s = x.to_string();
                s.push_str(y);
                Value::str(&s)
            }
            (DtDuration(x), DtDuration(y)) => DtDuration(crate::temporal::DtDuration(x.0 + y.0)),
            (YmDuration(x), YmDuration(y)) => YmDuration(crate::temporal::YmDuration(x.0 + y.0)),
            (DateTime(d), DtDuration(x)) | (DtDuration(x), DateTime(d)) => {
                d.add_nanos(x.0).map(DateTime).unwrap_or(Null)
            }
            (DateTime(d), YmDuration(x)) | (YmDuration(x), DateTime(d)) => {
                d.add_months(x.0).map(DateTime).unwrap_or(Null)
            }
            (Date(d), DtDuration(x)) | (DtDuration(x), Date(d)) => date_add_nanos(d, x.0),
            (Date(d), YmDuration(x)) | (YmDuration(x), Date(d)) => {
                d.add_months(x.0).map(Date).unwrap_or(Null)
            }
            (Time(t), DtDuration(x)) | (DtDuration(x), Time(t)) => Time(
                crate::temporal::Time::from_nanos_of_day(t.nanos_of_day() + x.0, t.zone),
            ),
            _ => Null,
        },
        ArithOp::Sub => match (a, b) {
            (Number(x), Number(y)) => Number(x.sub(y)),
            (DtDuration(x), DtDuration(y)) => DtDuration(crate::temporal::DtDuration(x.0 - y.0)),
            (YmDuration(x), YmDuration(y)) => YmDuration(crate::temporal::YmDuration(x.0 - y.0)),
            (DateTime(x), DateTime(y)) => datetime_sub(x, y),
            (Date(x), Date(y)) => DtDuration(crate::temporal::DtDuration(
                (x.days() - y.days()) as i128 * NANOS_PER_DAY,
            )),
            (Date(x), DateTime(y)) => datetime_sub(&date_as_dt(x, Some(Zone::Offset(0))), y),
            (DateTime(x), Date(y)) => datetime_sub(x, &date_as_dt(y, Some(Zone::Offset(0)))),
            (Time(x), Time(y)) => {
                if x.zone.is_some() != y.zone.is_some() {
                    return Null;
                }
                DtDuration(crate::temporal::DtDuration(time_value(x) - time_value(y)))
            }
            (DateTime(d), DtDuration(x)) => d.add_nanos(-x.0).map(DateTime).unwrap_or(Null),
            (DateTime(d), YmDuration(x)) => d.add_months(-x.0).map(DateTime).unwrap_or(Null),
            (Date(d), DtDuration(x)) => date_add_nanos(d, -x.0),
            (Date(d), YmDuration(x)) => d.add_months(-x.0).map(Date).unwrap_or(Null),
            (Time(t), DtDuration(x)) => Time(crate::temporal::Time::from_nanos_of_day(
                t.nanos_of_day() - x.0,
                t.zone,
            )),
            _ => Null,
        },
        ArithOp::Mul => match (a, b) {
            (Number(x), Number(y)) => Number(x.mul(y)),
            (DtDuration(d), Number(n)) | (Number(n), DtDuration(d)) => mul_dt(d, n),
            (YmDuration(d), Number(n)) | (Number(n), YmDuration(d)) => mul_ym(d, n),
            _ => Null,
        },
        ArithOp::Div => match (a, b) {
            (Number(x), Number(y)) => x.div(y).map(Number).unwrap_or(Null),
            (DtDuration(d), Number(n)) => div_dt(d, n),
            (YmDuration(d), Number(n)) => div_ym(d, n),
            (DtDuration(x), DtDuration(y)) => crate::number::Number::from_i128(x.0)
                .div(&crate::number::Number::from_i128(y.0))
                .map(Number)
                .unwrap_or(Null),
            (YmDuration(x), YmDuration(y)) => crate::number::Number::from_i64(x.0)
                .div(&crate::number::Number::from_i64(y.0))
                .map(Number)
                .unwrap_or(Null),
            _ => Null,
        },
        ArithOp::Pow => match (a, b) {
            (Number(x), Number(y)) => {
                if let Some(i) = y.to_i64()
                    && i.abs() <= 1_000_000
                {
                    return x.pow_int(i).map(Number).unwrap_or(Null);
                }
                x.pow_f64(y).map(Number).unwrap_or(Null)
            }
            _ => Null,
        },
    }
}

fn date_as_dt(d: &Date, zone: Option<Zone>) -> DateTime {
    let mut dt = DateTime::from_date(*d);
    dt.time.zone = zone;
    dt
}

pub fn offset_duration(o: i32) -> Value {
    Value::DtDuration(DtDuration(o as i128 * NANOS_PER_SEC))
}

fn time_props(t: &Time, name: &str, dt: Option<&DateTime>) -> Value {
    match name {
        "hour" => Value::int(t.hour as i64),
        "minute" => Value::int(t.minute as i64),
        "second" => {
            let n = Number::from_i64(t.second as i64).add(
                &Number::from_i64(t.nanos as i64)
                    .div(&Number::from_i64(1_000_000_000))
                    .unwrap(),
            );
            Value::Number(n)
        }
        "time offset" => match &t.zone {
            Some(Zone::Offset(o)) => offset_duration(*o),
            Some(Zone::Id(_)) => match dt.and_then(|d| d.offset()) {
                Some(o) => offset_duration(o),
                None => Value::Null,
            },
            None => Value::Null,
        },
        "timezone" => match &t.zone {
            Some(Zone::Id(id)) => Value::str(id.name()),
            _ => Value::Null,
        },
        _ => Value::Null,
    }
}

fn date_props(d: &Date, name: &str) -> Option<Value> {
    Some(match name {
        "year" => Value::int(d.year as i64),
        "month" => Value::int(d.month as i64),
        "day" => Value::int(d.day as i64),
        "weekday" => Value::int(d.weekday() as i64),
        _ => return None,
    })
}

pub fn path(b: &Value, name: &str) -> Value {
    match b {
        Value::Context(c) => c.get(name).cloned().unwrap_or(Value::Null),
        Value::List(l) => Value::list(l.iter().map(|i| path(i, name)).collect()),
        Value::Date(d) => date_props(d, name).unwrap_or(Value::Null),
        Value::DateTime(dt) => match date_props(&dt.date, name) {
            Some(v) => v,
            None => time_props(&dt.time, name, Some(dt)),
        },
        Value::Time(t) => time_props(t, name, None),
        Value::DtDuration(d) => {
            let n = d.0;
            let total_secs = n / NANOS_PER_SEC;
            match name {
                "days" => Value::int((total_secs / 86400) as i64),
                "hours" => Value::int(((total_secs / 3600) % 24) as i64),
                "minutes" => Value::int(((total_secs / 60) % 60) as i64),
                "seconds" => {
                    let secs = Number::from_i128(total_secs % 60);
                    let frac = Number::from_i128(n % NANOS_PER_SEC)
                        .div(&Number::from_i64(1_000_000_000))
                        .unwrap();
                    Value::Number(secs.add(&frac))
                }
                _ => Value::Null,
            }
        }
        Value::YmDuration(d) => match name {
            "years" => Value::int(d.0 / 12),
            "months" => Value::int(d.0 % 12),
            _ => Value::Null,
        },
        Value::Range(r) => match name {
            "start" => r.start.clone().unwrap_or(Value::Null),
            "end" => r.end.clone().unwrap_or(Value::Null),
            "start included" => Value::Bool(r.start_incl),
            "end included" => Value::Bool(r.end_incl),
            _ => Value::Null,
        },
        _ => Value::Null,
    }
}

/// Evaluate unary tests against an input value; returns Bool or Null.
pub fn eval_unary_tests(ut: &UnaryTests, x: &Value, scope: &Rc<Scope>) -> Value {
    let qs;
    let s = if ut.qmark {
        qs = Scope::with_qmark(scope, x.clone());
        &qs
    } else {
        scope
    };
    let mut any_null = false;
    let mut matched = false;
    for t in &ut.tests {
        match test_one(t, x, s) {
            Some(true) => {
                matched = true;
                break;
            }
            Some(false) => {}
            None => any_null = true,
        }
    }
    let r = if matched {
        Some(true)
    } else if any_null {
        None
    } else {
        Some(false)
    };
    match r {
        Some(b) => Value::Bool(if ut.negated { !b } else { b }),
        None => Value::Null,
    }
}

fn test_one(t: &UnaryTest, x: &Value, scope: &Rc<Scope>) -> Option<bool> {
    match t {
        UnaryTest::Any => Some(true),
        UnaryTest::Cmp(op, e) => {
            let v = eval(e, scope);
            match compare(*op, x, &v) {
                Value::Bool(b) => Some(b),
                _ => None,
            }
        }
        UnaryTest::Expr { expr, uses_qmark } => {
            let v = eval(expr, scope);
            if *uses_qmark {
                return match v {
                    Value::Bool(b) => Some(b),
                    _ => None,
                };
            }
            match &v {
                Value::Bool(true) if !matches!(x, Value::Bool(_)) => Some(true),
                Value::List(l) => {
                    if let Value::List(_) = x
                        && feel_eq(x, &v) == Some(true)
                    {
                        return Some(true);
                    }
                    for i in l.iter() {
                        let r = match i {
                            Value::Range(r) => r.contains(x),
                            _ => feel_eq(x, i),
                        };
                        if r == Some(true) {
                            return Some(true);
                        }
                    }
                    Some(false)
                }
                Value::Range(r) => r.contains(x),
                _ => feel_eq(x, &v),
            }
        }
    }
}
