import { LiteralExpression } from './LiteralExpression.js';

export function Expression(moddleElement) {
  switch (moddleElement.$type) {
  case 'dmn:LiteralExpression':
    return new LiteralExpression(moddleElement);
  default:
    throw new Error(`Unsupported type <${this.moddleElement.$type}>`);
  }
}
