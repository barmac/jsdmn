import { FunctionDefinition } from './FunctionDefinition.js';

export class BusinessKnowledgeModel {
  constructor(moddleElement) {
    this.name = moddleElement.get('name');
    this.type = moddleElement.get('typeRef');
    this.logic = new FunctionDefinition(moddleElement.get('encapsulatedLogic'));
  }

  /**
   * Get the name of the element.
   *
   * @returns {string} The name of the input data.
   */
  getName() {
    return this.name;
  }

  evaluate(context) {
    return this.logic.evaluate(context);
  }
}
