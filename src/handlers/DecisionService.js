import {
  DmnFunction,
  invocationFailure
} from '../feel.js';

import {
  ContextType,
  FunctionType,
  coerce,
  getReturnType,
  unwrap
} from '../types.js';

/**
 * A DMN `<decisionService>`; it evaluates to a FEEL function whose
 * parameters are the service's input data followed by its input decisions.
 */
export class DecisionService {

  /**
   * @param {object} element moddle `dmn:DecisionService`
   * @param {import('../model.js').Model} model
   */
  constructor(element, model) {
    this.id = element.get('id');
    this.name = element.get('name');
    this.model = model;
    this.typeRef = element.get('variable')?.get('typeRef');

    const hrefs = (name) => element.get(name).map(ref => ref.get('href'));

    this.hrefs = {
      outputDecisions: hrefs('outputDecision'),
      encapsulatedDecisions: hrefs('encapsulatedDecision'),
      inputDecisions: hrefs('inputDecision'),
      inputData: hrefs('inputData')
    };

    this._resolved = null;
    this._functions = new WeakMap();
  }

  /**
   * Get the name of the element.
   *
   * @returns {string} The name of the decision service.
   */
  getName() {
    return this.name;
  }

  get type() {
    return this.model.resolveType(this.typeRef);
  }

  get resolved() {

    if (!this._resolved) {
      const resolve = (hrefs) => hrefs.map(href => {
        const element = this.model.resolveReference(href);

        if (!element) {
          throw new Error(`unresolved reference <${ href }> in decision service <${ this.name }>`);
        }

        return element;
      });

      this._resolved = {
        outputDecisions: resolve(this.hrefs.outputDecisions),
        encapsulatedDecisions: resolve(this.hrefs.encapsulatedDecisions),
        inputDecisions: resolve(this.hrefs.inputDecisions),
        inputData: resolve(this.hrefs.inputData)
      };
    }

    return this._resolved;
  }

  /**
   * The formal parameters: input data followed by input decisions.
   */
  get parameters() {
    const { inputData, inputDecisions } = this.resolved;

    return [ ...inputData, ...inputDecisions ];
  }

  get outputDecisions() {
    return this.resolved.outputDecisions;
  }

  /**
   * Invoke the decision service with the given parameter values.
   *
   * @param {Map<object, any>} values parameter values by input element
   * @param {import('../evaluation.js').Evaluation} parentEvaluation
   * @param {object} [options]
   * @param {boolean} [options.strict=false] fail on non-conforming parameters
   *        rather than coercing them to `null`
   *
   * @return {any}
   */
  invoke(values, parentEvaluation, options = {}) {

    const { inputDecisions, outputDecisions } = this.resolved;

    if (options.strict) {
      for (const parameter of this.parameters) {
        const { error } = coerce(values.get(parameter), parameter.type);

        if (error) {
          parentEvaluation.error(`invalid parameter <${ parameter.name }> for decision service <${ this.name }>: ${ error }`, this);

          return invocationFailure(`invalid parameter <${ parameter.name }>`);
        }
      }
    }

    const evaluation = parentEvaluation.fork(inputData => values.get(inputData));

    for (const decision of inputDecisions) {
      evaluation.provide(
        decision,
        this.model.coerce(values.get(decision) ?? null, decision.type, evaluation, decision, 'input')
      );
    }

    const results = outputDecisions.map(decision => decision.evaluate(evaluation));

    const value = outputDecisions.length === 1
      ? results[0]
      : Object.fromEntries(outputDecisions.map((decision, idx) => [ decision.name, results[idx] ]));

    return this.model.coerce(value, getReturnType(this.type), parentEvaluation, this, 'result');
  }

  /**
   * Create the FEEL function this decision service represents.
   *
   * @param {import('../evaluation.js').Evaluation} evaluation
   *
   * @return {import('../feel.js').FeelFunction}
   */
  createFunction(evaluation) {

    let fn = this._functions.get(evaluation);

    if (fn) {
      return fn;
    }

    const parameters = this.parameters;

    const declaredType = unwrap(this.type);

    fn = new DmnFunction((args) => {
      const values = new Map(parameters.map((parameter, idx) => [ parameter, args[idx] ]));

      return this.invoke(values, evaluation, { strict: true });
    }, parameters.map(p => p.name), {
      type: declaredType instanceof FunctionType && declaredType.parameterTypes ? declaredType : new FunctionType(
        parameters.map(p => p.type),
        this.outputDecisions.length === 1
          ? this.outputDecisions[0].type
          : new ContextType(this.outputDecisions.map(d => [ d.name, d.type ]))
      )
    });

    this._functions.set(evaluation, fn);

    return fn;
  }

  evaluate(evaluation) {
    return this.createFunction(evaluation);
  }
}
