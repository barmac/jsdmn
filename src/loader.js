import { parseDmn } from './parser.js';

export class Loader {
  constructor() {
    this.decisions = [];
  }

  async load(dmn) {
    const parsed = await parseDmn(dmn);

    const drgElements = parsed.rootElement.get('drgElement');

    const decisions = drgElements.filter(e => e.$instanceOf('dmn:Decision'));

    this.decisions.push(...decisions);
  }

  getDecision(name) {
    return this.decisions.find(d => d.get('name') === name);
  }
}
