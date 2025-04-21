export class InputData {
  constructor(moddleElement) {
    this.name = moddleElement.get('name');
    this.type = moddleElement.get('typeRef');
  }

  /**
   * Get the name of the input data.
   *
   * @returns {string} The name of the input data.
   */
  getName() {
    return this.name;
  }
}
