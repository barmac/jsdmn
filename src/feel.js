/**
 * Bridge between the DMN engine and the FEEL interpreter (feelin).
 *
 * All FEEL evaluation done by the engine goes through this module so that
 * syntax errors and interpreter warnings are reported consistently.
 */

import {
  evaluate as feelEvaluate,
  unaryTest as feelUnaryTest,
  FeelFunction,
  FeelRange,
  FeelDate,
  FeelTime,
  FeelDateTime,
  FeelDuration
} from 'feelin';

export {
  FeelFunction,
  FeelRange,
  FeelDate,
  FeelTime,
  FeelDateTime,
  FeelDuration
};


/**
 * Evaluate a FEEL expression.
 *
 * @param {string} text
 * @param {object} scope
 * @param {import('./evaluation.js').Evaluation} [evaluation]
 * @param {object} [element] element the expression belongs to (for reporting)
 *
 * @return {any}
 */
export function evaluateExpression(text, scope, evaluation, element) {

  if (isBlank(text)) {
    return null;
  }

  try {
    const { value, warnings } = feelEvaluate(text, scope);

    report(evaluation, element, text, warnings, value);

    return normalize(value);
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
 *
 * @return {boolean|null}
 */
export function evaluateUnaryTests(text, value, scope, evaluation, element) {

  if (isBlank(text) || text.trim() === '-') {
    return true;
  }

  try {
    const { value: result, warnings } = feelUnaryTest(text, {
      ...scope,
      '?': value === undefined ? null : value
    });

    report(evaluation, element, text, warnings, result);

    return normalize(result);
  } catch (error) {
    evaluation?.error(`failed to evaluate unary tests <${ text }>: ${ error.message }`, element);

    return null;
  }
}

/**
 * FEEL equality (`a = b`).
 *
 * @return {boolean|null}
 */
export function feelEquals(a, b) {
  return normalize(feelEvaluate('a = b', { a, b }).value);
}

/**
 * FEEL `<` comparison.
 *
 * @return {boolean|null}
 */
export function feelLessThan(a, b) {
  return normalize(feelEvaluate('a < b', { a, b }).value);
}


// functions ///////////////

/**
 * feelin signals failed invocations through an (unexported) InvocationFailure
 * value which its interpreter turns into a warning plus `null`. Obtain the
 * class from a deliberately mismatched invocation so that engine functions
 * can signal failures the same way.
 */
const InvocationFailure = new FeelFunction(() => null, []).invoke([ null ]).constructor;

/**
 * @param {string} message
 */
export function invocationFailure(message) {
  return new InvocationFailure('FUNCTION_INVOCATION_FAILURE', String(message).replace(/[{}]/g, ''), {});
}

export function isInvocationFailure(value) {
  return value instanceof InvocationFailure;
}

/**
 * A FEEL function implemented by the engine (BKMs, decision services,
 * boxed function definitions).
 *
 * Positional invocations must match the number of parameters exactly;
 * named invocations may omit parameters (bound to `null`) but must not
 * use unknown names.
 */
export class DmnFunction extends FeelFunction {

  /**
   * @param {(args: any[]) => any} fn receives the arguments as array
   * @param {string[]} parameterNames
   * @param {object} [options]
   * @param {object} [options.type] the function's FEEL type
   */
  constructor(fn, parameterNames, options = {}) {
    super((...args) => fn(args), parameterNames);

    Object.defineProperty(this, 'type', { value: options.type || null, enumerable: false });
  }

  invoke(contextOrArgs) {

    const names = this.parameterNames;

    if (Array.isArray(contextOrArgs)) {

      if (contextOrArgs.length !== names.length) {
        return invocationFailure(`expected ${ names.length } parameter(s), got ${ contextOrArgs.length }`);
      }

      return this.fn(...contextOrArgs);
    }

    for (const key of Object.keys(contextOrArgs)) {
      if (!names.includes(key)) {
        return invocationFailure(`unknown parameter <${ key }>`);
      }
    }

    return this.fn(...names.map(name => contextOrArgs[name] ?? null));
  }
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

  if (typeof fn === 'function') {
    fn = new FeelFunction(fn, fn.$args || Array.from({ length: fn.length }, (_, idx) => `arg${ idx }`));
  }

  if (!(fn instanceof FeelFunction)) {
    evaluation?.error(`cannot invoke non-function value <${ describe(fn) }>`, element);

    return null;
  }

  try {
    const result = fn.invoke(args);

    if (isInvocationFailure(result)) {
      evaluation?.error(`function invocation failed: ${ formatFailure(result) }`, element);

      return null;
    }

    return normalize(result);
  } catch (error) {
    evaluation?.error(`function invocation failed: ${ error.message }`, element);

    return null;
  }
}

function formatFailure(failure) {

  const { template = 'invocation failure', values = {} } = failure;

  return template.replace(/\{(\w+)\}/g, (_, key) => describe(values[key]));
}


// values ///////////////

/**
 * Return the FEEL type name of a value.
 *
 * @param {any} value
 *
 * @return {string}
 */
export function getFeelType(value) {

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

  if (value instanceof FeelDateTime) {
    return 'date and time';
  }

  if (value instanceof FeelDate) {
    return 'date';
  }

  if (value instanceof FeelTime) {
    return 'time';
  }

  if (value instanceof FeelDuration) {
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
  return getFeelType(value) === 'context';
}

export function isFeelFunction(value) {
  return getFeelType(value) === 'function';
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

  const type = getFeelType(value);

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

function normalize(value) {
  return value === undefined ? null : value;
}

/**
 * Report problems encountered while evaluating FEEL. They are errors if
 * they caused the expression to evaluate to `null`, warnings otherwise.
 */
function report(evaluation, element, text, warnings, result) {

  if (!evaluation || !warnings || !warnings.length) {
    return;
  }

  const severity = result === null || result === undefined ? 'error' : 'warning';

  for (const warning of warnings) {
    evaluation.addMessage(severity, `${ warning.message } (in <${ text }>)`, element, warning.type);
  }
}
