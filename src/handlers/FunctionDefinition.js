import {
  DmnFunction,
  invocationFailure
} from '../feel.js';

import { FunctionType, getReturnType } from '../types.js';

import { createOptionalExpression } from './Expression.js';

/**
 * A boxed function definition. Only FEEL functions are supported; Java
 * and PMML functions are external functions, which the engine does not
 * execute.
 */
export class FunctionDefinition {

  /**
   * @param {object} element moddle `dmn:FunctionDefinition`
   * @param {import('../model.js').Model} model
   */
  constructor(element, model) {
    this.id = element.get('id');
    this.model = model;
    this.kind = element.get('kind') || 'FEEL';
    this.typeRef = element.get('typeRef');

    this.parameters = element.get('formalParameter').map(parameter => ({
      id: parameter.get('id'),
      name: parameter.get('name'),
      typeRef: parameter.get('typeRef')
    }));

    this.body = createOptionalExpression(element.get('body'), model);
  }

  evaluate(scope, evaluation) {
    return this.createFunction(scope, evaluation, this);
  }

  /**
   * Create the function, closing over the given scope.
   *
   * @param {object} scope
   * @param {import('../evaluation.js').Evaluation} evaluation
   * @param {object} owner element owning the function (for typing and reporting)
   *
   * @return {import('../feel.js').FeelFunction|null}
   */
  createFunction(scope, evaluation, owner = this) {

    const model = this.model;

    const parameterTypes = this.parameters.map(p => model.resolveType(p.typeRef));

    const returnType = this.getReturnType(owner);

    const type = new FunctionType(parameterTypes, returnType);

    if (this.kind !== 'FEEL') {
      evaluation.error(`unsupported function kind <${ this.kind }>`, owner);

      return null;
    }

    return new DmnFunction((args) => {

      const fnScope = { ...scope };

      for (const [ idx, parameter ] of this.parameters.entries()) {
        const result = model.coerce(args[idx], parameterTypes[idx], null, parameter);

        if (args[idx] !== null && args[idx] !== undefined && result === null) {
          evaluation.error(`invalid argument for parameter <${ parameter.name }> of <${ owner.name || 'function' }>`, owner);

          return invocationFailure(`invalid argument for parameter <${ parameter.name }>`);
        }

        fnScope[parameter.name] = result;
      }

      return evaluation.within(owner, () => {
        const value = this.body.evaluate(fnScope, evaluation);

        return model.coerce(value, returnType, evaluation, owner, 'result');
      });
    }, this.parameters.map(p => p.name), { type });
  }

  /**
   * The return type is given by the function definition's typeRef or,
   * for BKMs, by the BKM's variable type.
   */
  getReturnType(owner) {

    const model = this.model;

    if (this.typeRef) {
      return getReturnType(model.resolveType(this.typeRef));
    }

    if (owner !== this && owner.typeRef) {
      return getReturnType(model.resolveType(owner.typeRef));
    }

    return model.resolveType(null);
  }
}
