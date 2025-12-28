use udon_core::StreamingParser;
use udon_core::StreamingEvent;

fn main() {
    let mut parser = StreamingParser::new(1024);
    parser.feed(b"!if logged_in\n  |p Welcome!\n!else\n  |p Please login");
    parser.finish();
    
    println!("=== Events ===");
    while let Some(event) = parser.read() {
        match &event {
            StreamingEvent::DirectiveStart { name, raw, .. } => {
                if let Some(n) = parser.arena().resolve(*name) {
                    println!("DirStart({:?}, {})", String::from_utf8_lossy(n), raw);
                }
            }
            StreamingEvent::DirectiveStatement { content, .. } => {
                if let Some(c) = parser.arena().resolve(*content) {
                    println!("DirStmt({:?})", String::from_utf8_lossy(c));
                }
            }
            StreamingEvent::DirectiveEnd { .. } => println!("DirEnd"),
            StreamingEvent::ElementStart { name, .. } => {
                if let Some(n) = name {
                    if let Some(nm) = parser.arena().resolve(*n) {
                        println!("  ElementStart({:?})", String::from_utf8_lossy(nm));
                    }
                } else {
                    println!("  ElementStartAnon");
                }
            }
            StreamingEvent::ElementEnd { .. } => println!("  ElementEnd"),
            StreamingEvent::Text { content, .. } => {
                if let Some(t) = parser.arena().resolve(*content) {
                    println!("    Text({:?})", String::from_utf8_lossy(t));
                }
            }
            e => println!("{:?}", e),
        }
    }
}
