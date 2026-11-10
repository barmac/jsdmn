/**
 * Emulation of the Java external functions mapping (DMN 1.5, 10.3.2.13.3)
 * for a curated set of `java.lang` static methods.
 *
 * A Java function is described by a class name and a method signature, e.g.
 * `{ class: "java.lang.Math", method signature: "max(double, double)" }`.
 */

const NUMBER = (value) => typeof value === 'number';
const INTEGER = (value) => Number.isInteger(value);

const PARAMETER_TYPES = {
  'double': NUMBER,
  'float': NUMBER,
  'int': INTEGER,
  'long': INTEGER,
  'short': INTEGER,
  'byte': INTEGER,
  'char': (value) => typeof value === 'string' && [ ...value ].length === 1,
  'boolean': (value) => typeof value === 'boolean',
  'java.lang.Double': NUMBER,
  'java.lang.Float': NUMBER,
  'java.lang.Integer': INTEGER,
  'java.lang.Long': INTEGER,
  'java.lang.Short': INTEGER,
  'java.lang.Byte': INTEGER,
  'java.lang.Character': (value) => typeof value === 'string' && [ ...value ].length === 1,
  'java.lang.Boolean': (value) => typeof value === 'boolean',
  'java.lang.String': (value) => typeof value === 'string',
  'java.lang.Object': () => true,
  'java.math.BigDecimal': NUMBER,
  '[Ljava.lang.Object;': () => true
};

const NUMERIC = [ 'double', 'float', 'int', 'long' ];

function overloads(types, arity, impl) {
  const result = {};

  for (const type of types) {
    result[Array(arity).fill(type).join(',')] = impl;
  }

  return result;
}

function parseNumber(integer) {
  return (str) => {
    const trimmed = str.trim();

    if (integer ? !/^[+-]?\d+$/.test(trimmed) : !/^[+-]?(\d+\.?\d*|\.\d+)([eE][+-]?\d+)?$/.test(trimmed)) {
      throw new Error(`NumberFormatException: ${ str }`);
    }

    return Number(trimmed);
  };
}

const MATH = {
  abs: overloads(NUMERIC, 1, Math.abs),
  acos: { 'double': Math.acos },
  asin: { 'double': Math.asin },
  atan: { 'double': Math.atan },
  atan2: { 'double,double': Math.atan2 },
  cbrt: { 'double': Math.cbrt },
  ceil: { 'double': Math.ceil },
  cos: { 'double': Math.cos },
  cosh: { 'double': Math.cosh },
  exp: { 'double': Math.exp },
  expm1: { 'double': Math.expm1 },
  floor: { 'double': Math.floor },
  floorDiv: overloads([ 'int', 'long' ], 2, (a, b) => Math.floor(a / b)),
  floorMod: overloads([ 'int', 'long' ], 2, (a, b) => ((a % b) + b) % b),
  hypot: { 'double,double': Math.hypot },
  log: { 'double': Math.log },
  log10: { 'double': Math.log10 },
  log1p: { 'double': Math.log1p },
  max: overloads(NUMERIC, 2, Math.max),
  min: overloads(NUMERIC, 2, Math.min),
  pow: { 'double,double': Math.pow },
  rint: { 'double': (a) => Math.round(a) - (Math.abs(a % 1) === 0.5 && Math.round(a) % 2 ? 1 : 0) },
  round: overloads([ 'double', 'float' ], 1, (a) => Math.floor(a + 0.5)),
  signum: overloads([ 'double', 'float' ], 1, Math.sign),
  sin: { 'double': Math.sin },
  sinh: { 'double': Math.sinh },
  sqrt: { 'double': Math.sqrt },
  tan: { 'double': Math.tan },
  tanh: { 'double': Math.tanh },
  toDegrees: { 'double': (a) => a * 180 / Math.PI },
  toRadians: { 'double': (a) => a / 180 * Math.PI }
};

const valueOfNumber = (integer) => ({
  'java.lang.String': parseNumber(integer),
  ...overloads(integer ? [ 'int', 'long', 'short', 'byte' ] : [ 'double', 'float' ], 1, (a) => a)
});

const CLASSES = {
  'java.lang.Math': MATH,
  'java.lang.StrictMath': MATH,
  'java.lang.Integer': {
    valueOf: { 'java.lang.String': parseNumber(true), 'int': (a) => a },
    parseInt: { 'java.lang.String': parseNumber(true) },
    toString: { 'int': String },
    max: { 'int,int': Math.max },
    min: { 'int,int': Math.min },
    sum: { 'int,int': (a, b) => a + b }
  },
  'java.lang.Long': {
    valueOf: { 'java.lang.String': parseNumber(true), 'long': (a) => a },
    parseLong: { 'java.lang.String': parseNumber(true) },
    toString: { 'long': String },
    max: { 'long,long': Math.max },
    min: { 'long,long': Math.min },
    sum: { 'long,long': (a, b) => a + b }
  },
  'java.lang.Short': {
    valueOf: { 'java.lang.String': parseNumber(true), 'short': (a) => a },
    parseShort: { 'java.lang.String': parseNumber(true) }
  },
  'java.lang.Byte': {
    valueOf: { 'java.lang.String': parseNumber(true), 'byte': (a) => a },
    parseByte: { 'java.lang.String': parseNumber(true) }
  },
  'java.lang.Double': {
    valueOf: valueOfNumber(false),
    parseDouble: { 'java.lang.String': parseNumber(false) },
    toString: { 'double': String },
    max: { 'double,double': Math.max },
    min: { 'double,double': Math.min },
    sum: { 'double,double': (a, b) => a + b }
  },
  'java.lang.Float': {
    valueOf: valueOfNumber(false),
    parseFloat: { 'java.lang.String': parseNumber(false) },
    toString: { 'float': String },
    max: { 'float,float': Math.max },
    min: { 'float,float': Math.min },
    sum: { 'float,float': (a, b) => a + b }
  },
  'java.lang.Boolean': {
    valueOf: { 'java.lang.String': (str) => str.toLowerCase() === 'true', 'boolean': (a) => a },
    parseBoolean: { 'java.lang.String': (str) => str.toLowerCase() === 'true' }
  },
  'java.lang.String': {
    valueOf: {
      'char': String,
      'int': String,
      'long': String,
      'double': formatDouble,
      'float': formatDouble,
      'boolean': String,
      'java.lang.Object': String
    },
    format: {
      'java.lang.String,[Ljava.lang.Object;': (format, ...args) => javaFormat(format, args)
    }
  }
};

/**
 * Parse a Java method signature, e.g. `max(double, double)`.
 *
 * @param {string} signature
 *
 * @return { { name: string, parameterTypes: string[] } | null }
 */
export function parseMethodSignature(signature) {

  const match = /^\s*([\w$]+)\s*\(([^)]*)\)\s*$/.exec(signature || '');

  if (!match) {
    return null;
  }

  const [ , name, parameters ] = match;

  return {
    name,
    parameterTypes: parameters.trim() ? parameters.split(',').map(p => p.trim()) : []
  };
}

/**
 * Resolve a Java static method to a JavaScript implementation.
 *
 * @param {string} className
 * @param {string} methodSignature
 *
 * @return {((...args: any[]) => any) | null}
 */
export function resolveJavaMethod(className, methodSignature) {

  const methods = CLASSES[className];

  if (!methods) {
    return null;
  }

  const signature = parseMethodSignature(methodSignature);

  if (!signature || !Object.prototype.hasOwnProperty.call(methods, signature.name)) {
    return null;
  }

  const { name, parameterTypes } = signature;

  const impl = methods[name][parameterTypes.join(',')];

  if (!impl) {
    return null;
  }

  const varargs = parameterTypes[parameterTypes.length - 1]?.startsWith('[');

  return (...args) => {

    if (varargs) {
      const fixed = args.slice(0, parameterTypes.length - 1);
      const rest = args.slice(parameterTypes.length - 1).flat();

      args = [ ...fixed, ...rest ];
    } else if (args.length !== parameterTypes.length) {
      throw new Error(`expected ${ parameterTypes.length } argument(s)`);
    }

    parameterTypes.forEach((type, idx) => {

      if (type.startsWith('[')) {
        return;
      }

      const check = PARAMETER_TYPES[type];

      if (!check || !check(args[idx])) {
        throw new Error(`argument ${ idx + 1 } is not a valid <${ type }>`);
      }
    });

    return impl(...args);
  };
}

function formatDouble(value) {
  return Number.isInteger(value) ? `${ value }.0` : String(value);
}

/**
 * A minimal `java.lang.String#format` supporting `%s`, `%d`, `%f`, `%n` and `%%`.
 */
function javaFormat(format, args) {

  let idx = 0;

  return format.replace(/%(\d+\$)?([-#+ 0,(]*)?(\d+)?(\.\d+)?([sdfnbx%])/g, (match, position, flags, width, precision, conversion) => {

    if (conversion === '%') {
      return '%';
    }

    if (conversion === 'n') {
      return '\n';
    }

    const arg = position ? args[parseInt(position, 10) - 1] : args[idx++];

    let result;

    switch (conversion) {
    case 'd':
      result = String(Math.trunc(arg));
      break;
    case 'f':
      result = Number(arg).toFixed(precision ? parseInt(precision.slice(1), 10) : 6);
      break;
    case 'x':
      result = Number(arg).toString(16);
      break;
    case 'b':
      result = String(arg !== null && arg !== undefined && arg !== false);
      break;
    default:
      result = arg === null || arg === undefined ? 'null' : String(arg);
    }

    if (width) {
      const padded = parseInt(width, 10);

      result = flags && flags.includes('-') ? result.padEnd(padded) : result.padStart(padded);
    }

    return result;
  });
}
