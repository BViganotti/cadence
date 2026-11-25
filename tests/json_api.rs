//! Studio JSON parser/serializer acceptance tests.

use cadence_studio::json::{self, Value};

#[test]
fn values_round_trip_through_text() {
    let value = json::object(vec![
        ("string", json::string("héllo 😀\n\"quoted\"")),
        ("number", json::number(42.5)),
        ("integer", json::number(7)),
        ("bool", json::bool(false)),
        ("null", json::null()),
        (
            "array",
            json::array(vec![json::string("a"), json::number(-1), json::bool(true)]),
        ),
    ]);
    let text = value.stringify();
    let parsed = json::parse(&text).unwrap();
    assert_eq!(parsed.get("string").and_then(Value::as_str), Some("héllo 😀\n\"quoted\""));
    assert_eq!(parsed.get("number").and_then(Value::as_f64), Some(42.5));
    assert_eq!(parsed.get("integer").and_then(Value::as_u64), Some(7));
    assert_eq!(parsed.get("bool").and_then(Value::as_bool), Some(false));
    assert_eq!(parsed.get("array").unwrap().as_array().unwrap().len(), 3);
}

#[test]
fn unicode_escapes_and_surrogates_are_handled() {
    let value = json::parse(r#""\u00e9 \uD83D\uDE00 \n\t\/""#).unwrap();
    assert_eq!(value.as_str(), Some("é 😀 \n\t/"));
    assert!(json::parse(r#""\uD800""#).is_err());
    assert!(json::parse(r#""\uDC00x""#).is_err());
    assert!(json::parse(r#""\uD800\u0041""#).is_err());
}

#[test]
fn malformed_json_is_rejected() {
    let cases = [
        "",
        "{",
        "[",
        "{\"a\"}",
        "{\"a\":}",
        "{\"a\":1,}",
        "[1,]",
        "01",
        "1.",
        "1e",
        "truex",
        "\"unterminated",
        "\"\\q\"",
        "\"\u{0001}\"",
        "{\"a\":1}{\"b\":2}",
        "{\"a\":1,\"a\":2}",
    ];
    for case in cases {
        assert!(json::parse(case).is_err(), "accepted {case:?}");
    }
}

#[test]
fn numbers_are_finite_and_typed() {
    assert!(json::parse("1e999").is_err());
    assert!(json::parse("NaN").is_err());
    assert!(json::parse("-0").is_ok());
    let value = json::parse("-12.25").unwrap();
    assert_eq!(value.as_i64(), None);
    assert_eq!(value.as_f64(), Some(-12.25));
}

#[test]
fn nested_arrays_and_objects_respect_depth() {
    let mut text = String::from("\"ok\"");
    for _ in 0..10 {
        text = format!("[{text}]");
    }
    assert!(json::parse(&text).is_ok());
    let mut deep = String::from("1");
    for _ in 0..100 {
        deep = format!("[{deep}]");
    }
    assert!(json::parse(&deep).is_err());
}

#[test]
fn stringify_is_stable_and_escapes_controls() {
    let value = json::object(vec![
        ("a", json::string("\u{0001}\u{001f}")),
        ("b", json::number(1.0)),
    ]);
    let once = value.stringify();
    let twice = value.stringify();
    assert_eq!(once, twice);
    assert!(once.contains("\\u0001"));
    assert!(once.contains("\\u001f"));
    assert!(once.contains("\"b\":1"));
}

#[test]
fn helpers_build_ordered_objects() {
    let value = json::object(vec![
        ("z", json::number(1)),
        ("a", json::number(2)),
    ]);
    let text = value.stringify();
    assert!(text.find("\"z\"").unwrap() < text.find("\"a\"").unwrap());
    assert!(value.get("missing").is_none());
    assert!(Value::Null.get("x").is_none());
}
