use udon_core::Parser;
fn main() {
    let inputs = [
        (b"; this is a comment\n".as_slice(), "single comment"),
        (b"Hello world\n".as_slice(), "simple text"),
        (b"Some text ; with comment\n".as_slice(), "text with comment"),
        (b"\n\n\n".as_slice(), "blank lines"),
    ];
    
    for (input, desc) in inputs {
        println!("=== {} ===", desc);
        println!("Input: {:?}", std::str::from_utf8(input).unwrap());
        Parser::new(input).parse(|e| println!("  {}", e.format_line()));
        println!();
    }
}
