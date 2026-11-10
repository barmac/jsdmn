/**
 * FEEL builtin functions.
 *
 * Most builtins are provided by feelin; this module wraps them and
 * overrides / adds those whose DMN 1.5 semantics differ.
 */

import { evaluate as feelinEvaluate } from 'feelin';

import {
  invocationFailure,
  toFunction
} from './functions.js';

import { overrides } from './builtins/index.js';

const FEELIN_BUILTINS = [
  'number', 'string', 'date', 'date and time', 'time', 'duration', 'years and months duration',
  'now', 'today', 'not', 'substring', 'string length', 'upper case', 'lower case',
  'substring before', 'substring after', 'replace', 'contains', 'matches', 'starts with',
  'ends with', 'split', 'string join', 'list contains', 'list replace', 'count', 'min', 'max',
  'sum', 'mean', 'all', 'any', 'sublist', 'append', 'concatenate', 'insert before', 'remove',
  'reverse', 'index of', 'union', 'distinct values', 'flatten', 'product', 'median', 'stddev',
  'mode', 'decimal', 'floor', 'ceiling', 'abs', 'round up', 'round down', 'round half up',
  'round half down', 'modulo', 'sqrt', 'log', 'exp', 'odd', 'even', 'is', 'before', 'after',
  'meets', 'met by', 'overlaps', 'overlaps before', 'overlaps after', 'finishes', 'finished by',
  'includes', 'during', 'starts', 'started by', 'coincides', 'day of year', 'day of week',
  'month of year', 'week of year', 'sort', 'get value', 'get entries', 'context',
  'context merge', 'context put'
];

/**
 * @type {Map<string, import('./functions.js').FeelFunction>}
 */
const registry = new Map();

for (const name of FEELIN_BUILTINS) {

  let value;

  try {
    value = feelinEvaluate(name).value;
  } catch {
    continue;
  }

  const fn = toFunction(value);

  if (fn) {
    registry.set(name, fn);
  }
}

for (const [ name, fn ] of Object.entries(overrides)) {
  registry.set(name, fn);
}

/**
 * Get a builtin function by name.
 *
 * @param {string} name
 *
 * @return {import('./functions.js').FeelFunction|undefined}
 */
export function getBuiltin(name) {
  return registry.get(name);
}

/**
 * Register (or replace) a builtin function.
 *
 * @param {string} name
 * @param {import('./functions.js').FeelFunction} fn
 */
export function registerBuiltin(name, fn) {
  registry.set(name, fn);
}

/**
 * Whether the given value is a builtin function.
 *
 * @param {any} fn
 *
 * @return {boolean}
 */
export function isBuiltin(fn) {

  for (const builtin of registry.values()) {
    if (builtin === fn) {
      return true;
    }
  }

  return false;
}

export { invocationFailure };
