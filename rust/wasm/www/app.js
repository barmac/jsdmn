// Browser example of the rsdmn WebAssembly build.
import init, { DmnEngine, evaluateFeel, evaluateFeelToFeel, version } from "./pkg/rsdmn_wasm.js";

const EXAMPLE_INPUTS = {
  Applicant: {
    age: 32,
    "monthly income": 5200,
    "credit score": 710,
    employment: "EMPLOYED",
  },
  "Requested Amount": 250000,
  Term: 360,
  "Application Date": "2025-03-14",
};

const $ = (id) => document.getElementById(id);
const ALL = "__all__";

let engine = null;

function setMessage(el, text, kind = "") {
  el.textContent = text;
  el.className = `message ${kind}`;
}

function debounce(fn, ms = 150) {
  let timer;
  return (...args) => {
    clearTimeout(timer);
    timer = setTimeout(() => fn(...args), ms);
  };
}

function formatTime(ms) {
  return ms < 1 ? `${(ms * 1000).toFixed(0)} µs` : `${ms.toFixed(2)} ms`;
}

function loadModel() {
  const xml = $("model-xml").value;
  const started = performance.now();
  try {
    const next = new DmnEngine([xml]);
    const elapsed = performance.now() - started;
    engine?.free();
    engine = next;
    const decisions = engine.decisions();
    const inputs = engine.inputData();
    setMessage(
      $("model-status"),
      `Compiled “${engine.modelNames()[0] || "model"}” in ${formatTime(elapsed)}: ` +
        `${decisions.length} decisions, ${inputs.length} input data.`,
      "ok",
    );
    renderInputTypes(inputs);
    renderDecisions(decisions);
    evaluate();
  } catch (e) {
    setMessage($("model-status"), `Cannot load the model: ${e.message ?? e}`, "error");
  }
}

function renderInputTypes(inputs) {
  const table = $("input-types");
  table.replaceChildren(
    ...inputs.map(({ name, type }) => {
      const row = document.createElement("tr");
      const n = document.createElement("td");
      n.textContent = name;
      const t = document.createElement("td");
      t.textContent = type ?? "Any";
      row.append(n, t);
      return row;
    }),
  );
}

function renderDecisions(decisions) {
  const select = $("decision");
  const previous = select.value;
  const options = [{ name: ALL, label: "All decisions" }, ...decisions.map((d) => ({ name: d.name, label: d.name }))];
  select.replaceChildren(
    ...options.map(({ name, label }) => {
      const o = document.createElement("option");
      o.value = name;
      o.textContent = label;
      return o;
    }),
  );
  if (options.some((o) => o.name === previous)) select.value = previous;
}

function readInputs() {
  const text = $("inputs-json").value.trim();
  if (!text) return {};
  try {
    const inputs = JSON.parse(text);
    setMessage($("inputs-status"), "");
    return inputs;
  } catch (e) {
    setMessage($("inputs-status"), `Invalid JSON: ${e.message}`, "error");
    return null;
  }
}

function evaluate() {
  if (!engine) return;
  const inputs = readInputs();
  if (inputs === null) return;
  const decision = $("decision").value;
  const feel = $("feel-syntax").checked;
  const started = performance.now();
  try {
    let text;
    if (decision === ALL) {
      if (feel) {
        text = engine
          .decisions()
          .map((d) => `${d.name}: ${engine.evaluateToFeel(d.name, inputs)}`)
          .join("\n\n");
      } else {
        text = JSON.stringify(engine.evaluateAll(inputs), null, 2);
      }
    } else {
      text = feel
        ? engine.evaluateToFeel(decision, inputs)
        : JSON.stringify(engine.evaluate(decision, inputs), null, 2);
    }
    const elapsed = performance.now() - started;
    $("result").textContent = text;
    setMessage($("result-status"), `Evaluated in ${formatTime(elapsed)}.`, "ok");
  } catch (e) {
    $("result").textContent = "";
    setMessage($("result-status"), e.message ?? String(e), "error");
  }
}

function evaluatePlayground() {
  let vars = {};
  const text = $("feel-vars").value.trim();
  try {
    vars = text ? JSON.parse(text) : {};
  } catch (e) {
    $("feel-result").textContent = `Invalid variables JSON: ${e.message}`;
    return;
  }
  try {
    const expr = $("feel-expr").value;
    const feel = evaluateFeelToFeel(expr, vars);
    const js = JSON.stringify(evaluateFeel(expr, vars));
    $("feel-result").textContent = `${feel}\n\nas JavaScript: ${js}`;
  } catch (e) {
    $("feel-result").textContent = e.message ?? String(e);
  }
}

async function loadExample() {
  const response = await fetch("example.dmn");
  $("model-xml").value = await response.text();
  $("inputs-json").value = JSON.stringify(EXAMPLE_INPUTS, null, 2);
  loadModel();
}

async function main() {
  const started = performance.now();
  await init();
  setMessage(
    $("status"),
    `rsdmn ${version()} (WebAssembly) ready in ${formatTime(performance.now() - started)}.`,
  );

  $("model-xml").addEventListener("input", debounce(loadModel, 300));
  $("inputs-json").addEventListener("input", debounce(evaluate));
  $("decision").addEventListener("change", evaluate);
  $("feel-syntax").addEventListener("change", evaluate);
  $("feel-expr").addEventListener("input", debounce(evaluatePlayground, 100));
  $("feel-vars").addEventListener("input", debounce(evaluatePlayground, 100));
  $("reset-example").addEventListener("click", loadExample);
  $("open-file").addEventListener("change", async (event) => {
    const file = event.target.files[0];
    if (!file) return;
    $("model-xml").value = await file.text();
    $("inputs-json").value = "{}";
    loadModel();
  });

  await loadExample();
  evaluatePlayground();
  document.body.dataset.ready = "true";
}

main().catch((e) => {
  setMessage($("status"), `Failed to start: ${e.message ?? e}`, "error");
});
