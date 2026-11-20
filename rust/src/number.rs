//! Arbitrary precision decimal number following FEEL semantics.
//!
//! FEEL numbers are modelled after IEEE 754-2008 decimal128: 34 significant
//! digits with round-half-even. The value is `coef * 10^exp`, kept
//! normalized (no trailing zeros in the coefficient).

use num_bigint::{BigInt, Sign};
use num_integer::Integer;
use num_traits::{One, Signed, ToPrimitive, Zero};
use std::cmp::Ordering;
use std::fmt;

pub const PRECISION: u64 = 34;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Number {
    coef: BigInt,
    exp: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rounding {
    HalfEven,
    HalfUp,
    HalfDown,
    /// Away from zero.
    Up,
    /// Towards zero.
    Down,
    Floor,
    Ceiling,
}

fn pow10(n: u64) -> BigInt {
    num_traits::pow(BigInt::from(10u32), n as usize)
}

fn ndigits(n: &BigInt) -> u64 {
    if n.is_zero() {
        return 1;
    }
    let m = n.magnitude();
    if let Some(v) = m.to_u64() {
        return if v == 0 { 1 } else { (v.ilog10() + 1) as u64 };
    }
    // estimate using bits then adjust
    let bits = m.bits();
    let mut est = ((bits as f64 - 1.0) * std::f64::consts::LOG10_2).floor() as u64 + 1;
    let p = pow10(est);
    if BigInt::from(m.clone()) >= p {
        est += 1;
    }
    // est may be over by one in rare cases
    if est > 1 && BigInt::from(m.clone()) < pow10(est - 1) {
        est -= 1;
    }
    est
}

/// Decide whether to increment the truncated quotient.
fn round_increment(
    q: &BigInt,
    r: &BigInt,
    divisor: &BigInt,
    sticky: bool,
    negative: bool,
    mode: Rounding,
) -> bool {
    // q, r, divisor are magnitudes (non-negative)
    if r.is_zero() && !sticky {
        return false;
    }
    let twice: BigInt = r * 2u32;
    let cmp = match twice.cmp(divisor) {
        Ordering::Equal if sticky => Ordering::Greater,
        o => o,
    };
    match mode {
        Rounding::HalfEven => match cmp {
            Ordering::Greater => true,
            Ordering::Less => false,
            Ordering::Equal => q.is_odd(),
        },
        Rounding::HalfUp => cmp != Ordering::Less,
        Rounding::HalfDown => cmp == Ordering::Greater,
        Rounding::Up => true,
        Rounding::Down => false,
        Rounding::Floor => negative,
        Rounding::Ceiling => !negative,
    }
}

impl Number {
    fn make(coef: BigInt, exp: i64) -> Number {
        Self::make_sticky(coef, exp, false)
    }

    /// Normalize: round to PRECISION significant digits and strip trailing zeros.
    fn make_sticky(coef: BigInt, exp: i64, sticky: bool) -> Number {
        if coef.is_zero() {
            return Number { coef, exp: 0 };
        }
        let negative = coef.sign() == Sign::Minus;
        let mut mag = coef.abs();
        let mut exp = exp;
        let d = ndigits(&mag);
        if d > PRECISION || sticky {
            let k = d.saturating_sub(PRECISION);
            let divisor = pow10(k);
            let (q, r) = mag.div_rem(&divisor);
            let inc = round_increment(&q, &r, &divisor, sticky, negative, Rounding::HalfEven);
            mag = if inc { q + 1u32 } else { q };
            exp += k as i64;
        }
        // strip trailing zeros
        if !mag.is_zero() {
            let ten = BigInt::from(10u32);
            loop {
                let (q, r) = mag.div_rem(&ten);
                if r.is_zero() {
                    mag = q;
                    exp += 1;
                } else {
                    break;
                }
            }
        } else {
            exp = 0;
        }
        let coef = if negative { -mag } else { mag };
        Number { coef, exp }
    }

    pub fn zero() -> Number {
        Number {
            coef: BigInt::zero(),
            exp: 0,
        }
    }

    pub fn one() -> Number {
        Number {
            coef: BigInt::one(),
            exp: 0,
        }
    }

    pub fn from_i64(v: i64) -> Number {
        Self::make(BigInt::from(v), 0)
    }

    pub fn from_i128(v: i128) -> Number {
        Self::make(BigInt::from(v), 0)
    }

    pub fn from_bigint(v: BigInt) -> Number {
        Self::make(v, 0)
    }

    pub fn from_parts(coef: BigInt, exp: i64) -> Number {
        Self::make(coef, exp)
    }

    /// Parse a FEEL / XML number literal: `[+-]digits[.digits][e[+-]digits]`.
    pub fn parse(s: &str) -> Option<Number> {
        let s = s.trim();
        let bytes = s.as_bytes();
        let mut i = 0;
        let mut negative = false;
        if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
            negative = bytes[i] == b'-';
            i += 1;
        }
        let mut digits = String::new();
        let mut frac_len: i64 = 0;
        let mut seen_digit = false;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            digits.push(bytes[i] as char);
            i += 1;
            seen_digit = true;
        }
        if i < bytes.len() && bytes[i] == b'.' {
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                digits.push(bytes[i] as char);
                frac_len += 1;
                i += 1;
                seen_digit = true;
            }
        }
        if !seen_digit {
            return None;
        }
        let mut exp: i64 = 0;
        if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
            i += 1;
            let mut eneg = false;
            if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
                eneg = bytes[i] == b'-';
                i += 1;
            }
            let start = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if start == i {
                return None;
            }
            exp = s[start..i].parse::<i64>().ok()?;
            if eneg {
                exp = -exp;
            }
        }
        if i != bytes.len() {
            return None;
        }
        let mut coef: BigInt = digits.parse().ok()?;
        if negative {
            coef = -coef;
        }
        Some(Self::make(coef, exp - frac_len))
    }

    pub fn from_f64(f: f64) -> Option<Number> {
        if !f.is_finite() {
            return None;
        }
        let s = format!("{:e}", f);
        Number::parse(&s)
    }

    pub fn to_f64(&self) -> f64 {
        self.to_string().parse::<f64>().unwrap_or(f64::NAN)
    }

    pub fn is_zero(&self) -> bool {
        self.coef.is_zero()
    }

    pub fn is_negative(&self) -> bool {
        self.coef.sign() == Sign::Minus
    }

    pub fn is_positive(&self) -> bool {
        self.coef.sign() == Sign::Plus
    }

    pub fn is_integer(&self) -> bool {
        self.exp >= 0
    }

    pub fn exponent(&self) -> i64 {
        self.exp
    }

    /// Integer value if this number is integral and fits.
    pub fn to_i64(&self) -> Option<i64> {
        if !self.is_integer() {
            return None;
        }
        if self.exp > 40 {
            return None;
        }
        (&self.coef * pow10(self.exp as u64)).to_i64()
    }

    pub fn to_i128(&self) -> Option<i128> {
        if !self.is_integer() {
            return None;
        }
        if self.exp > 40 {
            return None;
        }
        (&self.coef * pow10(self.exp as u64)).to_i128()
    }

    /// Truncate towards zero and convert.
    pub fn trunc_i64(&self) -> Option<i64> {
        self.round_scale(0, Rounding::Down).to_i64()
    }

    pub fn to_bigint(&self) -> Option<BigInt> {
        if !self.is_integer() || self.exp > 100_000 {
            return None;
        }
        Some(&self.coef * pow10(self.exp as u64))
    }

    /// Adjusted exponent: exponent of the most significant digit.
    fn adjusted(&self) -> i64 {
        self.exp + ndigits(&self.coef) as i64 - 1
    }

    pub fn neg(&self) -> Number {
        Number {
            coef: -self.coef.clone(),
            exp: self.exp,
        }
    }

    pub fn abs(&self) -> Number {
        Number {
            coef: self.coef.abs(),
            exp: self.exp,
        }
    }

    pub fn add(&self, other: &Number) -> Number {
        if self.is_zero() {
            return other.clone();
        }
        if other.is_zero() {
            return self.clone();
        }
        // If the magnitudes are wildly apart the smaller one cannot affect the
        // rounded result (apart from sticky behaviour, which is negligible).
        let a1 = self.adjusted();
        let a2 = other.adjusted();
        if a1 - a2 > PRECISION as i64 + 3 {
            return self.clone();
        }
        if a2 - a1 > PRECISION as i64 + 3 {
            return other.clone();
        }
        let e = self.exp.min(other.exp);
        let c1 = &self.coef * pow10((self.exp - e) as u64);
        let c2 = &other.coef * pow10((other.exp - e) as u64);
        Self::make(c1 + c2, e)
    }

    pub fn sub(&self, other: &Number) -> Number {
        self.add(&other.neg())
    }

    pub fn mul(&self, other: &Number) -> Number {
        Self::make(&self.coef * &other.coef, self.exp + other.exp)
    }

    pub fn div(&self, other: &Number) -> Option<Number> {
        if other.is_zero() {
            return None;
        }
        if self.is_zero() {
            return Some(Number::zero());
        }
        let negative = self.is_negative() != other.is_negative();
        let a = self.coef.abs();
        let b = other.coef.abs();
        let da = ndigits(&a) as i64;
        let db = ndigits(&b) as i64;
        let shift = (PRECISION as i64 + 2 + db - da).max(0);
        let num = a * pow10(shift as u64);
        let (q, r) = num.div_rem(&b);
        let sticky = !r.is_zero();
        let exp = self.exp - other.exp - shift;
        let q = if negative { -q } else { q };
        Some(Self::make_sticky(q, exp, sticky))
    }

    /// Integer division rounding towards negative infinity.
    pub fn floor_div(&self, other: &Number) -> Option<Number> {
        let q = self.div(other)?;
        Some(q.round_scale(0, Rounding::Floor))
    }

    /// FEEL modulo: dividend - divisor * floor(dividend / divisor).
    pub fn modulo(&self, other: &Number) -> Option<Number> {
        if other.is_zero() {
            return None;
        }
        // exact computation: align exponents
        let e = self.exp.min(other.exp);
        if (self.exp - e) > 10000 || (other.exp - e) > 10000 {
            let q = self.floor_div(other)?;
            return Some(self.sub(&other.mul(&q)));
        }
        let c1 = &self.coef * pow10((self.exp - e) as u64);
        let c2 = &other.coef * pow10((other.exp - e) as u64);
        let r = c1.mod_floor(&c2);
        Some(Self::make(r, e))
    }

    fn compare(&self, other: &Number) -> Ordering {
        let s1 = self.coef.sign();
        let s2 = other.coef.sign();
        let sign_ord = |s: Sign| match s {
            Sign::Minus => -1,
            Sign::NoSign => 0,
            Sign::Plus => 1,
        };
        let (o1, o2) = (sign_ord(s1), sign_ord(s2));
        if o1 != o2 {
            return o1.cmp(&o2);
        }
        if o1 == 0 {
            return Ordering::Equal;
        }
        let a1 = self.adjusted();
        let a2 = other.adjusted();
        if a1 != a2 {
            let o = a1.cmp(&a2);
            return if o1 < 0 { o.reverse() } else { o };
        }
        let e = self.exp.min(other.exp);
        let c1 = &self.coef * pow10((self.exp - e) as u64);
        let c2 = &other.coef * pow10((other.exp - e) as u64);
        c1.cmp(&c2)
    }

    /// Round to the given number of decimal places (`scale`).
    pub fn round_scale(&self, scale: i64, mode: Rounding) -> Number {
        if self.exp >= -scale {
            return self.clone();
        }
        let k = (-scale - self.exp) as u64;
        let negative = self.is_negative();
        let mag = self.coef.abs();
        if k > ndigits(&mag) + 2 {
            // the whole number is below the rounding position
            let inc = round_increment(
                &BigInt::zero(),
                &BigInt::one(),
                &pow10(4),
                false,
                negative,
                mode,
            );
            // r = 1/10000 of divisor -> less than half
            let mag = if inc { BigInt::one() } else { BigInt::zero() };
            let coef = if negative { -mag } else { mag };
            return Self::make(coef, -scale);
        }
        let divisor = pow10(k);
        let (q, r) = mag.div_rem(&divisor);
        let inc = round_increment(&q, &r, &divisor, false, negative, mode);
        let q = if inc { q + 1u32 } else { q };
        let coef = if negative { -q } else { q };
        Self::make(coef, -scale)
    }

    pub fn floor(&self) -> Number {
        self.round_scale(0, Rounding::Floor)
    }

    pub fn ceiling(&self) -> Number {
        self.round_scale(0, Rounding::Ceiling)
    }

    /// Integer power.
    pub fn pow_int(&self, n: i64) -> Option<Number> {
        if n == 0 {
            return Some(Number::one());
        }
        if self.is_zero() {
            return if n < 0 { None } else { Some(Number::zero()) };
        }
        let mag = n.unsigned_abs();
        let pos = if mag <= 2000 {
            let c = num_traits::pow(self.coef.clone(), mag as usize);
            Self::make(c, self.exp * mag as i64)
        } else {
            // exponentiation by squaring with rounding
            let mut result = Number::one();
            let mut base = self.clone();
            let mut e = mag;
            while e > 0 {
                if e & 1 == 1 {
                    result = result.mul(&base);
                }
                base = base.mul(&base);
                e >>= 1;
            }
            result
        };
        if n < 0 {
            Number::one().div(&pos)
        } else {
            Some(pos)
        }
    }

    pub fn sqrt(&self) -> Option<Number> {
        if self.is_negative() {
            return None;
        }
        if self.is_zero() {
            return Some(Number::zero());
        }
        // value = coef * 10^exp; make exp even and coef have >= 2*(P+2) digits
        let mut coef = self.coef.clone();
        let mut exp = self.exp;
        let d = ndigits(&coef) as i64;
        let mut shift = (2 * (PRECISION as i64 + 2) - d).max(0);
        if (exp - shift) % 2 != 0 {
            shift += 1;
        }
        coef *= pow10(shift as u64);
        exp -= shift;
        let root = coef.sqrt();
        let exact = &root * &root == coef;
        Some(Self::make_sticky(root, exp / 2, !exact))
    }

    pub fn exp_fn(&self) -> Option<Number> {
        Number::from_f64(self.to_f64().exp())
    }

    pub fn ln(&self) -> Option<Number> {
        if !self.is_positive() {
            return None;
        }
        Number::from_f64(self.to_f64().ln())
    }

    pub fn pow_f64(&self, other: &Number) -> Option<Number> {
        Number::from_f64(self.to_f64().powf(other.to_f64()))
    }

    pub fn coefficient(&self) -> &BigInt {
        &self.coef
    }

    /// Plain (non scientific) string representation.
    pub fn to_plain_string(&self) -> String {
        let neg = self.is_negative();
        let digits = self.coef.abs().to_string();
        let mut s = String::new();
        if neg {
            s.push('-');
        }
        if self.exp >= 0 {
            s.push_str(&digits);
            if !self.coef.is_zero() {
                for _ in 0..self.exp.min(100_000) {
                    s.push('0');
                }
            }
        } else {
            let frac = (-self.exp) as usize;
            if digits.len() > frac {
                let (i, f) = digits.split_at(digits.len() - frac);
                s.push_str(i);
                s.push('.');
                s.push_str(f);
            } else {
                s.push_str("0.");
                for _ in 0..(frac - digits.len()) {
                    s.push('0');
                }
                s.push_str(&digits);
            }
        }
        s
    }
}

impl fmt::Display for Number {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_plain_string())
    }
}

impl PartialOrd for Number {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Number {
    fn cmp(&self, other: &Self) -> Ordering {
        self.compare(other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(s: &str) -> Number {
        Number::parse(s).unwrap()
    }

    #[test]
    fn parse_and_print() {
        assert_eq!(n("1.10").to_string(), "1.1");
        assert_eq!(n("-0").to_string(), "0");
        assert_eq!(n("1.23e4").to_string(), "12300");
        assert_eq!(n("1.23e-4").to_string(), "0.000123");
        assert_eq!(n(".5").to_string(), "0.5");
    }

    #[test]
    fn arithmetic() {
        assert_eq!(
            n("1").div(&n("3")).unwrap().to_string(),
            "0.3333333333333333333333333333333333"
        );
        assert_eq!(
            n("2").div(&n("3")).unwrap().to_string(),
            "0.6666666666666666666666666666666667"
        );
        assert_eq!(n("0.1").add(&n("0.2")).to_string(), "0.3");
        assert_eq!(n("10").pow_int(-2).unwrap().to_string(), "0.01");
        assert_eq!(n("16").sqrt().unwrap().to_string(), "4");
        assert_eq!(
            n("2").sqrt().unwrap().to_string(),
            "1.414213562373095048801688724209698"
        );
        assert_eq!(n("-5").modulo(&n("3")).unwrap().to_string(), "1");
        assert_eq!(n("5").modulo(&n("-3")).unwrap().to_string(), "-1");
    }

    #[test]
    fn rounding() {
        assert_eq!(n("2.5").round_scale(0, Rounding::HalfEven).to_string(), "2");
        assert_eq!(n("-2.5").round_scale(0, Rounding::HalfUp).to_string(), "-3");
        assert_eq!(n("-2.5").round_scale(0, Rounding::Floor).to_string(), "-3");
        assert_eq!(n("1.5").round_scale(0, Rounding::Ceiling).to_string(), "2");
        assert_eq!(n("0.001").round_scale(0, Rounding::Up).to_string(), "1");
        assert_eq!(
            n("12345").round_scale(-2, Rounding::HalfEven).to_string(),
            "12300"
        );
    }
}
