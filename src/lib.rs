//! Exact Egyptian fractions and bounded searches for alternative decompositions.

pub mod decomposition;
pub mod greedy;
pub mod metrics;
pub mod rational;

pub use decomposition::Decomposition;
pub use greedy::{GreedyLimits, GreedyReport, GreedyStatus, greedy};
pub use metrics::Metrics;
pub use rational::{Rational, RationalError, gcd};
