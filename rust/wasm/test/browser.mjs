// End-to-end test of the browser example (www/) with headless Chromium.
//
//   node test/browser.mjs            (requires Playwright, e.g. `npm i -g playwright`)
import { createRequire } from "node:module";
import { spawn } from "node:child_process";
import assert from "node:assert/strict";

const require = createRequire(import.meta.url);
const { chromium } = require("playwright");

const root = new URL("../www/", import.meta.url).pathname;
const port = 8000 + Math.floor(Math.random() * 1000);
const server = spawn("python3", ["-m", "http.server", String(port), "--bind", "127.0.0.1"], { cwd: root, stdio: "ignore" });

async function waitForServer() {
  for (let i = 0; i < 100; i++) {
    try {
      await fetch(`http://127.0.0.1:${port}/`);
      return;
    } catch {
      await new Promise((r) => setTimeout(r, 100));
    }
  }
  throw new Error("server did not start");
}

const browser = await chromium.launch();
try {
  await waitForServer();
  const page = await browser.newPage();
  const errors = [];
  page.on("pageerror", (e) => errors.push(e.message));
  page.on("console", (m) => m.type() === "error" && errors.push(m.text()));

  await page.goto(`http://127.0.0.1:${port}/`);
  await page.waitForSelector("body[data-ready=true]", { timeout: 30000 });

  // all decisions of the example model
  let result = JSON.parse(await page.textContent("#result"));
  assert.equal(result.Eligibility, "ELIGIBLE");
  assert.equal(result["Risk Category"], "MEDIUM");
  assert.equal(result.Summary["monthly payment"], 1458.93);
  assert.equal(result.Summary["last payment"], "2055-03-14");
  assert.match(await page.textContent("#model-status"), /8 decisions, 4 input data/);

  // editing the inputs re-evaluates
  const inputs = JSON.parse(await page.inputValue("#inputs-json"));
  inputs.Applicant.age = 16;
  await page.fill("#inputs-json", JSON.stringify(inputs));
  await page.waitForFunction(() => document.querySelector("#result").textContent.includes("minor"));

  // a single decision in FEEL syntax
  await page.selectOption("#decision", "First Payment Date");
  await page.check("#feel-syntax");
  await page.waitForFunction(() => document.querySelector("#result").textContent === 'date("2025-04-14")');

  // FEEL playground
  assert.match(await page.textContent("#feel-result"), /\[date\("2025-02-28"\), date\("2025-03-28"\)\]/);
  await page.fill("#feel-expr", "sum([1, 2, x])");
  await page.waitForFunction(() => document.querySelector("#feel-result").textContent.startsWith("44"));

  // a broken model is reported
  await page.fill("#model-xml", "<definitions");
  await page.waitForFunction(() => document.querySelector("#model-status").textContent.startsWith("Cannot load"));

  assert.deepEqual(errors, []);
  console.log("browser example tests passed");
} finally {
  await browser.close();
  server.kill();
}
