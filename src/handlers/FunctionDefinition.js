import { Expression } from './Expression.js';

export class FunctionDefinition {
  constructor(moddleElement) {
    this.parameters = moddleElement.get('formalParameter');

    const body = moddleElement.get('body');
    this.body = new Expression(body);
  }

  evaluate(context) {
    const body = this.body;
    const parameters = this.parameters;

    return function(...args) {
      const ctx = createContext(context, parameters, args);
      return body.evaluate(ctx);
    };
  }
}

function createContext(context, parameters, args) {
  const ctx = { ...context };

  parameters.forEach((param, index) => {
    ctx[param.get('name')] = args[index];
  });

  return ctx;
}
