use std::{error::Error, fmt, str::FromStr};

use num_bigint::BigUint;
use num_traits::{One, Zero};

use crate::Metrics;

/// A reduced nonnegative rational. Public inputs must be positive and proper;
/// zero is produced internally when a decomposition is complete.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rational {
    numerator: BigUint,
    denominator: BigUint,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RationalError {
    InvalidFormat,
    ZeroDenominator,
    NotPositiveProper,
}

impl fmt::Display for RationalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidFormat => "expected a fraction a/b with unsigned integer components",
            Self::ZeroDenominator => "denominator must be positive",
            Self::NotPositiveProper => "fraction must satisfy 0 < numerator < denominator",
        })
    }
}

impl Error for RationalError {}

/// Euclid's algorithm, including gcd(a, 0) = a and gcd(0, 0) = 0.
pub fn gcd(a: BigUint, b: BigUint) -> BigUint {
    measured_gcd(a, b, &mut Metrics::default())
}

fn measured_gcd(mut a: BigUint, mut b: BigUint, metrics: &mut Metrics) -> BigUint {
    while !b.is_zero() {
        let remainder = &a % &b;
        metrics.arithmetic_operations += 1;
        metrics.gcd_steps += 1;
        a = b;
        b = remainder;
    }
    a
}

impl Rational {
    pub fn new(numerator: BigUint, denominator: BigUint) -> Result<Self, RationalError> {
        if denominator.is_zero() {
            return Err(RationalError::ZeroDenominator);
        }
        if numerator.is_zero() || numerator >= denominator {
            return Err(RationalError::NotPositiveProper);
        }
        Ok(Self::reduced(
            numerator,
            denominator,
            &mut Metrics::default(),
        ))
    }

    fn reduced(numerator: BigUint, denominator: BigUint, metrics: &mut Metrics) -> Self {
        let divisor = measured_gcd(numerator.clone(), denominator.clone(), metrics);
        metrics.arithmetic_operations += 2;
        Self {
            numerator: numerator / &divisor,
            denominator: denominator / divisor,
        }
    }

    pub fn numerator(&self) -> &BigUint {
        &self.numerator
    }

    pub fn denominator(&self) -> &BigUint {
        &self.denominator
    }

    pub fn is_zero(&self) -> bool {
        self.numerator.is_zero()
    }

    pub fn is_proper(&self) -> bool {
        !self.is_zero() && self.numerator < self.denominator
    }

    /// The smallest denominator whose unit fraction does not exceed this value.
    /// Returns None for a zero remainder.
    pub fn ceil_reciprocal(&self, metrics: &mut Metrics) -> Option<BigUint> {
        if self.is_zero() {
            return None;
        }
        let quotient = &self.denominator / &self.numerator;
        let remainder = &self.denominator % &self.numerator;
        metrics.arithmetic_operations += 2;
        Some(if remainder.is_zero() {
            quotient
        } else {
            metrics.arithmetic_operations += 1;
            quotient + BigUint::one()
        })
    }

    /// Subtract a positive unit fraction without permitting a negative result.
    pub fn subtract_unit(&self, denominator: &BigUint, metrics: &mut Metrics) -> Option<Self> {
        if denominator.is_zero() {
            return None;
        }
        let scaled_numerator = &self.numerator * denominator;
        metrics.arithmetic_operations += 1;
        if scaled_numerator < self.denominator {
            return None;
        }
        let numerator = scaled_numerator - &self.denominator;
        let denominator = &self.denominator * denominator;
        metrics.arithmetic_operations += 2;
        Some(Self::reduced(numerator, denominator, metrics))
    }
}

impl FromStr for Rational {
    type Err = RationalError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (numerator, denominator) = value
            .trim()
            .split_once('/')
            .ok_or(RationalError::InvalidFormat)?;
        let parse_component = |text: &str| {
            let text = text.trim();
            if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(RationalError::InvalidFormat);
            }
            text.parse::<BigUint>()
                .map_err(|_| RationalError::InvalidFormat)
        };
        Self::new(parse_component(numerator)?, parse_component(denominator)?)
    }
}

impl fmt::Display for Rational {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.numerator, self.denominator)
    }
}
