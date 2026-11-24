/**
 * A DMN `<inputData>`.
 */
export class InputData {

  /**
   * @param {object} element moddle `dmn:InputData`
   * @param {import('../model.js').Model} model
   */
  constructor(element, model) {
    this.id = element.get('id');
    this.name = element.get('name');
    this.model = model;
    this.typeRef = element.get('variable')?.get('typeRef');
  }

  /**
   * Get the name of the input data.
   *
   * @returns {string} The name of the input data.
   */
  getName() {
    return this.name;
  }

  get type() {
    return this.model.resolveType(this.typeRef);
  }

  /**
   * Resolve the (type checked) value of this input.
   *
   * @param {import('../evaluation.js').Evaluation} evaluation
   */
  evaluate(evaluation) {
    return evaluation.resolve(this, () => {
      const value = evaluation.getInput(this);

      return this.model.coerce(value, this.type, evaluation, this, 'input');
    });
  }
}
