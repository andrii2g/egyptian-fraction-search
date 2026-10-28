use egyptian_fraction_search::{GreedyLimits, GreedyStatus, Metrics, Rational, gcd, greedy};
use num_bigint::BigUint;

fn denominators(fraction: &str) -> Vec<String> {
    let report = greedy(&fraction.parse().unwrap(), GreedyLimits::default()).unwrap();
    assert_eq!(report.status, GreedyStatus::Complete);
    assert_eq!(report.remainder.to_string(), "0/1");
    report
        .decomposition
        .denominators
        .iter()
        .map(ToString::to_string)
        .collect()
}

#[test]
fn euclid_and_reduction_cover_zero_and_large_integers() {
    assert_eq!(gcd(54u32.into(), 24u32.into()), BigUint::from(6u32));
    assert_eq!(gcd(0u32.into(), 24u32.into()), BigUint::from(24u32));
    assert_eq!(gcd(0u32.into(), 0u32.into()), BigUint::from(0u32));
    assert_eq!(" 12 / 18 ".parse::<Rational>().unwrap().to_string(), "2/3");
    let scale = BigUint::from(10u32).pow(100);
    let fraction = Rational::new(&scale * 2u32, &scale * 7u32).unwrap();
    assert_eq!(fraction.to_string(), "2/7");
}

#[test]
fn invalid_and_unsupported_fractions_are_rejected() {
    for fraction in [
        "0/3", "3/3", "4/3", "1/0", "-1/3", "+1/3", "1", "1/2/3", "/2",
    ] {
        assert!(fraction.parse::<Rational>().is_err(), "{fraction}");
    }
}

#[test]
fn subtraction_is_checked_and_zero_is_canonical() {
    let fraction: Rational = "1/3".parse().unwrap();
    let mut metrics = Metrics::default();
    assert!(fraction.subtract_unit(&2u32.into(), &mut metrics).is_none());
    assert!(fraction.subtract_unit(&0u32.into(), &mut metrics).is_none());
    let zero = fraction.subtract_unit(&3u32.into(), &mut metrics).unwrap();
    assert_eq!(zero.to_string(), "0/1");
    assert!(zero.ceil_reciprocal(&mut metrics).is_none());
    assert!(greedy(&zero, GreedyLimits::default()).is_err());
}

#[test]
fn known_greedy_expansions_and_unit_fraction_termination() {
    assert_eq!(denominators("1/7"), ["7"]);
    assert_eq!(denominators("4/5"), ["2", "4", "20"]);
    assert_eq!(denominators("2/7"), ["4", "28"]);
    assert_eq!(denominators("4/17"), ["5", "29", "1233", "3039345"]);
}

#[test]
fn greedy_results_are_distinct_and_exact_on_a_grid() {
    for b in 2u64..=35 {
        for a in 1u64..b {
            let fraction = Rational::new(a.into(), b.into()).unwrap();
            let report = greedy(&fraction, GreedyLimits::default()).unwrap();
            assert_eq!(report.status, GreedyStatus::Complete);
            let mut previous = BigUint::from(1u32);
            // Independent addition, rather than repeating greedy subtraction.
            let (mut sum_a, mut sum_b) = (BigUint::from(0u32), BigUint::from(1u32));
            for d in &report.decomposition.denominators {
                assert!(d > &previous);
                previous = d.clone();
                sum_a = sum_a * d + &sum_b;
                sum_b *= d;
            }
            assert_eq!(sum_a * fraction.denominator(), fraction.numerator() * sum_b);
        }
    }
}

#[test]
fn resource_limits_return_partial_results_with_a_remainder() {
    let fraction = "4/17".parse().unwrap();
    let report = greedy(
        &fraction,
        GreedyLimits {
            max_steps: 1,
            max_bits: 128,
        },
    )
    .unwrap();
    assert_eq!(report.status, GreedyStatus::StepLimit);
    assert_eq!(report.decomposition.denominators, [BigUint::from(5u32)]);
    assert_eq!(report.remainder.to_string(), "3/85");
    let report = greedy(
        &fraction,
        GreedyLimits {
            max_steps: 128,
            max_bits: 5,
        },
    )
    .unwrap();
    assert_eq!(report.status, GreedyStatus::BitLimit);
    assert_eq!(report.remainder, fraction);
    assert!(report.decomposition.denominators.is_empty());
    assert!(
        greedy(
            &fraction,
            GreedyLimits {
                max_steps: 0,
                max_bits: 128
            }
        )
        .is_err()
    );
}
