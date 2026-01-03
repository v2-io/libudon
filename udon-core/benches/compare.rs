//! Cross-parser comparison benchmarks.
//!
//! Compares UDON against other document formats using semantically equivalent
//! documents with a realistic mix of structure and prose.
//!
//! Document composition (reflecting real-world usage):
//! - ~50% structural elements (elements, attributes, nesting)
//! - ~30% short text content (labels, values, single lines)
//! - ~20% longer prose blocks
//!
//! Run with: cargo bench --bench compare

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use pulldown_cmark::Parser as MdParser;
use quick_xml::events::Event as XmlEvent;
use quick_xml::Reader as XmlReader;
use udon_core::Parser as UdonParser;

/// Short prose snippets (1-2 sentences)
const SHORT_PROSE: &[&str] = &[
    "A bounded initiative with origin and intent.",
    "The core unit of work tracking.",
    "Enables multi-agent coordination.",
    "Back-planning from desired states.",
    "Novel work doesn't have known steps.",
    "Graph topology emerges from links.",
];

/// Medium prose blocks (2-4 sentences)
const MEDIUM_PROSE: &[&str] = &[
    "Traditional task management is action-oriented: do X, then Y, then Z. \
     Operata is state-oriented: we want state S, what prepares for S?",
    "Every wikilink creates a forward link, a backward link, and a graph edge. \
     This enables automatic relationship discovery without manual maintenance.",
    "Documentation is generated from code, not manually maintained. \
     Module attributes drive frontmatter, keeping everything in sync.",
];

/// Generate semantically equivalent documents with realistic structure/prose mix.
fn generate_documents(item_count: usize) -> Documents {
    let mut udon = String::new();
    let mut markdown = String::new();
    let mut xml = String::from("<?xml version=\"1.0\"?>\n<doc>\n");
    let mut json_items: Vec<serde_json::Value> = Vec::new();
    let mut yaml = String::from("doc:\n  items:\n");
    let mut toml_str = String::from("[doc]\n\n");

    // UDON header
    udon.push_str("|doc :version 1.0 :status active\n");

    // Markdown header
    markdown.push_str("# Document v1.0\n\n");
    markdown.push_str("- **version** 1.0\n");
    markdown.push_str("- **status** active\n\n");

    let mut element_count = 0;
    let categories = ["research", "development", "testing", "documentation"];
    let priorities = ["high", "medium", "low"];
    let statuses = ["active", "pending", "complete"];

    for i in 0..item_count {
        let cat = categories[i % categories.len()];
        let pri = priorities[i % priorities.len()];
        let status = statuses[i % statuses.len()];
        let short = SHORT_PROSE[i % SHORT_PROSE.len()];
        let has_children = i % 3 == 0;
        let has_prose = i % 4 == 0;

        // ========== UDON ==========
        udon.push_str(&format!(
            "  |item[item-{}].{} :priority {} :status {}\n",
            i, cat, pri, status
        ));
        udon.push_str(&format!("    |title Item {} - {}\n", i, short));
        element_count += 1;

        if has_children {
            for j in 0..3 {
                udon.push_str(&format!(
                    "    |sub[sub-{}-{}] :order {}\n",
                    i, j, j + 1
                ));
                udon.push_str(&format!("      |label Task {}\n", j + 1));
                element_count += 1;
            }
        }

        if has_prose {
            let prose = MEDIUM_PROSE[i % MEDIUM_PROSE.len()];
            udon.push_str("    |desc\n");
            udon.push_str(&format!("      {}\n", prose));
        }

        // ========== Markdown ==========
        markdown.push_str(&format!("## Item {} - {}\n\n", i, short));
        markdown.push_str(&format!("- **id** item-{}\n", i));
        markdown.push_str(&format!("- **class** {}\n", cat));
        markdown.push_str(&format!("- **priority** {}\n", pri));
        markdown.push_str(&format!("- **status** {}\n\n", status));

        if has_children {
            for j in 0..3 {
                markdown.push_str(&format!("- Task {} (sub-{}-{}, order: {})\n", j + 1, i, j, j + 1));
            }
            markdown.push('\n');
        }

        if has_prose {
            let prose = MEDIUM_PROSE[i % MEDIUM_PROSE.len()];
            markdown.push_str(prose);
            markdown.push_str("\n\n");
        }

        // ========== XML ==========
        xml.push_str(&format!(
            "  <item id=\"item-{}\" class=\"{}\" priority=\"{}\" status=\"{}\">\n",
            i, cat, pri, status
        ));
        xml.push_str(&format!("    <title>Item {} - {}</title>\n", i, short));

        if has_children {
            for j in 0..3 {
                xml.push_str(&format!(
                    "    <sub id=\"sub-{}-{}\" order=\"{}\">\n",
                    i, j, j + 1
                ));
                xml.push_str(&format!("      <label>Task {}</label>\n", j + 1));
                xml.push_str("    </sub>\n");
            }
        }

        if has_prose {
            let prose = MEDIUM_PROSE[i % MEDIUM_PROSE.len()];
            xml.push_str(&format!("    <desc>{}</desc>\n", prose));
        }

        xml.push_str("  </item>\n");

        // ========== JSON ==========
        let mut item = serde_json::json!({
            "id": format!("item-{}", i),
            "class": cat,
            "priority": pri,
            "status": status,
            "title": format!("Item {} - {}", i, short)
        });

        if has_children {
            let children: Vec<_> = (0..3)
                .map(|j| {
                    serde_json::json!({
                        "id": format!("sub-{}-{}", i, j),
                        "order": j + 1,
                        "label": format!("Task {}", j + 1)
                    })
                })
                .collect();
            item["children"] = serde_json::Value::Array(children);
        }

        if has_prose {
            item["desc"] = serde_json::Value::String(MEDIUM_PROSE[i % MEDIUM_PROSE.len()].to_string());
        }

        json_items.push(item);

        // ========== YAML ==========
        yaml.push_str(&format!("    - id: item-{}\n", i));
        yaml.push_str(&format!("      class: {}\n", cat));
        yaml.push_str(&format!("      priority: {}\n", pri));
        yaml.push_str(&format!("      status: {}\n", status));
        yaml.push_str(&format!("      title: \"Item {} - {}\"\n", i, short));

        if has_children {
            yaml.push_str("      children:\n");
            for j in 0..3 {
                yaml.push_str(&format!("        - id: sub-{}-{}\n", i, j));
                yaml.push_str(&format!("          order: {}\n", j + 1));
                yaml.push_str(&format!("          label: Task {}\n", j + 1));
            }
        }

        if has_prose {
            let prose = MEDIUM_PROSE[i % MEDIUM_PROSE.len()];
            yaml.push_str(&format!("      desc: \"{}\"\n", prose));
        }

        // ========== TOML ==========
        toml_str.push_str("[[doc.items]]\n");
        toml_str.push_str(&format!("id = \"item-{}\"\n", i));
        toml_str.push_str(&format!("class = \"{}\"\n", cat));
        toml_str.push_str(&format!("priority = \"{}\"\n", pri));
        toml_str.push_str(&format!("status = \"{}\"\n", status));
        toml_str.push_str(&format!("title = \"Item {} - {}\"\n", i, short));

        if has_children {
            for j in 0..3 {
                toml_str.push_str(&format!("[[doc.items.children]]\n"));
                toml_str.push_str(&format!("id = \"sub-{}-{}\"\n", i, j));
                toml_str.push_str(&format!("order = {}\n", j + 1));
                toml_str.push_str(&format!("label = \"Task {}\"\n", j + 1));
            }
        }

        if has_prose {
            let prose = MEDIUM_PROSE[i % MEDIUM_PROSE.len()];
            toml_str.push_str(&format!("desc = \"{}\"\n", prose));
        }
        toml_str.push('\n');
    }

    xml.push_str("</doc>\n");

    let json = serde_json::json!({
        "doc": {
            "version": "1.0",
            "status": "active",
            "items": json_items
        }
    });
    let json_str = serde_json::to_string(&json).unwrap();

    Documents {
        udon: udon.into_bytes(),
        markdown,
        xml,
        json: json_str,
        yaml,
        toml: toml_str,
        element_count,
    }
}

struct Documents {
    udon: Vec<u8>,
    markdown: String,
    xml: String,
    json: String,
    yaml: String,
    toml: String,
    element_count: usize,
}

// ============ Parsers ============

fn parse_udon(input: &[u8]) -> usize {
    let mut elements = 0;
    UdonParser::new(input).parse(|event| {
        if matches!(&event, udon_core::Event::ElementStart { .. }) {
            elements += 1;
        }
        black_box(&event);
    });
    elements
}

fn parse_markdown(input: &str) -> usize {
    use pulldown_cmark::Event;
    let parser = MdParser::new(input);
    let mut elements = 0;
    for event in parser {
        if matches!(&event, Event::Start(_)) {
            elements += 1;
        }
        black_box(&event);
    }
    elements
}

fn parse_xml(input: &str) -> usize {
    let mut reader = XmlReader::from_str(input);
    reader.config_mut().trim_text(true);
    let mut elements = 0;
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(XmlEvent::Eof) => break,
            Ok(XmlEvent::Start(_)) | Ok(XmlEvent::Empty(_)) => elements += 1,
            Ok(ref event) => { black_box(event); }
            Err(e) => panic!("XML parse error: {:?}", e),
        }
        buf.clear();
    }
    elements
}

fn parse_json(input: &str) -> usize {
    let v: serde_json::Value = serde_json::from_str(input).unwrap();
    fn count_values(v: &serde_json::Value) -> usize {
        match v {
            serde_json::Value::Object(map) => 1 + map.values().map(count_values).sum::<usize>(),
            serde_json::Value::Array(arr) => 1 + arr.iter().map(count_values).sum::<usize>(),
            _ => 1,
        }
    }
    let n = count_values(&v);
    black_box(&v);
    n
}

fn parse_yaml(input: &str) -> usize {
    let v: serde_yaml::Value = serde_yaml::from_str(input).unwrap();
    fn count_values(v: &serde_yaml::Value) -> usize {
        match v {
            serde_yaml::Value::Mapping(map) => 1 + map.values().map(count_values).sum::<usize>(),
            serde_yaml::Value::Sequence(arr) => 1 + arr.iter().map(count_values).sum::<usize>(),
            _ => 1,
        }
    }
    let n = count_values(&v);
    black_box(&v);
    n
}

fn parse_toml(input: &str) -> usize {
    let v: toml::Value = input.parse().unwrap();
    fn count_values(v: &toml::Value) -> usize {
        match v {
            toml::Value::Table(map) => 1 + map.values().map(count_values).sum::<usize>(),
            toml::Value::Array(arr) => 1 + arr.iter().map(count_values).sum::<usize>(),
            _ => 1,
        }
    }
    let n = count_values(&v);
    black_box(&v);
    n
}

fn bench_parser_comparison(c: &mut Criterion) {
    let sizes = [10, 50, 200];

    for item_count in sizes {
        let docs = generate_documents(item_count);

        println!(
            "\n{} items ({} elements):",
            item_count, docs.element_count
        );
        println!(
            "  UDON={:.1}KB XML={:.1}KB JSON={:.1}KB YAML={:.1}KB TOML={:.1}KB MD={:.1}KB",
            docs.udon.len() as f64 / 1024.0,
            docs.xml.len() as f64 / 1024.0,
            docs.json.len() as f64 / 1024.0,
            docs.yaml.len() as f64 / 1024.0,
            docs.toml.len() as f64 / 1024.0,
            docs.markdown.len() as f64 / 1024.0,
        );

        // Bytes throughput group
        let mut group = c.benchmark_group(format!("compare_{}items", item_count));

        // UDON
        group.throughput(Throughput::Bytes(docs.udon.len() as u64));
        group.bench_with_input(BenchmarkId::new("udon", ""), &docs.udon, |b, doc| {
            b.iter(|| parse_udon(black_box(doc)))
        });

        // XML
        group.throughput(Throughput::Bytes(docs.xml.len() as u64));
        group.bench_with_input(BenchmarkId::new("quick-xml", ""), &docs.xml, |b, doc| {
            b.iter(|| parse_xml(black_box(doc)))
        });

        // JSON
        group.throughput(Throughput::Bytes(docs.json.len() as u64));
        group.bench_with_input(BenchmarkId::new("serde_json", ""), &docs.json, |b, doc| {
            b.iter(|| parse_json(black_box(doc)))
        });

        // YAML
        group.throughput(Throughput::Bytes(docs.yaml.len() as u64));
        group.bench_with_input(BenchmarkId::new("serde_yaml", ""), &docs.yaml, |b, doc| {
            b.iter(|| parse_yaml(black_box(doc)))
        });

        // TOML
        group.throughput(Throughput::Bytes(docs.toml.len() as u64));
        group.bench_with_input(BenchmarkId::new("toml", ""), &docs.toml, |b, doc| {
            b.iter(|| parse_toml(black_box(doc)))
        });

        // Markdown
        group.throughput(Throughput::Bytes(docs.markdown.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("pulldown-cmark", ""),
            &docs.markdown,
            |b, doc| b.iter(|| parse_markdown(black_box(doc))),
        );

        group.finish();
    }
}

criterion_group!(benches, bench_parser_comparison);
criterion_main!(benches);
