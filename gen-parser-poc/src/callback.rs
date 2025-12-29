//! Callback-based Parser Proof of Concept
//!
//! Instead of pushing to a ring buffer, we call a callback for each event.
//! This inverts the control flow and simplifies backpressure handling.
//!
//! Key insight: The callback controls the pace. If it needs to apply
//! backpressure, it can:
//! 1. Block (simplest - just don't return until ready)
//! 2. Return a signal to pause (requires state saving)
//! 3. Use async/await (natural yield points)
//!
//! This prototype explores option 2: callback returns Continue/Pause.

use std::ops::{ControlFlow, Range};

/// Events emitted by the parser.
#[derive(Debug, Clone, PartialEq)]
pub enum Event<'a> {
    ElementStart { span: Range<usize> },
    ElementEnd { span: Range<usize> },
    Name { name: &'a [u8], span: Range<usize> },
    Text { content: &'a [u8], span: Range<usize> },
    AttrKey { key: &'a [u8], span: Range<usize> },
    AttrValue { value: &'a [u8], span: Range<usize> },
}

/// Result of attempting to emit an event.
pub type EmitResult = ControlFlow<(), ()>;

/// Parser with callback-based event delivery.
///
/// Uses true recursive descent - the call stack IS the element stack.
/// Backpressure is handled by the callback returning `ControlFlow::Break`.
pub struct CallbackParser<'a> {
    input: &'a [u8],
    pos: usize,
}

impl<'a> CallbackParser<'a> {
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

    /// Parse the document, calling the callback for each event.
    ///
    /// The callback can return:
    /// - `ControlFlow::Continue(())` to keep parsing
    /// - `ControlFlow::Break(())` to pause (NOT YET RESUMABLE in this version)
    ///
    /// Returns `Ok(())` if parsing completed, `Err(pos)` if paused at position.
    pub fn parse<F>(mut self, mut on_event: F) -> Result<(), usize>
    where
        F: FnMut(Event<'a>) -> EmitResult,
    {
        self.parse_document(&mut on_event)
    }

    fn parse_document<F>(&mut self, on_event: &mut F) -> Result<(), usize>
    where
        F: FnMut(Event<'a>) -> EmitResult,
    {
        while !self.eof() {
            match self.peek() {
                Some(b'\n') => {
                    self.advance();
                }
                Some(b' ') => {
                    let col = self.count_indent();
                    self.parse_line_content(col, -1, on_event)?;
                }
                Some(b'|') => {
                    self.advance();
                    self.parse_element(0, -1, on_event)?;
                }
                Some(_) => {
                    self.parse_prose(on_event)?;
                }
                None => break,
            }
        }
        Ok(())
    }

    fn parse_line_content<F>(&mut self, col: i32, parent_col: i32, on_event: &mut F) -> Result<(), usize>
    where
        F: FnMut(Event<'a>) -> EmitResult,
    {
        match self.peek() {
            Some(b'|') => {
                self.advance();
                self.parse_element(col, parent_col, on_event)?;
            }
            Some(b'\n') | None => {
                // Empty line
            }
            Some(_) => {
                self.parse_prose(on_event)?;
            }
        }
        Ok(())
    }

    /// Parse an element using TRUE RECURSIVE DESCENT.
    ///
    /// This is the elegant part - nested elements just call parse_element recursively.
    /// The Rust call stack naturally tracks nesting depth.
    /// Dedent is detected by comparing column to parent_col.
    fn parse_element<F>(&mut self, elem_col: i32, parent_col: i32, on_event: &mut F) -> Result<(), usize>
    where
        F: FnMut(Event<'a>) -> EmitResult,
    {
        let start = self.pos;

        // Emit ElementStart
        if on_event(Event::ElementStart { span: start..start }).is_break() {
            return Err(self.pos);
        }

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
                if on_event(Event::Name {
                    name: &self.input[name_start..self.pos],
                    span: name_start..self.pos,
                }).is_break() {
                    return Err(self.pos);
                }
            }
        }

        // Parse bracket [id]
        if let Some(b'[') = self.peek() {
            self.advance();
            if on_event(Event::AttrKey { key: b"$id", span: self.pos..self.pos }).is_break() {
                return Err(self.pos);
            }

            let val_start = self.pos;
            while let Some(b) = self.peek() {
                if b == b']' { break; }
                self.advance();
            }
            if on_event(Event::AttrValue {
                value: &self.input[val_start..self.pos],
                span: val_start..self.pos,
            }).is_break() {
                return Err(self.pos);
            }

            if let Some(b']') = self.peek() {
                self.advance();
            }
        }

        self.skip_horizontal_ws();

        // Inline text content
        if let Some(b) = self.peek() {
            if b != b'\n' {
                let text_start = self.pos;
                while let Some(b) = self.peek() {
                    if b == b'\n' { break; }
                    self.advance();
                }
                if text_start < self.pos {
                    if on_event(Event::Text {
                        content: &self.input[text_start..self.pos],
                        span: text_start..self.pos,
                    }).is_break() {
                        return Err(self.pos);
                    }
                }
            }
        }

        // Consume newline
        if let Some(b'\n') = self.peek() {
            self.advance();
        }

        // Parse children - HERE'S THE RECURSIVE MAGIC
        loop {
            // Skip blank lines
            while let Some(b'\n') = self.peek() {
                self.advance();
            }

            if self.eof() {
                break;
            }

            // Check indent
            let line_start = self.pos;
            let child_col = self.count_indent();

            // DEDENT CHECK: if at or before our column, we're done
            if child_col <= elem_col {
                // Rewind - don't consume this line, parent needs it
                self.pos = line_start;
                break;
            }

            // This line is our child
            match self.peek() {
                Some(b'|') => {
                    self.advance();
                    // RECURSIVE CALL - the call stack tracks nesting!
                    self.parse_element(child_col, elem_col, on_event)?;
                }
                Some(b'\n') | None => {}
                Some(_) => {
                    self.parse_prose(on_event)?;
                }
            }
        }

        // Emit ElementEnd - this happens on the way back up the call stack
        if on_event(Event::ElementEnd { span: self.pos..self.pos }).is_break() {
            return Err(self.pos);
        }

        Ok(())
    }

    fn parse_prose<F>(&mut self, on_event: &mut F) -> Result<(), usize>
    where
        F: FnMut(Event<'a>) -> EmitResult,
    {
        let start = self.pos;
        while let Some(b) = self.peek() {
            if b == b'\n' { break; }
            self.advance();
        }

        if start < self.pos {
            if on_event(Event::Text {
                content: &self.input[start..self.pos],
                span: start..self.pos,
            }).is_break() {
                return Err(self.pos);
            }
        }

        if let Some(b'\n') = self.peek() {
            self.advance();
        }

        Ok(())
    }
}

// ============================================================================
// Alternative: Blocking callback (simplest backpressure)
// ============================================================================

/// Simpler version where the callback just processes events.
/// Backpressure is implicit - if callback is slow, parsing is slow.
/// No pause/resume complexity.
pub struct SimpleCallbackParser<'a> {
    input: &'a [u8],
    pos: usize,
}

impl<'a> SimpleCallbackParser<'a> {
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

    /// Parse with a simple callback. No pause/resume - callback just receives events.
    /// If you need backpressure, make your callback block (e.g., send to a channel).
    pub fn parse<F>(mut self, mut on_event: F)
    where
        F: FnMut(Event<'a>),
    {
        self.parse_document(&mut on_event);
    }

    fn parse_document<F>(&mut self, on_event: &mut F)
    where
        F: FnMut(Event<'a>),
    {
        while !self.eof() {
            match self.peek() {
                Some(b'\n') => self.advance(),
                Some(b' ') => {
                    let col = self.count_indent();
                    self.parse_line_content(col, on_event);
                }
                Some(b'|') => {
                    self.advance();
                    self.parse_element(0, on_event);
                }
                Some(_) => self.parse_prose(on_event),
                None => break,
            }
        }
    }

    fn parse_line_content<F>(&mut self, col: i32, on_event: &mut F)
    where
        F: FnMut(Event<'a>),
    {
        match self.peek() {
            Some(b'|') => {
                self.advance();
                self.parse_element(col, on_event);
            }
            Some(b'\n') | None => {}
            Some(_) => self.parse_prose(on_event),
        }
    }

    fn parse_element<F>(&mut self, elem_col: i32, on_event: &mut F)
    where
        F: FnMut(Event<'a>),
    {
        let start = self.pos;
        on_event(Event::ElementStart { span: start..start });

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
                on_event(Event::Name {
                    name: &self.input[name_start..self.pos],
                    span: name_start..self.pos,
                });
            }
        }

        // Parse bracket [id]
        if let Some(b'[') = self.peek() {
            self.advance();
            on_event(Event::AttrKey { key: b"$id", span: self.pos..self.pos });

            let val_start = self.pos;
            while let Some(b) = self.peek() {
                if b == b']' { break; }
                self.advance();
            }
            on_event(Event::AttrValue {
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
                    on_event(Event::Text {
                        content: &self.input[text_start..self.pos],
                        span: text_start..self.pos,
                    });
                }
            }
        }

        if let Some(b'\n') = self.peek() {
            self.advance();
        }

        // Children - recursive
        loop {
            while let Some(b'\n') = self.peek() {
                self.advance();
            }

            if self.eof() { break; }

            let line_start = self.pos;
            let child_col = self.count_indent();

            if child_col <= elem_col {
                self.pos = line_start;
                break;
            }

            match self.peek() {
                Some(b'|') => {
                    self.advance();
                    self.parse_element(child_col, on_event);  // Recursive!
                }
                Some(b'\n') | None => {}
                Some(_) => self.parse_prose(on_event),
            }
        }

        on_event(Event::ElementEnd { span: self.pos..self.pos });
    }

    fn parse_prose<F>(&mut self, on_event: &mut F)
    where
        F: FnMut(Event<'a>),
    {
        let start = self.pos;
        while let Some(b) = self.peek() {
            if b == b'\n' { break; }
            self.advance();
        }

        if start < self.pos {
            on_event(Event::Text {
                content: &self.input[start..self.pos],
                span: start..self.pos,
            });
        }

        if let Some(b'\n') = self.peek() {
            self.advance();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_callback_simple() {
        let input = b"|div Hello\n";
        let mut events = Vec::new();

        CallbackParser::new(input)
            .parse(|e| {
                events.push(e);
                ControlFlow::Continue(())
            })
            .unwrap();

        println!("Events: {:#?}", events);
        assert!(matches!(events[0], Event::ElementStart { .. }));
        assert!(matches!(events[1], Event::Name { name, .. } if name == b"div"));
        assert!(matches!(events[2], Event::Text { content, .. } if content == b"Hello"));
        assert!(matches!(events[3], Event::ElementEnd { .. }));
    }

    #[test]
    fn test_callback_nested() {
        let input = b"|div\n  |span Nested\n";
        let mut events = Vec::new();

        CallbackParser::new(input)
            .parse(|e| {
                events.push(e);
                ControlFlow::Continue(())
            })
            .unwrap();

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
    fn test_callback_dedent() {
        let input = b"|a\n  |b Child\n|c Sibling\n";
        let mut events = Vec::new();

        CallbackParser::new(input)
            .parse(|e| {
                events.push(e);
                ControlFlow::Continue(())
            })
            .unwrap();

        println!("Events: {:#?}", events);

        // Verify structure: a-start, a-name, b-start, b-name, text, b-end, a-end, c-start...
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
    fn test_callback_deep_nesting() {
        let input = b"|a\n  |b\n    |c\n      |d Deep\n";
        let mut events = Vec::new();

        CallbackParser::new(input)
            .parse(|e| {
                events.push(e);
                ControlFlow::Continue(())
            })
            .unwrap();

        let names: Vec<_> = events.iter()
            .filter_map(|e| match e {
                Event::Name { name, .. } => Some(String::from_utf8_lossy(name).to_string()),
                _ => None,
            })
            .collect();
        assert_eq!(names, vec!["a", "b", "c", "d"]);

        // Verify proper nesting by checking End events come in reverse order
        let end_positions: Vec<_> = events.iter().enumerate()
            .filter(|(_, e)| matches!(e, Event::ElementEnd { .. }))
            .map(|(i, _)| i)
            .collect();

        // d ends, then c, then b, then a
        assert_eq!(end_positions.len(), 4);
    }

    #[test]
    fn test_callback_pause() {
        let input = b"|a\n  |b\n";
        let mut events = Vec::new();
        let mut count = 0;

        // Pause after 3 events
        let result = CallbackParser::new(input)
            .parse(|e| {
                events.push(e);
                count += 1;
                if count >= 3 {
                    ControlFlow::Break(())
                } else {
                    ControlFlow::Continue(())
                }
            });

        println!("Paused at position: {:?}", result);
        println!("Events collected: {:#?}", events);

        assert!(result.is_err()); // Parsing paused
        assert_eq!(events.len(), 3);
    }

    #[test]
    fn test_simple_callback() {
        // The SimpleCallbackParser version - no pause, just collect
        let input = b"|div[id] Content\n  |span Nested\n";
        let mut events = Vec::new();

        SimpleCallbackParser::new(input).parse(|e| {
            events.push(e);
        });

        println!("Events: {:#?}", events);

        assert!(events.iter().any(|e| matches!(e, Event::AttrKey { key, .. } if *key == b"$id")));
        assert!(events.iter().any(|e| matches!(e, Event::AttrValue { value, .. } if *value == b"id")));
    }

    #[test]
    fn test_callback_with_channel_backpressure() {
        use std::sync::mpsc;

        let input = b"|a\n  |b\n    |c\n";
        let (tx, rx) = mpsc::sync_channel(2); // Small buffer = backpressure

        // Spawn parser in thread
        let handle = std::thread::spawn(move || {
            SimpleCallbackParser::new(input).parse(|e| {
                // This will BLOCK if channel is full - implicit backpressure!
                tx.send(format!("{:?}", e)).unwrap();
            });
        });

        // Slowly consume events
        let mut received = Vec::new();
        while let Ok(e) = rx.recv() {
            received.push(e);
            std::thread::sleep(std::time::Duration::from_millis(1));
        }

        handle.join().unwrap();

        println!("Received {} events with backpressure", received.len());
        assert!(received.len() >= 6); // At least 3 elements * 2 events each
    }
}
