use udon_core::{Parser, Event};

fn main() {
    let inputs = [
        ("|div Hello World", "simple element with text"),
        ("|div[myid].class1.class2 Content", "element with id and classes"),
        ("|ul\n  |li Item 1\n  |li Item 2", "nested elements"),
        ("|p Some |{em emphasis} here", "embedded element"),
        ("; A comment\n|div After", "comment then element"),
    ];
    
    for (input, desc) in inputs {
        println!("\n=== {} ===\n{:?}", desc, input);
        println!("Events:");
        Parser::new(input.as_bytes()).parse(|e| {
            println!("  {}", e.format_line());
        });
    }
}
