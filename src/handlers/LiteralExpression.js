import { evaluate as feelEvaluate } from 'feelin';

export class LiteralExpression {
  constructor(moddleElement) {
    this.expression = moddleElement.get('text');
  }

  evaluate(context) {
    return feelEvaluate(this.expression, context);
  }
}