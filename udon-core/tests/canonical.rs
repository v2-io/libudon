//! Canonical tests loaded from YAML fixtures
//!
//! Runs each fixture test case:
//! 1. Canonical (exact input → exact events)
//! 2. With variations (stochastic context wrapping)

mod common;

use common::{load_fixtures_by_name, run_test, run_with_variations, Gen};

/// Run canonical tests for a fixture file
fn run_fixture(name: &str) {
    let cases = load_fixtures_by_name(name);
    let mut gen = Gen::from_env_or_random();
    let mut failures = Vec::new();

    for case in &cases {
        // Canonical test (exact match)
        let result = run_test(case);
        if !result.passed {
            result.print_failure(&format!("{}::{} (canonical)", name, case.id));
            failures.push(format!("{}::{}", name, case.id));
        }

        // Variation tests (Poisson count, default λ=3)
        let variation_count = std::env::var("UDON_TEST_COUNT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| gen.poisson(3.0).max(1));

        for i in 0..variation_count {
            let result = run_with_variations(case, &mut gen);
            if !result.passed {
                result.print_failure(&format!("{}::{} (variation {})", name, case.id, i));
                failures.push(format!("{}::{} (var {})", name, case.id, i));
            }
        }
    }

    if !failures.is_empty() {
        panic!(
            "\n{} tests failed:\n  {}\n\nSeed: {} (set UDON_TEST_SEED={} to reproduce)",
            failures.len(),
            failures.join("\n  "),
            gen.seed,
            gen.seed
        );
    }
}

#[test]
fn test_elements() {
    run_fixture("elements");
}

#[test]
fn test_values() {
    run_fixture("values");
}

#[test]
fn test_indentation() {
    run_fixture("indentation");
}

// Integration test: run all fixtures
#[test]
#[ignore] // Run with --ignored for full suite
fn test_all_fixtures() {
    for name in &["elements", "values", "indentation"] {
        run_fixture(name);
    }
}

// Quick smoke test
#[test]
fn smoke_test() {
    use udon_core::Parser;

    let input = b"|div :class container\n  Hello world\n";
    let mut events = Vec::new();
    Parser::new(input).parse(|e| events.push(e.format_line()));

    assert!(!events.is_empty(), "Should produce events");
    assert!(
        events.iter().any(|e| e.contains("ElementStart")),
        "Should have ElementStart"
    );
    assert!(
        events.iter().any(|e| e.contains("ElementEnd")),
        "Should have ElementEnd"
    );
}
