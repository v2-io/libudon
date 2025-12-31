use udon_core::{Parser, Event};

fn main() {
    let input = b"|div[myid].class1 Hello World\n  |span nested\n";
    
    println!("Input: {:?}\n", std::str::from_utf8(input).unwrap());
    println!("Events:");
    
    Parser::new(input).parse(|event| {
        match &event {
            Event::Name { content, .. } => {
                println!("  Name: {:?}", std::str::from_utf8(content).unwrap());
            }
            Event::Text { content, .. } => {
                println!("  Text: {:?}", std::str::from_utf8(content).unwrap());
            }
            Event::Attr { content, .. } => {
                println!("  Attr: {:?}", std::str::from_utf8(content).unwrap());
            }
            Event::StringValue { content, .. } => {
                println!("  StringValue: {:?}", std::str::from_utf8(content).unwrap());
            }
            _ => {
                println!("  {:?}", event);
            }
        }
    });
}
