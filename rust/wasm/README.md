# rsdmn in the browser (WebAssembly)

WebAssembly bindings of the rsdmn DMN engine, plus a browser example
([`www/`](./www)): edit a DMN model and its inputs and see the results of all
decisions as you type, or try FEEL expressions in a playground. Everything runs
locally in the browser.

The WebAssembly build passes the complete DMN TCK, like the native one
(`node test/tck.mjs`).

## Build

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.130 --locked   # must match the wasm-bindgen crate
./build.sh                                                 # writes www/pkg
```

## Run the example

Serve `www/` with any static file server, e.g.

```sh
python3 -m http.server --directory www 8000
```

and open <http://localhost:8000>. (Opening `index.html` from the file system
does not work: browsers only load WebAssembly modules over HTTP.)

The example loads [`www/example.dmn`](./www/example.dmn), a loan approval
model with a structured input, decision tables, a business knowledge model,
an invocation, date arithmetic and a boxed context. Use *Open .dmn* to try
your own models.

## API

`www/pkg/rsdmn_wasm.js` is an ES module (TypeScript declarations in
`rsdmn_wasm.d.ts`):

```js
import init, { DmnEngine, evaluateFeel } from "./pkg/rsdmn_wasm.js";

await init(); // fetches rsdmn_wasm_bg.wasm next to the module

// a model and the models it imports; the first one is the main model
const engine = new DmnEngine([xml]);

engine.decisions();   // [{ name: "Eligibility", type: "string" }, ...]
engine.inputData();   // [{ name: "Applicant", type: "tApplicant" }, ...]

const inputs = {
  Applicant: { age: 32, "monthly income": 5200, "credit score": 710, employment: "EMPLOYED" },
  "Requested Amount": 250000,
  Term: 360,
  "Application Date": "2025-03-14",       // a date, because the input is typed `date`
};
engine.evaluate("Eligibility", inputs);      // "ELIGIBLE"
engine.evaluateAll(inputs);                  // { "Risk Category": "MEDIUM", ... }
engine.evaluateToFeel("Summary", inputs);    // '{"eligibility": "ELIGIBLE", ..., "first payment": date("2025-04-14")}'
engine.evaluateService("My Service", inputs);

evaluateFeel("sum([1, 2, x])", { x: 3 });    // 6
```

Conversions:

* results: numbers become JavaScript `number`s, lists arrays, contexts
  objects, temporal values ISO 8601 strings (`"2025-04-14"`, `"P1Y2M"`),
  ranges and functions their FEEL text. `evaluateToFeel` returns the exact
  value in FEEL syntax (34 digit decimals, typed temporal values).
* inputs: converted according to the types declared in the model (also inside
  structures and lists). A string for an input of type `date`, `time`,
  `date and time` or a duration type is parsed as such; a string for a
  `number` is read as an exact decimal (`"0.1"`); JavaScript `Date`s become
  date-times in UTC.

The functions throw on invalid models, unknown decisions and invalid inputs.

The module also works in Node.js: `initSync({ module: readFileSync(".../rsdmn_wasm_bg.wasm") })`.

## Tests

```sh
node test/api.mjs       # API smoke test (Node.js)
node test/tck.mjs       # the DMN TCK, run by the WebAssembly build
node test/browser.mjs   # the example in headless Chromium (needs Playwright)
```
