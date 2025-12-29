use udon_core::StreamingParser;

fn main() {
    let input = include_bytes!("/Users/josephwecker-v2/src/udon/examples/mathml-to-latex.udon");
    
    // Run many iterations for profiling
    for _ in 0..5000 {
        let mut parser = StreamingParser::new(2048);
        parser.feed(input);
        parser.finish();
        while parser.read().is_some() {}
    }
}
