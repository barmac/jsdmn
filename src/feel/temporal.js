/**
 * FEEL temporal values: parsing, formatting and arithmetic.
 *
 * Values are represented by feelin's temporal classes (backed by
 * `Temporal`) so that they interoperate with feelin's builtin functions.
 * This module constructs them with canonical FEEL formatting (e.g.
 * negative years as `-2021-01-01`) and implements the DMN 1.5 semantics
 * of temporal arithmetic and comparison.
 *
 * Time zones are given as `null` (local), `UTC`, a fixed offset
 * (`+HH:MM`, `+HH:MM:SS`) or an IANA zone name.
 */

import { Temporal } from 'temporal-polyfill';

import {
  FeelDate,
  FeelTime,
  FeelDateTime,
  FeelDuration
} from 'feelin';

export {
  Temporal,
  FeelDate,
  FeelTime,
  FeelDateTime,
  FeelDuration
};

const NS_PER_SECOND = 1000000000n;
const NS_PER_DAY = 86400n * NS_PER_SECOND;

const REFERENCE_DATE = Temporal.PlainDate.from('1970-01-01');

const MAX_OFFSET_SECONDS = 18 * 3600;

/**
 * FEEL supports years in [-999,999,999..999,999,999], Temporal only
 * about +-271,821. Dates outside of the Temporal range are represented by
 * an in-range proxy date plus a year offset that is a multiple of 400:
 * the Gregorian calendar repeats every 400 years (146,097 days), so the
 * proxy has the same leap years, weekdays and days of year.
 */
const MAX_TEMPORAL_YEAR = 275000;
const MIN_TEMPORAL_YEAR = -271000;

const MAX_FEEL_YEAR = 999999999;

const CYCLE_YEARS = 400;
const CYCLE_DAYS = 146097n;

/**
 * Split a year into a Temporal compatible proxy year and an offset.
 */
function splitYear(year) {

  if (year >= MIN_TEMPORAL_YEAR && year <= MAX_TEMPORAL_YEAR) {
    return { year, offset: 0 };
  }

  const offset = Math.floor((year - 2000) / CYCLE_YEARS) * CYCLE_YEARS;

  return { year: year - offset, offset };
}

/**
 * Year offset of a (proxied) date or date time.
 */
export function yearOffset(value) {
  return value?.yearOffset || 0;
}

function offsetNanoseconds(offset) {
  return offset ? BigInt(offset / CYCLE_YEARS) * CYCLE_DAYS * NS_PER_DAY : 0n;
}

/**
 * A date outside of the Temporal range.
 */
class ExtendedDate extends FeelDate {

  constructor(value, offset) {
    super(value);

    Object.defineProperty(this, 'yearOffset', { value: offset, enumerable: false });
  }

  get year() {
    return this.value.year + this.yearOffset;
  }
}

/**
 * A date time outside of the Temporal range.
 */
class ExtendedDateTime extends FeelDateTime {

  constructor(value, zone, offset) {
    super(value, zone);

    Object.defineProperty(this, 'yearOffset', { value: offset, enumerable: false });
  }

  get year() {
    return this.value.year + this.yearOffset;
  }
}

const OFFSET_PATTERN = /^([+-])(\d{2}):(\d{2})(?::(\d{2}))?$/;


// type guards ///////////////

export function isDate(value) {
  return value instanceof FeelDate;
}

export function isTime(value) {
  return value instanceof FeelTime;
}

export function isDateTime(value) {
  return value instanceof FeelDateTime;
}

export function isDuration(value) {
  return value instanceof FeelDuration;
}

export function isYearsMonthsDuration(value) {
  return isDuration(value) && value.yearsMonths;
}

export function isDaysTimeDuration(value) {
  return isDuration(value) && !value.yearsMonths;
}

export function isTemporal(value) {
  return isDate(value) || isTime(value) || isDateTime(value);
}


// zones ///////////////

/**
 * Offset of a fixed offset zone in seconds; `null` for named zones.
 *
 * @param {string|null} zone
 *
 * @return {number|null}
 */
function fixedOffsetSeconds(zone) {

  if (zone === 'UTC') {
    return 0;
  }

  const match = OFFSET_PATTERN.exec(zone || '');

  if (!match) {
    return null;
  }

  const [ , sign, hours, minutes, seconds = '0' ] = match;

  return (sign === '-' ? -1 : 1) * (Number(hours) * 3600 + Number(minutes) * 60 + Number(seconds));
}

const validZones = new Map();

function isValidZone(zone) {

  if (!validZones.has(zone)) {
    let valid;

    try {
      REFERENCE_DATE.toZonedDateTime(zone);
      valid = true;
    } catch {
      valid = false;
    }

    validZones.set(zone, valid);
  }

  return validZones.get(zone);
}

/**
 * Normalize a zone designator (`Z`, `+01:00`, `@Europe/Paris`) to a zone
 * as used by the temporal classes; returns `undefined` if invalid.
 */
function parseZone(designator) {

  if (!designator) {
    return null;
  }

  if (designator === 'Z' || designator === 'z') {
    return 'UTC';
  }

  if (designator.startsWith('@')) {
    const name = designator.slice(1);

    return isValidZone(name) ? name : undefined;
  }

  const seconds = fixedOffsetSeconds(designator);

  if (seconds === null || Math.abs(seconds) > MAX_OFFSET_SECONDS) {
    return undefined;
  }

  // normalize -00:00 / +00:00:00 forms
  return formatOffset(seconds);
}

function formatOffset(seconds) {
  const sign = seconds < 0 ? '-' : '+';
  const abs = Math.abs(seconds);

  const hours = Math.floor(abs / 3600);
  const minutes = Math.floor((abs % 3600) / 60);
  const secs = abs % 60;

  return `${ sign }${ pad(hours, 2) }:${ pad(minutes, 2) }${ secs ? `:${ pad(secs, 2) }` : '' }`;
}

function zoneSuffix(zone) {

  if (zone === null || zone === undefined) {
    return '';
  }

  if (zone === 'UTC') {
    return 'Z';
  }

  if (OFFSET_PATTERN.test(zone)) {
    return zone;
  }

  return `@${ zone }`;
}

/**
 * Offset (in seconds) of a zone at the given wall clock date time.
 */
function zoneOffsetSeconds(zone, plainDateTime) {

  const fixed = fixedOffsetSeconds(zone);

  if (fixed !== null) {
    return fixed;
  }

  const zoned = plainDateTime.toZonedDateTime(zone, { disambiguation: 'compatible' });

  return zoned.offsetNanoseconds / 1e9;
}


// formatting ///////////////

function pad(value, length) {
  return String(value).padStart(length, '0');
}

function formatYear(year) {
  return `${ year < 0 ? '-' : '' }${ pad(Math.abs(year), 4) }`;
}

export function formatPlainDate(date, offset = 0) {
  return `${ formatYear(date.year + offset) }-${ pad(date.month, 2) }-${ pad(date.day, 2) }`;
}

export function formatPlainTime(time) {
  const fraction = time.millisecond * 1e6 + time.microsecond * 1e3 + time.nanosecond;

  return `${ pad(time.hour, 2) }:${ pad(time.minute, 2) }:${ pad(time.second, 2) }${
    fraction ? `.${ pad(fraction, 9).replace(/0+$/, '') }` : ''
  }`;
}


// construction ///////////////

/**
 * @param {Temporal.PlainDate} plainDate
 *
 * @return {FeelDate}
 */
export function createDate(plainDate, offset = 0) {
  const date = offset ? new ExtendedDate(plainDate, offset) : new FeelDate(plainDate);

  date.iso = formatPlainDate(plainDate, offset);

  return date;
}

/**
 * @param {Temporal.PlainTime} plainTime
 * @param {string|null} [zone]
 *
 * @return {FeelTime}
 */
export function createTime(plainTime, zone = null) {
  const time = new FeelTime(plainTime, zone);

  time.iso = formatPlainTime(plainTime) + zoneSuffix(zone);

  return time;
}

/**
 * @param {Temporal.PlainDateTime} plainDateTime
 * @param {string|null} [zone]
 *
 * @return {FeelDateTime}
 */
export function createDateTime(plainDateTime, zone = null, offset = 0) {
  const dateTime = offset ? new ExtendedDateTime(plainDateTime, zone, offset) : new FeelDateTime(plainDateTime, zone);

  dateTime.iso = `${ formatPlainDate(plainDateTime, offset) }T${ formatPlainTime(plainDateTime) }${ zoneSuffix(zone) }`;

  return dateTime;
}

/**
 * Create a years and months duration.
 *
 * @param {number} totalMonths
 *
 * @return {FeelDuration}
 */
export function createYearsMonthsDuration(totalMonths) {

  totalMonths = Math.trunc(totalMonths) || 0;

  const years = Math.trunc(totalMonths / 12);
  const months = totalMonths - years * 12;

  return createDuration(Temporal.Duration.from({ years, months }), true);
}

/**
 * Create a duration from an already normalized `Temporal.Duration`.
 *
 * Bypasses feelin's constructor which re-normalizes relative to a
 * reference date and thereby fails for durations exceeding the
 * supported date range.
 */
function createDuration(value, yearsMonths) {

  const duration = Object.create(FeelDuration.prototype);

  Object.defineProperty(duration, 'value', { value, enumerable: false, writable: true });

  duration.yearsMonths = yearsMonths;
  duration.iso = value.sign === 0 ? (yearsMonths ? 'P0M' : 'PT0S') : value.toString();

  return duration;
}

/**
 * Create a days and time duration.
 *
 * @param {bigint} totalNanoseconds
 *
 * @return {FeelDuration}
 */
export function createDaysTimeDuration(totalNanoseconds) {

  const sign = totalNanoseconds < 0n ? -1n : 1n;

  let rest = totalNanoseconds * sign;

  const take = (unit) => {
    const value = rest / unit;

    rest = rest % unit;

    return Number(value * sign) || 0;
  };

  const days = take(NS_PER_DAY);
  const hours = take(3600n * NS_PER_SECOND);
  const minutes = take(60n * NS_PER_SECOND);
  const seconds = take(NS_PER_SECOND);
  const milliseconds = take(1000000n);
  const microseconds = take(1000n);
  const nanoseconds = take(1n);

  return createDuration(Temporal.Duration.from({
    days,
    hours,
    minutes,
    seconds,
    milliseconds,
    microseconds,
    nanoseconds
  }), false);
}

/**
 * Total months of a years and months duration.
 */
export function totalMonths(duration) {
  const { years, months } = duration.value;

  return years * 12 + months;
}

/**
 * Total nanoseconds of a days and time duration.
 *
 * @return {bigint}
 */
export function totalNanoseconds(duration) {
  const d = duration.value;

  return (
    BigInt(d.days) * NS_PER_DAY +
    BigInt(d.hours) * 3600n * NS_PER_SECOND +
    BigInt(d.minutes) * 60n * NS_PER_SECOND +
    BigInt(d.seconds) * NS_PER_SECOND +
    BigInt(d.milliseconds) * 1000000n +
    BigInt(d.microseconds) * 1000n +
    BigInt(d.nanoseconds)
  );
}

/**
 * Total seconds of a days and time duration.
 */
export function totalSeconds(duration) {
  return Number(totalNanoseconds(duration)) / 1e9;
}

/**
 * Create a date from its components.
 *
 * @return {FeelDate|null}
 */
export function dateFromParts(year, month, day) {

  if (![ year, month, day ].every(Number.isInteger) || Math.abs(year) > MAX_FEEL_YEAR) {
    return null;
  }

  const { year: proxyYear, offset } = splitYear(year);

  try {
    return createDate(Temporal.PlainDate.from({ year: proxyYear, month, day }, { overflow: 'reject' }), offset);
  } catch {
    return null;
  }
}

/**
 * Create a time from its components.
 *
 * @param {number} hour
 * @param {number} minute
 * @param {number} second may be fractional
 * @param {string|null} [zone]
 *
 * @return {FeelTime|null}
 */
export function timeFromParts(hour, minute, second, zone = null) {

  if (![ hour, minute ].every(Number.isInteger) || typeof second !== 'number' || !Number.isFinite(second)) {
    return null;
  }

  const plainTime = plainTimeFromParts(hour, minute, second);

  return plainTime ? createTime(plainTime, zone) : null;
}

function plainTimeFromParts(hour, minute, second, fraction = null) {

  if (second < 0 || second >= 60 || minute < 0 || minute > 59 || hour < 0 || hour > 23) {
    return null;
  }

  const wholeSeconds = Math.floor(second);

  const nanos = fraction !== null
    ? fraction
    : Math.round((second - wholeSeconds) * 1e9);

  try {
    return Temporal.PlainTime.from({
      hour,
      minute,
      second: wholeSeconds,
      millisecond: Math.floor(nanos / 1e6),
      microsecond: Math.floor(nanos / 1e3) % 1000,
      nanosecond: nanos % 1000
    }, { overflow: 'reject' });
  } catch {
    return null;
  }
}


// parsing ///////////////

const DATE_PATTERN = /^(-?)(\d{4,9})-(\d{2})-(\d{2})$/;

const TIME_PATTERN = /^(\d{2}):(\d{2}):(\d{2})(?:\.(\d+))?(Z|z|[+-]\d{2}:\d{2}(?::\d{2})?|@[^@\s]+)?$/;

const DURATION_PATTERN = /^(-)?P(?=\d|T\d|T\.\d)(?:(\d+)Y)?(?:(\d+)M)?(?:(\d+)D)?(?:T(?=\d|\.\d)(?:(\d+)H)?(?:(\d+)M)?(?:(\d+\.?\d*|\.\d+)S)?)?$/;

/**
 * @param {string} str
 *
 * @return {FeelDate|null}
 */
export function parseDate(str) {

  const match = DATE_PATTERN.exec(str);

  if (!match) {
    return null;
  }

  const [ , sign, year, month, day ] = match;

  if (year.length > 4 && year.startsWith('0')) {
    return null;
  }

  return dateFromParts((sign ? -1 : 1) * Number(year), Number(month), Number(day));
}

/**
 * Parse a time, returning the wall clock time, its zone and
 * whether it denotes end of day (`24:00:00`).
 */
function parseTimeParts(str) {

  const match = TIME_PATTERN.exec(str);

  if (!match) {
    return null;
  }

  const [ , hours, minutes, seconds, fraction, designator ] = match;

  const zone = parseZone(designator);

  if (zone === undefined) {
    return null;
  }

  const hour = Number(hours);
  const minute = Number(minutes);
  const second = Number(seconds);
  const nanos = fraction ? Number(fraction.slice(0, 9).padEnd(9, '0')) : 0;

  // end of day
  if (hour === 24 && minute === 0 && second === 0 && nanos === 0) {
    return {
      time: Temporal.PlainTime.from('00:00:00'),
      zone,
      endOfDay: true
    };
  }

  const time = plainTimeFromParts(hour, minute, second, nanos);

  if (!time) {
    return null;
  }

  return { time, zone, endOfDay: false };
}

/**
 * @param {string} str
 *
 * @return {FeelTime|null}
 */
export function parseTime(str) {

  const parts = parseTimeParts(str);

  return parts ? createTime(parts.time, parts.zone) : null;
}

/**
 * @param {string} str
 *
 * @return {FeelDateTime|null}
 */
export function parseDateTime(str) {

  const separatorIdx = str.indexOf('T');

  if (separatorIdx === -1) {
    const date = parseDate(str);

    return date ? createDateTime(date.value.toPlainDateTime(), null, yearOffset(date)) : null;
  }

  const date = parseDate(str.slice(0, separatorIdx));
  const time = parseTimeParts(str.slice(separatorIdx + 1));

  if (!date || !time) {
    return null;
  }

  let plainDateTime = date.value.toPlainDateTime(time.time);

  if (time.endOfDay) {
    plainDateTime = plainDateTime.add({ days: 1 });
  }

  return createDateTime(plainDateTime, time.zone, yearOffset(date));
}

/**
 * @param {string} str
 *
 * @return {FeelDuration|null}
 */
export function parseDuration(str) {

  const match = DURATION_PATTERN.exec(str);

  if (!match) {
    return null;
  }

  const [ , negative, years, months, days, hours, minutes, seconds ] = match;

  const sign = negative ? -1 : 1;

  const isYearsMonths = years !== undefined || months !== undefined;
  const isDaysTime = days !== undefined || hours !== undefined || minutes !== undefined || seconds !== undefined;

  if (isYearsMonths && isDaysTime) {
    return null;
  }

  if (isYearsMonths) {
    return createYearsMonthsDuration(sign * (Number(years || 0) * 12 + Number(months || 0)));
  }

  const [ wholeSeconds = '0', fraction = '' ] = (seconds || '0').split('.');

  const total =
    BigInt(days || 0) * NS_PER_DAY +
    BigInt(hours || 0) * 3600n * NS_PER_SECOND +
    BigInt(minutes || 0) * 60n * NS_PER_SECOND +
    BigInt(wholeSeconds || 0) * NS_PER_SECOND +
    BigInt((fraction || '').slice(0, 9).padEnd(9, '0') || 0);

  return createDaysTimeDuration(BigInt(sign) * total);
}

/**
 * Parse the value of an `@"..."` literal.
 *
 * @param {string} str
 *
 * @return {FeelDate|FeelTime|FeelDateTime|FeelDuration|null}
 */
export function parseTemporal(str) {

  if (/^-?P/.test(str)) {
    return parseDuration(str);
  }

  if (str.includes('T')) {
    return parseDateTime(str);
  }

  if (DATE_PATTERN.test(str)) {
    return parseDate(str);
  }

  return parseTime(str);
}


// conversion ///////////////

/**
 * Convert a date to a date and time at midnight UTC.
 */
export function dateToDateTime(date, zone = 'UTC') {
  return createDateTime(date.value.toPlainDateTime(), zone, yearOffset(date));
}

/**
 * Epoch nanoseconds of a date time (local values are interpreted as UTC).
 *
 * @return {bigint}
 */
export function epochNanoseconds(value) {

  if (isDate(value)) {
    return value.value.toPlainDateTime().toZonedDateTime('UTC').epochNanoseconds + offsetNanoseconds(yearOffset(value));
  }

  const plainDateTime = isTime(value)
    ? REFERENCE_DATE.toPlainDateTime(value.value)
    : value.value;

  const wallClock = plainDateTime.toZonedDateTime('UTC').epochNanoseconds + offsetNanoseconds(yearOffset(value));

  if (value.zone === null) {
    return wallClock;
  }

  return wallClock - BigInt(Math.round(zoneOffsetSeconds(value.zone, plainDateTime) * 1e9));
}

function fromEpochNanoseconds(ns, zone) {

  const fixed = fixedOffsetSeconds(zone);

  if (zone === null) {
    return Temporal.Instant.fromEpochNanoseconds(ns).toZonedDateTimeISO('UTC').toPlainDateTime();
  }

  if (fixed !== null) {
    return Temporal.Instant.fromEpochNanoseconds(ns + BigInt(fixed) * NS_PER_SECOND).toZonedDateTimeISO('UTC').toPlainDateTime();
  }

  return Temporal.Instant.fromEpochNanoseconds(ns).toZonedDateTimeISO(zone).toPlainDateTime();
}


// arithmetic ///////////////

/**
 * Add (sign = 1) or subtract (sign = -1) a duration to / from a
 * temporal value. Returns `null` for invalid combinations.
 */
export function addDuration(temporal, duration, sign = 1) {

  try {

    if (duration.yearsMonths) {
      const months = totalMonths(duration) * sign;

      if (isDate(temporal)) {
        return createDate(temporal.value.add({ months }, { overflow: 'constrain' }), yearOffset(temporal));
      }

      if (isDateTime(temporal)) {
        return createDateTime(temporal.value.add({ months }, { overflow: 'constrain' }), temporal.zone, yearOffset(temporal));
      }

      // time + years and months duration
      return null;
    }

    const ns = totalNanoseconds(duration) * BigInt(sign);

    if (isDate(temporal)) {
      const dateTime = addNanoseconds(temporal.value.toPlainDateTime(), ns);

      return createDate(dateTime.toPlainDate(), yearOffset(temporal));
    }

    if (isTime(temporal)) {
      const nsOfDay = ((ns % NS_PER_DAY) + NS_PER_DAY) % NS_PER_DAY;

      return createTime(addNanoseconds(REFERENCE_DATE.toPlainDateTime(temporal.value), nsOfDay).toPlainTime(), temporal.zone);
    }

    if (temporal.zone === null || fixedOffsetSeconds(temporal.zone) !== null) {
      return createDateTime(addNanoseconds(temporal.value, ns), temporal.zone, yearOffset(temporal));
    }

    // named zone: exact time arithmetic
    return createDateTime(fromEpochNanoseconds(epochNanoseconds(temporal) + ns, temporal.zone), temporal.zone);
  } catch {
    return null;
  }
}

function addNanoseconds(plainDateTime, ns) {

  const days = ns / NS_PER_DAY;
  const rest = ns % NS_PER_DAY;

  return plainDateTime.add({
    days: Number(days),
    nanoseconds: Number(rest)
  });
}

/**
 * Subtract two temporal values (`a - b`), yielding a days and time
 * duration. Returns `null` for invalid combinations.
 */
export function subtractTemporals(a, b) {

  try {

    if (isDate(a) && isDate(b)) {
      return createDaysTimeDuration(epochNanoseconds(a) - epochNanoseconds(b));
    }

    if (isTime(a) !== isTime(b)) {
      return null;
    }

    // a date is a date and time at midnight UTC
    if (isDate(a)) {
      a = dateToDateTime(a);
    }

    if (isDate(b)) {
      b = dateToDateTime(b);
    }

    if ((a.zone === null) !== (b.zone === null)) {
      return null;
    }

    return createDaysTimeDuration(epochNanoseconds(a) - epochNanoseconds(b));
  } catch {
    return null;
  }
}

/**
 * Add / subtract two durations of the same kind.
 */
export function addDurations(a, b, sign = 1) {

  if (a.yearsMonths !== b.yearsMonths) {
    return null;
  }

  if (a.yearsMonths) {
    return createYearsMonthsDuration(totalMonths(a) + sign * totalMonths(b));
  }

  return createDaysTimeDuration(totalNanoseconds(a) + BigInt(sign) * totalNanoseconds(b));
}

/**
 * Multiply a duration by a number.
 */
export function multiplyDuration(duration, factor) {

  if (!Number.isFinite(factor)) {
    return null;
  }

  if (duration.yearsMonths) {
    return createYearsMonthsDuration(Math.trunc(roundTo(totalMonths(duration) * factor)));
  }

  return createDaysTimeDuration(BigInt(Math.round(Number(totalNanoseconds(duration)) * factor)));
}

/**
 * Divide a duration by a number or by a duration of the same kind.
 */
export function divideDuration(duration, divisor) {

  if (typeof divisor === 'number') {

    if (divisor === 0 || !Number.isFinite(divisor)) {
      return null;
    }

    return multiplyDuration(duration, 1 / divisor);
  }

  if (!isDuration(divisor) || divisor.yearsMonths !== duration.yearsMonths) {
    return null;
  }

  if (duration.yearsMonths) {
    const months = totalMonths(divisor);

    return months === 0 ? null : totalMonths(duration) / months;
  }

  const ns = totalNanoseconds(divisor);

  return ns === 0n ? null : Number(totalNanoseconds(duration)) / Number(ns);
}

export function negateDuration(duration) {
  return duration.yearsMonths
    ? createYearsMonthsDuration(-totalMonths(duration))
    : createDaysTimeDuration(-totalNanoseconds(duration));
}

function roundTo(value) {
  return Number(value.toPrecision(15));
}


// comparison ///////////////

/**
 * Compare two temporal values (or durations) of the same type.
 *
 * @return {number|null} negative, zero or positive; `null` if not comparable
 */
export function compareTemporals(a, b) {

  if (isDuration(a) && isDuration(b)) {

    if (a.yearsMonths !== b.yearsMonths) {
      return null;
    }

    if (a.yearsMonths) {
      return Math.sign(totalMonths(a) - totalMonths(b));
    }

    return sign(totalNanoseconds(a) - totalNanoseconds(b));
  }

  if (isDate(a) && isDate(b)) {
    return sign(epochNanoseconds(a) - epochNanoseconds(b));
  }

  const bothDateTimes = isDateTime(a) && isDateTime(b);
  const bothTimes = isTime(a) && isTime(b);

  if (!bothDateTimes && !bothTimes) {
    return null;
  }

  if ((a.zone === null) !== (b.zone === null)) {
    return null;
  }

  return sign(epochNanoseconds(a) - epochNanoseconds(b));
}

/**
 * FEEL equality of temporal values (instants compare at
 * millisecond precision).
 *
 * @return {boolean|null}
 */
export function temporalEquals(a, b) {

  if (isDuration(a) && isDuration(b)) {

    if (a.yearsMonths !== b.yearsMonths) {

      // zero durations of either kind are equal
      if (isZeroDuration(a) && isZeroDuration(b)) {
        return true;
      }

      return null;
    }

    return compareTemporals(a, b) === 0;
  }

  if (isDate(a) && isDate(b)) {
    return epochNanoseconds(a) === epochNanoseconds(b);
  }

  if (!(isDateTime(a) && isDateTime(b)) && !(isTime(a) && isTime(b))) {
    return null;
  }

  if ((a.zone === null) !== (b.zone === null)) {
    return false;
  }

  return (epochNanoseconds(a) / 1000000n) === (epochNanoseconds(b) / 1000000n);
}

export function isZeroDuration(duration) {
  return duration.value.sign === 0;
}

function sign(bigint) {
  return bigint < 0n ? -1 : bigint > 0n ? 1 : 0;
}


// misc ///////////////

/**
 * Offset of a zoned time / date time as a days and time duration.
 */
export function timeOffset(value) {

  if (value.zone === null) {
    return null;
  }

  const plainDateTime = isTime(value) ? REFERENCE_DATE.toPlainDateTime(value.value) : value.value;

  return createDaysTimeDuration(BigInt(Math.round(zoneOffsetSeconds(value.zone, plainDateTime))) * NS_PER_SECOND);
}

/**
 * The FEEL `timezone` property: the zone name for named zones, `null` for
 * fixed offsets and local values.
 */
export function timezone(value) {

  if (value.zone === null || fixedOffsetSeconds(value.zone) !== null) {
    return value.zone === 'UTC' ? 'Z' : null;
  }

  return value.zone;
}

/**
 * The seconds component of a time / date time, including fractions.
 */
export function secondWithFraction(value) {
  const time = value.value;

  return time.second + (time.millisecond * 1e6 + time.microsecond * 1e3 + time.nanosecond) / 1e9;
}

export function now() {
  const zoned = Temporal.Now.zonedDateTimeISO();

  return createDateTime(zoned.toPlainDateTime(), zoned.timeZoneId);
}

export function today() {
  return createDate(Temporal.Now.plainDateISO());
}

/**
 * Convert any feelin temporal value to one with canonical formatting.
 */
export function normalizeTemporal(value) {

  if (isDate(value)) {
    return createDate(value.value, yearOffset(value));
  }

  if (isTime(value)) {
    return createTime(value.value, value.zone);
  }

  if (isDateTime(value)) {
    return createDateTime(value.value, value.zone, yearOffset(value));
  }

  return value;
}
