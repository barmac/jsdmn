import { createExpression } from './Expression.js';

/**
 * A boxed list.
 */
export class List {

  /**
   * @param {object} element moddle `dmn:List`
   * @param {import('../model.js').Model} model
   */
  constructor(element, model) {
    this.id = element.get('id');
    this.elements = element.get('elements').map(e => createExpression(e, model));
  }

  evaluate(scope, evaluation) {
    return this.elements.map(element => element.evaluate(scope, evaluation));
  }
}
