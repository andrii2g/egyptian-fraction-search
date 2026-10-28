use std::time::Instant;

use crate::{Decomposition, Metrics, Rational};

#[derive(Clone, Copy, Debug)]
pub struct GreedyLimits {
    pub max_steps: usize,
    pub max_bits: u64,
}

impl Default for GreedyLimits {
    fn default() -> Self {
        Self {
            max_steps: 128,
            max_bits: 16_384,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GreedyStatus {
    Complete,
    StepLimit,
    BitLimit,
}

#[derive(Clone, Debug)]
pub struct GreedyReport {
    pub decomposition: Decomposition,
    pub remainder: Rational,
    pub status: GreedyStatus,
    pub metrics: Metrics,
}

/// Greedy selection with exact arithmetic. A limit returns an explicit partial
/// prefix and its exact remainder, never a purported complete decomposition.
pub fn greedy(input: &Rational, limits: GreedyLimits) -> Result<GreedyReport, &'static str> {
    if !input.is_proper() {
        return Err("greedy input must be positive and proper");
    }
    if limits.max_steps == 0 || limits.max_bits == 0 {
        return Err("greedy step and bit limits must be positive");
    }
    let started = Instant::now();
    let mut metrics = Metrics::default();
    let mut remainder = input.clone();
    let mut decomposition = Decomposition::default();
    let status = loop {
        if remainder.is_zero() {
            break GreedyStatus::Complete;
        }
        if decomposition.term_count() >= limits.max_steps {
            break GreedyStatus::StepLimit;
        }
        if remainder.numerator().bits() > limits.max_bits
            || remainder.denominator().bits() > limits.max_bits
        {
            break GreedyStatus::BitLimit;
        }
        let denominator = remainder
            .ceil_reciprocal(&mut metrics)
            .expect("the remainder is positive");
        metrics.denominators_examined += 1;
        if denominator.bits() > limits.max_bits {
            break GreedyStatus::BitLimit;
        }
        let next = remainder
            .subtract_unit(&denominator, &mut metrics)
            .expect("greedy selection cannot exceed the remainder");
        if next.numerator().bits() > limits.max_bits || next.denominator().bits() > limits.max_bits
        {
            break GreedyStatus::BitLimit;
        }
        decomposition.denominators.push(denominator);
        remainder = next;
    };
    metrics.elapsed = started.elapsed();
    Ok(GreedyReport {
        decomposition,
        remainder,
        status,
        metrics,
    })
}
