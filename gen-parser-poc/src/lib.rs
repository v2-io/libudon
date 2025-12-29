//! Generator-based Parser Proof of Concept - Attempt 2
//!
//! Working around:
//! - E0626: borrow across yield (self-referential generators)
//! - E0733: recursion requires boxing
//!
//! Strategy: Don't borrow self while yielding - collect or inline instead.
//!
//! This crate provides two implementations:
//! - `genawaiter_attempt`: Works on stable Rust using the genawaiter crate
//! - `nightly`: Uses native gen blocks (requires nightly + "nightly" feature)

// Only enable gen_blocks on nightly when the feature is enabled
#![cfg_attr(feature = "nightly", feature(gen_blocks))]

// Genawaiter-based implementation (works on stable Rust)
pub mod genawaiter_attempt;

// Callback-based implementation (works on stable Rust, true recursive descent)
pub mod callback;

// Ring-buffer based implementation (mimics current libudon architecture)
pub mod ringbuffer;

// Nightly gen_blocks implementation - only compile on nightly with feature enabled
// The separate file approach prevents parsing on stable Rust
#[cfg(feature = "nightly")]
#[path = "nightly.rs"]
pub mod nightly;
