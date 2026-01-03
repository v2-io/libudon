//! Example: Parse UDON to a tree and navigate it.
//!
//! Run with: cargo run --example tree_parse

use udon_core::tree::{Document, NodeKind};

fn main() {
    let input = br#"|article[intro].featured
  :author Joseph Wecker
  :date 2025-12-22
  :tags [udon notation design]

  |heading Welcome to UDON

  UDON treats documents and data as the same thing - because they are.
  Structure and prose coexist naturally.

  |section
    |p First paragraph with |{em emphasis} and |{strong bold}.
    |p Second paragraph.
"#;

    let doc = Document::parse(input).expect("parse failed");

    println!("=== Document Tree ===\n");
    print_node(&doc.root(), 0);

    println!("\n=== Element Details ===\n");
    for node in doc.root().children() {
        if let Some(el) = node.as_element() {
            println!("Element: {}", el.name());
            if let Some(id) = el.id() {
                println!("  id: {}", id);
            }
            if !el.classes().is_empty() {
                println!("  classes: {:?}", el.classes());
            }
            for (name, value) in el.attrs() {
                println!("  :{} = {:?}", name, value);
            }
            println!();
        }
    }
}

fn print_node(node: &udon_core::tree::Node, depth: usize) {
    let indent = "  ".repeat(depth);

    match node.kind() {
        NodeKind::Document => {
            println!("{}Document", indent);
        }
        NodeKind::Element { name, id, classes, embedded, .. } => {
            let mut desc = format!("{}", name);
            if let Some(id) = id {
                desc.push_str(&format!("[{}]", id));
            }
            for class in classes {
                desc.push_str(&format!(".{}", class));
            }
            if *embedded {
                println!("{}|{{{}}}", indent, desc);
            } else {
                println!("{}|{}", indent, desc);
            }
        }
        NodeKind::Text(s) => {
            let text = s.trim();
            if !text.is_empty() {
                if text.len() > 40 {
                    println!("{}\"{}...\"", indent, &text[..40]);
                } else {
                    println!("{}\"{}\"", indent, text);
                }
            }
        }
        NodeKind::Comment(s) => {
            println!("{}; {}", indent, s.trim());
        }
        NodeKind::Directive { name, .. } => {
            println!("{}!{}", indent, name);
        }
        NodeKind::Interpolation(expr) => {
            println!("{}!{{{{{}}}}}", indent, expr);
        }
        NodeKind::Reference(r) => {
            println!("{}@[{}]", indent, r);
        }
        NodeKind::Raw { lang, content } => {
            if let Some(lang) = lang {
                println!("{}```{}", indent, lang);
            } else {
                println!("{}```", indent);
            }
            println!("{}{}", indent, content.lines().next().unwrap_or(""));
            println!("{}...", indent);
        }
    }

    for child in node.children() {
        print_node(&child, depth + 1);
    }
}
