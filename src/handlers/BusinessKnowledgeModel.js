import { DmnFunction } from '../feel.js';
import { FunctionDefinition } from './FunctionDefinition.js';
import { Requirements } from './requirements.js';

/**
 * A DMN `<businessKnowledgeModel>`; it evaluates to a FEEL function.
 */
export class BusinessKnowledgeModel {

  /**
   * @param {object} element moddle `dmn:BusinessKnowledgeModel`
   * @param {import('../model.js').Model} model
   */
  constructor(element, model) {
    this.id = element.get('id');
    this.name = element.get('name');
    this.model = model;
    this.typeRef = element.get('variable')?.get('typeRef');

    const encapsulatedLogic = element.get('encapsulatedLogic');

    this.logic = encapsulatedLogic ? new FunctionDefinition(encapsulatedLogic, model) : null;

    this.requirements = new Requirements(model, [], element.get('knowledgeRequirement'));

    this._functions = new WeakMap();
  }

  /**
   * Get the name of the element.
   *
   * @returns {string} The name of the business knowledge model.
   */
  getName() {
    return this.name;
  }

  get type() {
    return this.model.resolveType(this.typeRef);
  }

  /**
   * Create the FEEL function this BKM represents.
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

    // the function closes over this scope; it is populated
    // after creating the function to support (mutual) recursion
    const scope = this.model.createScope();

    if (this.logic) {
      fn = this.logic.createFunction(scope, evaluation, this);
    } else {
      fn = new DmnFunction(() => {
        evaluation.error(`business knowledge model <${ this.name }> has no logic`, this);

        return null;
      }, []);
    }

    this._functions.set(evaluation, fn);

    this.model.assign(scope, this, fn);

    this.requirements.populate(scope, evaluation);

    return fn;
  }

  /**
   * Evaluate the BKM itself, i.e. return its function.
   */
  evaluate(evaluation) {
    return this.createFunction(evaluation);
  }
}
