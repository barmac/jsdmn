import { DmnModdle } from 'dmn-moddle';

import dmnSchema from './moddle/dmn15.js';
import dmndiSchema from './moddle/dmndi15.js';

const DMN15_NAMESPACE = 'https://www.omg.org/spec/DMN/20230324/MODEL/';
const DMNDI15_NAMESPACE = 'https://www.omg.org/spec/DMN/20230324/DMNDI/';

/**
 * Model namespaces of DMN versions handled as DMN 1.5.
 */
const DMN_NAMESPACES = [
  'http://www.omg.org/spec/DMN/20151101/dmn.xsd',
  'http://www.omg.org/spec/DMN/20180521/MODEL/',
  'https://www.omg.org/spec/DMN/20191111/MODEL/',
  'https://www.omg.org/spec/DMN/20211108/MODEL/'
];

const DMNDI_NAMESPACES = [
  'http://www.omg.org/spec/DMN/20180521/DMNDI/',
  'https://www.omg.org/spec/DMN/20191111/DMNDI/',
  'https://www.omg.org/spec/DMN/20211108/DMNDI/'
];

const moddle = DmnModdle({
  dmn: dmnSchema,
  dmndi: dmndiSchema
});

/**
 * @typedef {object} ParseResult
 * @property {object} definitions moddle `dmn:Definitions`
 * @property {object[]} warnings
 */

/**
 * Parse DMN 1.1 - 1.5 XML.
 *
 * @param {string} xml
 *
 * @returns {Promise<ParseResult>}
 */
export async function parseDmn(xml) {

  const { rootElement, warnings } = await moddle.fromXML(normalizeNamespaces(xml), 'dmn:Definitions', { lax: false });

  return {
    definitions: rootElement,
    warnings
  };
}

/**
 * Rewrite the namespaces of older DMN versions to DMN 1.5, which is a
 * superset of them for the purpose of execution.
 *
 * @param {string} xml
 *
 * @return {string}
 */
export function normalizeNamespaces(xml) {

  for (const namespace of DMN_NAMESPACES) {
    xml = xml.replaceAll(namespace, DMN15_NAMESPACE);
  }

  for (const namespace of DMNDI_NAMESPACES) {
    xml = xml.replaceAll(namespace, DMNDI15_NAMESPACE);
  }

  return xml;
}
