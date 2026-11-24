import { createExpression } from './Expression.js';
import { Requirements } from './requirements.js';

/**
 * A DMN `<decision>`.
 */
export class Decision {

  /**
   * @param {object} element moddle `dmn:Decision`
   * @param {import('../model.js').Model} model
   */
  constructor(element, model) {
    this.id = element.get('id');
    this.name = element.get('name');
    this.model = model;
    this.typeRef = element.get('variable')?.get('typeRef');

    const decisionLogic = element.get('decisionLogic');

    this.logic = decisionLogic ? createExpression(decisionLogic, model) : null;

    this.requirements = new Requirements(
      model,
      element.get('informationRequirement'),
      element.get('knowledgeRequirement')
    );
  }

  /**
   * Get the name of the decision.
   *
   * @returns {string} The name of the decision.
   */
  getName() {
    return this.name;
  }

  get type() {
    return this.model.resolveType(this.typeRef);
  }

  /**
   * Evaluate the decision (at most once per evaluation).
   *
   * A value provided for a decision that is evaluated as a requirement of
   * another element is used instead of evaluating the decision.
   *
   * @param {import('../evaluation.js').Evaluation} evaluation
   * @param {object} [options]
   * @param {boolean} [options.required] whether the decision is evaluated as a requirement
   */
  evaluate(evaluation, { required = false } = {}) {
    return evaluation.resolve(this, () => {

      const provided = required ? evaluation.getInput(this) : undefined;

      if (provided !== undefined) {
        return this.model.coerce(provided, this.type, evaluation, this, 'input');
      }

      const scope = this.requirements.populate({
        ...evaluation.variables,
        ...this.model.createScope()
      }, evaluation);

      if (!this.logic) {
        evaluation.error(`decision <${ this.name }> has no decision logic`, this);

        return null;
      }

      const value = this.logic.evaluate(scope, evaluation);

      return this.model.coerce(value, this.type, evaluation, this, 'result');
    });
  }
}
