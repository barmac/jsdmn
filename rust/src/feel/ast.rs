//! FEEL / boxed expression syntax tree.

use crate::types::DmnType;
use crate::value::{CmpOp, Value};
use std::rc::Rc;

pub type TypeRef = Rc<DmnType>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArithOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
}

#[derive(Debug)]
pub enum Expr {
    Const(Value),
    Name(Rc<str>),
    Neg(Box<Expr>),
    Arith(ArithOp, Box<Expr>, Box<Expr>),
    Cmp(CmpOp, Box<Expr>, Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Between(Box<Expr>, Box<Expr>, Box<Expr>),
    In(Box<Expr>, Rc<UnaryTests>),
    InstanceOf(Box<Expr>, TypeRef),
    If(Box<Expr>, Box<Expr>, Box<Expr>),
    For(Vec<(Rc<str>, Iteration)>, Box<Expr>),
    Quantified {
        every: bool,
        iters: Vec<(Rc<str>, Expr)>,
        cond: Box<Expr>,
    },
    List(Vec<Expr>),
    Context(Vec<ContextEntry>),
    Range {
        start: Box<Expr>,
        start_incl: bool,
        end: Box<Expr>,
        end_incl: bool,
    },
    UnaryRange(CmpOp, Box<Expr>),
    FunctionDef(Rc<FunctionDef>),
    Call(Box<Expr>, Args),
    Path(Box<Expr>, Rc<str>),
    Filter(Box<Expr>, Box<Expr>),
    /// Boxed decision table.
    DecisionTable(Rc<crate::dmn::decision_table::DecisionTable>),
    /// Coerce result to a declared type.
    Coerce(Box<Expr>, TypeRef),
    /// Boxed (strict) variant of conditional / filter / iterators: type errors yield null.
    Strict(Box<Expr>),
    /// Expression that failed to parse; evaluates to null.
    Error(String),
}

#[derive(Debug)]
pub enum Iteration {
    Expr(Expr),
    Range(Expr, Expr),
}

#[derive(Debug)]
pub struct ContextEntry {
    pub name: Option<Rc<str>>,
    pub expr: Expr,
}

#[derive(Debug)]
pub enum Args {
    Positional(Vec<Expr>),
    Named(Vec<(Rc<str>, Expr)>),
}

#[derive(Debug)]
pub enum FnKind {
    Feel,
    /// `function(..) external { java: {...} }`
    External,
}

#[derive(Debug)]
pub struct FunctionDef {
    pub params: Vec<(Rc<str>, Option<TypeRef>)>,
    pub body: Expr,
    pub kind: FnKind,
    pub result_type: Option<TypeRef>,
}

#[derive(Debug)]
pub enum UnaryTest {
    /// `-`
    Any,
    Cmp(CmpOp, Expr),
    /// Plain expression (may be a range, list, or boolean expression using `?`).
    Expr {
        expr: Expr,
        uses_qmark: bool,
    },
}

#[derive(Debug)]
pub struct UnaryTests {
    pub tests: Vec<UnaryTest>,
    pub negated: bool,
    /// Whether any test references `?` (requires binding the input value).
    pub qmark: bool,
}

impl Expr {
    pub fn uses_name(&self, n: &str) -> bool {
        use Expr::*;
        match self {
            Name(x) => &**x == n,
            Const(_) | Error(_) | DecisionTable(_) => false,
            Neg(a)
            | Coerce(a, _)
            | InstanceOf(a, _)
            | Path(a, _)
            | UnaryRange(_, a)
            | Strict(a) => a.uses_name(n),
            Arith(_, a, b) | Cmp(_, a, b) | And(a, b) | Or(a, b) | Filter(a, b) => {
                a.uses_name(n) || b.uses_name(n)
            }
            Between(a, b, c) | If(a, b, c) => a.uses_name(n) || b.uses_name(n) || c.uses_name(n),
            In(a, t) => a.uses_name(n) || t.uses_name(n),
            For(its, body) => {
                its.iter().any(|(_, it)| match it {
                    Iteration::Expr(e) => e.uses_name(n),
                    Iteration::Range(a, b) => a.uses_name(n) || b.uses_name(n),
                }) || body.uses_name(n)
            }
            Quantified { iters, cond, .. } => {
                iters.iter().any(|(_, e)| e.uses_name(n)) || cond.uses_name(n)
            }
            List(l) => l.iter().any(|e| e.uses_name(n)),
            Context(c) => c.iter().any(|e| e.expr.uses_name(n)),
            Range { start, end, .. } => start.uses_name(n) || end.uses_name(n),
            FunctionDef(f) => f.body.uses_name(n),
            Call(f, args) => {
                f.uses_name(n)
                    || match args {
                        Args::Positional(a) => a.iter().any(|e| e.uses_name(n)),
                        Args::Named(a) => a.iter().any(|(_, e)| e.uses_name(n)),
                    }
            }
        }
    }
}

impl UnaryTests {
    pub fn new(tests: Vec<UnaryTest>, negated: bool) -> UnaryTests {
        let mut u = UnaryTests {
            tests,
            negated,
            qmark: false,
        };
        u.qmark = u.uses_name("?");
        u
    }

    pub fn uses_name(&self, n: &str) -> bool {
        self.tests.iter().any(|t| match t {
            UnaryTest::Any => false,
            UnaryTest::Cmp(_, e) => e.uses_name(n),
            UnaryTest::Expr { expr, .. } => expr.uses_name(n),
        })
    }
}
