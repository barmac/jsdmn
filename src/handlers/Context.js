import { createOptionalExpression } from './Expression.js';

/**
 * A boxed context. Entries are evaluated in order and may refer to the
 * entries preceding them; an entry without a variable is the result.
 */
export class Context {

  /**
   * @param {object} element moddle `dmn:Context`
   * @param {import('../model.js').Model} model
   */
  constructor(element, model) {
    this.id = element.get('id');
    this.model = model;

    this.entries = element.get('contextEntry').map(entry => {
      const variable = entry.get('variable');

      return {
        id: entry.get('id'),
        name: variable?.get('name'),
        typeRef: variable?.get('typeRef'),
        expression: createOptionalExpression(entry.get('value'), model)
      };
    });
  }

  evaluate(scope, evaluation) {

    // functions defined in entries close over this (mutable) scope,
    // enabling (mutual) recursion
    const entryScope = { ...scope };

    const result = {};

    for (const entry of this.entries) {
      const value = entry.expression.evaluate(entryScope, evaluation);

      if (!entry.name) {
        return value;
      }

      const coerced = this.model.coerce(value, this.model.resolveType(entry.typeRef), evaluation, entry, 'context entry');

      entryScope[entry.name] = coerced;
      result[entry.name] = coerced;
    }

    return result;
  }
}
