use udon_core::{Parser, Event};

fn main() {
    let tests = [
        ("```\nfreeform content\n```", "basic freeform block"),
        ("```\n|not-an-element\n|another\n```", "freeform preserves pipes"),
        ("|code\n  ```\n  raw content\n  ```\n", "freeform inside element"),
        ("```\nunclosed", "unclosed freeform"),
    ];
    
    for (input, desc) in tests {
        println!("\n=== {} ===", desc);
        println!("Input: {:?}", input);
        println!("Events:");
        Parser::new(input.as_bytes()).parse(|event| {
            println!("  {:?}", event);
        });
    }
}
