//! Performance benchmarks.
//!
//! Usage: `bench [path-to-tck/TestCases] [--filter <name>]`

use rsdmn::Engine;
use rsdmn::tck::{compile_suite, load_all, run_suite};
use rsdmn::value::{Context, Value};
use std::hint::black_box;
use std::path::PathBuf;
use std::time::{Duration, Instant};

fn measure<F: FnMut()>(name: &str, iterations: u32, mut f: F) -> Duration {
    // warm up, then report the best of several batches to reduce noise
    f();
    let batches = 5;
    let per_batch = (iterations / batches).max(1);
    let mut best = Duration::MAX;
    for _ in 0..batches {
        let start = Instant::now();
        for _ in 0..per_batch {
            f();
        }
        best = best.min(start.elapsed() / per_batch);
    }
    let per = best;
    println!("{:<34} {:>12.3} µs/iter", name, per.as_secs_f64() * 1e6);
    per
}

fn literal_decision(name: &str, text: &str, inputs: &[(&str, &str)]) -> String {
    let mut s = String::from(
        r#"<definitions xmlns="https://www.omg.org/spec/DMN/20230324/MODEL/" namespace="bench" name="bench">"#,
    );
    for (n, t) in inputs {
        s.push_str(&format!(
            r#"<inputData id="i_{n}" name="{n}"><variable name="{n}" typeRef="{t}"/></inputData>"#
        ));
    }
    s.push_str(&format!(
        r#"<decision id="d" name="{name}"><variable name="{name}"/>"#
    ));
    for (n, _) in inputs {
        s.push_str(&format!(
            r##"<informationRequirement><requiredInput href="#i_{n}"/></informationRequirement>"##
        ));
    }
    s.push_str(&format!(
        "<literalExpression><text>{}</text></literalExpression></decision></definitions>",
        text.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    ));
    s
}

fn decision_table(rules: usize) -> String {
    let mut s = String::from(
        r##"<definitions xmlns="https://www.omg.org/spec/DMN/20230324/MODEL/" namespace="bench" name="bench">
<inputData id="i_age" name="Age"><variable name="Age" typeRef="number"/></inputData>
<inputData id="i_cat" name="Category"><variable name="Category" typeRef="string"/></inputData>
<inputData id="i_vip" name="Is VIP"><variable name="Is VIP" typeRef="boolean"/></inputData>
<decision id="d" name="Discount"><variable name="Discount" typeRef="number"/>
<informationRequirement><requiredInput href="#i_age"/></informationRequirement>
<informationRequirement><requiredInput href="#i_cat"/></informationRequirement>
<informationRequirement><requiredInput href="#i_vip"/></informationRequirement>
<decisionTable hitPolicy="FIRST">
<input><inputExpression typeRef="number"><text>Age</text></inputExpression></input>
<input><inputExpression typeRef="string"><text>Category</text></inputExpression></input>
<input><inputExpression typeRef="boolean"><text>Is VIP</text></inputExpression></input>
<output name="discount" typeRef="number"/>"##,
    );
    for i in 0..rules {
        let lo = i % 100;
        s.push_str(&format!(
            "<rule><inputEntry><text>[{}..{})</text></inputEntry><inputEntry><text>\"cat{}\",\"other{}\"</text></inputEntry><inputEntry><text>{}</text></inputEntry><outputEntry><text>{}.5</text></outputEntry></rule>",
            lo,
            lo + 10,
            i % 37,
            i,
            if i % 2 == 0 { "true" } else { "-" },
            i
        ));
    }
    s.push_str("</decisionTable></decision></definitions>");
    s
}

fn engine(xml: String) -> Engine {
    Engine::from_xml(&[xml]).expect("model")
}

fn eval(engine: &Engine, decision: &str, inputs: &[(&str, Value)]) -> Value {
    let mut session = engine.session();
    for (n, v) in inputs {
        session.set_input(0, n, v.clone());
    }
    session.evaluate_decision(0, decision).unwrap()
}

fn main() {
    let mut args = std::env::args().skip(1);
    let mut root = PathBuf::from("../tck/TestCases");
    let mut filter: Option<String> = None;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--filter" => filter = args.next(),
            _ => root = PathBuf::from(a),
        }
    }
    let run = |name: &str| {
        filter
            .as_ref()
            .map(|f| name.contains(f.as_str()))
            .unwrap_or(true)
    };

    if filter
        .as_ref()
        .map(|f| f.contains("tck") || "tck".contains(f.as_str()))
        .unwrap_or(true)
        && root.exists()
    {
        let suites = load_all(&root, &["compliance-level-2", "compliance-level-3"], None);
        if run("tck compile") {
            measure("tck compile (all models)", 20, || {
                for s in &suites {
                    black_box(compile_suite(s));
                }
            });
        }
        if run("tck xml parse") {
            measure("tck xml parse", 20, || {
                for s in &suites {
                    for src in &s.sources {
                        black_box(
                            rsdmn::dmn::dom::parse(src, &["DMNDI", "extensionElements"]).ok(),
                        );
                    }
                }
            });
        }
        if run("tck read definitions") {
            measure("tck read definitions", 20, || {
                for s in &suites {
                    for src in &s.sources {
                        black_box(rsdmn::dmn::xml::read_definitions(src).ok());
                    }
                }
            });
        }
        if run("tck compile raw") {
            let raws: Vec<Vec<_>> = suites
                .iter()
                .map(|s| {
                    s.sources
                        .iter()
                        .filter_map(|x| rsdmn::dmn::xml::read_definitions(x).ok())
                        .collect()
                })
                .collect();
            measure("tck compile raw", 20, || {
                for r in &raws {
                    black_box(Engine::new(r.clone()));
                }
            });
        }
        let engines: Vec<_> = suites.iter().map(compile_suite).collect();
        if run("tck evaluate") {
            measure("tck evaluate (3437 cases)", 20, || {
                for (s, e) in suites.iter().zip(&engines) {
                    black_box(run_suite(s, e.as_ref()));
                }
            });
        }
    }

    if run("decision table 500 rules") || run("decision table 20 rules") {
        let e = engine(decision_table(500));
        let inputs = [
            ("Age", Value::int(95)),
            ("Category", Value::str("cat3")),
            ("Is VIP", Value::Bool(false)),
        ];
        measure("decision table 500 rules", 2000, || {
            black_box(eval(&e, "Discount", &inputs));
        });
        let e = engine(decision_table(20));
        let inputs = [
            ("Age", Value::int(15)),
            ("Category", Value::str("cat1")),
            ("Is VIP", Value::Bool(true)),
        ];
        measure("decision table 20 rules", 50000, || {
            black_box(eval(&e, "Discount", &inputs));
        });
    }

    if run("compile decision table 500") {
        let xml = decision_table(500);
        measure("compile decision table 500", 50, || {
            black_box(engine(xml.clone()));
        });
    }

    if run("for loop 10k arithmetic") || run("loan payment formula") {
        let e = engine(literal_decision(
            "d",
            "sum(for i in 1..10000 return i * 1.5 + i / 3)",
            &[],
        ));
        measure("for loop 10k arithmetic", 50, || {
            black_box(eval(&e, "d", &[]));
        });
        let e = engine(literal_decision(
            "d",
            "(Principal * Rate / 12) / (1 - (1 + Rate / 12) ** -Term)",
            &[
                ("Principal", "number"),
                ("Rate", "number"),
                ("Term", "number"),
            ],
        ));
        let inputs = [
            ("Principal", Value::int(600000)),
            (
                "Rate",
                Value::Number(rsdmn::number::Number::parse("0.0375").unwrap()),
            ),
            ("Term", Value::int(360)),
        ];
        measure("loan payment formula", 20000, || {
            black_box(eval(&e, "d", &inputs));
        });
    }

    if run("filter 10k contexts") {
        let employees: Vec<Value> = (0..10000)
            .map(|i| {
                let mut c = Context::new();
                c.put("id", Value::int(i));
                c.put("dept", Value::int(i % 50));
                c.put("name", Value::str(&format!("employee {}", i)));
                c.put("salary", Value::int(1000 + (i * 7) % 5000));
                Value::context(c)
            })
            .collect();
        let e = engine(literal_decision(
            "d",
            "count(Employees[dept = 10 and salary > 3000].name)",
            &[("Employees", "Any")],
        ));
        let inputs = [("Employees", Value::list(employees))];
        measure("filter 10k contexts", 100, || {
            black_box(eval(&e, "d", &inputs));
        });
    }

    if run("string functions 2k") {
        let e = engine(literal_decision(
            "d",
            "string length(string join(for i in 1..2000 return upper case(\"item \" + string(i)), \", \"))",
            &[],
        ));
        measure("string functions 2k", 100, || {
            black_box(eval(&e, "d", &[]));
        });
    }

    if run("date arithmetic 2k") {
        let e = engine(literal_decision(
            "d",
            "count(for i in 1..2000 return date(\"2020-01-01\") + duration(\"P\" + string(i) + \"D\") - @\"P1M\")",
            &[],
        ));
        measure("date arithmetic 2k", 100, || {
            black_box(eval(&e, "d", &[]));
        });
    }

    if run("sort 2k with lambda") {
        let e = engine(literal_decision(
            "d",
            "sort(for i in 1..2000 return modulo(i * 7919, 2003), function(a, b) a < b)[1]",
            &[],
        ));
        measure("sort 2k with lambda", 20, || {
            black_box(eval(&e, "d", &[]));
        });
    }
}
