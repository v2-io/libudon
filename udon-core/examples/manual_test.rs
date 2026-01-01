use udon_core::Parser;

fn main() {
    let inputs = [
        ("''literal-apostrophe\n", "escaped apostrophe"),
        ("';not-a-comment\n", "escaped semicolon"),
        ("'|not-an-element\n", "escaped pipe"),
    ];

    for (input, desc) in inputs {
        println!("\n=== {} ===\nInput: {:?}", desc, input);
        Parser::new(input.as_bytes()).parse(|e| {
            println!("  {}", e.format_line());
        });
    }
}
