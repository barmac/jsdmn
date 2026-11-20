# rsdmn

A DMN 1.5 interpreter written in Rust: a complete FEEL implementation plus
decision models (decision tables, boxed expressions, BKMs, decision services
and imports).

It passes **all 3437 test cases** of compliance levels 2 and 3 of the
[DMN TCK](https://github.com/dmn-tck/tck). The TCK is included as a git
submodule (`../tck`) and runs as part of `cargo test`.

## Usage

```rust
use rsdmn::{Engine, Value};

let xml = std::fs::read_to_string("../test/fixtures/input.dmn").unwrap();
let engine = Engine::from_xml(&[xml]).unwrap();

let mut session = engine.session();
session.set_input(0, "a", Value::int(2));
session.set_input(0, "b", Value::int(2));

let result = session.evaluate_decision(0, "Decision").unwrap();
println!("{:?}", result); // prints 4
```

Several models (e.g. a model and the models it imports) are loaded together
by passing all of them to `Engine::from_xml`; the model index passed to the
session methods is the position in that list.

Standalone FEEL expressions can be evaluated with `rsdmn::feel::evaluate`:

```rust
let v = rsdmn::feel::evaluate("for x in 1..3 return x * 2", &[]).unwrap();
println!("{:?}", v); // prints [2, 4, 6]
```

## Running the TCK

```sh
git submodule update --init        # fetches ../tck
cd rust
cargo test --release               # unit tests + full TCK
cargo run --release --bin tck -- ../tck/TestCases --csv tck_results.csv
```

The `tck` binary prints every failing test case and a summary, and can
write the results in the TCK's `tck_results.csv` format.

Numbers are compared like the reference TCK runners do (absolute tolerance of
`1e-8`). This only matters for 30 test cases whose expected values in the TCK
are rounded (e.g. `exp(4)` is expected as `54.59815003`). Set
`TCK_STRICT_NUMBERS=1` to compare numbers exactly.

## Implementation notes

* `number` – decimal arithmetic with 34 significant digits and
  round-half-even (IEEE 754 decimal128 semantics) on top of `num-bigint`.
* `temporal` – dates, times, date-times and durations. The calendar is
  implemented in-house so that years up to ±999 999 999 work; IANA time zone
  rules come from `chrono-tz`.
* `feel` – lexer, parser (names containing spaces and operators such as
  `date and time` or `Loan-to-value` are recognised using the names known
  from the model), evaluator, built-in functions and XPath regular
  expressions (translated to `fancy-regex`).
* `types` – item definitions, `instance of` and implicit conversions
  (singleton lists, date to date and time, constraint checks).
* `dmn` – reading DMN XML, decision tables (all hit policies), DRG
  evaluation, decision services and imports.
* `tck` – the TCK test case reader and runner.
