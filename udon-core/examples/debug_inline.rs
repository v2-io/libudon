use udon_core::StreamingParser;

fn main() {
    let mut parser = StreamingParser::new(1024);
    parser.feed(b"|p Text ;{TODO: fix this} more text.");
    parser.finish();
    
    println!("Events for: |p Text ;{{TODO: fix this}} more text.");
    while let Some(event) = parser.read() {
        println!("  {:?}", event);
    }
    
    parser.reset();
    parser.feed(b"|p First ;{note 1} middle ;{note 2} end.");
    parser.finish();
    
    println!("\nEvents for: |p First ;{{note 1}} middle ;{{note 2}} end.");
    while let Some(event) = parser.read() {
        println!("  {:?}", event);
    }
}
