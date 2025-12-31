use udon_core::Parser;
fn main() {
    let inputs = [
        (b"'|not-an-element\n".as_slice(), "escaped pipe"),
        (b"':not-an-attribute\n".as_slice(), "escaped colon"),
        (b"';not-a-comment\n".as_slice(), "escaped semicolon"),
        (b"''literal-apostrophe\n".as_slice(), "escaped apostrophe"),
    ];
    
    for (input, desc) in inputs {
        println!("=== {} ===", desc);
        println!("Input: {:?}", std::str::from_utf8(input).unwrap());
        Parser::new(input).parse(|e| println!("  {}", e.format_line()));
        println!();
    }
}
