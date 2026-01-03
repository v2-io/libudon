//! Cross-parser comparison benchmarks.
//!
//! Compares UDON against:
//! - pulldown-cmark (Markdown, streaming/pull parser)
//! - quick-xml (XML, streaming SAX parser)
//! - serde_json (JSON)
//! - serde_yaml (YAML)
//! - toml (TOML)
//!
//! All documents are semantically equivalent with rich structure:
//! - Deep nesting (sections, subsections, items)
//! - Many attributes
//! - Comments (where supported)
//! - Varied prose content
//!
//! Run with: cargo bench --bench compare

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use pulldown_cmark::Parser as MdParser;
use quick_xml::events::Event as XmlEvent;
use quick_xml::Reader as XmlReader;
use udon_core::Parser as UdonParser;

/// Prose paragraphs - varied, realistic content
const PROSE: &[&str] = &[
    "The quick brown fox jumps over the lazy dog. This pangram contains every letter of the alphabet, making it useful for typography and testing.",
    "In the beginning, there was nothing but potential. Then came the first spark of creation, igniting a cascade of events that would shape the universe.",
    "She walked through the ancient forest, leaves crunching beneath her feet. The trees whispered secrets in a language only the wind understood.",
    "Technology advances at an exponential rate, transforming how we live, work, and connect. What once seemed like science fiction is now everyday reality.",
    "The recipe called for three cups of flour, two eggs, and a pinch of salt. Grandma always said the secret ingredient was love.",
    "Mountains rose in the distance, their peaks shrouded in mist. The hikers paused to catch their breath and admire the breathtaking view.",
    "Code is poetry written for machines to execute and humans to understand. The best programs are both elegant and efficient.",
    "The jazz quartet played late into the night, their improvisations weaving through smoke and shadow. Each note told a story.",
    "Scientists discovered a new species of deep-sea creature, bioluminescent and otherworldly. The ocean still holds countless mysteries.",
    "Children laughed and played in the park while parents watched from weathered benches. Summer afternoons stretched endlessly.",
    "The old bookshop smelled of dust and adventure. Every spine on every shelf promised a journey to another world.",
    "Rain drummed against the window as she typed the final chapter. After three years, the novel was finally complete.",
];

/// Comments for realism
const COMMENTS: &[&str] = &[
    "TODO: Review this section",
    "Note: Updated 2024-01-15",
    "FIXME: Needs fact-checking",
    "Author's note: This is a draft",
    "Editor: Please verify citations",
    "Reminder: Add more examples here",
];

/// Generate semantically equivalent documents with rich structure.
/// Structure: Document > Sections > Subsections > Items (with attributes, comments, prose)
fn generate_documents(section_count: usize) -> Documents {
    let mut udon = String::new();
    let mut markdown = String::new();
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<!-- Document root with metadata -->\n");
    xml.push_str("<document version=\"1.0\" lang=\"en\" status=\"draft\">\n");
    xml.push_str("  <metadata>\n");
    xml.push_str("    <created>2024-01-15</created>\n");
    xml.push_str("    <modified>2024-06-20</modified>\n");
    xml.push_str("    <author>Test Author</author>\n");
    xml.push_str("  </metadata>\n");

    let mut json_sections: Vec<serde_json::Value> = Vec::new();

    let mut yaml = String::from("# Document with rich structure\n");
    yaml.push_str("document:\n");
    yaml.push_str("  version: \"1.0\"\n");
    yaml.push_str("  lang: en\n");
    yaml.push_str("  status: draft\n");
    yaml.push_str("  metadata:\n");
    yaml.push_str("    created: 2024-01-15\n");
    yaml.push_str("    modified: 2024-06-20\n");
    yaml.push_str("    author: Test Author\n");
    yaml.push_str("  sections:\n");

    let mut toml_str = String::from("# Document configuration\n");
    toml_str.push_str("[document]\n");
    toml_str.push_str("version = \"1.0\"\n");
    toml_str.push_str("lang = \"en\"\n");
    toml_str.push_str("status = \"draft\"\n\n");
    toml_str.push_str("[document.metadata]\n");
    toml_str.push_str("created = 2024-01-15\n");
    toml_str.push_str("modified = 2024-06-20\n");
    toml_str.push_str("author = \"Test Author\"\n\n");

    // UDON header (compact attributes)
    udon.push_str("; Document with rich structure\n");
    udon.push_str("|document[version-1.0] :lang en :status draft\n");
    udon.push_str("  ; Metadata section\n");
    udon.push_str("  |metadata :created 2024-01-15 :modified 2024-06-20 :author Test Author\n");

    // Markdown header
    markdown.push_str("# Document v1.0\n\n");
    markdown.push_str("<!-- Metadata: lang=en, status=draft -->\n\n");
    markdown.push_str("**Created:** 2024-01-15 | **Modified:** 2024-06-20 | **Author:** Test Author\n\n");
    markdown.push_str("---\n\n");

    let subsections_per_section = 3;
    let items_per_subsection = 2;
    let mut element_count = 0;

    for s in 0..section_count {
        let section_title = format!("Section {}: Main Topic Area", s + 1);
        let comment = COMMENTS[s % COMMENTS.len()];
        let priority = ["high", "medium", "low"][s % 3];
        let category = ["research", "development", "testing", "documentation"][s % 4];

        // UDON section (compact single-line attributes like XML)
        udon.push_str(&format!("  ; {}\n", comment));
        udon.push_str(&format!("  |section[sec-{}] :title {} :priority {} :category {} :index {}\n",
            s, section_title, priority, category, s));
        element_count += 1;

        // Markdown section
        markdown.push_str(&format!("## {}\n\n", section_title));
        markdown.push_str(&format!("<!-- {} -->\n\n", comment));
        markdown.push_str(&format!("*Priority: {} | Category: {}*\n\n", priority, category));

        // XML section
        xml.push_str(&format!("  <!-- {} -->\n", comment));
        xml.push_str(&format!("  <section id=\"sec-{}\" priority=\"{}\" category=\"{}\" index=\"{}\">\n",
            s, priority, category, s));
        xml.push_str(&format!("    <title>{}</title>\n", section_title));

        // JSON section (build structure)
        let mut json_subsections: Vec<serde_json::Value> = Vec::new();

        // YAML section
        yaml.push_str(&format!("    # {}\n", comment));
        yaml.push_str(&format!("    - id: sec-{}\n", s));
        yaml.push_str(&format!("      title: \"{}\"\n", section_title));
        yaml.push_str(&format!("      priority: {}\n", priority));
        yaml.push_str(&format!("      category: {}\n", category));
        yaml.push_str(&format!("      index: {}\n", s));
        yaml.push_str("      subsections:\n");

        // TOML section
        toml_str.push_str(&format!("# {}\n", comment));
        toml_str.push_str(&format!("[[document.sections]]\n"));
        toml_str.push_str(&format!("id = \"sec-{}\"\n", s));
        toml_str.push_str(&format!("title = \"{}\"\n", section_title));
        toml_str.push_str(&format!("priority = \"{}\"\n", priority));
        toml_str.push_str(&format!("category = \"{}\"\n", category));
        toml_str.push_str(&format!("index = {}\n\n", s));

        for ss in 0..subsections_per_section {
            let subsection_title = format!("Subsection {}.{}: Detailed Analysis", s + 1, ss + 1);
            let status = ["complete", "in-progress", "pending"][ss % 3];
            let reviewer = ["Alice", "Bob", "Carol"][ss % 3];

            // UDON subsection (compact)
            udon.push_str(&format!("    |subsection[subsec-{}-{}] :title {} :status {} :reviewer {} :parent sec-{}\n",
                s, ss, subsection_title, status, reviewer, s));
            element_count += 1;

            // Markdown subsection
            markdown.push_str(&format!("### {}\n\n", subsection_title));
            markdown.push_str(&format!("*Status: {} | Reviewer: {}*\n\n", status, reviewer));

            // XML subsection
            xml.push_str(&format!("    <subsection id=\"subsec-{}-{}\" status=\"{}\" reviewer=\"{}\" parent=\"sec-{}\">\n",
                s, ss, status, reviewer, s));
            xml.push_str(&format!("      <title>{}</title>\n", subsection_title));

            let mut json_items: Vec<serde_json::Value> = Vec::new();

            // YAML subsection
            yaml.push_str(&format!("        - id: subsec-{}-{}\n", s, ss));
            yaml.push_str(&format!("          title: \"{}\"\n", subsection_title));
            yaml.push_str(&format!("          status: {}\n", status));
            yaml.push_str(&format!("          reviewer: {}\n", reviewer));
            yaml.push_str(&format!("          parent: sec-{}\n", s));
            yaml.push_str("          items:\n");

            for i in 0..items_per_subsection {
                let prose_idx = (s * 7 + ss * 3 + i) % PROSE.len();
                let prose = PROSE[prose_idx];
                let prose2_idx = (prose_idx + 4) % PROSE.len();
                let prose2 = PROSE[prose2_idx];
                let item_type = ["note", "example", "definition", "reference"][i % 4];
                let weight = ((s + ss + i) % 10) + 1;
                let visible = i % 2 == 0;
                let tags: Vec<&str> = vec![
                    ["alpha", "beta", "gamma"][i % 3],
                    ["primary", "secondary"][ss % 2],
                ];

                // UDON item (compact single-line attributes)
                udon.push_str(&format!("      ; Item {} of subsection\n", i + 1));
                udon.push_str(&format!("      |item[item-{}-{}-{}].{} :type {} :weight {} :visible {} :tags {} {} :section sec-{} :subsection subsec-{}-{}\n",
                    s, ss, i, item_type, item_type, weight, visible, tags[0], tags[1], s, s, ss));
                udon.push_str("        |content\n");
                udon.push_str(&format!("          {}\n", prose));
                udon.push_str(&format!("          {}\n", prose2));
                element_count += 1;

                // Markdown item
                markdown.push_str(&format!("#### Item: {} ({})\n\n", item_type, if visible { "visible" } else { "hidden" }));
                markdown.push_str(&format!("> Weight: {} | Tags: {}, {}\n\n", weight, tags[0], tags[1]));
                markdown.push_str(&format!("{}\n\n", prose));
                markdown.push_str(&format!("{}\n\n", prose2));

                // XML item
                xml.push_str(&format!("      <item id=\"item-{}-{}-{}\" class=\"{}\" type=\"{}\" weight=\"{}\" visible=\"{}\">\n",
                    s, ss, i, item_type, item_type, weight, visible));
                xml.push_str(&format!("        <tags><tag>{}</tag><tag>{}</tag></tags>\n", tags[0], tags[1]));
                xml.push_str(&format!("        <refs section=\"sec-{}\" subsection=\"subsec-{}-{}\"/>\n", s, s, ss));
                xml.push_str("        <content>\n");
                xml.push_str(&format!("          <p>{}</p>\n", prose));
                xml.push_str(&format!("          <p>{}</p>\n", prose2));
                xml.push_str("        </content>\n");
                xml.push_str("      </item>\n");

                // JSON item
                json_items.push(serde_json::json!({
                    "id": format!("item-{}-{}-{}", s, ss, i),
                    "type": item_type,
                    "weight": weight,
                    "visible": visible,
                    "tags": tags,
                    "refs": {
                        "section": format!("sec-{}", s),
                        "subsection": format!("subsec-{}-{}", s, ss)
                    },
                    "content": [prose, prose2]
                }));

                // YAML item
                yaml.push_str(&format!("            # Item {} of subsection\n", i + 1));
                yaml.push_str(&format!("            - id: item-{}-{}-{}\n", s, ss, i));
                yaml.push_str(&format!("              type: {}\n", item_type));
                yaml.push_str(&format!("              weight: {}\n", weight));
                yaml.push_str(&format!("              visible: {}\n", visible));
                yaml.push_str(&format!("              tags: [{}, {}]\n", tags[0], tags[1]));
                yaml.push_str(&format!("              section: sec-{}\n", s));
                yaml.push_str(&format!("              subsection: subsec-{}-{}\n", s, ss));
                yaml.push_str("              content:\n");
                yaml.push_str(&format!("                - \"{}\"\n", prose));
                yaml.push_str(&format!("                - \"{}\"\n", prose2));
            }

            json_subsections.push(serde_json::json!({
                "id": format!("subsec-{}-{}", s, ss),
                "title": subsection_title,
                "status": status,
                "reviewer": reviewer,
                "parent": format!("sec-{}", s),
                "items": json_items
            }));

            // XML close subsection
            xml.push_str("    </subsection>\n");
        }

        json_sections.push(serde_json::json!({
            "id": format!("sec-{}", s),
            "title": section_title,
            "priority": priority,
            "category": category,
            "index": s,
            "subsections": json_subsections
        }));

        // XML close section
        xml.push_str("  </section>\n");

        // TOML subsections (flattened due to TOML limitations)
        for ss in 0..subsections_per_section {
            let subsection_title = format!("Subsection {}.{}: Detailed Analysis", s + 1, ss + 1);
            let status = ["complete", "in-progress", "pending"][ss % 3];
            let reviewer = ["Alice", "Bob", "Carol"][ss % 3];

            toml_str.push_str(&format!("[[document.sections.subsections]]\n"));
            toml_str.push_str(&format!("id = \"subsec-{}-{}\"\n", s, ss));
            toml_str.push_str(&format!("title = \"{}\"\n", subsection_title));
            toml_str.push_str(&format!("status = \"{}\"\n", status));
            toml_str.push_str(&format!("reviewer = \"{}\"\n", reviewer));
            toml_str.push_str(&format!("parent = \"sec-{}\"\n\n", s));

            for i in 0..items_per_subsection {
                let prose_idx = (s * 7 + ss * 3 + i) % PROSE.len();
                let prose = PROSE[prose_idx];
                let prose2_idx = (prose_idx + 4) % PROSE.len();
                let prose2 = PROSE[prose2_idx];
                let item_type = ["note", "example", "definition", "reference"][i % 4];
                let weight = ((s + ss + i) % 10) + 1;
                let visible = i % 2 == 0;
                let tags: Vec<&str> = vec![
                    ["alpha", "beta", "gamma"][i % 3],
                    ["primary", "secondary"][ss % 2],
                ];

                toml_str.push_str(&format!("[[document.sections.subsections.items]]\n"));
                toml_str.push_str(&format!("id = \"item-{}-{}-{}\"\n", s, ss, i));
                toml_str.push_str(&format!("type = \"{}\"\n", item_type));
                toml_str.push_str(&format!("weight = {}\n", weight));
                toml_str.push_str(&format!("visible = {}\n", visible));
                toml_str.push_str(&format!("tags = [\"{}\", \"{}\"]\n", tags[0], tags[1]));
                toml_str.push_str(&format!("section = \"sec-{}\"\n", s));
                toml_str.push_str(&format!("subsection = \"subsec-{}-{}\"\n", s, ss));
                toml_str.push_str(&format!("content = [\"{}\", \"{}\"]\n\n", prose, prose2));
            }
        }
    }

    xml.push_str("</document>\n");

    let json = serde_json::json!({
        "document": {
            "version": "1.0",
            "lang": "en",
            "status": "draft",
            "metadata": {
                "created": "2024-01-15",
                "modified": "2024-06-20",
                "author": "Test Author"
            },
            "sections": json_sections
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

/// Parse UDON and count all elements.
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

/// Parse Markdown and count structural elements.
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

/// Parse XML and count all elements.
fn parse_xml(input: &str) -> usize {
    let mut reader = XmlReader::from_str(input);
    reader.config_mut().trim_text(true);
    let mut elements = 0;
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(XmlEvent::Eof) => break,
            Ok(XmlEvent::Start(_)) | Ok(XmlEvent::Empty(_)) => {
                elements += 1;
            }
            Ok(ref event) => { black_box(event); }
            Err(e) => panic!("XML parse error: {:?}", e),
        }
        buf.clear();
    }
    elements
}

/// Parse JSON fully.
fn parse_json(input: &str) -> usize {
    let v: serde_json::Value = serde_json::from_str(input).unwrap();
    // Count by traversing
    fn count_values(v: &serde_json::Value) -> usize {
        match v {
            serde_json::Value::Object(map) => {
                1 + map.values().map(count_values).sum::<usize>()
            }
            serde_json::Value::Array(arr) => {
                1 + arr.iter().map(count_values).sum::<usize>()
            }
            _ => 1,
        }
    }
    let n = count_values(&v);
    black_box(&v);
    n
}

/// Parse YAML fully.
fn parse_yaml(input: &str) -> usize {
    let v: serde_yaml::Value = serde_yaml::from_str(input).unwrap();
    fn count_values(v: &serde_yaml::Value) -> usize {
        match v {
            serde_yaml::Value::Mapping(map) => {
                1 + map.values().map(count_values).sum::<usize>()
            }
            serde_yaml::Value::Sequence(arr) => {
                1 + arr.iter().map(count_values).sum::<usize>()
            }
            _ => 1,
        }
    }
    let n = count_values(&v);
    black_box(&v);
    n
}

/// Parse TOML fully.
fn parse_toml(input: &str) -> usize {
    let v: toml::Value = input.parse().unwrap();
    fn count_values(v: &toml::Value) -> usize {
        match v {
            toml::Value::Table(map) => {
                1 + map.values().map(count_values).sum::<usize>()
            }
            toml::Value::Array(arr) => {
                1 + arr.iter().map(count_values).sum::<usize>()
            }
            _ => 1,
        }
    }
    let n = count_values(&v);
    black_box(&v);
    n
}

/// Benchmark comparison across all parsers.
fn bench_parser_comparison(c: &mut Criterion) {
    // section_count determines overall document size
    // Each section has 3 subsections, each with 2 items
    let sizes = [5, 10, 20];

    for section_count in sizes {
        let docs = generate_documents(section_count);

        // Get parse counts for info
        let udon_n = parse_udon(&docs.udon);
        let md_n = parse_markdown(&docs.markdown);
        let xml_n = parse_xml(&docs.xml);
        let json_n = parse_json(&docs.json);
        let yaml_n = parse_yaml(&docs.yaml);
        let toml_n = parse_toml(&docs.toml);

        println!(
            "\n{} sections ({} elements): UDON={}B MD={}B XML={}B JSON={}B YAML={}B TOML={}B",
            section_count,
            docs.element_count,
            docs.udon.len(),
            docs.markdown.len(),
            docs.xml.len(),
            docs.json.len(),
            docs.yaml.len(),
            docs.toml.len(),
        );
        println!(
            "  Parse counts: UDON={} MD={} XML={} JSON={} YAML={} TOML={}",
            udon_n, md_n, xml_n, json_n, yaml_n, toml_n
        );

        let mut group = c.benchmark_group(format!("compare_{}sections", section_count));

        // Use UDON element count as the standard (most directly comparable)
        group.throughput(Throughput::Elements(docs.element_count as u64));

        group.bench_with_input(BenchmarkId::new("udon", ""), &docs.udon, |b, doc| {
            b.iter(|| parse_udon(black_box(doc)))
        });

        group.bench_with_input(BenchmarkId::new("quick-xml", ""), &docs.xml, |b, doc| {
            b.iter(|| parse_xml(black_box(doc)))
        });

        group.bench_with_input(BenchmarkId::new("serde_json", ""), &docs.json, |b, doc| {
            b.iter(|| parse_json(black_box(doc)))
        });

        group.bench_with_input(BenchmarkId::new("serde_yaml", ""), &docs.yaml, |b, doc| {
            b.iter(|| parse_yaml(black_box(doc)))
        });

        group.bench_with_input(BenchmarkId::new("toml", ""), &docs.toml, |b, doc| {
            b.iter(|| parse_toml(black_box(doc)))
        });

        group.bench_with_input(BenchmarkId::new("pulldown-cmark", ""), &docs.markdown, |b, doc| {
            b.iter(|| parse_markdown(black_box(doc)))
        });

        group.finish();
    }
}

criterion_group!(benches, bench_parser_comparison);
criterion_main!(benches);
