use udon_core::StreamingParser;
use udon_core::StreamingEvent;

fn main() {
    println!("=== Test 1: prose with interpolation (child_prose) ===");
    test(b"|parent\n  before !{{middle}} after");

    println!("\n=== Test 2: inline content with interpolation (inline_text) ===");
    test(b"|parent before !{{middle}} after");
}

fn test(input: &[u8]) {
    let mut parser = StreamingParser::new(1024);
    parser.feed(input);
    parser.finish();

    while let Some(event) = parser.read() {
        match &event {
            StreamingEvent::Text { content, .. } => {
                let bytes = parser.arena().resolve(*content).unwrap_or(&[]);
                println!("Text: {:?} (len {})", String::from_utf8_lossy(bytes), bytes.len());
            }
            StreamingEvent::Interpolation { expression, .. } => {
                let bytes = parser.arena().resolve(*expression).unwrap_or(&[]);
                println!("Interp: {:?}", String::from_utf8_lossy(bytes));
            }
            StreamingEvent::ElementStart { name, .. } => {
                if let Some(n) = name {
                    if let Some(bytes) = parser.arena().resolve(*n) {
                        println!("ElementStart: {:?}", String::from_utf8_lossy(bytes));
                    }
                }
            }
            StreamingEvent::ElementEnd { .. } => println!("ElementEnd"),
            e => println!("Other: {:?}", e),
        }
    }
}
