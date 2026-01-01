//! Canonical tests loaded from YAML fixtures
//!
//! Runs each fixture test case:
//! 1. Canonical (exact input → exact events)
//! 2. With variations (stochastic context wrapping)
//!
//! Tests with empty `events: []` are TODO tests - they run the parser
//! to check for panics but don't compare output.

mod common;

use common::{load_fixtures_by_name, run_test, run_with_variations, Gen};

/// Run canonical tests for a fixture file
fn run_fixture(name: &str) {
    let cases = load_fixtures_by_name(name);
    let mut gen = Gen::from_env_or_random();
    let mut failures = Vec::new();
    let mut todo_count = 0;

    for case in &cases {
        // Track TODO tests
        if case.events.is_empty() {
            todo_count += 1;
        }

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

    if todo_count > 0 {
        eprintln!("  {} - {} tests ({} TODO with empty events)", name, cases.len(), todo_count);
    }
}

// === Core fixtures (fully specified) ===

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

#[test]
fn test_attributes() {
    run_fixture("attributes");
}

#[test]
fn test_comments() {
    run_fixture("comments");
}

#[test]
fn test_escapes() {
    run_fixture("escapes");
}

// === Fixtures needing events filled in ===

#[test]
fn test_arrays() {
    run_fixture("arrays");
}

#[test]
fn test_dynamics() {
    run_fixture("dynamics");
}

#[test]
fn test_embedded_elements() {
    run_fixture("embedded_elements");
}

#[test]
fn test_inline_element_nesting() {
    run_fixture("inline_element_nesting");
}

#[test]
fn test_references() {
    run_fixture("references");
}

#[test]
fn test_freeform_blocks() {
    run_fixture("freeform_blocks");
}

#[test]
fn test_inline_comments() {
    run_fixture("inline_comments");
}

#[test]
fn test_inline_attributes() {
    run_fixture("inline_attributes");
}

#[test]
fn test_value_types() {
    run_fixture("value_types");
}

#[test]
fn test_element_names() {
    run_fixture("element_names");
}

#[test]
fn test_element_id() {
    run_fixture("element_id");
}

#[test]
fn test_element_class() {
    run_fixture("element_class");
}

#[test]
fn test_element_suffix() {
    run_fixture("element_suffix");
}

#[test]
fn test_element_combined() {
    run_fixture("element_combined");
}

#[test]
fn test_element_recognition() {
    run_fixture("element_recognition");
}

#[test]
fn test_text() {
    run_fixture("text");
}

#[test]
fn test_prose_dedentation() {
    run_fixture("prose_dedentation");
}

#[test]
fn test_indentation_hierarchy() {
    run_fixture("indentation_hierarchy");
}

#[test]
fn test_indentation_edge_cases() {
    run_fixture("indentation_edge_cases");
}

#[test]
fn test_comment_indentation() {
    run_fixture("comment_indentation");
}

#[test]
fn test_comments_and_text() {
    run_fixture("comments_and_text");
}

#[test]
fn test_suffix_positions() {
    run_fixture("suffix_positions");
}

#[test]
fn test_error_cases() {
    run_fixture("error_cases");
}

#[test]
fn test_escape_prefix() {
    run_fixture("escape_prefix");
}

#[test]
fn test_literal_escape() {
    run_fixture("literal_escape");
}

// Integration test: run all fixtures
#[test]
#[ignore] // Run with --ignored for full suite
fn test_all_fixtures() {
    let all_fixtures = [
        "elements", "values", "indentation", "attributes", "comments", "escapes",
        "arrays", "dynamics", "embedded_elements", "inline_element_nesting",
        "references", "freeform_blocks", "inline_comments", "inline_attributes",
        "value_types", "element_names", "element_id", "element_class",
        "element_suffix", "element_combined", "element_recognition", "text",
        "prose_dedentation", "indentation_hierarchy", "indentation_edge_cases",
        "comment_indentation", "comments_and_text", "suffix_positions",
        "error_cases", "escape_prefix", "literal_escape",
    ];
    for name in &all_fixtures {
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
