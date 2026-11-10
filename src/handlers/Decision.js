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
   * @param {import('../evaluation.js').Evaluation} evaluation
   */
  evaluate(evaluation) {
    return evaluation.resolve(this, () => {

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
