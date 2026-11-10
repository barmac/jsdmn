import { evaluateExpression } from '../feel.js';

/**
 * Whether the given expression language denotes FEEL, e.g. `FEEL`,
 * `http://www.omg.org/spec/FEEL/20140401` or `https://www.omg.org/spec/DMN/20230324/FEEL/`.
 */
function isFeel(expressionLanguage) {
  return !expressionLanguage || /(^|\/)feel(\/[\d/]*)?$/i.test(expressionLanguage.trim().replace(/\/$/, ''));
}

/**
 * A FEEL literal expression.
 */
export class LiteralExpression {

  /**
   * @param {object} element moddle `dmn:LiteralExpression`
   * @param {import('../model.js').Model} model
   */
  constructor(element, model) {
    this.id = element.get('id');
    this.text = element.get('text') || '';
    this.model = model;
    this.expressionLanguage = element.get('expressionLanguage') || model?.definitions.get('expressionLanguage');
  }

  evaluate(scope, evaluation) {

    if (!isFeel(this.expressionLanguage)) {
      evaluation?.error(`unsupported expression language <${ this.expressionLanguage }>`, this);

      return null;
    }

    return evaluateExpression(this.text, scope, evaluation, this, this.model);
  }
}
