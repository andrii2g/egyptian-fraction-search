use std::{error::Error, fmt};

use num_bigint::BigUint;
use num_traits::{One, Zero};

use crate::{Decomposition, Rational, gcd};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VerificationError {
    InvalidInput,
    EmptyDecomposition,
    InvalidDenominator { index: usize },
    NotIncreasing { index: usize },
    IncorrectSum,
    TooManyTerms,
    DenominatorLimit,
}

impl fmt::Display for VerificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput => f.write_str("verification input must be positive and proper"),
            Self::EmptyDecomposition => f.write_str("a complete decomposition cannot be empty"),
            Self::InvalidDenominator { index } => {
                write!(f, "denominator {} must be at least 2", index + 1)
            }
            Self::NotIncreasing { index } => {
                write!(f, "denominator {} is not strictly increasing", index + 1)
            }
            Self::IncorrectSum => f.write_str("decomposition does not sum exactly to the input"),
            Self::TooManyTerms => f.write_str("decomposition exceeds the term limit"),
            Self::DenominatorLimit => f.write_str("decomposition exceeds the denominator limit"),
        }
    }
}

impl Error for VerificationError {}

/// Add fractions independently of the solvers' remainder-subtraction arithmetic.
fn sum_terms(decomposition: &Decomposition) -> Result<(BigUint, BigUint), VerificationError> {
    let (mut numerator, mut denominator) = (BigUint::zero(), BigUint::one());
    let mut previous = BigUint::one();
    for (index, unit_denominator) in decomposition.denominators.iter().enumerate() {
        if unit_denominator <= &BigUint::one() {
            return Err(VerificationError::InvalidDenominator { index });
        }
        if unit_denominator <= &previous {
            return Err(VerificationError::NotIncreasing { index });
        }
        numerator = numerator * unit_denominator + &denominator;
        denominator *= unit_denominator;
        let divisor = gcd(numerator.clone(), denominator.clone());
        numerator /= &divisor;
        denominator /= divisor;
        previous = unit_denominator.clone();
    }
    Ok((numerator, denominator))
}

pub fn verify(input: &Rational, decomposition: &Decomposition) -> Result<(), VerificationError> {
    if !input.is_proper() {
        return Err(VerificationError::InvalidInput);
    }
    if decomposition.denominators.is_empty() {
        return Err(VerificationError::EmptyDecomposition);
    }
    let (numerator, denominator) = sum_terms(decomposition)?;
    if numerator * input.denominator() != input.numerator() * denominator {
        return Err(VerificationError::IncorrectSum);
    }
    Ok(())
}

pub fn verify_bounded(
    input: &Rational,
    decomposition: &Decomposition,
    max_denominator: u64,
    max_terms: usize,
) -> Result<(), VerificationError> {
    verify(input, decomposition)?;
    if decomposition.term_count() > max_terms {
        return Err(VerificationError::TooManyTerms);
    }
    if decomposition
        .largest_denominator()
        .is_some_and(|denominator| denominator > &BigUint::from(max_denominator))
    {
        return Err(VerificationError::DenominatorLimit);
    }
    Ok(())
}

/// Verify a partial prefix plus its exact remainder. This establishes sum and
/// ordering validity, not which selection algorithm produced the prefix.
pub fn verify_prefix(
    input: &Rational,
    prefix: &Decomposition,
    remainder: &Rational,
) -> Result<(), VerificationError> {
    if !input.is_proper() {
        return Err(VerificationError::InvalidInput);
    }
    let (numerator, denominator) = sum_terms(prefix)?;
    let total_numerator =
        numerator * remainder.denominator() + remainder.numerator() * &denominator;
    let total_denominator = denominator * remainder.denominator();
    if total_numerator * input.denominator() != input.numerator() * total_denominator {
        return Err(VerificationError::IncorrectSum);
    }
    Ok(())
}
