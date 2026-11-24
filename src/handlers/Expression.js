import { Context } from './Context.js';
import { DecisionTable } from './DecisionTable.js';
import { FunctionDefinition } from './FunctionDefinition.js';
import { Invocation } from './Invocation.js';
import { List } from './List.js';
import { LiteralExpression } from './LiteralExpression.js';
import { Relation } from './Relation.js';
import { Conditional } from './Conditional.js';
import { Filter } from './Filter.js';
import { For, Every, Some } from './Iterator.js';

/**
 * @typedef {object} ExpressionHandler
 * @property {(scope: object, evaluation: import('../evaluation.js').Evaluation) => any} evaluate
 */

/**
 * Resolved lazily, as expression handlers and this module
 * depend on each other.
 */
function getHandler(type) {
  return {
    'dmn:LiteralExpression': LiteralExpression,
    'dmn:Context': Context,
    'dmn:FunctionDefinition': FunctionDefinition,
    'dmn:DecisionTable': DecisionTable,
    'dmn:Invocation': Invocation,
    'dmn:List': List,
    'dmn:Relation': Relation,
    'dmn:Conditional': Conditional,
    'dmn:Filter': Filter,
    'dmn:For': For,
    'dmn:Every': Every,
    'dmn:Some': Some
  }[type];
}

/**
 * Create the handler for a boxed expression.
 *
 * @param {object} element moddle `dmn:Expression`
 * @param {import('../model.js').Model} model
 *
 * @return {ExpressionHandler}
 */
export function createExpression(element, model) {

  const Handler = getHandler(element.$type);

  if (!Handler) {
    return new UnsupportedExpression(element);
  }

  const handler = new Handler(element, model);

  const typeRef = element.get('typeRef');

  // the type of a function definition denotes its result type
  if (typeRef && model && !(handler instanceof FunctionDefinition)) {
    return new TypedExpression(handler, typeRef, model);
  }

  return handler;
}

/**
 * Coerces the result of an expression to the expression's type.
 */
class TypedExpression {

  constructor(expression, typeRef, model) {
    this.expression = expression;
    this.id = expression.id;
    this.typeRef = typeRef;
    this.model = model;
  }

  evaluate(scope, evaluation) {
    const value = this.expression.evaluate(scope, evaluation);

    return this.model.coerce(value, this.model.resolveType(this.typeRef), evaluation, this, 'expression result');
  }
}

/**
 * Create the handler for an optional (child) expression; a missing
 * expression evaluates to `null`.
 *
 * @param {object} [element] moddle `dmn:Expression` or `dmn:ChildExpression`
 * @param {import('../model.js').Model} model
 *
 * @return {ExpressionHandler}
 */
export function createOptionalExpression(element, model) {

  if (element && element.$instanceOf('dmn:ChildExpression')) {
    element = element.get('expression');
  }

  return element ? createExpression(element, model) : NULL_EXPRESSION;
}

/**
 * Backwards compatible alias.
 */
export { createExpression as Expression };

const NULL_EXPRESSION = {
  evaluate() {
    return null;
  }
};

class UnsupportedExpression {

  constructor(element) {
    this.id = element.get('id');
    this.type = element.$type;
  }

  evaluate(scope, evaluation) {
    evaluation.error(`unsupported expression type <${ this.type }>`, this);

    return null;
  }
}
