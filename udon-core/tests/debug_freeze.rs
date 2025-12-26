use udon_core::Parser;

fn test_input(name: &str, input: &[u8]) {
    println!("Testing {}: {:?}", name, input);
    let mut parser = Parser::new(input);
    let events = parser.parse();
    println!("  -> {} events", events.len());
}

#[test]
fn test_various_edge_cases() {
    // Various edge cases that might freeze
    test_input("empty", b"");
    test_input("nul", b"\0");
    test_input("pipe_nul", b"|\0");
    test_input("pipe_bracket", b"|[");
    test_input("pipe_bracket_nul", b"|[\0");
    test_input("colon", b":");
    test_input("colon_nul", b":\0");
    test_input("bracket_open", b"[");
    test_input("brace_open", b"{");
    test_input("quote_single", b"'");
    test_input("quote_double", b"\"");
    test_input("pipe_quote", b"|'");
    test_input("pipe_quote_nul", b"|'\0");
    test_input("newline_colon", b"\n:");
    test_input("newline_colon_nul", b"\n:\0");
}
