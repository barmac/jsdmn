/**
 * Infrastructure for defining builtin functions.
 */

import { FeelFunction } from 'feelin';

import { invocationFailure } from '../functions.js';

/**
 * @typedef {object} Signature
 * @property {string[]} parameters parameter names; optional ones end with `?`,
 *           a variadic last one starts with `...`
 * @property {(...args: any[]) => any} impl
 */

/**
 * A builtin function supporting overloads, optional and variadic
 * parameters as well as positional and named invocation.
 */
export class BuiltinFunction extends FeelFunction {

  /**
   * @param {string} name
   * @param {Signature[]} signatures
   */
  constructor(name, signatures) {

    const parsed = signatures.map(parseSignature);

    super((...args) => null, parsed[0].names);

    Object.defineProperty(this, 'name', { value: name, enumerable: false });
    Object.defineProperty(this, 'signatures', { value: parsed, enumerable: false });
  }

  invoke(contextOrArgs) {

    const positional = Array.isArray(contextOrArgs);

    for (const signature of this.signatures) {
      const args = positional
        ? matchPositional(signature, contextOrArgs)
        : matchNamed(signature, contextOrArgs);

      if (args) {
        const result = signature.impl(...args);

        return result === undefined ? null : result;
      }
    }

    return invocationFailure(`no matching signature for <${ this.name }> with ${
      positional ? `${ contextOrArgs.length } argument(s)` : `parameters ${ Object.keys(contextOrArgs).join(', ') }`
    }`);
  }
}

function parseSignature({ parameters, impl }) {

  const names = [];

  let required = 0;
  let variadic = false;

  for (const parameter of parameters) {

    if (parameter.startsWith('...')) {
      variadic = true;
      names.push(parameter.slice(3));
      continue;
    }

    if (parameter.endsWith('?')) {
      names.push(parameter.slice(0, -1));
      continue;
    }

    names.push(parameter);
    required = names.length;
  }

  return { names, required, variadic, impl };
}

function matchPositional(signature, args) {

  const { names, required, variadic } = signature;

  if (variadic) {
    const fixed = names.length - 1;

    if (args.length < fixed) {
      return null;
    }

    return [ ...args.slice(0, fixed), args.slice(fixed) ];
  }

  if (args.length < required || args.length > names.length) {
    return null;
  }

  return names.map((_, idx) => idx < args.length ? args[idx] : undefined);
}

function matchNamed(signature, args) {

  const { names, required, variadic } = signature;

  const keys = Object.keys(args);

  if (!keys.every(key => names.includes(key))) {
    return null;
  }

  if (!names.slice(0, required).every(name => keys.includes(name))) {
    return null;
  }

  return names.map((name, idx) => {
    const value = args[name];

    if (variadic && idx === names.length - 1) {
      return value === undefined ? [] : Array.isArray(value) ? value : [ value ];
    }

    return value;
  });
}

/**
 * Define a builtin function.
 *
 * @param {string} name
 * @param {...Signature} signatures
 *
 * @return {BuiltinFunction}
 */
export function builtin(name, ...signatures) {
  return new BuiltinFunction(name, signatures);
}

export { invocationFailure };
