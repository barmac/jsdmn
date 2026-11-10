/**
 * Numeric builtins (DMN 1.5, 10.3.4.5) with exact decimal rounding.
 */

import { builtin, invocationFailure } from './function.js';

const MIN_SCALE = -6111;
const MAX_SCALE = 6176;

const isNumber = (value) => typeof value === 'number' && Number.isFinite(value);

/**
 * Split a number into an exact decimal representation `coefficient * 10^exponent`
 * of its shortest round-tripping string form.
 */
function toDecimal(n) {

  const [ mantissa, exp = '0' ] = String(Math.abs(n)).split('e');
  const [ integer, fraction = '' ] = mantissa.split('.');

  return {
    negative: n < 0 || Object.is(n, -0),
    coefficient: BigInt(integer + fraction),
    exponent: Number(exp) - fraction.length
  };
}

/**
 * Round a number to the given scale (digits after the decimal point).
 *
 * @param {number} n
 * @param {number} scale
 * @param {'UP'|'DOWN'|'CEILING'|'FLOOR'|'HALF_UP'|'HALF_DOWN'|'HALF_EVEN'} mode
 *
 * @return {number}
 */
export function roundToScale(n, scale, mode) {

  if (n === 0) {
    return 0;
  }

  const { negative, coefficient, exponent } = toDecimal(n);

  // no rounding necessary
  if (-exponent <= scale) {
    return n;
  }

  const digits = -exponent - scale;

  // beyond the precision of the coefficient
  if (digits > 400) {
    return roundTiny(negative, mode, scale, n);
  }

  const divisor = 10n ** BigInt(digits);

  let quotient = coefficient / divisor;
  const remainder = coefficient % divisor;

  if (remainder !== 0n) {
    const twice = remainder * 2n;

    const half = twice === divisor ? 0 : twice > divisor ? 1 : -1;

    let increment;

    switch (mode) {
    case 'UP': increment = true; break;
    case 'DOWN': increment = false; break;
    case 'CEILING': increment = !negative; break;
    case 'FLOOR': increment = negative; break;
    case 'HALF_UP': increment = half >= 0; break;
    case 'HALF_DOWN': increment = half > 0; break;
    case 'HALF_EVEN': increment = half > 0 || (half === 0 && quotient % 2n === 1n); break;
    default: throw new Error(`unknown rounding mode <${ mode }>`);
    }

    if (increment) {
      quotient += 1n;
    }
  }

  const result = Number(`${ negative ? '-' : '' }${ quotient }e${ -scale }`);

  if (!Number.isFinite(result)) {
    return n;
  }

  return result === 0 ? 0 : result;
}

function roundTiny(negative, mode, scale, n) {

  const away = mode === 'UP' || (mode === 'CEILING' && !negative) || (mode === 'FLOOR' && negative);

  if (!away) {
    return 0;
  }

  const result = Number(`${ negative ? '-' : '' }1e${ -scale }`);

  return Number.isFinite(result) ? result : n;
}

function isValidScale(scale) {
  return Number.isInteger(scale) && scale >= MIN_SCALE && scale <= MAX_SCALE;
}

function rounding(name, mode) {
  return builtin(name, {
    parameters: [ 'n', 'scale' ],
    impl(n, scale) {

      if (!isNumber(n) || !isValidScale(scale)) {
        return invocationFailure(`${ name }(n, scale) expects a number and an integer scale in [${ MIN_SCALE }..${ MAX_SCALE }]`);
      }

      return roundToScale(n, scale, mode);
    }
  });
}

function floorOrCeiling(name, mode) {
  return builtin(name, {
    parameters: [ 'n', 'scale?' ],
    impl(n, scale) {

      if (scale === undefined) {
        scale = 0;
      }

      if (!isNumber(n) || !isValidScale(scale)) {
        return invocationFailure(`${ name }(n, scale) expects a number and an integer scale`);
      }

      return roundToScale(n, scale, mode);
    }
  });
}

const decimal = builtin('decimal', {
  parameters: [ 'n', 'scale' ],
  impl(n, scale) {

    if (!isNumber(n) || !isNumber(scale)) {
      return invocationFailure('decimal(n, scale) expects numbers');
    }

    scale = Math.trunc(scale);

    if (!isValidScale(scale)) {
      return invocationFailure(`decimal(n, scale) expects a scale in [${ MIN_SCALE }..${ MAX_SCALE }]`);
    }

    return roundToScale(n, scale, 'HALF_EVEN');
  }
});

export default {
  'decimal': decimal,
  'floor': floorOrCeiling('floor', 'FLOOR'),
  'ceiling': floorOrCeiling('ceiling', 'CEILING'),
  'round up': rounding('round up', 'UP'),
  'round down': rounding('round down', 'DOWN'),
  'round half up': rounding('round half up', 'HALF_UP'),
  'round half down': rounding('round half down', 'HALF_DOWN')
};
