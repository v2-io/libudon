//! Nightly gen blocks implementation
//! This module requires nightly Rust + the "nightly" feature

use std::ops::Range;

/// A simplified event for the PoC.
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

    /// Parse document - FLAT approach (no recursion in gen block).
    ///
    /// Instead of recursive gen blocks, we use an explicit stack.
    /// This is essentially the "simulated call stack" approach but
    /// with gen block syntax for yielding.
    pub fn parse_flat(mut self) -> impl Iterator<Item = Event<'a>> {
        gen move {
            // Stack of (elem_col, parent_col) for open elements
            let mut stack: Vec<i32> = Vec::new();

            while !self.eof() {
                // Skip blank lines
                while let Some(b'\n') = self.peek() {
                    self.advance();
                }

                if self.eof() {
                    break;
                }

                // Count indent
                let _line_start = self.pos;
                let col = self.count_indent();

                // Dedent: close elements whose column >= current
                while let Some(&elem_col) = stack.last() {
                    if col <= elem_col {
                        stack.pop();
                        yield Event::ElementEnd { span: self.pos..self.pos };
                    } else {
                        break;
                    }
                }

                // Now dispatch on content
                match self.peek() {
                    Some(b'|') => {
                        self.advance();
                        stack.push(col);

                        let start = self.pos;
                        yield Event::ElementStart { name: b"", span: start..start };

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
                                yield Event::Name {
                                    name: &self.input[name_start..self.pos],
                                    span: name_start..self.pos,
                                };
                            }
                        }

                        // Parse bracket [id]
                        if let Some(b'[') = self.peek() {
                            self.advance();
                            yield Event::AttrKey { key: b"$id", span: self.pos..self.pos };

                            let val_start = self.pos;
                            while let Some(b) = self.peek() {
                                if b == b']' { break; }
                                self.advance();
                            }
                            yield Event::AttrValue {
                                value: &self.input[val_start..self.pos],
                                span: val_start..self.pos,
                            };

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
                                    yield Event::Text {
                                        content: &self.input[text_start..self.pos],
                                        span: text_start..self.pos,
                                    };
                                }
                            }
                        }

                        // Consume newline
                        if let Some(b'\n') = self.peek() {
                            self.advance();
                        }
                    }
                    Some(b'\n') | None => {
                        // Empty line
                    }
                    Some(_) => {
                        // Prose
                        let text_start = self.pos;
                        while let Some(b) = self.peek() {
                            if b == b'\n' { break; }
                            self.advance();
                        }
                        if text_start < self.pos {
                            yield Event::Text {
                                content: &self.input[text_start..self.pos],
                                span: text_start..self.pos,
                            };
                        }
                        if let Some(b'\n') = self.peek() {
                            self.advance();
                        }
                    }
                }
            }

            // Close remaining elements
            while stack.pop().is_some() {
                yield Event::ElementEnd { span: self.pos..self.pos };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flat_simple() {
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
    fn test_flat_nested() {
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
    fn test_flat_dedent() {
        let input = b"|a\n  |b Child\n|c Sibling\n";
        let parser = Parser::new(input);
        let events: Vec<_> = parser.parse_flat().collect();

        println!("Events: {:#?}", events);

        // Count ElementStart and ElementEnd
        let starts = events.iter().filter(|e| matches!(e, Event::ElementStart { .. })).count();
        let ends = events.iter().filter(|e| matches!(e, Event::ElementEnd { .. })).count();

        assert_eq!(starts, 3); // a, b, c
        assert_eq!(ends, 3);   // b closes, a closes, c closes

        let names: Vec<_> = events.iter()
            .filter_map(|e| match e {
                Event::Name { name, .. } => Some(String::from_utf8_lossy(name).to_string()),
                _ => None,
            })
            .collect();

        assert_eq!(names, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_backpressure() {
        // With gen blocks, backpressure is natural - we control iteration pace
        let input = b"|a\n  |b\n    |c Deep\n";
        let parser = Parser::new(input);
        let mut iter = parser.parse_flat();

        // Pull one event at a time - parser pauses between yields
        let e1 = iter.next();
        println!("Event 1: {:?}", e1);

        // Could do other work here...

        let e2 = iter.next();
        println!("Event 2: {:?}", e2);

        // Continue when ready
        let rest: Vec<_> = iter.collect();
        println!("Rest: {:#?}", rest);
    }

    #[test]
    fn test_bracket_id() {
        let input = b"|div[myid] Content\n";
        let parser = Parser::new(input);
        let events: Vec<_> = parser.parse_flat().collect();

        println!("Events: {:#?}", events);

        assert!(events.iter().any(|e| matches!(e, Event::AttrKey { key, .. } if *key == b"$id")));
        assert!(events.iter().any(|e| matches!(e, Event::AttrValue { value, .. } if *value == b"myid")));
    }
}
