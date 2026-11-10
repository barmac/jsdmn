import { FeelRange, evaluateExpression } from '../feel.js';
import { createOptionalExpression } from './Expression.js';

/**
 * Base for the boxed iterators (`for`, `every`, `some`).
 */
class Iterator {

  /**
   * @param {object} element moddle `dmn:Iterator`
   * @param {import('../model.js').Model} model
   */
  constructor(element, model) {
    this.id = element.get('id');
    this.variable = element.get('iteratorVariable');
    this.in = createOptionalExpression(element.get('in'), model);
  }

  /**
   * @return {any[]|null}
   */
  getItems(scope, evaluation) {

    const value = this.in.evaluate(scope, evaluation);

    if (value === null) {
      return null;
    }

    if (Array.isArray(value)) {
      return value;
    }

    if (value instanceof FeelRange) {
      return evaluateExpression('for i in range return i', { range: value }, evaluation, this);
    }

    return [ value ];
  }

  createItemScope(scope, item) {
    return {
      ...scope,
      [this.variable]: item
    };
  }
}

/**
 * A boxed `for` expression.
 */
export class For extends Iterator {

  constructor(element, model) {
    super(element, model);

    this.return = createOptionalExpression(element.get('return'), model);
  }

  evaluate(scope, evaluation) {

    const items = this.getItems(scope, evaluation);

    if (items === null) {
      return null;
    }

    const partial = [];

    for (const item of items) {
      partial.push(this.return.evaluate({
        ...this.createItemScope(scope, item),
        partial: [ ...partial ]
      }, evaluation));
    }

    return partial;
  }
}

class Quantified extends Iterator {

  constructor(element, model) {
    super(element, model);

    this.satisfies = createOptionalExpression(element.get('satisfies'), model);
  }

  /**
   * @return {Array<boolean|null>|null}
   */
  evaluateConditions(scope, evaluation) {

    const items = this.getItems(scope, evaluation);

    if (items === null) {
      return null;
    }

    const results = [];

    for (const item of items) {
      const result = this.satisfies.evaluate(this.createItemScope(scope, item), evaluation);

      if (typeof result !== 'boolean' && result !== null) {
        evaluation.error('<satisfies> did not evaluate to a boolean', this);

        return null;
      }

      results.push(result);
    }

    return results;
  }
}

/**
 * A boxed `every` expression (conjunction of all conditions).
 */
export class Every extends Quantified {

  evaluate(scope, evaluation) {

    const results = this.evaluateConditions(scope, evaluation);

    if (results === null) {
      return null;
    }

    if (results.includes(false)) {
      return false;
    }

    return results.includes(null) ? null : true;
  }
}

/**
 * A boxed `some` expression (disjunction of all conditions).
 */
export class Some extends Quantified {

  evaluate(scope, evaluation) {

    const results = this.evaluateConditions(scope, evaluation);

    if (results === null) {
      return null;
    }

    if (results.includes(true)) {
      return true;
    }

    return results.includes(null) ? null : false;
  }
}
