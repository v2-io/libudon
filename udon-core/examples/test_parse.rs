use udon_core::Parser;

fn main() {
    // Embedded element error cases
    let tests = [
        ("|p This has |{em unclosed", "unclosed_embedded_element_error"),
        ("|p |{a |{b text}", "unclosed_nested_embedded_error"),
    ];

    for (input, desc) in tests {
        println!("\n=== {} ===", desc);
        println!("Input: {:?}", input);
        Parser::new(input.as_bytes()).parse(|event| {
            println!("  {:?}", event);
        });
    }
}
