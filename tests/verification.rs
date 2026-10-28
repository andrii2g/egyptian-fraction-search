use egyptian_fraction_search::verify::{VerificationError, verify, verify_bounded, verify_prefix};
use egyptian_fraction_search::{Decomposition, GreedyLimits, Rational, greedy};

fn decomposition(denominators: &[u64]) -> Decomposition {
    Decomposition {
        denominators: denominators.iter().copied().map(Into::into).collect(),
    }
}

#[test]
fn verifies_alternatives_and_rejects_malformed_candidates() {
    let target: Rational = "4/5".parse().unwrap();
    verify(&target, &decomposition(&[2, 5, 10])).unwrap();
    assert_eq!(
        verify(&target, &decomposition(&[])),
        Err(VerificationError::EmptyDecomposition)
    );
    assert_eq!(
        verify(&target, &decomposition(&[0])),
        Err(VerificationError::InvalidDenominator { index: 0 })
    );
    assert_eq!(
        verify(&target, &decomposition(&[1])),
        Err(VerificationError::InvalidDenominator { index: 0 })
    );
    assert_eq!(
        verify(&target, &decomposition(&[2, 2])),
        Err(VerificationError::NotIncreasing { index: 1 })
    );
    assert_eq!(
        verify(&target, &decomposition(&[5, 2, 10])),
        Err(VerificationError::NotIncreasing { index: 1 })
    );
    assert_eq!(
        verify(&target, &decomposition(&[2, 5, 11])),
        Err(VerificationError::IncorrectSum)
    );
}

#[test]
fn exact_verification_has_no_fixed_width_overflow() {
    let denominator = num_bigint::BigUint::from(10u32).pow(100) + 7u32;
    let target = Rational::new(1u32.into(), denominator.clone()).unwrap();
    verify(
        &target,
        &Decomposition {
            denominators: vec![denominator],
        },
    )
    .unwrap();
}

#[test]
fn verifies_constraints_in_addition_to_the_sum() {
    let target = "2/7".parse().unwrap();
    let candidate = decomposition(&[6, 14, 21]);
    verify_bounded(&target, &candidate, 21, 3).unwrap();
    assert_eq!(
        verify_bounded(&target, &candidate, 20, 3),
        Err(VerificationError::DenominatorLimit)
    );
    assert_eq!(
        verify_bounded(&target, &candidate, 21, 2),
        Err(VerificationError::TooManyTerms)
    );
}

#[test]
fn partial_prefix_and_remainder_are_checked_together() {
    // Prefix validity does not require following the greedy selection rule.
    verify_prefix(
        &"4/5".parse().unwrap(),
        &decomposition(&[4]),
        &"11/20".parse().unwrap(),
    )
    .unwrap();
    let target = "4/17".parse().unwrap();
    let report = greedy(
        &target,
        GreedyLimits {
            max_steps: 1,
            max_bits: 128,
        },
    )
    .unwrap();
    verify_prefix(&target, &report.decomposition, &report.remainder).unwrap();
    assert_eq!(
        verify(&target, &report.decomposition),
        Err(VerificationError::IncorrectSum)
    );
    assert_eq!(
        verify_prefix(&target, &report.decomposition, &"1/17".parse().unwrap()),
        Err(VerificationError::IncorrectSum)
    );
}
