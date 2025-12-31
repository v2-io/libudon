//! Stochastic content generators for test variations
//!
//! Uses seeded RNG for reproducibility. Print seed on failure for replay.

use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

/// Seeded generator for reproducible stochastic tests
pub struct Gen {
    pub rng: StdRng,
    pub seed: u64,
}

impl Gen {
    /// Create with specific seed (for reproduction)
    pub fn new(seed: u64) -> Self {
        Self {
            rng: StdRng::seed_from_u64(seed),
            seed,
        }
    }

    /// Create from environment or random seed
    pub fn from_env_or_random() -> Self {
        let seed = std::env::var("UDON_TEST_SEED")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| rand::random());
        Self::new(seed)
    }

    /// Geometric distribution: count until rand > alpha
    /// Returns 0, 1, 2, ... with decreasing probability
    pub fn geometric(&mut self, alpha: f64) -> usize {
        let mut n = 0;
        while self.rng.gen::<f64>() < alpha {
            n += 1;
        }
        n
    }

    /// Poisson-like count (simplified)
    pub fn poisson(&mut self, lambda: f64) -> usize {
        let l = (-lambda).exp();
        let mut k = 0;
        let mut p = 1.0;
        loop {
            k += 1;
            p *= self.rng.gen::<f64>();
            if p <= l {
                break;
            }
        }
        k - 1
    }

    /// Random boolean with probability p
    pub fn chance(&mut self, p: f64) -> bool {
        self.rng.gen::<f64>() < p
    }

    /// Random element name (XID_Start + XID_Continue*)
    pub fn name(&mut self) -> Vec<u8> {
        // Simplified: just ASCII letters for now
        let len = 1 + self.geometric(0.7);
        let mut name = Vec::with_capacity(len);
        // First char: letter
        name.push(self.rng.gen_range(b'a'..=b'z'));
        // Rest: letter, digit, hyphen, underscore
        let chars = b"abcdefghijklmnopqrstuvwxyz0123456789-_";
        for _ in 1..len {
            name.push(chars[self.rng.gen_range(0..chars.len())]);
        }
        name
    }

    /// Random identifier (for id/class values)
    pub fn identifier(&mut self) -> Vec<u8> {
        self.name() // Same rules for now
    }

    /// Random bare value (no spaces, no special chars)
    pub fn bare_value(&mut self) -> Vec<u8> {
        let len = 1 + self.geometric(0.8);
        let chars = b"abcdefghijklmnopqrstuvwxyz0123456789-_.";
        let mut val = Vec::with_capacity(len);
        for _ in 0..len {
            val.push(chars[self.rng.gen_range(0..chars.len())]);
        }
        val
    }

    /// Random integer literal
    pub fn integer(&mut self) -> Vec<u8> {
        let val: i32 = self.rng.gen_range(-9999..9999);
        val.to_string().into_bytes()
    }

    /// Random valid UDON fragment (for context wrapping)
    pub fn udon_fragment(&mut self, base_indent: usize) -> Vec<u8> {
        let mut out = Vec::new();
        let indent: String = " ".repeat(base_indent);

        // Simple element with optional content
        out.extend(indent.as_bytes());
        out.push(b'|');
        out.extend(self.name());

        // Maybe add an attribute
        if self.chance(0.3) {
            out.extend(b" :");
            out.extend(self.name());
            out.push(b' ');
            out.extend(self.bare_value());
        }

        // Maybe add prose
        if self.chance(0.3) {
            out.extend(b" some prose here");
        }

        out.push(b'\n');
        out
    }

    /// Add random indent (geometric, α=0.9)
    pub fn indent_level(&mut self) -> usize {
        self.geometric(0.9) * 2 // 2 spaces per level
    }

    /// Inject random blank lines
    pub fn blank_lines(&mut self) -> Vec<u8> {
        let count = self.geometric(0.1); // Usually 0
        vec![b'\n'; count]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reproducibility() {
        let mut g1 = Gen::new(42);
        let mut g2 = Gen::new(42);

        for _ in 0..10 {
            assert_eq!(g1.name(), g2.name());
            assert_eq!(g1.geometric(0.9), g2.geometric(0.9));
        }
    }

    #[test]
    fn test_geometric_distribution() {
        let mut gen = Gen::new(12345);
        let samples: Vec<usize> = (0..1000).map(|_| gen.geometric(0.9)).collect();

        // With α=0.9, we expect mean ≈ 9 (geometric mean = α/(1-α))
        let mean: f64 = samples.iter().sum::<usize>() as f64 / samples.len() as f64;
        assert!(mean > 5.0 && mean < 15.0, "Mean {} out of expected range", mean);
    }
}
