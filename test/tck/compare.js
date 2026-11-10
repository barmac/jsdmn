/**
 * Structural comparison of actual and expected TCK values.
 */

import { getFeelType, describe, feelEquals } from '../../src/feel.js';

import { ExpectedNumber, UnparsableValue } from './test-cases.js';

/**
 * Relative tolerance used to compare numbers. FEEL numbers are decimal128
 * (34 significant digits), the engine computes with IEEE 754 doubles.
 */
const NUMBER_TOLERANCE = 1e-12;

/**
 * Expected numbers given with at least this many fraction digits are
 * compared against the actual value rounded to that scale (the TCK gives
 * results of transcendental functions, e.g. `exp(4)` or Java's `Math.cos`,
 * to 7 - 8 digits).
 */
const MIN_ROUNDING_SCALE = 7;

/**
 * Compare two values, returning a description of the first
 * difference found or `null` if they match.
 *
 * @param {any} actual
 * @param {any} expected
 * @param {string} [path]
 *
 * @return {string|null}
 */
export function compare(actual, expected, path = '') {

  if (expected instanceof UnparsableValue) {
    return `${ path || 'value' }: ${ expected }`;
  }

  if (expected instanceof ExpectedNumber) {

    if (typeof actual === 'number' && numbersEqual(actual, expected.value, expected.scale)) {
      return null;
    }

    return `${ path || 'value' }: expected <${ expected.value }> (number), got <${ describe(actual) }> (${ getFeelType(actual) })`;
  }

  const actualType = getFeelType(actual);
  const expectedType = getFeelType(expected);

  const mismatch = () => `${ path || 'value' }: expected <${ describe(expected) }> (${ expectedType }), got <${ describe(actual) }> (${ actualType })`;

  if (actualType !== expectedType) {
    return mismatch();
  }

  switch (expectedType) {

  case 'Null':
    return null;

  case 'number':
    return numbersEqual(actual, expected) ? null : mismatch();

  case 'string':
  case 'boolean':
    return actual === expected ? null : mismatch();

  case 'list': {
    if (actual.length !== expected.length) {
      return `${ path || 'value' }: expected list of ${ expected.length } (${ describe(expected) }), got ${ actual.length } (${ describe(actual) })`;
    }

    for (let idx = 0; idx < expected.length; idx++) {
      const difference = compare(actual[idx], expected[idx], `${ path }[${ idx }]`);

      if (difference) {
        return difference;
      }
    }

    return null;
  }

  case 'context': {
    const actualKeys = Object.keys(actual);
    const expectedKeys = Object.keys(expected);

    const missing = expectedKeys.filter(key => !actualKeys.includes(key));
    const unexpected = actualKeys.filter(key => !expectedKeys.includes(key));

    if (missing.length || unexpected.length) {
      return `${ path || 'value' }: expected keys <${ expectedKeys.join(', ') }>, got <${ actualKeys.join(', ') }>`;
    }

    for (const key of expectedKeys) {
      const difference = compare(actual[key], expected[key], path ? `${ path }.${ key }` : key);

      if (difference) {
        return difference;
      }
    }

    return null;
  }

  default:

    // temporal values: same type, equal value
    return feelEquals(actual, expected) === true
      ? null
      : mismatch();
  }
}

function numbersEqual(a, b, scale = 0) {

  if (Number.isNaN(a) || Number.isNaN(b)) {
    return Number.isNaN(a) && Number.isNaN(b);
  }

  if (a === b) {
    return true;
  }

  if (Math.abs(a - b) <= NUMBER_TOLERANCE * Math.max(1, Math.abs(a), Math.abs(b))) {
    return true;
  }

  if (scale < MIN_ROUNDING_SCALE) {
    return false;
  }

  // rounded or truncated to the scale of the expected value
  const factor = 10 ** scale;

  return Number(a.toFixed(scale)) === b || Math.trunc(a * factor) / factor === b;
}
