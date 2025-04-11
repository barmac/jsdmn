import { parseDmn } from './parser.js';

/**
 * Interpret a decision from a given DMN with given context.
 *
 * @param {string} dmn
 * @param {string} decisionName
 * @param {object} context
 */
export async function interpret(dmn, decisionName, context) {
  const parseResult = await parseDmn(dmn);
  const decision = parseResult.decisions.find(d => d.getName() === decisionName);

  return decision.evaluate(context);
}
