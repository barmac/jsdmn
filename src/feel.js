/**
 * Bridge between the DMN engine and the FEEL interpreter.
 *
 * All FEEL evaluation done by the engine goes through this module so that
 * syntax errors and interpreter warnings are reported consistently.
 */

import {
  evaluate as feelEvaluate,
  unaryTest as feelUnaryTest
} from './feel/interpreter.js';

import {
  getType,
  equals,
  compare
} from './feel/values.js';

import {
  DmnFunction,
  FeelFunction,
  invoke,
  invocationFailure,
  isInvocationFailure
} from './feel/functions.js';

import { Range } from './feel/range.js';

import {
  FeelDate,
  FeelTime,
  FeelDateTime,
  FeelDuration
} from './feel/temporal.js';

import { FeelRange } from 'feelin';

export {
  DmnFunction,
  FeelFunction,
  FeelRange,
  Range,
  FeelDate,
  FeelTime,
  FeelDateTime,
  FeelDuration,
  invocationFailure,
  isInvocationFailure
};


/**
 * Evaluate a FEEL expression.
 *
 * @param {string} text
 * @param {object} scope
 * @param {import('./evaluation.js').Evaluation} [evaluation]
 * @param {object} [element] element the expression belongs to (for reporting)
 * @param {import('./model.js').Model} [model] model to resolve type names in
 *
 * @return {any}
 */
export function evaluateExpression(text, scope, evaluation, element, model) {

  if (isBlank(text)) {
    return null;
  }

  try {
    const { value, warnings } = feelEvaluate(text, scope, getOptions(model));

    report(evaluation, element, text, warnings);

    return value;
  } catch (error) {
    evaluation?.error(`failed to evaluate expression <${ text }>: ${ error.message }`, element);

    return null;
  }
}

/**
 * Test a value against FEEL unary tests.
 *
 * @param {string} text
 * @param {any} value the input value (`?`)
 * @param {object} scope
 * @param {import('./evaluation.js').Evaluation} [evaluation]
 * @param {object} [element]
 * @param {import('./model.js').Model} [model]
 *
 * @return {boolean|null}
 */
export function evaluateUnaryTests(text, value, scope, evaluation, element, model) {

  if (isBlank(text) || text.trim() === '-') {
    return true;
  }

  try {
    const { value: result, warnings } = feelUnaryTest(text, value, scope, getOptions(model));

    report(evaluation, element, text, warnings);

    return result;
  } catch (error) {
    evaluation?.error(`failed to evaluate unary tests <${ text }>: ${ error.message }`, element);

    return null;
  }
}

function getOptions(model) {
  return model ? { resolveType: (name) => model.resolveTypeName(name) } : {};
}

/**
 * FEEL equality (`a = b`).
 *
 * @return {boolean|null}
 */
export function feelEquals(a, b) {
  return equals(a, b);
}

/**
 * FEEL `<` comparison.
 *
 * @return {boolean|null}
 */
export function feelLessThan(a, b) {
  const result = compare(a, b);

  return result === null ? null : result < 0;
}

/**
 * Invoke a FEEL function value with positional (array) or named (object)
 * parameters.
 *
 * @param {any} fn
 * @param {any[]|object} args
 * @param {import('./evaluation.js').Evaluation} [evaluation]
 * @param {object} [element]
 *
 * @return {any}
 */
export function invokeFunction(fn, args, evaluation, element) {

  if (getType(fn) !== 'function') {
    evaluation?.error(`cannot invoke non-function value <${ describe(fn) }>`, element);

    return null;
  }

  const { value, error } = invoke(fn, args);

  if (error) {
    evaluation?.error(`function invocation failed: ${ error }`, element);
  }

  return value;
}

/**
 * Return the FEEL type name of a value.
 *
 * @param {any} value
 *
 * @return {string}
 */
export function getFeelType(value) {
  return getType(value);
}

export function isContext(value) {
  return getType(value) === 'context';
}

export function isFeelFunction(value) {
  return getType(value) === 'function';
}

/**
 * Human readable representation of a value, used in messages.
 */
export function describe(value) {

  if (value === null || value === undefined) {
    return 'null';
  }

  if (typeof value === 'string') {
    return JSON.stringify(value);
  }

  if (Array.isArray(value)) {
    return `[${ value.map(describe).join(', ') }]`;
  }

  const type = getType(value);

  if (type === 'context') {
    return `{${ Object.entries(value).map(([ key, val ]) => `${ key }: ${ describe(val) }`).join(', ') }}`;
  }

  if (type === 'function') {
    return 'function';
  }

  return String(value);
}

export function isBlank(text) {
  return typeof text !== 'string' || !text.trim();
}

/**
 * Split comma separated (top-level) unary tests, e.g. `"a", [1..2], "b,c"`.
 *
 * @param {string} text
 *
 * @return {string[]}
 */
export function splitUnaryTests(text) {

  const tests = [];

  let depth = 0;
  let inString = false;
  let current = '';

  for (let idx = 0; idx < text.length; idx++) {
    const char = text[idx];

    if (inString) {
      current += char;

      if (char === '\\') {
        current += text[++idx] || '';
      } else if (char === '"') {
        inString = false;
      }

      continue;
    }

    if (char === '"') {
      inString = true;
    } else if ('([{'.includes(char)) {
      depth++;
    } else if (')]}'.includes(char)) {
      depth--;
    } else if (char === ',' && depth <= 0) {
      tests.push(current.trim());
      current = '';
      continue;
    }

    current += char;
  }

  if (current.trim()) {
    tests.push(current.trim());
  }

  return tests;
}


// helpers ///////////////

function report(evaluation, element, text, warnings) {

  if (!evaluation || !warnings || !warnings.length) {
    return;
  }

  for (const warning of warnings) {
    evaluation.warn(`${ warning.message } (in <${ text }>)`, element, warning.type);
  }
}
