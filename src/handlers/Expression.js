import { Context } from './Context.js';
import { LiteralExpression } from './LiteralExpression.js';

export function Expression(moddleElement) {
  switch (moddleElement.$type) {
  case 'dmn:LiteralExpression':
    return new LiteralExpression(moddleElement);
  case 'dmn:Context':
    return new Context(moddleElement);
  default:
    throw new Error(`Unsupported type <${moddleElement.$type}>`);
  }
}
