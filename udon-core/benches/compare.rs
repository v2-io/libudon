//! Cross-parser comparison benchmarks.
//!
//! Compares UDON against:
//! - pulldown-cmark (Markdown, streaming/pull parser)
//! - quick-xml (XML, streaming SAX parser)
//!
//! All parsers are streaming/event-based for fair comparison.
//! Benchmarks measure parse + full event consumption.
//!
//! Run with: cargo bench --bench compare
//!
//! Key insight: Different formats emit different numbers of events per byte.
//! UDON emits fine-grained events (Name, Attr, Value separately), while
//! quick-xml batches attributes. So we measure both throughput AND events/sec.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use pulldown_cmark::Parser as MdParser;
use quick_xml::events::Event as XmlEvent;
use quick_xml::Reader as XmlReader;
use udon_core::Parser as UdonParser;

/// Generate flat documents (not deeply nested) for comparable benchmarks.
/// Each format gets ~N elements with similar content.
fn generate_flat_documents(count: usize) -> (Vec<u8>, String, String) {
    let mut udon = String::new();
    let mut markdown = String::new();
    let mut xml = String::from("<?xml version=\"1.0\"?>\n<root>\n");

    for i in 0..count {
        // UDON: element with id and text
        udon.push_str(&format!("|item[id-{}]\n", i));
        udon.push_str(&format!("  This is the content for item number {}.\n", i));

        // Markdown: header + paragraph
        markdown.push_str(&format!("## Item {}\n\n", i));
        markdown.push_str(&format!("This is the content for item number {}.\n\n", i));

        // XML: element with attribute and text
        xml.push_str(&format!("  <item id=\"id-{}\">\n", i));
        xml.push_str(&format!("    This is the content for item number {}.\n", i));
        xml.push_str("  </item>\n");
    }

    xml.push_str("</root>\n");

    (udon.into_bytes(), markdown, xml)
}

/// Parse UDON and count elements (ElementStart events).
fn parse_udon(input: &[u8]) -> usize {
    let mut elements = 0;
    UdonParser::new(input).parse(|event| {
        black_box(&event);
        if matches!(event, udon_core::Event::ElementStart { .. }) {
            elements += 1;
        }
    });
    elements
}

/// Parse Markdown and count block elements (Start events only).
fn parse_markdown(input: &str) -> usize {
    use pulldown_cmark::Event;
    let parser = MdParser::new(input);
    let mut elements = 0;
    for event in parser {
        black_box(&event);
        // Count start of block/inline elements only
        if matches!(event, Event::Start(_)) {
            elements += 1;
        }
    }
    elements
}

/// Parse XML and count element starts.
fn parse_xml(input: &str) -> usize {
    let mut reader = XmlReader::from_str(input);
    reader.config_mut().trim_text(true);
    let mut elements = 0;
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(XmlEvent::Eof) => break,
            Ok(ref event) => {
                black_box(event);
                if matches!(event, XmlEvent::Start(_) | XmlEvent::Empty(_)) {
                    elements += 1;
                }
            }
            Err(e) => panic!("XML parse error: {:?}", e),
        }
        buf.clear();
    }
    elements
}

/// Benchmark comparison across parsers with flat documents.
/// Measures elements/second for semantic fairness.
fn bench_parser_comparison(c: &mut Criterion) {
    let sizes = [50, 200, 500];

    for count in sizes {
        let (udon_doc, md_doc, xml_doc) = generate_flat_documents(count);

        // Verify element counts
        let udon_elem = parse_udon(&udon_doc);
        let md_elem = parse_markdown(&md_doc);
        let xml_elem = parse_xml(&xml_doc);

        // Print document info
        println!(
            "\n{}elem: UDON={}B/{}elem  MD={}B/{}elem  XML={}B/{}elem",
            count,
            udon_doc.len(),
            udon_elem,
            md_doc.len(),
            md_elem,
            xml_doc.len(),
            xml_elem
        );

        let mut group = c.benchmark_group(format!("compare_{}elem", count));

        // All use same element count for fair comparison
        group.throughput(Throughput::Elements(count as u64));

        group.bench_with_input(BenchmarkId::new("udon", ""), &udon_doc, |b, doc| {
            b.iter(|| parse_udon(black_box(doc)))
        });

        group.bench_with_input(BenchmarkId::new("pulldown-cmark", ""), &md_doc, |b, doc| {
            b.iter(|| parse_markdown(black_box(doc)))
        });

        group.bench_with_input(BenchmarkId::new("quick-xml", ""), &xml_doc, |b, doc| {
            b.iter(|| parse_xml(black_box(doc)))
        });

        group.finish();
    }
}

criterion_group!(benches, bench_parser_comparison);
criterion_main!(benches);
