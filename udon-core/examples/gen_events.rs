//! Generate events for fixture inputs
//! Usage: cargo run --example gen_events < input.yaml > output.yaml

use std::io::{self, BufRead, Write};
use udon_core::Parser;

fn format_event(event: &udon_core::Event) -> String {
    use udon_core::Event::*;
    match event {
        ElementStart { .. } => "ElementStart".to_string(),
        ElementEnd { .. } => "ElementEnd".to_string(),
        EmbeddedStart { .. } => "EmbeddedStart".to_string(),
        EmbeddedEnd { .. } => "EmbeddedEnd".to_string(),
        DirectiveStart { .. } => "DirectiveStart".to_string(),
        DirectiveEnd { .. } => "DirectiveEnd".to_string(),
        ArrayStart { .. } => "ArrayStart".to_string(),
        ArrayEnd { .. } => "ArrayEnd".to_string(),
        FreeformStart { .. } => "FreeformStart".to_string(),
        FreeformEnd { .. } => "FreeformEnd".to_string(),
        Name { content, .. } => format!("[Name, {:?}]", String::from_utf8_lossy(content)),
        Text { content, .. } => format!("[Text, {:?}]", String::from_utf8_lossy(content)),
        Comment { content, .. } => format!("[Comment, {:?}]", String::from_utf8_lossy(content)),
        Attr { content, .. } => format!("[Attr, {:?}]", String::from_utf8_lossy(content)),
        StringValue { content, .. } => format!("[StringValue, {:?}]", String::from_utf8_lossy(content)),
        BareValue { content, .. } => format!("[BareValue, {:?}]", String::from_utf8_lossy(content)),
        BoolTrue { .. } => "BoolTrue".to_string(),
        BoolFalse { .. } => "BoolFalse".to_string(),
        Nil { .. } => "Nil".to_string(),
        Interpolation { content, .. } => format!("[Interpolation, {:?}]", String::from_utf8_lossy(content)),
        Reference { content, .. } => format!("[Reference, {:?}]", String::from_utf8_lossy(content)),
        RawContent { content, .. } => format!("[RawContent, {:?}]", String::from_utf8_lossy(content)),
        Raw { content, .. } => format!("[Raw, {:?}]", String::from_utf8_lossy(content)),
        Integer { content, .. } => format!("[Integer, {:?}]", String::from_utf8_lossy(content)),
        Float { content, .. } => format!("[Float, {:?}]", String::from_utf8_lossy(content)),
        Rational { content, .. } => format!("[Rational, {:?}]", String::from_utf8_lossy(content)),
        Complex { content, .. } => format!("[Complex, {:?}]", String::from_utf8_lossy(content)),
        Warning { content, .. } => format!("[Warning, {:?}]", String::from_utf8_lossy(content)),
        Error { code, .. } => format!("[Error, \"{:?}\"]", code),
    }
}

fn main() {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    
    // Read YAML from stdin, process each test, write YAML to stdout
    let mut in_udon = false;
    let mut udon_content = String::new();
    let mut indent = "";
    
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        
        if line.trim().starts_with("udon:") {
            // Check if it's inline or block
            let after_udon = line.trim().strip_prefix("udon:").unwrap().trim();
            if after_udon.starts_with('"') || after_udon.starts_with('\'') || after_udon.starts_with('|') {
                // Inline or block scalar
                writeln!(stdout, "{}", line).unwrap();
                if after_udon == "|" || after_udon == "|-" {
                    in_udon = true;
                    udon_content.clear();
                    indent = "    "; // assume 4 space indent
                } else {
                    // Inline string - parse it
                    // Extract quoted content
                    let udon = if after_udon.starts_with('"') {
                        serde_yaml::from_str::<String>(after_udon).unwrap_or_default()
                    } else {
                        after_udon.trim_matches('\'').to_string()
                    };
                    
                    // Generate events
                    let mut events = Vec::new();
                    Parser::new(udon.as_bytes()).parse(|e| events.push(format_event(&e)));
                    
                    // Skip the "events: []" line and write our events
                    // We'll handle this after we see the events line
                }
            }
            continue;
        }
        
        if in_udon {
            if line.starts_with("  ") && !line.trim().starts_with("events:") {
                udon_content.push_str(&line[2.min(line.len())..]);
                udon_content.push('\n');
                writeln!(stdout, "{}", line).unwrap();
                continue;
            } else {
                in_udon = false;
            }
        }
        
        if line.trim() == "events: []" {
            // Generate events for the udon we collected
            let udon = if udon_content.is_empty() {
                // Need to get from previous line context - skip for now
                String::new()
            } else {
                udon_content.clone()
            };
            
            if !udon.is_empty() {
                let mut events = Vec::new();
                Parser::new(udon.as_bytes()).parse(|e| events.push(format_event(&e)));
                
                writeln!(stdout, "  events:").unwrap();
                for ev in events {
                    writeln!(stdout, "    - {}", ev).unwrap();
                }
            } else {
                writeln!(stdout, "{}", line).unwrap();
            }
            udon_content.clear();
        } else {
            writeln!(stdout, "{}", line).unwrap();
        }
    }
}
