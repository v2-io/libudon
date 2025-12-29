//! Ring-buffer based Parser (simulating current approach)
//!
//! This mimics the architecture of the current libudon parser:
//! - Explicit element stack (simulated call stack)
//! - Ring buffer for events
//! - Flat state machine (no recursion)
//!
//! This is intentionally NOT using true recursion to match the current impl.

use std::ops::Range;

/// Events - same as other implementations for fair comparison.
#[derive(Debug, Clone, PartialEq)]
pub enum Event<'a> {
    ElementStart { span: Range<usize> },
    ElementEnd { span: Range<usize> },
    Name { name: &'a [u8], span: Range<usize> },
    Text { content: &'a [u8], span: Range<usize> },
    AttrKey { key: &'a [u8], span: Range<usize> },
    AttrValue { value: &'a [u8], span: Range<usize> },
}

/// Simple ring buffer for events.
pub struct EventRing<'a> {
    events: Vec<Option<Event<'a>>>,
    read_pos: usize,
    write_pos: usize,
    count: usize,
    capacity: usize,
    mask: usize,
}

impl<'a> EventRing<'a> {
    pub fn new(capacity: usize) -> Self {
        // Round up to power of 2
        let capacity = capacity.next_power_of_two();
        let mask = capacity - 1;
        Self {
            events: (0..capacity).map(|_| None).collect(),
            read_pos: 0,
            write_pos: 0,
            count: 0,
            capacity,
            mask,
        }
    }

    #[inline]
    pub fn push(&mut self, event: Event<'a>) -> bool {
        if self.count == self.capacity {
            return false; // Full
        }
        self.events[self.write_pos] = Some(event);
        self.write_pos = (self.write_pos + 1) & self.mask;
        self.count += 1;
        true
    }

    #[inline]
    pub fn pop(&mut self) -> Option<Event<'a>> {
        if self.count == 0 {
            return None;
        }
        let event = self.events[self.read_pos].take();
        self.read_pos = (self.read_pos + 1) & self.mask;
        self.count -= 1;
        event
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.count
    }
}

/// Ring-buffer based parser with explicit element stack.
/// Mimics the current libudon architecture.
pub struct RingBufferParser<'a> {
    input: &'a [u8],
    pos: usize,
    events: EventRing<'a>,
    element_stack: Vec<i32>, // Stack of element columns
}

impl<'a> RingBufferParser<'a> {
    pub fn new(input: &'a [u8], event_capacity: usize) -> Self {
        Self {
            input,
            pos: 0,
            events: EventRing::new(event_capacity),
            element_stack: Vec::with_capacity(32),
        }
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

    fn skip_horizontal_ws(&mut self) {
        while let Some(b' ' | b'\t') = self.peek() {
            self.advance();
        }
    }

    #[inline]
    fn emit(&mut self, event: Event<'a>) {
        // In real impl this would handle backpressure
        // For benchmark, we just push (assume buffer is big enough)
        self.events.push(event);
    }

    /// Parse the entire document, filling the ring buffer.
    pub fn parse(&mut self) {
        while !self.eof() {
            // Skip blank lines
            while let Some(b'\n') = self.peek() {
                self.advance();
            }

            if self.eof() {
                break;
            }

            // Count indent
            let col = self.count_indent();

            // Dedent: close elements at >= this column
            while let Some(&elem_col) = self.element_stack.last() {
                if col <= elem_col {
                    self.element_stack.pop();
                    self.emit(Event::ElementEnd { span: self.pos..self.pos });
                } else {
                    break;
                }
            }

            // Dispatch on content
            match self.peek() {
                Some(b'|') => {
                    self.advance();
                    self.parse_element_flat(col);
                }
                Some(b'\n') | None => {}
                Some(_) => {
                    self.parse_prose();
                }
            }
        }

        // Close remaining elements
        while self.element_stack.pop().is_some() {
            self.emit(Event::ElementEnd { span: self.pos..self.pos });
        }
    }

    /// Parse element WITHOUT recursion - uses explicit stack.
    fn parse_element_flat(&mut self, elem_col: i32) {
        self.element_stack.push(elem_col);
        let start = self.pos;

        self.emit(Event::ElementStart { span: start..start });

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
                self.emit(Event::Name {
                    name: &self.input[name_start..self.pos],
                    span: name_start..self.pos,
                });
            }
        }

        // Parse bracket [id]
        if let Some(b'[') = self.peek() {
            self.advance();
            self.emit(Event::AttrKey { key: b"$id", span: self.pos..self.pos });

            let val_start = self.pos;
            while let Some(b) = self.peek() {
                if b == b']' { break; }
                self.advance();
            }
            self.emit(Event::AttrValue {
                value: &self.input[val_start..self.pos],
                span: val_start..self.pos,
            });

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
                    self.emit(Event::Text {
                        content: &self.input[text_start..self.pos],
                        span: text_start..self.pos,
                    });
                }
            }
        }

        // Consume newline
        if let Some(b'\n') = self.peek() {
            self.advance();
        }

        // Note: children are handled by the main loop's dedent logic
        // The element_stack tracks what needs closing
    }

    fn parse_prose(&mut self) {
        let start = self.pos;
        while let Some(b) = self.peek() {
            if b == b'\n' { break; }
            self.advance();
        }

        if start < self.pos {
            self.emit(Event::Text {
                content: &self.input[start..self.pos],
                span: start..self.pos,
            });
        }

        if let Some(b'\n') = self.peek() {
            self.advance();
        }
    }

    /// Drain all events from the ring buffer.
    pub fn drain(&mut self) -> Vec<Event<'a>> {
        let mut events = Vec::with_capacity(self.events.len());
        while let Some(e) = self.events.pop() {
            events.push(e);
        }
        events
    }

    /// Read next event (for iterator-style consumption).
    pub fn read(&mut self) -> Option<Event<'a>> {
        self.events.pop()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ringbuffer_simple() {
        let input = b"|div Hello\n";
        let mut parser = RingBufferParser::new(input, 64);
        parser.parse();
        let events = parser.drain();

        println!("Events: {:#?}", events);
        assert!(matches!(events[0], Event::ElementStart { .. }));
        assert!(matches!(events[1], Event::Name { name, .. } if name == b"div"));
    }

    #[test]
    fn test_ringbuffer_nested() {
        let input = b"|a\n  |b\n    |c Deep\n";
        let mut parser = RingBufferParser::new(input, 64);
        parser.parse();
        let events = parser.drain();

        let names: Vec<_> = events.iter()
            .filter_map(|e| match e {
                Event::Name { name, .. } => Some(String::from_utf8_lossy(name).to_string()),
                _ => None,
            })
            .collect();
        assert_eq!(names, vec!["a", "b", "c"]);

        let starts = events.iter().filter(|e| matches!(e, Event::ElementStart { .. })).count();
        let ends = events.iter().filter(|e| matches!(e, Event::ElementEnd { .. })).count();
        assert_eq!(starts, 3);
        assert_eq!(ends, 3);
    }
}
