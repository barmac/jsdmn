/**
 * Fetches the DMN TCK (https://github.com/dmn-tck/tck) test cases
 * at a pinned revision into `tmp/dmn-tck` (or `$TCK_DIR`).
 *
 * Usage: node tasks/fetch-tck.js [--force]
 *
 * Environment:
 *
 *   TCK_DIR - checkout location (default: tmp/dmn-tck)
 *   TCK_REF - commit to check out (default: pinned below)
 */

import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, rmSync } from 'node:fs';
import { resolve } from 'node:path';

const TCK_REPOSITORY = 'https://github.com/dmn-tck/tck.git';

// dmn-tck/tck@master as of 2026-11-04
const TCK_REF = process.env.TCK_REF || '5cd912f281831bc535e710aba435e16de198028a';

const root = resolve(new URL('..', import.meta.url).pathname);

const target = resolve(root, process.env.TCK_DIR || 'tmp/dmn-tck');

const force = process.argv.includes('--force');

function git(...args) {
  return execFileSync('git', args, {
    cwd: target,
    encoding: 'utf8',
    stdio: [ 'ignore', 'pipe', 'inherit' ]
  }).trim();
}

function currentRevision() {
  try {
    return git('rev-parse', 'HEAD');
  } catch {
    return null;
  }
}

if (existsSync(target) && !force) {

  if (currentRevision() === TCK_REF) {
    console.log(`DMN TCK @ ${ TCK_REF } already present in ${ target }`);

    process.exit(0);
  }
}

if (existsSync(target)) {
  rmSync(target, { recursive: true, force: true });
}

mkdirSync(target, { recursive: true });

console.log(`fetching DMN TCK @ ${ TCK_REF } into ${ target }`);

git('init', '--quiet');
git('remote', 'add', 'origin', TCK_REPOSITORY);
git('sparse-checkout', 'set', '--no-cone', '/TestCases/', '/LICENSE-ASL-2.0.txt', '/README.md');
git('fetch', '--quiet', '--depth', '1', '--filter=blob:none', 'origin', TCK_REF);
git('checkout', '--quiet', 'FETCH_HEAD');

console.log(`fetched DMN TCK @ ${ currentRevision() }`);
