//! FEEL built-in functions.

use super::eval::{CallArgs, Scope, call};
use super::regex::{translate_regex, translate_replacement};
use crate::number::{Number, Rounding};
use crate::temporal::{self, Date, DateTime, DtDuration, NANOS_PER_SEC, Time, YmDuration, Zone};
use crate::value::{CmpOp, Context, Value, feel_cmp, feel_eq, to_feel_string};
use std::cmp::Ordering;
use std::rc::Rc;

/// Signatures: list of parameter name lists. A trailing `...` marks varargs.
fn signatures(name: &str) -> Option<&'static [&'static [&'static str]]> {
    Some(match name {
        "date" => &[&["from"], &["year", "month", "day"]],
        "date and time" => &[
            &["from"],
            &["date", "time"],
            &["year", "month", "day", "hour", "minute", "second"],
            &["year", "month", "day", "hour", "minute", "second", "offset"],
        ],
        "time" => &[
            &["from"],
            &["hour", "minute", "second"],
            &["hour", "minute", "second", "offset"],
        ],
        "number" => &[
            &["from", "grouping separator", "decimal separator"],
            &["from"],
        ],
        "string" => &[&["from"]],
        "duration" => &[&["from"]],
        "years and months duration" => &[&["from", "to"]],
        "not" => &[&["negand"]],
        "is" => &[&["value1", "value2"]],
        "substring" => &[
            &["string", "start position"],
            &["string", "start position", "length"],
        ],
        "string length" | "upper case" | "lower case" => &[&["string"]],
        "substring before" | "substring after" | "contains" | "starts with" | "ends with" => {
            &[&["string", "match"]]
        }
        "replace" => &[
            &["input", "pattern", "replacement"],
            &["input", "pattern", "replacement", "flags"],
        ],
        "matches" => &[&["input", "pattern"], &["input", "pattern", "flags"]],
        "split" => &[&["string", "delimiter"], &["string", "delimiter", "flags"]],
        "string join" => &[&["list"], &["list", "delimiter"]],
        "list contains" => &[&["list", "element"]],
        "count" | "reverse" | "distinct values" | "flatten" => &[&["list"]],
        "min" | "max" | "sum" | "mean" | "all" | "any" | "product" | "median" | "stddev"
        | "mode" => &[&["list"], &["..."]],
        "sublist" => &[
            &["list", "start position"],
            &["list", "start position", "length"],
        ],
        "append" => &[&["list", "..."]],
        "concatenate" | "union" => &[&["..."]],
        "insert before" => &[&["list", "position", "newItem"]],
        "remove" => &[&["list", "position"]],
        "index of" => &[&["list", "match"]],
        "sort" => &[&["list"], &["list", "precedes"]],
        "list replace" => &[
            &["list", "position", "newItem"],
            &["list", "match", "newItem"],
        ],
        "decimal" => &[&["n", "scale"]],
        "floor" | "ceiling" => &[&["n"], &["n", "scale"]],
        "round up" | "round down" | "round half up" | "round half down" => {
            &[&["n"], &["n", "scale"]]
        }
        "abs" => &[&["n"]],
        "modulo" => &[&["dividend", "divisor"]],
        "sqrt" | "log" | "exp" | "odd" | "even" => &[&["number"]],
        "day of year" | "day of week" | "month of year" | "week of year" => &[&["date"]],
        "now" | "today" => &[&[]],
        "before" | "after" | "meets" | "met by" | "overlaps" | "overlaps before"
        | "overlaps after" | "finishes" | "finished by" | "includes" | "during" | "starts"
        | "started by" | "coincides" => &[
            &["point1", "point2"],
            &["point", "range"],
            &["range", "point"],
            &["range1", "range2"],
        ],
        "get value" => &[&["m", "key"]],
        "get entries" => &[&["m"]],
        "context" => &[&["entries"]],
        "context put" => &[&["context", "key", "value"], &["context", "keys", "value"]],
        "context merge" => &[&["contexts"]],
        "range" => &[&["from"]],
        _ => return None,
    })
}

pub fn is_builtin_name(n: &str) -> bool {
    signatures(n).is_some()
}

/// Resolve call arguments into positional arguments (None = omitted).
fn resolve_args(name: &str, args: CallArgs) -> Option<(usize, Vec<Option<Value>>)> {
    let sigs = signatures(name)?;
    match args {
        CallArgs::Positional(v) => {
            // varargs?
            for (i, sig) in sigs.iter().enumerate() {
                if sig.last() == Some(&"...") {
                    if v.len() >= sig.len() - 1 {
                        return Some((i, v.into_iter().map(Some).collect()));
                    }
                } else if sig.len() == v.len() {
                    return Some((i, v.into_iter().map(Some).collect()));
                }
            }
            None
        }
        CallArgs::Named(named) => {
            for (i, sig) in sigs.iter().enumerate() {
                if sig.contains(&"...") && !(sig.len() == 2 && named.len() == 1) {
                    continue;
                }
                let params: Vec<&str> = sig.iter().copied().filter(|p| *p != "...").collect();
                if named.iter().all(|(n, _)| params.contains(&&**n)) {
                    let mut out = vec![Some(Value::Null); params.len()];
                    for (n, v) in &named {
                        let idx = params.iter().position(|p| *p == &**n).unwrap();
                        out[idx] = Some(v.clone());
                    }
                    return Some((i, out));
                }
            }
            // allow omitted trailing optional params of a longer signature
            None
        }
    }
}

fn num(v: &Option<Value>) -> Option<&Number> {
    match v {
        Some(Value::Number(n)) => Some(n),
        Some(Value::List(l)) if l.len() == 1 => l[0].as_number(),
        _ => None,
    }
}

fn string(v: &Option<Value>) -> Option<&str> {
    match v {
        Some(Value::String(s)) => Some(s),
        Some(Value::List(l)) if l.len() == 1 => l[0].as_str(),
        _ => None,
    }
}

fn int(v: &Option<Value>) -> Option<i64> {
    num(v)?.to_i64()
}

fn bool_v(b: bool) -> Value {
    Value::Bool(b)
}

fn opt(v: Option<Value>) -> Value {
    v.unwrap_or(Value::Null)
}

fn list_arg(v: &Option<Value>) -> Option<Rc<Vec<Value>>> {
    match v {
        Some(Value::List(l)) => Some(l.clone()),
        Some(Value::Null) | None => None,
        Some(other) => Some(Rc::new(vec![other.clone()])),
    }
}

fn flags_arg(v: &Option<Value>) -> Option<&str> {
    match v {
        Some(Value::String(s)) => Some(s),
        Some(Value::Null) | None => Some(""),
        _ => None,
    }
}

/// Varargs helper: either a single list argument or the arguments themselves.
fn varargs(args: &[Option<Value>], sig: usize) -> Option<Vec<Value>> {
    if sig == 0 {
        // (list)
        return match &args[0] {
            Some(Value::List(l)) => Some(l.as_ref().clone()),
            Some(Value::Null) | None => None,
            Some(v) => Some(vec![v.clone()]),
        };
    }
    if args.len() == 1
        && let Some(Value::List(l)) = &args[0]
    {
        return Some(l.as_ref().clone());
    }
    if args.is_empty() {
        return None;
    }
    Some(
        args.iter()
            .map(|a| a.clone().unwrap_or(Value::Null))
            .collect(),
    )
}

pub fn call_builtin(name: &str, args: CallArgs, scope: &Rc<Scope>) -> Value {
    if name == "context put"
        && let CallArgs::Named(n) = &args
        && n.iter()
            .any(|(k, v)| &**k == "key" && matches!(v, Value::List(_)))
    {
        return Value::Null;
    }
    let (sig, a) = match resolve_args(name, args) {
        Some(x) => x,
        None => return Value::Null,
    };
    opt(dispatch(name, sig, &a, scope))
}

fn dispatch(name: &str, sig: usize, a: &[Option<Value>], scope: &Rc<Scope>) -> Option<Value> {
    match name {
        // ---- conversion
        "date" => {
            if sig == 0 {
                match a[0].as_ref()? {
                    Value::String(s) => Date::parse(s).map(Value::Date),
                    Value::Date(d) => Some(Value::Date(*d)),
                    Value::DateTime(dt) => Some(Value::Date(dt.date)),
                    _ => None,
                }
            } else {
                let y = int(&a[0])?;
                let m = int(&a[1])?;
                let d = int(&a[2])?;
                if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
                    return None;
                }
                Date::new(y, m as u32, d as u32).map(Value::Date)
            }
        }
        "date and time" => match sig {
            0 => match a[0].as_ref()? {
                Value::String(s) => {
                    if s.contains('T') {
                        DateTime::parse(s).map(Value::DateTime)
                    } else {
                        Date::parse(s).map(|d| Value::DateTime(DateTime::from_date(d)))
                    }
                }
                Value::DateTime(d) => Some(Value::DateTime(d.clone())),
                Value::Date(d) => Some(Value::DateTime(DateTime::from_date(*d))),
                _ => None,
            },
            1 => {
                let date = match a[0].as_ref()? {
                    Value::Date(d) => *d,
                    Value::DateTime(dt) => dt.date,
                    _ => return None,
                };
                let time = match a[1].as_ref()? {
                    Value::Time(t) => t.clone(),
                    _ => return None,
                };
                Some(Value::DateTime(DateTime { date, time }))
            }
            _ => {
                let y = int(&a[0])?;
                let mo = int(&a[1])?;
                let d = int(&a[2])?;
                let h = int(&a[3])?;
                let mi = int(&a[4])?;
                let sn = num(&a[5])?;
                let date = Date::new(y, u32::try_from(mo).ok()?, u32::try_from(d).ok()?)?;
                let (s, nanos) = split_seconds(sn)?;
                let zone = if sig == 3 {
                    match a[6].as_ref()? {
                        Value::DtDuration(o) => {
                            Some(Zone::Offset(i32::try_from(o.0 / NANOS_PER_SEC).ok()?))
                        }
                        Value::String(z) => Some(Zone::parse_id(z)?),
                        Value::Null => None,
                        _ => return None,
                    }
                } else {
                    None
                };
                let time = Time::new(
                    u32::try_from(h).ok()?,
                    u32::try_from(mi).ok()?,
                    s,
                    nanos,
                    zone,
                )?;
                Some(Value::DateTime(DateTime { date, time }))
            }
        },
        "time" => {
            if sig == 0 {
                match a[0].as_ref()? {
                    Value::String(s) => Time::parse(s).map(Value::Time),
                    Value::Time(t) => Some(Value::Time(t.clone())),
                    Value::DateTime(dt) => {
                        let mut t = dt.time.clone();
                        if let Some(Zone::Id(_)) = &t.zone {
                            // keep zone id
                            t.zone = dt.time.zone.clone();
                        }
                        Some(Value::Time(t))
                    }
                    Value::Date(_) => Some(Value::Time(Time {
                        hour: 0,
                        minute: 0,
                        second: 0,
                        nanos: 0,
                        zone: Some(Zone::Offset(0)),
                    })),
                    _ => None,
                }
            } else {
                let h = int(&a[0])?;
                let m = int(&a[1])?;
                let sn = num(&a[2])?;
                let (s, nanos) = split_seconds(sn)?;
                let zone = if sig == 2 {
                    match a[3].as_ref()? {
                        Value::DtDuration(o) => {
                            if o.0 % NANOS_PER_SEC != 0 {
                                return None;
                            }
                            let secs = o.0 / NANOS_PER_SEC;
                            if secs.abs() > 18 * 3600 {
                                return None;
                            }
                            Some(Zone::Offset(secs as i32))
                        }
                        Value::Null => None,
                        _ => return None,
                    }
                } else {
                    None
                };
                Time::new(
                    u32::try_from(h).ok()?,
                    u32::try_from(m).ok()?,
                    s,
                    nanos,
                    zone,
                )
                .map(Value::Time)
            }
        }
        "number" => {
            let s = string(&a[0])?;
            let (g, d) = if sig == 0 {
                (
                    a[1].clone().unwrap_or(Value::Null),
                    a[2].clone().unwrap_or(Value::Null),
                )
            } else {
                (Value::Null, Value::Null)
            };
            let g = match &g {
                Value::Null => None,
                Value::String(s) if [" ", ",", "."].contains(&&**s) => Some(s.to_string()),
                _ => return None,
            };
            let d = match &d {
                Value::Null => None,
                Value::String(s) if [",", "."].contains(&&**s) => Some(s.to_string()),
                _ => return None,
            };
            if g.is_some() && g == d {
                return None;
            }
            let mut t = s.to_string();
            if let Some(g) = &g {
                t = t.replace(g.as_str(), "");
            }
            if let Some(d) = &d {
                t = t.replace(d.as_str(), ".");
            }
            Number::parse(&t).map(Value::Number)
        }
        "string" => {
            let v = a[0].as_ref()?;
            if v.is_null() {
                return None;
            }
            Some(Value::str(&to_feel_string(v)))
        }
        "duration" => match a[0].as_ref()? {
            Value::String(s) => match temporal::parse_duration(s)? {
                temporal::Duration::Dt(d) => Some(Value::DtDuration(d)),
                temporal::Duration::Ym(d) => Some(Value::YmDuration(d)),
            },
            Value::DtDuration(d) => Some(Value::DtDuration(*d)),
            Value::YmDuration(d) => Some(Value::YmDuration(*d)),
            _ => None,
        },
        "years and months duration" => {
            let from = date_of(a[0].as_ref()?)?;
            let to = date_of(a[1].as_ref()?)?;
            Some(Value::YmDuration(YmDuration(months_between(&from, &to))))
        }
        // ---- boolean
        "not" => match a[0].as_ref()? {
            Value::Bool(b) => Some(Value::Bool(!b)),
            _ => None,
        },
        "is" => {
            let x = a[0].as_ref()?;
            let y = a[1].as_ref()?;
            Some(Value::Bool(is_same(x, y)))
        }
        // ---- strings
        "substring" => {
            let s = string(&a[0])?;
            let start = num(&a[1])?.trunc_i64()?;
            let chars: Vec<char> = s.chars().collect();
            let len = chars.len() as i64;
            let begin = if start > 0 {
                start - 1
            } else if start < 0 {
                len + start
            } else {
                return None;
            };
            if begin < 0 {
                return None;
            }
            if begin > len {
                return Some(Value::str(""));
            }
            let end = if sig == 1 {
                let l = num(&a[2])?.trunc_i64()?;
                if l < 0 {
                    return None;
                }
                (begin + l).min(len)
            } else {
                len
            };
            Some(Value::str(
                &chars[begin as usize..end as usize]
                    .iter()
                    .collect::<String>(),
            ))
        }
        "string length" => Some(Value::int(string(&a[0])?.chars().count() as i64)),
        "upper case" => Some(Value::str(&string(&a[0])?.to_uppercase())),
        "lower case" => Some(Value::str(&string(&a[0])?.to_lowercase())),
        "substring before" => {
            let s = string(&a[0])?;
            let m = string(&a[1])?;
            Some(Value::str(match s.find(m) {
                Some(i) => &s[..i],
                None => "",
            }))
        }
        "substring after" => {
            let s = string(&a[0])?;
            let m = string(&a[1])?;
            Some(Value::str(match s.find(m) {
                Some(i) => &s[i + m.len()..],
                None => "",
            }))
        }
        "contains" => Some(bool_v(string(&a[0])?.contains(string(&a[1])?))),
        "starts with" => Some(bool_v(string(&a[0])?.starts_with(string(&a[1])?))),
        "ends with" => Some(bool_v(string(&a[0])?.ends_with(string(&a[1])?))),
        "matches" => {
            let input = string(&a[0])?;
            let pattern = string(&a[1])?;
            let flags = if sig == 1 { flags_arg(&a[2])? } else { "" };
            let re = translate_regex(pattern, flags)?;
            re.is_match(input).ok().map(Value::Bool)
        }
        "replace" => {
            let input = string(&a[0])?;
            let pattern = string(&a[1])?;
            let repl = string(&a[2])?;
            let flags = if sig == 1 { flags_arg(&a[3])? } else { "" };
            let re = translate_regex(pattern, flags)?;
            if re.is_match("").ok()? {
                return None;
            }
            let repl = if flags.contains('q') {
                repl.replace('$', "$$")
            } else {
                translate_replacement(repl)?
            };
            let mut out = String::new();
            let mut last = 0;
            for caps in re.captures_iter(input) {
                let caps = caps.ok()?;
                let m = caps.get(0)?;
                out.push_str(&input[last..m.start()]);
                caps.expand(&repl, &mut out);
                last = m.end();
            }
            out.push_str(&input[last..]);
            Some(Value::str(&out))
        }
        "split" => {
            let s = string(&a[0])?;
            let d = string(&a[1])?;
            let flags = if sig == 1 { flags_arg(&a[2])? } else { "" };
            let re = translate_regex(d, flags)?;
            if re.is_match("").ok()? {
                return None;
            }
            let mut out = Vec::new();
            let mut last = 0;
            for m in re.find_iter(s) {
                let m = m.ok()?;
                out.push(Value::str(&s[last..m.start()]));
                last = m.end();
            }
            out.push(Value::str(&s[last..]));
            Some(Value::list(out))
        }
        "string join" => {
            let l = list_arg(&a[0])?;
            let delim = if sig == 1 {
                match a[1].as_ref()? {
                    Value::String(s) => s.to_string(),
                    Value::Null => String::new(),
                    _ => return None,
                }
            } else {
                String::new()
            };
            let mut parts = Vec::new();
            for v in l.iter() {
                match v {
                    Value::String(s) => parts.push(s.to_string()),
                    Value::Null => {}
                    _ => return None,
                }
            }
            Some(Value::str(&parts.join(&delim)))
        }
        // ---- lists
        "list contains" => {
            let l = list_arg(&a[0])?;
            let e = a[1].clone().unwrap_or(Value::Null);
            Some(bool_v(l.iter().any(|x| feel_eq(x, &e) == Some(true))))
        }
        "count" => Some(Value::int(list_arg(&a[0])?.len() as i64)),
        "min" | "max" => {
            let items = varargs(a, sig)?;
            if items.is_empty() {
                return None;
            }
            let mut best = items[0].clone();
            if best.is_null() {
                return None;
            }
            for x in &items[1..] {
                let o = feel_cmp(x, &best)?;
                if (name == "min" && o == Ordering::Less)
                    || (name == "max" && o == Ordering::Greater)
                {
                    best = x.clone();
                }
            }
            Some(best)
        }
        "sum" => {
            let items = varargs(a, sig)?;
            if items.is_empty() {
                return None;
            }
            let mut s = Number::zero();
            for x in &items {
                s = s.add(x.as_number()?);
            }
            Some(Value::Number(s))
        }
        "mean" => {
            let items = varargs(a, sig)?;
            if items.is_empty() {
                return None;
            }
            let mut s = Number::zero();
            for x in &items {
                s = s.add(x.as_number()?);
            }
            s.div(&Number::from_i64(items.len() as i64))
                .map(Value::Number)
        }
        "product" => {
            let items = varargs(a, sig)?;
            if items.is_empty() {
                return None;
            }
            let mut s = Number::one();
            for x in &items {
                s = s.mul(x.as_number()?);
            }
            Some(Value::Number(s))
        }
        "median" => {
            let items = varargs(a, sig)?;
            let mut nums: Vec<Number> = items
                .iter()
                .map(|x| x.as_number().cloned())
                .collect::<Option<Vec<_>>>()?;
            if nums.is_empty() {
                return None;
            }
            nums.sort();
            let n = nums.len();
            if n % 2 == 1 {
                Some(Value::Number(nums[n / 2].clone()))
            } else {
                nums[n / 2 - 1]
                    .add(&nums[n / 2])
                    .div(&Number::from_i64(2))
                    .map(Value::Number)
            }
        }
        "stddev" => {
            let items = varargs(a, sig)?;
            let nums: Vec<Number> = items
                .iter()
                .map(|x| x.as_number().cloned())
                .collect::<Option<Vec<_>>>()?;
            if nums.len() < 2 {
                return None;
            }
            let n = Number::from_i64(nums.len() as i64);
            let mut s = Number::zero();
            for x in &nums {
                s = s.add(x);
            }
            let mean = s.div(&n)?;
            let mut sq = Number::zero();
            for x in &nums {
                let d = x.sub(&mean);
                sq = sq.add(&d.mul(&d));
            }
            let var = sq.div(&n.sub(&Number::one()))?;
            var.sqrt().map(Value::Number)
        }
        "mode" => {
            let items = varargs(a, sig)?;
            if items.iter().any(|x| x.is_null()) {
                return None;
            }
            let mut counts: Vec<(Value, usize)> = Vec::new();
            for x in &items {
                match counts.iter_mut().find(|(v, _)| feel_eq(v, x) == Some(true)) {
                    Some(e) => e.1 += 1,
                    None => counts.push((x.clone(), 1)),
                }
            }
            let max = counts.iter().map(|(_, c)| *c).max().unwrap_or(0);
            let mut modes: Vec<Value> = counts
                .into_iter()
                .filter(|(_, c)| *c == max)
                .map(|(v, _)| v)
                .collect();
            let mut err = false;
            modes.sort_by(|x, y| {
                feel_cmp(x, y).unwrap_or_else(|| {
                    err = true;
                    Ordering::Equal
                })
            });
            if err {
                return None;
            }
            Some(Value::list(modes))
        }
        "all" | "any" => {
            let items = match varargs(a, sig) {
                Some(i) => i,
                None => {
                    if sig == 1 && a.is_empty() {
                        return None;
                    }
                    return None;
                }
            };
            let target = name == "any";
            let mut has_null = false;
            for x in &items {
                match x {
                    Value::Bool(b) if *b == target => return Some(Value::Bool(target)),
                    Value::Bool(_) => {}
                    _ => has_null = true,
                }
            }
            if has_null {
                None
            } else {
                Some(Value::Bool(!target))
            }
        }
        "sublist" => {
            let l = list_arg(&a[0])?;
            let start = int(&a[1])?;
            let len = l.len() as i64;
            let begin = if start > 0 {
                start - 1
            } else if start < 0 {
                len + start
            } else {
                return None;
            };
            if begin < 0 || begin >= len {
                return None;
            }
            let end = if sig == 1 {
                let n = int(&a[2])?;
                if n < 0 || begin + n > len {
                    return None;
                }
                begin + n
            } else {
                len
            };
            Some(Value::list(l[begin as usize..end as usize].to_vec()))
        }
        "append" => {
            let mut l = match a[0].as_ref()? {
                Value::List(l) => l.as_ref().clone(),
                _ => return None,
            };
            for x in &a[1..] {
                l.push(x.clone().unwrap_or(Value::Null));
            }
            Some(Value::list(l))
        }
        "concatenate" => {
            let mut out = Vec::new();
            for x in a {
                match x.as_ref()? {
                    Value::List(l) => out.extend(l.iter().cloned()),
                    _ => return None,
                }
            }
            Some(Value::list(out))
        }
        "insert before" => {
            let mut l = list_arg(&a[0])?.as_ref().clone();
            let p = int(&a[1])?;
            let len = l.len() as i64;
            let idx = if p > 0 {
                p - 1
            } else if p < 0 {
                len + p
            } else {
                return None;
            };
            // the position must refer to an existing element
            if idx < 0 || idx >= len {
                return None;
            }
            l.insert(idx as usize, a[2].clone().unwrap_or(Value::Null));
            Some(Value::list(l))
        }
        "remove" => {
            let mut l = list_arg(&a[0])?.as_ref().clone();
            let p = int(&a[1])?;
            let len = l.len() as i64;
            let idx = if p > 0 {
                p - 1
            } else if p < 0 {
                len + p
            } else {
                return None;
            };
            if idx < 0 || idx >= len {
                return None;
            }
            l.remove(idx as usize);
            Some(Value::list(l))
        }
        "reverse" => {
            let mut l = list_arg(&a[0])?.as_ref().clone();
            l.reverse();
            Some(Value::list(l))
        }
        "index of" => {
            let l = list_arg(&a[0])?;
            let m = a[1].clone().unwrap_or(Value::Null);
            Some(Value::list(
                l.iter()
                    .enumerate()
                    .filter(|(_, x)| feel_eq(x, &m) == Some(true))
                    .map(|(i, _)| Value::int(i as i64 + 1))
                    .collect(),
            ))
        }
        "union" => {
            let mut out: Vec<Value> = Vec::new();
            for x in a {
                let items = match x.as_ref()? {
                    Value::List(l) => l.as_ref().clone(),
                    v => vec![v.clone()],
                };
                for i in items {
                    if !out.iter().any(|o| feel_eq(o, &i) == Some(true)) {
                        out.push(i);
                    }
                }
            }
            Some(Value::list(out))
        }
        "distinct values" => {
            let l = list_arg(&a[0])?;
            let mut out: Vec<Value> = Vec::new();
            for i in l.iter() {
                if !out.iter().any(|o| feel_eq(o, i) == Some(true)) {
                    out.push(i.clone());
                }
            }
            Some(Value::list(out))
        }
        "flatten" => {
            let l = list_arg(&a[0])?;
            let mut out = Vec::new();
            fn flat(v: &Value, out: &mut Vec<Value>) {
                match v {
                    Value::List(l) => l.iter().for_each(|x| flat(x, out)),
                    x => out.push(x.clone()),
                }
            }
            l.iter().for_each(|x| flat(x, &mut out));
            Some(Value::list(out))
        }
        "sort" => {
            let mut l = list_arg(&a[0])?.as_ref().clone();
            if sig == 1 {
                let f = match a[1].as_ref()? {
                    Value::Function(f) => f.clone(),
                    _ => return None,
                };
                let mut err = false;
                // insertion sort with user comparator (stable, tolerant of inconsistent comparators)
                let mut sorted: Vec<Value> = Vec::with_capacity(l.len());
                for x in l {
                    let mut pos = sorted.len();
                    for (i, y) in sorted.iter().enumerate() {
                        match call(&f, vec![x.clone(), y.clone()], scope) {
                            Value::Bool(true) => {
                                pos = i;
                                break;
                            }
                            Value::Bool(false) => {}
                            _ => {
                                err = true;
                                break;
                            }
                        }
                    }
                    sorted.insert(pos, x);
                }
                if err {
                    return None;
                }
                Some(Value::list(sorted))
            } else {
                let mut err = false;
                l.sort_by(|x, y| {
                    feel_cmp(x, y).unwrap_or_else(|| {
                        err = true;
                        Ordering::Equal
                    })
                });
                if err {
                    return None;
                }
                Some(Value::list(l))
            }
        }
        "list replace" => {
            let mut l = list_arg(&a[0])?.as_ref().clone();
            let new_item = a[2].clone().unwrap_or(Value::Null);
            match a[1].as_ref()? {
                Value::Number(n) => {
                    let p = n.trunc_i64()?;
                    let len = l.len() as i64;
                    let idx = if p > 0 {
                        p - 1
                    } else if p < 0 {
                        len + p
                    } else {
                        return None;
                    };
                    if idx < 0 || idx >= len {
                        return None;
                    }
                    l[idx as usize] = new_item;
                    Some(Value::list(l))
                }
                Value::Function(f) => {
                    let f = f.clone();
                    for item in l.iter_mut() {
                        match call(&f, vec![item.clone(), new_item.clone()], scope) {
                            Value::Bool(true) => *item = new_item.clone(),
                            Value::Bool(false) => {}
                            _ => return None,
                        }
                    }
                    Some(Value::list(l))
                }
                _ => None,
            }
        }
        // ---- numbers
        "decimal" => {
            let n = num(&a[0])?;
            let s = scale(&a[1])?;
            Some(Value::Number(n.round_scale(s, Rounding::HalfEven)))
        }
        "floor" | "ceiling" => {
            let n = num(&a[0])?;
            let s = if sig == 1 { scale(&a[1])? } else { 0 };
            Some(Value::Number(n.round_scale(
                s,
                if name == "floor" {
                    Rounding::Floor
                } else {
                    Rounding::Ceiling
                },
            )))
        }
        "round up" | "round down" | "round half up" | "round half down" => {
            let n = num(&a[0])?;
            let s = if sig == 1 { scale(&a[1])? } else { 0 };
            let mode = match name {
                "round up" => Rounding::Up,
                "round down" => Rounding::Down,
                "round half up" => Rounding::HalfUp,
                _ => Rounding::HalfDown,
            };
            Some(Value::Number(n.round_scale(s, mode)))
        }
        "abs" => match a[0].as_ref()? {
            Value::Number(n) => Some(Value::Number(n.abs())),
            Value::DtDuration(d) => Some(Value::DtDuration(DtDuration(d.0.abs()))),
            Value::YmDuration(d) => Some(Value::YmDuration(YmDuration(d.0.abs()))),
            _ => None,
        },
        "modulo" => num(&a[0])?.modulo(num(&a[1])?).map(Value::Number),
        "sqrt" => num(&a[0])?.sqrt().map(Value::Number),
        "log" => num(&a[0])?.ln().map(Value::Number),
        "exp" => num(&a[0])?.exp_fn().map(Value::Number),
        "odd" | "even" => {
            let n = num(&a[0])?;
            if !n.is_integer() {
                return None;
            }
            let two = Number::from_i64(2);
            let r = n.modulo(&two)?;
            let is_even = r.is_zero();
            Some(Value::Bool(if name == "even" { is_even } else { !is_even }))
        }
        // ---- dates
        "day of year" => Some(Value::int(date_of(a[0].as_ref()?)?.day_of_year() as i64)),
        "day of week" => {
            let d = date_of(a[0].as_ref()?)?;
            let names = [
                "Monday",
                "Tuesday",
                "Wednesday",
                "Thursday",
                "Friday",
                "Saturday",
                "Sunday",
            ];
            Some(Value::str(names[d.weekday() as usize - 1]))
        }
        "month of year" => {
            let d = date_of(a[0].as_ref()?)?;
            let names = [
                "January",
                "February",
                "March",
                "April",
                "May",
                "June",
                "July",
                "August",
                "September",
                "October",
                "November",
                "December",
            ];
            Some(Value::str(names[d.month as usize - 1]))
        }
        "week of year" => Some(Value::int(date_of(a[0].as_ref()?)?.week_of_year() as i64)),
        "now" => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .ok()?;
            let nanos = now.as_nanos() as i128;
            let nanos = nanos - nanos % 1_000_000;
            DateTime::from_instant(nanos, Zone::Offset(0)).map(Value::DateTime)
        }
        "today" => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .ok()?;
            let days = (now.as_secs() / 86400) as i64;
            Date::from_days(days).map(Value::Date)
        }
        // ---- ranges
        "before" | "after" | "meets" | "met by" | "overlaps" | "overlaps before"
        | "overlaps after" | "finishes" | "finished by" | "includes" | "during" | "starts"
        | "started by" | "coincides" => {
            range_fn(name, sig, a[0].as_ref()?, a[1].as_ref()?).map(Value::Bool)
        }
        // ---- contexts
        "get value" => {
            let m = match a[0].as_ref()? {
                Value::Context(c) => c.clone(),
                _ => return None,
            };
            match a[1].as_ref()? {
                Value::String(k) => m.get(k).cloned(),
                Value::List(keys) => {
                    let mut cur = Value::Context(m);
                    for k in keys.iter() {
                        cur = match (&cur, k) {
                            (Value::Context(c), Value::String(k)) => c.get(k)?.clone(),
                            _ => return None,
                        };
                    }
                    Some(cur)
                }
                _ => None,
            }
        }
        "get entries" => {
            let m = match a[0].as_ref()? {
                Value::Context(c) => c.clone(),
                _ => return None,
            };
            Some(Value::list(
                m.entries()
                    .iter()
                    .map(|(k, v)| {
                        let mut c = Context::new();
                        c.put("key", Value::String(k.clone()));
                        c.put("value", v.clone());
                        Value::context(c)
                    })
                    .collect(),
            ))
        }
        "context" => {
            let l = match a[0].as_ref()? {
                Value::List(l) => l.as_ref().clone(),
                Value::Context(c) => vec![Value::Context(c.clone())],
                _ => return None,
            };
            let mut out = Context::new();
            for e in &l {
                let c = match e {
                    Value::Context(c) => c,
                    _ => return None,
                };
                let k = match c.get("key")? {
                    Value::String(s) => s.clone(),
                    _ => return None,
                };
                if !c.contains("value") {
                    return None;
                }
                if out.contains(&k) {
                    return None;
                }
                out.put(k, c.get("value")?.clone());
            }
            Some(Value::context(out))
        }
        "context put" => {
            let ctx = match a[0].as_ref()? {
                Value::Context(c) => c.clone(),
                _ => return None,
            };
            let value = a[2].clone().unwrap_or(Value::Null);
            let keys: Vec<Rc<str>> = match a[1].as_ref()? {
                Value::String(s) => vec![s.clone()],
                Value::List(l) => {
                    if l.is_empty() {
                        return None;
                    }
                    l.iter()
                        .map(|k| k.as_str().map(Rc::from))
                        .collect::<Option<Vec<_>>>()?
                }
                _ => return None,
            };
            fn put(c: &Context, keys: &[Rc<str>], v: Value) -> Option<Context> {
                let mut out = c.clone();
                if keys.len() == 1 {
                    out.put(keys[0].clone(), v);
                    return Some(out);
                }
                let inner = match c.get(&keys[0]) {
                    Some(Value::Context(ic)) => ic.as_ref().clone(),
                    None => Context::new(),
                    _ => return None,
                };
                let ni = put(&inner, &keys[1..], v)?;
                out.put(keys[0].clone(), Value::context(ni));
                Some(out)
            }
            put(&ctx, &keys, value).map(Value::context)
        }
        "context merge" => {
            let l = match a[0].as_ref()? {
                Value::List(l) => l.as_ref().clone(),
                Value::Context(c) => vec![Value::Context(c.clone())],
                _ => return None,
            };
            let mut out = Context::new();
            for e in &l {
                match e {
                    Value::Context(c) => {
                        for (k, v) in c.entries() {
                            out.put(k.clone(), v.clone());
                        }
                    }
                    _ => return None,
                }
            }
            Some(Value::context(out))
        }
        "range" => {
            let s = string(&a[0])?;
            parse_range_string(s, scope)
        }
        _ => None,
    }
}

fn scale(v: &Option<Value>) -> Option<i64> {
    let s = num(v)?.trunc_i64()?;
    if !(-6111..=6176).contains(&s) {
        return None;
    }
    Some(s)
}

fn split_seconds(n: &Number) -> Option<(u32, u32)> {
    if n.is_negative() {
        return None;
    }
    let whole = n.floor();
    let s = whole.to_i64()?;
    let frac = n
        .sub(&whole)
        .mul(&Number::from_i64(1_000_000_000))
        .round_scale(0, Rounding::Down);
    Some((u32::try_from(s).ok()?, frac.to_i64()? as u32))
}

fn date_of(v: &Value) -> Option<Date> {
    match v {
        Value::Date(d) => Some(*d),
        Value::DateTime(dt) => Some(dt.date),
        _ => None,
    }
}

fn months_between(from: &Date, to: &Date) -> i64 {
    let mut months = (to.year - from.year) * 12 + to.month as i64 - from.month as i64;
    if months > 0 && to.day < from.day {
        months -= 1;
    } else if months < 0 && to.day > from.day {
        months += 1;
    }
    months
}

fn is_same(x: &Value, y: &Value) -> bool {
    match (x, y) {
        (Value::Time(a), Value::Time(b)) => a == b,
        (Value::DateTime(a), Value::DateTime(b)) => a == b,
        (Value::Date(a), Value::Date(b)) => a == b,
        (Value::DtDuration(a), Value::DtDuration(b)) => a == b,
        (Value::YmDuration(a), Value::YmDuration(b)) => a == b,
        (Value::Null, Value::Null) => true,
        _ => {
            if std::mem::discriminant(x) != std::mem::discriminant(y) {
                return false;
            }
            feel_eq(x, y) == Some(true)
        }
    }
}

fn parse_range_string(s: &str, scope: &Rc<Scope>) -> Option<Value> {
    use super::parser::{EmptyEnv, parse_expression};
    let e = parse_expression(s.trim(), &EmptyEnv).ok()?;
    // only literal ranges / unary comparisons with literal endpoints are allowed
    fn literal_ok(e: &super::ast::Expr) -> bool {
        use super::ast::Expr;
        match e {
            Expr::Const(_) => true,
            Expr::Call(f, args) => {
                matches!(&**f, Expr::Name(n) if ["date", "time", "date and time", "duration"].contains(&&**n))
                    && match args {
                        super::ast::Args::Positional(a) => {
                            a.iter().all(|x| matches!(x, Expr::Const(Value::String(_))))
                        }
                        _ => false,
                    }
            }
            _ => false,
        }
    }
    match &e {
        super::ast::Expr::Range { start, end, .. } => {
            if !literal_ok(start) || !literal_ok(end) {
                return None;
            }
        }
        _ => return None,
    }
    let v = super::eval::eval(&e, scope);
    match v {
        Value::Range(_) => Some(v),
        _ => None,
    }
}

// ---------------------------------------------------------------- ranges

#[derive(Clone, Copy)]
enum Ext<'a> {
    NegInf,
    Val(&'a Value),
    PosInf,
}

fn ext_cmp(a: Ext, b: Ext) -> Option<Ordering> {
    match (a, b) {
        (Ext::NegInf, Ext::NegInf) | (Ext::PosInf, Ext::PosInf) => Some(Ordering::Equal),
        (Ext::NegInf, _) | (_, Ext::PosInf) => Some(Ordering::Less),
        (_, Ext::NegInf) | (Ext::PosInf, _) => Some(Ordering::Greater),
        (Ext::Val(x), Ext::Val(y)) => {
            if let Some(true) = feel_eq(x, y) {
                return Some(Ordering::Equal);
            }
            feel_cmp(x, y)
        }
    }
}

struct R<'a> {
    s: Ext<'a>,
    si: bool,
    e: Ext<'a>,
    ei: bool,
}

fn as_range(v: &Value) -> Option<R<'_>> {
    match v {
        Value::Range(r) => {
            if matches!(r.unary, Some(CmpOp::Ne)) {
                return None;
            }
            Some(R {
                s: r.low().map(Ext::Val).unwrap_or(Ext::NegInf),
                si: r.start_incl,
                e: r.high().map(Ext::Val).unwrap_or(Ext::PosInf),
                ei: r.end_incl,
            })
        }
        _ => None,
    }
}

fn range_fn(name: &str, sig: usize, x: &Value, y: &Value) -> Option<bool> {
    use Ordering::*;
    let is_r = |v: &Value| matches!(v, Value::Range(_));
    // determine the actual shape from the values (positional calls)
    let shape = match (is_r(x), is_r(y)) {
        (false, false) => 0,
        (false, true) => 1,
        (true, false) => 2,
        (true, true) => 3,
    };
    // named calls must match the declared shape
    if sig != shape && sig != 0 {
        return None;
    }
    if x.is_null() || y.is_null() {
        return None;
    }
    let c = |a: Ext, b: Ext| ext_cmp(a, b);
    match shape {
        0 => {
            let o = c(Ext::Val(x), Ext::Val(y))?;
            match name {
                "before" => Some(o == Less),
                "after" => Some(o == Greater),
                "coincides" => Some(o == Equal),
                _ => None,
            }
        }
        1 => {
            // point, range
            let p = Ext::Val(x);
            let r = as_range(y)?;
            match name {
                "before" => Some(c(p, r.s)? == Less || (c(p, r.s)? == Equal && !r.si)),
                "after" => Some(c(p, r.e)? == Greater || (c(p, r.e)? == Equal && !r.ei)),
                "finishes" => Some(r.ei && c(r.e, p)? == Equal),
                "during" => Some(
                    (c(r.s, p)? == Less && c(r.e, p)? == Greater)
                        || (c(r.s, p)? == Equal && r.si)
                        || (c(r.e, p)? == Equal && r.ei),
                ),
                "starts" => Some(c(r.s, p)? == Equal && r.si),
                _ => None,
            }
        }
        2 => {
            // range, point
            let r = as_range(x)?;
            let p = Ext::Val(y);
            match name {
                "before" => Some(c(r.e, p)? == Less || (c(r.e, p)? == Equal && !r.ei)),
                "after" => Some(c(r.s, p)? == Greater || (c(r.s, p)? == Equal && !r.si)),
                "finished by" => Some(r.ei && c(r.e, p)? == Equal),
                "includes" => Some(
                    (c(r.s, p)? == Less && c(r.e, p)? == Greater)
                        || (c(r.s, p)? == Equal && r.si)
                        || (c(r.e, p)? == Equal && r.ei),
                ),
                "started by" => Some(c(r.s, p)? == Equal && r.si),
                _ => None,
            }
        }
        _ => {
            let a = as_range(x)?;
            let b = as_range(y)?;
            match name {
                "before" => {
                    Some(c(a.e, b.s)? == Less || ((!a.ei || !b.si) && c(a.e, b.s)? == Equal))
                }
                "after" => {
                    Some(c(a.s, b.e)? == Greater || ((!a.si || !b.ei) && c(a.s, b.e)? == Equal))
                }
                "meets" => Some(a.ei && b.si && c(a.e, b.s)? == Equal),
                "met by" => Some(a.si && b.ei && c(a.s, b.e)? == Equal),
                "overlaps" => Some(
                    (c(a.e, b.s)? == Greater || (c(a.e, b.s)? == Equal && a.ei && b.si))
                        && (c(a.s, b.e)? == Less || (c(a.s, b.e)? == Equal && a.si && b.ei)),
                ),
                "overlaps before" => Some(
                    (c(a.s, b.s)? == Less || (c(a.s, b.s)? == Equal && a.si && !b.si))
                        && (c(a.e, b.s)? == Greater || (c(a.e, b.s)? == Equal && a.ei && b.si))
                        && (c(a.e, b.e)? == Less || (c(a.e, b.e)? == Equal && (!a.ei || b.ei))),
                ),
                "overlaps after" => Some(
                    (c(b.s, a.s)? == Less || (c(b.s, a.s)? == Equal && b.si && !a.si))
                        && (c(b.e, a.s)? == Greater || (c(b.e, a.s)? == Equal && b.ei && a.si))
                        && (c(b.e, a.e)? == Less || (c(b.e, a.e)? == Equal && (!b.ei || a.ei))),
                ),
                "finishes" => Some(
                    a.ei == b.ei
                        && c(a.e, b.e)? == Equal
                        && (c(a.s, b.s)? == Greater || (c(a.s, b.s)? == Equal && (!a.si || b.si))),
                ),
                "finished by" => Some(
                    a.ei == b.ei
                        && c(a.e, b.e)? == Equal
                        && (c(a.s, b.s)? == Less || (c(a.s, b.s)? == Equal && (a.si || !b.si))),
                ),
                "includes" => Some(
                    (c(a.s, b.s)? == Less || (c(a.s, b.s)? == Equal && (a.si || !b.si)))
                        && (c(a.e, b.e)? == Greater || (c(a.e, b.e)? == Equal && (a.ei || !b.ei))),
                ),
                "during" => Some(
                    (c(b.s, a.s)? == Less || (c(b.s, a.s)? == Equal && (b.si || !a.si)))
                        && (c(b.e, a.e)? == Greater || (c(b.e, a.e)? == Equal && (b.ei || !a.ei))),
                ),
                "starts" => Some(
                    c(a.s, b.s)? == Equal
                        && a.si == b.si
                        && (c(b.e, a.e)? == Greater || (c(b.e, a.e)? == Equal && (!a.ei || b.ei))),
                ),
                "started by" => Some(
                    c(a.s, b.s)? == Equal
                        && a.si == b.si
                        && (c(a.e, b.e)? == Greater || (c(a.e, b.e)? == Equal && (!b.ei || a.ei))),
                ),
                "coincides" => Some(
                    c(a.s, b.s)? == Equal && a.si == b.si && c(a.e, b.e)? == Equal && a.ei == b.ei,
                ),
                _ => None,
            }
        }
    }
}

// ---------------------------------------------------------------- external (Java) functions

fn java_number(v: &Value, ty: &str) -> Option<f64> {
    let n = v.as_number()?;
    match ty {
        "int" | "long" | "short" | "byte" | "java.lang.Integer" | "java.lang.Long" => {
            if !n.is_integer() {
                return None;
            }
            Some(n.to_f64())
        }
        "double" | "float" | "java.lang.Double" | "java.lang.Float" => Some(n.to_f64()),
        _ => None,
    }
}

fn from_f64(f: f64) -> Value {
    Number::from_f64(f)
        .map(Value::Number)
        .unwrap_or(Value::Null)
}

fn from_f32(f: f64) -> Value {
    // emulate float -> BigDecimal conversion through the shortest float representation
    let s = format!("{}", f as f32);
    Number::parse(&s).map(Value::Number).unwrap_or(Value::Null)
}

pub fn call_external(spec: &Value, args: Vec<Value>) -> Value {
    let java = match spec {
        Value::Context(c) => match c.get("java") {
            Some(Value::Context(j)) => j.clone(),
            _ => return Value::Null,
        },
        _ => return Value::Null,
    };
    let class = match java.get("class") {
        Some(Value::String(s)) => s.to_string(),
        _ => return Value::Null,
    };
    let sig = match java.get("method signature") {
        Some(Value::String(s)) => s.to_string(),
        _ => return Value::Null,
    };
    let open = match sig.find('(') {
        Some(i) => i,
        None => return Value::Null,
    };
    let method = sig[..open].trim().to_string();
    let params: Vec<String> = sig[open + 1..sig.rfind(')').unwrap_or(sig.len())]
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let ptypes: Vec<&str> = params.iter().map(|s| s.as_str()).collect();
    match (class.as_str(), method.as_str(), ptypes.as_slice()) {
        ("java.lang.Math", m, [t]) if ["int", "long", "double", "float"].contains(t) => {
            let x = match args.first().and_then(|a| java_number(a, t)) {
                Some(x) => x,
                None => return Value::Null,
            };
            let r = match m {
                "abs" => x.abs(),
                "cos" => x.cos(),
                "sin" => x.sin(),
                "tan" => x.tan(),
                "acos" => x.acos(),
                "asin" => x.asin(),
                "atan" => x.atan(),
                "sqrt" => x.sqrt(),
                "cbrt" => x.cbrt(),
                "exp" => x.exp(),
                "log" => x.ln(),
                "log10" => x.log10(),
                "floor" => x.floor(),
                "ceil" => x.ceil(),
                "signum" => x.signum(),
                "toDegrees" => x.to_degrees(),
                "toRadians" => x.to_radians(),
                "round" => x.round(),
                _ => return Value::Null,
            };
            from_f64(r)
        }
        ("java.lang.Math", m, [t1, t2])
            if t1 == t2 && ["int", "long", "double", "float"].contains(t1) =>
        {
            let x = match args.first().and_then(|a| java_number(a, t1)) {
                Some(x) => x,
                None => return Value::Null,
            };
            let y = match args.get(1).and_then(|a| java_number(a, t2)) {
                Some(x) => x,
                None => return Value::Null,
            };
            let r = match m {
                "max" => x.max(y),
                "min" => x.min(y),
                "pow" => x.powf(y),
                "atan2" => x.atan2(y),
                "hypot" => x.hypot(y),
                _ => return Value::Null,
            };
            if *t1 == "float" {
                from_f32(r)
            } else {
                from_f64(r)
            }
        }
        ("java.lang.Short", "valueOf", ["short"])
        | ("java.lang.Byte", "valueOf", ["byte"])
        | ("java.lang.Integer", "valueOf", ["int"])
        | ("java.lang.Long", "valueOf", ["long"]) => match args.first() {
            Some(Value::Number(n)) if n.is_integer() => Value::Number(n.clone()),
            _ => Value::Null,
        },
        ("java.lang.String", "valueOf", ["char"]) => match args.first() {
            Some(Value::String(s)) if s.chars().count() == 1 => Value::String(s.clone()),
            _ => Value::Null,
        },
        ("java.lang.String", "valueOf", [_]) => match args.first() {
            Some(v) if !v.is_null() => Value::str(&to_feel_string(v)),
            _ => Value::Null,
        },
        ("java.lang.Integer", "valueOf", ["java.lang.String"])
        | ("java.lang.Integer", "parseInt", ["java.lang.String"])
        | ("java.lang.Long", "valueOf", ["java.lang.String"]) => match args.first() {
            Some(Value::String(s)) => s
                .trim()
                .parse::<i64>()
                .map(Value::int)
                .unwrap_or(Value::Null),
            _ => Value::Null,
        },
        ("java.lang.Float", "valueOf", ["java.lang.String"]) => match args.first() {
            Some(Value::String(s)) => s
                .trim()
                .parse::<f32>()
                .map(|f| from_f32(f as f64))
                .unwrap_or(Value::Null),
            _ => Value::Null,
        },
        ("java.lang.Double", "valueOf", ["java.lang.String"])
        | ("java.lang.Double", "parseDouble", ["java.lang.String"]) => match args.first() {
            Some(Value::String(s)) => s.trim().parse::<f64>().map(from_f64).unwrap_or(Value::Null),
            _ => Value::Null,
        },
        ("java.lang.String", "format", ["java.lang.String", "[Ljava.lang.Object;"]) => {
            let fmt = match args.first() {
                Some(Value::String(s)) => s.to_string(),
                _ => return Value::Null,
            };
            let rest: Vec<Value> = match args.get(1) {
                Some(Value::List(l)) => l.as_ref().clone(),
                Some(v) => vec![v.clone()],
                None => vec![],
            };
            let mut out = String::new();
            let mut it = rest.iter();
            let mut chars = fmt.chars().peekable();
            while let Some(c) = chars.next() {
                if c == '%' {
                    match chars.next() {
                        Some('s') | Some('d') => match it.next() {
                            Some(v) => out.push_str(&to_feel_string(v)),
                            None => return Value::Null,
                        },
                        Some('%') => out.push('%'),
                        Some('n') => out.push('\n'),
                        _ => return Value::Null,
                    }
                } else {
                    out.push(c);
                }
            }
            Value::str(&out)
        }
        _ => Value::Null,
    }
}
