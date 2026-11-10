/**
 * Range builtins (DMN 1.5, 10.3.4.9 and `range(from)`).
 */

import { builtin, invocationFailure } from './function.js';

import { isRange, toRange } from '../range.js';

import { equals, compare } from '../values.js';


// endpoint comparison ///////////////

const NEGATIVE_INFINITY = { infinity: -1 };
const POSITIVE_INFINITY = { infinity: 1 };

function start(range) {
  return range.startUnbounded ? NEGATIVE_INFINITY : range.start;
}

function end(range) {
  return range.endUnbounded ? POSITIVE_INFINITY : range.end;
}

function cmp(a, b) {

  if (a?.infinity || b?.infinity) {
    const ia = a?.infinity || 0;
    const ib = b?.infinity || 0;

    return ia === ib ? 0 : ia < ib ? -1 : 1;
  }

  return compare(a, b);
}

const lt = (a, b) => bool(cmp(a, b), c => c < 0);
const gt = (a, b) => bool(cmp(a, b), c => c > 0);

function eq(a, b) {

  if (a?.infinity || b?.infinity) {
    return a === b;
  }

  return equals(a, b);
}

function bool(result, test) {
  return result === null ? null : test(result);
}

function and(...values) {

  if (values.includes(false)) {
    return false;
  }

  return values.includes(null) ? null : true;
}

function or(...values) {

  if (values.includes(true)) {
    return true;
  }

  return values.includes(null) ? null : false;
}

function not(value) {
  return value === null ? null : !value;
}

const startIncluded = (range) => range['start included'];
const endIncluded = (range) => range['end included'];


// relations ///////////////

function asRange(value) {
  return isRange(value) ? toRange(value) : null;
}

/**
 * Define a relation between points and / or ranges.
 *
 * @param {string} name
 * @param {object} impls
 * @param {Function} [impls.pp] point, point
 * @param {Function} [impls.pr] point, range
 * @param {Function} [impls.rp] range, point
 * @param {Function} [impls.rr] range, range
 */
function relation(name, impls) {

  const dispatch = (a, b) => {

    if (a === null || a === undefined || b === null || b === undefined) {
      return invocationFailure(`${ name } does not accept null arguments`);
    }

    const ra = asRange(a);
    const rb = asRange(b);

    const impl = ra
      ? (rb ? impls.rr : impls.rp)
      : (rb ? impls.pr : impls.pp);

    if (!impl) {
      return invocationFailure(`${ name } not defined for the given argument types`);
    }

    return impl(ra || a, rb || b);
  };

  const signatures = [];

  if (impls.pp) {
    signatures.push({ parameters: [ 'point1', 'point2' ], impl: dispatch });
  }

  if (impls.pr) {
    signatures.push({ parameters: [ 'point', 'range' ], impl: dispatch });
  }

  if (impls.rp) {
    signatures.push({ parameters: [ 'range', 'point' ], impl: dispatch });
  }

  if (impls.rr) {
    signatures.push({ parameters: [ 'range1', 'range2' ], impl: dispatch });
  }

  return builtin(name, ...signatures);
}

const before = relation('before', {
  pp: (p1, p2) => lt(p1, p2),
  pr: (p, r) => or(lt(p, start(r)), and(eq(p, start(r)), not(startIncluded(r)))),
  rp: (r, p) => or(lt(end(r), p), and(eq(end(r), p), not(endIncluded(r)))),
  rr: (r1, r2) => or(
    lt(end(r1), start(r2)),
    and(or(not(endIncluded(r1)), not(startIncluded(r2))), eq(end(r1), start(r2)))
  )
});

const after = relation('after', {
  pp: (p1, p2) => gt(p1, p2),
  pr: (p, r) => or(gt(p, end(r)), and(eq(p, end(r)), not(endIncluded(r)))),
  rp: (r, p) => or(gt(start(r), p), and(eq(start(r), p), not(startIncluded(r)))),
  rr: (r1, r2) => or(
    gt(start(r1), end(r2)),
    and(or(not(startIncluded(r1)), not(endIncluded(r2))), eq(start(r1), end(r2)))
  )
});

const meets = relation('meets', {
  rr: (r1, r2) => and(endIncluded(r1), startIncluded(r2), eq(end(r1), start(r2)))
});

const metBy = relation('met by', {
  rr: (r1, r2) => and(startIncluded(r1), endIncluded(r2), eq(start(r1), end(r2)))
});

const overlaps = relation('overlaps', {
  rr: (r1, r2) => and(
    or(gt(end(r1), start(r2)), and(eq(end(r1), start(r2)), endIncluded(r1), startIncluded(r2))),
    or(lt(start(r1), end(r2)), and(eq(start(r1), end(r2)), startIncluded(r1), endIncluded(r2)))
  )
});

const overlapsBefore = relation('overlaps before', {
  rr: (r1, r2) => and(
    or(lt(start(r1), start(r2)), and(eq(start(r1), start(r2)), startIncluded(r1), not(startIncluded(r2)))),
    or(gt(end(r1), start(r2)), and(eq(end(r1), start(r2)), endIncluded(r1), startIncluded(r2))),
    or(lt(end(r1), end(r2)), and(eq(end(r1), end(r2)), or(not(endIncluded(r1)), endIncluded(r2))))
  )
});

const overlapsAfter = relation('overlaps after', {
  rr: (r1, r2) => and(
    or(lt(start(r2), start(r1)), and(eq(start(r2), start(r1)), startIncluded(r2), not(startIncluded(r1)))),
    or(gt(end(r2), start(r1)), and(eq(end(r2), start(r1)), endIncluded(r2), startIncluded(r1))),
    or(lt(end(r2), end(r1)), and(eq(end(r2), end(r1)), or(not(endIncluded(r2)), endIncluded(r1))))
  )
});

const finishes = relation('finishes', {
  pr: (p, r) => and(endIncluded(r), eq(end(r), p)),
  rr: (r1, r2) => and(
    endIncluded(r1) === endIncluded(r2),
    eq(end(r1), end(r2)),
    or(gt(start(r1), start(r2)), and(eq(start(r1), start(r2)), or(not(startIncluded(r1)), startIncluded(r2))))
  )
});

const finishedBy = relation('finished by', {
  rp: (r, p) => and(endIncluded(r), eq(end(r), p)),
  rr: (r1, r2) => and(
    endIncluded(r1) === endIncluded(r2),
    eq(end(r1), end(r2)),
    or(lt(start(r1), start(r2)), and(eq(start(r1), start(r2)), or(startIncluded(r1), not(startIncluded(r2)))))
  )
});

function rangeIncludesPoint(r, p) {
  return or(
    and(lt(start(r), p), gt(end(r), p)),
    and(eq(start(r), p), startIncluded(r)),
    and(eq(end(r), p), endIncluded(r))
  );
}

function rangeIncludesRange(r1, r2) {
  return and(
    or(lt(start(r1), start(r2)), and(eq(start(r1), start(r2)), or(startIncluded(r1), not(startIncluded(r2))))),
    or(gt(end(r1), end(r2)), and(eq(end(r1), end(r2)), or(endIncluded(r1), not(endIncluded(r2)))))
  );
}

const includes = relation('includes', {
  rp: rangeIncludesPoint,
  rr: rangeIncludesRange
});

const during = relation('during', {
  pr: (p, r) => rangeIncludesPoint(r, p),
  rr: (r1, r2) => rangeIncludesRange(r2, r1)
});

const starts = relation('starts', {
  pr: (p, r) => and(eq(start(r), p), startIncluded(r)),
  rr: (r1, r2) => and(
    eq(start(r1), start(r2)),
    startIncluded(r1) === startIncluded(r2),
    or(lt(end(r1), end(r2)), and(eq(end(r1), end(r2)), or(not(endIncluded(r1)), endIncluded(r2))))
  )
});

const startedBy = relation('started by', {
  rp: (r, p) => and(eq(start(r), p), startIncluded(r)),
  rr: (r1, r2) => and(
    eq(start(r1), start(r2)),
    startIncluded(r1) === startIncluded(r2),
    or(lt(end(r2), end(r1)), and(eq(end(r2), end(r1)), or(not(endIncluded(r2)), endIncluded(r1))))
  )
});

const coincides = relation('coincides', {
  pp: (p1, p2) => eq(p1, p2),
  rr: (r1, r2) => and(
    eq(start(r1), start(r2)),
    startIncluded(r1) === startIncluded(r2),
    eq(end(r1), end(r2)),
    endIncluded(r1) === endIncluded(r2)
  )
});


/**
 * `range(from)`, parsing a range literal whose endpoints are literals.
 *
 * Receives a function evaluating a FEEL expression to break the cyclic
 * dependency on the interpreter.
 */
export function createRangeFunction(parseRangeLiteral) {
  return builtin('range', {
    parameters: [ 'from' ],
    impl(from) {

      if (typeof from !== 'string') {
        return invocationFailure('range(from) expects a string');
      }

      return parseRangeLiteral(from) ?? invocationFailure(`range(<${ from }>) is not a valid range`);
    }
  });
}

export default {
  'before': before,
  'after': after,
  'meets': meets,
  'met by': metBy,
  'overlaps': overlaps,
  'overlaps before': overlapsBefore,
  'overlaps after': overlapsAfter,
  'finishes': finishes,
  'finished by': finishedBy,
  'includes': includes,
  'during': during,
  'starts': starts,
  'started by': startedBy,
  'coincides': coincides
};
