//! XML parser and writer acceptance tests beyond the unit suite.

use cadence_import::xml::{self, Element, MAX_ATTRIBUTES, MAX_DEPTH, MAX_NAME_BYTES};

#[test]
fn attributes_entities_and_text_are_preserved() {
    let xml = r#"<score a="1" b='two &amp; three' c="&#233;">
        hello &lt;world&gt; &#x1F600;
    </score>"#;
    let document = xml::parse(xml).unwrap();
    assert_eq!(document.root.attr("a"), Some("1"));
    assert_eq!(document.root.attr("b"), Some("two & three"));
    assert_eq!(document.root.attr("c"), Some("é"));
    assert!(document.root.text_trimmed().contains("hello <world>"));
}

#[test]
fn cdata_comments_and_processing_instructions_are_transparent() {
    let xml = r#"<?xml version="1.0"?>
    <!-- leading -->
    <a><?target data?>A<![CDATA[<raw & text>]]>B<!-- inner -->C</a>"#;
    let document = xml::parse(xml).unwrap();
    assert_eq!(document.root.text_trimmed(), "A<raw & text>BC");
}

#[test]
fn malformed_documents_return_errors_instead_of_panicking() {
    let cases = [
        "",
        "<",
        "<a",
        "<a>",
        "<a></b>",
        "<a/><b/>",
        "<a x></a>",
        "<a x=1></a>",
        "<a x=\"1\" x=\"2\"></a>",
        "<a>&unknown;</a>",
        "<a>&#x110000;</a>",
        "<a>&#xD800;</a>",
        "<!-- unterminated",
        "<![CDATA[unterminated</a>",
        "<!DOCTYPE a><a></a>",
        "<a><b></a></b>",
        "text before <a></a>",
        "<a></a>text after",
    ];
    for case in cases {
        assert!(xml::parse(case).is_err(), "accepted {case:?}");
    }
}

#[test]
fn limits_are_enforced() {
    let deep = format!(
        "{}x{}",
        "<a>".repeat(MAX_DEPTH + 1),
        "</a>".repeat(MAX_DEPTH + 1)
    );
    assert!(xml::parse(&deep).is_err());

    let mut attributes = String::from("<a");
    for index in 0..=MAX_ATTRIBUTES {
        attributes.push_str(&format!(" a{index}=\"v\""));
    }
    attributes.push_str("></a>");
    assert!(xml::parse(&attributes).is_err());

    let long_name = format!("<{}></a>", "a".repeat(MAX_NAME_BYTES + 10));
    assert!(xml::parse(&long_name).is_err());
}

#[test]
fn namespace_prefixes_are_preserved_but_queries_are_local() {
    let document = xml::parse("<mx:a mx:b=\"1\"><mx:c/></mx:a>").unwrap();
    assert_eq!(document.root.name, "mx:a");
    assert_eq!(document.root.local_name(), "a");
    assert_eq!(document.root.attr("b"), Some("1"));
    assert!(document.root.child("c").is_some());
}

#[test]
fn writer_output_round_trips_hostile_text() {
    let mut writer = xml::XmlWriter::with_declaration();
    writer.start("root", &[("attr", "a&b<c>\"'")]).unwrap();
    writer.text_element("text", "line\n<tag>&value").unwrap();
    writer.end("root");
    let text = writer.finish();
    let document = xml::parse(&text).unwrap();
    assert_eq!(document.root.attr("attr"), Some("a&b<c>\"'"));
    assert_eq!(
        document.root.child("text").unwrap().text_trimmed(),
        "line\n<tag>&value"
    );
}

#[test]
fn empty_elements_and_helpers_work() {
    let mut element = Element::new("note");
    element.attributes.push(("id".to_string(), "1".to_string()));
    assert_eq!(element.attr("id"), Some("1"));
    assert!(element.children_named("x").next().is_none());
    assert!(!element.has_child("x"));
    let document = xml::parse("<a><b/></a>").unwrap();
    assert!(document.root.has_child("b"));
    assert_eq!(document.root.child_text("b"), Some(""));
}
