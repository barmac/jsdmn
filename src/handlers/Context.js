import { Expression } from './Expression.js';

export class Context {
  constructor(moddleElement) {
    this.contextEntries = moddleElement.get('contextEntry').map(entry => {
      return new ContextEntry(entry);
    });
  }

  evaluate(context) {
    let result = {};
    for (const entry of this.contextEntries) {
      const entryResult = entry.evaluate({
        ...context,
        ...result
      });
      result = { ...result, ...entryResult };
    }

    return result;
  }
}

class ContextEntry {
  constructor(moddleElement) {
    this.name = moddleElement.get('variable').get('name');
    this.expression = new Expression(moddleElement.get('value'));
  }

  evaluate(context) {
    const value = this.expression.evaluate(context);
    return {
      [this.name]: value
    };
  }
}