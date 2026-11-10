import { Evaluation } from './evaluation.js';
import { invokeFunction } from './feel.js';
import { Loader } from './loader.js';

/**
 * @typedef {object} EvaluateOptions
 * @property {string} [namespace] namespace (or name) of the model to look up the element in
 */

/**
 * @typedef {object} EvaluationResult
 * @property {any} value
 * @property {import('./evaluation.js').Message[]} messages
 */

/**
 * Evaluates decisions, decision services and business knowledge models of
 * loaded DMN models.
 *
 * Inputs are given by input data name. Inputs of imported models may be
 * nested under the respective import name, or be given in qualified form
 * as `{namespace}name`.
 */
export class Engine {

  constructor(loader = new Loader()) {
    this.loader = loader;
  }

  /**
   * Load a DMN model. Models importing each other may be loaded in
   * any order.
   *
   * @param {string} dmn XML
   *
   * @return {Promise<import('./model.js').Model>}
   */
  load(dmn) {
    return this.loader.load(dmn);
  }

  /**
   * Evaluate a decision and return its value.
   *
   * @param {string} name
   * @param {object} [inputs]
   * @param {EvaluateOptions} [options]
   *
   * @return {Promise<any>}
   */
  async evaluateDecision(name, inputs, options) {
    const { value } = await this.evaluate('decision', name, inputs, options);

    return value;
  }

  /**
   * Evaluate a decision service and return its value: the value of its
   * output decision or, if it has many, a context of their values.
   *
   * @param {string} name
   * @param {object} [inputs] values for the service's input data and input decisions
   * @param {EvaluateOptions} [options]
   *
   * @return {Promise<any>}
   */
  async evaluateDecisionService(name, inputs, options) {
    const { value } = await this.evaluate('decisionService', name, inputs, options);

    return value;
  }

  /**
   * Invoke a business knowledge model.
   *
   * @param {string} name
   * @param {object|any[]} [args] named or positional arguments
   * @param {EvaluateOptions} [options]
   *
   * @return {Promise<any>}
   */
  async evaluateBusinessKnowledgeModel(name, args, options) {
    const { value } = await this.evaluate('businessKnowledgeModel', name, args, options);

    return value;
  }

  /**
   * Evaluate an element, returning its value along with the
   * messages reported during evaluation.
   *
   * @param {'decision'|'decisionService'|'businessKnowledgeModel'} type
   * @param {string} name
   * @param {object|any[]} [inputs]
   * @param {EvaluateOptions} [options]
   *
   * @return {Promise<EvaluationResult>}
   */
  async evaluate(type, name, inputs = {}, options = {}) {
    return this.evaluateSync(type, name, inputs, options);
  }

  /**
   * Synchronous variant of {@link Engine#evaluate}.
   */
  evaluateSync(type, name, inputs = {}, options = {}) {

    const { namespace } = options;

    switch (type) {

    case 'decision': {
      const decision = this.loader.getDecision(name, namespace);

      if (!decision) {
        throw new Error(`Decision <${ name }> not found`);
      }

      const evaluation = this.createEvaluation(decision.model, inputs);

      const value = decision.evaluate(evaluation);

      return { value, messages: evaluation.messages };
    }

    case 'decisionService': {
      const decisionService = this.loader.getDecisionService(name, namespace);

      if (!decisionService) {
        throw new Error(`Decision service <${ name }> not found`);
      }

      const evaluation = this.createEvaluation(decisionService.model, inputs);

      const values = new Map();

      for (const parameter of decisionService.parameters) {
        values.set(parameter, evaluation.getInput(parameter) ?? null);
      }

      const value = decisionService.invoke(values, evaluation);

      return { value, messages: evaluation.messages };
    }

    case 'businessKnowledgeModel': {
      const bkm = this.loader.getBusinessKnowledgeModel(name, namespace);

      if (!bkm) {
        throw new Error(`Business knowledge model <${ name }> not found`);
      }

      const evaluation = this.createEvaluation(bkm.model, {});

      const value = invokeFunction(bkm.createFunction(evaluation), inputs || [], evaluation, bkm);

      return { value, messages: evaluation.messages };
    }

    default:
      throw new Error(`unknown element type <${ type }>`);
    }
  }

  /**
   * @param {import('./model.js').Model} model
   * @param {object} inputs
   */
  createEvaluation(model, inputs) {
    return new Evaluation({
      getInput: (element) => lookupInput(model, element, inputs),
      variables: getFreeVariables(model, inputs)
    });
  }
}

/**
 * Inputs that do not correspond to any modeled input data element are
 * exposed to decisions as free variables. This keeps DRG scoping intact for
 * modeled inputs while supporting decisions that refer to plain variables.
 */
function getFreeVariables(model, inputs) {

  if (!inputs || typeof inputs !== 'object' || Array.isArray(inputs)) {
    return {};
  }

  const importNames = new Set(model.imports.map(i => i.name));

  return Object.fromEntries(Object.entries(inputs).filter(([ key ]) => {
    return !key.startsWith('{') && !importNames.has(key) && !model.getInputData(key);
  }));
}

/**
 * Look up the value provided for an input data (or decision service input)
 * element, from the perspective of the given model.
 */
function lookupInput(model, element, inputs) {

  if (!inputs || typeof inputs !== 'object') {
    return undefined;
  }

  const qualifiedName = `{${ element.model.namespace }}${ element.name }`;

  if (has(inputs, qualifiedName)) {
    return inputs[qualifiedName];
  }

  if (element.model === model) {
    return has(inputs, element.name) ? inputs[element.name] : undefined;
  }

  for (const importName of model.getImportNames(element.model)) {

    if (!importName) {
      if (has(inputs, element.name)) {
        return inputs[element.name];
      }

      continue;
    }

    const imported = inputs[importName];

    if (imported && typeof imported === 'object' && has(imported, element.name)) {
      return imported[element.name];
    }
  }

  return undefined;
}

function has(obj, key) {
  return Object.prototype.hasOwnProperty.call(obj, key);
}
