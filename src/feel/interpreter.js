/**
 * A FEEL interpreter.
 *
 * Expressions are parsed with feelin (lezer-feel), which recognizes names
 * containing spaces based on the variables in scope, and evaluated by
 * walking the resulting syntax tree.
 */

import {
  parseExpression,
  parseUnaryTests
} from 'feelin';

import {
  getType,
  isContext,
  equals,
  compare,
  rangeIncludes
} from './values.js';

import {
  Range,
  createInterval,
  createComparisonRange,
  isRange,
  toRange
} from './range.js';

import {
  isDate,
  isDateTime,
  isDuration,
  isTemporal,
  isTime,
  addDuration,
  addDurations,
  subtractTemporals,
  multiplyDuration,
  divideDuration,
  negateDuration,
  parseTemporal,
  createDate,
  yearOffset,
  timeOffset,
  secondWithFraction
} from './temporal.js';

import {
  DmnFunction,
  invoke,
  invocationFailure
} from './functions.js';

import { getBuiltin, registerBuiltin } from './builtins.js';

import { createRangeFunction } from './builtins/ranges.js';

import { resolveJavaMethod } from '../java.js';

import {
  ANY,
  ContextType,
  FunctionType,
  ListType,
  RangeType,
  coerce,
  getBuiltinType
} from '../types.js';


/**
 * @typedef {object} Warning
 * @property {string} type
 * @property {string} message
 * @property { { from: number, to: number } } [position]
 */

/**
 * @typedef {object} EvaluateOptions
 * @property {(name: string) => import('../types.js').Type|null} [resolveType]
 *           resolve a (user defined) type name, e.g. an item definition
 */

export class FeelSyntaxError extends Error {

  constructor(message, input, position) {
    super(message);

    this.input = input;
    this.position = position;
  }
}


// parsing ///////////////

const COMMENTS = new Set([ 'LineComment', 'BlockComment' ]);

const SIMPLE_NAME = /^[\p{L}_$?][\p{L}\p{N}_$]*$/u;

const astCache = new Map();

const AST_CACHE_SIZE = 5000;

/**
 * Parse FEEL into a light weight AST.
 *
 * @param {string} text
 * @param {object} scope
 * @param {'expression'|'unaryTests'} mode
 */
function parse(text, scope, mode) {

  const names = [];

  for (const key in scope) {
    if (!SIMPLE_NAME.test(key)) {
      names.push(key);
    }
  }

  const cacheKey = `${ mode }\u0000${ text }\u0000${ names.sort().join('\u0000') }`;

  let ast = astCache.get(cacheKey);

  if (ast) {
    return ast;
  }

  const parseContext = {};

  for (const key in scope) {
    parseContext[key] = scope[key];
  }

  const source = rewriteRangeTypes(text);

  const tree = mode === 'unaryTests'
    ? parseUnaryTests(source, parseContext)
    : parseExpression(source, parseContext);

  ast = buildAst(tree.topNode, source);

  const error = findError(ast);

  if (error) {
    throw new FeelSyntaxError(
      `syntax error at position ${ error.from }${ error.text ? ` near <${ error.text }>` : '' }`,
      text,
      { from: error.from, to: error.to }
    );
  }

  if (astCache.size > AST_CACHE_SIZE) {
    astCache.clear();
  }

  astCache.set(cacheKey, ast);

  return ast;
}

const RANGE_TYPE_MARKER = '__range__';

/**
 * The grammar does not support `range<T>` types; rewrite them to a
 * marker context type, resolved as range type during evaluation.
 */
function rewriteRangeTypes(text) {

  if (!/\brange\s*</.test(text)) {
    return text;
  }

  return text.replace(/(\binstance\s+of\s+|<\s*|,\s*|:\s*)range\s*</g, `$1context<${ RANGE_TYPE_MARKER }: `);
}

function buildAst(node, input) {

  const children = [];

  for (let child = node.firstChild; child; child = child.nextSibling) {

    if (COMMENTS.has(child.name)) {
      continue;
    }

    children.push(buildAst(child, input));
  }

  return {
    type: node.type.isError ? '⚠' : node.name,
    text: input.slice(node.from, node.to),
    from: node.from,
    to: node.to,
    children
  };
}

function findError(ast, parent = null) {

  if (ast.type === '⚠') {

    // `function` as a type parses with an empty error node
    if (!ast.text && parent?.type === 'FunctionType' && parent.children.length === 2) {
      return null;
    }

    return ast;
  }

  for (const child of ast.children) {
    const error = findError(child, ast);

    if (error) {
      return error;
    }
  }

  return null;
}

function containsInputReference(ast) {

  if (ast.type === '?') {
    return true;
  }

  return ast.children.some(containsInputReference);
}


// public API ///////////////

/**
 * Evaluate a FEEL expression.
 *
 * @param {string} text
 * @param {object} [scope]
 * @param {EvaluateOptions} [options]
 *
 * @return { { value: any, warnings: Warning[] } }
 */
export function evaluate(text, scope = {}, options = {}) {

  const ast = parse(text, scope, 'expression');

  const interpreter = new Interpreter(options);

  const value = interpreter.evaluate(ast, createScope(scope));

  return {
    value: normalize(value),
    warnings: interpreter.warnings
  };
}

/**
 * Test an input value against FEEL unary tests.
 *
 * @param {string} text
 * @param {any} input
 * @param {object} [scope]
 * @param {EvaluateOptions} [options]
 *
 * @return { { value: boolean|null, warnings: Warning[] } }
 */
export function unaryTest(text, input, scope = {}, options = {}) {

  const rootScope = createScope(scope);

  rootScope['?'] = input === undefined ? null : input;

  const ast = parse(text, rootScope, 'unaryTests');

  const interpreter = new Interpreter(options);

  const value = interpreter.evaluateUnaryTests(ast, input === undefined ? null : input, rootScope);

  return {
    value,
    warnings: interpreter.warnings
  };
}

function createScope(scope) {
  const root = Object.create(null);

  for (const key in scope) {
    root[key] = scope[key];
  }

  return root;
}

function childScope(scope, values = {}) {
  const child = Object.create(scope);

  for (const key in values) {
    child[key] = values[key];
  }

  return child;
}

function normalize(value) {
  return value === undefined ? null : value;
}

function normalizeName(name) {
  return name.replace(/\s+/g, ' ').trim();
}

/**
 * Look up a name in a scope, tolerating whitespace differences.
 */
function lookup(scope, name) {

  if (name in scope) {
    return { found: true, value: scope[name] };
  }

  const normalized = normalizeName(name);

  if (normalized in scope) {
    return { found: true, value: scope[normalized] };
  }

  for (const key in scope) {
    if (normalizeName(key) === normalized) {
      return { found: true, value: scope[key] };
    }
  }

  return { found: false, value: null };
}

function has(obj, key) {
  return Object.prototype.hasOwnProperty.call(obj, key);
}

/**
 * Get a context entry, tolerating whitespace differences.
 */
function getEntry(context, name) {

  if (has(context, name)) {
    return { found: true, value: context[name] };
  }

  const normalized = normalizeName(name);

  for (const key of Object.keys(context)) {
    if (normalizeName(key) === normalized) {
      return { found: true, value: context[key] };
    }
  }

  return { found: false, value: null };
}


// string literals ///////////////

function parseString(text) {

  const str = text.slice(1, -1);

  return str.replace(/\\(?:u\{([0-9a-fA-F]+)\}|U([0-9a-fA-F]{6})|u([0-9a-fA-F]{4})|(.))/gs, (match, braced, long, short, char) => {

    if (braced || long) {
      return String.fromCodePoint(parseInt(braced || long, 16));
    }

    if (short) {
      return String.fromCharCode(parseInt(short, 16));
    }

    switch (char) {
    case 'n': return '\n';
    case 'r': return '\r';
    case 't': return '\t';
    case 'b': return '\b';
    case 'f': return '\f';
    case '"': return '"';
    case '\'': return '\'';
    case '\\': return '\\';
    default: return match;
    }
  });
}


// interpreter ///////////////

const TEMPORAL_PROPERTIES = {
  'date': [ 'year', 'month', 'day', 'weekday' ],
  'date and time': [ 'year', 'month', 'day', 'weekday', 'hour', 'minute', 'second', 'time offset', 'timezone' ],
  'time': [ 'hour', 'minute', 'second', 'time offset', 'timezone' ],
  'days and time duration': [ 'days', 'hours', 'minutes', 'seconds' ],
  'years and months duration': [ 'years', 'months' ]
};

const RANGE_PROPERTIES = [ 'start', 'end', 'start included', 'end included' ];

class Interpreter {

  /**
   * @param {EvaluateOptions} options
   */
  constructor(options) {
    this.options = options;

    /**
     * @type {Warning[]}
     */
    this.warnings = [];
  }

  warn(type, message, node) {
    this.warnings.push({
      type,
      message,
      position: node ? { from: node.from, to: node.to } : undefined
    });
  }

  /**
   * @param {object} node
   * @param {object} scope
   */
  evaluate(node, scope) {

    const handler = this[node.type];

    if (!handler) {
      this.warn('UNSUPPORTED', `unsupported expression <${ node.text }> (${ node.type })`, node);

      return null;
    }

    return handler.call(this, node, scope);
  }

  // structure

  Expression(node, scope) {
    return this.evaluate(node.children[0], scope);
  }

  ParenthesizedExpression(node, scope) {
    return this.evaluate(node.children[1], scope);
  }

  FunctionBody(node, scope) {
    return this.evaluate(node.children[0], scope);
  }

  PositiveUnaryTest(node, scope) {
    return this.evaluate(node.children[0], scope);
  }

  // literals

  NumericLiteral(node) {
    return Number(node.text.replace(/\s+/g, ''));
  }

  StringLiteral(node) {
    return parseString(node.text);
  }

  BooleanLiteral(node) {
    return node.text === 'true';
  }

  null() {
    return null;
  }

  List(node, scope) {
    return node.children
      .filter(child => child.type !== '[' && child.type !== ']')
      .map(child => this.evaluate(child, scope));
  }

  Context(node, scope) {

    const entryScope = childScope(scope);
    const result = {};

    for (const entry of node.children.filter(child => child.type === 'ContextEntry')) {

      const [ keyNode, valueNode ] = entry.children;

      const key = this.getKey(keyNode);

      if (has(result, key)) {
        this.warn('INVALID_TYPE', `duplicate context entry <${ key }>`, entry);

        return null;
      }

      const value = normalize(this.evaluate(valueNode, entryScope));

      entryScope[key] = value;
      result[key] = value;
    }

    return result;
  }

  getKey(keyNode) {
    const child = keyNode.type === 'Key' ? keyNode.children[0] : keyNode;

    if (child.type === 'StringLiteral') {
      return parseString(child.text);
    }

    return normalizeName(child.text);
  }

  DateTimeLiteral(node, scope) {

    const [ first ] = node.children;

    if (first.type === 'AtLiteral') {
      return this.evaluate(first, scope);
    }

    const fn = getBuiltin(normalizeName(first.text));

    return this.invokeFunction(fn, node.children.find(isParameters), scope, node, first.text);
  }

  AtLiteral(node) {

    const literal = parseString(node.children[0].text);
    const value = parseTemporal(literal);

    if (value === null) {
      this.warn('INVALID_TYPE', `invalid temporal literal <@"${ literal }">`, node);
    }

    return value;
  }

  Interval(node, scope) {

    const [ open, startNode, , endNode, close ] = node.children;

    const start = normalize(this.evaluate(startNode, scope));
    const end = normalize(this.evaluate(endNode, scope));

    const startType = getType(start);
    const endType = getType(end);

    if (start !== null && end !== null && startType !== endType) {
      this.warn('INVALID_TYPE', `range bounds <${ start }> and <${ end }> are of different types`, node);

      return null;
    }

    if (startType === 'context' || startType === 'list' || startType === 'boolean' || startType === 'function' || startType === 'range') {
      this.warn('INVALID_TYPE', `invalid range bound type <${ startType }>`, node);

      return null;
    }

    return createInterval(start, end, open.type === '[', close.type === ']', start !== null ? startType : end !== null ? endType : null);
  }

  SimplePositiveUnaryTest(node, scope) {

    const [ first, second ] = node.children;

    if (first.type === 'Interval') {
      return this.evaluate(first, scope);
    }

    const endpoint = normalize(this.evaluate(second, scope));

    return createComparisonRange(first.text, endpoint, endpoint === null ? null : getType(endpoint));
  }

  // names

  VariableName(node, scope) {

    const name = node.text;

    const { found, value } = lookup(scope, name);

    if (found) {
      return value;
    }

    const builtin = getBuiltin(normalizeName(name));

    if (builtin) {
      return builtin;
    }

    this.warn('NO_VARIABLE_FOUND', `variable <${ normalizeName(name) }> not found`, node);

    return null;
  }

  Name(node) {
    return normalizeName(node.text);
  }

  SpecialFunctionName(node) {
    return getBuiltin(normalizeName(node.text)) || null;
  }

  '?'(node, scope) {
    return lookup(scope, '?').value;
  }

  PathExpression(node, scope) {

    const [ baseNode, , nameNode ] = node.children;

    const base = normalize(this.evaluate(baseNode, scope));

    return this.getProperty(base, normalizeName(nameNode.text), node);
  }

  getProperty(base, name, node) {

    if (base === null) {
      return null;
    }

    if (Array.isArray(base)) {
      return base.map(element => this.getProperty(element, name, node));
    }

    const type = getType(base);

    if (type === 'context') {
      const { found, value } = getEntry(base, name);

      if (!found) {
        this.warn('NO_CONTEXT_ENTRY_FOUND', `context entry <${ name }> not found`, node);
      }

      return value;
    }

    if (TEMPORAL_PROPERTIES[type]) {

      if (!TEMPORAL_PROPERTIES[type].includes(name)) {
        this.warn('NO_PROPERTY_FOUND', `property <${ name }> not defined for <${ type }>`, node);

        return null;
      }

      if (name === 'time offset') {
        return timeOffset(base);
      }

      if (name === 'second' && (isTime(base) || isDateTime(base))) {
        return secondWithFraction(base);
      }

      if (name === 'timezone') {
        return base.zone === null || base.zone === 'UTC' || /^[+-]/.test(base.zone) ? null : base.zone;
      }

      return normalize(base[name]);
    }

    if (type === 'range') {

      if (!RANGE_PROPERTIES.includes(name)) {
        this.warn('NO_PROPERTY_FOUND', `property <${ name }> not defined for ranges`, node);

        return null;
      }

      return normalize(base[name]);
    }

    this.warn('NO_PROPERTY_FOUND', `cannot access <${ name }> of <${ type }>`, node);

    return null;
  }

  FilterExpression(node, scope) {

    const [ targetNode, , filterNode ] = node.children;

    let target = normalize(this.evaluate(targetNode, scope));

    if (target === null) {
      return null;
    }

    if (!Array.isArray(target)) {
      target = [ target ];
    }

    // entries of context items shadow the implicit `item`
    const itemScope = (item) => childScope(scope, {
      item,
      ...(isContext(item) ? item : {})
    });

    if (!target.length) {
      const result = normalize(this.evaluate(filterNode, itemScope(null)));

      return typeof result === 'number' ? null : [];
    }

    const results = [];

    for (const [ idx, item ] of target.entries()) {

      const result = normalize(this.evaluate(filterNode, itemScope(item)));

      // index access
      if (idx === 0 && typeof result === 'number') {
        return this.getByIndex(target, result);
      }

      if (result === true || (isRange(result) && rangeIncludes(result, item) === true)) {
        results.push(item);
      }
    }

    return results;
  }

  getByIndex(list, index) {

    if (!Number.isInteger(index) || index === 0) {
      return null;
    }

    const value = index > 0 ? list[index - 1] : list[list.length + index];

    return value === undefined ? null : value;
  }

  // functions

  FunctionInvocation(node, scope) {

    const [ targetNode ] = node.children;
    const parameters = node.children.find(isParameters);

    let fn;

    if (targetNode.type === 'VariableName' || targetNode.type === 'SpecialFunctionName') {
      const name = targetNode.text;

      const { found, value } = lookup(scope, name);

      fn = found && getType(value) === 'function'
        ? value
        : getBuiltin(normalizeName(name)) || (found ? value : undefined);

      if (fn === undefined) {
        this.warn('NO_FUNCTION_FOUND', `function <${ normalizeName(name) }> not found`, targetNode);

        return null;
      }
    } else {
      fn = this.evaluate(targetNode, scope);
    }

    return this.invokeFunction(fn, parameters, scope, node, targetNode.text);
  }

  invokeFunction(fn, parameters, scope, node, name) {

    let args = [];

    if (parameters && parameters.type === 'NamedParameters') {
      args = {};

      for (const parameter of parameters.children.filter(child => child.type === 'NamedParameter')) {
        const [ nameNode, valueNode ] = parameter.children;

        args[normalizeName(nameNode.text)] = normalize(this.evaluate(valueNode, scope));
      }
    } else if (parameters) {
      args = parameters.children.map(child => normalize(this.evaluate(child, scope)));
    }

    if (getType(fn) !== 'function') {
      this.warn('NO_FUNCTION_FOUND', `<${ normalizeName(name) }> is not a function`, node);

      return null;
    }

    const { value, error } = invoke(fn, args);

    if (error) {
      this.warn('FUNCTION_INVOCATION_FAILURE', `failed to invoke <${ normalizeName(name) }>: ${ error }`, node);
    }

    return value;
  }

  FunctionDefinition(node, scope) {

    const parametersNode = node.children.find(child => child.type === 'FormalParameters');
    const body = node.children.find(child => child.type === 'FunctionBody');
    const external = node.children.some(child => child.type === 'external');

    const parameters = (parametersNode?.children || [])
      .filter(child => child.type === 'FormalParameter')
      .map(parameter => {
        const [ nameNode, typeNode ] = parameter.children;

        return {
          name: normalizeName(nameNode.text),
          type: typeNode ? this.evaluateType(typeNode, scope) : ANY
        };
      });

    const names = parameters.map(p => p.name);

    const type = new FunctionType(parameters.map(p => p.type), ANY);

    if (external) {
      return this.createExternalFunction(body, parameters, scope, node, type);
    }

    return new DmnFunction((args) => {

      const fnScope = childScope(scope);

      for (const [ idx, parameter ] of parameters.entries()) {
        const { value, error } = coerce(args[idx], parameter.type);

        if (error) {
          return invocationFailure(`parameter <${ parameter.name }>: ${ error }`);
        }

        fnScope[parameter.name] = value;
      }

      return normalize(this.evaluate(body, fnScope));
    }, names, { type });
  }

  createExternalFunction(body, parameters, scope, node, type) {

    const mapping = normalize(this.evaluate(body, scope));

    const java = isContext(mapping) && isContext(mapping.java) ? mapping.java : null;

    if (!java) {
      this.warn('UNSUPPORTED', 'only Java external functions are supported', node);

      return null;
    }

    const className = java['class'];
    const methodSignature = java['method signature'];

    const method = resolveJavaMethod(className, methodSignature);

    if (!method) {
      this.warn('FUNCTION_INVOCATION_FAILURE', `cannot resolve Java method <${ className }.${ methodSignature }>`, node);
    }

    return new DmnFunction((args) => {

      if (!method) {
        return invocationFailure(`cannot resolve Java method <${ className }.${ methodSignature }>`);
      }

      try {
        return method(...args);
      } catch (error) {
        return invocationFailure(error.message);
      }
    }, parameters.map(p => p.name), { type });
  }

  // operators

  ArithmeticExpression(node, scope) {

    const { children } = node;

    // unary minus
    if (children.length === 2) {
      const value = normalize(this.evaluate(children[1], scope));

      if (value === null) {
        return null;
      }

      if (typeof value === 'number') {
        return -value;
      }

      if (isDuration(value)) {
        return negateDuration(value);
      }

      this.warn('INVALID_TYPE', `cannot negate <${ getType(value) }>`, node);

      return null;
    }

    const [ leftNode, opNode, rightNode ] = children;

    const left = normalize(this.evaluate(leftNode, scope));
    const right = normalize(this.evaluate(rightNode, scope));

    if (left === null || right === null) {
      return null;
    }

    const result = arithmetic(opNode.text, left, right);

    if (result === undefined) {
      this.warn('INVALID_TYPE', `cannot apply <${ opNode.text }> to <${ getType(left) }> and <${ getType(right) }>`, node);

      return null;
    }

    return result;
  }

  Comparison(node, scope) {

    const { children } = node;

    const [ leftNode, opNode ] = children;

    const left = normalize(this.evaluate(leftNode, scope));

    if (opNode.type === 'between') {
      const low = normalize(this.evaluate(children[2], scope));
      const high = normalize(this.evaluate(children[4], scope));

      return and(
        compareWith('>=', left, low),
        compareWith('<=', left, high)
      );
    }

    if (opNode.type === 'in') {
      const target = children[2].type === '(' ? children[3] : children[2];

      return this.testMembership(left, target, scope);
    }

    const right = normalize(this.evaluate(children[2], scope));

    const result = compareWith(opNode.text, left, right);

    if (result === null && left !== null && right !== null) {
      this.warn('NOT_COMPARABLE', `cannot compare <${ getType(left) }> and <${ getType(right) }>`, node);
    }

    return result;
  }

  testMembership(value, target, scope) {

    if (target.type === 'PositiveUnaryTests') {
      return this.testPositiveUnaryTests(target, value, scope);
    }

    if (target.type === 'PositiveUnaryTest') {
      return this.testPositiveUnaryTest(target, value, scope);
    }

    return this.testExpression(target, value, scope);
  }

  Conjunction(node, scope) {

    const left = normalize(this.evaluate(node.children[0], scope));

    if (left === false) {
      return false;
    }

    return and(left, normalize(this.evaluate(node.children[2], scope)));
  }

  Disjunction(node, scope) {

    const left = normalize(this.evaluate(node.children[0], scope));

    if (left === true) {
      return true;
    }

    return or(left, normalize(this.evaluate(node.children[2], scope)));
  }

  IfExpression(node, scope) {

    const [ , conditionNode, , thenNode, , elseNode ] = node.children;

    const condition = normalize(this.evaluate(conditionNode, scope));

    return this.evaluate(condition === true ? thenNode : elseNode, scope);
  }

  InstanceOfExpression(node, scope) {

    const [ valueNode, , , typeNode ] = node.children;

    const value = normalize(this.evaluate(valueNode, scope));

    const type = this.evaluateType(typeNode, scope);

    if (!type) {
      return null;
    }

    return type.isInstance(value);
  }

  evaluateType(node, scope) {

    switch (node.type) {

    case 'Type':
      return this.evaluateType(node.children[0], scope);

    case 'QualifiedName':
    case 'SpecialType': {
      const name = normalizeName(node.text.replace(/\s*\.\s*/g, '.'));

      const type = getBuiltinType(name) || this.options.resolveType?.(name);

      if (type) {
        return type;
      }

      // a type test provided through the scope
      const { found, value } = lookup(scope, name);

      if (found && typeof value === 'function') {
        return {
          name,
          isInstance: (v) => v instanceof value
        };
      }

      this.warn('NO_VARIABLE_FOUND', `unknown type <${ name }>`, node);

      return null;
    }

    case 'ListType': {
      const elementType = this.evaluateType(node.children.find(child => child.type === 'Type'), scope);

      return elementType ? new ListType(elementType) : null;
    }

    case 'ContextType': {
      const entriesNode = node.children.find(child => child.type === 'ContextEntryTypes');

      const entries = (entriesNode?.children || [])
        .filter(child => child.type === 'ContextEntryType')
        .map(entry => {
          const [ nameNode, typeNode ] = entry.children;

          return [ normalizeName(nameNode.text), this.evaluateType(typeNode, scope) ];
        });

      if (entries.length === 1 && entries[0][0] === RANGE_TYPE_MARKER) {
        return entries[0][1] ? new RangeType(entries[0][1]) : null;
      }

      return entries.every(([ , type ]) => type) ? new ContextType(entries) : null;
    }

    case 'FunctionType': {
      const argumentTypes = node.children.find(child => child.type === 'ArgumentTypes');
      const returnTypeNode = node.children.find(child => child.type === 'Type');

      if (!argumentTypes) {
        return getBuiltinType('function');
      }

      const parameterTypes = argumentTypes.children
        .filter(child => child.type === 'ArgumentType')
        .map(child => this.evaluateType(child.children[0], scope));

      const returnType = returnTypeNode ? this.evaluateType(returnTypeNode, scope) : ANY;

      return new FunctionType(parameterTypes, returnType);
    }

    default:
      this.warn('UNSUPPORTED', `unsupported type <${ node.text }>`, node);

      return null;
    }
  }

  // iteration

  ForExpression(node, scope) {

    const inExpressions = node.children.find(child => child.type === 'InExpressions');
    const body = node.children[node.children.length - 1];

    const results = [];

    const ok = this.iterate(inExpressions.children.filter(c => c.type === 'InExpression'), scope, (iterationScope) => {
      iterationScope.partial = [ ...results ];

      results.push(normalize(this.evaluate(body, iterationScope)));
    });

    return ok ? results : null;
  }

  QuantifiedExpression(node, scope) {

    const quantifier = node.children[0].type;
    const inExpressions = node.children.find(child => child.type === 'InExpressions');
    const body = node.children[node.children.length - 1];

    const results = [];

    const ok = this.iterate(inExpressions.children.filter(c => c.type === 'InExpression'), scope, (iterationScope) => {
      results.push(normalize(this.evaluate(body, iterationScope)));
    });

    if (!ok) {
      return null;
    }

    return quantifier === 'some'
      ? results.some(result => result === true)
      : results.every(result => result === true);
  }

  /**
   * Iterate over the cartesian product of iteration contexts.
   *
   * @return {boolean} whether the iteration contexts were valid
   */
  iterate(inExpressions, scope, callback) {

    if (!inExpressions.length) {
      callback(childScope(scope));

      return true;
    }

    const [ first, ...rest ] = inExpressions;

    const [ nameNode, , contextNode ] = first.children;

    const name = normalizeName(nameNode.text);

    const items = this.getIterationItems(contextNode, scope);

    if (items === null) {
      return false;
    }

    for (const item of items) {
      const iterationScope = childScope(scope, { [name]: item });

      if (!this.iterate(rest, iterationScope, callback)) {
        return false;
      }
    }

    return true;
  }

  getIterationItems(contextNode, scope) {

    const { children } = contextNode;

    // a..b
    if (children.length === 3) {
      const start = normalize(this.evaluate(children[0], scope));
      const end = normalize(this.evaluate(children[2], scope));

      const items = enumerate(start, end, true, true, true);

      if (items === null) {
        this.warn('INVALID_TYPE', `cannot iterate from <${ start }> to <${ end }>`, contextNode);
      }

      return items;
    }

    const value = normalize(this.evaluate(children[0], scope));

    if (value === null) {
      return [];
    }

    if (Array.isArray(value)) {
      return value;
    }

    if (isRange(value)) {
      const range = toRange(value);

      const items = range.startUnbounded || range.endUnbounded || range.operator
        ? null
        : enumerate(range.start, range.end, range['start included'], range['end included'], false);

      if (items === null) {
        this.warn('INVALID_TYPE', `cannot iterate over range <${ range }>`, contextNode);
      }

      return items;
    }

    return [ value ];
  }

  // unary tests

  evaluateUnaryTests(ast, input, scope) {

    const [ first ] = ast.children;

    if (!first) {
      return true;
    }

    if (first.type === 'Wildcard') {
      return true;
    }

    if (first.type === 'not') {
      const tests = ast.children.find(child => child.type === 'PositiveUnaryTests');

      return not(this.testPositiveUnaryTests(tests, input, scope));
    }

    return this.testPositiveUnaryTests(first, input, scope);
  }

  testPositiveUnaryTests(node, input, scope) {

    const results = node.children
      .filter(child => child.type === 'PositiveUnaryTest')
      .map(child => this.testPositiveUnaryTest(child, input, scope));

    if (results.includes(true)) {
      return true;
    }

    return results.includes(null) ? null : false;
  }

  testPositiveUnaryTest(node, input, scope) {

    const test = node.type === 'PositiveUnaryTest' ? node.children[0] : node;

    if (test.type === 'SimplePositiveUnaryTest') {
      const [ first, second ] = test.children;

      if (first.type === 'Interval') {
        const range = this.evaluate(first, scope);

        return range === null ? null : rangeIncludes(range, input);
      }

      const endpoint = normalize(this.evaluate(second, scope));

      return compareWith(first.text, input, endpoint);
    }

    if (test.type === 'null') {
      return input === null;
    }

    return this.testExpression(test, input, scope);
  }

  testExpression(test, input, scope) {

    const usesInput = containsInputReference(test);

    const value = normalize(this.evaluate(test, usesInput ? childScope(scope, { '?': input }) : scope));

    if (usesInput && typeof value === 'boolean') {
      return value;
    }

    if (isRange(value)) {
      return rangeIncludes(value, input);
    }

    if (Array.isArray(value)) {

      if (Array.isArray(input) && equals(input, value) === true) {
        return true;
      }

      return value.some(element => {
        return isRange(element) ? rangeIncludes(element, input) === true : equals(input, element) === true;
      });
    }

    if (typeof value === 'boolean' && typeof input !== 'boolean' && input !== null) {
      return value;
    }

    return equals(input, value);
  }
}

function isParameters(node) {
  return node.type === 'PositionalParameters' || node.type === 'NamedParameters';
}


// semantics helpers ///////////////

function and(a, b) {

  if (a === false || b === false) {
    return false;
  }

  if (a === true && b === true) {
    return true;
  }

  return null;
}

function or(a, b) {

  if (a === true || b === true) {
    return true;
  }

  if (a === false && b === false) {
    return false;
  }

  return null;
}

function not(a) {
  return typeof a === 'boolean' ? !a : null;
}

/**
 * @param {string} op
 * @param {any} a
 * @param {any} b
 *
 * @return {boolean|null}
 */
export function compareWith(op, a, b) {

  switch (op) {
  case '=':
    return equals(a, b);
  case '!=':
    return not(equals(a, b));
  }

  const result = compare(a, b);

  if (result === null) {
    return null;
  }

  switch (op) {
  case '<': return result < 0;
  case '<=': return result <= 0;
  case '>': return result > 0;
  case '>=': return result >= 0;
  default: return null;
  }
}

/**
 * @return {any|undefined} `undefined` for invalid operand types
 */
function arithmetic(op, a, b) {

  const typeA = getType(a);
  const typeB = getType(b);

  const numbers = typeA === 'number' && typeB === 'number';

  switch (op) {

  case '+':
    if (numbers || (typeA === 'string' && typeB === 'string')) {
      return a + b;
    }

    if (isDuration(a) && isDuration(b)) {
      return addDurations(a, b, 1) ?? undefined;
    }

    if (isTemporal(a) && isDuration(b)) {
      return addDuration(a, b, 1) ?? undefined;
    }

    if (isDuration(a) && isTemporal(b)) {
      return addDuration(b, a, 1) ?? undefined;
    }

    return undefined;

  case '-':
    if (numbers) {
      return a - b;
    }

    if (isDuration(a) && isDuration(b)) {
      return addDurations(a, b, -1) ?? undefined;
    }

    if (isTemporal(a) && isDuration(b)) {
      return addDuration(a, b, -1) ?? undefined;
    }

    if (isTemporal(a) && isTemporal(b)) {
      return subtractTemporals(a, b) ?? undefined;
    }

    return undefined;

  case '*':
    if (numbers) {
      return a * b;
    }

    if (isDuration(a) && typeB === 'number') {
      return multiplyDuration(a, b) ?? undefined;
    }

    if (typeA === 'number' && isDuration(b)) {
      return multiplyDuration(b, a) ?? undefined;
    }

    return undefined;

  case '/':
    if (numbers) {
      return b === 0 ? null : a / b;
    }

    if (isDuration(a) && (typeB === 'number' || isDuration(b))) {
      return divideDuration(a, b) ?? undefined;
    }

    return undefined;

  case '**':
  case '^':
    if (numbers) {
      const result = Math.pow(a, b);

      return Number.isFinite(result) ? result : null;
    }

    return undefined;

  default:
    return undefined;
  }
}

/**
 * Enumerate the elements from start to end (numbers by 1, dates by day).
 *
 * @return {any[]|null}
 */
function enumerate(start, end, startIncluded, endIncluded, allowDescending) {

  if (typeof start === 'number' && typeof end === 'number') {

    if (!Number.isInteger(start) || !Number.isInteger(end)) {
      return null;
    }

    const step = start <= end ? 1 : -1;

    if (step < 0 && !allowDescending) {
      return null;
    }

    const items = [];

    for (let i = start; step > 0 ? i <= end : i >= end; i += step) {
      if ((i === start && !startIncluded) || (i === end && !endIncluded)) {
        continue;
      }

      items.push(i);
    }

    return items;
  }

  if (isDate(start) && isDate(end)) {

    const step = compare(start, end) <= 0 ? 1 : -1;

    if (step < 0 && !allowDescending) {
      return null;
    }

    const items = [];

    const offset = yearOffset(start);

    for (let date = start.value; step > 0 ? compare(createDate(date, offset), end) <= 0 : compare(createDate(date, offset), end) >= 0; date = date.add({ days: step })) {
      const value = createDate(date, offset);

      if ((equals(value, start) && !startIncluded) || (equals(value, end) && !endIncluded)) {
        continue;
      }

      items.push(value);
    }

    return items;
  }

  return null;
}

export { Range };


// range(from) ///////////////

const LITERAL_TYPES = new Set([ 'NumericLiteral', 'StringLiteral', 'null' ]);

function isLiteral(node) {

  if (LITERAL_TYPES.has(node.type)) {
    return true;
  }

  if (node.type !== 'DateTimeLiteral') {
    return false;
  }

  const [ first ] = node.children;

  if (first.type === 'AtLiteral') {
    return true;
  }

  const parameters = node.children.find(isParameters);

  return (
    parameters?.type === 'PositionalParameters' &&
    parameters.children.length === 1 &&
    parameters.children[0].type === 'StringLiteral'
  );
}

/**
 * Parse a range literal (`[1..10)`, `< 5`) with literal endpoints.
 *
 * @param {string} text
 *
 * @return {Range|null}
 */
export function parseRangeLiteral(text) {

  let ast;

  try {
    ast = parse(text.trim(), {}, 'expression');
  } catch {
    return null;
  }

  const test = ast.children[0];

  if (test?.type !== 'SimplePositiveUnaryTest') {
    return null;
  }

  const [ first, second ] = test.children;

  const endpoints = first.type === 'Interval'
    ? [ first.children[1], first.children[3] ]
    : [ second ];

  if (!endpoints.every(endpoint => endpoint && isLiteral(endpoint))) {
    return null;
  }

  const value = new Interpreter({}).evaluate(test, createScope({}));

  if (!isRange(value) || value.operator || (value.start === null && value.end === null)) {
    return null;
  }

  if (value.start !== null && value.end !== null && !(compare(value.start, value.end) <= 0)) {
    return null;
  }

  return value;
}

registerBuiltin('range', createRangeFunction(parseRangeLiteral));
