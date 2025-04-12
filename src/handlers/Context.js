import { Expression } from './Expression.js';

export class Context {
  constructor(moddleElement) {
    const entries = moddleElement.get('contextEntry');
    this.contextEntries = entries.map(entry => {
      return new ContextEntry(entry);
    });
  }

  evaluate(context) {
    let result = {};
    for (const entry of this.contextEntries) {
      const entryValue = entry.evaluate({
        ...context,
        ...result
      });

      const entryName = entry.getName();

      if (!entryName) {
        return entryValue;
      }

      result = {
        ...result,
        [entryName]: entryValue
      };
    }

    return result;
  }
}

class ContextEntry {
  constructor(moddleElement) {
    this.name = moddleElement.get('variable')?.get('name');
    this.expression = new Expression(moddleElement.get('value'));
  }

  evaluate(context) {
    return this.expression.evaluate(context);
  }

  getName() {
    return this.name;
  }
}