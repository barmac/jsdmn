/**
 * Fetches the DMN TCK (https://github.com/dmn-tck/tck) test cases into
 * `tmp/dmn-tck` (or `$TCK_DIR`).
 *
 * The TCK evolves over time; by default its latest revision (`master`) is
 * fetched. An existing checkout is updated in place if the TCK moved on and
 * reused as-is if it cannot be reached (e.g. offline).
 *
 * Usage:
 *
 *   node tasks/fetch-tck.js            fetch / update the TCK
 *   node tasks/fetch-tck.js --resolve  print the commit TCK_REF resolves to
 *
 * Environment:
 *
 *   TCK_DIR - checkout location (default: tmp/dmn-tck)
 *   TCK_REF - branch, tag or commit to fetch (default: master)
 */

import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, rmSync } from 'node:fs';
import { resolve } from 'node:path';

const TCK_REPOSITORY = 'https://github.com/dmn-tck/tck.git';

const TCK_REF = process.env.TCK_REF || 'master';

const SPARSE_PATHS = [ '/TestCases/', '/LICENSE-ASL-2.0.txt', '/README.md' ];

const root = resolve(new URL('..', import.meta.url).pathname);

const target = resolve(root, process.env.TCK_DIR || 'tmp/dmn-tck');

function git(args, cwd = target) {
  return execFileSync('git', args, {
    cwd,
    encoding: 'utf8',
    stdio: [ 'ignore', 'pipe', 'pipe' ]
  }).trim();
}

/**
 * The ref does not exist (as opposed to the repository not being reachable).
 */
class UnknownRefError extends Error {}

/**
 * Resolve a branch, tag or commit of the TCK repository to a commit.
 *
 * @param {string} ref
 *
 * @return {string} commit SHA
 */
function resolveRef(ref) {

  if (/^[0-9a-f]{40}$/i.test(ref)) {
    return ref.toLowerCase();
  }

  const output = git([ 'ls-remote', TCK_REPOSITORY, `refs/heads/${ ref }`, `refs/tags/${ ref }^{}`, `refs/tags/${ ref }` ], root);

  const refs = Object.fromEntries(output.split('\n').filter(Boolean).map(line => {
    const [ sha, name ] = line.split('\t');

    return [ name, sha ];
  }));

  const sha = refs[`refs/heads/${ ref }`] || refs[`refs/tags/${ ref }^{}`] || refs[`refs/tags/${ ref }`];

  if (!sha) {
    throw new UnknownRefError(`cannot resolve <${ ref }> in ${ TCK_REPOSITORY }`);
  }

  return sha;
}

function currentRevision() {

  if (!existsSync(resolve(target, '.git'))) {
    return null;
  }

  try {
    return git([ 'rev-parse', 'HEAD' ]);
  } catch {
    return null;
  }
}

function checkout(sha) {

  if (!existsSync(resolve(target, '.git'))) {
    rmSync(target, { recursive: true, force: true });
    mkdirSync(target, { recursive: true });

    git([ 'init', '--quiet' ]);
    git([ 'remote', 'add', 'origin', TCK_REPOSITORY ]);
    git([ 'sparse-checkout', 'set', '--no-cone', ...SPARSE_PATHS ]);
  }

  git([ 'fetch', '--quiet', '--depth', '1', '--filter=blob:none', 'origin', sha ]);
  git([ 'checkout', '--quiet', '--force', 'FETCH_HEAD' ]);
}


if (process.argv.includes('--resolve')) {
  console.log(resolveRef(TCK_REF));

  process.exit(0);
}

const current = currentRevision();

let sha;

try {
  sha = resolveRef(TCK_REF);
} catch (error) {

  // reuse an existing checkout if the TCK cannot be reached
  if (current && !(error instanceof UnknownRefError)) {
    console.warn(`cannot resolve DMN TCK <${ TCK_REF }> (${ error.message.split('\n')[0] }), using existing checkout @ ${ current }`);

    process.exit(0);
  }

  throw error;
}

if (current === sha) {
  console.log(`DMN TCK <${ TCK_REF }> @ ${ sha } is up to date in ${ target }`);

  process.exit(0);
}

console.log(`fetching DMN TCK <${ TCK_REF }> @ ${ sha } into ${ target }${ current ? ` (was @ ${ current })` : '' }`);

checkout(sha);

console.log(`fetched DMN TCK @ ${ currentRevision() }`);
