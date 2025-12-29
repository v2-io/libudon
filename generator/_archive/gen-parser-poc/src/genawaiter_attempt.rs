//! Generator-based Parser using genawaiter - Stable Rust Attempt
//!
//! This module attempts to reimplement the gen blocks PoC using genawaiter,
//! which provides generator functionality on stable Rust via async/await.
//!
//! Key questions:
//! - Does it compile on stable?
//! - Does it handle borrow across yield (E0626)?
//! - Does it support recursive generators (E0733)?

use genawaiter::rc::Gen;
use std::ops::Range;

/// A simplified event for the PoC (same as lib.rs).
#[derive(Debug, Clone, PartialEq)]
pub enum Event<'a> {
    ElementStart { name: &'a [u8], span: Range<usize> },
    ElementEnd { span: Range<usize> },
    Text { content: &'a [u8], span: Range<usize> },
    Name { name: &'a [u8], span: Range<usize> },
    AttrKey { key: &'a [u8], span: Range<usize> },
    AttrValue { value: &'a [u8], span: Range<usize> },
}

/// Parser state - holds input and position.
pub struct Parser<'a> {
    input: &'a [u8],
    pos: usize,
    line: usize,
    col: usize,
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a [u8]) -> Self {
        Self {
            input,
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    #[inline]
    fn peek(&self) -> Option<u8> {
        self.input.get(self.pos).copied()
    }

    #[inline]
    fn advance(&mut self) {
        if self.pos < self.input.len() {
            if self.input[self.pos] == b'\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
            self.pos += 1;
        }
    }

    #[inline]
    fn eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    fn skip_horizontal_ws(&mut self) {
        while let Some(b' ' | b'\t') = self.peek() {
            self.advance();
        }
    }

    #[inline]
    fn is_label_char(b: u8) -> bool {
        b.is_ascii_alphanumeric() || b == b'_' || b == b'-'
    }

    fn count_indent(&mut self) -> i32 {
        let mut count = 0;
        while let Some(b' ') = self.peek() {
            self.advance();
            count += 1;
        }
        count
    }

    /// Parse document using genawaiter - FLAT approach (explicit stack).
    ///
    /// This mirrors the nightly gen blocks approach but uses genawaiter.
    /// Using the rc::Gen variant which heap-allocates but is simpler.
    ///
    /// Note: genawaiter's gen! macro conflicts with the reserved `gen` keyword
    /// in Rust 2024 edition, so we use the explicit Gen::new approach instead.
    pub fn parse_flat(mut self) -> impl Iterator<Item = Event<'a>> {
        Gen::new(|co| async move {
            let mut stack: Vec<i32> = Vec::new();

            while !self.eof() {
                // Skip blank lines
                while let Some(b'\n') = self.peek() {
                    self.advance();
                }

                if self.eof() {
                    break;
                }

                let col = self.count_indent();

                // Dedent handling
                while let Some(&elem_col) = stack.last() {
                    if col <= elem_col {
                        stack.pop();
                        co.yield_(Event::ElementEnd { span: self.pos..self.pos }).await;
                    } else {
                        break;
                    }
                }

                match self.peek() {
                    Some(b'|') => {
                        self.advance();
                        stack.push(col);

                        let start = self.pos;
                        co.yield_(Event::ElementStart { name: b"", span: start..start }).await;

                        // Parse name
                        if let Some(b) = self.peek() {
                            if b.is_ascii_alphabetic() {
                                let name_start = self.pos;
                                while let Some(b) = self.peek() {
                                    if Self::is_label_char(b) {
                                        self.advance();
                                    } else {
                                        break;
                                    }
                                }
                                co.yield_(Event::Name {
                                    name: &self.input[name_start..self.pos],
                                    span: name_start..self.pos,
                                }).await;
                            }
                        }

                        // Parse bracket [id]
                        if let Some(b'[') = self.peek() {
                            self.advance();
                            co.yield_(Event::AttrKey { key: b"$id", span: self.pos..self.pos }).await;

                            let val_start = self.pos;
                            while let Some(b) = self.peek() {
                                if b == b']' { break; }
                                self.advance();
                            }
                            co.yield_(Event::AttrValue {
                                value: &self.input[val_start..self.pos],
                                span: val_start..self.pos,
                            }).await;

                            if let Some(b']') = self.peek() {
                                self.advance();
                            }
                        }

                        self.skip_horizontal_ws();

                        // Inline text
                        if let Some(b) = self.peek() {
                            if b != b'\n' {
                                let text_start = self.pos;
                                while let Some(b) = self.peek() {
                                    if b == b'\n' { break; }
                                    self.advance();
                                }
                                if text_start < self.pos {
                                    co.yield_(Event::Text {
                                        content: &self.input[text_start..self.pos],
                                        span: text_start..self.pos,
                                    }).await;
                                }
                            }
                        }

                        if let Some(b'\n') = self.peek() {
                            self.advance();
                        }
                    }
                    Some(b'\n') | None => {}
                    Some(_) => {
                        let text_start = self.pos;
                        while let Some(b) = self.peek() {
                            if b == b'\n' { break; }
                            self.advance();
                        }
                        if text_start < self.pos {
                            co.yield_(Event::Text {
                                content: &self.input[text_start..self.pos],
                                span: text_start..self.pos,
                            }).await;
                        }
                        if let Some(b'\n') = self.peek() {
                            self.advance();
                        }
                    }
                }
            }

            while stack.pop().is_some() {
                co.yield_(Event::ElementEnd { span: self.pos..self.pos }).await;
            }
        }).into_iter()
    }
}

// ============================================================================
// EXPERIMENT: Recursive generators with genawaiter
//
// Can we do actual recursion? Let's try...
// ============================================================================

/// Attempt at recursive parsing using genawaiter.
/// This is to test if E0733 (recursive generator) is solved.
pub struct RecursiveParser<'a> {
    input: &'a [u8],
    pos: usize,
}

impl<'a> RecursiveParser<'a> {
    pub fn new(input: &'a [u8]) -> Self {
        Self { input, pos: 0 }
    }

    #[inline]
    fn peek(&self) -> Option<u8> {
        self.input.get(self.pos).copied()
    }

    #[inline]
    fn advance(&mut self) {
        if self.pos < self.input.len() {
            self.pos += 1;
        }
    }

    #[inline]
    fn eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    fn count_indent(&mut self) -> i32 {
        let mut count = 0;
        while let Some(b' ') = self.peek() {
            self.advance();
            count += 1;
        }
        count
    }

    #[inline]
    fn is_label_char(b: u8) -> bool {
        b.is_ascii_alphanumeric() || b == b'_' || b == b'-'
    }
}

/// The key test: Can we yield events from a recursive function?
///
/// With native gen blocks, this would be:
/// ```ignore
/// gen fn parse_element(&mut self, parent_col: i32) -> Event<'a> { ... }
/// ```
/// But gen blocks require special handling for recursion.
///
/// With genawaiter, recursion requires boxing the future returned by async.
/// Let's see if that works here.
#[cfg(feature = "recursive_experiment")]
mod recursive_experiment {
    use super::*;
    use std::future::Future;
    use std::pin::Pin;

    // This would be the recursive approach - but it requires boxing
    // async fn parse_element_recursive<'a>(
    //     parser: &'a mut RecursiveParser<'a>,
    //     co: &genawaiter::rc::Co<Event<'a>>,
    //     parent_col: i32,
    // ) -> Pin<Box<dyn Future<Output = ()> + 'a>> {
    //     Box::pin(async move {
    //         // ... parsing logic
    //         // Can we call ourselves recursively here?
    //         // parse_element_recursive(parser, co, new_col).await;
    //     })
    // }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_genawaiter_simple() {
        let input = b"|div Hello\n";
        let parser = Parser::new(input);
        let events: Vec<_> = parser.parse_flat().collect();

        println!("Events: {:#?}", events);

        assert!(matches!(events[0], Event::ElementStart { .. }));
        assert!(matches!(events[1], Event::Name { name, .. } if name == b"div"));
        assert!(matches!(events[2], Event::Text { content, .. } if content == b"Hello"));
        assert!(matches!(events[3], Event::ElementEnd { .. }));
    }

    #[test]
    fn test_genawaiter_nested() {
        let input = b"|div\n  |span Nested\n";
        let parser = Parser::new(input);
        let events: Vec<_> = parser.parse_flat().collect();

        println!("Events: {:#?}", events);

        let names: Vec<_> = events.iter()
            .filter_map(|e| match e {
                Event::Name { name, .. } => Some(String::from_utf8_lossy(name).to_string()),
                _ => None,
            })
            .collect();

        assert_eq!(names, vec!["div", "span"]);
    }

    #[test]
    fn test_genawaiter_dedent() {
        let input = b"|a\n  |b Child\n|c Sibling\n";
        let parser = Parser::new(input);
        let events: Vec<_> = parser.parse_flat().collect();

        println!("Events: {:#?}", events);

        let starts = events.iter().filter(|e| matches!(e, Event::ElementStart { .. })).count();
        let ends = events.iter().filter(|e| matches!(e, Event::ElementEnd { .. })).count();

        assert_eq!(starts, 3);
        assert_eq!(ends, 3);

        let names: Vec<_> = events.iter()
            .filter_map(|e| match e {
                Event::Name { name, .. } => Some(String::from_utf8_lossy(name).to_string()),
                _ => None,
            })
            .collect();

        assert_eq!(names, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_genawaiter_backpressure() {
        let input = b"|a\n  |b\n    |c Deep\n";
        let parser = Parser::new(input);
        let mut iter = parser.parse_flat();

        // Pull one event at a time
        let e1 = iter.next();
        println!("Event 1: {:?}", e1);

        let e2 = iter.next();
        println!("Event 2: {:?}", e2);

        let rest: Vec<_> = iter.collect();
        println!("Rest: {:#?}", rest);
    }

    #[test]
    fn test_genawaiter_bracket_id() {
        let input = b"|div[myid] Content\n";
        let parser = Parser::new(input);
        let events: Vec<_> = parser.parse_flat().collect();

        println!("Events: {:#?}", events);

        assert!(events.iter().any(|e| matches!(e, Event::AttrKey { key, .. } if *key == b"$id")));
        assert!(events.iter().any(|e| matches!(e, Event::AttrValue { value, .. } if *value == b"myid")));
    }

}
