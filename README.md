# jsdmn

[![CI](https://github.com/barmac/jsdmn/workflows/CI/badge.svg)](https://github.com/barmac/jsdmn/actions?query=workflow%3ACI)

Evaluate DMN 1.3, 1.4 and 1.5 decisions both in NodeJS and in browser.

`jsdmn` passes __2831__ of the 3437 test cases of compliance level 2 and 3 of [the DMN TCK](https://github.com/dmn-tck/tck). The remaining test cases fail because of gaps in [feelin](https://github.com/nikku/feelin), the FEEL interpreter it uses, see [DMN TCK](#dmn-tck).

## Goal

This project is built for learning, so don't expect production-readiness soon (or anytime, really). Nevertheless, it is tested against [the dmntck test suite](https://github.com/dmn-tck/tck).

## Usage

```javascript
import { Engine } from 'jsdmn';

const engine = new Engine();

await engine.load(dmnXML);
const result = await engine.evaluateDecision('Decision', { input: 4 });

console.log(result) // prints result
```

### Inputs

Inputs are passed by the name of the respective input data element. Values not matching any input data element of the model are visible to decisions as plain variables. A value given for a decision that another decision requires is used instead of evaluating it.

Inputs of [imported models](#imports) can be nested under the name of the import or be given in qualified form, as `{namespace}name`:

```javascript
await engine.evaluateDecision('Decision', {
  'Applicant': { name: 'Ada', age: 36 },
  'importedModel': { 'Loan Amount': 1000 },
  '{https://example.com/other-model}Rate': 0.5
});
```

FEEL values map to JavaScript as follows: `number`, `string` and `boolean` to their JavaScript counterparts, lists to arrays, contexts to objects, `null` to `null`. Temporal values (date, time, date and time, durations) and ranges are represented by feelin's value classes (`FeelDate`, `FeelTime`, `FeelDateTime`, `FeelDuration`, `FeelRange`), FEEL functions by `FeelFunction`.

### Decision services and business knowledge models

```javascript
// by input data and input decision names
await engine.evaluateDecisionService('Calculate', { Amount: 100, Rate: 0.5 });

// positional or named parameters
await engine.evaluateBusinessKnowledgeModel('Double', [ 21 ]);
await engine.evaluateBusinessKnowledgeModel('Double', { x: 21 });
```

### Messages

Errors (e.g. values not conforming to their type, `null` operands, invalid builtin function arguments) evaluate to `null`, as specified by DMN. To understand why, evaluate with `engine.evaluate`, which returns the reported messages, too:

```javascript
const { value, messages } = await engine.evaluate('decision', 'Decision', { name: null });

// value: null
// messages: [ {
//   severity: 'error',
//   type: 'INVALID_TYPE',
//   message: 'Can\'t add \'null\' to \'"Hello "\' (in <"Hello " + name>)',
//   elementId: 'Decision_1',
//   elementName: 'Decision'
// } ]
```

Problems that cause an expression to evaluate to `null` are reported as errors. Problems an expression recovers from, e.g. accessing a property of `null` in `if a.b = null then "none" else a.b`, are reported as warnings. Regular `null` results, such as `not(null)` (three-valued logic) or `get value(context, "missing key")`, are not reported.

FEEL problems are reported as far as feelin reports them. Some, such as division by zero or invalid arguments to many builtin functions, yield `null` without a message (see [barmac/feelin#12](https://github.com/barmac/feelin/pull/12)).

### DMN versions

DMN 1.3, 1.4 and 1.5 models are supported. The version is detected from the model's namespace and exposed by the loaded model:

```javascript
const model = await engine.load(dmnXML);

console.log(model.version); // '1.5'
```

Each version is parsed strictly, with its own schema: elements a version does not define, e.g. a boxed `<conditional>` (DMN 1.4) in a DMN 1.3 model, are rejected. Diagram interchange (DMNDI) of any of these versions is accepted. DMN 1.1 and 1.2 models are read as DMN 1.3.

Expressions are evaluated with DMN 1.5 semantics regardless of the model's version, i.e. builtin functions introduced by later versions are available in models of earlier versions, too.

### Imports

Models may import each other and can be loaded in any order. Pass `namespace` to choose among elements of the same name in different models:

```javascript
await engine.load(mainXML);
await engine.load(importedXML);

await engine.evaluateDecision('Decision', inputs, { namespace: 'https://example.com/model' });
```

## Features

* DMN 1.3, 1.4 and 1.5 models (DMN 1.1 and 1.2 are read as DMN 1.3), see [DMN versions](#dmn-versions)
* Decisions, input data, business knowledge models, decision services (invoked directly or as functions), DRG scoping
* Imports of DMN models (named and unnamed)
* Boxed expressions: literal expressions, contexts, decision tables (all hit policies and aggregations, input / output values, default outputs), invocations, function definitions (FEEL), lists, relations, conditionals, filters, iterators (`for`, `some`, `every`)
* Item definitions: structures, collections, allowed values, type constraints, function types; type checks and implicit conversions (to and from singleton lists, date to date and time)
* FEEL expressions and unary tests, evaluated by [feelin](https://github.com/nikku/feelin), including `instance of` checks against item definitions; see [DMN TCK](#dmn-tck) for the parts of FEEL that feelin does not implement as specified yet

### Implementation notes

* FEEL is evaluated by [feelin](https://github.com/nikku/feelin). The engine evaluates the DRG, boxed expressions and decision tables, and checks and coerces values according to item definitions; item definitions are passed to feelin so that `instance of` can test against them.
* Numbers are IEEE 754 doubles, as in feelin, not decimal128 (`0.1 + 0.2 = 0.3` is `false`).
* Function definitions of kind `Java` or `PMML` are external functions, which the engine does not execute: they evaluate to `null` and report an error.
* Decision tables with multiple hit policies (`COLLECT`, `RULE ORDER`, `OUTPUT ORDER`) yield an empty list if no rule matches (`COLLECT` with `COUNT`: `0`).

## DMN TCK

The engine is tested against the compliance level 2 and 3 test cases of the [DMN TCK](https://github.com/dmn-tck/tck).

```sh
# fetch (or update) the DMN TCK into tmp/dmn-tck
npm run tck:fetch

# run the DMN TCK test cases, fetching / updating the TCK first
npm run tck
```

The TCK evolves over time, so its latest revision (`master`) is used by default: an existing checkout is updated whenever the TCK moved on, and reused as-is if the TCK cannot be reached. Set `TCK_REF` to test against a specific branch, tag or commit; the `DMNv1.3` and `DMNv1.4` tags hold the TCK as published for DMN 1.3 (138 models) and DMN 1.4 (155 models). Known failures are recorded for the latest revision; earlier revisions encode earlier expectations. CI caches the TCK by the commit it resolves to and additionally runs weekly to catch up with TCK changes; each run records the TCK revision it used in `tmp/tck-results/summary.json`.

The TCK models are DMN 1.5 models. Besides running them as authored, the runner re-expresses them as DMN 1.4 and DMN 1.3 models (rewriting model, DMNDI and FEEL namespaces) and runs them again. Results as of TCK revision [`22f109c`](https://github.com/dmn-tck/tck/commit/22f109c37cb55d5ac9bc84feaa1bb51b156f76ba) (2026-11-23):

| Version | Passed | Skipped |
| ------- | ------ | ------- |
| DMN 1.5 | 2831 / 3437 | 606 known failures |
| DMN 1.4 | 2831 / 3437 | 606 known failures |
| DMN 1.3 | 2812 / 3437 | 606 known failures, 19 using boxed `conditional`, `filter`, `for`, `some` and `every` (introduced with DMN 1.4) |

For skipped folders, the runner asserts that their models are rejected when read as the respective version.

Results are written to `tmp/tck-results/tck_results.csv` (DMN 1.5) and `tck_results-dmn14.csv` / `tck_results-dmn13.csv`, in the format used by the [TCK results](https://github.com/dmn-tck/tck/tree/master/TestResults).

The test run can be customized via environment variables:

* `TCK_DIR` - location of the TCK checkout (default: `tmp/dmn-tck`)
* `TCK_REF` - TCK branch, tag or commit to fetch (default: `master`)
* `TCK_FILTER` - only run test case folders containing the given string, e.g. `TCK_FILTER=0085 npm run tck`
* `TCK_LEVELS` - comma separated test case folders to run (default: `compliance-level-2,compliance-level-3`, add `non-compliant` to run the TCK's non-compliant tests)
* `TCK_DMN_VERSIONS` - comma separated DMN versions to run the TCK as (default: `1.5,1.4,1.3`)

Results expected to be errors (`errorResult="true"`) must evaluate to `null` and be reported as an error; results expected to be non-`null` must not report errors.

Numbers are compared with a relative tolerance of `1e-12`; expected values given with 7 or more fraction digits (e.g. results of `exp(4)`) are compared at that precision.

### Known failures

Test cases that fail because of gaps in feelin are listed in [`test/tck/known-failures.js`](./test/tck/known-failures.js), grouped by gap, and reported as skipped. A known failure that passes is noted in the results, a failure that is not listed fails the run. Each gap links the feelin pull request that closes it; this was checked by running the TCK against each of them.

| Gap in feelin | Test cases | Fixed by |
| ------------- | ---------- | -------- |
| `null` results of errors are not reported, e.g. division by zero, invalid builtin arguments | 239 | [barmac/feelin#12](https://github.com/barmac/feelin/pull/12) |
| temporal and duration arithmetic | 165 | [barmac/feelin#13](https://github.com/barmac/feelin/pull/13) |
| temporal value range (years beyond +/- 271,821) and parsing | 86 | [barmac/feelin#17](https://github.com/barmac/feelin/pull/17) |
| `range(from)` function | 19 | [barmac/feelin#9](https://github.com/barmac/feelin/pull/9) |
| XML Schema regular expressions | 14 | [barmac/feelin#11](https://github.com/barmac/feelin/pull/11) |
| range equality, `null` bounds and iteration | 12 | [barmac/feelin#14](https://github.com/barmac/feelin/pull/14) |
| rounding scales | 9 | [barmac/feelin#7](https://github.com/barmac/feelin/pull/7) |
| range functions (`before`, `during`, ...) | 8 | [barmac/feelin#10](https://github.com/barmac/feelin/pull/10) |
| `list replace` | 7 | [barmac/feelin#4](https://github.com/barmac/feelin/pull/4) |
| filters on non-list values | 3 | [barmac/feelin#3](https://github.com/barmac/feelin/pull/3) |
| boolean unary tests | 2 | [barmac/feelin#6](https://github.com/barmac/feelin/pull/6) |
| context literals with duplicate keys | 1 | [barmac/feelin#8](https://github.com/barmac/feelin/pull/8) |
| `is` with durations of different kinds | 1 | [barmac/feelin#2](https://github.com/barmac/feelin/pull/2) |
| parameter types of function definitions | 1 | [nikku/feelin#184](https://github.com/nikku/feelin/pull/184) |
| arithmetic or range semantics, whose fix then yields `null` without reporting an error | 7 | [barmac/feelin#13](https://github.com/barmac/feelin/pull/13) or [#14](https://github.com/barmac/feelin/pull/14), errors: [#12](https://github.com/barmac/feelin/pull/12) |
| `range<T>` types in `instance of` (8), errors for missing duration properties (6) and `floor` / `ceiling` with a `null` scale (4) | 18 | - |

Besides these, the 14 test cases of `0076-feel-external-java` fail, because the engine does not execute external (Java) functions.

## Build and Run

Requires a maintained [Node.js LTS release](https://nodejs.org/en/about/previous-releases) (22 or later, see [`.nvmrc`](./.nvmrc)).

```sh
# lint, run unit tests and the DMN TCK
npm run all

# run unit tests
npm test
```

## Credits

This library is built upon these amazing projects:

* [feelin](https://github.com/nikku/feelin) - JavaScript FEEL parser and interpreter
* [dmn-moddle](https://github.com/bpmn-io/dmn-moddle) - DMN parser

## License

Use under the terms of the [MIT license](http://opensource.org/licenses/MIT).
