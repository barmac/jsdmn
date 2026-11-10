/**
 * List and boolean builtins whose semantics differ from feelin's.
 */

import { builtin, invocationFailure } from './function.js';

import { invoke } from '../functions.js';

import { getType, equals } from '../values.js';

import { isDuration, isDate, isTime, isDateTime } from '../temporal.js';

const listReplace = builtin('list replace',
  {
    parameters: [ 'list', 'position', 'newItem' ],
    impl(list, positionOrMatch, newItem) {

      if (list === null || list === undefined) {
        return invocationFailure('list replace(list, ...) expects a list');
      }

      if (!Array.isArray(list)) {
        list = [ list ];
      }

      if (typeof positionOrMatch === 'number') {
        return replaceAt(list, positionOrMatch, newItem);
      }

      if (getType(positionOrMatch) === 'function') {
        return replaceMatching(list, positionOrMatch, newItem);
      }

      return invocationFailure('list replace(list, position, newItem) expects a position or a match function');
    }
  },
  {
    parameters: [ 'list', 'match', 'newItem' ],
    impl(list, match, newItem) {

      if (list === null || list === undefined || getType(match) !== 'function') {
        return invocationFailure('list replace(list, match, newItem) expects a list and a function');
      }

      return replaceMatching(Array.isArray(list) ? list : [ list ], match, newItem);
    }
  }
);

function replaceAt(list, position, newItem) {

  position = Math.trunc(position);

  if (position === 0 || Math.abs(position) > list.length) {
    return invocationFailure(`position ${ position } is out of bounds`);
  }

  const idx = position > 0 ? position - 1 : list.length + position;

  const result = [ ...list ];

  result[idx] = newItem ?? null;

  return result;
}

function replaceMatching(list, match, newItem) {

  const result = [];

  for (const item of list) {
    const { value, error } = invoke(match, [ item, newItem ?? null ]);

    if (error || typeof value !== 'boolean') {
      return invocationFailure('list replace match function must return a boolean');
    }

    result.push(value ? newItem ?? null : item);
  }

  return result;
}

/**
 * `is(value1, value2)`: whether both values are the same element of the
 * FEEL semantic domain (same type, same value, same time zone).
 */
const is = builtin('is', {
  parameters: [ 'value1?', 'value2?' ],
  impl(a, b) {

    a = a ?? null;
    b = b ?? null;

    if (getType(a) !== getType(b)) {
      return false;
    }

    if (isDuration(a)) {
      return a.yearsMonths === b.yearsMonths && equals(a, b) === true;
    }

    if (isTime(a) || isDateTime(a)) {
      return sameZone(a.zone, b.zone) && a.value.equals(b.value);
    }

    if (isDate(a)) {
      return a.value.equals(b.value);
    }

    return equals(a, b) === true;
  }
});

function sameZone(a, b) {

  const normalize = (zone) => zone === '+00:00' || zone === '-00:00' ? 'UTC' : zone;

  return normalize(a) === normalize(b);
}

export default {
  'list replace': listReplace,
  'is': is
};
