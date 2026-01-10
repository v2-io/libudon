use std::io::Read;
use udon_core::Parser;

fn main() {
    let mut input = Vec::new();
    std::io::stdin().read_to_end(&mut input).unwrap();

    Parser::new(&input).parse(|event| {
        eprintln!("EVENT: {:?}", event);
    });
}
