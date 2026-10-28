use std::time::Duration;

/// Work performed by one algorithm. Input parsing and independent verification
/// are excluded. Arithmetic counts BigUint +, -, *, /, and % operations, including
/// GCD remainders; comparisons, allocation, and primitive loop counters are excluded.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Metrics {
    pub denominators_examined: u64,
    pub search_branches: u64,
    pub search_nodes: u64,
    pub arithmetic_operations: u64,
    pub gcd_steps: u64,
    pub elapsed: Duration,
}
