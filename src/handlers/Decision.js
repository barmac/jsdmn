import { Expression } from './Expression.js';

export class Decision {
  constructor(moddleElement) {
    this.name = moddleElement.get('name');
    this.logic = new Expression(moddleElement.get('decisionLogic'));
  }

  evaluate(context) {
    return this.logic.evaluate(context);
  }

  /**
   * Get the name of the decision.
   *
   * @returns {string} The name of the decision.
   */
  getName() {
    return this.name;
  }
}