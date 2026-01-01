use udon_core::Parser;

fn main() {
    // Test the bug fixes
    let tests = [
        // Nested arrays
        ("|foo\n  :matrix [[1 2] [3 4]]\n", "nested arrays"),
        ("|foo\n  :deep [[[[[1]]]]]\n", "deeply nested"),

        // Quoted strings in arrays
        ("|foo\n  :names [\"Alice\" \"Bob\"]\n", "double quoted in array"),
        ("|foo\n  :names ['Alice' 'Bob']\n", "single quoted in array"),

        // Braces in embedded quoted strings
        ("|p |{code \"func() { return 1; }\"}", "braces in quoted content"),

        // Include directive at EOF (no trailing newline)
        ("!include partials/header", "include at EOF"),
    ];

    for (input, desc) in tests {
        println!("\n=== {} ===", desc);
        println!("Input: {:?}", input);
        Parser::new(input.as_bytes()).parse(|event| {
            println!("  {:?}", event);
        });
    }
}
