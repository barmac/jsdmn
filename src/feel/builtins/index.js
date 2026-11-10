import lists from './lists.js';
import numeric from './numeric.js';
import ranges from './ranges.js';
import strings from './strings.js';
import temporal from './temporal.js';

/**
 * Builtins overriding (or added to) the ones provided by feelin.
 */
export const overrides = {
  ...lists,
  ...numeric,
  ...ranges,
  ...strings,
  ...temporal
};
