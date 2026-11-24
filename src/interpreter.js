import { Engine } from './engine.js';

/**
 * Interpret a decision from a given DMN with given context.
 *
 * @param {string} dmn
 * @param {string} decisionName
 * @param {object} context
 *
 * @return {Promise<any>}
 */
export async function interpret(dmn, decisionName, context) {
  const engine = new Engine();

  await engine.load(dmn);

  return engine.evaluateDecision(decisionName, context);
}
