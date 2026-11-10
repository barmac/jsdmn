# jsdmn

[![CI](https://github.com/barmac/jsdmn/workflows/CI/badge.svg)](https://github.com/barmac/jsdmn/actions?query=workflow%3ACI)

Evaluate DMN 1.3, 1.4 and 1.5 decisions both in NodeJS and in browser.

`jsdmn` passes all __3407__ test cases of compliance level 2 and 3 of [the DMN TCK](https://github.com/dmn-tck/tck), see [DMN TCK](#dmn-tck).

## Goal

This project is built for learning, so don't expect production-readiness soon (or anytime, really). Nevertheless, it passes [the dmntck test suite](https://github.com/dmn-tck/tck).

## Usage

```javascript
import { Engine } from 'jsdmn';

const engine = new Engine();

await engine.load(dmnXML);
const result = await engine.evaluateDecision('Decision', { input: 4 });

console.log(result) // prints result
```

### Inputs

Inputs are passed by the name of the respective input data element. Values not matching any input data element of the model are visible to decisions as plain variables.

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

Errors (e.g. values not conforming to their type, failing function invocations) evaluate to `null`, as specified by DMN. To understand why, evaluate with `engine.evaluate`, which returns the reported messages, too:

```javascript
const { value, messages } = await engine.evaluate('decision', 'Decision', inputs);

// messages: [ { severity: 'error', message: '...', elementId, elementName } ]
```

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
* Boxed expressions: literal expressions, contexts, decision tables (all hit policies and aggregations, input / output values, default outputs), invocations, function definitions (FEEL and Java), lists, relations, conditionals, filters, iterators (`for`, `some`, `every`)
* Item definitions: structures, collections, allowed values, type constraints, function types; type checks and implicit conversions (to and from singleton lists, date to date and time)
* Full FEEL, including temporal arithmetic, ranges, `instance of`, external Java functions and all builtin functions of DMN 1.5

### Implementation notes

* FEEL is evaluated by an interpreter that is part of this library. It uses [feelin](https://github.com/nikku/feelin) for parsing and its builtin functions, overriding those whose semantics deviate from the DMN 1.5 specification.
* Numbers are IEEE 754 doubles, not decimal128. Equality and comparison tolerate binary rounding errors (`0.1 + 0.2 = 0.3` is `true`), rounding functions (`decimal`, `round half up`, ...) work on the exact decimal representation.
* Temporal values use [`Temporal`](https://tc39.es/proposal-temporal/docs/) (via [temporal-polyfill](https://github.com/fullcalendar/temporal-polyfill)), extended to FEEL's year range of +/- 999,999,999.
* Java functions are emulated for a curated set of `java.lang` methods (`Math`, `Integer`, `Double`, `String#format`, ...).
* Decision tables with multiple hit policies (`COLLECT`, `RULE ORDER`, `OUTPUT ORDER`) yield an empty list if no rule matches (`COLLECT` with `COUNT`: `0`).

## DMN TCK

The engine is tested against the compliance level 2 and 3 test cases of the [DMN TCK](https://github.com/dmn-tck/tck), pinned to a specific revision in [`tasks/fetch-tck.js`](./tasks/fetch-tck.js).

```sh
# fetch the DMN TCK into tmp/dmn-tck
npm run tck:fetch

# run the DMN TCK test cases
npm run tck
```

The TCK models are DMN 1.5 models. Besides running them as authored, the runner re-expresses them as DMN 1.4 and DMN 1.3 models (rewriting model, DMNDI and FEEL namespaces) and runs them again:

| Version | Passed | Skipped |
| ------- | ------ | ------- |
| DMN 1.5 | 3407 / 3407 | - |
| DMN 1.4 | 3407 / 3407 | - |
| DMN 1.3 | 3388 / 3407 | 19, using boxed `conditional`, `filter`, `for`, `some` and `every` (introduced with DMN 1.4) |

For skipped folders, the runner asserts that their models are rejected when read as the respective version.

Results are written to `tmp/tck-results/tck_results.csv` (DMN 1.5) and `tck_results-dmn14.csv` / `tck_results-dmn13.csv`, in the format used by the [TCK results](https://github.com/dmn-tck/tck/tree/master/TestResults).

The test run can be customized via environment variables:

* `TCK_DIR` - location of the TCK checkout (default: `tmp/dmn-tck`)
* `TCK_FILTER` - only run test case folders containing the given string, e.g. `TCK_FILTER=0085 npm run tck`
* `TCK_LEVELS` - comma separated test case folders to run (default: `compliance-level-2,compliance-level-3`, add `non-compliant` to run the TCK's non-compliant tests)
* `TCK_STRICT_ERRORS` - additionally require the engine to report an error for results expected to be errors (`errorResult="true"`)
* `TCK_DMN_VERSIONS` - comma separated DMN versions to run the TCK as (default: `1.5,1.4,1.3`)

Numbers are compared with a relative tolerance of `1e-12`; expected values given with 7 or more fraction digits (e.g. results of `exp(4)`) are compared at that precision. Known failures can be registered in [`test/tck/known-failures.js`](./test/tck/known-failures.js); there are none.

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
* [temporal-polyfill](https://github.com/fullcalendar/temporal-polyfill) - `Temporal` implementation

## License

Use under the terms of the [MIT license](http://opensource.org/licenses/MIT).
