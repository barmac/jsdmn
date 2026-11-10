import { createOptionalExpression } from './Expression.js';

/**
 * A boxed conditional (`if` / `then` / `else`).
 */
export class Conditional {

  /**
   * @param {object} element moddle `dmn:Conditional`
   * @param {import('../model.js').Model} model
   */
  constructor(element, model) {
    this.id = element.get('id');

    this.condition = createOptionalExpression(element.get('if'), model);
    this.then = createOptionalExpression(element.get('then'), model);
    this.else = createOptionalExpression(element.get('else'), model);
  }

  evaluate(scope, evaluation) {

    const condition = this.condition.evaluate(scope, evaluation);

    if (condition === true) {
      return this.then.evaluate(scope, evaluation);
    }

    if (condition !== false && condition !== null) {
      evaluation.error('conditional <if> did not evaluate to a boolean', this);

      return null;
    }

    return this.else.evaluate(scope, evaluation);
  }
}
