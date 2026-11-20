//! Runner for the DMN Technology Compatibility Kit (https://github.com/dmn-tck/tck).

use crate::dmn::engine::Engine;
use crate::number::Number;
use crate::temporal::{self, Date, DateTime, Time};
use crate::value::{Context, Value, feel_eq};
use roxmltree::{Document, Node};
use std::fs;
use std::path::{Path, PathBuf};

const XSI: &str = "http://www.w3.org/2001/XMLSchema-instance";

#[derive(Debug, Clone)]
pub struct InputNode {
    pub name: String,
    pub namespace: Option<String>,
    pub value: Value,
}

#[derive(Debug, Clone)]
pub struct ResultNode {
    pub name: String,
    pub namespace: Option<String>,
    pub error_result: bool,
    pub expected: Value,
}

#[derive(Debug, Clone)]
pub struct TestCase {
    pub id: String,
    pub kind: String,
    pub invocable: Option<String>,
    pub inputs: Vec<InputNode>,
    pub results: Vec<ResultNode>,
}

#[derive(Debug, Clone)]
pub struct TestFile {
    pub path: PathBuf,
    pub model_name: Option<String>,
    pub cases: Vec<TestCase>,
}

#[derive(Debug, Clone)]
pub struct CaseResult {
    pub suite: String,
    pub file: String,
    pub id: String,
    pub success: bool,
    pub message: String,
}

fn child<'a, 'i>(n: Node<'a, 'i>, name: &str) -> Option<Node<'a, 'i>> {
    n.children()
        .find(|c| c.is_element() && c.tag_name().name() == name)
}

fn is_nil(n: Node) -> bool {
    n.attribute((XSI, "nil")) == Some("true")
}

fn typed_value(n: Node) -> Value {
    if is_nil(n) {
        return Value::Null;
    }
    let text: String = n.children().filter_map(|c| c.text()).collect();
    let ty = n.attribute((XSI, "type")).unwrap_or("xsd:string");
    let ty = ty.rsplit(':').next().unwrap_or(ty);
    let t = text.trim();
    match ty {
        "decimal" | "double" | "float" | "int" | "integer" | "long" | "short" => {
            Number::parse(t).map(Value::Number).unwrap_or(Value::Null)
        }
        "boolean" => match t {
            "true" | "1" => Value::Bool(true),
            "false" | "0" => Value::Bool(false),
            _ => Value::Null,
        },
        "date" => Date::parse(t).map(Value::Date).unwrap_or(Value::Null),
        "dateTime" => DateTime::parse(t)
            .map(Value::DateTime)
            .unwrap_or(Value::Null),
        "time" => Time::parse(t).map(Value::Time).unwrap_or(Value::Null),
        "duration" => match temporal::parse_duration(t) {
            Some(temporal::Duration::Dt(d)) => Value::DtDuration(d),
            Some(temporal::Duration::Ym(d)) => Value::YmDuration(d),
            None => Value::Null,
        },
        _ => Value::str(&text),
    }
}

/// Parse a `valueType` element (`value` | `component`* | `list`).
pub fn parse_value(n: Node) -> Value {
    if is_nil(n) {
        return Value::Null;
    }
    if let Some(v) = child(n, "value") {
        return typed_value(v);
    }
    if let Some(l) = child(n, "list") {
        if is_nil(l) {
            return Value::Null;
        }
        return Value::list(
            l.children()
                .filter(|c| c.is_element() && c.tag_name().name() == "item")
                .map(parse_value)
                .collect(),
        );
    }
    let comps: Vec<Node> = n
        .children()
        .filter(|c| c.is_element() && c.tag_name().name() == "component")
        .collect();
    if !comps.is_empty() {
        let mut ctx = Context::new();
        for c in comps {
            let name = c.attribute("name").unwrap_or("").to_string();
            ctx.put(name.as_str(), parse_value(c));
        }
        return Value::context(ctx);
    }
    Value::Null
}

pub fn read_test_file(path: &Path) -> Result<TestFile, String> {
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let doc = Document::parse(&text).map_err(|e| e.to_string())?;
    let root = doc.root_element();
    if root.tag_name().name() != "testCases" {
        return Err("not a test case file".into());
    }
    let model_name = child(root, "modelName")
        .and_then(|m| m.text())
        .map(|s| s.trim().to_string());
    let mut cases = Vec::new();
    for tc in root
        .children()
        .filter(|c| c.is_element() && c.tag_name().name() == "testCase")
    {
        let id = tc.attribute("id").unwrap_or("").to_string();
        let kind = tc.attribute("type").unwrap_or("decision").to_string();
        let invocable = tc.attribute("invocableName").map(|s| s.to_string());
        let inputs = tc
            .children()
            .filter(|c| c.is_element() && c.tag_name().name() == "inputNode")
            .map(|i| InputNode {
                name: i.attribute("name").unwrap_or("").to_string(),
                namespace: i.attribute("namespace").map(|s| s.to_string()),
                value: parse_value(i),
            })
            .collect();
        let results = tc
            .children()
            .filter(|c| c.is_element() && c.tag_name().name() == "resultNode")
            .map(|r| ResultNode {
                name: r.attribute("name").unwrap_or("").to_string(),
                namespace: r.attribute("namespace").map(|s| s.to_string()),
                error_result: r.attribute("errorResult") == Some("true"),
                expected: child(r, "expected").map(parse_value).unwrap_or(Value::Null),
            })
            .collect();
        cases.push(TestCase {
            id,
            kind,
            invocable,
            inputs,
            results,
        });
    }
    Ok(TestFile {
        path: path.to_path_buf(),
        model_name,
        cases,
    })
}

/// Number comparison tolerance used by the reference TCK runners.
pub fn numbers_match(a: &Number, b: &Number) -> bool {
    if a == b {
        return true;
    }
    if std::env::var("TCK_STRICT_NUMBERS").is_ok() {
        return false;
    }
    let tol = Number::parse("0.00000001").unwrap();
    a.sub(b).abs() < tol
}

/// Compare an expected TCK value with an actual result.
pub fn values_match(expected: &Value, actual: &Value) -> bool {
    match (expected, actual) {
        (Value::Null, Value::Null) => true,
        (Value::Number(a), Value::Number(b)) => numbers_match(a, b),
        (Value::List(a), Value::List(b)) => {
            a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| values_match(x, y))
        }
        (Value::Context(a), Value::Context(b)) => {
            a.len() == b.len()
                && a.entries()
                    .iter()
                    .all(|(k, v)| b.get(k).map(|w| values_match(v, w)).unwrap_or(false))
        }
        (Value::DateTime(a), Value::DateTime(b)) => {
            if a == b {
                return true;
            }
            a.date == b.date
                && a.time.hour == b.time.hour
                && a.time.minute == b.time.minute
                && a.time.second == b.time.second
                && a.time.nanos == b.time.nanos
                && a.offset() == b.offset()
                && a.has_zone() == b.has_zone()
        }
        (Value::Time(a), Value::Time(b)) => a == b,
        (Value::Date(a), Value::Date(b)) => a == b,
        (Value::String(a), Value::String(b)) => a == b,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::DtDuration(_), _) | (Value::YmDuration(_), _) => {
            feel_eq(expected, actual) == Some(true)
        }
        _ => false,
    }
}

fn dmn_files(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().map(|x| x == "dmn").unwrap_or(false))
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

fn test_files(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().map(|x| x == "xml").unwrap_or(false))
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

/// A TCK test directory loaded into memory (models and test files).
pub struct Suite {
    pub name: String,
    pub sources: Vec<String>,
    pub model_names: Vec<String>,
    pub files: Vec<(String, TestFile)>,
}

/// Read the models and test files of one TCK test directory.
pub fn load_suite(dir: &Path, name: &str) -> Suite {
    let mut sources = Vec::new();
    let mut model_names = Vec::new();
    for d in &dmn_files(dir) {
        if let Ok(s) = fs::read_to_string(d) {
            sources.push(s);
            model_names.push(d.file_name().unwrap().to_string_lossy().to_string());
        }
    }
    let mut files = Vec::new();
    for tf in test_files(dir) {
        if let Ok(f) = read_test_file(&tf) {
            files.push((tf.file_stem().unwrap().to_string_lossy().to_string(), f));
        }
    }
    Suite {
        name: name.to_string(),
        sources,
        model_names,
        files,
    }
}

/// Compile the models of a suite.
pub fn compile_suite(suite: &Suite) -> Option<Engine> {
    Engine::from_xml(&suite.sources).ok()
}

/// Run all test cases of a suite against a compiled engine.
pub fn run_suite(suite: &Suite, engine: Option<&Engine>) -> Vec<CaseResult> {
    let mut out = Vec::new();
    for (file_name, file) in &suite.files {
        for case in &file.cases {
            let (success, message) = match engine {
                None => (false, "model failed to load".to_string()),
                Some(engine) => {
                    let main = file
                        .model_name
                        .as_ref()
                        .and_then(|m| suite.model_names.iter().position(|n| n == m))
                        .unwrap_or(0);
                    run_case(engine, main, case)
                }
            };
            out.push(CaseResult {
                suite: suite.name.clone(),
                file: file_name.clone(),
                id: case.id.clone(),
                success,
                message,
            });
        }
    }
    out
}

/// Run all test files of one TCK test directory.
pub fn run_test_dir(dir: &Path, suite: &str) -> Vec<CaseResult> {
    let suite = load_suite(dir, suite);
    let engine = compile_suite(&suite);
    run_suite(&suite, engine.as_ref())
}

fn verbose() -> bool {
    static V: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *V.get_or_init(|| std::env::var("TCK_VERBOSE").is_ok())
}

fn run_case(engine: &Engine, main: usize, case: &TestCase) -> (bool, String) {
    let mut session = engine.session();
    for i in &case.inputs {
        let m = i
            .namespace
            .as_ref()
            .and_then(|ns| engine.model_index(ns))
            .unwrap_or(main);
        session.set_input(m, &i.name, i.value.clone());
    }
    let mut failures = Vec::new();
    let service_result = if case.kind == "decisionService" {
        let name = case.invocable.clone().unwrap_or_default();
        Some(session.evaluate_service(main, &name).unwrap_or(Value::Null))
    } else {
        None
    };
    for r in &case.results {
        let m = r
            .namespace
            .as_ref()
            .and_then(|ns| engine.model_index(ns))
            .unwrap_or(main);
        let actual = match &service_result {
            Some(v) => match v {
                Value::Context(c)
                    if c.contains(&crate::feel::lexer::normalize_name(&r.name))
                        && !matches!(r.expected, Value::Context(_)) =>
                {
                    c.get(&crate::feel::lexer::normalize_name(&r.name))
                        .cloned()
                        .unwrap_or(Value::Null)
                }
                other => other.clone(),
            },
            None => if case.kind == "bkm" {
                session.invoke_bkm(m, case.invocable.as_deref().unwrap_or(&r.name))
            } else {
                session.evaluate_decision(m, &r.name)
            }
            .unwrap_or(Value::Null),
        };
        if !values_match(&r.expected, &actual) {
            failures.push(format!(
                "'{}' expected {:?} but was {:?}",
                r.name, r.expected, actual
            ));
        } else if verbose() {
            eprintln!("  ok '{}' = {:?}", r.name, actual);
        }
    }
    (failures.is_empty(), failures.join("; "))
}

/// Load every test directory found below `root` (e.g. `tck/TestCases`).
pub fn load_all(root: &Path, levels: &[&str], filter: Option<&str>) -> Vec<Suite> {
    let mut out = Vec::new();
    for level in levels {
        let ldir = root.join(level);
        let mut dirs: Vec<PathBuf> = match fs::read_dir(&ldir) {
            Ok(rd) => rd
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.is_dir())
                .collect(),
            Err(_) => continue,
        };
        dirs.sort();
        for d in dirs {
            let suite = format!("{}/{}", level, d.file_name().unwrap().to_string_lossy());
            if let Some(f) = filter
                && !suite.contains(f)
            {
                continue;
            }
            out.push(load_suite(&d, &suite));
        }
    }
    out
}

/// Run every test directory found below `root` (e.g. `tck/TestCases`).
pub fn run_all(root: &Path, levels: &[&str], filter: Option<&str>) -> Vec<CaseResult> {
    let mut out = Vec::new();
    for level in levels {
        let ldir = root.join(level);
        let mut dirs: Vec<PathBuf> = match fs::read_dir(&ldir) {
            Ok(rd) => rd
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.is_dir())
                .collect(),
            Err(_) => continue,
        };
        dirs.sort();
        for d in dirs {
            let suite = format!("{}/{}", level, d.file_name().unwrap().to_string_lossy());
            if let Some(f) = filter
                && !suite.contains(f)
            {
                continue;
            }
            out.extend(run_test_dir(&d, &suite));
        }
    }
    out
}
