/**
 * Re-expressing TCK models in other DMN versions.
 *
 * The latest TCK is authored in DMN 1.5; earlier TCK revisions (e.g. the
 * `DMNv1.3` and `DMNv1.4` tags) in earlier versions.
 */

/**
 * The DMN version the latest TCK models are authored in.
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
 * The DMN version a model is authored in.
 *
 * @param {string} xml
 *
 * @return {string|null}
 */
export function getModelVersion(xml) {
  return DMN_VERSIONS.find(version => xml.includes(`"${ NAMESPACES[version].model }"`)) || null;
}

/**
 * Elements used in a model that do not exist in the given version.
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
 * Convert a model to the given DMN version. Converting to a later
 * version is always possible, converting to an earlier one only if
 * the model uses no elements introduced later (cf. {@link getUnsupportedElements}).
 *
 * @param {string} xml
 * @param {string} version
 *
 * @return {string}
 */
export function convertModel(xml, version) {

  const sourceVersion = getModelVersion(xml);

  if (!sourceVersion || sourceVersion === version) {
    return xml;
  }

  const from = NAMESPACES[sourceVersion];
  const to = NAMESPACES[version];

  if (!to) {
    throw new Error(`unsupported DMN version <${ version }>`);
  }

  xml = xml
    .replaceAll(from.model, to.model)
    .replaceAll(from.dmndi, to.dmndi)
    .replaceAll(from.feel, to.feel);

  // introduced with DMNDI 1.5
  if (compareVersions(version, '1.5') < 0) {
    xml = xml.replace(/\s+useAlternativeInputDataShape="[^"]*"/g, '');
  }

  return xml;
}

function compareVersions(a, b) {
  const [ majorA, minorA ] = a.split('.').map(Number);
  const [ majorB, minorB ] = b.split('.').map(Number);

  return majorA - majorB || minorA - minorB;
}
