//! Test harness for running tests with stochastic variations

use crate::common::{TestCase, ExpectedEvent, Gen};
use udon_core::{Parser, Event};

/// Result of running a test
#[derive(Debug)]
pub struct TestResult {
    pub passed: bool,
    pub input: Vec<u8>,
    pub expected: Vec<String>,
    pub actual: Vec<String>,
    pub seed: u64,
    pub errors: Vec<String>,
}

/// Collect events from parsing
fn collect_events(input: &[u8]) -> Vec<Event<'_>> {
    let mut events = Vec::new();
    Parser::new(input).parse(|e| events.push(e));
    events
}

/// Format event for comparison (simplified, no spans)
fn format_event(event: &Event) -> String {
    match event {
        Event::ElementStart { .. } => "ElementStart".to_string(),
        Event::ElementEnd { .. } => "ElementEnd".to_string(),
        Event::EmbeddedStart { .. } => "EmbeddedStart".to_string(),
        Event::EmbeddedEnd { .. } => "EmbeddedEnd".to_string(),
        Event::DirectiveStart { .. } => "DirectiveStart".to_string(),
        Event::DirectiveEnd { .. } => "DirectiveEnd".to_string(),
        Event::ArrayStart { .. } => "ArrayStart".to_string(),
        Event::ArrayEnd { .. } => "ArrayEnd".to_string(),
        Event::FreeformStart { .. } => "FreeformStart".to_string(),
        Event::FreeformEnd { .. } => "FreeformEnd".to_string(),
        Event::Name { content, .. } => format!("Name {:?}", String::from_utf8_lossy(content)),
        Event::Text { content, .. } => format!("Text {:?}", String::from_utf8_lossy(content)),
        Event::Comment { content, .. } => format!("Comment {:?}", String::from_utf8_lossy(content)),
        Event::Attr { content, .. } => format!("Attr {:?}", String::from_utf8_lossy(content)),
        Event::StringValue { content, .. } => format!("StringValue {:?}", String::from_utf8_lossy(content)),
        Event::BareValue { content, .. } => format!("BareValue {:?}", String::from_utf8_lossy(content)),
        Event::BoolTrue { .. } => "BoolTrue".to_string(),
        Event::BoolFalse { .. } => "BoolFalse".to_string(),
        Event::Nil { .. } => "Nil".to_string(),
        Event::Interpolation { content, .. } => format!("Interpolation {:?}", String::from_utf8_lossy(content)),
        Event::Reference { content, .. } => format!("Reference {:?}", String::from_utf8_lossy(content)),
        Event::RawContent { content, .. } => format!("RawContent {:?}", String::from_utf8_lossy(content)),
        Event::Raw { content, .. } => format!("Raw {:?}", String::from_utf8_lossy(content)),
        Event::Integer { content, .. } => format!("Integer {:?}", String::from_utf8_lossy(content)),
        Event::Float { content, .. } => format!("Float {:?}", String::from_utf8_lossy(content)),
        Event::Error { code, .. } => format!("Error {:?}", code),
    }
}

/// Format expected event for comparison
fn format_expected(event: &ExpectedEvent) -> String {
    match event {
        ExpectedEvent::Bare(name) => name.clone(),
        ExpectedEvent::WithContent(name, content) => format!("{} {:?}", name, content),
    }
}

/// Run a single test case (canonical, no variations)
pub fn run_test(case: &TestCase) -> TestResult {
    let input = case.udon.as_bytes();
    let events = collect_events(input);

    let actual: Vec<String> = events.iter().map(format_event).collect();
    let expected: Vec<String> = case.events.iter().map(format_expected).collect();

    let mut errors = Vec::new();

    // Check event count
    if actual.len() != expected.len() {
        errors.push(format!(
            "Event count mismatch: expected {}, got {}",
            expected.len(),
            actual.len()
        ));
    }

    // Check each event
    for (i, (act, exp)) in actual.iter().zip(expected.iter()).enumerate() {
        if act != exp {
            errors.push(format!("Event {}: expected '{}', got '{}'", i, exp, act));
        }
    }

    TestResult {
        passed: errors.is_empty(),
        input: input.to_vec(),
        expected,
        actual,
        seed: 0,
        errors,
    }
}

/// Run test with stochastic variations
///
/// Applies independent variations:
/// - 40% chance of UDON above
/// - Geometric indent (α=0.9)
/// - Random blank lines
/// - 40% chance of UDON below
pub fn run_with_variations(case: &TestCase, gen: &mut Gen) -> TestResult {
    let mut input = Vec::new();

    // 40% chance: add UDON above
    if gen.chance(0.4) {
        input.extend(gen.udon_fragment(0));
    }

    // Determine indent level (geometric, α=0.9)
    let indent_level = gen.indent_level();
    let indent: Vec<u8> = vec![b' '; indent_level];

    // Add canonical test with indent and possible blank lines
    for line in case.udon.as_bytes().split(|&b| b == b'\n') {
        // Maybe inject blank line before
        input.extend(gen.blank_lines());

        if !line.is_empty() {
            input.extend(&indent);
            input.extend(line);
        }
        input.push(b'\n');
    }

    // 40% chance: add UDON below
    if gen.chance(0.4) {
        input.extend(gen.udon_fragment(indent_level));
    }

    // Parse and collect events
    let events = collect_events(&input);
    let actual: Vec<String> = events.iter().map(format_event).collect();
    let expected: Vec<String> = case.events.iter().map(format_expected).collect();

    // For variations, we check that expected events appear in order (subsequence match)
    // because we may have extra events from the wrapping context
    let mut errors = Vec::new();
    let mut exp_idx = 0;

    for act in &actual {
        if exp_idx < expected.len() && act == &expected[exp_idx] {
            exp_idx += 1;
        }
    }

    if exp_idx < expected.len() {
        errors.push(format!(
            "Missing expected events starting at index {}: {:?}",
            exp_idx,
            &expected[exp_idx..]
        ));
    }

    // Check for Error events (unless expected)
    for act in &actual {
        if act.starts_with("Error") && !expected.iter().any(|e| e.starts_with("Error")) {
            errors.push(format!("Unexpected error: {}", act));
        }
    }

    TestResult {
        passed: errors.is_empty(),
        input,
        expected,
        actual,
        seed: gen.seed,
        errors,
    }
}

impl TestResult {
    /// Print detailed failure info
    pub fn print_failure(&self, case_id: &str) {
        eprintln!("\n=== FAILED: {} ===", case_id);
        eprintln!("Seed: {} (set UDON_TEST_SEED={} to reproduce)", self.seed, self.seed);
        eprintln!("\nInput:");
        eprintln!("{}", String::from_utf8_lossy(&self.input));
        eprintln!("\nExpected events:");
        for (i, e) in self.expected.iter().enumerate() {
            eprintln!("  {}: {}", i, e);
        }
        eprintln!("\nActual events:");
        for (i, e) in self.actual.iter().enumerate() {
            eprintln!("  {}: {}", i, e);
        }
        eprintln!("\nErrors:");
        for e in &self.errors {
            eprintln!("  - {}", e);
        }
    }
}
