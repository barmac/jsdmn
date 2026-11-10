import { getFeelType } from '../feel.js';
import { createOptionalExpression } from './Expression.js';

/**
 * A boxed filter: the items of the `in` list for which `match` is true.
 */
export class Filter {

  /**
   * @param {object} element moddle `dmn:Filter`
   * @param {import('../model.js').Model} model
   */
  constructor(element, model) {
    this.id = element.get('id');

    this.in = createOptionalExpression(element.get('in'), model);
    this.match = createOptionalExpression(element.get('match'), model);
  }

  evaluate(scope, evaluation) {

    let list = this.in.evaluate(scope, evaluation);

    if (list === null) {
      return null;
    }

    if (!Array.isArray(list)) {
      list = [ list ];
    }

    const result = [];

    for (const item of list) {
      const itemScope = {
        ...scope,
        item,
        ...(getFeelType(item) === 'context' ? item : {})
      };

      const match = this.match.evaluate(itemScope, evaluation);

      if (typeof match !== 'boolean' && match !== null) {
        evaluation.error('filter <match> did not evaluate to a boolean', this);

        return null;
      }

      if (match === true) {
        result.push(item);
      }
    }

    return result;
  }
}
