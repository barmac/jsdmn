//! Decimal number following FEEL semantics.
//!
//! FEEL numbers are modelled after IEEE 754-2008 decimal128: 34 significant
//! digits with round-half-even. The value is `coef * 10^exp`, kept
//! normalized (no trailing zeros in the coefficient, zero has exponent 0).
//!
//! Because every result is rounded to 34 digits the coefficient always fits
//! in an `i128`, so numbers are small `Copy` values. Intermediate results
//! that overflow 128 bits fall back to arbitrary precision (`num-bigint`).

use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Signed, ToPrimitive, Zero};
use std::cmp::Ordering;
use std::fmt;

pub const PRECISION: u32 = 34;

/// 10^34: exclusive upper bound of coefficient magnitudes.
const COEF_LIMIT: u128 = 10_000_000_000_000_000_000_000_000_000_000_000;

/// `coef * 10^exp`; the 128 bit coefficient is stored as two 64 bit halves so
/// that the struct only needs 8 byte alignment (24 bytes in total).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Number {
    lo: u64,
    hi: i64,
    e: i32,
}

impl fmt::Debug for Number {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_plain_string())
    }
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

const POW10: [u128; 39] = {
    let mut t = [1u128; 39];
    let mut i = 1;
    while i < 39 {
        t[i] = t[i - 1] * 10;
        i += 1;
    }
    t
};

#[inline]
fn pow10_u128(n: u32) -> Option<u128> {
    POW10.get(n as usize).copied()
}

#[inline]
fn digits_u128(v: u128) -> u32 {
    if v <= u64::MAX as u128 {
        let v = v as u64;
        return if v == 0 { 1 } else { v.ilog10() + 1 };
    }
    // log10 estimate from the bit length, corrected with the power table
    let bits = 128 - v.leading_zeros();
    let t = (bits * 1233) >> 12;
    t + (v >= POW10[t as usize]) as u32
}

/// `v % 10` using 64 bit arithmetic (2^64 = 6 mod 10).
#[inline]
fn mod10_u128(v: u128) -> u32 {
    let hi = (v >> 64) as u64;
    let lo = v as u64;
    (((hi % 10) * 6 + lo % 10) % 10) as u32
}

fn pow10_big(n: u64) -> BigInt {
    thread_local! {
        static CACHE: Vec<BigInt> = {
            let mut v = Vec::with_capacity(160);
            let mut p = BigInt::one();
            for _ in 0..160 {
                v.push(p.clone());
                p *= 10u32;
            }
            v
        };
    }
    if n < 160 {
        return CACHE.with(|c| c[n as usize].clone());
    }
    num_traits::pow(BigInt::from(10u32), n as usize)
}

fn digits_big(n: &BigInt) -> u64 {
    if n.is_zero() {
        return 1;
    }
    if let Some(v) = n.magnitude().to_u128() {
        return digits_u128(v) as u64;
    }
    // estimate from the bit length, then correct by one if needed
    let bits = n.magnitude().bits();
    let est = ((bits - 1) as f64 * std::f64::consts::LOG10_2).floor() as u64 + 1;
    let m = BigInt::from(n.magnitude().clone());
    if m >= pow10_big(est) {
        est + 1
    } else if est > 1 && m < pow10_big(est - 1) {
        est - 1
    } else {
        est
    }
}

/// Truncate a coefficient to at most `w` digits (returns the dropped digit count
/// and whether anything non-zero was dropped).
fn truncate_big(c: BigInt, w: u64) -> (BigInt, u64, bool) {
    let d = digits_big(&c);
    if d <= w {
        return (c, 0, false);
    }
    let k = d - w;
    let (q, r) = c.div_rem(&pow10_big(k));
    (q, k, !r.is_zero())
}

/// Decide whether to increment a truncated quotient.
/// `cmp_half` compares the discarded remainder with half a unit.
#[inline]
fn round_up(
    q_odd: bool,
    cmp_half: Ordering,
    inexact: bool,
    negative: bool,
    mode: Rounding,
) -> bool {
    if !inexact {
        return false;
    }
    match mode {
        Rounding::HalfEven => match cmp_half {
            Ordering::Greater => true,
            Ordering::Less => false,
            Ordering::Equal => q_odd,
        },
        Rounding::HalfUp => cmp_half != Ordering::Less,
        Rounding::HalfDown => cmp_half == Ordering::Greater,
        Rounding::Up => true,
        Rounding::Down => false,
        Rounding::Floor => negative,
        Rounding::Ceiling => !negative,
    }
}

/// Split `mag` by `10^k` and round the quotient.
#[inline]
fn round_div_u128(mag: u128, k: u32, sticky: bool, negative: bool, mode: Rounding) -> u128 {
    if k == 0 {
        return mag;
    }
    let (q, r) = match pow10_u128(k) {
        Some(d) => {
            let (q, r) = if mag <= u64::MAX as u128 {
                let (m, d) = (mag as u64, d as u64);
                ((m / d) as u128, (m % d) as u128)
            } else {
                let q = mag / d;
                (q, mag - q * d)
            };
            let half = d / 2;
            let cmp = match r.cmp(&half) {
                Ordering::Equal if sticky => Ordering::Greater,
                o => o,
            };
            let inexact = r != 0 || sticky;
            return if round_up(q & 1 == 1, cmp, inexact, negative, mode) {
                q + 1
            } else {
                q
            };
        }
        None => (0u128, mag),
    };
    // k > 38: everything is discarded and the remainder is below half
    let inexact = r != 0 || sticky;
    if round_up(false, Ordering::Less, inexact, negative, mode) {
        q + 1
    } else {
        q
    }
}

impl Number {
    #[inline]
    const fn raw(coef: i128, exp: i64) -> Number {
        let e = if exp > i32::MAX as i64 {
            i32::MAX
        } else if exp < i32::MIN as i64 {
            i32::MIN
        } else {
            exp as i32
        };
        Number {
            lo: coef as u64,
            hi: (coef >> 64) as i64,
            e,
        }
    }

    #[inline]
    fn coef(&self) -> i128 {
        ((self.hi as i128) << 64) | self.lo as i128
    }

    #[inline]
    fn exp(&self) -> i64 {
        self.e as i64
    }

    /// Build from an exact magnitude/sign/exponent, rounding to 34 digits.
    #[inline]
    fn from_mag(mag: u128, negative: bool, exp: i64, sticky: bool) -> Number {
        let mut mag = mag;
        let mut exp = exp;
        if mag >= COEF_LIMIT || sticky {
            let d = digits_u128(mag);
            let k = d.saturating_sub(PRECISION);
            mag = round_div_u128(mag, k, sticky, negative, Rounding::HalfEven);
            exp += k as i64;
            if mag >= COEF_LIMIT {
                // rounding carried into a 35th digit (it is now a power of ten)
                mag /= 10;
                exp += 1;
            }
        }
        if mag == 0 {
            return Number::raw(0, 0);
        }
        // strip trailing zeros (64 bit arithmetic is much faster than 128 bit)
        if mag <= u64::MAX as u128 {
            let mut m = mag as u64;
            while m.is_multiple_of(10) {
                m /= 10;
                exp += 1;
            }
            mag = m as u128;
        } else {
            while mod10_u128(mag) == 0 {
                mag /= 10;
                exp += 1;
            }
        }
        let coef = mag as i128;
        Number::raw(if negative { -coef } else { coef }, exp)
    }

    #[inline]
    fn from_i128_exp(v: i128, exp: i64) -> Number {
        Self::from_mag(v.unsigned_abs(), v < 0, exp, false)
    }

    /// Build from an arbitrary precision coefficient.
    fn from_big(coef: BigInt, exp: i64, sticky: bool) -> Number {
        if let Some(v) = coef.to_i128() {
            return Self::from_mag(v.unsigned_abs(), v < 0, exp, sticky);
        }
        let negative = coef.is_negative();
        let mag = coef.abs();
        let d = digits_big(&mag);
        let k = d - PRECISION as u64;
        let divisor = pow10_big(k);
        let (q, r) = mag.div_rem(&divisor);
        let twice: BigInt = &r * 2u32;
        let cmp = match twice.cmp(&divisor) {
            Ordering::Equal if sticky => Ordering::Greater,
            o => o,
        };
        let inexact = !r.is_zero() || sticky;
        let q = q.to_u128().expect("34 digit quotient");
        let q = if round_up(q & 1 == 1, cmp, inexact, negative, Rounding::HalfEven) {
            q + 1
        } else {
            q
        };
        Self::from_mag(q, negative, exp + k as i64, false)
    }

    fn to_big(self) -> BigInt {
        BigInt::from(self.coef())
    }

    pub const fn zero() -> Number {
        Number::raw(0, 0)
    }

    pub const fn one() -> Number {
        Number::raw(1, 0)
    }

    pub fn from_i64(v: i64) -> Number {
        Self::from_i128_exp(v as i128, 0)
    }

    pub fn from_i128(v: i128) -> Number {
        Self::from_i128_exp(v, 0)
    }

    pub fn from_bigint(v: BigInt) -> Number {
        Self::from_big(v, 0, false)
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
        let mut mag: u128 = 0;
        let mut big: Option<String> = None;
        let mut frac_len: i64 = 0;
        let mut seen_digit = false;
        let push = |c: u8, mag: &mut u128, big: &mut Option<String>| {
            if let Some(b) = big {
                b.push(c as char);
                return;
            }
            match mag
                .checked_mul(10)
                .and_then(|m| m.checked_add((c - b'0') as u128))
            {
                Some(m) => *mag = m,
                None => {
                    let mut b = mag.to_string();
                    b.push(c as char);
                    *big = Some(b);
                }
            }
        };
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            push(bytes[i], &mut mag, &mut big);
            i += 1;
            seen_digit = true;
        }
        if i < bytes.len() && bytes[i] == b'.' {
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                push(bytes[i], &mut mag, &mut big);
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
        match big {
            None => Some(Self::from_mag(mag, negative, exp - frac_len, false)),
            Some(digits) => {
                let mut coef: BigInt = digits.parse().ok()?;
                if negative {
                    coef = -coef;
                }
                Some(Self::from_big(coef, exp - frac_len, false))
            }
        }
    }

    pub fn from_f64(f: f64) -> Option<Number> {
        if !f.is_finite() {
            return None;
        }
        let s = format!("{:e}", f);
        Number::parse(&s)
    }

    pub fn to_f64(&self) -> f64 {
        if self.exp() == 0 {
            return self.coef() as f64;
        }
        self.to_string().parse::<f64>().unwrap_or(f64::NAN)
    }

    #[inline]
    pub fn is_zero(&self) -> bool {
        self.coef() == 0
    }

    #[inline]
    pub fn is_negative(&self) -> bool {
        self.coef() < 0
    }

    #[inline]
    pub fn is_positive(&self) -> bool {
        self.coef() > 0
    }

    #[inline]
    pub fn is_integer(&self) -> bool {
        self.exp() >= 0
    }

    pub fn exponent(&self) -> i64 {
        self.exp()
    }

    /// Integer value if this number is integral and fits.
    #[inline]
    pub fn to_i128(&self) -> Option<i128> {
        if self.exp() < 0 {
            return None;
        }
        if self.exp() == 0 {
            return Some(self.coef());
        }
        let p = pow10_u128(u32::try_from(self.exp()).ok()?)?;
        self.coef().checked_mul(i128::try_from(p).ok()?)
    }

    #[inline]
    pub fn to_i64(&self) -> Option<i64> {
        self.to_i128().and_then(|v| i64::try_from(v).ok())
    }

    /// Truncate towards zero and convert.
    pub fn trunc_i64(&self) -> Option<i64> {
        self.round_scale(0, Rounding::Down).to_i64()
    }

    pub fn to_bigint(&self) -> Option<BigInt> {
        if !self.is_integer() || self.exp() > 100_000 {
            return None;
        }
        Some(self.to_big() * pow10_big(self.exp() as u64))
    }

    #[inline]
    fn digits(&self) -> u32 {
        digits_u128(self.coef().unsigned_abs())
    }

    /// Adjusted exponent: exponent of the most significant digit.
    #[inline]
    fn adjusted(&self) -> i64 {
        self.exp() + self.digits() as i64 - 1
    }

    #[inline]
    pub fn neg(&self) -> Number {
        Number::raw(-self.coef(), self.exp())
    }

    #[inline]
    pub fn abs(&self) -> Number {
        Number::raw(self.coef().abs(), self.exp())
    }

    pub fn add(&self, other: &Number) -> Number {
        if self.coef() == 0 {
            return *other;
        }
        if other.coef() == 0 {
            return *self;
        }
        if self.exp() == other.exp() {
            // |coef| < 10^34 so the sum cannot overflow
            return Self::from_i128_exp(self.coef() + other.coef(), self.exp());
        }
        let (hi, lo) = if self.exp() > other.exp() {
            (self, other)
        } else {
            (other, self)
        };
        // If the magnitudes are wildly apart the smaller one cannot affect the
        // rounded result.
        let a1 = hi.adjusted();
        let a2 = lo.adjusted();
        if a1 - a2 > PRECISION as i64 + 3 {
            return *hi;
        }
        if a2 - a1 > PRECISION as i64 + 3 {
            return *lo;
        }
        let shift = (hi.exp() - lo.exp()) as u32;
        if let Some(p) = pow10_u128(shift)
            && let Some(scaled) = hi.coef().checked_mul(p as i128)
            && let Some(sum) = scaled.checked_add(lo.coef())
        {
            return Self::from_i128_exp(sum, lo.exp());
        }
        // Aligning overflows, so |hi| > 10^4 |lo|: keep three guard digits
        // below hi's precision and fold the rest of lo into a sticky bit.
        let e2 = a1 - (PRECISION as i64 + 2);
        let hmag = hi.coef().unsigned_abs() * POW10[(hi.exp() - e2) as usize];
        let k = (e2 - lo.exp()) as u64;
        let lm = lo.coef().unsigned_abs();
        let (lq, inexact) = if k > 38 {
            (0, lm != 0)
        } else {
            let d = POW10[k as usize];
            (lm / d, lm % d != 0)
        };
        let negative = hi.coef() < 0;
        let mag = if negative == (lo.coef() < 0) {
            hmag + lq
        } else {
            // the dropped digits make the true magnitude slightly smaller
            hmag - lq - inexact as u128
        };
        Self::from_mag(mag, negative, e2, inexact)
    }

    #[inline]
    pub fn sub(&self, other: &Number) -> Number {
        self.add(&other.neg())
    }

    pub fn mul(&self, other: &Number) -> Number {
        if self.coef() == 0 || other.coef() == 0 {
            return Number::zero();
        }
        let exp = self.exp() + other.exp();
        match self.coef().checked_mul(other.coef()) {
            Some(c) => Self::from_i128_exp(c, exp),
            None => {
                let negative = (self.coef() < 0) != (other.coef() < 0);
                let (hi, lo) = mul_u128(self.coef().unsigned_abs(), other.coef().unsigned_abs());
                Self::from_u256(hi, lo, negative, exp)
            }
        }
    }

    /// Build from a 256-bit magnitude `hi * 2^128 + lo`.
    fn from_u256(hi: u128, lo: u128, negative: bool, exp: i64) -> Number {
        if hi == 0 {
            return Self::from_mag(lo, negative, exp, false);
        }
        // lower bound of the digit count; reduce to 35 or 36 digits
        let bits = 256 - hi.leading_zeros();
        let est = ((bits - 1) as f64 * std::f64::consts::LOG10_2 - 1e-9).floor() as u32 + 1;
        let k = est - (PRECISION + 1);
        let mut limbs = [lo as u64, (lo >> 64) as u64, hi as u64, (hi >> 64) as u64];
        let mut sticky = false;
        let mut left = k;
        while left > 0 {
            let step = left.min(19);
            sticky |= div_limbs(&mut limbs, POW10[step as usize] as u64) != 0;
            left -= step;
        }
        debug_assert!(limbs[2] == 0 && limbs[3] == 0);
        let q = ((limbs[1] as u128) << 64) | limbs[0] as u128;
        Self::from_mag(q, negative, exp + k as i64, sticky)
    }

    pub fn div(&self, other: &Number) -> Option<Number> {
        if other.coef() == 0 {
            return None;
        }
        if self.coef() == 0 {
            return Some(Number::zero());
        }
        let negative = (self.coef() < 0) != (other.coef() < 0);
        let a = self.coef().unsigned_abs();
        let b = other.coef().unsigned_abs();
        let mut exp = self.exp() - other.exp();
        let mut q = a / b;
        let mut r = a % b;
        let db = digits_u128(b);
        // long division in chunks of decimal digits until 36 digits are known
        while r != 0 {
            let dq = if q == 0 { 0 } else { digits_u128(q) };
            if dq >= PRECISION + 2 {
                break;
            }
            let k = (38 - db).min(PRECISION + 2 - dq).max(1);
            let p = POW10[k as usize];
            let t = r * p;
            q = q * p + t / b;
            r = t % b;
            exp -= k as i64;
        }
        Some(Self::from_mag(q, negative, exp, r != 0))
    }

    /// Integer division rounding towards negative infinity.
    pub fn floor_div(&self, other: &Number) -> Option<Number> {
        let q = self.div(other)?;
        Some(q.round_scale(0, Rounding::Floor))
    }

    /// FEEL modulo: dividend - divisor * floor(dividend / divisor).
    pub fn modulo(&self, other: &Number) -> Option<Number> {
        if other.coef() == 0 {
            return None;
        }
        let e = self.exp().min(other.exp());
        let s1 = (self.exp() - e) as u64;
        let s2 = (other.exp() - e) as u64;
        if s1 <= 38 && s2 <= 38 {
            let c1 = pow10_u128(s1 as u32).and_then(|p| self.coef().checked_mul(p as i128));
            let c2 = pow10_u128(s2 as u32).and_then(|p| other.coef().checked_mul(p as i128));
            if let (Some(c1), Some(c2)) = (c1, c2) {
                let mut r = c1 % c2;
                if r != 0 && ((r < 0) != (c2 < 0)) {
                    r += c2;
                }
                return Some(Self::from_i128_exp(r, e));
            }
        }
        if s1 > 10000 || s2 > 10000 {
            let q = self.floor_div(other)?;
            return Some(self.sub(&other.mul(&q)));
        }
        let c1 = self.to_big() * pow10_big(s1);
        let c2 = other.to_big() * pow10_big(s2);
        Some(Self::from_big(c1.mod_floor(&c2), e, false))
    }

    fn compare(&self, other: &Number) -> Ordering {
        let o1 = self.coef().signum();
        let o2 = other.coef().signum();
        if o1 != o2 {
            return o1.cmp(&o2);
        }
        if o1 == 0 {
            return Ordering::Equal;
        }
        if self.exp() == other.exp() {
            return self.coef().cmp(&other.coef());
        }
        // align by scaling the operand with the larger exponent when it fits
        let d = self.exp() - other.exp();
        if (1..=38).contains(&d) {
            if let Some(c1) = self.coef().checked_mul(POW10[d as usize] as i128) {
                return c1.cmp(&other.coef());
            }
        } else if (-38..0).contains(&d)
            && let Some(c2) = other.coef().checked_mul(POW10[(-d) as usize] as i128)
        {
            return self.coef().cmp(&c2);
        }
        let a1 = self.adjusted();
        let a2 = other.adjusted();
        if a1 != a2 {
            let o = a1.cmp(&a2);
            return if o1 < 0 { o.reverse() } else { o };
        }
        // equal adjusted exponents: scaling the one with the larger exponent
        // keeps it within 34 digits
        if self.exp() > other.exp() {
            let c1 = self.coef() * POW10[(self.exp() - other.exp()) as usize] as i128;
            c1.cmp(&other.coef())
        } else {
            let c2 = other.coef() * POW10[(other.exp() - self.exp()) as usize] as i128;
            self.coef().cmp(&c2)
        }
    }

    /// Round to the given number of decimal places (`scale`).
    pub fn round_scale(&self, scale: i64, mode: Rounding) -> Number {
        if self.exp() >= -scale {
            return *self;
        }
        let k = -scale - self.exp();
        let negative = self.coef() < 0;
        let mag = self.coef().unsigned_abs();
        let k32 = u32::try_from(k).unwrap_or(u32::MAX);
        let q = round_div_u128(mag, k32, false, negative, mode);
        Self::from_mag(q, negative, -scale, false)
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
        if self.coef() == 0 {
            return if n < 0 { None } else { Some(Number::zero()) };
        }
        let mag = n.unsigned_abs();
        let pos = 'pos: {
            if mag <= 128
                && let Ok(m32) = u32::try_from(mag)
                && let Some(c) = self.coef().checked_pow(m32)
            {
                break 'pos Self::from_i128_exp(c, self.exp() * mag as i64);
            }
            if let Some(r) = self.pow_fast(mag) {
                break 'pos r;
            }
            if self.digits() as u64 * mag <= 400 {
                let c = num_traits::pow(self.to_big(), mag as usize);
                break 'pos Self::from_big(c, self.exp() * mag as i64, false);
            }
            // exponentiation by squaring with a 60 digit working precision
            const W: u64 = 60;
            let mut result = (BigInt::one(), 0i64);
            let mut base = (self.to_big(), self.exp());
            let mut inexact = false;
            let mut e = mag;
            while e > 0 {
                if e & 1 == 1 {
                    let (c, k, x) = truncate_big(&result.0 * &base.0, W);
                    result = (c, result.1 + base.1 + k as i64);
                    inexact |= x;
                }
                e >>= 1;
                if e > 0 {
                    let (c, k, x) = truncate_big(&base.0 * &base.0, W);
                    base = (c, 2 * base.1 + k as i64);
                    inexact |= x;
                }
            }
            Self::from_big(result.0, result.1, inexact)
        };
        if n < 0 {
            Number::one().div(&pos)
        } else {
            Some(pos)
        }
    }

    /// Integer power computed with ~57 digit (192 bit) mantissas and a
    /// rigorous error bound; `None` when correct rounding cannot be guaranteed.
    fn pow_fast(&self, n: u64) -> Option<Number> {
        // (mantissa limbs, decimal exponent, relative error bound)
        type Approx = ([u64; 3], i64, f64);
        fn mul(a: &Approx, b: &Approx) -> Option<Approx> {
            let mut p = [0u64; 6];
            for i in 0..3 {
                let mut carry: u128 = 0;
                for j in 0..3 {
                    let t = a.0[i] as u128 * b.0[j] as u128 + p[i + j] as u128 + carry;
                    p[i + j] = t as u64;
                    carry = t >> 64;
                }
                p[i + 3] = carry as u64;
            }
            let mut r = a.2 + b.2 + a.2 * b.2;
            let mut exp = a.1 + b.1;
            let bits = limbs_bits(&p);
            // keep at most 56-57 digits (< 10^57 < 2^192)
            let est =
                ((bits.max(1) - 1) as f64 * std::f64::consts::LOG10_2 - 1e-9).floor() as u32 + 1;
            if est > 56 {
                let k = est - 56;
                let mut inexact = false;
                let mut left = k;
                while left > 0 {
                    let step = left.min(19);
                    inexact |= limbs_div_small(&mut p, POW10[step as usize] as u64) != 0;
                    left -= step;
                }
                exp += k as i64;
                if inexact {
                    r += 1.0 / limbs_f64(&p);
                }
            }
            if p[3] != 0 || p[4] != 0 || p[5] != 0 {
                return None;
            }
            Some(([p[0], p[1], p[2]], exp, r))
        }
        let negative = self.coef() < 0 && n % 2 == 1;
        let c = self.coef().unsigned_abs();
        let mut result: Approx = ([1, 0, 0], 0, 0.0);
        let mut base: Approx = ([c as u64, (c >> 64) as u64, 0], self.exp(), 0.0);
        let mut e = n;
        loop {
            if e & 1 == 1 {
                result = mul(&result, &base)?;
            }
            e >>= 1;
            if e == 0 {
                break;
            }
            base = mul(&base, &base)?;
        }
        let (mut m, exp, r) = result;
        let mut limbs6 = [m[0], m[1], m[2], 0, 0, 0];
        let bits = limbs_bits(&limbs6);
        let d = ((bits.max(1) - 1) as f64 * std::f64::consts::LOG10_2 - 1e-9).floor() as u32 + 1;
        if d <= PRECISION + 1 {
            // small enough to be held exactly in 128 bits
            if m[2] != 0 {
                return None;
            }
            if r != 0.0 {
                return None;
            }
            let v = ((m[1] as u128) << 64) | m[0] as u128;
            return Some(Self::from_mag(v, negative, exp, false));
        }
        // split off k1 low digits so that a 35 or 36 digit quotient remains
        let k1 = d - (PRECISION + 1);
        let mut rem: u128 = 0;
        let mut scale: u128 = 1;
        let mut left = k1;
        while left > 0 {
            let step = left.min(19);
            let rr = limbs_div_small(&mut limbs6, POW10[step as usize] as u64) as u128;
            rem += rr * scale;
            scale *= POW10[step as usize];
            left -= step;
        }
        m = [limbs6[0], limbs6[1], limbs6[2]];
        if m[2] != 0 {
            return None;
        }
        let q = ((m[1] as u128) << 64) | m[0] as u128;
        if r == 0.0 {
            return Some(Self::from_mag(q, negative, exp + k1 as i64, rem != 0));
        }
        // the true value lies within `err` units (of the last kept digit of the
        // full mantissa) of q * 10^k1 + rem; round only if that interval does
        // not contain a rounding boundary of the final 34 digits
        let full = (q as f64) * (scale as f64) + rem as f64;
        let err = r * full * 1.01 + 2.0;
        let dq = digits_u128(q);
        let drop = dq - PRECISION; // 1 or 2 more digits to drop from q
        let p = POW10[drop as usize];
        let (q34, low) = (q / p, q % p);
        let low_full = low * scale + rem; // remainder below the 34 digits
        let half = (p / 2) * scale;
        let diff = low_full.abs_diff(half);
        if (diff as f64) <= err {
            return None;
        }
        let q34 = if low_full > half { q34 + 1 } else { q34 };
        Some(Self::from_mag(
            q34,
            negative,
            exp + (k1 + drop) as i64,
            false,
        ))
    }

    pub fn sqrt(&self) -> Option<Number> {
        if self.coef() < 0 {
            return None;
        }
        if self.coef() == 0 {
            return Some(Number::zero());
        }
        // value = coef * 10^exp; make exp even and coef have >= 2*(P+2) digits
        let mut coef = self.to_big();
        let mut exp = self.exp();
        let d = self.digits() as i64;
        let mut shift = (2 * (PRECISION as i64 + 2) - d).max(0);
        if (exp - shift) % 2 != 0 {
            shift += 1;
        }
        coef *= pow10_big(shift as u64);
        exp -= shift;
        let root = coef.sqrt();
        let exact = &root * &root == coef;
        Some(Self::from_big(root, exp / 2, !exact))
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

    /// Plain (non scientific) string representation.
    pub fn to_plain_string(&self) -> String {
        let neg = self.coef() < 0;
        let digits = self.coef().unsigned_abs().to_string();
        let mut s = String::with_capacity(digits.len() + 4);
        if neg {
            s.push('-');
        }
        if self.exp() >= 0 {
            s.push_str(&digits);
            if self.coef() != 0 {
                for _ in 0..self.exp().min(100_000) {
                    s.push('0');
                }
            }
        } else {
            let frac = (-self.exp()) as usize;
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

/// Number of significant bits of a little endian limb array.
fn limbs_bits(l: &[u64]) -> u32 {
    for i in (0..l.len()).rev() {
        if l[i] != 0 {
            return i as u32 * 64 + (64 - l[i].leading_zeros());
        }
    }
    0
}

/// Approximate value of a little endian limb array.
fn limbs_f64(l: &[u64]) -> f64 {
    l.iter()
        .rev()
        .fold(0.0, |acc, &x| acc * 18446744073709551616.0 + x as f64)
}

/// Divide a little endian number by `d` in place, returning the remainder.
fn limbs_div_small(l: &mut [u64], d: u64) -> u64 {
    let d = d as u128;
    let mut rem: u128 = 0;
    for i in (0..l.len()).rev() {
        let cur = (rem << 64) | l[i] as u128;
        l[i] = (cur / d) as u64;
        rem = cur % d;
    }
    rem as u64
}

/// Divide a little endian 256 bit number by `d` in place, returning the remainder.
fn div_limbs(l: &mut [u64; 4], d: u64) -> u64 {
    let d = d as u128;
    let mut rem: u128 = 0;
    for i in (0..4).rev() {
        let cur = (rem << 64) | l[i] as u128;
        l[i] = (cur / d) as u64;
        rem = cur % d;
    }
    rem as u64
}

/// Full 128 x 128 -> 256 bit multiplication.
fn mul_u128(a: u128, b: u128) -> (u128, u128) {
    let mask = u64::MAX as u128;
    let (a1, a0) = (a >> 64, a & mask);
    let (b1, b0) = (b >> 64, b & mask);
    let p00 = a0 * b0;
    let p01 = a0 * b1;
    let p10 = a1 * b0;
    let p11 = a1 * b1;
    let mid = (p00 >> 64) + (p01 & mask) + (p10 & mask);
    let lo = (p00 & mask) | (mid << 64);
    let hi = p11 + (p01 >> 64) + (p10 >> 64) + (mid >> 64);
    (hi, lo)
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

impl Default for Number {
    fn default() -> Self {
        Number::zero()
    }
}

#[allow(dead_code)]
fn _assert_one(n: &BigInt) -> bool {
    n.is_one()
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
        assert_eq!(
            n("123456789012345678901234567890123456789").to_string(),
            "123456789012345678901234567890123500000"
        );
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
        assert_eq!(n("10").div(&n("4")).unwrap().to_string(), "2.5");
        assert_eq!(
            n("1e30").add(&n("1e-30")).to_string(),
            "1000000000000000000000000000000"
        );
        assert_eq!(
            n("9999999999999999999999999999999999")
                .add(&n("1"))
                .to_string(),
            "10000000000000000000000000000000000"
        );
        let big = n("1234567890123456789012345678901234");
        assert_eq!(
            big.mul(&big).to_string(),
            "1524157875323883675049535156256667000000000000000000000000000000000"
        );
        assert_eq!(
            n("2").pow_int(200).unwrap().to_string(),
            "1606938044258990275541962092341163000000000000000000000000000"
        );
        assert_eq!(n("0.1").cmp(&n("0.10")), Ordering::Equal);
        assert_eq!(n("1.5").cmp(&n("15")), Ordering::Less);
        assert_eq!(n("-1.5").cmp(&n("-15")), Ordering::Greater);
    }

    /// xorshift64* pseudo random numbers
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 >> 12;
            self.0 ^= self.0 << 25;
            self.0 ^= self.0 >> 27;
            self.0.wrapping_mul(0x2545F4914F6CDD1D)
        }
        fn number(&mut self) -> Number {
            let digits = [1, 3, 10, 20, 30, 34, 34, 34][(self.next() % 8) as usize];
            let mut c: i128 = 0;
            for _ in 0..digits {
                c = c * 10 + (self.next() % 10) as i128;
            }
            if self.next().is_multiple_of(2) {
                c = -c;
            }
            let e = (self.next() % 81) as i64 - 40;
            Number::from_i128_exp(c, e)
        }
    }

    fn exact(a: &Number, b: &Number) -> (BigInt, BigInt, i64) {
        let e = a.exp().min(b.exp());
        (
            a.to_big() * pow10_big((a.exp() - e) as u64),
            b.to_big() * pow10_big((b.exp() - e) as u64),
            e,
        )
    }

    #[test]
    fn fast_paths_match_exact_reference() {
        let mut rng = Rng(0x9E3779B97F4A7C15);
        for _ in 0..50_000 {
            let a = rng.number();
            let b = rng.number();
            let (x, y, e) = exact(&a, &b);
            assert_eq!(
                a.add(&b),
                Number::from_big(&x + &y, e, false),
                "{} + {}",
                a,
                b
            );
            assert_eq!(
                a.sub(&b),
                Number::from_big(&x - &y, e, false),
                "{} - {}",
                a,
                b
            );
            assert_eq!(
                a.mul(&b),
                Number::from_big(a.to_big() * b.to_big(), a.exp() + b.exp(), false),
                "{} * {}",
                a,
                b
            );
            if !b.is_zero() {
                // exact quotient digits plus sticky remainder
                let num = a.to_big().abs() * pow10_big(80);
                let den = b.to_big().abs();
                let (q, r) = num.div_rem(&den);
                let q = if a.is_negative() != b.is_negative() {
                    -q
                } else {
                    q
                };
                let expected = Number::from_big(q, a.exp() - b.exp() - 80, !r.is_zero());
                assert_eq!(a.div(&b).unwrap(), expected, "{} / {}", a, b);
            }
        }
    }

    #[test]
    fn powers_match_exact_reference() {
        let mut rng = Rng(0xD1B54A32D192ED03);
        for _ in 0..2_000 {
            let mut a = rng.number();
            if a.digits() > 10 {
                a = a.round_scale(-(a.exp() + a.digits() as i64 - 8), Rounding::Down);
            }
            if a.is_zero() {
                continue;
            }
            let n = (rng.next() % 400) as i64;
            let expected =
                Number::from_big(num_traits::pow(a.to_big(), n as usize), a.exp() * n, false);
            assert_eq!(a.pow_int(n).unwrap(), expected, "{} ** {}", a, n);
        }
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
        assert_eq!(
            n("1e-50").round_scale(2, Rounding::Ceiling).to_string(),
            "0.01"
        );
        assert_eq!(n("1e-50").round_scale(2, Rounding::HalfUp).to_string(), "0");
    }
}
