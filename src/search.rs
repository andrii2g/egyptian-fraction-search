use std::{cmp::Ordering, fmt, str::FromStr, time::Instant};

use num_bigint::BigUint;
use num_traits::{One, ToPrimitive};

use crate::{Decomposition, Metrics, Rational, verify_bounded};

pub const MAX_SEARCH_NUMERATOR: u64 = 32;
pub const MAX_SEARCH_INPUT_DENOMINATOR: u64 = 200;
pub const MAX_SEARCH_DENOMINATOR: u64 = 10_000;
pub const MAX_SEARCH_TERMS: usize = 8;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Objective {
    Terms,
    Denominator,
    #[default]
    Pareto,
}

impl fmt::Display for Objective {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Terms => "terms",
            Self::Denominator => "denominator",
            Self::Pareto => "pareto",
        })
    }
}

impl FromStr for Objective {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "terms" => Ok(Self::Terms),
            "denominator" => Ok(Self::Denominator),
            "pareto" => Ok(Self::Pareto),
            _ => Err("objective must be terms, denominator, or pareto"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SearchConfig {
    pub max_denominator: u64,
    pub max_terms: usize,
    pub max_nodes: u64,
    pub objective: Objective,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            max_denominator: 200,
            max_terms: 5,
            max_nodes: 1_000_000,
            objective: Objective::Pareto,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchStatus {
    Completed,
    NoSolutionWithinBounds,
    BudgetExhausted,
}

impl fmt::Display for SearchStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Completed => "completed",
            Self::NoSolutionWithinBounds => "no solution within bounds",
            Self::BudgetExhausted => "budget exhausted",
        })
    }
}

#[derive(Clone, Debug)]
pub struct SearchResult {
    /// Optimal solutions after completion, or the best incumbents when exhausted.
    /// Pareto keeps the lexicographically first representative per metric pair.
    pub solutions: Vec<Decomposition>,
    pub status: SearchStatus,
    pub metrics: Metrics,
}

/// Search small proper fractions using exact-length, increasing-denominator DFS.
///
/// Nodes are entered DFS states, including each exact-length root. Branches are
/// recursive descents into entered states. Denominators examined counts each
/// candidate whose remainder is computed, plus feasible final unit denominators
/// accepted within bounds. A denied node consumes no node or branch; some candidate arithmetic
/// may already have been performed to discover it. An exact node-budget match is
/// complete if no further state is needed. Input validation and independent
/// verification of returned solutions are excluded from the metrics and timer.
pub fn bounded_search(
    input: &Rational,
    config: &SearchConfig,
) -> Result<SearchResult, &'static str> {
    if !input.is_proper() {
        return Err("search input must be positive and proper");
    }
    if input.numerator() > &BigUint::from(MAX_SEARCH_NUMERATOR)
        || input.denominator() > &BigUint::from(MAX_SEARCH_INPUT_DENOMINATOR)
    {
        return Err("bounded search requires reduced numerator <= 32 and denominator <= 200");
    }
    if !(2..=MAX_SEARCH_DENOMINATOR).contains(&config.max_denominator) {
        return Err("maximum search denominator must be between 2 and 10000");
    }
    if !(1..=MAX_SEARCH_TERMS).contains(&config.max_terms) {
        return Err("maximum search term count must be between 1 and 8");
    }

    let started = Instant::now();
    let mut search = Search {
        config,
        metrics: Metrics::default(),
        solutions: Vec::new(),
        exhausted: false,
    };
    let mut prefix = Vec::with_capacity(config.max_terms);
    let possible_terms = config.max_terms.min((config.max_denominator - 1) as usize);
    for slots in 1..=possible_terms {
        search.visit(input, 2, slots, &mut prefix);
        if search.exhausted
            || (config.objective == Objective::Terms && !search.solutions.is_empty())
        {
            break;
        }
    }
    search.solutions.sort_by(|a, b| terms_order(a, b));
    let solutions: Vec<_> = search
        .solutions
        .into_iter()
        .map(|denominators| Decomposition {
            denominators: denominators.into_iter().map(BigUint::from).collect(),
        })
        .collect();
    search.metrics.elapsed = started.elapsed();
    for solution in &solutions {
        verify_bounded(input, solution, config.max_denominator, config.max_terms)
            .map_err(|_| "internal search result failed independent verification")?;
    }
    let status = if search.exhausted {
        SearchStatus::BudgetExhausted
    } else if solutions.is_empty() {
        SearchStatus::NoSolutionWithinBounds
    } else {
        SearchStatus::Completed
    };
    Ok(SearchResult {
        solutions,
        status,
        metrics: search.metrics,
    })
}

struct Search<'a> {
    config: &'a SearchConfig,
    metrics: Metrics,
    solutions: Vec<Vec<u64>>,
    exhausted: bool,
}

impl Search<'_> {
    /// Return whether this state was entered, including states pruned immediately.
    fn visit(
        &mut self,
        remainder: &Rational,
        next: u64,
        slots: usize,
        prefix: &mut Vec<u64>,
    ) -> bool {
        if self.metrics.search_nodes >= self.config.max_nodes {
            self.exhausted = true;
            return false;
        }
        self.metrics.search_nodes += 1;
        if next > self.config.max_denominator
            || slots as u64 > self.config.max_denominator - next + 1
        {
            return true;
        }
        if slots == 1 {
            if remainder.numerator().is_one()
                && let Some(denominator) = remainder.denominator().to_u64()
                && (next..=self.config.max_denominator).contains(&denominator)
            {
                self.metrics.denominators_examined += 1;
                prefix.push(denominator);
                self.consider(prefix);
                prefix.pop();
            }
            return true;
        }

        // No unit may exceed a/b, so d >= ceil(b/a). Also a/b <= slots/d,
        // hence d <= floor(slots*b/a). Reserve space below D for later units.
        let lower = remainder
            .ceil_reciprocal(&mut self.metrics)
            .and_then(|value| value.to_u64())
            .unwrap_or(u64::MAX)
            .max(next);
        let upper = (remainder.denominator() * BigUint::from(slots)) / remainder.numerator();
        self.metrics.arithmetic_operations += 2;
        let upper = upper
            .to_u64()
            .unwrap_or(u64::MAX)
            .min(self.config.max_denominator - slots as u64 + 1);
        for denominator in lower..=upper {
            self.metrics.denominators_examined += 1;
            let remaining = remainder
                .subtract_unit(&BigUint::from(denominator), &mut self.metrics)
                .expect("candidate interval excludes negative remainders");
            if remaining.is_zero() {
                // An exact-length search cannot fill further positive terms.
                continue;
            }
            prefix.push(denominator);
            if self.visit(&remaining, denominator + 1, slots - 1, prefix) {
                self.metrics.search_branches += 1;
            }
            prefix.pop();
            if self.exhausted {
                break;
            }
        }
        true
    }

    fn consider(&mut self, candidate: &[u64]) {
        match self.config.objective {
            Objective::Terms | Objective::Denominator => {
                let better = self.solutions.first().is_none_or(|best| {
                    if self.config.objective == Objective::Terms {
                        terms_order(candidate, best) == Ordering::Less
                    } else {
                        denominator_order(candidate, best) == Ordering::Less
                    }
                });
                if better {
                    self.solutions.clear();
                    self.solutions.push(candidate.to_vec());
                }
            }
            Objective::Pareto => {
                if self.solutions.iter().any(|solution| {
                    dominates(solution, candidate)
                        && (solution.len() != candidate.len()
                            || solution.last() != candidate.last()
                            || solution.as_slice() <= candidate)
                }) {
                    return;
                }
                self.solutions
                    .retain(|solution| !dominates(candidate, solution));
                self.solutions.push(candidate.to_vec());
            }
        }
    }
}

fn terms_order(a: &[u64], b: &[u64]) -> Ordering {
    a.len()
        .cmp(&b.len())
        .then_with(|| a.last().cmp(&b.last()))
        .then_with(|| a.cmp(b))
}

fn denominator_order(a: &[u64], b: &[u64]) -> Ordering {
    a.last()
        .cmp(&b.last())
        .then_with(|| a.len().cmp(&b.len()))
        .then_with(|| a.cmp(b))
}

fn dominates(a: &[u64], b: &[u64]) -> bool {
    a.len() <= b.len() && a.last() <= b.last()
}
