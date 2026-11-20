//! Compile time constant folding.
//!
//! Sub-expressions that do not depend on any variable (e.g. `[1..10)`,
//! `date("2020-01-01")`, `-5`, `"a" + "b"`) are evaluated once when the
//! model is compiled instead of on every evaluation.

use super::ast::*;
use super::eval::{Scope, eval};
use std::rc::Rc;

struct Folder<'a> {
    /// Names defined by the model (they may shadow built-in functions).
    known: &'a dyn Fn(&str) -> bool,
    /// Names bound by enclosing contexts, functions and iterators.
    bound: Vec<Rc<str>>,
    /// Depth of filter predicates (item entries may shadow anything).
    in_filter: usize,
}

/// Built-ins whose result does not only depend on their arguments.
fn is_impure(n: &str) -> bool {
    matches!(n, "now" | "today")
}

fn is_const(e: &Expr) -> bool {
    matches!(e, Expr::Const(_))
}

impl Folder<'_> {
    fn shadowed(&self, n: &str) -> bool {
        self.in_filter > 0 || (self.known)(n) || self.bound.iter().any(|b| &**b == n)
    }

    fn foldable(&self, e: &Expr) -> bool {
        use Expr::*;
        match e {
            Neg(a) | InstanceOf(a, _) | UnaryRange(_, a) | Path(a, _) | Coerce(a, _) => is_const(a),
            Arith(_, a, b) | Cmp(_, a, b) | And(a, b) | Or(a, b) => is_const(a) && is_const(b),
            Between(a, b, c) | If(a, b, c) => is_const(a) && is_const(b) && is_const(c),
            List(items) => items.iter().all(is_const),
            Context(entries) => entries.iter().all(|e| is_const(&e.expr)),
            Range { start, end, .. } => is_const(start) && is_const(end),
            In(a, t) => {
                is_const(a)
                    && !t.qmark
                    && t.tests.iter().all(|t| match t {
                        UnaryTest::Any => true,
                        UnaryTest::Cmp(_, e) => is_const(e),
                        UnaryTest::Expr { expr, .. } => is_const(expr),
                    })
            }
            Builtin(b, args) => {
                !is_impure(b.name())
                    && match args {
                        Args::Positional(a) => a.iter().all(is_const),
                        Args::Named(a) => a.iter().all(|(_, e)| is_const(e)),
                    }
            }
            _ => false,
        }
    }

    #[allow(clippy::boxed_local)]
    fn b(&mut self, e: Box<Expr>) -> Box<Expr> {
        Box::new(self.fold(*e))
    }

    fn fold_tests(&mut self, u: UnaryTests) -> UnaryTests {
        let tests = u
            .tests
            .into_iter()
            .map(|t| match t {
                UnaryTest::Any => UnaryTest::Any,
                UnaryTest::Cmp(op, e) => UnaryTest::Cmp(op, self.fold(e)),
                UnaryTest::Expr { expr, uses_qmark } => UnaryTest::Expr {
                    expr: self.fold(expr),
                    uses_qmark,
                },
            })
            .collect();
        UnaryTests::new(tests, u.negated)
    }

    fn fold(&mut self, e: Expr) -> Expr {
        use Expr::*;
        let e = match e {
            Neg(a) => Neg(self.b(a)),
            Arith(op, a, c) => Arith(op, self.b(a), self.b(c)),
            Cmp(op, a, c) => Cmp(op, self.b(a), self.b(c)),
            And(a, c) => And(self.b(a), self.b(c)),
            Or(a, c) => Or(self.b(a), self.b(c)),
            Between(a, c, d) => Between(self.b(a), self.b(c), self.b(d)),
            In(a, t) => {
                let a = self.b(a);
                match Rc::try_unwrap(t) {
                    Ok(t) => In(a, Rc::new(self.fold_tests(t))),
                    Err(t) => In(a, t),
                }
            }
            InstanceOf(a, t) => InstanceOf(self.b(a), t),
            If(c, a, d) => If(self.b(c), self.b(a), self.b(d)),
            For(iters, body) => {
                let n = self.bound.len();
                let mut out = Vec::with_capacity(iters.len());
                for (name, it) in iters {
                    let it = match it {
                        Iteration::Expr(e) => Iteration::Expr(self.fold(e)),
                        Iteration::Range(a, c) => Iteration::Range(self.fold(a), self.fold(c)),
                    };
                    self.bound.push(name.clone());
                    out.push((name, it));
                }
                self.bound.push(Rc::from("partial"));
                let body = self.b(body);
                self.bound.truncate(n);
                For(out, body)
            }
            Quantified { every, iters, cond } => {
                let n = self.bound.len();
                let mut out = Vec::with_capacity(iters.len());
                for (name, e) in iters {
                    let e = self.fold(e);
                    self.bound.push(name.clone());
                    out.push((name, e));
                }
                let cond = self.b(cond);
                self.bound.truncate(n);
                Quantified {
                    every,
                    iters: out,
                    cond,
                }
            }
            List(items) => List(items.into_iter().map(|i| self.fold(i)).collect()),
            Context(entries) => {
                let n = self.bound.len();
                self.bound
                    .extend(entries.iter().filter_map(|e| e.name.clone()));
                let entries = entries
                    .into_iter()
                    .map(|e| ContextEntry {
                        name: e.name,
                        expr: self.fold(e.expr),
                    })
                    .collect();
                self.bound.truncate(n);
                Context(entries)
            }
            Range {
                start,
                start_incl,
                end,
                end_incl,
            } => Range {
                start: self.b(start),
                start_incl,
                end: self.b(end),
                end_incl,
            },
            UnaryRange(op, a) => UnaryRange(op, self.b(a)),
            FunctionDef(def) => match Rc::try_unwrap(def) {
                Ok(mut def) => {
                    let n = self.bound.len();
                    self.bound.extend(def.params.iter().map(|(p, _)| p.clone()));
                    def.body = self.fold(def.body);
                    self.bound.truncate(n);
                    FunctionDef(Rc::new(def))
                }
                Err(def) => FunctionDef(def),
            },
            Call(f, args) => {
                let f = match *f {
                    Name(n) => Box::new(Name(n)),
                    other => Box::new(self.fold(other)),
                };
                let args = match args {
                    Args::Positional(a) => {
                        Args::Positional(a.into_iter().map(|e| self.fold(e)).collect())
                    }
                    Args::Named(a) => {
                        Args::Named(a.into_iter().map(|(n, e)| (n, self.fold(e))).collect())
                    }
                };
                // resolve calls of (unshadowed) built-in functions at compile time
                if let Name(n) = &*f
                    && !self.shadowed(n)
                    && let Some(b) = super::builtins::B::from_name(n)
                {
                    Builtin(b, args)
                } else {
                    Call(f, args)
                }
            }
            Path(a, n) => Path(self.b(a), n),
            Filter(a, f) => {
                let a = self.b(a);
                self.in_filter += 1;
                let f = self.b(f);
                self.in_filter -= 1;
                Filter(a, f)
            }
            Coerce(a, t) => Coerce(self.b(a), t),
            Strict(a) => Strict(self.b(a)),
            other => other,
        };
        if self.foldable(&e) {
            Expr::Const(eval(&e, &Scope::root()))
        } else {
            e
        }
    }
}

/// Fold constant sub-expressions; `known` tells which names the model defines.
pub fn fold_expr(e: Expr, known: &dyn Fn(&str) -> bool) -> Expr {
    Folder {
        known,
        bound: Vec::new(),
        in_filter: 0,
    }
    .fold(e)
}

/// Fold constant sub-expressions of unary tests.
pub fn fold_unary_tests(u: UnaryTests, known: &dyn Fn(&str) -> bool) -> UnaryTests {
    Folder {
        known,
        bound: Vec::new(),
        in_filter: 0,
    }
    .fold_tests(u)
}
