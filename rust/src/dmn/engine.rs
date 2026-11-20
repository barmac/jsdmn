//! Compilation and evaluation of DMN models.

use super::decision_table::{Aggregation, DecisionTable, DtInput, DtOutput, DtRule, HitPolicy};
use super::xml::*;
use crate::feel::ast::{
    Args, ContextEntry, Expr, FnKind, FunctionDef, Iteration, TypeRef, UnaryTest, UnaryTests,
};
use crate::feel::eval::{Closure, Function, NativeFunction, Scope, eval};
use crate::feel::lexer::normalize_name;
use crate::feel::parser::{ParseEnv, parse_expression, parse_type_str, parse_unary_tests};
use crate::types::{self, DmnType, NamedType};
use crate::value::{Context, Value};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Ref {
    pub model: usize,
    pub idx: usize,
}

pub enum ElementKind {
    Input {
        ty: Option<TypeRef>,
    },
    Decision {
        ty: Option<TypeRef>,
        info: Vec<Ref>,
        logic: Option<Expr>,
    },
    Bkm {
        def: Option<Rc<FunctionDef>>,
    },
    Service {
        outputs: Vec<Ref>,
        input_decisions: Vec<Ref>,
        input_data: Vec<Ref>,
        ty: Option<TypeRef>,
    },
}

pub struct Element {
    pub id: String,
    pub name: Rc<str>,
    pub kind: ElementKind,
}

pub struct Model {
    pub name: String,
    pub namespace: String,
    pub imports: Vec<RawImport>,
    pub types: HashMap<String, Rc<DmnType>>,
    pub elements: Vec<Element>,
}

impl Model {
    pub fn find(&self, name: &str) -> Option<usize> {
        let n = normalize_name(name);
        self.elements.iter().position(|e| *e.name == *n)
    }
}

struct ModelEnv<'a> {
    names: &'a HashSet<String>,
    types: &'a HashMap<String, Rc<DmnType>>,
}

impl ParseEnv for ModelEnv<'_> {
    fn is_known_name(&self, n: &str) -> bool {
        self.names.contains(n)
    }
    fn resolve_type(&self, n: &str) -> Option<Rc<DmnType>> {
        if let Some(t) = self.types.get(n) {
            return Some(t.clone());
        }
        if let Some(t) = types::builtin_type(n) {
            return Some(Rc::new(t));
        }
        None
    }
}

fn collect_names_expr(e: &RawExpr, out: &mut HashSet<String>) {
    match &e.kind {
        RawExprKind::Context(entries) => {
            for ce in entries {
                if let Some(n) = &ce.name {
                    out.insert(normalize_name(n));
                }
                if let Some(v) = &ce.value {
                    collect_names_expr(v, out);
                }
            }
        }
        RawExprKind::DecisionTable(dt) => {
            for o in &dt.outputs {
                if let Some(n) = &o.name {
                    out.insert(normalize_name(n));
                }
            }
        }
        RawExprKind::Invocation { callee, bindings } => {
            collect_names_expr(callee, out);
            for (n, b) in bindings {
                out.insert(normalize_name(n));
                if let Some(b) = b {
                    collect_names_expr(b, out);
                }
            }
        }
        RawExprKind::FunctionDef { params, body, .. } => {
            for (p, _) in params {
                out.insert(normalize_name(p));
            }
            if let Some(b) = body {
                collect_names_expr(b, out);
            }
        }
        RawExprKind::Relation { columns, rows } => {
            for (c, _) in columns {
                out.insert(normalize_name(c));
            }
            for r in rows {
                for x in r.iter().flatten() {
                    collect_names_expr(x, out);
                }
            }
        }
        RawExprKind::List(items) => items.iter().for_each(|i| collect_names_expr(i, out)),
        RawExprKind::Conditional {
            cond,
            then,
            otherwise,
        } => {
            collect_names_expr(cond, out);
            collect_names_expr(then, out);
            collect_names_expr(otherwise, out);
        }
        RawExprKind::Filter { input, matcher } => {
            collect_names_expr(input, out);
            collect_names_expr(matcher, out);
        }
        RawExprKind::For { var, input, ret } => {
            out.insert(normalize_name(var));
            collect_names_expr(input, out);
            collect_names_expr(ret, out);
        }
        RawExprKind::Quantified {
            var, input, cond, ..
        } => {
            out.insert(normalize_name(var));
            collect_names_expr(input, out);
            collect_names_expr(cond, out);
        }
        _ => {}
    }
}

fn collect_names_item(i: &RawItemDef, out: &mut HashSet<String>) {
    out.insert(normalize_name(&i.name));
    for c in &i.components {
        collect_names_item(c, out);
    }
    if let Some((params, _)) = &i.function_item {
        for (p, _) in params {
            out.insert(normalize_name(p));
        }
    }
}

fn collect_names(d: &RawDefinitions, out: &mut HashSet<String>) {
    for i in &d.item_defs {
        collect_names_item(i, out);
    }
    for e in &d.drg {
        out.insert(normalize_name(e.name()));
        match e {
            RawDrg::Decision { logic: Some(l), .. } | RawDrg::Bkm { logic: Some(l), .. } => {
                collect_names_expr(l, out)
            }
            _ => {}
        }
    }
}

fn resolve_typeref(s: &Option<String>, env: &ModelEnv) -> Option<TypeRef> {
    let s = s.as_ref()?.trim();
    if s.is_empty() {
        return None;
    }
    let n = normalize_name(s);
    if let Some(t) = env.resolve_type(&n) {
        return Some(t);
    }
    if let Ok(t) = parse_type_str(s, env)
        && !matches!(&*t, DmnType::Unknown(_))
    {
        return Some(t);
    }
    // qualified with an XML namespace prefix (e.g. "tns:tPerson")
    if let Some(i) = n.find(':')
        && let Some(t) = env.resolve_type(&n[i + 1..])
    {
        return Some(t);
    }
    Some(Rc::new(DmnType::Unknown(n)))
}

fn parse_ut(s: &str, env: &ModelEnv) -> UnaryTests {
    match parse_unary_tests(s, env) {
        Ok(u) => u,
        Err(e) => UnaryTests {
            tests: vec![UnaryTest::Expr {
                expr: Expr::Error(e.0),
                uses_qmark: true,
            }],
            negated: false,
        },
    }
}

fn parse_feel(s: &str, env: &ModelEnv) -> Expr {
    match parse_expression(s, env) {
        Ok(e) => e,
        Err(e) => Expr::Error(e.0),
    }
}

fn null_expr() -> Expr {
    Expr::Const(Value::Null)
}

fn compile_expr(raw: &RawExpr, env: &ModelEnv) -> Expr {
    let e = compile_expr_inner(raw, env);
    if matches!(raw.kind, RawExprKind::FunctionDef { .. }) {
        return e;
    }
    match resolve_typeref(&raw.type_ref, env) {
        Some(t) if !matches!(&*t, DmnType::Unknown(_) | DmnType::Any) => {
            Expr::Coerce(Box::new(e), t)
        }
        _ => e,
    }
}

fn compile_expr_inner(raw: &RawExpr, env: &ModelEnv) -> Expr {
    match &raw.kind {
        RawExprKind::Literal(Some(t)) => parse_feel(t, env),
        RawExprKind::Literal(None) => null_expr(),
        RawExprKind::UnaryTests(_) => null_expr(),
        RawExprKind::Context(entries) => Expr::Context(
            entries
                .iter()
                .map(|ce| {
                    let e = ce
                        .value
                        .as_ref()
                        .map(|v| compile_expr(v, env))
                        .unwrap_or_else(null_expr);
                    let e = match resolve_typeref(&ce.type_ref, env) {
                        Some(t) => Expr::Coerce(Box::new(e), t),
                        None => e,
                    };
                    ContextEntry {
                        name: ce
                            .name
                            .as_ref()
                            .map(|n| Rc::from(normalize_name(n).as_str())),
                        expr: e,
                    }
                })
                .collect(),
        ),
        RawExprKind::DecisionTable(dt) => Expr::DecisionTable(Rc::new(compile_dt(dt, env))),
        RawExprKind::Invocation { callee, bindings } => {
            let f = compile_expr(callee, env);
            let args = bindings
                .iter()
                .map(|(n, b)| {
                    (
                        Rc::from(normalize_name(n).as_str()),
                        b.as_ref()
                            .map(|b| compile_expr(b, env))
                            .unwrap_or_else(null_expr),
                    )
                })
                .collect();
            Expr::Call(Box::new(f), Args::Named(args))
        }
        RawExprKind::FunctionDef { params, kind, body } => {
            let params: Vec<(Rc<str>, Option<TypeRef>)> = params
                .iter()
                .map(|(n, t)| {
                    (
                        Rc::from(normalize_name(n).as_str()),
                        resolve_typeref(t, env),
                    )
                })
                .collect();
            let body_expr = body
                .as_ref()
                .map(|b| compile_expr(b, env))
                .unwrap_or_else(null_expr);
            let (kind, body_expr) = match kind.as_str() {
                "Java" => (
                    FnKind::External,
                    Expr::Context(vec![ContextEntry {
                        name: Some(Rc::from("java")),
                        expr: body_expr,
                    }]),
                ),
                "PMML" => (FnKind::Feel, Expr::Error("PMML not supported".into())),
                _ => (FnKind::Feel, body_expr),
            };
            Expr::FunctionDef(Rc::new(FunctionDef {
                params,
                body: body_expr,
                kind,
                result_type: resolve_typeref(&raw.type_ref, env),
            }))
        }
        RawExprKind::Relation { columns, rows } => Expr::List(
            rows.iter()
                .map(|r| {
                    Expr::Context(
                        columns
                            .iter()
                            .enumerate()
                            .map(|(i, (c, t))| {
                                let e = r
                                    .get(i)
                                    .and_then(|x| x.as_ref())
                                    .map(|x| compile_expr(x, env))
                                    .unwrap_or_else(null_expr);
                                let e = match resolve_typeref(t, env) {
                                    Some(t) => Expr::Coerce(Box::new(e), t),
                                    None => e,
                                };
                                ContextEntry {
                                    name: Some(Rc::from(normalize_name(c).as_str())),
                                    expr: e,
                                }
                            })
                            .collect(),
                    )
                })
                .collect(),
        ),
        RawExprKind::List(items) => {
            Expr::List(items.iter().map(|i| compile_expr(i, env)).collect())
        }
        RawExprKind::Conditional {
            cond,
            then,
            otherwise,
        } => Expr::Strict(Box::new(Expr::If(
            Box::new(compile_expr(cond, env)),
            Box::new(compile_expr(then, env)),
            Box::new(compile_expr(otherwise, env)),
        ))),
        RawExprKind::Filter { input, matcher } => Expr::Strict(Box::new(Expr::Filter(
            Box::new(compile_expr(input, env)),
            Box::new(compile_expr(matcher, env)),
        ))),
        RawExprKind::For { var, input, ret } => Expr::For(
            vec![(
                Rc::from(normalize_name(var).as_str()),
                Iteration::Expr(compile_expr(input, env)),
            )],
            Box::new(compile_expr(ret, env)),
        ),
        RawExprKind::Quantified {
            every,
            var,
            input,
            cond,
        } => Expr::Strict(Box::new(Expr::Quantified {
            every: *every,
            iters: vec![(
                Rc::from(normalize_name(var).as_str()),
                compile_expr(input, env),
            )],
            cond: Box::new(compile_expr(cond, env)),
        })),
        RawExprKind::Unsupported(k) => Expr::Error(format!("unsupported expression <{}>", k)),
    }
}

fn compile_dt(dt: &RawDecisionTable, env: &ModelEnv) -> DecisionTable {
    let hit_policy = match dt.hit_policy.as_str() {
        "FIRST" => HitPolicy::First,
        "PRIORITY" => HitPolicy::Priority,
        "ANY" => HitPolicy::Any,
        "COLLECT" => HitPolicy::Collect,
        "RULE ORDER" => HitPolicy::RuleOrder,
        "OUTPUT ORDER" => HitPolicy::OutputOrder,
        _ => HitPolicy::Unique,
    };
    let aggregation = match dt.aggregation.as_deref() {
        Some("SUM") => Some(Aggregation::Sum),
        Some("COUNT") => Some(Aggregation::Count),
        Some("MIN") => Some(Aggregation::Min),
        Some("MAX") => Some(Aggregation::Max),
        _ => None,
    };
    DecisionTable {
        hit_policy,
        aggregation,
        inputs: dt
            .inputs
            .iter()
            .map(|i| DtInput {
                expr: parse_feel(&i.expr, env),
                values: i.input_values.as_ref().map(|v| parse_ut(v, env)),
                ty: resolve_typeref(&i.type_ref, env),
            })
            .collect(),
        outputs: dt
            .outputs
            .iter()
            .map(|o| DtOutput {
                name: o
                    .name
                    .as_ref()
                    .map(|n| Rc::from(normalize_name(n).as_str())),
                values: o.output_values.as_ref().map(|v| parse_ut(v, env)),
                ty: resolve_typeref(&o.type_ref, env),
                default: o.default.as_ref().map(|d| parse_feel(d, env)),
            })
            .collect(),
        rules: dt
            .rules
            .iter()
            .map(|(ins, outs)| DtRule {
                inputs: ins.iter().map(|t| parse_ut(t, env)).collect(),
                outputs: outs.iter().map(|t| parse_feel(t, env)).collect(),
            })
            .collect(),
    }
}

/// Build the type of an item definition (or component).
fn build_item_type(
    item: &RawItemDef,
    named: &Rc<NamedType>,
    env: &ModelEnv,
    names: &HashSet<String>,
) {
    let base: Rc<DmnType> = if !item.components.is_empty() {
        let comps = item
            .components
            .iter()
            .map(|c| {
                let nt = NamedType::new(&c.name);
                build_item_type(c, &nt, env, names);
                let t: Rc<DmnType> = Rc::new(DmnType::Named(nt));
                (Rc::from(normalize_name(&c.name).as_str()), t)
            })
            .collect();
        Rc::new(DmnType::Context(comps))
    } else if let Some((params, out)) = &item.function_item {
        let ps = params
            .iter()
            .map(|(_, t)| resolve_typeref(t, env).unwrap_or_else(|| Rc::new(DmnType::Any)))
            .collect();
        let r = resolve_typeref(out, env).unwrap_or_else(|| Rc::new(DmnType::Any));
        Rc::new(DmnType::Function(ps, r))
    } else {
        match resolve_typeref(&item.type_ref, env) {
            Some(t) => match &*t {
                DmnType::Unknown(_) => Rc::new(DmnType::Any),
                _ => t,
            },
            None => Rc::new(DmnType::Any),
        }
    };
    let base = if item.is_collection {
        Rc::new(DmnType::List(base))
    } else {
        base
    };
    *named.base.borrow_mut() = base;
    named.collection.set(item.is_collection);
    if let Some(av) = &item.allowed_values {
        *named.allowed_values.borrow_mut() = Some(Rc::new(parse_ut(av, env)));
    }
    if let Some(tc) = &item.type_constraint {
        *named.type_constraint.borrow_mut() = Some(Rc::new(parse_ut(tc, env)));
    }
    let _ = names;
}

/// Unresolved hrefs of an element: (information / outputs, input decisions, input data, unused).
type PendingRefs = (Vec<String>, Vec<String>, Vec<String>, Vec<String>);

pub struct Engine {
    pub models: Rc<Vec<Rc<Model>>>,
}

fn split_href(h: &str) -> (&str, &str) {
    match h.find('#') {
        Some(i) => (&h[..i], &h[i + 1..]),
        None => ("", h),
    }
}

impl Engine {
    pub fn new(raws: Vec<RawDefinitions>) -> Engine {
        let mut names = HashSet::new();
        for r in &raws {
            collect_names(r, &mut names);
        }
        // phase 1: type shells
        let shells: Vec<Vec<(String, Rc<NamedType>)>> = raws
            .iter()
            .map(|r| {
                r.item_defs
                    .iter()
                    .map(|i| (normalize_name(&i.name), NamedType::new(&i.name)))
                    .collect()
            })
            .collect();
        let ns_index = |ns: &str| raws.iter().position(|r| r.namespace == ns);
        let mut registries: Vec<HashMap<String, Rc<DmnType>>> = Vec::new();
        for (mi, r) in raws.iter().enumerate() {
            let mut reg: HashMap<String, Rc<DmnType>> = HashMap::new();
            // imported types first so that local definitions win
            for imp in &r.imports {
                if let Some(j) = ns_index(&imp.namespace) {
                    for (n, t) in &shells[j] {
                        let key = if imp.name.is_empty() {
                            n.clone()
                        } else {
                            format!("{}.{}", normalize_name(&imp.name), n)
                        };
                        reg.insert(key, Rc::new(DmnType::Named(t.clone())));
                    }
                }
            }
            for (n, t) in &shells[mi] {
                reg.insert(n.clone(), Rc::new(DmnType::Named(t.clone())));
            }
            registries.push(reg);
        }
        // phase 2: link item definitions
        for (mi, r) in raws.iter().enumerate() {
            let env = ModelEnv {
                names: &names,
                types: &registries[mi],
            };
            for (item, (_, shell)) in r.item_defs.iter().zip(shells[mi].iter()) {
                build_item_type(item, shell, &env, &names);
            }
        }
        // phase 3: compile elements
        let mut models: Vec<Model> = Vec::new();
        let mut pending_refs: Vec<Vec<PendingRefs>> = Vec::new();
        for (mi, r) in raws.iter().enumerate() {
            let env = ModelEnv {
                names: &names,
                types: &registries[mi],
            };
            let mut elements = Vec::new();
            let mut refs = Vec::new();
            for e in &r.drg {
                let name: Rc<str> = Rc::from(normalize_name(e.name()).as_str());
                let (kind, rf) = match e {
                    RawDrg::InputData { type_ref, .. } => (
                        ElementKind::Input {
                            ty: resolve_typeref(type_ref, &env),
                        },
                        (vec![], vec![], vec![], vec![]),
                    ),
                    RawDrg::Decision {
                        type_ref,
                        info_reqs,
                        logic,
                        ..
                    } => {
                        let info: Vec<String> = info_reqs
                            .iter()
                            .map(|q| match q {
                                RawReq::Input(h) | RawReq::Decision(h) => h.clone(),
                            })
                            .collect();
                        (
                            ElementKind::Decision {
                                ty: resolve_typeref(type_ref, &env),
                                info: vec![],
                                logic: logic.as_ref().map(|l| compile_expr(l, &env)),
                            },
                            (info, vec![], vec![], vec![]),
                        )
                    }
                    RawDrg::Bkm {
                        type_ref, logic, ..
                    } => {
                        let var_ty = resolve_typeref(type_ref, &env)
                            .map(|t| types::function_return(&t).unwrap_or(t));
                        let def = logic.as_ref().and_then(|l| match compile_expr(l, &env) {
                            Expr::FunctionDef(d) => {
                                if (d.result_type.is_none() || var_ty.is_some())
                                    && let Some(t) = var_ty.clone()
                                {
                                    let fd = Rc::try_unwrap(d).unwrap_or_else(|_| unreachable!());
                                    return Some(Rc::new(FunctionDef {
                                        result_type: Some(t),
                                        ..fd
                                    }));
                                }
                                Some(d)
                            }
                            _ => None,
                        });
                        (ElementKind::Bkm { def }, (vec![], vec![], vec![], vec![]))
                    }
                    RawDrg::DecisionService {
                        outputs,
                        input_decisions,
                        input_data,
                        type_ref,
                        ..
                    } => (
                        ElementKind::Service {
                            outputs: vec![],
                            input_decisions: vec![],
                            input_data: vec![],
                            ty: resolve_typeref(type_ref, &env)
                                .map(|t| types::function_return(&t).unwrap_or(t)),
                        },
                        (
                            outputs.clone(),
                            input_decisions.clone(),
                            input_data.clone(),
                            vec![],
                        ),
                    ),
                };
                elements.push(Element {
                    id: e.id().to_string(),
                    name,
                    kind,
                });
                refs.push(rf);
            }
            models.push(Model {
                name: r.name.clone(),
                namespace: r.namespace.clone(),
                imports: r.imports.clone(),
                types: registries[mi].clone(),
                elements,
            });
            pending_refs.push(refs);
        }
        // phase 4: resolve references
        let resolve = |models: &Vec<Model>, mi: usize, h: &str| -> Option<Ref> {
            let (ns, id) = split_href(h);
            let m = if ns.is_empty() {
                mi
            } else {
                models.iter().position(|m| m.namespace == ns)?
            };
            let idx = models[m].elements.iter().position(|e| e.id == id)?;
            Some(Ref { model: m, idx })
        };
        #[allow(clippy::needless_range_loop)]
        for mi in 0..models.len() {
            for ei in 0..models[mi].elements.len() {
                let (a, b, c, _) = pending_refs[mi][ei].clone();
                let ra: Vec<Ref> = a.iter().filter_map(|h| resolve(&models, mi, h)).collect();
                let rb: Vec<Ref> = b.iter().filter_map(|h| resolve(&models, mi, h)).collect();
                let rc: Vec<Ref> = c.iter().filter_map(|h| resolve(&models, mi, h)).collect();
                match &mut models[mi].elements[ei].kind {
                    ElementKind::Decision { info, .. } => *info = ra,
                    ElementKind::Service {
                        outputs,
                        input_decisions,
                        input_data,
                        ..
                    } => {
                        *outputs = ra;
                        *input_decisions = rb;
                        *input_data = rc;
                    }
                    _ => {}
                }
            }
        }
        Engine {
            models: Rc::new(models.into_iter().map(Rc::new).collect()),
        }
    }

    pub fn from_xml(sources: &[String]) -> Result<Engine, String> {
        let raws = sources
            .iter()
            .map(|s| read_definitions(s))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Engine::new(raws))
    }

    pub fn model_index(&self, ns_or_name: &str) -> Option<usize> {
        self.models
            .iter()
            .position(|m| m.namespace == ns_or_name)
            .or_else(|| self.models.iter().position(|m| m.name == ns_or_name))
    }

    pub fn session(&self) -> Session {
        Session::new(self.models.clone())
    }
}

/// Evaluation session: inputs plus memoized decision results.
pub struct Session {
    pub models: Rc<Vec<Rc<Model>>>,
    inputs: HashMap<(usize, Rc<str>), Value>,
    cache: RefCell<HashMap<Ref, Value>>,
    roots: RefCell<HashMap<usize, Rc<Scope>>>,
    active: RefCell<HashSet<Ref>>,
}

struct ServiceFn {
    models: Rc<Vec<Rc<Model>>>,
    r: Ref,
}

impl NativeFunction for ServiceFn {
    fn param_names(&self) -> Vec<Rc<str>> {
        let el = &self.models[self.r.model].elements[self.r.idx];
        match &el.kind {
            ElementKind::Service {
                input_decisions,
                input_data,
                ..
            } => input_data
                .iter()
                .chain(input_decisions.iter())
                .map(|r| self.models[r.model].elements[r.idx].name.clone())
                .collect(),
            _ => vec![],
        }
    }

    fn invoke(&self, args: Vec<Value>) -> Value {
        let el = &self.models[self.r.model].elements[self.r.idx];
        let (input_decisions, input_data) = match &el.kind {
            ElementKind::Service {
                input_decisions,
                input_data,
                ..
            } => (input_decisions, input_data),
            _ => return Value::Null,
        };
        let mut session = Session::new(self.models.clone());
        let params: Vec<Ref> = input_data
            .iter()
            .chain(input_decisions.iter())
            .copied()
            .collect();
        for (r, v) in params.iter().zip(args) {
            let v = match &self.models[r.model].elements[r.idx].kind {
                ElementKind::Input { ty: Some(t) } | ElementKind::Decision { ty: Some(t), .. } => {
                    let c = types::coerce(&v, t);
                    if c.is_null() && !v.is_null() {
                        return Value::Null;
                    }
                    c
                }
                _ => v,
            };
            session.cache.borrow_mut().insert(*r, v.clone());
            session.inputs.insert(
                (r.model, self.models[r.model].elements[r.idx].name.clone()),
                v,
            );
        }
        session.evaluate_service_ref(self.r, true)
    }
}

impl Session {
    fn new(models: Rc<Vec<Rc<Model>>>) -> Session {
        Session {
            models,
            inputs: HashMap::new(),
            cache: RefCell::new(HashMap::new()),
            roots: RefCell::new(HashMap::new()),
            active: RefCell::new(HashSet::new()),
        }
    }

    pub fn set_input(&mut self, model: usize, name: &str, v: Value) {
        self.inputs
            .insert((model, Rc::from(normalize_name(name).as_str())), v);
    }

    fn import_prefix(&self, from: usize, target: usize) -> Option<String> {
        let ns = &self.models[target].namespace;
        self.models[from]
            .imports
            .iter()
            .find(|i| &i.namespace == ns)
            .map(|i| normalize_name(&i.name))
    }

    /// Bind a value for element `r` into `scope` as seen from model `from`.
    fn bind(&self, scope: &Rc<Scope>, from: usize, r: Ref, v: Value) {
        let name = self.models[r.model].elements[r.idx].name.clone();
        if r.model == from {
            scope.set(&name, v);
            return;
        }
        let path = self.import_path(from, r.model).unwrap_or_default();
        if path.is_empty() {
            scope.set(&name, v);
            return;
        }
        // nested contexts along the prefix path
        let existing = scope.lookup_local(&path[0]);
        let mut ctx = match existing {
            Some(Value::Context(c)) => c.as_ref().clone(),
            _ => Context::new(),
        };
        fn put_path(ctx: &mut Context, path: &[String], name: &str, v: Value) {
            if path.is_empty() {
                ctx.put(name, v);
                return;
            }
            let mut inner = match ctx.get(&path[0]) {
                Some(Value::Context(c)) => c.as_ref().clone(),
                _ => Context::new(),
            };
            put_path(&mut inner, &path[1..], name, v);
            ctx.put(path[0].as_str(), Value::context(inner));
        }
        put_path(&mut ctx, &path[1..], &name, v);
        scope.set(&path[0], Value::context(ctx));
    }

    /// Chain of import prefixes leading from model `from` to model `target`.
    fn import_path(&self, from: usize, target: usize) -> Option<Vec<String>> {
        let mut seen = HashSet::new();
        let mut queue = vec![(from, Vec::<String>::new())];
        while let Some((m, path)) = queue.pop() {
            if m == target {
                return Some(path);
            }
            if !seen.insert(m) {
                continue;
            }
            for imp in &self.models[m].imports {
                if let Some(j) = self
                    .models
                    .iter()
                    .position(|x| x.namespace == imp.namespace)
                {
                    let mut p = path.clone();
                    let n = normalize_name(&imp.name);
                    if !n.is_empty() {
                        p.push(n);
                    }
                    queue.insert(0, (j, p));
                }
            }
        }
        let _ = self.import_prefix(from, target);
        None
    }

    /// Root scope of a model: all BKMs and decision services (plus imported ones).
    fn root_scope(&self, m: usize) -> Rc<Scope> {
        if let Some(s) = self.roots.borrow().get(&m) {
            return s.clone();
        }
        let scope = Scope::root();
        self.roots.borrow_mut().insert(m, scope.clone());
        let model = self.models[m].clone();
        for (idx, el) in model.elements.iter().enumerate() {
            if let Some(v) = self.invocable_value(Ref { model: m, idx }, &scope) {
                scope.set(&el.name, v);
            }
        }
        // imported invocables
        for imp in &model.imports {
            if let Some(j) = self
                .models
                .iter()
                .position(|x| x.namespace == imp.namespace)
            {
                if j == m {
                    continue;
                }
                let other = self.root_scope(j);
                for (idx, _) in self.models[j].elements.iter().enumerate() {
                    let r = Ref { model: j, idx };
                    if matches!(
                        self.models[j].elements[idx].kind,
                        ElementKind::Bkm { .. } | ElementKind::Service { .. }
                    ) && let Some(v) = other.lookup(&self.models[j].elements[idx].name)
                    {
                        self.bind(&scope, m, r, v);
                    }
                }
            }
        }
        scope
    }

    fn invocable_value(&self, r: Ref, scope: &Rc<Scope>) -> Option<Value> {
        let el = &self.models[r.model].elements[r.idx];
        match &el.kind {
            ElementKind::Bkm { def } => {
                let def = def.clone().unwrap_or_else(|| {
                    Rc::new(FunctionDef {
                        params: vec![],
                        body: Expr::Const(Value::Null),
                        kind: FnKind::Feel,
                        result_type: None,
                    })
                });
                Some(Value::Function(Rc::new(Function::Closure(Closure {
                    def,
                    scope: scope.clone(),
                    name: RefCell::new(Some(el.name.clone())),
                }))))
            }
            ElementKind::Service { .. } => Some(Value::Function(Rc::new(Function::Native(
                Rc::new(ServiceFn {
                    models: self.models.clone(),
                    r,
                }),
            )))),
            _ => None,
        }
    }

    pub fn value_of(&self, r: Ref) -> Value {
        if let Some(v) = self.cache.borrow().get(&r) {
            return v.clone();
        }
        let el = &self.models[r.model].elements[r.idx];
        let v = match &el.kind {
            ElementKind::Input { ty } => {
                let v = self
                    .inputs
                    .get(&(r.model, el.name.clone()))
                    .cloned()
                    .unwrap_or(Value::Null);
                match ty {
                    Some(t) => types::coerce(&v, t),
                    None => v,
                }
            }
            ElementKind::Decision { .. } => {
                if let Some(v) = self.inputs.get(&(r.model, el.name.clone())) {
                    v.clone()
                } else {
                    self.evaluate_decision_ref(r)
                }
            }
            ElementKind::Bkm { .. } | ElementKind::Service { .. } => {
                let root = self.root_scope(r.model);
                root.lookup(&el.name).unwrap_or(Value::Null)
            }
        };
        self.cache.borrow_mut().insert(r, v.clone());
        v
    }

    fn evaluate_decision_ref(&self, r: Ref) -> Value {
        if !self.active.borrow_mut().insert(r) {
            return Value::Null; // cycle
        }
        let el = &self.models[r.model].elements[r.idx];
        let result = match &el.kind {
            ElementKind::Decision { ty, info, logic } => {
                let scope = Scope::child(&self.root_scope(r.model));
                // inputs that do not correspond to any model element are free variables
                for ((m, name), v) in &self.inputs {
                    if *m == r.model && self.models[*m].find(name).is_none() {
                        scope.set(name, v.clone());
                    }
                }
                for req in info {
                    let v = self.value_of(*req);
                    self.bind(&scope, r.model, *req, v);
                }
                let v = match logic {
                    Some(e) => eval(e, &scope),
                    None => Value::Null,
                };
                match ty {
                    Some(t) => types::coerce(&v, t),
                    None => v,
                }
            }
            _ => Value::Null,
        };
        self.active.borrow_mut().remove(&r);
        result
    }

    fn evaluate_service_ref(&self, r: Ref, _direct: bool) -> Value {
        let el = &self.models[r.model].elements[r.idx];
        match &el.kind {
            ElementKind::Service { outputs, ty, .. } => {
                let v = if outputs.len() == 1 {
                    self.value_of(outputs[0])
                } else {
                    let mut c = Context::new();
                    for o in outputs {
                        let v = self.value_of(*o);
                        c.put(self.models[o.model].elements[o.idx].name.clone(), v);
                    }
                    Value::context(c)
                };
                match ty {
                    Some(t) => types::coerce(&v, t),
                    None => v,
                }
            }
            _ => Value::Null,
        }
    }

    /// Evaluate a decision by name.
    pub fn evaluate_decision(&self, model: usize, name: &str) -> Option<Value> {
        let idx = self.models[model].find(name)?;
        Some(self.value_of(Ref { model, idx }))
    }

    /// Invoke a decision service by name with the session inputs as parameters.
    pub fn evaluate_service(&self, model: usize, name: &str) -> Option<Value> {
        let idx = self.models[model].find(name)?;
        let r = Ref { model, idx };
        let f = ServiceFn {
            models: self.models.clone(),
            r,
        };
        let names = f.param_names();
        let args: Vec<Value> = names
            .iter()
            .map(|n| {
                // parameters may come from any model
                self.inputs
                    .iter()
                    .find(|((_, k), _)| k == n)
                    .map(|(_, v)| v.clone())
                    .unwrap_or(Value::Null)
            })
            .collect();
        Some(f.invoke(args))
    }

    /// Invoke a BKM by name using the session inputs as (named) arguments.
    pub fn invoke_bkm(&self, model: usize, name: &str) -> Option<Value> {
        let idx = self.models[model].find(name)?;
        let f = self.value_of(Ref { model, idx });
        let f = match f {
            Value::Function(f) => f,
            _ => return None,
        };
        let params: Vec<Rc<str>> = match &*f {
            Function::Closure(c) => c.def.params.iter().map(|(p, _)| p.clone()).collect(),
            _ => vec![],
        };
        let args = params
            .iter()
            .map(|p| {
                self.inputs
                    .iter()
                    .find(|((_, k), _)| k == p)
                    .map(|(_, v)| v.clone())
                    .unwrap_or(Value::Null)
            })
            .collect();
        Some(crate::feel::eval::call(&f, args, &Scope::root()))
    }

    /// Value of a BKM / any element by name.
    pub fn element_value(&self, model: usize, name: &str) -> Option<Value> {
        let idx = self.models[model].find(name)?;
        Some(self.value_of(Ref { model, idx }))
    }
}

impl Scope {
    pub fn lookup_local(&self, name: &str) -> Option<Value> {
        self.lookup(name)
    }
}
