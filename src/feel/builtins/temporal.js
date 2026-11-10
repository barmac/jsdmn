/**
 * Conversion and temporal builtins (DMN 1.5, 10.3.4.1 / 10.3.4.7).
 */

import { builtin, invocationFailure } from './function.js';

import {
  yearOffset,
  createDate,
  createDateTime,
  createTime,
  createYearsMonthsDuration,
  dateFromParts,
  isDate,
  isDateTime,
  isDaysTimeDuration,
  isDuration,
  isTime,
  now,
  parseDate,
  parseDateTime,
  parseDuration,
  parseTime,
  timeFromParts,
  today,
  totalSeconds
} from '../temporal.js';

const isString = (value) => typeof value === 'string';

function invalid(name, value) {
  return invocationFailure(`${ name }(${ value }) is invalid`);
}

const date = builtin('date',
  {
    parameters: [ 'from' ],
    impl(from) {

      if (isString(from)) {
        return parseDate(from) || invalid('date', from);
      }

      if (isDate(from)) {
        return from;
      }

      if (isDateTime(from)) {
        return createDate(from.value.toPlainDate(), yearOffset(from));
      }

      return invalid('date', from);
    }
  },
  {
    parameters: [ 'year', 'month', 'day' ],
    impl(year, month, day) {
      return dateFromParts(year, month, day) || invocationFailure(`${ year }, ${ month }, ${ day } is not a valid date`);
    }
  }
);

const time = builtin('time',
  {
    parameters: [ 'from' ],
    impl(from) {

      if (isString(from)) {
        return parseTime(from) || invalid('time', from);
      }

      if (isTime(from)) {
        return from;
      }

      if (isDateTime(from)) {
        return createTime(from.value.toPlainTime(), from.zone);
      }

      if (isDate(from)) {
        return parseTime('00:00:00Z');
      }

      return invalid('time', from);
    }
  },
  {
    parameters: [ 'hour', 'minute', 'second', 'offset?' ],
    impl(hour, minute, second, offset) {

      let zone = null;

      if (offset !== undefined && offset !== null) {

        if (!isDaysTimeDuration(offset)) {
          return invocationFailure('offset must be a days and time duration');
        }

        const seconds = totalSeconds(offset);

        if (Math.abs(seconds) > 18 * 3600 || !Number.isInteger(seconds)) {
          return invocationFailure(`invalid offset <${ offset }>`);
        }

        zone = formatOffset(seconds);
      }

      return timeFromParts(hour, minute, second, zone) || invocationFailure(`${ hour }, ${ minute }, ${ second } is not a valid time`);
    }
  }
);

function formatOffset(seconds) {

  if (seconds === 0) {
    return 'UTC';
  }

  const sign = seconds < 0 ? '-' : '+';
  const abs = Math.abs(seconds);

  const pad = (n) => String(n).padStart(2, '0');

  const secs = abs % 60;

  return `${ sign }${ pad(Math.floor(abs / 3600)) }:${ pad(Math.floor((abs % 3600) / 60)) }${ secs ? `:${ pad(secs) }` : '' }`;
}

const dateAndTime = builtin('date and time',
  {
    parameters: [ 'from' ],
    impl(from) {

      if (isString(from)) {
        return parseDateTime(from) || invalid('date and time', from);
      }

      if (isDateTime(from)) {
        return from;
      }

      if (isDate(from)) {
        return createDateTime(from.value.toPlainDateTime(), null, yearOffset(from));
      }

      return invalid('date and time', from);
    }
  },
  {
    parameters: [ 'date', 'time' ],
    impl(date, time) {

      if (!(isDate(date) || isDateTime(date)) || !isTime(time)) {
        return invocationFailure('date and time(date, time) expects a date and a time');
      }

      const plainDate = isDate(date) ? date.value : date.value.toPlainDate();

      return createDateTime(plainDate.toPlainDateTime(time.value), time.zone, yearOffset(date));
    }
  }
);

const duration = builtin('duration', {
  parameters: [ 'from' ],
  impl(from) {

    if (isString(from)) {
      return parseDuration(from) || invalid('duration', from);
    }

    if (isDuration(from)) {
      return from;
    }

    return invalid('duration', from);
  }
});

const yearsAndMonthsDuration = builtin('years and months duration', {
  parameters: [ 'from', 'to' ],
  impl(from, to) {

    const toPlainDate = (value) => {

      if (isDate(value) || isDateTime(value)) {
        return { year: value.year, month: value.month, day: value.day };
      }

      return null;
    };

    const start = toPlainDate(from);
    const end = toPlainDate(to);

    if (!start || !end) {
      return invocationFailure('years and months duration(from, to) expects dates');
    }

    let months = (end.year - start.year) * 12 + (end.month - start.month);

    // incomplete months do not count
    if (months > 0 && end.day < start.day) {
      months--;
    } else if (months < 0 && end.day > start.day) {
      months++;
    }

    return createYearsMonthsDuration(months);
  }
});

const nowFn = builtin('now', {
  parameters: [],
  impl: () => now()
});

const todayFn = builtin('today', {
  parameters: [],
  impl: () => today()
});

export default {
  'date': date,
  'time': time,
  'date and time': dateAndTime,
  'duration': duration,
  'years and months duration': yearsAndMonthsDuration,
  'now': nowFn,
  'today': todayFn
};

