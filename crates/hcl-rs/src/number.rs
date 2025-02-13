use crate::Error;
use serde::de::{self, Unexpected};
use serde::{forward_to_deserialize_any, ser};
use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::{Add, Div, Mul, Neg, Rem, Sub};

/// Represents an HCL number.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd)]
pub struct Number {
    n: N,
}

#[derive(Clone, Copy)]
enum N {
    /// Represents a positive integer.
    PosInt(u64),
    /// Represents a negative integer.
    NegInt(i64),
    /// Represents a float.
    Float(f64),
}

impl N {
    fn as_i64(&self) -> Option<i64> {
        match *self {
            N::PosInt(n) => {
                if i64::try_from(n).is_ok() {
                    Some(n as i64)
                } else {
                    None
                }
            }
            N::NegInt(n) => Some(n),
            N::Float(_) => None,
        }
    }

    fn as_u64(&self) -> Option<u64> {
        match *self {
            N::PosInt(n) => Some(n),
            N::NegInt(_) | N::Float(_) => None,
        }
    }

    #[allow(clippy::wrong_self_convention)]
    fn to_f64(&self) -> f64 {
        match *self {
            N::PosInt(n) => n as f64,
            N::NegInt(n) => n as f64,
            N::Float(n) => n,
        }
    }
}

impl PartialEq for N {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (N::PosInt(a), N::PosInt(b)) => a == b,
            (N::NegInt(a), N::NegInt(b)) => a == b,
            (N::Float(a), N::Float(b)) => a == b,
            (a, b) => a.to_f64() == b.to_f64(),
        }
    }
}

// N is `Eq` because we ensure that the wrapped f64 is always finite.
impl Eq for N {}

impl PartialOrd for N {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (*self, *other) {
            (N::PosInt(a), N::PosInt(b)) => a.partial_cmp(&b),
            (N::NegInt(a), N::NegInt(b)) => a.partial_cmp(&b),
            (N::Float(a), N::Float(b)) => a.partial_cmp(&b),
            (a, b) => a.to_f64().partial_cmp(&b.to_f64()),
        }
    }
}

impl Hash for N {
    fn hash<H>(&self, h: &mut H)
    where
        H: Hasher,
    {
        // Use the float representation to ensure that 0u64 and 0.0f64 etc. hash to the same value.
        let f = self.to_f64();

        if f == 0.0f64 {
            // There are 2 zero representations, +0 and -0, which
            // compare equal but have different bits. We use the +0 hash
            // for both so that hash(+0) == hash(-0).
            0.0f64.to_bits().hash(h);
        } else {
            f.to_bits().hash(h);
        }
    }
}

impl From<i64> for N {
    fn from(i: i64) -> Self {
        if i < 0 {
            N::NegInt(i)
        } else {
            N::PosInt(i as u64)
        }
    }
}

impl Number {
    /// Creates a new `Number` from a `f64`. Returns `None` if the float is infinite or NaN.
    ///
    /// ```
    /// use hcl::Number;
    ///
    /// assert!(Number::from_f64(42.0).is_some());
    /// assert!(Number::from_f64(f64::NAN).is_none());
    /// assert!(Number::from_f64(f64::INFINITY).is_none());
    /// assert!(Number::from_f64(f64::NEG_INFINITY).is_none());
    /// ```
    pub fn from_f64(f: f64) -> Option<Number> {
        if f.is_finite() {
            Some(Number { n: N::Float(f) })
        } else {
            None
        }
    }
    /// Represents the `Number` as f64 if possible. Returns None otherwise.
    pub fn as_f64(&self) -> Option<f64> {
        Some(self.n.to_f64())
    }

    /// If the `Number` is an integer, represent it as i64 if possible. Returns None otherwise.
    pub fn as_i64(&self) -> Option<i64> {
        self.n.as_i64()
    }

    /// If the `Number` is an integer, represent it as u64 if possible. Returns None otherwise.
    pub fn as_u64(&self) -> Option<u64> {
        self.n.as_u64()
    }

    /// Returns true if the `Number` is a float.
    ///
    /// For any `Number` on which `is_f64` returns true, `as_f64` is guaranteed to return the
    /// float value.
    pub fn is_f64(&self) -> bool {
        match self.n {
            N::Float(_) => true,
            N::PosInt(_) | N::NegInt(_) => false,
        }
    }

    /// Returns true if the `Number` is an integer between `i64::MIN` and `i64::MAX`.
    ///
    /// For any `Number` on which `is_i64` returns true, `as_i64` is guaranteed to return the
    /// integer value.
    pub fn is_i64(&self) -> bool {
        match self.n {
            N::PosInt(v) => i64::try_from(v).is_ok(),
            N::NegInt(_) => true,
            N::Float(_) => false,
        }
    }

    /// Returns true if the `Number` is an integer between zero and `u64::MAX`.
    ///
    /// For any `Number` on which `is_u64` returns true, `as_u64` is guaranteed to return the
    /// integer value.
    pub fn is_u64(&self) -> bool {
        match self.n {
            N::PosInt(_) => true,
            N::NegInt(_) | N::Float(_) => false,
        }
    }

    #[cold]
    pub(crate) fn unexpected(&self) -> Unexpected {
        match self.n {
            N::PosInt(v) => Unexpected::Unsigned(v),
            N::NegInt(v) => Unexpected::Signed(v),
            N::Float(v) => Unexpected::Float(v),
        }
    }
}

macro_rules! impl_from_unsigned {
    ($($ty:ty),*) => {
        $(
            impl From<$ty> for Number {
                fn from(u: $ty) -> Self {
                    Number {
                        n: N::PosInt(u as u64)
                    }
                }
            }
        )*
    };
}

macro_rules! impl_from_signed {
    ($($ty:ty),*) => {
        $(
            impl From<$ty> for Number {
                fn from(i: $ty) -> Self {
                    Number {
                        n: N::from(i as i64)
                    }
                }
            }
        )*
    };
}

impl_from_unsigned!(u8, u16, u32, u64, usize);
impl_from_signed!(i8, i16, i32, i64, isize);

impl fmt::Display for Number {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self.n {
            N::PosInt(u) => formatter.write_str(itoa::Buffer::new().format(u)),
            N::NegInt(i) => formatter.write_str(itoa::Buffer::new().format(i)),
            N::Float(f) => formatter.write_str(ryu::Buffer::new().format_finite(f)),
        }
    }
}

impl fmt::Debug for Number {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "Number({self})")
    }
}

impl ser::Serialize for Number {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        match self.n {
            N::PosInt(i) => serializer.serialize_u64(i),
            N::NegInt(i) => serializer.serialize_i64(i),
            N::Float(f) => serializer.serialize_f64(f),
        }
    }
}

impl<'de> de::Deserialize<'de> for Number {
    fn deserialize<D>(deserializer: D) -> Result<Number, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct NumberVisitor;

        impl<'de> de::Visitor<'de> for NumberVisitor {
            type Value = Number;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("an HCL number")
            }

            fn visit_i64<E>(self, value: i64) -> Result<Number, E> {
                Ok(value.into())
            }

            fn visit_u64<E>(self, value: u64) -> Result<Number, E> {
                Ok(value.into())
            }

            fn visit_f64<E>(self, value: f64) -> Result<Number, E>
            where
                E: de::Error,
            {
                Number::from_f64(value).ok_or_else(|| de::Error::custom("not an HCL number"))
            }
        }

        deserializer.deserialize_any(NumberVisitor)
    }
}

impl<'de> de::Deserializer<'de> for Number {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.n {
            N::PosInt(i) => visitor.visit_u64(i),
            N::NegInt(i) => visitor.visit_i64(i),
            N::Float(f) => visitor.visit_f64(f),
        }
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct enum map struct identifier ignored_any
    }
}

impl Neg for Number {
    type Output = Number;

    fn neg(self) -> Self::Output {
        let n = match self.n {
            N::PosInt(value) => N::NegInt(-(value as i64)),
            N::NegInt(value) => {
                let value = -value;
                if value < 0 {
                    N::NegInt(value)
                } else {
                    N::PosInt(value as u64)
                }
            }
            N::Float(value) => N::Float(-value),
        };

        Number { n }
    }
}

impl Add for Number {
    type Output = Number;

    fn add(self, rhs: Self) -> Self::Output {
        let n = match (self.n, rhs.n) {
            (N::PosInt(a), N::PosInt(b)) => N::PosInt(a + b),
            (N::PosInt(a), N::NegInt(b)) => N::from(a as i64 + b),
            (N::NegInt(a), N::NegInt(b)) => N::from(a + b),
            (N::NegInt(a), N::PosInt(b)) => N::from(a + b as i64),
            (N::Float(a), N::Float(b)) => N::Float(a + b),
            (a, b) => N::Float(a.to_f64() + b.to_f64()),
        };

        Number { n }
    }
}

impl Sub for Number {
    type Output = Number;

    fn sub(self, rhs: Self) -> Self::Output {
        let n = match (self.n, rhs.n) {
            (N::PosInt(a), N::PosInt(b)) => {
                if a < b {
                    N::NegInt(a as i64 - b as i64)
                } else {
                    N::PosInt(a - b)
                }
            }
            (N::PosInt(a), N::NegInt(b)) => N::from(a as i64 - b),
            (N::NegInt(a), N::NegInt(b)) => N::from(a - b),
            (N::NegInt(a), N::PosInt(b)) => N::from(a - b as i64),
            (N::Float(a), N::Float(b)) => N::Float(a - b),
            (a, b) => N::Float(a.to_f64() - b.to_f64()),
        };

        Number { n }
    }
}

impl Mul for Number {
    type Output = Number;

    fn mul(self, rhs: Self) -> Self::Output {
        let n = match (self.n, rhs.n) {
            (N::PosInt(a), N::PosInt(b)) => N::PosInt(a * b),
            (N::PosInt(a), N::NegInt(b)) => N::from(a as i64 * b),
            (N::NegInt(a), N::NegInt(b)) => N::from(a * b),
            (N::NegInt(a), N::PosInt(b)) => N::from(a * b as i64),
            (N::Float(a), N::Float(b)) => N::Float(a * b),
            (a, b) => N::Float(a.to_f64() * b.to_f64()),
        };

        Number { n }
    }
}

impl Div for Number {
    type Output = Number;

    fn div(self, rhs: Self) -> Self::Output {
        let both_integer = !(self.is_f64() || self.is_f64());
        let value = self.n.to_f64() / rhs.n.to_f64();

        let n = if both_integer && value.fract() == 0.0 {
            if value < 0.0 {
                N::from(value as i64)
            } else {
                N::PosInt(value as u64)
            }
        } else {
            N::Float(value)
        };

        Number { n }
    }
}

impl Rem for Number {
    type Output = Number;

    fn rem(self, rhs: Self) -> Self::Output {
        let n = match (self.n, rhs.n) {
            (N::PosInt(a), N::PosInt(b)) => N::PosInt(a % b),
            (N::PosInt(a), N::NegInt(b)) => N::from(a as i64 % b),
            (N::NegInt(a), N::NegInt(b)) => N::from(a % b),
            (N::NegInt(a), N::PosInt(b)) => N::from(a % b as i64),
            (N::Float(a), N::Float(b)) => N::Float(a % b),
            (a, b) => N::Float(a.to_f64() % b.to_f64()),
        };

        Number { n }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn div() {
        assert_eq!(
            Number::from(1u64) / Number::from(2u64),
            Number::from_f64(0.5).unwrap()
        );
        assert_eq!(
            Number::from_f64(4.1).unwrap() / Number::from_f64(2.0).unwrap(),
            Number::from_f64(2.05).unwrap()
        );
        assert_eq!(Number::from(4u64) / Number::from(2u64), Number::from(2u64));
        assert_eq!(
            Number::from(-4i64) / Number::from(2u64),
            Number::from(-2i64)
        );
        assert_eq!(
            Number::from_f64(4.0).unwrap() / Number::from_f64(2.0).unwrap(),
            Number::from(2)
        );
        assert_eq!(
            Number::from_f64(-4.0).unwrap() / Number::from_f64(2.0).unwrap(),
            Number::from(-2)
        );
    }
}
