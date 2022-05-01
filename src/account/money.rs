use std::{
    fmt::{Debug, Display},
    num::ParseIntError,
    ops::{Add, AddAssign, Sub, SubAssign},
};

use serde::{de::Visitor, Deserialize, Serialize};

type MoneyInner = i64;

const ONE_MONEY: MoneyInner = 1_0000;

/// `Money` is a numeric quantity with four decimal places.
#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Money(MoneyInner);

impl Money {
    pub const ZERO: Money = Money(0);

    #[cfg(test)]
    pub fn from_i64(num: i64) -> Self {
        Money(num)
    }

    #[allow(unused)]
    pub fn is_positive(&self) -> bool {
        self.0 > 0
    }

    pub fn is_negative(&self) -> bool {
        self.0 < 0
    }

    #[cfg(test)]
    pub fn set_sign_negative(&mut self, negative: bool) {
        self.0 = self.0.abs();
        if negative {
            self.0 = -self.0;
        }
    }

    pub fn to_f64(self) -> f64 {
        self.0 as f64 / ONE_MONEY as f64
    }
}

impl Serialize for Money {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Money {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct MoneyVisitor;
        impl MoneyVisitor {
            fn parseint_error<E>(e: ParseIntError) -> E
            where
                E: serde::de::Error,
            {
                E::custom(format!("error parsing as integer: {}", e))
            }
        }
        impl<'de> Visitor<'de> for MoneyVisitor {
            type Value = Money;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a positive amount of money")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let (whole, fraction) = if let Some((whole, fraction_s)) = v.split_once('.') {
                    // fraction can't start with negative sign
                    if fraction_s.starts_with('-') {
                        return Err(E::custom(format!(
                            "invalid digit after decimal point in money field: {:?}",
                            v
                        )));
                    }
                    let mut fraction = if fraction_s.is_empty() {
                        // "" is a valid fractional part
                        0
                    } else {
                        fraction_s
                            .parse::<MoneyInner>()
                            .map_err(Self::parseint_error)?
                    };
                    fraction *= ONE_MONEY;
                    // divide the fraction by 10 for every digit present after
                    for _ in 0..fraction_s.len() {
                        fraction += 5; // round up
                        fraction /= 10;
                    }
                    // transfer sign from whole to fraction, keeping in mind that the
                    // whole portion may be -0, so can't trust whole.parse to preserve sign
                    if whole.starts_with('-') {
                        fraction = -fraction;
                    }
                    // "-" isn't a valid integer, but it is a valid whole portion of a decimal,
                    // but only if we have a fraction
                    let whole = if (whole == "-" || whole.is_empty()) && !fraction_s.is_empty() {
                        0
                    } else {
                        whole.parse::<MoneyInner>().map_err(Self::parseint_error)?
                    };
                    (whole, fraction)
                } else {
                    let whole = v.parse::<MoneyInner>().map_err(Self::parseint_error)?;
                    let fraction = 0;
                    (whole, fraction)
                };
                Ok(Money(whole * ONE_MONEY + fraction))
            }
        }
        deserializer.deserialize_str(MoneyVisitor)
    }
}

impl Display for Money {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.0 / ONE_MONEY, self.0.abs() % ONE_MONEY)
    }
}

impl Debug for Money {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Money").field(&self.to_string()).finish()
    }
}

// manually implemented arithmatic will always panic, even in release mode.
// Better to crash the application than corrupt someone's account balance
impl Add for Money {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Money(self.0.checked_add(rhs.0).unwrap())
    }
}
impl Sub for Money {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Money(self.0.checked_sub(rhs.0).unwrap())
    }
}
impl AddAssign for Money {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}
impl SubAssign for Money {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

#[cfg(test)]
mod tests {
    use serde::de::value::Error as SerdeError;
    use serde::{de::IntoDeserializer, Deserialize};

    use super::*;

    fn deser_str(s: &str) -> Result<Money, SerdeError> {
        Money::deserialize(s.into_deserializer())
    }

    #[test]
    fn deser_negative_zero_whole_portion() -> Result<(), SerdeError> {
        let expected = Money(-1234);
        let parsed = deser_str("-0.1234")?;
        assert_eq!(expected, parsed);
        Ok(())
    }

    #[test]
    fn deser_negative_empty_whole_portion() -> Result<(), SerdeError> {
        let expected = Money(-1234);
        let parsed = deser_str("-.1234")?;
        assert_eq!(expected, parsed);
        Ok(())
    }

    #[test]
    fn deser_negative_one_whole_portion() -> Result<(), SerdeError> {
        let expected = Money(-12345);
        let parsed = deser_str("-1.2345")?;
        assert_eq!(expected, parsed);
        Ok(())
    }

    #[test]
    fn deser_one_tenth() -> Result<(), SerdeError> {
        let expected = Money(ONE_MONEY / 10);
        let parsed = deser_str("0.1")?;
        assert_eq!(expected, parsed);
        Ok(())
    }

    #[test]
    fn deser_one_hundredth() -> Result<(), SerdeError> {
        let expected = Money(ONE_MONEY / 100);
        let parsed = deser_str("0.01")?;
        assert_eq!(expected, parsed);
        Ok(())
    }

    #[test]
    fn deser_one_thousandth() -> Result<(), SerdeError> {
        let expected = Money(ONE_MONEY / 1000);
        let parsed = deser_str("0.001")?;
        assert_eq!(expected, parsed);
        Ok(())
    }

    #[test]
    fn deser_one_ten_thousandth() -> Result<(), SerdeError> {
        let expected = Money(ONE_MONEY / 10000);
        let parsed = deser_str("0.0001")?;
        assert_eq!(expected, parsed);
        Ok(())
    }

    #[test]
    fn deser_one_hundred_thousandth_is_zero() -> Result<(), SerdeError> {
        let expected = Money(0);
        let parsed = deser_str("0.00001")?;
        assert_eq!(expected, parsed);
        Ok(())
    }

    #[test]
    fn deser_nine_hundred_thousandths_rounds_up_to_one_ten_thousandth() -> Result<(), SerdeError> {
        let expected = Money(ONE_MONEY / 10000);
        let parsed = deser_str("0.00009")?;
        assert_eq!(expected, parsed);
        Ok(())
    }

    #[test]
    fn deser_various_halves() -> Result<(), SerdeError> {
        let expected = Money(ONE_MONEY / 2);
        for s in [
            "0.5",
            "0.50",
            "0.500",
            "0.5000",
            "0.50000",
            "0.500000",
            "0.5000000",
        ] {
            let parsed = deser_str(s)?;
            assert_eq!(expected, parsed);
        }
        Ok(())
    }

    #[test]
    fn deser_blanks() -> Result<(), SerdeError> {
        assert_eq!(deser_str("0.0")?, Money::ZERO);
        assert_eq!(deser_str("0.")?, Money::ZERO);
        assert_eq!(deser_str(".0")?, Money::ZERO);
        assert_eq!(deser_str("0")?, Money::ZERO);
        assert_eq!(deser_str("-0.0")?, Money::ZERO);
        assert_eq!(deser_str("-0.")?, Money::ZERO);
        assert_eq!(deser_str("-.0")?, Money::ZERO);
        assert_eq!(deser_str("-0")?, Money::ZERO);
        assert!(deser_str("").is_err());
        assert!(deser_str(".").is_err());
        assert!(deser_str(".0.").is_err());
        assert!(deser_str("-").is_err());
        assert!(deser_str("-.").is_err());
        assert!(deser_str("-.0.").is_err());
        Ok(())
    }
}
