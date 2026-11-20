//! Decision table evaluation.

use crate::feel::ast::{Expr, TypeRef, UnaryTest, UnaryTests};
use crate::feel::eval::{Scope, eval, eval_unary_tests};
use crate::number::Number;
use crate::types;
use crate::value::{Context, Value, feel_cmp, feel_eq};
use std::cmp::Ordering;
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitPolicy {
    Unique,
    Any,
    Priority,
    First,
    RuleOrder,
    OutputOrder,
    Collect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Aggregation {
    Sum,
    Count,
    Min,
    Max,
}

#[derive(Debug)]
pub struct DtInput {
    pub expr: Expr,
    pub values: Option<UnaryTests>,
    pub ty: Option<TypeRef>,
}

#[derive(Debug)]
pub struct DtOutput {
    pub name: Option<Rc<str>>,
    pub values: Option<UnaryTests>,
    pub ty: Option<TypeRef>,
    pub default: Option<Expr>,
}

#[derive(Debug)]
pub struct DtRule {
    pub inputs: Vec<UnaryTests>,
    pub outputs: Vec<Expr>,
}

#[derive(Debug)]
pub struct DecisionTable {
    pub hit_policy: HitPolicy,
    pub aggregation: Option<Aggregation>,
    pub inputs: Vec<DtInput>,
    pub outputs: Vec<DtOutput>,
    pub rules: Vec<DtRule>,
}

impl DecisionTable {
    fn output_values(&self, i: usize, scope: &Rc<Scope>) -> Vec<Value> {
        match &self.outputs[i].values {
            Some(ut) => ut
                .tests
                .iter()
                .filter_map(|t| match t {
                    UnaryTest::Expr { expr, .. } => Some(eval(expr, scope)),
                    _ => None,
                })
                .collect(),
            None => Vec::new(),
        }
    }

    fn check_output(&self, i: usize, v: Value, scope: &Rc<Scope>) -> Value {
        let o = &self.outputs[i];
        let v = match &o.ty {
            Some(t) => types::coerce(&v, t),
            None => v,
        };
        if let Some(ut) = &o.values
            && !v.is_null()
            && !matches!(eval_unary_tests(ut, &v, scope), Value::Bool(true))
        {
            return Value::Null;
        }
        v
    }

    fn rule_output(&self, rule: &DtRule, scope: &Rc<Scope>) -> Vec<Value> {
        rule.outputs
            .iter()
            .enumerate()
            .map(|(i, e)| self.check_output(i, eval(e, scope), scope))
            .collect()
    }

    fn pack(&self, vals: Vec<Value>) -> Value {
        if self.outputs.len() == 1 {
            return vals.into_iter().next().unwrap_or(Value::Null);
        }
        let mut c = Context::new();
        for (o, v) in self.outputs.iter().zip(vals) {
            c.put(o.name.clone().unwrap_or_else(|| Rc::from("")), v);
        }
        Value::context(c)
    }

    fn priority_key(&self, vals: &[Value], prio: &[Vec<Value>]) -> Vec<usize> {
        vals.iter()
            .enumerate()
            .map(|(i, v)| {
                prio[i]
                    .iter()
                    .position(|p| feel_eq(p, v) == Some(true))
                    .unwrap_or(usize::MAX)
            })
            .collect()
    }

    pub fn evaluate(&self, scope: &Rc<Scope>) -> Value {
        // evaluate inputs
        let mut input_values = Vec::with_capacity(self.inputs.len());
        for inp in &self.inputs {
            let mut v = eval(&inp.expr, scope);
            if let Some(t) = &inp.ty {
                let c = types::coerce(&v, t);
                if !v.is_null() && c.is_null() {
                    return Value::Null;
                }
                v = c;
            }
            if let Some(ut) = &inp.values
                && !matches!(eval_unary_tests(ut, &v, scope), Value::Bool(true))
            {
                return Value::Null;
            }
            input_values.push(v);
        }
        // match rules
        // FIRST needs only the first match, UNIQUE fails as soon as a second one is found
        let stop_after = match self.hit_policy {
            HitPolicy::First => 1,
            HitPolicy::Unique => 2,
            _ => usize::MAX,
        };
        let null = Value::Null;
        let mut matched: Vec<&DtRule> = Vec::new();
        'rules: for rule in &self.rules {
            for (i, ut) in rule.inputs.iter().enumerate() {
                let x = input_values.get(i).unwrap_or(&null);
                if !matches!(eval_unary_tests(ut, x, scope), Value::Bool(true)) {
                    continue 'rules;
                }
            }
            matched.push(rule);
            if matched.len() >= stop_after {
                break;
            }
        }
        if matched.is_empty() {
            if self.outputs.iter().any(|o| o.default.is_some()) {
                let vals: Vec<Value> = self
                    .outputs
                    .iter()
                    .enumerate()
                    .map(|(i, o)| match &o.default {
                        Some(e) => self.check_output(i, eval(e, scope), scope),
                        None => Value::Null,
                    })
                    .collect();
                return self.pack(vals);
            }
            return match self.hit_policy {
                HitPolicy::Collect => match self.aggregation {
                    Some(Aggregation::Count) => Value::int(0),
                    Some(_) => Value::Null,
                    None => Value::list(vec![]),
                },
                HitPolicy::RuleOrder | HitPolicy::OutputOrder => Value::list(vec![]),
                _ => Value::Null,
            };
        }
        let results: Vec<Vec<Value>> = matched.iter().map(|r| self.rule_output(r, scope)).collect();
        match self.hit_policy {
            HitPolicy::Unique => {
                if results.len() > 1 {
                    return Value::Null;
                }
                self.pack(results.into_iter().next().unwrap())
            }
            HitPolicy::First => self.pack(results.into_iter().next().unwrap()),
            HitPolicy::Any => {
                let first = &results[0];
                for r in &results[1..] {
                    for (a, b) in first.iter().zip(r.iter()) {
                        if feel_eq(a, b) != Some(true) {
                            return Value::Null;
                        }
                    }
                }
                self.pack(results.into_iter().next().unwrap())
            }
            HitPolicy::Priority | HitPolicy::OutputOrder => {
                let prio: Vec<Vec<Value>> = (0..self.outputs.len())
                    .map(|i| self.output_values(i, scope))
                    .collect();
                let mut keyed: Vec<(Vec<usize>, Vec<Value>)> = results
                    .into_iter()
                    .map(|r| (self.priority_key(&r, &prio), r))
                    .collect();
                keyed.sort_by(|a, b| a.0.cmp(&b.0));
                if self.hit_policy == HitPolicy::Priority {
                    self.pack(keyed.into_iter().next().unwrap().1)
                } else {
                    Value::list(keyed.into_iter().map(|(_, r)| self.pack(r)).collect())
                }
            }
            HitPolicy::RuleOrder => {
                Value::list(results.into_iter().map(|r| self.pack(r)).collect())
            }
            HitPolicy::Collect => {
                let packed: Vec<Value> = results.into_iter().map(|r| self.pack(r)).collect();
                match self.aggregation {
                    None => Value::list(packed),
                    Some(agg) => {
                        if self.outputs.len() > 1 {
                            return Value::Null;
                        }
                        let vals: Vec<Value> =
                            packed.into_iter().filter(|v| !v.is_null()).collect();
                        match agg {
                            Aggregation::Count => {
                                let mut distinct: Vec<Value> = Vec::new();
                                for v in vals {
                                    if !distinct.iter().any(|d| feel_eq(d, &v) == Some(true)) {
                                        distinct.push(v);
                                    }
                                }
                                Value::int(distinct.len() as i64)
                            }
                            Aggregation::Sum => {
                                if vals.is_empty() {
                                    return Value::Null;
                                }
                                let mut s = Number::zero();
                                for v in &vals {
                                    match v {
                                        Value::Number(n) => s = s.add(n),
                                        _ => return Value::Null,
                                    }
                                }
                                Value::Number(s)
                            }
                            Aggregation::Min | Aggregation::Max => {
                                if vals.is_empty() {
                                    return Value::Null;
                                }
                                let mut best = vals[0].clone();
                                for v in &vals[1..] {
                                    match feel_cmp(v, &best) {
                                        Some(o) => {
                                            if (agg == Aggregation::Min && o == Ordering::Less)
                                                || (agg == Aggregation::Max
                                                    && o == Ordering::Greater)
                                            {
                                                best = v.clone();
                                            }
                                        }
                                        None => return Value::Null,
                                    }
                                }
                                best
                            }
                        }
                    }
                }
            }
        }
    }
}
