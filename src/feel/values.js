/**
 * FEEL value semantics: types, equality and ordering.
 */

import { FeelFunction, FeelRange } from 'feelin';

import {
  isDate,
  isTime,
  isDateTime,
  isDuration,
  compareTemporals,
  temporalEquals
} from './temporal.js';

import { toRange } from './range.js';


/**
 * Return the FEEL type name of a value.
 *
 * @param {any} value
 *
 * @return {string}
 */
export function getType(value) {

  if (value === null || value === undefined) {
    return 'Null';
  }

  switch (typeof value) {
  case 'number':
    return 'number';
  case 'string':
    return 'string';
  case 'boolean':
    return 'boolean';
  case 'function':
    return 'function';
  }

  if (isDateTime(value)) {
    return 'date and time';
  }

  if (isDate(value)) {
    return 'date';
  }

  if (isTime(value)) {
    return 'time';
  }

  if (isDuration(value)) {
    return value.yearsMonths ? 'years and months duration' : 'days and time duration';
  }

  if (Array.isArray(value)) {
    return 'list';
  }

  if (value instanceof FeelRange) {
    return 'range';
  }

  if (value instanceof FeelFunction) {
    return 'function';
  }

  return 'context';
}

export function isContext(value) {
  return getType(value) === 'context';
}

export function isFunction(value) {
  return getType(value) === 'function';
}

export function isNil(value) {
  return value === null || value === undefined;
}


// numbers ///////////////

/**
 * Relative tolerance for number equality; makes binary floating point
 * arithmetic behave closer to FEEL's decimal semantics (`0.1 + 0.2 = 0.3`).
 */
const NUMBER_TOLERANCE = 4 * Number.EPSILON;

export function numbersEqual(a, b) {

  if (a === b) {
    return true;
  }

  if (!Number.isFinite(a) || !Number.isFinite(b)) {
    return false;
  }

  return Math.abs(a - b) <= NUMBER_TOLERANCE * Math.max(Math.abs(a), Math.abs(b));
}

export function compareNumbers(a, b) {

  if (Number.isNaN(a) || Number.isNaN(b)) {
    return null;
  }

  if (numbersEqual(a, b)) {
    return 0;
  }

  return a < b ? -1 : 1;
}


// equality ///////////////

/**
 * FEEL equality (`=`).
 *
 * @return {boolean|null} `null` if values of different types are compared
 */
export function equals(a, b) {

  if (isNil(a) || isNil(b)) {
    return isNil(a) && isNil(b);
  }

  const typeA = getType(a);
  const typeB = getType(b);

  if (typeA !== typeB) {

    if (isDuration(a) && isDuration(b)) {
      return temporalEquals(a, b);
    }

    return null;
  }

  switch (typeA) {

  case 'number':
    return numbersEqual(a, b);

  case 'string':
  case 'boolean':
    return a === b;

  case 'date':
  case 'time':
  case 'date and time':
  case 'days and time duration':
  case 'years and months duration':
    return temporalEquals(a, b);

  case 'list':
    return listsEqual(a, b);

  case 'context':
    return contextsEqual(a, b);

  case 'range':
    return rangesEqual(a, b);

  case 'function':
    return a === b;

  default:
    return null;
  }
}

function combine(results) {

  if (results.includes(false)) {
    return false;
  }

  if (results.includes(null)) {
    return null;
  }

  return true;
}

function listsEqual(a, b) {

  if (a.length !== b.length) {
    return false;
  }

  return combine(a.map((element, idx) => equals(element, b[idx])));
}

function contextsEqual(a, b) {

  const keysA = Object.keys(a);
  const keysB = Object.keys(b);

  if (keysA.length !== keysB.length || !keysA.every(key => Object.prototype.hasOwnProperty.call(b, key))) {
    return false;
  }

  return combine(keysA.map(key => equals(a[key], b[key])));
}

function rangesEqual(a, b) {

  a = toRange(a);
  b = toRange(b);

  if (
    a.operator !== b.operator ||
    a.startUnbounded !== b.startUnbounded ||
    a.endUnbounded !== b.endUnbounded ||
    a['start included'] !== b['start included'] ||
    a['end included'] !== b['end included']
  ) {
    return false;
  }

  return combine([
    a.startUnbounded || equals(a.start, b.start),
    a.endUnbounded || equals(a.end, b.end)
  ]);
}


// ordering ///////////////

/**
 * Compare two values for ordering (`<`, `<=`, `>`, `>=`).
 *
 * @return {number|null} negative, zero or positive; `null` if not comparable
 */
export function compare(a, b) {

  if (isNil(a) || isNil(b)) {
    return null;
  }

  const typeA = getType(a);
  const typeB = getType(b);

  if (typeA === 'number' && typeB === 'number') {
    return compareNumbers(a, b);
  }

  if (typeA === 'string' && typeB === 'string') {
    return a < b ? -1 : a > b ? 1 : 0;
  }

  if (typeA !== typeB) {
    return null;
  }

  switch (typeA) {
  case 'date':
  case 'time':
  case 'date and time':
  case 'days and time duration':
  case 'years and months duration':
    return compareTemporals(a, b);
  default:
    return null;
  }
}


// ranges ///////////////

/**
 * Whether a range includes a point.
 *
 * @return {boolean|null}
 */
export function rangeIncludes(range, value) {

  range = toRange(range);

  if (range.operator === '!=') {
    const result = equals(value, range.start);

    return result === null ? null : !result;
  }

  if (range.operator === '=') {
    return equals(value, range.start);
  }

  if (isNil(value)) {
    return null;
  }

  if (!range.startUnbounded) {
    const result = compare(value, range.start);

    if (result === null) {
      return null;
    }

    if (range['start included'] ? result < 0 : result <= 0) {
      return false;
    }
  }

  if (!range.endUnbounded) {
    const result = compare(value, range.end);

    if (result === null) {
      return null;
    }

    if (range['end included'] ? result > 0 : result >= 0) {
      return false;
    }
  }

  return true;
}
