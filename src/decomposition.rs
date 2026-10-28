use std::fmt;

use num_bigint::BigUint;

/// A candidate decomposition. Use the independent verifier to establish validity.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Decomposition {
    pub denominators: Vec<BigUint>,
}

impl Decomposition {
    pub fn term_count(&self) -> usize {
        self.denominators.len()
    }

    pub fn largest_denominator(&self) -> Option<&BigUint> {
        self.denominators.iter().max()
    }
}

impl fmt::Display for Decomposition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.denominators.is_empty() {
            return f.write_str("0");
        }
        for (index, denominator) in self.denominators.iter().enumerate() {
            if index > 0 {
                f.write_str(" + ")?;
            }
            write!(f, "1/{denominator}")?;
        }
        Ok(())
    }
}
