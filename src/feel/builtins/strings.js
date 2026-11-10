/**
 * String builtins based on regular expressions (DMN 1.5, 10.3.4.3).
 */

import { builtin, invocationFailure } from './function.js';

import { createRegex, createReplacer } from './regex.js';

const isString = (value) => typeof value === 'string';

const matches = builtin('matches', {
  parameters: [ 'input', 'pattern', 'flags?' ],
  impl(input, pattern, flags = '') {

    if (!isString(input) || !isString(pattern) || (flags !== null && !isString(flags))) {
      return invocationFailure('matches(input, pattern, flags) expects strings');
    }

    try {
      return createRegex(pattern, flags || '').test(input);
    } catch (error) {
      return invocationFailure(error.message);
    }
  }
});

const replace = builtin('replace', {
  parameters: [ 'input', 'pattern', 'replacement', 'flags?' ],
  impl(input, pattern, replacement, flags = '') {

    if (!isString(input) || !isString(pattern) || !isString(replacement) || (flags !== null && !isString(flags))) {
      return invocationFailure('replace(input, pattern, replacement, flags) expects strings');
    }

    try {
      const regex = createRegex(pattern, flags || '', 'g');

      if (regex.test('')) {
        return invocationFailure(`pattern <${ pattern }> matches the empty string`);
      }

      regex.lastIndex = 0;

      return input.replace(regex, createReplacer(replacement));
    } catch (error) {
      return invocationFailure(error.message);
    }
  }
});

export default {
  'matches': matches,
  'replace': replace
};
