// Smoke test of the WebAssembly API (the browser build in ../www/pkg, run in Node.js).
import { readFileSync } from "node:fs";
import assert from "node:assert/strict";
import { initSync, DmnEngine, evaluateFeel, evaluateFeelToFeel, version } from "../www/pkg/rsdmn_wasm.js";

initSync({ module: readFileSync(new URL("../www/pkg/rsdmn_wasm_bg.wasm", import.meta.url)) });

const fixture = (name) => readFileSync(new URL(`../../../test/fixtures/${name}`, import.meta.url), "utf8");

assert.equal(typeof version(), "string");

// FEEL
assert.equal(evaluateFeel("1 + 2 * 3"), 7);
assert.deepEqual(evaluateFeel("for x in 1..3 return x * 2"), [2, 4, 6]);
assert.equal(evaluateFeel("a + b", { a: 1, b: 2 }), 3);
assert.equal(evaluateFeel("date(\"2021-01-31\") + duration(\"P1M\")"), "2021-02-28");
assert.equal(evaluateFeelToFeel("1 / 3"), "0.3333333333333333333333333333333333");
assert.equal(evaluateFeel("today() instance of date"), true);
assert.deepEqual(evaluateFeel("{a: 1, b: [true, null]}"), { a: 1, b: [true, null] });
assert.throws(() => evaluateFeel("1 +"));

// DMN models
const engine = new DmnEngine([fixture("requirements.dmn")]);
assert.ok(engine.decisions().some((d) => d.name === "Average"));
assert.equal(engine.evaluate("Average", { a: 3, b: 5 }), 4);
assert.equal(new DmnEngine([fixture("simple.dmn")]).evaluate("Decision", {}), 4);
assert.deepEqual(new DmnEngine([fixture("context.dmn")]).evaluate("Decision with multiple entries", {}), { first: 4, second: 6 });
assert.throws(() => engine.evaluate("No such decision", {}));
assert.throws(() => new DmnEngine(["<not dmn"]));

console.log("wasm API tests passed");
