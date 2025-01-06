import { evaluate as feelEvaluate } from 'feelin';

export function evaluate(element, context) {
  switch (element.$type) {
  case 'dmn:Decision': return evaluateDecision(element, context);
  case 'dmn:LiteralExpression': return evaluateLiteralExpression(element, context);
  default: throw new Error(`Cannot evaluate ${element.$type}`);
  }
}

function evaluateDecision(decision, context) {
  const logic = getDecisionLogic(decision);

  return evaluate(logic, context);
}

function evaluateLiteralExpression(expression, context) {
  const text = expression.get('text');

  return evaluateFeelExpression(text, context);
}

function evaluateFeelExpression(expression, context) {
  return feelEvaluate(expression, context);
}

// helper

function getDecisionLogic(decision) {
  const logic = decision.get('decisionLogic');

  return logic;
}