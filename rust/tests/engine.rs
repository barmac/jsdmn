//! Ports of the JavaScript specs (`test/spec`) using the shared fixtures.

use rsdmn::Engine;
use rsdmn::number::Number;
use rsdmn::value::{Context, Value};
use std::path::Path;

fn engine(fixture: &str) -> Engine {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../test/fixtures")
        .join(fixture);
    let xml = std::fs::read_to_string(path).unwrap();
    Engine::from_xml(&[xml]).unwrap()
}

fn evaluate(fixture: &str, decision: &str, inputs: &[(&str, Value)]) -> Value {
    let engine = engine(fixture);
    let mut session = engine.session();
    for (name, value) in inputs {
        session.set_input(0, name, value.clone());
    }
    session
        .evaluate_decision(0, decision)
        .expect("decision not found")
}

fn num(s: &str) -> Value {
    Value::Number(Number::parse(s).unwrap())
}

fn ctx(entries: &[(&str, Value)]) -> Value {
    let mut c = Context::new();
    for (k, v) in entries {
        c.put(*k, v.clone());
    }
    Value::context(c)
}

fn assert_value(actual: Value, expected: Value) {
    assert!(
        rsdmn::tck::values_match(&expected, &actual),
        "expected {:?} but got {:?}",
        expected,
        actual
    );
}

#[test]
fn literal_expression() {
    assert_value(evaluate("simple.dmn", "Decision", &[]), num("4"));
}

#[test]
fn decision_with_input() {
    assert_value(
        evaluate("input.dmn", "Decision", &[("a", num("2")), ("b", num("2"))]),
        num("4"),
    );
}

#[test]
fn simple_context() {
    assert_value(
        evaluate("context.dmn", "Decision", &[]),
        ctx(&[("expression", num("4"))]),
    );
}

#[test]
fn context_with_dependent_entries() {
    assert_value(
        evaluate("context.dmn", "Decision with multiple entries", &[]),
        ctx(&[("first", num("4")), ("second", num("6"))]),
    );
}

#[test]
fn context_with_result() {
    assert_value(
        evaluate("context.dmn", "Context with result", &[]),
        num("6"),
    );
}

#[test]
fn function_definition() {
    assert_value(
        evaluate("functionDefinition.dmn", "Decision", &[]),
        num("8"),
    );
}

#[test]
fn requirements() {
    assert_value(
        evaluate(
            "requirements.dmn",
            "Average",
            &[("a", num("3")), ("b", num("5"))],
        ),
        num("4"),
    );
}
