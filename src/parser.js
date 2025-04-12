import dmnModdle from 'dmn-moddle';

import { Decision } from './handlers/Decision.js';

/**
 * @typedef {object} ParseResult
 * @property {string} namespace
 * @property {Decision[]} decisions
 */

/**
 *
 * @param {string} dmn
 * @returns {Promise<ParseResult>}
 */
export async function parseDmn(dmn) {
  const moddleResult = await dmnModdle().fromXML(dmn, { lax: false });

  const definitions = moddleResult.rootElement;

  const parseResult = {
    namespace: definitions.get('namespace'),
    decisions: []
  };

  for (const element of definitions.get('drgElement')) {
    if (element.$instanceOf('dmn:Decision')) {
      const decision = new Decision(element);
      parseResult.decisions.push(decision);
    }
  }

  return parseResult;
}
