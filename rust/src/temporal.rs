//! FEEL temporal values: date, time, date and time and the two duration kinds.
//!
//! Calendar computations are done in-house (proleptic Gregorian calendar on
//! `i64` years) so that very large years (up to ±999 999 999) are supported.
//! IANA time zone rules are looked up with `chrono-tz`.

use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

pub const NANOS_PER_SEC: i128 = 1_000_000_000;
pub const SECS_PER_DAY: i128 = 86_400;
pub const NANOS_PER_DAY: i128 = SECS_PER_DAY * NANOS_PER_SEC;

pub const MAX_YEAR: i64 = 999_999_999;

pub fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

pub fn days_in_month(y: i64, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap(y) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

/// Days since 1970-01-01 (Howard Hinnant's algorithm).
pub fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let m = m as i64;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

pub fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Zone {
    /// Fixed offset in seconds east of UTC.
    Offset(i32),
    /// IANA zone id.
    Id(String),
}

impl Zone {
    pub fn parse_id(name: &str) -> Option<Zone> {
        if chrono_tz::Tz::from_str(name).is_ok() {
            Some(Zone::Id(name.to_string()))
        } else {
            None
        }
    }
}

/// Map a (possibly huge) year onto an equivalent year that chrono can handle.
fn clamp_year_for_tz(y: i64) -> i64 {
    if (1..=9999).contains(&y) {
        y
    } else if y > 9999 {
        // tz rules repeat; shift by 400 year cycles into the 9600..9999 window
        9600 + (y - 9600).rem_euclid(400)
    } else {
        // far past: use local mean time era equivalent
        1 + (y - 1).rem_euclid(400)
    }
}

fn naive(local_days: i64, secs_of_day: i64) -> Option<chrono::NaiveDateTime> {
    let (y, m, d) = civil_from_days(local_days);
    let y2 = clamp_year_for_tz(y);
    let date = chrono::NaiveDate::from_ymd_opt(y2 as i32, m, d.min(days_in_month(y2, m)))?;
    let time = chrono::NaiveTime::from_num_seconds_from_midnight_opt(secs_of_day as u32, 0)?;
    Some(chrono::NaiveDateTime::new(date, time))
}

/// Offset (seconds) of the given zone for a local date time.
pub fn zone_offset_for_local(zone: &str, local_days: i64, secs_of_day: i64) -> Option<i32> {
    use chrono::{Offset, TimeZone};
    let tz = chrono_tz::Tz::from_str(zone).ok()?;
    let nd = naive(local_days, secs_of_day)?;
    match tz.offset_from_local_datetime(&nd) {
        chrono::LocalResult::Single(o) => Some(o.fix().local_minus_utc()),
        chrono::LocalResult::Ambiguous(a, _) => Some(a.fix().local_minus_utc()),
        chrono::LocalResult::None => {
            // in a gap: use the offset before the transition
            let earlier = nd - chrono::Duration::hours(3);
            match tz.offset_from_local_datetime(&earlier) {
                chrono::LocalResult::Single(o) => Some(o.fix().local_minus_utc()),
                chrono::LocalResult::Ambiguous(a, _) => Some(a.fix().local_minus_utc()),
                _ => Some(tz.offset_from_utc_datetime(&nd).fix().local_minus_utc()),
            }
        }
    }
}

/// Offset (seconds) of the given zone at an UTC instant.
pub fn zone_offset_for_instant(zone: &str, utc_days: i64, secs_of_day: i64) -> Option<i32> {
    use chrono::{Offset, TimeZone};
    let tz = chrono_tz::Tz::from_str(zone).ok()?;
    let nd = naive(utc_days, secs_of_day)?;
    Some(tz.offset_from_utc_datetime(&nd).fix().local_minus_utc())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Date {
    pub year: i64,
    pub month: u32,
    pub day: u32,
}

impl Date {
    pub fn new(year: i64, month: u32, day: u32) -> Option<Date> {
        if !(1..=12).contains(&month) || day < 1 || day > days_in_month(year, month) {
            return None;
        }
        if year.abs() > MAX_YEAR {
            return None;
        }
        Some(Date { year, month, day })
    }

    pub fn from_days(days: i64) -> Option<Date> {
        let (y, m, d) = civil_from_days(days);
        Date::new(y, m, d)
    }

    pub fn days(&self) -> i64 {
        days_from_civil(self.year, self.month, self.day)
    }

    /// ISO weekday, Monday = 1.
    pub fn weekday(&self) -> u32 {
        // 1970-01-01 was a Thursday (4)
        ((self.days() + 3).rem_euclid(7) + 1) as u32
    }

    pub fn day_of_year(&self) -> u32 {
        (self.days() - days_from_civil(self.year, 1, 1) + 1) as u32
    }

    /// ISO 8601 week of year.
    pub fn week_of_year(&self) -> u32 {
        let doy = self.day_of_year() as i64;
        let wd = self.weekday() as i64;
        let w = (doy - wd + 10) / 7;
        if w < 1 {
            weeks_in_year(self.year - 1)
        } else if w > weeks_in_year(self.year) as i64 {
            1
        } else {
            w as u32
        }
    }

    pub fn add_months(&self, months: i64) -> Option<Date> {
        let total = self.year * 12 + (self.month as i64 - 1) + months;
        let y = total.div_euclid(12);
        let m = (total.rem_euclid(12) + 1) as u32;
        let d = self.day.min(days_in_month(y, m));
        Date::new(y, m, d)
    }

    /// Parse `[-]yyyy-mm-dd`.
    pub fn parse(s: &str) -> Option<Date> {
        let (neg, rest) = match s.strip_prefix('-') {
            Some(r) => (true, r),
            None => (false, s),
        };
        let mut parts = rest.splitn(3, '-');
        let ys = parts.next()?;
        let ms = parts.next()?;
        let ds = parts.next()?;
        if ys.len() < 4 || !ys.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        if ys.len() > 4 && ys.starts_with('0') {
            return None;
        }
        if ms.len() != 2 || ds.len() != 2 {
            return None;
        }
        if !ms.bytes().all(|b| b.is_ascii_digit()) || !ds.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let y: i64 = ys.parse().ok()?;
        let y = if neg { -y } else { y };
        Date::new(y, ms.parse().ok()?, ds.parse().ok()?)
    }
}

fn weeks_in_year(y: i64) -> u32 {
    let p = |y: i64| (y + y.div_euclid(4) - y.div_euclid(100) + y.div_euclid(400)).rem_euclid(7);
    if p(y) == 4 || p(y - 1) == 3 { 53 } else { 52 }
}

fn fmt_year(f: &mut fmt::Formatter<'_>, y: i64) -> fmt::Result {
    if y < 0 {
        write!(f, "-{:04}", -y)
    } else {
        write!(f, "{:04}", y)
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_year(f, self.year)?;
        write!(f, "-{:02}-{:02}", self.month, self.day)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Time {
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
    pub nanos: u32,
    pub zone: Option<Zone>,
}

fn parse_fixed_offset(s: &str) -> Option<i32> {
    // [+-]hh:mm[:ss]
    let sign = match s.as_bytes().first()? {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let rest = &s[1..];
    let parts: Vec<&str> = rest.split(':').collect();
    if parts.len() < 2 || parts.len() > 3 {
        return None;
    }
    for p in &parts {
        if p.len() != 2 || !p.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
    }
    let h: i32 = parts[0].parse().ok()?;
    let m: i32 = parts[1].parse().ok()?;
    let sec: i32 = if parts.len() == 3 {
        parts[2].parse().ok()?
    } else {
        0
    };
    if m > 59 || sec > 59 {
        return None;
    }
    let total = h * 3600 + m * 60 + sec;
    if total > 18 * 3600 {
        return None;
    }
    Some(sign * total)
}

impl Time {
    pub fn new(
        hour: u32,
        minute: u32,
        second: u32,
        nanos: u32,
        zone: Option<Zone>,
    ) -> Option<Time> {
        if hour > 23 || minute > 59 || second > 59 || nanos >= 1_000_000_000 {
            return None;
        }
        Some(Time {
            hour,
            minute,
            second,
            nanos,
            zone,
        })
    }

    pub fn secs_of_day(&self) -> i64 {
        (self.hour * 3600 + self.minute * 60 + self.second) as i64
    }

    pub fn nanos_of_day(&self) -> i128 {
        self.secs_of_day() as i128 * NANOS_PER_SEC + self.nanos as i128
    }

    pub fn from_nanos_of_day(n: i128, zone: Option<Zone>) -> Time {
        let n = n.rem_euclid(NANOS_PER_DAY);
        let secs = (n / NANOS_PER_SEC) as u32;
        let nanos = (n % NANOS_PER_SEC) as u32;
        Time {
            hour: secs / 3600,
            minute: (secs / 60) % 60,
            second: secs % 60,
            nanos,
            zone,
        }
    }

    /// Parse `hh:mm:ss[.fff][Z|±hh:mm|@zone]`.
    pub fn parse(s: &str) -> Option<Time> {
        let b = s.as_bytes();
        if b.len() < 8 {
            return None;
        }
        let dig2 = |i: usize| -> Option<u32> {
            let c1 = *b.get(i)?;
            let c2 = *b.get(i + 1)?;
            if c1.is_ascii_digit() && c2.is_ascii_digit() {
                Some(((c1 - b'0') * 10 + (c2 - b'0')) as u32)
            } else {
                None
            }
        };
        let h = dig2(0)?;
        if b[2] != b':' {
            return None;
        }
        let m = dig2(3)?;
        if b[5] != b':' {
            return None;
        }
        let sec = dig2(6)?;
        let mut i = 8;
        let mut nanos: u32 = 0;
        if i < b.len() && b[i] == b'.' {
            i += 1;
            let start = i;
            while i < b.len() && b[i].is_ascii_digit() {
                i += 1;
            }
            if start == i {
                return None;
            }
            let frac = &s[start..i];
            let mut digits: String = frac.chars().take(9).collect();
            while digits.len() < 9 {
                digits.push('0');
            }
            nanos = digits.parse().ok()?;
        }
        let rest = &s[i..];
        let zone = if rest.is_empty() {
            None
        } else if rest == "Z" || rest == "z" {
            Some(Zone::Offset(0))
        } else if let Some(id) = rest.strip_prefix('@') {
            Some(Zone::parse_id(id)?)
        } else {
            Some(Zone::Offset(parse_fixed_offset(rest)?))
        };
        if h == 24 && m == 0 && sec == 0 && nanos == 0 {
            return Time::new(0, 0, 0, 0, zone);
        }
        Time::new(h, m, sec, nanos, zone)
    }

    /// Offset in seconds if fixed (zone ids have no offset without a date).
    pub fn fixed_offset(&self) -> Option<i32> {
        match &self.zone {
            Some(Zone::Offset(o)) => Some(*o),
            _ => None,
        }
    }
}

fn fmt_frac(f: &mut fmt::Formatter<'_>, nanos: u32) -> fmt::Result {
    if nanos > 0 {
        let s = format!("{:09}", nanos);
        write!(f, ".{}", s.trim_end_matches('0'))?;
    }
    Ok(())
}

pub fn fmt_offset(o: i32) -> String {
    if o == 0 {
        return "Z".to_string();
    }
    let sign = if o < 0 { '-' } else { '+' };
    let a = o.abs();
    if a % 60 != 0 {
        format!("{}{:02}:{:02}:{:02}", sign, a / 3600, (a / 60) % 60, a % 60)
    } else {
        format!("{}{:02}:{:02}", sign, a / 3600, (a / 60) % 60)
    }
}

fn fmt_zone(f: &mut fmt::Formatter<'_>, zone: &Option<Zone>) -> fmt::Result {
    match zone {
        None => Ok(()),
        Some(Zone::Offset(o)) => f.write_str(&fmt_offset(*o)),
        Some(Zone::Id(id)) => write!(f, "@{}", id),
    }
}

impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}:{:02}:{:02}", self.hour, self.minute, self.second)?;
        fmt_frac(f, self.nanos)?;
        fmt_zone(f, &self.zone)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DateTime {
    pub date: Date,
    pub time: Time,
}

impl DateTime {
    pub fn parse(s: &str) -> Option<DateTime> {
        match s.find('T') {
            Some(i) => {
                let date = Date::parse(&s[..i])?;
                let time = Time::parse(&s[i + 1..])?;
                // 24:00:00 rolls over to the next day
                if s[i + 1..].starts_with("24:") {
                    let next = Date::from_days(date.days() + 1)?;
                    return Some(DateTime { date: next, time });
                }
                Some(DateTime { date, time })
            }
            None => None,
        }
    }

    pub fn from_date(date: Date) -> DateTime {
        DateTime {
            date,
            time: Time {
                hour: 0,
                minute: 0,
                second: 0,
                nanos: 0,
                zone: None,
            },
        }
    }

    /// Nanoseconds since epoch, interpreting the local fields as UTC.
    pub fn local_nanos(&self) -> i128 {
        self.date.days() as i128 * NANOS_PER_DAY + self.time.nanos_of_day()
    }

    pub fn from_local_nanos(n: i128, zone: Option<Zone>) -> Option<DateTime> {
        let days = n.div_euclid(NANOS_PER_DAY);
        let rem = n.rem_euclid(NANOS_PER_DAY);
        let date = Date::from_days(i64::try_from(days).ok()?)?;
        Some(DateTime {
            date,
            time: Time::from_nanos_of_day(rem, zone),
        })
    }

    /// Offset in seconds, if the value has a zone.
    pub fn offset(&self) -> Option<i32> {
        match &self.time.zone {
            None => None,
            Some(Zone::Offset(o)) => Some(*o),
            Some(Zone::Id(id)) => {
                zone_offset_for_local(id, self.date.days(), self.time.secs_of_day())
            }
        }
    }

    pub fn has_zone(&self) -> bool {
        self.time.zone.is_some()
    }

    /// UTC instant in nanoseconds since epoch (only for zoned values).
    pub fn instant(&self) -> Option<i128> {
        let o = self.offset()?;
        Some(self.local_nanos() - o as i128 * NANOS_PER_SEC)
    }

    /// Build from an instant and zone.
    pub fn from_instant(instant: i128, zone: Zone) -> Option<DateTime> {
        let offset = match &zone {
            Zone::Offset(o) => *o,
            Zone::Id(id) => {
                let days = instant.div_euclid(NANOS_PER_DAY);
                let secs = instant.rem_euclid(NANOS_PER_DAY) / NANOS_PER_SEC;
                zone_offset_for_instant(id, i64::try_from(days).ok()?, secs as i64)?
            }
        };
        DateTime::from_local_nanos(instant + offset as i128 * NANOS_PER_SEC, Some(zone))
    }

    /// Add a days-and-time duration.
    pub fn add_nanos(&self, d: i128) -> Option<DateTime> {
        match &self.time.zone {
            Some(Zone::Id(_)) => {
                let i = self.instant()?;
                DateTime::from_instant(i + d, self.time.zone.clone().unwrap())
            }
            _ => DateTime::from_local_nanos(self.local_nanos() + d, self.time.zone.clone()),
        }
    }

    pub fn add_months(&self, months: i64) -> Option<DateTime> {
        let date = self.date.add_months(months)?;
        Some(DateTime {
            date,
            time: self.time.clone(),
        })
    }
}

impl fmt::Display for DateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}T{}", self.date, self.time)
    }
}

/// Days and time duration, stored in nanoseconds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DtDuration(pub i128);

/// Years and months duration, stored in months.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct YmDuration(pub i64);

impl fmt::Display for DtDuration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let n = self.0;
        if n == 0 {
            return f.write_str("PT0S");
        }
        if n < 0 {
            f.write_str("-")?;
        }
        let a = n.unsigned_abs();
        let nanos = (a % NANOS_PER_SEC as u128) as u32;
        let total_secs = a / NANOS_PER_SEC as u128;
        let days = total_secs / 86400;
        let hours = (total_secs / 3600) % 24;
        let minutes = (total_secs / 60) % 60;
        let secs = total_secs % 60;
        f.write_str("P")?;
        if days > 0 {
            write!(f, "{}D", days)?;
        }
        if hours > 0 || minutes > 0 || secs > 0 || nanos > 0 {
            f.write_str("T")?;
            if hours > 0 {
                write!(f, "{}H", hours)?;
            }
            if minutes > 0 {
                write!(f, "{}M", minutes)?;
            }
            if secs > 0 || nanos > 0 {
                write!(f, "{}", secs)?;
                fmt_frac(f, nanos)?;
                f.write_str("S")?;
            }
        }
        Ok(())
    }
}

impl fmt::Display for YmDuration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let n = self.0;
        if n == 0 {
            return f.write_str("P0M");
        }
        if n < 0 {
            f.write_str("-")?;
        }
        let a = n.unsigned_abs();
        let y = a / 12;
        let m = a % 12;
        f.write_str("P")?;
        if y > 0 {
            write!(f, "{}Y", y)?;
        }
        if m > 0 {
            write!(f, "{}M", m)?;
        }
        Ok(())
    }
}

pub enum Duration {
    Dt(DtDuration),
    Ym(YmDuration),
}

/// Parse an ISO 8601 / XML Schema duration.
pub fn parse_duration(s: &str) -> Option<Duration> {
    let (neg, rest) = match s.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, s),
    };
    let rest = rest.strip_prefix('P')?;
    if rest.is_empty() {
        return None;
    }
    let (date_part, time_part) = match rest.find('T') {
        Some(i) => {
            let t = &rest[i + 1..];
            if t.is_empty() {
                return None;
            }
            (&rest[..i], Some(t))
        }
        None => (rest, None),
    };
    let mut years: Option<i128> = None;
    let mut months: Option<i128> = None;
    let mut days: Option<i128> = None;
    let mut hours: Option<i128> = None;
    let mut minutes: Option<i128> = None;
    let mut secs: Option<(i128, u32)> = None;

    // date part
    let mut num = String::new();
    let mut order = 0;
    for c in date_part.chars() {
        if c.is_ascii_digit() {
            num.push(c);
            continue;
        }
        if num.is_empty() {
            return None;
        }
        let v: i128 = num.parse().ok()?;
        num.clear();
        match c {
            'Y' if order < 1 => {
                years = Some(v);
                order = 1;
            }
            'M' if order < 2 => {
                months = Some(v);
                order = 2;
            }
            'D' if order < 3 => {
                days = Some(v);
                order = 3;
            }
            _ => return None,
        }
    }
    if !num.is_empty() {
        return None;
    }
    if let Some(t) = time_part {
        let mut order = 0;
        let mut frac: Option<String> = None;
        let mut in_frac = false;
        for c in t.chars() {
            if c.is_ascii_digit() {
                if in_frac {
                    frac.as_mut().unwrap().push(c);
                } else {
                    num.push(c);
                }
                continue;
            }
            if c == '.' && !in_frac && !num.is_empty() {
                in_frac = true;
                frac = Some(String::new());
                continue;
            }
            if num.is_empty() {
                return None;
            }
            let v: i128 = num.parse().ok()?;
            num.clear();
            match c {
                'H' if order < 1 && !in_frac => {
                    hours = Some(v);
                    order = 1;
                }
                'M' if order < 2 && !in_frac => {
                    minutes = Some(v);
                    order = 2;
                }
                'S' if order < 3 => {
                    let fr = frac.take().unwrap_or_default();
                    let mut digits: String = fr.chars().take(9).collect();
                    while digits.len() < 9 {
                        digits.push('0');
                    }
                    secs = Some((v, digits.parse().ok()?));
                    in_frac = false;
                    order = 3;
                }
                _ => return None,
            }
        }
        if !num.is_empty() || in_frac {
            return None;
        }
    }
    let has_ym = years.is_some() || months.is_some();
    let has_dt = days.is_some() || hours.is_some() || minutes.is_some() || secs.is_some();
    if has_ym && has_dt {
        return None;
    }
    if !has_ym && !has_dt {
        return None;
    }
    if has_ym {
        let total = years.unwrap_or(0) * 12 + months.unwrap_or(0);
        let total = i64::try_from(total).ok()?;
        Some(Duration::Ym(YmDuration(if neg { -total } else { total })))
    } else {
        let (s, ns) = secs.unwrap_or((0, 0));
        let total_secs =
            days.unwrap_or(0) * 86400 + hours.unwrap_or(0) * 3600 + minutes.unwrap_or(0) * 60 + s;
        let total = total_secs
            .checked_mul(NANOS_PER_SEC)?
            .checked_add(ns as i128)?;
        Some(Duration::Dt(DtDuration(if neg { -total } else { total })))
    }
}

/// Compare times (only when comparable).
pub fn time_cmp(a: &Time, b: &Time) -> Option<Ordering> {
    match (&a.zone, &b.zone) {
        (None, None) => Some(a.nanos_of_day().cmp(&b.nanos_of_day())),
        (Some(Zone::Offset(o1)), Some(Zone::Offset(o2))) => {
            let x = a.nanos_of_day() - *o1 as i128 * NANOS_PER_SEC;
            let y = b.nanos_of_day() - *o2 as i128 * NANOS_PER_SEC;
            Some(x.cmp(&y))
        }
        (Some(Zone::Id(z1)), Some(Zone::Id(z2))) if z1 == z2 => {
            Some(a.nanos_of_day().cmp(&b.nanos_of_day()))
        }
        _ => None,
    }
}

pub fn datetime_cmp(a: &DateTime, b: &DateTime) -> Option<Ordering> {
    match (a.has_zone(), b.has_zone()) {
        (false, false) => Some(a.local_nanos().cmp(&b.local_nanos())),
        (true, true) => Some(a.instant()?.cmp(&b.instant()?)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        let d = Date::parse("2018-12-08").unwrap();
        assert_eq!(Date::from_days(d.days()).unwrap(), d);
        assert_eq!(d.weekday(), 6);
        assert_eq!(Date::parse("2019-12-30").unwrap().week_of_year(), 1);
        assert_eq!(Date::parse("2021-01-03").unwrap().week_of_year(), 53);
    }

    #[test]
    fn durations() {
        match parse_duration("P1DT2H3M4.5S").unwrap() {
            Duration::Dt(d) => assert_eq!(d.to_string(), "P1DT2H3M4.5S"),
            _ => panic!(),
        }
        match parse_duration("-P14M").unwrap() {
            Duration::Ym(d) => assert_eq!(d.to_string(), "-P1Y2M"),
            _ => panic!(),
        }
        assert!(parse_duration("P").is_none());
        assert!(parse_duration("PT").is_none());
    }

    #[test]
    fn zones() {
        let dt = DateTime::parse("2018-10-08T00:00:00@Europe/Paris").unwrap();
        assert_eq!(dt.offset(), Some(7200));
    }
}
