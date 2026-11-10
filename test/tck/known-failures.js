/**
 * DMN TCK test cases that are known to fail, keyed by
 * `<folder>/<test file>/<test case id>` (`*` matches all test cases of a file).
 *
 * Each entry documents why the test case fails.
 */
export const KNOWN_FAILURES = {};

/**
 * @param {string} folder
 * @param {string} file
 * @param {string} id
 *
 * @return {string|null} reason
 */
export function getKnownFailure(folder, file, id) {
  return KNOWN_FAILURES[`${ folder }/${ file }/${ id }`] || KNOWN_FAILURES[`${ folder }/${ file }/*`] || null;
}
