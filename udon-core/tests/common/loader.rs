//! Fixture loading from YAML files
//!
//! TODO: Once UDON parser is solid, convert fixtures to UDON format (dogfooding)

use serde::Deserialize;

#[allow(unused_imports)]
use std::path::Path;

/// A single test case from a fixture file
#[derive(Debug, Clone, Deserialize)]
pub struct TestCase {
    pub id: String,
    pub desc: String,
    pub udon: String,
    pub events: Vec<ExpectedEvent>,
}

/// Expected event - either a bare name or [name, content]
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ExpectedEvent {
    /// Bracket event with no content (ElementStart, ElementEnd, etc.)
    Bare(String),
    /// Content event [EventName, "content"]
    WithContent(String, String),
}

impl ExpectedEvent {
    pub fn name(&self) -> &str {
        match self {
            ExpectedEvent::Bare(name) => name,
            ExpectedEvent::WithContent(name, _) => name,
        }
    }

    pub fn content(&self) -> Option<&str> {
        match self {
            ExpectedEvent::Bare(_) => None,
            ExpectedEvent::WithContent(_, content) => Some(content),
        }
    }
}

/// Load all test cases from a YAML fixture file
pub fn load_fixtures(path: &Path) -> Vec<TestCase> {
    let content = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("Failed to read fixture file {:?}: {}", path, e));
    serde_yaml::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse fixture file {:?}: {}", path, e))
}

/// Load fixtures from the standard fixtures directory
pub fn load_fixtures_by_name(name: &str) -> Vec<TestCase> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(format!("{}.yaml", name));
    load_fixtures(&path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_elements() {
        let cases = load_fixtures_by_name("elements");
        assert!(!cases.is_empty());
        assert!(cases.iter().any(|c| c.id == "simple_element"));
    }
}
