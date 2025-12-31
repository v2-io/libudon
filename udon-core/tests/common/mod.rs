//! Test infrastructure for UDON parser
//!
//! Provides fixture loading, stochastic test generation, and assertion helpers.

mod loader;
mod harness;
mod generators;

pub use loader::{TestCase, ExpectedEvent, load_fixtures_by_name};
pub use harness::{run_test, run_with_variations};
pub use generators::Gen;
