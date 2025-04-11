import { Loader } from './loader.js';

export class Engine {
  constructor(loader = new Loader()) {
    this.loader = loader;
  }

  async evaluateDecision(name, context) {
    const decision = this.loader.getDecision(name);

    if (!decision) {
      throw new Error(`Decision <${name}> not found`);
    }

    return decision.evaluate(context);
  }

  load(dmn) {
    return this.loader.load(dmn);
  }
}