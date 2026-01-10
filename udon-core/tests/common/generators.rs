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

    // === Temporal value generators ===

    /// Random date: YYYY-MM-DD or YYYY-MM
    pub fn date(&mut self) -> Vec<u8> {
        let year = self.rng.gen_range(1900..2100);
        let month = self.rng.gen_range(1..=12);

        if self.chance(0.3) {
            // YYYY-MM only
            format!("{:04}-{:02}", year, month).into_bytes()
        } else {
            // YYYY-MM-DD
            let day = self.rng.gen_range(1..=28); // Safe for all months
            format!("{:04}-{:02}-{:02}", year, month, day).into_bytes()
        }
    }

    /// Random time: HH:MM or HH:MM:SS with optional fractional seconds
    pub fn time(&mut self) -> Vec<u8> {
        let hour = self.rng.gen_range(0..24);
        let min = self.rng.gen_range(0..60);

        if self.chance(0.3) {
            // HH:MM only
            format!("{:02}:{:02}", hour, min).into_bytes()
        } else {
            let sec = self.rng.gen_range(0..60);
            if self.chance(0.3) {
                // With fractional seconds
                let frac = self.rng.gen_range(0..999999999);
                let digits = self.rng.gen_range(1..=9);
                let frac_str = format!("{:09}", frac);
                format!("{:02}:{:02}:{:02}.{}", hour, min, sec, &frac_str[..digits]).into_bytes()
            } else {
                format!("{:02}:{:02}:{:02}", hour, min, sec).into_bytes()
            }
        }
    }

    /// Random datetime: date T time with optional timezone
    pub fn datetime(&mut self) -> Vec<u8> {
        let mut result = self.date();
        // Must be full date for datetime
        if result.len() == 7 {
            // Was YYYY-MM, add day
            let day = self.rng.gen_range(1..=28);
            result.extend(format!("-{:02}", day).bytes());
        }
        result.push(b'T');
        result.extend(self.time());

        // Optional timezone
        if self.chance(0.5) {
            match self.rng.gen_range(0..3) {
                0 => result.push(b'Z'),
                1 => {
                    let h = self.rng.gen_range(0..=14);
                    let m = if self.chance(0.5) { 0 } else { 30 };
                    result.extend(format!("+{:02}:{:02}", h, m).bytes());
                }
                _ => {
                    let h = self.rng.gen_range(0..=12);
                    let m = if self.chance(0.5) { 0 } else { 30 };
                    result.extend(format!("-{:02}:{:02}", h, m).bytes());
                }
            }
        }
        result
    }

    /// Random shorthand duration: 30s, 5m, 2h, 1d, 1w, 3mo, 1y
    pub fn shorthand_duration(&mut self) -> Vec<u8> {
        let num = self.rng.gen_range(1..100);
        let suffix = match self.rng.gen_range(0..7) {
            0 => "s",
            1 => "m",
            2 => "h",
            3 => "d",
            4 => "w",
            5 => "mo",
            _ => "y",
        };
        format!("{}{}", num, suffix).into_bytes()
    }

    /// Random ISO duration: P[nY][nM][nW][nD][T[nH][nM][nS]]
    pub fn iso_duration(&mut self) -> Vec<u8> {
        let mut result = vec![b'P'];
        let mut has_date = false;
        let mut has_time = false;

        // Date components
        if self.chance(0.4) {
            let y = self.rng.gen_range(1..10);
            result.extend(format!("{}Y", y).bytes());
            has_date = true;
        }
        if self.chance(0.4) {
            let m = self.rng.gen_range(1..12);
            result.extend(format!("{}M", m).bytes());
            has_date = true;
        }
        if self.chance(0.3) {
            let w = self.rng.gen_range(1..52);
            result.extend(format!("{}W", w).bytes());
            has_date = true;
        }
        if self.chance(0.4) {
            let d = self.rng.gen_range(1..30);
            result.extend(format!("{}D", d).bytes());
            has_date = true;
        }

        // Time components
        if self.chance(0.5) || !has_date {
            result.push(b'T');
            if self.chance(0.5) {
                let h = self.rng.gen_range(1..24);
                result.extend(format!("{}H", h).bytes());
                has_time = true;
            }
            if self.chance(0.5) {
                let m = self.rng.gen_range(1..60);
                result.extend(format!("{}M", m).bytes());
                has_time = true;
            }
            if self.chance(0.5) || !has_time {
                let s = self.rng.gen_range(1..60);
                if self.chance(0.3) {
                    let frac: f64 = self.rng.gen_range(0.0..1.0);
                    result.extend(format!("{:.3}S", s as f64 + frac).bytes());
                } else {
                    result.extend(format!("{}S", s).bytes());
                }
            }
        }

        result
    }

    /// Random duration (either shorthand or ISO)
    pub fn duration(&mut self) -> Vec<u8> {
        if self.chance(0.5) {
            self.shorthand_duration()
        } else {
            self.iso_duration()
        }
    }

    /// Random relative time: +/- followed by duration
    pub fn relative_time(&mut self) -> Vec<u8> {
        let mut result = vec![if self.chance(0.5) { b'+' } else { b'-' }];
        result.extend(self.duration());
        result
    }

    /// Random temporal value (any type)
    pub fn temporal_value(&mut self) -> Vec<u8> {
        match self.rng.gen_range(0..5) {
            0 => self.date(),
            1 => self.time(),
            2 => self.datetime(),
            3 => self.duration(),
            _ => self.relative_time(),
        }
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

    #[test]
    fn test_temporal_generators_produce_valid_output() {
        let mut gen = Gen::new(54321);

        // Just verify generators don't panic and produce reasonable output
        for _ in 0..100 {
            let d = gen.date();
            assert!(d.len() >= 7 && d.len() <= 10, "Date length: {}", d.len());

            let t = gen.time();
            assert!(t.len() >= 5, "Time length: {}", t.len());

            let dt = gen.datetime();
            assert!(dt.len() >= 16, "DateTime length: {}", dt.len());

            let dur = gen.duration();
            assert!(!dur.is_empty(), "Duration empty");

            let rel = gen.relative_time();
            assert!(rel[0] == b'+' || rel[0] == b'-', "RelativeTime sign");
        }
    }
}
