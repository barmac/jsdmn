import { Context } from './Context.js';
import { FunctionDefinition } from './FunctionDefinition.js';
import { LiteralExpression } from './LiteralExpression.js';

export function Expression(moddleElement) {
  switch (moddleElement.$type) {
  case 'dmn:LiteralExpression':
    return new LiteralExpression(moddleElement);
  case 'dmn:Context':
    return new Context(moddleElement);
  case 'dmn:FunctionDefinition':
    return new FunctionDefinition(moddleElement);
  default:
    throw new Error(`Unsupported type <${moddleElement.$type}>`);
  }
}
