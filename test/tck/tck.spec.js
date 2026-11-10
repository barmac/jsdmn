/**
 * Runs the DMN TCK (https://github.com/dmn-tck/tck) against the engine.
 *
 * Fetch the TCK via `npm run tck:fetch`, then execute `npm run tck`.
 *
 * The TCK models are DMN 1.5 models. Besides running them as authored, the
 * runner re-expresses them in DMN 1.4 and DMN 1.3 (rewriting namespaces) and
 * runs them again; models using elements a version does not define are
 * skipped for that version.
 *
 * Environment:
 *
 *   TCK_DIR      - TCK checkout (default: tmp/dmn-tck)
 *   TCK_FILTER   - only run test folders whose name includes this string
 *   TCK_LEVELS   - comma separated test case folders to run
 *                  (default: compliance-level-2,compliance-level-3)
 *   TCK_RESULTS  - directory to write tck_results.csv to (default: tmp/tck-results)
 *   TCK_STRICT_ERRORS - additionally require that the engine reports an error
 *                  for result nodes marked with `errorResult="true"`
 *   TCK_DMN_VERSIONS - comma separated DMN versions to run the TCK as
 *                  (default: 1.5,1.4,1.3)
 */

import { existsSync, mkdirSync, readFileSync, readdirSync, statSync, writeFileSync } from 'node:fs';
import { basename, resolve } from 'node:path';

import { Engine } from '../../src/engine.js';

import { compare } from './compare.js';
import { parseTestSuite, UnparsableValue } from './test-cases.js';
import { getKnownFailure } from './known-failures.js';
import { TCK_DMN_VERSION, DMN_VERSIONS, convertModel, getUnsupportedElements } from './versions.js';

const ROOT = resolve(new URL('../..', import.meta.url).pathname);

const TCK_DIR = resolve(ROOT, process.env.TCK_DIR || 'tmp/dmn-tck');
const TEST_CASES_DIR = resolve(TCK_DIR, 'TestCases');
const RESULTS_DIR = resolve(ROOT, process.env.TCK_RESULTS || 'tmp/tck-results');

const LEVELS = (process.env.TCK_LEVELS || 'compliance-level-2,compliance-level-3').split(',');
const FILTER = process.env.TCK_FILTER || null;
const STRICT_ERRORS = !!process.env.TCK_STRICT_ERRORS;

const VERSIONS = (process.env.TCK_DMN_VERSIONS || [ ...DMN_VERSIONS ].reverse().join(',')).split(',');

for (const version of VERSIONS) {
  if (!DMN_VERSIONS.includes(version)) {
    throw new Error(`unsupported TCK_DMN_VERSIONS entry <${ version }>, expected one of ${ DMN_VERSIONS.join(', ') }`);
  }
}


describe('DMN TCK', function() {

  if (!existsSync(TEST_CASES_DIR)) {

    it('requires the DMN TCK', function() {
      throw new Error(`DMN TCK not found in <${ TCK_DIR }>, fetch it via <npm run tck:fetch>`);
    });

    return;
  }

  for (const version of VERSIONS) {

    describe(`DMN ${ version }`, function() {

      /**
       * @type { Array<{ folder: string, file: string, id: string, status: string, comment: string }> }
       */
      const results = [];

      after(function() {
        writeResults(version, results);
      });

      for (const level of LEVELS) {

        const levelDir = resolve(TEST_CASES_DIR, level);

        if (!existsSync(levelDir)) {
          continue;
        }

        describe(level, function() {

          for (const folder of listDirectories(levelDir)) {

            if (FILTER && !folder.includes(FILTER)) {
              continue;
            }

            describeTestFolder(version, results, level, folder, resolve(levelDir, folder));
          }
        });
      }
    });
  }
});


function describeTestFolder(version, results, level, folder, dir) {

  const files = readdirSync(dir).sort();

  const modelFiles = files.filter(file => file.endsWith('.dmn'));
  const testFiles = files.filter(file => /-test-\d+\.xml$/.test(file));

  const modelXml = Object.fromEntries(modelFiles.map(file => [ file, readFileSync(resolve(dir, file), 'utf8') ]));

  const unsupported = [ ...new Set(Object.values(modelXml).flatMap(xml => getUnsupportedElements(xml, version))) ];

  const notApplicable = unsupported.length
    ? `uses ${ unsupported.map(element => `<${ element }>`).join(', ') }, not defined in DMN ${ version }`
    : null;

  describe(folder, function() {

    const models = {};

    let engine;
    let loadError;

    before(async function() {

      if (notApplicable) {
        return;
      }

      engine = new Engine();

      try {
        for (const file of modelFiles) {
          models[file] = await engine.load(convertModel(modelXml[file], version));
        }
      } catch (error) {
        loadError = error;
      }
    });

    if (notApplicable) {

      it(`should reject models using elements not defined in DMN ${ version }`, async function() {

        for (const [ file, xml ] of Object.entries(modelXml)) {

          if (!getUnsupportedElements(xml, version).length) {
            continue;
          }

          let error;

          try {
            await new Engine().load(convertModel(xml, version));
          } catch (e) {
            error = e;
          }

          if (!error) {
            throw new Error(`expected <${ file }> to be rejected as DMN ${ version }`);
          }
        }
      });
    }

    for (const testFile of testFiles) {

      const suite = parseTestSuite(readFileSync(resolve(dir, testFile), 'utf8'));

      const fileName = basename(testFile, '.xml');

      describe(fileName, function() {

        for (const testCase of suite.testCases) {

          const knownFailure = getKnownFailure(folder, fileName, testCase.id);

          const title = `${ testCase.id }${ testCase.description ? ` - ${ truncate(testCase.description) }` : '' }`;

          it(title, function() {

            const record = {
              folder: `${ level }/${ folder }`,
              file: fileName,
              id: testCase.id,
              status: 'SUCCESS',
              comment: ''
            };

            results.push(record);

            if (notApplicable) {
              record.status = 'IGNORED';
              record.comment = notApplicable;

              this.skip();
            }

            try {

              if (loadError) {
                throw loadError;
              }

              runTestCase(engine, models[suite.modelName], testCase);

              if (knownFailure) {
                record.comment = `passes, but listed as known failure: ${ knownFailure }`;
              }
            } catch (error) {
              record.status = knownFailure ? 'IGNORED' : 'ERROR';
              record.comment = knownFailure || error.message.split('\n')[0];

              if (knownFailure) {
                this.skip();
              }

              throw error;
            }
          });
        }
      });
    }
  });
}


/**
 * @param {Engine} engine
 * @param {import('../../src/model.js').Model} model
 * @param {import('./test-cases.js').TestCase} testCase
 */
function runTestCase(engine, model, testCase) {

  if (!model) {
    throw new Error('model not found');
  }

  const inputs = {};

  for (const input of testCase.inputs) {

    const unparsable = findUnparsable(input.value);

    if (unparsable) {
      throw new Error(`input <${ input.name }>: ${ unparsable }`);
    }

    const key = input.namespace && input.namespace !== model.namespace
      ? `{${ input.namespace }}${ input.name }`
      : input.name;

    inputs[key] = input.value;
  }

  const failures = [];
  const messages = [];

  const check = (result, actual, evaluationMessages) => {
    const difference = compare(actual, result.expected);

    if (difference) {
      failures.push(`<${ result.name }> ${ difference }`);
    }

    if (STRICT_ERRORS && result.errorResult && !evaluationMessages.some(m => m.severity === 'error' || m.severity === 'warning')) {
      failures.push(`<${ result.name }> expected an error to be reported`);
    }
  };

  if (testCase.type === 'decisionService') {
    const namespace = model.namespace;

    const decisionService = engine.loader.getDecisionService(testCase.invocableName, namespace);

    if (!decisionService) {
      throw new Error(`decision service <${ testCase.invocableName }> not found`);
    }

    const { value, messages: evaluationMessages } = engine.evaluateSync('decisionService', testCase.invocableName, inputs, { namespace });

    messages.push(...evaluationMessages);

    for (const result of testCase.results) {
      const actual = decisionService.outputDecisions.length > 1 ? value?.[result.name] ?? null : value;

      check(result, actual, evaluationMessages);
    }
  } else if (testCase.type === 'bkm') {

    const { value, messages: evaluationMessages } = engine.evaluateSync('businessKnowledgeModel', testCase.invocableName, inputs, {
      namespace: model.namespace
    });

    messages.push(...evaluationMessages);

    for (const result of testCase.results) {
      check(result, value, evaluationMessages);
    }
  } else {

    for (const result of testCase.results) {

      const namespace = result.namespace || (model.getDecision(result.name) ? model.namespace : undefined);

      const { value, messages: evaluationMessages } = engine.evaluateSync('decision', result.name, inputs, { namespace });

      messages.push(...evaluationMessages);

      check(result, value, evaluationMessages);
    }
  }

  if (failures.length) {
    const details = messages.slice(0, 5).map(message => `  [${ message.severity }] ${ message.message }`);

    throw new Error([ ...failures, ...(details.length ? [ 'messages:', ...details ] : []) ].join('\n'));
  }
}


// helpers ///////////////

function findUnparsable(value) {

  if (value instanceof UnparsableValue) {
    return value;
  }

  if (value && typeof value === 'object') {
    for (const nested of Object.values(value)) {
      const unparsable = findUnparsable(nested);

      if (unparsable) {
        return unparsable;
      }
    }
  }

  return null;
}

function listDirectories(dir) {
  return readdirSync(dir)
    .filter(name => statSync(resolve(dir, name)).isDirectory())
    .sort();
}

function truncate(str, length = 80) {
  str = str.replace(/\s+/g, ' ');

  return str.length > length ? `${ str.slice(0, length - 3) }...` : str;
}

function writeResults(version, results) {

  if (!results.length) {
    return;
  }

  mkdirSync(RESULTS_DIR, { recursive: true });

  // results of the models as authored use the name of the official TCK results
  const suffix = version === TCK_DMN_VERSION ? '' : `-dmn${ version.replace('.', '') }`;

  const csv = results.map(({ folder, file, id, status, comment }) => {
    return [ folder, file, id, status, comment ].map(value => `"${ String(value).replace(/"/g, '\'') }"`).join(',');
  }).join('\n');

  writeFileSync(resolve(RESULTS_DIR, `tck_results${ suffix }.csv`), csv + '\n', 'utf8');

  const counts = results.reduce((counts, { status }) => {
    counts[status] = (counts[status] || 0) + 1;

    return counts;
  }, {});

  const summary = {
    total: results.length,
    success: counts.SUCCESS || 0,
    error: counts.ERROR || 0,
    ignored: counts.IGNORED || 0
  };

  writeFileSync(resolve(RESULTS_DIR, `summary${ suffix }.json`), JSON.stringify({ version, ...summary }, null, 2) + '\n', 'utf8');

  console.log(
    `\nDMN TCK as DMN ${ version }: ${ summary.success } / ${ summary.total } test cases passed` +
    `${ summary.error ? `, ${ summary.error } failed` : '' }` +
    `${ summary.ignored ? `, ${ summary.ignored } skipped (known failures or elements not defined in DMN ${ version })` : '' }` +
    ` (results in ${ RESULTS_DIR })\n`
  );
}
