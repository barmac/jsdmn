/**
 * Re-expressing the (DMN 1.5) TCK models in earlier DMN versions.
 */

/**
 * The DMN version the TCK models are authored in.
 */
export const TCK_DMN_VERSION = '1.5';

const NAMESPACES = {
  '1.3': {
    model: 'https://www.omg.org/spec/DMN/20191111/MODEL/',
    dmndi: 'https://www.omg.org/spec/DMN/20191111/DMNDI/',
    feel: 'https://www.omg.org/spec/DMN/20191111/FEEL/'
  },
  '1.4': {
    model: 'https://www.omg.org/spec/DMN/20211108/MODEL/',
    dmndi: 'https://www.omg.org/spec/DMN/20191111/DMNDI/',
    feel: 'https://www.omg.org/spec/DMN/20211108/FEEL/'
  },
  '1.5': {
    model: 'https://www.omg.org/spec/DMN/20230324/MODEL/',
    dmndi: 'https://www.omg.org/spec/DMN/20230324/DMNDI/',
    feel: 'https://www.omg.org/spec/DMN/20230324/FEEL/'
  }
};

/**
 * Model elements by the DMN version introducing them.
 */
const INTRODUCED_ELEMENTS = {
  '1.4': [ 'conditional', 'filter', 'for', 'every', 'some' ],
  '1.5': [ 'typeConstraint' ]
};

export const DMN_VERSIONS = Object.keys(NAMESPACES);

/**
 * Elements used in a DMN 1.5 model that do not exist in the given version.
 *
 * @param {string} xml
 * @param {string} version
 *
 * @return {string[]}
 */
export function getUnsupportedElements(xml, version) {

  const unsupported = Object.entries(INTRODUCED_ELEMENTS)
    .filter(([ introducedIn ]) => compareVersions(introducedIn, version) > 0)
    .flatMap(([ , elements ]) => elements);

  return unsupported.filter(element => new RegExp(`<(?:[\\w.-]+:)?${ element }(?=[\\s/>])`).test(xml));
}

/**
 * Convert a DMN 1.5 model to the given DMN version.
 *
 * @param {string} xml
 * @param {string} version
 *
 * @return {string}
 */
export function convertModel(xml, version) {

  if (version === TCK_DMN_VERSION) {
    return xml;
  }

  const from = NAMESPACES[TCK_DMN_VERSION];
  const to = NAMESPACES[version];

  if (!to) {
    throw new Error(`unsupported DMN version <${ version }>`);
  }

  return xml
    .replaceAll(from.model, to.model)
    .replaceAll(from.dmndi, to.dmndi)
    .replaceAll(from.feel, to.feel)

    // introduced with DMNDI 1.5
    .replace(/\s+useAlternativeInputDataShape="[^"]*"/g, '');
}

function compareVersions(a, b) {
  const [ majorA, minorA ] = a.split('.').map(Number);
  const [ majorB, minorB ] = b.split('.').map(Number);

  return majorA - majorB || minorA - minorB;
}
