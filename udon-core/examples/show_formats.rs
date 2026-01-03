// Show first 30 lines of each format for comparison
// Run with: cargo run --example show_formats

fn indent(s: &str, prefix: &str) -> String {
    s.lines()
        .map(|line| {
            if line.is_empty() {
                String::new()
            } else {
                format!("{}{}", prefix, line)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn main() {
    let content = r#"## Core Insight

Traditional task management is action-oriented.

This inversion matters because:
- Novel work doesn't have known steps
- Back-planning from desired states

```rust
fn example() {
    println!("hello");
}
```"#;

    let section_title = "Section 1: Topic Area";
    let comment = "TODO: Review this section";
    let priority = "high";
    let category = "research";

    // UDON
    let mut udon = String::new();
    udon.push_str("; Document with embedded Markdown\n");
    udon.push_str("|document :version 1.0 :lang en :status draft\n");
    udon.push_str(&format!("  ; {}\n", comment));
    udon.push_str(&format!(
        "  |section[sec-0] :title \"{}\" :priority {} :category {}\n",
        section_title, priority, category
    ));
    udon.push_str("    |content\n");
    udon.push_str(&indent(content, "      "));
    udon.push_str("\n");

    // XML
    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<document version=\"1.0\" lang=\"en\" status=\"draft\">\n");
    xml.push_str(&format!("  <!-- {} -->\n", comment));
    xml.push_str(&format!(
        "  <section id=\"sec-0\" title=\"{}\" priority=\"{}\" category=\"{}\">\n",
        section_title, priority, category
    ));
    xml.push_str("    <content><![CDATA[\n");
    xml.push_str(content);
    xml.push_str("\n    ]]></content>\n");
    xml.push_str("  </section>\n");
    xml.push_str("</document>\n");

    // Markdown
    let mut md = String::new();
    md.push_str("# Document v1.0\n\n");
    md.push_str("- **version** 1.0\n");
    md.push_str("- **lang** en\n");
    md.push_str("- **status** draft\n\n");
    md.push_str("---\n\n");
    md.push_str(&format!("## {}\n\n", section_title));
    md.push_str(&format!("<!-- {} -->\n\n", comment));
    md.push_str("- **id** sec-0\n");
    md.push_str(&format!("- **priority** {}\n", priority));
    md.push_str(&format!("- **category** {}\n\n", category));
    md.push_str(content);
    md.push_str("\n");

    // JSON
    let json = serde_json::json!({
        "document": {
            "version": "1.0",
            "lang": "en",
            "status": "draft",
            "sections": [{
                "id": "sec-0",
                "title": section_title,
                "priority": priority,
                "category": category,
                "comment": comment,
                "content": content
            }]
        }
    });
    let json_str = serde_json::to_string_pretty(&json).unwrap();

    // YAML
    let mut yaml = String::new();
    yaml.push_str("document:\n");
    yaml.push_str("  version: \"1.0\"\n");
    yaml.push_str("  lang: en\n");
    yaml.push_str("  status: draft\n");
    yaml.push_str("  sections:\n");
    yaml.push_str(&format!("    # {}\n", comment));
    yaml.push_str("    - id: sec-0\n");
    yaml.push_str(&format!("      title: \"{}\"\n", section_title));
    yaml.push_str(&format!("      priority: {}\n", priority));
    yaml.push_str(&format!("      category: {}\n", category));
    yaml.push_str("      content: |\n");
    yaml.push_str(&indent(content, "        "));
    yaml.push_str("\n");

    // TOML
    let mut toml = String::new();
    toml.push_str("[document]\n");
    toml.push_str("version = \"1.0\"\n");
    toml.push_str("lang = \"en\"\n");
    toml.push_str("status = \"draft\"\n\n");
    toml.push_str(&format!("# {}\n", comment));
    toml.push_str("[[document.sections]]\n");
    toml.push_str("id = \"sec-0\"\n");
    toml.push_str(&format!("title = \"{}\"\n", section_title));
    toml.push_str(&format!("priority = \"{}\"\n", priority));
    toml.push_str(&format!("category = \"{}\"\n", category));
    toml.push_str("content = \"\"\"\n");
    toml.push_str(content);
    toml.push_str("\"\"\"\n");

    println!("===== UDON ({} bytes) =====", udon.len());
    for (i, line) in udon.lines().take(30).enumerate() {
        println!("{:2}: {}", i + 1, line);
    }

    println!("\n===== XML ({} bytes) =====", xml.len());
    for (i, line) in xml.lines().take(30).enumerate() {
        println!("{:2}: {}", i + 1, line);
    }

    println!("\n===== MARKDOWN ({} bytes) =====", md.len());
    for (i, line) in md.lines().take(30).enumerate() {
        println!("{:2}: {}", i + 1, line);
    }

    println!("\n===== JSON ({} bytes) =====", json_str.len());
    for (i, line) in json_str.lines().take(30).enumerate() {
        println!("{:2}: {}", i + 1, line);
    }

    println!("\n===== YAML ({} bytes) =====", yaml.len());
    for (i, line) in yaml.lines().take(30).enumerate() {
        println!("{:2}: {}", i + 1, line);
    }

    println!("\n===== TOML ({} bytes) =====", toml.len());
    for (i, line) in toml.lines().take(30).enumerate() {
        println!("{:2}: {}", i + 1, line);
    }
}
