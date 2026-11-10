/**
 * FEEL ranges.
 *
 * Besides intervals (`[1..10)`) FEEL knows ranges created from unary
 * comparisons (`< 10`, `= 10`, `!= 10`). These are distinct values: `(< 10)`
 * is not equal to `(null..10)`, the latter having an explicit (null) start
 * that makes every membership test `null`.
 */

import { FeelRange } from 'feelin';

/**
 * @typedef {'<'|'<='|'>'|'>='|'='|'!='|null} RangeOperator
 */

export class Range extends FeelRange {

  /**
   * @param {object} props
   * @param {any} props.start
   * @param {any} props.end
   * @param {boolean} props.startIncluded
   * @param {boolean} props.endIncluded
   * @param {boolean} [props.startUnbounded]
   * @param {boolean} [props.endUnbounded]
   * @param {RangeOperator} [props.operator]
   * @param {string|null} [props.valueType]
   */
  constructor(props) {
    super({
      'start included': props.startIncluded,
      start: props.start ?? null,
      end: props.end ?? null,
      'end included': props.endIncluded,
      valueType: props.valueType ?? null
    });

    Object.defineProperty(this, 'startUnbounded', { value: !!props.startUnbounded, enumerable: false });
    Object.defineProperty(this, 'endUnbounded', { value: !!props.endUnbounded, enumerable: false });
    Object.defineProperty(this, 'operator', { value: props.operator || null, enumerable: false });
  }

  toString() {

    if (this.operator && this.operator !== '=' && this.operator !== '!=') {
      const endpoint = this.startUnbounded ? this.end : this.start;

      return `${ this.operator } ${ endpoint }`;
    }

    if (this.operator) {
      return `${ this.operator } ${ this.start }`;
    }

    return `${ this['start included'] ? '[' : '(' }${ this.start }..${ this.end }${ this['end included'] ? ']' : ')' }`;
  }
}

/**
 * Create an interval range.
 */
export function createInterval(start, end, startIncluded, endIncluded, valueType) {
  return new Range({ start, end, startIncluded, endIncluded, valueType });
}

/**
 * Create a range from a unary comparison (`< 10`, `= 10`, ...).
 *
 * @param {RangeOperator} operator
 * @param {any} endpoint
 * @param {string} [valueType]
 */
export function createComparisonRange(operator, endpoint, valueType) {

  switch (operator) {
  case '<':
  case '<=':
    return new Range({
      start: null,
      startIncluded: false,
      startUnbounded: true,
      end: endpoint,
      endIncluded: operator === '<=',
      operator,
      valueType
    });
  case '>':
  case '>=':
    return new Range({
      start: endpoint,
      startIncluded: operator === '>=',
      end: null,
      endIncluded: false,
      endUnbounded: true,
      operator,
      valueType
    });
  case '=':
  case '!=':
    return new Range({
      start: endpoint,
      startIncluded: operator === '=',
      end: endpoint,
      endIncluded: operator === '=',
      operator,
      valueType
    });
  default:
    throw new Error(`unknown range operator <${ operator }>`);
  }
}

export function isRange(value) {
  return value instanceof FeelRange;
}

/**
 * Normalize a (possibly feelin created) range.
 *
 * @param {FeelRange} range
 *
 * @return {Range}
 */
export function toRange(range) {

  if (range instanceof Range) {
    return range;
  }

  return new Range({
    start: range.start,
    end: range.end,
    startIncluded: range['start included'],
    endIncluded: range['end included'],
    startUnbounded: range.start === null,
    endUnbounded: range.end === null,
    valueType: range.valueType
  });
}
