//! WebAssembly bindings of the [rsdmn](../README.md) DMN engine.
//!
//! ```js
//! import init, { DmnEngine, evaluateFeel } from "./pkg/rsdmn_wasm.js";
//! await init();
//! const engine = new DmnEngine([dmnXml]);
//! engine.evaluate("Eligibility", { Applicant: { age: 32, income: 4000 } });
//! ```
//!
//! FEEL values are converted to plain JavaScript values: numbers become
//! `number`s, contexts objects, lists arrays and temporal values ISO 8601
//! strings (e.g. `"2021-02-28"`, `"P1DT2H"`). Inputs are converted the other
//! way round, guided by the types declared in the model: a string given for an
//! input of type `date` becomes a date, a string given for a `number` an exact
//! decimal, and so on.

use js_sys::{Array, Object, Reflect};
use rsdmn::dmn::engine::{ElementKind, Engine, Model, Session};
use rsdmn::number::Number;
use rsdmn::temporal::{self, Date, DateTime, Time, Zone};
use rsdmn::types::{DmnType, base_of};
use rsdmn::value::{Context, Value};
use std::rc::Rc;
use wasm_bindgen::prelude::*;

/// Convert a FEEL value to a plain JavaScript value.
fn to_js(v: &Value) -> JsValue {
    match v {
        Value::Null => JsValue::NULL,
        Value::Bool(b) => JsValue::from_bool(*b),
        Value::Number(n) => JsValue::from_f64(n.to_f64()),
        Value::String(s) => JsValue::from_str(s),
        Value::List(l) => {
            let a = Array::new();
            for x in l.iter() {
                a.push(&to_js(x));
            }
            a.into()
        }
        Value::Context(c) => {
            let o = Object::new();
            for (k, x) in c.entries() {
                let _ = Reflect::set(&o, &JsValue::from_str(k), &to_js(x));
            }
            o.into()
        }
        // temporal values, ranges and functions: their FEEL string form
        other => JsValue::from_str(&other.to_string()),
    }
}

fn parse_typed_string(s: &str, ty: &DmnType) -> Result<Value, String> {
    let bad = || format!("`{}` is not a valid {:?}", s, ty);
    Ok(match ty {
        DmnType::Number => Value::Number(Number::parse(s).ok_or_else(bad)?),
        DmnType::Date => Value::Date(Date::parse(s).ok_or_else(bad)?),
        DmnType::Time => Value::Time(Time::parse(s).ok_or_else(bad)?),
        DmnType::DateTime => Value::DateTime(if s.contains('T') {
            DateTime::parse(s).ok_or_else(bad)?
        } else {
            DateTime::from_date(Date::parse(s).ok_or_else(bad)?)
        }),
        DmnType::DtDuration | DmnType::YmDuration => {
            match temporal::parse_duration(s).ok_or_else(bad)? {
                temporal::Duration::Dt(d) => Value::DtDuration(d),
                temporal::Duration::Ym(d) => Value::YmDuration(d),
            }
        }
        _ => Value::str(s),
    })
}

/// Convert a JavaScript value to a FEEL value, guided by an (optional) type.
fn from_js(js: &JsValue, ty: Option<&DmnType>) -> Result<Value, String> {
    let base: Option<Rc<DmnType>> = ty.map(base_of);
    let base = base.as_deref();
    if js.is_null() || js.is_undefined() {
        return Ok(Value::Null);
    }
    if let Some(b) = js.as_bool() {
        return Ok(Value::Bool(b));
    }
    if let Some(f) = js.as_f64() {
        return Number::from_f64(f)
            .map(Value::Number)
            .ok_or_else(|| format!("{} is not a finite number", f));
    }
    if let Some(s) = js.as_string() {
        return match base {
            Some(t) => parse_typed_string(&s, t),
            None => Ok(Value::str(&s)),
        };
    }
    if let Some(d) = js.dyn_ref::<js_sys::Date>() {
        let nanos = d.get_time() as i128 * 1_000_000;
        let dt = DateTime::from_instant(nanos, Zone::Offset(0)).ok_or("invalid date")?;
        return Ok(match base {
            Some(DmnType::Date) => Value::Date(dt.date),
            _ => Value::DateTime(dt),
        });
    }
    if Array::is_array(js) {
        let elem = match base {
            Some(DmnType::List(et)) => Some(et.as_ref()),
            _ => None,
        };
        let items = Array::from(js)
            .iter()
            .map(|x| from_js(&x, elem))
            .collect::<Result<Vec<_>, _>>()?;
        return Ok(Value::list(items));
    }
    if js.is_object() {
        let components: &[(Rc<str>, Rc<DmnType>)] = match base {
            Some(DmnType::Context(c)) => c,
            _ => &[],
        };
        let mut ctx = Context::new();
        for entry in Object::entries(js.unchecked_ref::<Object>()).iter() {
            let pair = Array::from(&entry);
            let key = pair.get(0).as_string().unwrap_or_default();
            let ty = components
                .iter()
                .find(|(k, _)| **k == *key)
                .map(|(_, t)| t.as_ref());
            ctx.put(key.as_str(), from_js(&pair.get(1), ty)?);
        }
        return Ok(Value::context(ctx));
    }
    Err("unsupported JavaScript value".into())
}

fn type_name(ty: &Option<Rc<DmnType>>) -> JsValue {
    match ty {
        Some(t) => JsValue::from_str(&format!("{:?}", t)),
        None => JsValue::NULL,
    }
}

fn entry(name: &str, ty: &Option<Rc<DmnType>>) -> JsValue {
    let o = Object::new();
    let _ = Reflect::set(&o, &"name".into(), &JsValue::from_str(name));
    let _ = Reflect::set(&o, &"type".into(), &type_name(ty));
    o.into()
}

fn err(msg: impl AsRef<str>) -> JsError {
    JsError::new(msg.as_ref())
}

/// A set of loaded DMN models (a model and the models it imports).
#[wasm_bindgen]
pub struct DmnEngine {
    engine: Engine,
}

#[wasm_bindgen]
impl DmnEngine {
    /// Load DMN models from XML strings; the first one is the main model.
    #[wasm_bindgen(constructor)]
    pub fn new(models: Vec<String>) -> Result<DmnEngine, JsError> {
        Engine::from_xml(&models)
            .map(|engine| DmnEngine { engine })
            .map_err(err)
    }

    fn model(&self, model: Option<usize>) -> Result<&Rc<Model>, JsError> {
        let i = model.unwrap_or(0);
        self.engine
            .models
            .get(i)
            .ok_or_else(|| err(format!("no model with index {}", i)))
    }

    /// Names of the loaded models, in load order.
    #[wasm_bindgen(js_name = modelNames)]
    pub fn model_names(&self) -> Vec<String> {
        self.engine.models.iter().map(|m| m.name.clone()).collect()
    }

    /// Input data of a model: `[{ name, type }]`.
    #[wasm_bindgen(js_name = inputData)]
    pub fn input_data(&self, model: Option<usize>) -> Result<Array, JsError> {
        Ok(self
            .model(model)?
            .elements
            .iter()
            .filter_map(|e| match &e.kind {
                ElementKind::Input { ty } => Some(entry(&e.name, ty)),
                _ => None,
            })
            .collect())
    }

    /// Decisions of a model: `[{ name, type }]`.
    pub fn decisions(&self, model: Option<usize>) -> Result<Array, JsError> {
        Ok(self
            .model(model)?
            .elements
            .iter()
            .filter_map(|e| match &e.kind {
                ElementKind::Decision { ty, .. } => Some(entry(&e.name, ty)),
                _ => None,
            })
            .collect())
    }

    /// Decision services of a model: `[{ name, type }]`.
    #[wasm_bindgen(js_name = decisionServices)]
    pub fn decision_services(&self, model: Option<usize>) -> Result<Array, JsError> {
        Ok(self
            .model(model)?
            .elements
            .iter()
            .filter_map(|e| match &e.kind {
                ElementKind::Service { ty, .. } => Some(entry(&e.name, ty)),
                _ => None,
            })
            .collect())
    }

    fn session(&self, model: usize, inputs: &JsValue) -> Result<Session, JsError> {
        let m = self.model(Some(model))?;
        let mut session = self.engine.session();
        if inputs.is_null() || inputs.is_undefined() {
            return Ok(session);
        }
        if !inputs.is_object() || Array::is_array(inputs) {
            return Err(err(
                "inputs must be an object mapping input names to values",
            ));
        }
        for pair in Object::entries(inputs.unchecked_ref::<Object>()).iter() {
            let pair = Array::from(&pair);
            let name = pair.get(0).as_string().unwrap_or_default();
            let ty = m.find(&name).and_then(|i| match &m.elements[i].kind {
                ElementKind::Input { ty } | ElementKind::Decision { ty, .. } => ty.clone(),
                _ => None,
            });
            let value = from_js(&pair.get(1), ty.as_deref())
                .map_err(|e| err(format!("input `{}`: {}", name, e)))?;
            session.set_input(model, &name, value);
        }
        Ok(session)
    }

    /// Evaluate a decision; `inputs` maps input data (or decision) names to values.
    pub fn evaluate(
        &self,
        decision: &str,
        inputs: JsValue,
        model: Option<usize>,
    ) -> Result<JsValue, JsError> {
        let m = model.unwrap_or(0);
        let session = self.session(m, &inputs)?;
        let v = session
            .evaluate_decision(m, decision)
            .ok_or_else(|| err(format!("decision `{}` not found", decision)))?;
        Ok(to_js(&v))
    }

    /// Like `evaluate`, but returns the result in FEEL syntax (exact numbers,
    /// typed temporal values), e.g. `{"rate": 0.035, "start": date("2021-01-01")}`.
    #[wasm_bindgen(js_name = evaluateToFeel)]
    pub fn evaluate_to_feel(
        &self,
        decision: &str,
        inputs: JsValue,
        model: Option<usize>,
    ) -> Result<String, JsError> {
        let m = model.unwrap_or(0);
        let session = self.session(m, &inputs)?;
        let v = session
            .evaluate_decision(m, decision)
            .ok_or_else(|| err(format!("decision `{}` not found", decision)))?;
        Ok(format!("{:?}", v))
    }

    /// Evaluate every decision of a model: `{ decisionName: value }`.
    #[wasm_bindgen(js_name = evaluateAll)]
    pub fn evaluate_all(&self, inputs: JsValue, model: Option<usize>) -> Result<JsValue, JsError> {
        let m = model.unwrap_or(0);
        let session = self.session(m, &inputs)?;
        let o = Object::new();
        for e in self.model(Some(m))?.elements.iter() {
            if let ElementKind::Decision { .. } = e.kind {
                let v = session.evaluate_decision(m, &e.name).unwrap_or(Value::Null);
                let _ = Reflect::set(&o, &JsValue::from_str(&e.name), &to_js(&v));
            }
        }
        Ok(o.into())
    }

    /// Invoke a decision service with the given inputs.
    #[wasm_bindgen(js_name = evaluateService)]
    pub fn evaluate_service(
        &self,
        service: &str,
        inputs: JsValue,
        model: Option<usize>,
    ) -> Result<JsValue, JsError> {
        let m = model.unwrap_or(0);
        let session = self.session(m, &inputs)?;
        let v = session
            .evaluate_service(m, service)
            .ok_or_else(|| err(format!("decision service `{}` not found", service)))?;
        Ok(to_js(&v))
    }
}

fn feel_vars(vars: &JsValue) -> Result<Vec<(String, Value)>, JsError> {
    let mut out = Vec::new();
    if vars.is_null() || vars.is_undefined() {
        return Ok(out);
    }
    for pair in Object::entries(vars.unchecked_ref::<Object>()).iter() {
        let pair = Array::from(&pair);
        let name = pair.get(0).as_string().unwrap_or_default();
        out.push((name, from_js(&pair.get(1), None).map_err(err)?));
    }
    Ok(out)
}

/// Evaluate a standalone FEEL expression with the given variables.
#[wasm_bindgen(js_name = evaluateFeel)]
pub fn evaluate_feel(expression: &str, variables: JsValue) -> Result<JsValue, JsError> {
    let vars = feel_vars(&variables)?;
    let vars: Vec<(&str, Value)> = vars.iter().map(|(n, v)| (n.as_str(), v.clone())).collect();
    rsdmn::feel::evaluate(expression, &vars)
        .map(|v| to_js(&v))
        .map_err(err)
}

/// Evaluate a FEEL expression and return the result in FEEL syntax.
#[wasm_bindgen(js_name = evaluateFeelToFeel)]
pub fn evaluate_feel_to_feel(expression: &str, variables: JsValue) -> Result<String, JsError> {
    let vars = feel_vars(&variables)?;
    let vars: Vec<(&str, Value)> = vars.iter().map(|(n, v)| (n.as_str(), v.clone())).collect();
    rsdmn::feel::evaluate(expression, &vars)
        .map(|v| format!("{:?}", v))
        .map_err(err)
}

/// Run one DMN TCK test directory given as in-memory files; returns
/// `[{ file, id, success, message }]`.
#[wasm_bindgen(js_name = runTckSuite)]
pub fn run_tck_suite(
    name: &str,
    model_files: Vec<String>,
    model_sources: Vec<String>,
    test_files: Vec<String>,
    test_sources: Vec<String>,
) -> Array {
    let suite = rsdmn::tck::Suite::from_sources(
        name,
        model_files.into_iter().zip(model_sources).collect(),
        test_files.into_iter().zip(test_sources).collect(),
    );
    let engine = rsdmn::tck::compile_suite(&suite);
    rsdmn::tck::run_suite(&suite, engine.as_ref())
        .into_iter()
        .map(|r| {
            let o = Object::new();
            let _ = Reflect::set(&o, &"file".into(), &JsValue::from_str(&r.file));
            let _ = Reflect::set(&o, &"id".into(), &JsValue::from_str(&r.id));
            let _ = Reflect::set(&o, &"success".into(), &JsValue::from_bool(r.success));
            let _ = Reflect::set(&o, &"message".into(), &JsValue::from_str(&r.message));
            JsValue::from(o)
        })
        .collect()
}

/// Version of the engine.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
