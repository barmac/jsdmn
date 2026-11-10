/**
 * The DMN / FEEL type system: type references, item definitions,
 * conformance checks and the implicit conversions applied when values are
 * bound to typed variables.
 */

import {
  getFeelType,
  evaluateUnaryTests,
  describe,
  FeelDate,
  FeelRange
} from './feel.js';

import { dateToDateTime } from './feel/temporal.js';


/**
 * @typedef {object} Type
 * @property {string} name
 * @property {(value: any) => boolean} isInstance strict instance check (`instance of`)
 */

class BaseType {

  constructor(name) {
    this.name = name;
  }

  /**
   * Whether value is an instance of this type (`instance of`); `null` is
   * an instance of no type but `Null`. Type constraints (allowed values)
   * are not considered.
   */
  isInstance(value) {
    return value !== null && value !== undefined && this.test(value, true);
  }

  /**
   * Whether value conforms to this type, accepting `null`.
   */
  conforms(value) {
    return this.accepts(value, false);
  }

  /**
   * @param {any} value
   * @param {boolean} strict whether to perform an `instance of` check
   */
  accepts(value, strict) {
    return value === null || value === undefined || this.test(value, strict);
  }

  /**
   * @param {any} value (non-null)
   * @param {boolean} strict
   *
   * @return {boolean}
   */
  test() {
    return false;
  }

  toString() {
    return this.name;
  }
}

class AnyType extends BaseType {

  constructor() {
    super('Any');
  }

  test() {
    return true;
  }
}

class NullType extends BaseType {

  constructor() {
    super('Null');
  }

  isInstance(value) {
    return value === null || value === undefined;
  }

  test() {
    return false;
  }
}

class SimpleType extends BaseType {

  /**
   * @param {string} name
   * @param {(value: any) => boolean} test
   */
  constructor(name, test) {
    super(name);

    this.test = test;
  }
}

export class ListType extends BaseType {

  constructor(elementType) {
    super(`list<${ elementType }>`);

    this.elementType = elementType;
  }

  test(value, strict) {
    return Array.isArray(value) && value.every(element => this.elementType.accepts(element, strict));
  }
}

export class ContextType extends BaseType {

  /**
   * @param {Array<[ string, Type ]>} entries
   * @param {string} [name]
   */
  constructor(entries, name) {
    super(name || `context<${ entries.map(([ key, type ]) => `${ key }: ${ type }`).join(', ') }>`);

    this.entries = entries;
  }

  test(value, strict) {

    if (getFeelType(value) !== 'context') {
      return false;
    }

    return this.entries.every(([ key, type ]) => {
      return Object.prototype.hasOwnProperty.call(value, key) && type.accepts(value[key], strict);
    });
  }
}

export class FunctionType extends BaseType {

  /**
   * @param {Type[]|null} parameterTypes
   * @param {Type} returnType
   */
  constructor(parameterTypes, returnType) {
    super(
      parameterTypes
        ? `function<${ parameterTypes.join(', ') }>->${ returnType }`
        : 'function'
    );

    this.parameterTypes = parameterTypes;
    this.returnType = returnType;
  }

  test(value, strict) {

    if (getFeelType(value) !== 'function') {
      return false;
    }

    if (!this.parameterTypes) {
      return true;
    }

    const parameterNames = value.parameterNames;

    if (Array.isArray(parameterNames) && !parameterNames.some(name => name.startsWith('...'))) {
      if (parameterNames.length !== this.parameterTypes.length) {
        return false;
      }
    }

    const actualType = unwrap(value.type);

    if (!(actualType instanceof FunctionType) || !actualType.parameterTypes) {
      return true;
    }

    // parameters are contravariant, the return type is covariant; when
    // binding values, an undeclared (Any) return type is accepted
    const returnTypeConforms = (!strict && unwrap(actualType.returnType) === ANY) ||
      isSubtype(actualType.returnType, this.returnType);

    return (
      this.parameterTypes.every((type, idx) => isSubtype(type, actualType.parameterTypes[idx])) &&
      returnTypeConforms
    );
  }
}

export class RangeType extends BaseType {

  constructor(elementType) {
    super(`range<${ elementType }>`);

    this.elementType = elementType;
  }

  test(value, strict) {

    if (!(value instanceof FeelRange)) {
      return false;
    }

    return [ value.start, value.end ].every(bound => this.elementType.accepts(bound, strict));
  }
}

/**
 * A type restricted by unary tests (allowed values / type constraint).
 */
export class ConstrainedType extends BaseType {

  /**
   * @param {Type} baseType
   * @param {string} constraint FEEL unary tests
   */
  constructor(baseType, constraint) {
    super(baseType.name);

    this.baseType = baseType;
    this.constraint = constraint;
  }

  test(value, strict) {

    // constraints do not participate in `instance of` checks
    if (strict) {
      return this.baseType.test(value, strict);
    }

    return this.baseType.test(value, strict) && evaluateUnaryTests(this.constraint, value, {}) === true;
  }

  get elementType() {
    return this.baseType.elementType;
  }
}

/**
 * A named type (item definition), resolved lazily to support
 * recursive definitions.
 */
export class NamedType extends BaseType {

  /**
   * @param {string} name
   * @param {() => Type} resolve
   */
  constructor(name, resolve) {
    super(name);

    this._resolve = resolve;
    this._type = null;
  }

  get type() {

    if (!this._type) {
      this._type = this._resolve();
    }

    return this._type;
  }

  test(value, strict) {
    return this.type.test(value, strict);
  }

  accepts(value, strict) {
    return this.type.accepts(value, strict);
  }

  isInstance(value) {
    return this.type.isInstance(value);
  }
}


// builtin types ///////////////

const isType = (name) => (value) => getFeelType(value) === name;

export const ANY = new AnyType();

export const NULL = new NullType();

const DURATION = new SimpleType('duration', value => [
  'days and time duration',
  'years and months duration'
].includes(getFeelType(value)));

export const BUILTIN_TYPES = {
  'Any': ANY,
  'Null': NULL,
  'number': new SimpleType('number', isType('number')),
  'string': new SimpleType('string', isType('string')),
  'boolean': new SimpleType('boolean', isType('boolean')),
  'date': new SimpleType('date', isType('date')),
  'time': new SimpleType('time', isType('time')),
  'date and time': new SimpleType('date and time', isType('date and time')),
  'days and time duration': new SimpleType('days and time duration', isType('days and time duration')),
  'years and months duration': new SimpleType('years and months duration', isType('years and months duration')),
  'duration': DURATION,
  'context': new SimpleType('context', isType('context')),
  'list': new SimpleType('list', isType('list')),
  'function': new FunctionType(null, ANY),
  'range': new SimpleType('range', isType('range'))
};

/**
 * Alternative spellings of builtin type names (XML Schema names
 * used by DMN 1.1 / older modelers).
 */
const TYPE_ALIASES = {
  'dateTime': 'date and time',
  'dayTimeDuration': 'days and time duration',
  'yearMonthDuration': 'years and months duration',
  'decimal': 'number',
  'double': 'number',
  'integer': 'number',
  'int': 'number',
  'long': 'number',
  'float': 'number'
};

const TYPE_PREFIXES = /^(feel|xsd|xs):/;

/**
 * Resolve a builtin type name.
 *
 * @param {string} name
 *
 * @return {Type|null}
 */
export function getBuiltinType(name) {

  name = normalizeTypeName(name).replace(TYPE_PREFIXES, '');

  return BUILTIN_TYPES[name] || BUILTIN_TYPES[TYPE_ALIASES[name]] || null;
}

export function normalizeTypeName(name) {
  return name.replace(/\s+/g, ' ').trim();
}


// type references ///////////////

/**
 * Parse a type reference, e.g. `number`, `tPerson`, `list<string>`,
 * `context<a: number>` or `function<number>->string`.
 *
 * @param {string} typeRef
 * @param {(name: string) => Type|null} resolveName
 *
 * @return {Type|null}
 */
export function parseTypeRef(typeRef, resolveName) {

  const tokens = tokenize(typeRef);

  let position = 0;

  const peek = () => tokens[position];
  const next = () => tokens[position++];

  const expect = (token) => {
    if (next() !== token) {
      throw new Error(`expected <${ token }> in type <${ typeRef }>`);
    }
  };

  function parseType() {
    const name = next();

    if (!name || isPunctuation(name)) {
      throw new Error(`unexpected <${ name }> in type <${ typeRef }>`);
    }

    if (peek() === '<') {

      if (name === 'list') {
        expect('<');
        const elementType = parseType();
        expect('>');

        return new ListType(elementType);
      }

      if (name === 'range') {
        expect('<');
        const elementType = parseType();
        expect('>');

        return new RangeType(elementType);
      }

      if (name === 'context') {
        expect('<');

        const entries = [];

        while (peek() !== '>') {
          const key = next();
          expect(':');
          entries.push([ key, parseType() ]);

          if (peek() === ',') {
            next();
          }
        }

        expect('>');

        return new ContextType(entries);
      }

      if (name === 'function') {
        expect('<');

        const parameterTypes = [];

        while (peek() !== '>') {
          parameterTypes.push(parseType());

          if (peek() === ',') {
            next();
          }
        }

        expect('>');
        expect('->');

        return new FunctionType(parameterTypes, parseType());
      }

      throw new Error(`unknown parametrized type <${ name }> in <${ typeRef }>`);
    }

    const type = getBuiltinType(name) || resolveName(name);

    if (!type) {
      throw new Error(`unknown type <${ name }>`);
    }

    return type;
  }

  const type = parseType();

  if (position !== tokens.length) {
    throw new Error(`unexpected <${ peek() }> in type <${ typeRef }>`);
  }

  return type;
}

function isPunctuation(token) {
  return [ '<', '>', ',', ':', '->' ].includes(token);
}

function tokenize(text) {
  const tokens = [];

  let current = '';

  const flush = () => {
    const token = normalizeTypeName(current);

    if (token) {
      tokens.push(token);
    }

    current = '';
  };

  for (let idx = 0; idx < text.length; idx++) {
    const char = text[idx];

    if (char === '-' && text[idx + 1] === '>') {
      flush();
      tokens.push('->');
      idx++;
      continue;
    }

    if ([ '<', '>', ',', ':' ].includes(char)) {
      flush();
      tokens.push(char);
      continue;
    }

    current += char;
  }

  flush();

  return tokens;
}


// subtyping ///////////////

/**
 * Whether type `a` conforms to type `b` (structurally).
 */
export function isSubtype(a, b) {

  a = unwrap(a);
  b = unwrap(b);

  if (a === b || b === ANY) {
    return true;
  }

  if (a instanceof ListType && b instanceof ListType) {
    return isSubtype(a.elementType, b.elementType);
  }

  if (a instanceof ContextType && b instanceof ContextType) {
    return b.entries.every(([ key, type ]) => {
      const entry = a.entries.find(([ k ]) => k === key);

      return entry && isSubtype(entry[1], type);
    });
  }

  if (a instanceof FunctionType && b instanceof FunctionType) {

    if (!b.parameterTypes) {
      return true;
    }

    if (!a.parameterTypes || a.parameterTypes.length !== b.parameterTypes.length) {
      return false;
    }

    return (
      b.parameterTypes.every((type, idx) => isSubtype(type, a.parameterTypes[idx])) &&
      isSubtype(a.returnType, b.returnType)
    );
  }

  if (a instanceof RangeType && b instanceof RangeType) {
    return isSubtype(a.elementType, b.elementType);
  }

  if (a === BUILTIN_TYPES['days and time duration'] || a === BUILTIN_TYPES['years and months duration']) {
    return b === DURATION;
  }

  return a.name === b.name && !(a instanceof ConstrainedType) && !(b instanceof ConstrainedType);
}

export function unwrap(type) {

  while (type instanceof NamedType) {
    type = type.type;
  }

  return type;
}

/**
 * The return type of a function type, or the type itself.
 */
export function getReturnType(type) {
  const resolved = unwrap(type);

  return resolved instanceof FunctionType && resolved.parameterTypes ? resolved.returnType : type;
}


// conversions ///////////////

/**
 * Coerce a value to the given type, applying the implicit
 * conversions defined by DMN 1.5 (10.3.2.9.4):
 *
 *   - to singleton list
 *   - from singleton list
 *   - from date to date and time
 *
 * Returns `{ value }` on success and `{ error }` if the value does
 * not conform to the type.
 *
 * @param {any} value
 * @param {Type|null} type
 *
 * @return { { value: any, error?: string } }
 */
export function coerce(value, type) {

  if (value === undefined) {
    value = null;
  }

  if (!type || type === ANY || value === null) {
    return { value };
  }

  if (type.conforms(value)) {
    return { value };
  }

  const base = unwrap(type);
  const listType = base instanceof ConstrainedType ? unwrap(base.baseType) : base;

  // to singleton list
  if (listType instanceof ListType && !Array.isArray(value)) {

    const element = coerce(value, listType.elementType);

    if (!element.error && type.conforms([ element.value ])) {
      return { value: [ element.value ] };
    }
  }

  // from singleton list (one level only)
  if (Array.isArray(value) && value.length === 1) {

    const [ element ] = value;

    if (type.conforms(element)) {
      return { value: element };
    }

    if (element instanceof FeelDate && type.conforms(toDateTime(element))) {
      return { value: toDateTime(element) };
    }
  }

  // from date to date and time
  if (value instanceof FeelDate && type.conforms(toDateTime(value))) {
    return { value: toDateTime(value) };
  }

  return {
    value: null,
    error: `value <${ describe(value) }> does not conform to type <${ type }>`
  };
}

function toDateTime(date) {
  return dateToDateTime(date);
}
