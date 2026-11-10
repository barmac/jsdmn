import { invokeFunction } from '../feel.js';
import { createOptionalExpression } from './Expression.js';

/**
 * A boxed invocation, binding parameters by name.
 */
export class Invocation {

  /**
   * @param {object} element moddle `dmn:Invocation`
   * @param {import('../model.js').Model} model
   */
  constructor(element, model) {
    this.id = element.get('id');

    this.calledFunction = createOptionalExpression(element.get('calledFunction'), model);

    this.bindings = element.get('binding').map(binding => ({
      name: binding.get('parameter')?.get('name'),
      expression: createOptionalExpression(binding.get('bindingFormula'), model)
    }));
  }

  evaluate(scope, evaluation) {

    const fn = this.calledFunction.evaluate(scope, evaluation);

    const args = {};

    for (const binding of this.bindings) {
      args[binding.name] = binding.expression.evaluate(scope, evaluation);
    }

    return invokeFunction(fn, args, evaluation, this);
  }
}
