import { DmnModdle } from 'dmn-moddle';

import dmn14Schema from './moddle/dmn14.js';
import dmn15Schema from './moddle/dmn15.js';
import dmndi15Schema from './moddle/dmndi15.js';

/**
 * Supported DMN versions, each parsed (strictly) with its own schema.
 */
export const DMN_VERSIONS = {
  '1.3': {
    namespace: 'https://www.omg.org/spec/DMN/20191111/MODEL/',
    dmndi: 'https://www.omg.org/spec/DMN/20191111/DMNDI/'
  },
  '1.4': {
    namespace: 'https://www.omg.org/spec/DMN/20211108/MODEL/',

    // DMN 1.4 did not revise DMNDI
    dmndi: 'https://www.omg.org/spec/DMN/20191111/DMNDI/'
  },
  '1.5': {
    namespace: 'https://www.omg.org/spec/DMN/20230324/MODEL/',
    dmndi: 'https://www.omg.org/spec/DMN/20230324/DMNDI/'
  }
};

/**
 * Older DMN versions, read as DMN 1.3.
 */
const LEGACY_VERSIONS = {
  'http://www.omg.org/spec/DMN/20151101/dmn.xsd': '1.1',
  'http://www.omg.org/spec/DMN/20180521/MODEL/': '1.2'
};

const DMNDI_NAMESPACES = [
  'http://www.omg.org/spec/DMN/20180521/DMNDI/',
  'https://www.omg.org/spec/DMN/20191111/DMNDI/',
  'https://www.omg.org/spec/DMN/20230324/DMNDI/'
];

const moddles = {};

function getModdle(version) {

  if (!moddles[version]) {
    moddles[version] = version === '1.5'
      ? DmnModdle({ dmn: dmn15Schema, dmndi: dmndi15Schema })
      : version === '1.4'
        ? DmnModdle({ dmn: dmn14Schema })
        : DmnModdle();
  }

  return moddles[version];
}

/**
 * @typedef {object} ParseResult
 * @property {object} definitions moddle `dmn:Definitions`
 * @property {string} version DMN version of the model, e.g. `1.5`
 * @property {object[]} warnings
 */

/**
 * Parse DMN 1.3, 1.4 or 1.5 XML (as well as DMN 1.1 and 1.2, read as
 * DMN 1.3). Elements not defined by the model's DMN version are rejected.
 *
 * @param {string} xml
 *
 * @returns {Promise<ParseResult>}
 */
export async function parseDmn(xml) {

  const namespace = getDefinitionsNamespace(xml);

  let version = Object.keys(DMN_VERSIONS).find(v => DMN_VERSIONS[v].namespace === namespace);

  if (!version && LEGACY_VERSIONS[namespace]) {
    version = LEGACY_VERSIONS[namespace];

    xml = xml.replaceAll(namespace, DMN_VERSIONS['1.3'].namespace);
  }

  if (!version) {
    throw new Error(
      namespace
        ? `unsupported DMN namespace <${ namespace }>`
        : 'not a DMN document: no <definitions> element found'
    );
  }

  const parseVersion = DMN_VERSIONS[version] ? version : '1.3';

  // diagram information does not affect execution, accept any DMNDI version
  for (const dmndi of DMNDI_NAMESPACES) {
    if (dmndi !== DMN_VERSIONS[parseVersion].dmndi) {
      xml = xml.replaceAll(dmndi, DMN_VERSIONS[parseVersion].dmndi);
    }
  }

  const { rootElement, warnings } = await getModdle(parseVersion).fromXML(xml, 'dmn:Definitions', { lax: false });

  return {
    definitions: rootElement,
    version,
    warnings
  };
}

/**
 * Get the namespace of the `<definitions>` root element.
 *
 * @param {string} xml
 *
 * @return {string|null}
 */
export function getDefinitionsNamespace(xml) {

  // skip XML declaration, processing instructions, comments and doctype
  const content = xml.replace(/<\?[\s\S]*?\?>|<!--[\s\S]*?-->|<!DOCTYPE[^>]*>/g, '');

  const match = /<(?:([A-Za-z_][\w.-]*):)?definitions(?=[\s/>])((?:[^>"']|"[^"]*"|'[^']*')*)>/.exec(content);

  if (!match) {
    return null;
  }

  const [ , prefix = '', attributes ] = match;

  const pattern = /xmlns(?::([\w.-]+))?\s*=\s*(?:"([^"]*)"|'([^']*)')/g;

  for (let declaration; (declaration = pattern.exec(attributes));) {
    const [ , declaredPrefix = '', doubleQuoted, singleQuoted ] = declaration;

    if (declaredPrefix === prefix) {
      return doubleQuoted ?? singleQuoted;
    }
  }

  return null;
}
