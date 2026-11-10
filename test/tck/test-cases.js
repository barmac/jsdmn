/**
 * Reader for DMN TCK test case files (testCases.xsd).
 */

import { Parser } from 'saxen';

import {
  parseDate,
  parseDateTime,
  parseDuration,
  parseTime
} from '../../src/feel/temporal.js';

const TC_NS = 'http://www.omg.org/spec/DMN/20160719/testcase';
const XSI_NS = 'http://www.w3.org/2001/XMLSchema-instance';


/**
 * @typedef { { name: string, attrs: Record<string, string>, children: XmlElement[], text: string } } XmlElement
 */

/**
 * Parse XML into a minimal DOM; elements of the test case namespace
 * are prefixed with `tc:`, XML schema instance attributes with `xsi:`.
 *
 * @param {string} xml
 *
 * @return {XmlElement}
 */
export function parseXml(xml) {

  const parser = new Parser({ proxy: true });

  parser.ns({
    [TC_NS]: 'tc',
    [XSI_NS]: 'xsi'
  });

  const root = { name: '#root', attrs: {}, children: [], text: '' };
  const stack = [ root ];

  // closeTag is emitted for self-closing elements, too
  parser.on('openTag', (el) => {
    const element = {
      name: el.name,
      attrs: { ...el.attrs },
      children: [],
      text: ''
    };

    stack[stack.length - 1].children.push(element);
    stack.push(element);
  });

  parser.on('closeTag', () => {
    stack.pop();
  });

  parser.on('text', (text, decodeEntities) => {
    stack[stack.length - 1].text += decodeEntities(text);
  });

  parser.on('cdata', (text) => {
    stack[stack.length - 1].text += text;
  });

  parser.on('error', (error) => {
    throw error;
  });

  parser.parse(xml);

  return root.children[0];
}

function children(element, name) {
  return element.children.filter(child => child.name === name);
}

function child(element, name) {
  return children(element, name)[0] || null;
}


/**
 * @typedef {object} TestCase
 * @property {string} id
 * @property {string} [name]
 * @property {string} [description]
 * @property {'decision'|'bkm'|'decisionService'} type
 * @property {string} [invocableName]
 * @property {Array<{ name: string, namespace?: string, value: any }>} inputs
 * @property {Array<{ name: string, namespace?: string, type?: string, errorResult: boolean, expected: any }>} results
 */

/**
 * @typedef {object} TestSuite
 * @property {string} modelName
 * @property {string[]} labels
 * @property {TestCase[]} testCases
 */

/**
 * Parse a TCK test case file.
 *
 * @param {string} xml
 *
 * @return {TestSuite}
 */
export function parseTestSuite(xml) {

  const root = parseXml(xml);

  return {
    modelName: child(root, 'tc:modelName')?.text.trim(),
    labels: children(child(root, 'tc:labels') || { children: [] }, 'tc:label').map(label => label.text.trim()),
    testCases: children(root, 'tc:testCase').map(parseTestCase)
  };
}

function parseTestCase(element) {
  return {
    id: element.attrs.id,
    name: element.attrs.name,
    description: child(element, 'tc:description')?.text.trim(),
    type: element.attrs.type || 'decision',
    invocableName: element.attrs.invocableName,
    inputs: children(element, 'tc:inputNode').map(input => ({
      name: input.attrs.name,
      namespace: input.attrs.namespace,
      value: parseValue(input)
    })),
    results: children(element, 'tc:resultNode').map(result => {
      const expected = child(result, 'tc:expected');

      return {
        name: result.attrs.name,
        namespace: result.attrs.namespace,
        type: result.attrs.type,
        errorResult: result.attrs.errorResult === 'true',
        expected: expected ? parseValue(expected, { expected: true }) : null
      };
    })
  };
}

/**
 * Convert a TCK value (valueType) to a FEEL value.
 *
 * @param {XmlElement} element
 *
 * @return {any}
 */
export function parseValue(element, options = {}) {

  if (isNil(element)) {
    return null;
  }

  const value = child(element, 'tc:value');

  if (value) {
    return parseSimpleValue(value, options);
  }

  const list = child(element, 'tc:list');

  if (list) {
    return isNil(list) ? null : children(list, 'tc:item').map(item => parseValue(item, options));
  }

  const components = children(element, 'tc:component');

  if (components.length) {
    return Object.fromEntries(components.map(component => [
      component.attrs.name,
      parseValue(component, options)
    ]));
  }

  return null;
}

/**
 * An expected number, retaining the scale it is given with.
 */
export class ExpectedNumber {

  constructor(value, text) {
    this.value = value;
    this.scale = (/\.(\d+)$/.exec(text) || [ '', '' ])[1].length;
  }

  toString() {
    return String(this.value);
  }
}

function isNil(element) {
  return element.attrs['xsi:nil'] === 'true';
}

/**
 * A value the FEEL implementation cannot represent.
 */
export class UnparsableValue {

  constructor(type, text, error) {
    this.type = type;
    this.text = text;
    this.error = error;
  }

  toString() {
    return `unparsable ${ this.type } <${ this.text }>: ${ this.error.message }`;
  }
}

function parseSimpleValue(element, options) {

  if (isNil(element)) {
    return null;
  }

  const type = (element.attrs['xsi:type'] || '').replace(/^\w+:/, '');
  const text = element.text;

  let value;

  try {
    value = convertSimpleValue(type, text);
  } catch (error) {
    return new UnparsableValue(type, text, error);
  }

  if (value === null && [ 'date', 'dateTime', 'time', 'duration' ].includes(type)) {
    return new UnparsableValue(type, text, new Error('invalid or unsupported value'));
  }

  if (options.expected && typeof value === 'number') {
    return new ExpectedNumber(value, text.trim());
  }

  return value;
}

function convertSimpleValue(type, text) {

  switch (type) {
  case 'decimal':
  case 'double':
  case 'float':
  case 'integer':
  case 'int':
  case 'long':
    return parseNumber(text.trim());
  case 'boolean':
    return text.trim() === 'true';
  case 'date':
    return parseDate(text.trim());
  case 'dateTime':
    return parseDateTime(text.trim());
  case 'time':
    return parseTime(text.trim());
  case 'duration':
    return parseDuration(text.trim());
  default:
    return text;
  }
}

function parseNumber(text) {

  if (text === 'NaN') {
    return NaN;
  }

  if (text === 'INF' || text === '+INF') {
    return Infinity;
  }

  if (text === '-INF') {
    return -Infinity;
  }

  return Number(text);
}
