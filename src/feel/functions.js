/**
 * FEEL function values and their invocation.
 */

import { FeelFunction, FeelRange } from 'feelin';

export { FeelFunction };

/**
 * feelin signals failed invocations through an (unexported) InvocationFailure
 * value. Obtain the class from a deliberately mismatched invocation so that
 * engine functions can signal failures the same way.
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
 * Format the message of an invocation failure.
 */
export function formatFailure(failure) {
  const { template = 'invocation failure', values = {} } = failure;

  return template.replace(/\{(\w+)\}/g, (_, key) => {
    const value = values[key];

    return typeof value === 'string' ? `"${ value }"` : String(value);
  });
}


/**
 * A FEEL function implemented by the engine (FEEL function definitions,
 * BKMs, decision services, ...).
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
   * @param {boolean} [options.variadic] whether the last parameter is variadic
   */
  constructor(fn, parameterNames, options = {}) {
    super((...args) => fn(args), parameterNames);

    Object.defineProperty(this, 'type', { value: options.type || null, enumerable: false });
    Object.defineProperty(this, 'variadic', { value: !!options.variadic, enumerable: false });
  }

  invoke(contextOrArgs) {

    const names = this.parameterNames;

    if (Array.isArray(contextOrArgs)) {

      if (this.variadic) {
        const fixed = names.length - 1;

        if (contextOrArgs.length < fixed) {
          return invocationFailure(`expected at least ${ fixed } parameter(s), got ${ contextOrArgs.length }`);
        }

        return this.fn(...contextOrArgs.slice(0, fixed), contextOrArgs.slice(fixed));
      }

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
 * Convert a value into an invocable FEEL function, if possible.
 *
 * @return {FeelFunction|null}
 */
export function toFunction(value) {

  if (value instanceof FeelFunction) {
    return value;
  }

  if (typeof value === 'function') {
    return new FeelFunction(value, value.$args || parseParameterNames(value));
  }

  return null;
}

/**
 * Invoke a function value.
 *
 * @param {any} fn
 * @param {any[]|object} args positional or named arguments
 *
 * @return { { value: any, error?: string } }
 */
export function invoke(fn, args) {

  if (fn instanceof FeelRange) {
    return { error: 'cannot invoke a range' };
  }

  const feelFunction = toFunction(fn);

  if (!feelFunction) {
    return { error: 'not a function' };
  }

  try {
    const result = feelFunction.invoke(args);

    if (isInvocationFailure(result)) {
      return { value: null, error: formatFailure(result) };
    }

    return { value: result === undefined ? null : result };
  } catch (error) {
    return { value: null, error: error.message };
  }
}

function parseParameterNames(fn) {

  const match = /^(?:[^(]*\s*)?\(([^)]*)\)/.exec(fn.toString());

  if (!match || !match[1].trim()) {
    return [];
  }

  return match[1].split(',').map(p => p.trim().replace(/\s*=.*$/, '')).filter(Boolean);
}
