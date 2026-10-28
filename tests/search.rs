use std::collections::BTreeMap;

use egyptian_fraction_search::{
    Metrics, Objective, Rational, SearchConfig, SearchResult, SearchStatus, bounded_search,
    verify_bounded,
};
use num_bigint::BigUint;
use num_traits::ToPrimitive;

fn denominators(result: &SearchResult) -> Vec<Vec<u64>> {
    result
        .solutions
        .iter()
        .map(|solution| {
            solution
                .denominators
                .iter()
                .map(|value| value.to_u64().unwrap())
                .collect()
        })
        .collect()
}

// Independent oracle: enumerate subsets by bit masks and sum integer weights in
// a common factorial denominator. It never subtracts rational remainders or uses
// the search's candidate bounds. The tiny denominator ceiling keeps u128 exact.
fn oracle(numerator: u128, denominator: u128, config: &SearchConfig) -> Vec<Vec<u64>> {
    let common: u128 = (2..=config.max_denominator as u128).product();
    let mut all = Vec::new();
    for mask in 1_u64..(1 << (config.max_denominator - 1)) {
        if mask.count_ones() as usize > config.max_terms {
            continue;
        }
        let subset: Vec<u64> = (2..=config.max_denominator)
            .filter(|d| mask & (1 << (d - 2)) != 0)
            .collect();
        let sum: u128 = subset.iter().map(|d| common / u128::from(*d)).sum();
        if sum * denominator == numerator * common {
            all.push(subset);
        }
    }
    all.sort();
    match config.objective {
        Objective::Terms => all
            .into_iter()
            .min_by_key(|s| (s.len(), *s.last().unwrap(), s.clone()))
            .into_iter()
            .collect(),
        Objective::Denominator => all
            .into_iter()
            .min_by_key(|s| (*s.last().unwrap(), s.len(), s.clone()))
            .into_iter()
            .collect(),
        Objective::Pareto => {
            let mut representatives = BTreeMap::new();
            for candidate in &all {
                let pair = (candidate.len(), *candidate.last().unwrap());
                let dominated = all.iter().any(|other| {
                    let other_pair = (other.len(), *other.last().unwrap());
                    other_pair != pair && other_pair.0 <= pair.0 && other_pair.1 <= pair.1
                });
                if !dominated {
                    representatives
                        .entry(pair)
                        .or_insert_with(|| candidate.clone());
                }
            }
            representatives.into_values().collect()
        }
    }
}

#[test]
fn all_objectives_match_independent_tiny_subset_enumeration() {
    for denominator in 2..=9_u128 {
        for numerator in 1..denominator {
            let input: Rational = format!("{numerator}/{denominator}").parse().unwrap();
            for max_denominator in [2, 5, 8, 10] {
                for max_terms in [1, 2, 4] {
                    for objective in [Objective::Terms, Objective::Denominator, Objective::Pareto] {
                        let config = SearchConfig {
                            max_denominator,
                            max_terms,
                            max_nodes: 100_000,
                            objective,
                        };
                        let result = bounded_search(&input, &config).unwrap();
                        let expected = oracle(numerator, denominator, &config);
                        assert_eq!(
                            denominators(&result),
                            expected,
                            "{input}, D={max_denominator}, K={max_terms}, {objective}"
                        );
                        assert_eq!(
                            result.status,
                            if expected.is_empty() {
                                SearchStatus::NoSolutionWithinBounds
                            } else {
                                SearchStatus::Completed
                            }
                        );
                        for solution in &result.solutions {
                            verify_bounded(&input, solution, max_denominator, max_terms).unwrap();
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn two_sevenths_shows_the_pareto_tradeoff() {
    let input = "2/7".parse().unwrap();
    let mut config = SearchConfig {
        max_denominator: 35,
        max_terms: 4,
        max_nodes: 100_000,
        objective: Objective::Terms,
    };
    assert_eq!(
        denominators(&bounded_search(&input, &config).unwrap()),
        vec![vec![4, 28]]
    );
    config.objective = Objective::Denominator;
    assert_eq!(
        denominators(&bounded_search(&input, &config).unwrap()),
        vec![vec![6, 14, 21]]
    );
    config.objective = Objective::Pareto;
    assert_eq!(
        denominators(&bounded_search(&input, &config).unwrap()),
        vec![vec![4, 28], vec![6, 14, 21]]
    );
}

#[test]
fn equal_metric_pairs_choose_lexicographically_first_denominators() {
    let input = "13/20".parse().unwrap();
    // Both [2, 12, 15] and [3, 4, 15] have the optimal pair (3 terms, D=15).
    for objective in [Objective::Terms, Objective::Denominator, Objective::Pareto] {
        let config = SearchConfig {
            max_denominator: 15,
            max_terms: 3,
            max_nodes: 10_000,
            objective,
        };
        let result = bounded_search(&input, &config).unwrap();
        assert_eq!(denominators(&result), vec![vec![2, 12, 15]]);
        assert_eq!(denominators(&result), oracle(13, 20, &config));
    }
}

#[test]
fn denominator_and_term_bounds_are_inclusive() {
    let input = "4/5".parse().unwrap();
    let mut config = SearchConfig {
        max_denominator: 10,
        max_terms: 3,
        max_nodes: 10_000,
        objective: Objective::Terms,
    };
    assert_eq!(
        denominators(&bounded_search(&input, &config).unwrap()),
        vec![vec![2, 5, 10]]
    );
    config.max_denominator = 9;
    assert_eq!(
        bounded_search(&input, &config).unwrap().status,
        SearchStatus::NoSolutionWithinBounds
    );
    config.max_denominator = 10;
    config.max_terms = 2;
    assert_eq!(
        bounded_search(&input, &config).unwrap().status,
        SearchStatus::NoSolutionWithinBounds
    );
}

#[test]
fn zero_budget_does_no_search_work() {
    let input = "2/7".parse().unwrap();
    let config = SearchConfig {
        max_nodes: 0,
        ..SearchConfig::default()
    };
    let result = bounded_search(&input, &config).unwrap();
    assert_eq!(result.status, SearchStatus::BudgetExhausted);
    assert!(result.solutions.is_empty());
    assert_eq!(result.metrics.search_nodes, 0);
    assert_eq!(result.metrics.search_branches, 0);
    assert_eq!(result.metrics.denominators_examined, 0);
    assert_eq!(result.metrics.arithmetic_operations, 0);
}

#[test]
fn exact_node_budget_can_complete_or_prove_no_solution() {
    let config = SearchConfig {
        max_denominator: 2,
        max_terms: 1,
        max_nodes: 1,
        objective: Objective::Pareto,
    };
    let result = bounded_search(&"1/2".parse().unwrap(), &config).unwrap();
    assert_eq!(result.status, SearchStatus::Completed);
    assert_eq!(result.metrics.search_nodes, 1);
    assert_eq!(result.metrics.search_branches, 0);
    assert_eq!(result.metrics.denominators_examined, 1);
    assert_eq!(result.metrics.arithmetic_operations, 0);
    let result = bounded_search(&"1/3".parse().unwrap(), &config).unwrap();
    assert_eq!(result.status, SearchStatus::NoSolutionWithinBounds);
    assert_eq!(result.metrics.search_nodes, 1);
    // With D=2, no longer decomposition can exist, even when K permits it.
    let result = bounded_search(
        &"1/2".parse().unwrap(),
        &SearchConfig {
            max_terms: 8,
            ..config
        },
    )
    .unwrap();
    assert_eq!(result.status, SearchStatus::Completed);
    assert_eq!(result.metrics.search_nodes, 1);
}

#[test]
fn exhausted_search_preserves_verified_incumbents() {
    let input = "2/7".parse().unwrap();
    let mut config = SearchConfig {
        max_denominator: 35,
        max_terms: 4,
        max_nodes: 3,
        objective: Objective::Pareto,
    };
    let partial = bounded_search(&input, &config).unwrap();
    assert_eq!(partial.status, SearchStatus::BudgetExhausted);
    assert_eq!(partial.metrics.search_nodes, 3);
    assert_eq!(denominators(&partial), vec![vec![4, 28]]);
    verify_bounded(&input, &partial.solutions[0], 35, 4).unwrap();

    config.max_nodes = 100_000;
    let complete = bounded_search(&input, &config).unwrap();
    config.max_nodes = complete.metrics.search_nodes;
    let exact = bounded_search(&input, &config).unwrap();
    assert_eq!(exact.status, SearchStatus::Completed);
    assert_eq!(denominators(&exact), denominators(&complete));
    assert_eq!(exact.metrics.search_nodes, config.max_nodes);
    config.max_nodes -= 1;
    assert_eq!(
        bounded_search(&input, &config).unwrap().status,
        SearchStatus::BudgetExhausted
    );
}

#[test]
fn small_search_policy_validates_reduced_inputs_and_config() {
    let valid = "4/5".parse().unwrap();
    for max_denominator in [0, 1, 10_001] {
        assert!(
            bounded_search(
                &valid,
                &SearchConfig {
                    max_denominator,
                    ..SearchConfig::default()
                }
            )
            .is_err()
        );
    }
    for max_terms in [0, 9] {
        assert!(
            bounded_search(
                &valid,
                &SearchConfig {
                    max_terms,
                    ..SearchConfig::default()
                }
            )
            .is_err()
        );
    }
    for input in ["33/34", "1/201"] {
        assert!(bounded_search(&input.parse().unwrap(), &SearchConfig::default()).is_err());
    }
    assert!(bounded_search(&"100/125".parse().unwrap(), &SearchConfig::default()).is_ok());
    let zero = valid
        .subtract_unit(&BigUint::from(2_u32), &mut Metrics::default())
        .unwrap()
        .subtract_unit(&BigUint::from(4_u32), &mut Metrics::default())
        .unwrap()
        .subtract_unit(&BigUint::from(20_u32), &mut Metrics::default())
        .unwrap();
    assert!(bounded_search(&zero, &SearchConfig::default()).is_err());
}

#[test]
fn objective_parsing_and_results_are_deterministic() {
    assert_eq!("terms".parse::<Objective>().unwrap(), Objective::Terms);
    assert_eq!(
        "denominator".parse::<Objective>().unwrap(),
        Objective::Denominator
    );
    assert_eq!("pareto".parse::<Objective>().unwrap(), Objective::Pareto);
    assert!("unknown".parse::<Objective>().is_err());
    let input = "5/6".parse().unwrap();
    let config = SearchConfig::default();
    let a = bounded_search(&input, &config).unwrap();
    let b = bounded_search(&input, &config).unwrap();
    assert_eq!(a.solutions, b.solutions);
    assert_eq!(a.metrics.search_nodes, b.metrics.search_nodes);
    assert_eq!(a.metrics.search_branches, b.metrics.search_branches);
    assert_eq!(
        a.metrics.denominators_examined,
        b.metrics.denominators_examined
    );
    assert_eq!(
        a.metrics.arithmetic_operations,
        b.metrics.arithmetic_operations
    );
    assert_eq!(a.metrics.gcd_steps, b.metrics.gcd_steps);
}
