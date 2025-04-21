import dmnModdle from 'dmn-moddle';

import { Decision } from './handlers/Decision.js';
import { LocalResolver } from './resolvers/LocalResolver.js';

/**
 * @typedef {object} ParseResult
 * @property {string} namespace
 * @property {Decision[]} decisions
 */

/**
 *
 * @param {string} dmn
 * @param {import('./resolvers/LocalResolver.js').Resolver} resolver
 * @returns {Promise<ParseResult>}
 */
export async function parseDmn(dmn, resolver = new LocalResolver()) {
  const moddleResult = await dmnModdle().fromXML(dmn, { lax: false });

  const definitions = moddleResult.rootElement;

  const parseResult = {
    namespace: definitions.get('namespace'),
    decisions: []
  };

  for (const element of definitions.get('drgElement')) {
    if (element.$instanceOf('dmn:Decision')) {
      const decision = new Decision(element, resolver);
      parseResult.decisions.push(decision);
    }
  }

  return parseResult;
}
