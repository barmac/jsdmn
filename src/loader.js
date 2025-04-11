import { parseDmn } from './parser.js';

export class Loader {
  constructor() {
    this.decisions = [];
  }

  async load(dmn) {
    const parseResult = await parseDmn(dmn);

    this.decisions.push(...parseResult.decisions);
  }

  getDecision(name) {
    return this.decisions.find(d => d.getName() === name);
  }
}
