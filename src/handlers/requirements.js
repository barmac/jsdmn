/**
 * Resolve requirement references of a DRG element lazily: referenced
 * elements may live in imported models which are loaded later.
 */
export class Requirements {

  /**
   * @param {import('../model.js').Model} model
   * @param {object[]} informationRequirements moddle `dmn:InformationRequirement`s
   * @param {object[]} knowledgeRequirements moddle `dmn:KnowledgeRequirement`s
   */
  constructor(model, informationRequirements = [], knowledgeRequirements = []) {
    this.model = model;

    this.informationHrefs = informationRequirements.map(requirement => {
      return (requirement.get('requiredDecision') || requirement.get('requiredInput'))?.get('href');
    }).filter(Boolean);

    this.knowledgeHrefs = knowledgeRequirements.map(requirement => {
      return requirement.get('requiredKnowledge')?.get('href');
    }).filter(Boolean);

    this._information = null;
    this._knowledge = null;
  }

  /**
   * Required decisions and input data.
   */
  get information() {

    if (!this._information) {
      this._information = this.resolve(this.informationHrefs);
    }

    return this._information;
  }

  /**
   * Required business knowledge models and decision services.
   */
  get knowledge() {

    if (!this._knowledge) {
      this._knowledge = this.resolve(this.knowledgeHrefs);
    }

    return this._knowledge;
  }

  resolve(hrefs) {
    return hrefs.map(href => {
      const element = this.model.resolveReference(href);

      if (!element) {
        throw new Error(`unresolved requirement <${ href }> in ${ this.model }`);
      }

      return element;
    });
  }

  /**
   * Populate the scope with the values of all required elements.
   *
   * @param {object} scope
   * @param {import('../evaluation.js').Evaluation} evaluation
   */
  populate(scope, evaluation) {

    for (const element of this.information) {
      this.model.assign(scope, element, element.evaluate(evaluation));
    }

    for (const element of this.knowledge) {
      this.model.assign(scope, element, element.createFunction(evaluation));
    }

    return scope;
  }
}
