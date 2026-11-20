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

## WebAssembly

The engine also runs in the browser: [`wasm/`](./wasm) contains JavaScript
bindings and an example page. See [wasm/README.md](./wasm/README.md).

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

## Performance

`cargo run --release --bin bench -- ../tck/TestCases` runs the benchmarks
(best of 5 batches; `--filter <name>` selects some). Numbers from a 4 core
cloud VM, comparing the first version that passed the TCK with the current one:

| Benchmark | before | now | speedup |
|---|---:|---:|---:|
| full TCK run (`tck` binary, 3437 cases, wall clock; now on 4 threads) | 60 ms | 18 ms | 3.3x |
| decision table, 500 rules | 86.7 µs | 11.7 µs | 7.4x |
| decision table, 20 rules (incl. session and inputs) | 5.7 µs | 1.1 µs | 5.3x |
| `sum(for i in 1..10000 return i * 1.5 + i / 3)` | 1,473 ms | 2.15 ms | 685x |
| loan payment formula (`(1 + r) ** -360`) | 20.3 µs | 2.4 µs | 8.6x |
| filter 10k contexts | 2.72 ms | 0.80 ms | 3.4x |
| string functions, 2k items | 17.8 ms | 0.45 ms | 39x |
| date arithmetic, 2k items | 9.17 ms | 0.52 ms | 18x |
| `sort` of 2k numbers with a comparator | 164 ms | 1.89 ms | 87x |

What makes it fast:

* numbers are `Copy` values with an `i128` coefficient; arbitrary precision
  is only used for rare intermediate results, and integer powers use a
  192 bit fast path that proves correct rounding (or falls back);
* sub-expressions without variables are folded when the model is compiled
  and calls of built-in functions are resolved to an enum at compile time;
* `partial` in `for` loops is only materialized when used, `sort` with a
  comparator is a merge sort, FIRST / UNIQUE decision tables stop early;
* small `Value`s (32 bytes), compact scope frames reused across loop
  iterations, borrowed tokens and a zero-copy XML reader;
* the TCK runner runs test directories in parallel.

The binaries can use mimalloc (`--features mimalloc`), which speeds up model
compilation by about 25%.

## Implementation notes

* `number` – decimal arithmetic with 34 significant digits and
  round-half-even (IEEE 754 decimal128 semantics); `i128` coefficients with
  a `num-bigint` fallback.
* `temporal` – dates, times, date-times and durations. The calendar is
  implemented in-house so that years up to ±999 999 999 work; IANA time zone
  rules come from `chrono-tz`.
* `feel` – lexer, parser (names containing spaces and operators such as
  `date and time` or `Loan-to-value` are recognised using the names known
  from the model), evaluator, built-in functions and XPath regular
  expressions (translated to `fancy-regex`).
* `types` – item definitions, `instance of` and implicit conversions
  (singleton lists, date to date and time, constraint checks).
* `dmn` – reading DMN XML (a small zero-copy XML reader that skips diagram
  interchange), decision tables (all hit policies), DRG evaluation,
  decision services and imports.
* `tck` – the TCK test case reader and runner.
