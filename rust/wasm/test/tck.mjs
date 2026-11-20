// Runs the DMN TCK (compliance levels 2 and 3) with the WebAssembly build.
//
//   node test/tck.mjs [path-to-tck/TestCases]
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";
import { initSync, runTckSuite } from "../www/pkg/rsdmn_wasm.js";

initSync({ module: readFileSync(new URL("../www/pkg/rsdmn_wasm_bg.wasm", import.meta.url)) });

const root = process.argv[2] ?? new URL("../../../tck/TestCases", import.meta.url).pathname;
let total = 0;
let passed = 0;
const started = performance.now();
for (const level of ["compliance-level-2", "compliance-level-3"]) {
  const levelDir = join(root, level);
  for (const dir of readdirSync(levelDir).sort()) {
    const path = join(levelDir, dir);
    if (!statSync(path).isDirectory()) continue;
    const files = readdirSync(path);
    const read = (ext) => {
      const names = files.filter((f) => f.endsWith(ext)).sort();
      return [names, names.map((f) => readFileSync(join(path, f), "utf8"))];
    };
    const [modelFiles, models] = read(".dmn");
    const [testFiles, tests] = read(".xml");
    for (const r of runTckSuite(`${level}/${dir}`, modelFiles, models, testFiles, tests)) {
      total++;
      if (r.success) passed++;
      else console.log(`FAIL ${level}/${dir} ${r.file} ${r.id}: ${r.message}`);
    }
  }
}
const ms = (performance.now() - started).toFixed(0);
console.log(`${passed} / ${total} test cases passed (WebAssembly, ${ms} ms)`);
if (total < 3000 || passed !== total) process.exit(1);
