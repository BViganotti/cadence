//! Original, bounded XML parser and writer.
//!
//! Only the subset needed by MusicXML is supported, but the parser is strict:
//! it rejects DTDs (and therefore external entities), malformed tags,
//! mismatched closing tags, unknown entities and any input over the hard
//! limits. No recursion is deeper than `MAX_DEPTH`.

use cadence_core::{Error, Result};

pub const MAX_XML_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_DEPTH: usize = 64;
pub const MAX_NODES: usize = 500_000;
pub const MAX_ATTRIBUTES: usize = 128;
pub const MAX_NAME_BYTES: usize = 128;
pub const MAX_ATTR_VALUE_BYTES: usize = 8 * 1024;
pub const MAX_TEXT_BYTES: usize = 2 * 1024 * 1024;

/// A parsed XML element. Namespace prefixes are preserved on element and
/// attribute names; query helpers strip them.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Element {
    pub name: String,
    pub attributes: Vec<(String, String)>,
    pub children: Vec<Element>,
    pub text: String,
}

impl Element {
    pub fn new(name: impl Into<String>) -> Element {
        Element {
            name: name.into(),
            attributes: Vec::new(),
            children: Vec::new(),
            text: String::new(),
        }
    }

    pub fn local_name(&self) -> &str {
        local_name(&self.name)
    }

    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(key, _)| key == name || local_name(key) == name)
            .map(|(_, value)| value.as_str())
    }

    pub fn child(&self, name: &str) -> Option<&Element> {
        self.children
            .iter()
            .find(|child| child.local_name() == name)
    }

    pub fn children_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Element> + 'a {
        self.children
            .iter()
            .filter(move |child| child.local_name() == name)
    }

    pub fn text_trimmed(&self) -> &str {
        self.text.trim()
    }

    /// Trimmed text of the first child with the given name.
    pub fn child_text(&self, name: &str) -> Option<&str> {
        self.child(name).map(|child| child.text_trimmed())
    }

    pub fn has_child(&self, name: &str) -> bool {
        self.child(name).is_some()
    }
}

pub fn local_name(name: &str) -> &str {
    match name.rsplit_once(':') {
        Some((_, local)) => local,
        None => name,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    pub root: Element,
}

pub fn parse(input: &str) -> Result<Document> {
    Parser::new(input).parse()
}

struct Parser<'a> {
    input: &'a [u8],
    pos: usize,
    depth: usize,
    nodes: usize,
    text_bytes: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Parser<'a> {
        Parser {
            input: input.as_bytes(),
            pos: 0,
            depth: 0,
            nodes: 0,
            text_bytes: 0,
        }
    }

    fn parse(mut self) -> Result<Document> {
        if self.input.len() > MAX_XML_BYTES {
            return Err(Error::Limit("XML input bytes"));
        }
        let input = std::str::from_utf8(self.input)
            .map_err(|_| Error::msg("XML input is not valid UTF-8"))?;
        if input.contains("<![CDATA[") && !input.contains("]]>") {
            return Err(Error::msg("unterminated CDATA section"));
        }
        // XML declarations, comments, whitespace and processing instructions
        // may precede the root element. A DOCTYPE is always rejected.
        loop {
            self.skip_whitespace();
            if self.starts_with("<?") {
                self.skip_until("?>")?;
            } else if self.starts_with("<!--") {
                self.skip_comment()?;
            } else if self.starts_with("<!DOCTYPE") || self.starts_with("<!doctype") {
                return Err(Error::msg("DOCTYPE is not supported"));
            } else {
                break;
            }
        }
        if self.at_end() {
            return Err(Error::UnexpectedEof);
        }
        if !self.starts_with("<") {
            return Err(Error::msg("expected root element"));
        }
        let root = self.parse_element()?;
        loop {
            self.skip_whitespace();
            if self.at_end() {
                break;
            }
            if self.starts_with("<!--") {
                self.skip_comment()?;
            } else if self.starts_with("<?") {
                self.skip_until("?>")?;
            } else {
                return Err(Error::msg("trailing content after root element"));
            }
        }
        Ok(Document { root })
    }

    fn parse_element(&mut self) -> Result<Element> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(Error::Limit("XML nesting depth"));
        }
        self.nodes += 1;
        if self.nodes > MAX_NODES {
            return Err(Error::Limit("XML nodes"));
        }
        self.expect("<")?;
        if self.starts_with("/") || self.starts_with("!") || self.starts_with("?") {
            return Err(Error::msg("expected element start"));
        }
        let name = self.parse_name()?;
        let mut element = Element::new(name);
        loop {
            self.skip_whitespace();
            if self.starts_with("/>") {
                self.pos += 2;
                self.depth -= 1;
                return Ok(element);
            }
            if self.starts_with(">") {
                self.pos += 1;
                break;
            }
            if self.at_end() {
                return Err(Error::UnexpectedEof);
            }
            if element.attributes.len() >= MAX_ATTRIBUTES {
                return Err(Error::Limit("XML attributes per element"));
            }
            let key = self.parse_name()?;
            self.skip_whitespace();
            self.expect("=")?;
            self.skip_whitespace();
            let value = self.parse_attribute_value()?;
            if element.attributes.iter().any(|(existing, _)| existing == &key) {
                return Err(Error::msg(format!("duplicate attribute {key:?}")));
            }
            element.attributes.push((key, value));
        }
        // Content until matching end tag.
        loop {
            if self.at_end() {
                return Err(Error::UnexpectedEof);
            }
            if self.starts_with("</") {
                self.pos += 2;
                let close = self.parse_name()?;
                if close != element.name {
                    return Err(Error::msg(format!(
                        "mismatched closing tag: expected </{}>, found </{}>",
                        element.name, close
                    )));
                }
                self.skip_whitespace();
                self.expect(">")?;
                self.depth -= 1;
                return Ok(element);
            }
            if self.starts_with("<!--") {
                self.skip_comment()?;
                continue;
            }
            if self.starts_with("<![CDATA[") {
                self.pos += 9;
                let start = self.pos;
                let mut end = None;
                while self.pos + 3 <= self.input.len() {
                    if &self.input[self.pos..self.pos + 3] == b"]]>" {
                        end = Some(self.pos);
                        self.pos += 3;
                        break;
                    }
                    self.pos += 1;
                }
                let end = end.ok_or_else(|| Error::msg("unterminated CDATA"))?;
                let text = String::from_utf8_lossy(&self.input[start..end]).into_owned();
                self.append_text(&text)?;
                element.text.push_str(&text);
                continue;
            }
            if self.starts_with("<?") {
                self.skip_until("?>")?;
                continue;
            }
            if self.starts_with("<") {
                let child = self.parse_element()?;
                element.children.push(child);
                continue;
            }
            let text = self.parse_text()?;
            self.append_text(&text)?;
            element.text.push_str(&text);
        }
    }

    fn append_text(&mut self, text: &str) -> Result<()> {
        self.text_bytes += text.len();
        if self.text_bytes > MAX_TEXT_BYTES {
            return Err(Error::Limit("XML text bytes"));
        }
        Ok(())
    }

    fn parse_text(&mut self) -> Result<String> {
        let mut out = String::new();
        let mut raw = Vec::new();
        while !self.at_end() {
            let byte = self.input[self.pos];
            if byte == b'<' {
                break;
            }
            if byte == b'&' {
                self.flush_raw(&mut raw, &mut out)?;
                self.pos += 1;
                out.push_str(&self.parse_entity()?);
                continue;
            }
            raw.push(byte);
            self.pos += 1;
        }
        self.flush_raw(&mut raw, &mut out)?;
        Ok(out)
    }

    fn flush_raw(&mut self, raw: &mut Vec<u8>, out: &mut String) -> Result<()> {
        if raw.is_empty() {
            return Ok(());
        }
        let text = std::str::from_utf8(raw).map_err(|_| Error::msg("invalid UTF-8 in text"))?;
        out.push_str(text);
        raw.clear();
        Ok(())
    }

    fn parse_attribute_value(&mut self) -> Result<String> {
        let quote = if self.starts_with("\"") {
            b'"'
        } else if self.starts_with("'") {
            b'\''
        } else {
            return Err(Error::msg("attribute value must be quoted"));
        };
        self.pos += 1;
        let mut out = String::new();
        let mut raw = Vec::new();
        let mut bytes = 0usize;
        loop {
            if self.at_end() {
                return Err(Error::UnexpectedEof);
            }
            let byte = self.input[self.pos];
            self.pos += 1;
            if byte == quote {
                self.flush_raw(&mut raw, &mut out)?;
                if bytes > MAX_ATTR_VALUE_BYTES {
                    return Err(Error::Limit("XML attribute value bytes"));
                }
                return Ok(out);
            }
            if byte == b'&' {
                self.flush_raw(&mut raw, &mut out)?;
                let entity = self.parse_entity()?;
                bytes += entity.len();
                out.push_str(&entity);
                continue;
            }
            bytes += 1;
            if byte == b'<' {
                return Err(Error::msg("'<' in attribute value"));
            }
            raw.push(byte);
        }
    }

    fn parse_entity(&mut self) -> Result<String> {
        let start = self.pos;
        let mut length = 0;
        while !self.at_end() && self.input[self.pos] != b';' {
            if length > 12 {
                return Err(Error::msg("entity name too long"));
            }
            self.pos += 1;
            length += 1;
        }
        if self.at_end() {
            return Err(Error::UnexpectedEof);
        }
        let name = std::str::from_utf8(&self.input[start..self.pos])
            .map_err(|_| Error::msg("invalid entity name"))?;
        self.pos += 1;
        let value = match name {
            "amp" => "&".to_string(),
            "lt" => "<".to_string(),
            "gt" => ">".to_string(),
            "quot" => "\"".to_string(),
            "apos" => "'".to_string(),
            _ => {
                if let Some(hex) = name.strip_prefix("#x").or_else(|| name.strip_prefix("#X")) {
                    decode_codepoint(u32::from_str_radix(hex, 16).map_err(|_| {
                        Error::msg(format!("invalid numeric entity &{name};"))
                    })?)?
                } else if let Some(decimal) = name.strip_prefix('#') {
                    decode_codepoint(decimal.parse::<u32>().map_err(|_| {
                        Error::msg(format!("invalid numeric entity &{name};"))
                    })?)?
                } else {
                    return Err(Error::msg(format!("unknown entity &{name};")));
                }
            }
        };
        Ok(value)
    }

    fn parse_name(&mut self) -> Result<String> {
        let start = self.pos;
        if self.at_end() {
            return Err(Error::UnexpectedEof);
        }
        let first = self.input[self.pos];
        if !is_name_start(first) {
            return Err(Error::msg(format!(
                "invalid name start byte 0x{first:02x}"
            )));
        }
        self.pos += 1;
        while !self.at_end() && is_name_char(self.input[self.pos]) {
            self.pos += 1;
            if self.pos - start > MAX_NAME_BYTES {
                return Err(Error::Limit("XML name bytes"));
            }
        }
        std::str::from_utf8(&self.input[start..self.pos])
            .map(|name| name.to_string())
            .map_err(|_| Error::msg("invalid UTF-8 in name"))
    }

    fn skip_comment(&mut self) -> Result<()> {
        self.expect("<!--")?;
        self.skip_until("-->")
    }

    fn skip_until(&mut self, marker: &str) -> Result<()> {
        let bytes = marker.as_bytes();
        while self.pos + bytes.len() <= self.input.len() {
            if &self.input[self.pos..self.pos + bytes.len()] == bytes {
                self.pos += bytes.len();
                return Ok(());
            }
            self.pos += 1;
        }
        Err(Error::UnexpectedEof)
    }

    fn skip_whitespace(&mut self) {
        while !self.at_end() && matches!(self.input[self.pos], b' ' | b'\t' | b'\r' | b'\n') {
            self.pos += 1;
        }
    }

    fn expect(&mut self, text: &str) -> Result<()> {
        if self.starts_with(text) {
            self.pos += text.len();
            Ok(())
        } else {
            Err(Error::msg(format!("expected {text:?}")))
        }
    }

    fn starts_with(&self, text: &str) -> bool {
        self.input[self.pos..].starts_with(text.as_bytes())
    }

    fn at_end(&self) -> bool {
        self.pos >= self.input.len()
    }
}

fn decode_codepoint(value: u32) -> Result<String> {
    if value > 0x10FFFF || (0xD800..=0xDFFF).contains(&value) {
        return Err(Error::msg("invalid Unicode codepoint in entity"));
    }
    char::from_u32(value)
        .map(|c| c.to_string())
        .ok_or_else(|| Error::msg("invalid Unicode codepoint in entity"))
}

fn is_name_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_' || byte == b':'
}

fn is_name_char(byte: u8) -> bool {
    is_name_start(byte) || byte.is_ascii_digit() || byte == b'-' || byte == b'.'
}

/// Deterministic pretty XML writer.
pub struct XmlWriter {
    out: String,
    depth: usize,
}

impl Default for XmlWriter {
    fn default() -> Self {
        XmlWriter::new()
    }
}

impl XmlWriter {
    pub fn new() -> XmlWriter {
        XmlWriter {
            out: String::new(),
            depth: 0,
        }
    }

    pub fn with_declaration() -> XmlWriter {
        let mut writer = XmlWriter::new();
        writer.out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        writer
    }

    pub fn start(&mut self, name: &str, attrs: &[(&str, &str)]) -> Result<()> {
        self.indent();
        self.out.push('<');
        self.out.push_str(name);
        for (key, value) in attrs {
            self.out.push(' ');
            self.out.push_str(key);
            self.out.push_str("=\"");
            escape_attr_into(&mut self.out, value);
            self.out.push('"');
        }
        self.out.push_str(">\n");
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(Error::Limit("XML writer depth"));
        }
        Ok(())
    }

    pub fn end(&mut self, name: &str) {
        self.depth = self.depth.saturating_sub(1);
        self.indent();
        self.out.push_str("</");
        self.out.push_str(name);
        self.out.push_str(">\n");
    }

    pub fn empty(&mut self, name: &str, attrs: &[(&str, &str)]) -> Result<()> {
        self.indent();
        self.out.push('<');
        self.out.push_str(name);
        for (key, value) in attrs {
            self.out.push(' ');
            self.out.push_str(key);
            self.out.push_str("=\"");
            escape_attr_into(&mut self.out, value);
            self.out.push('"');
        }
        self.out.push_str("/>\n");
        Ok(())
    }

    pub fn text_element(&mut self, name: &str, text: &str) -> Result<()> {
        self.start(name, &[])?;
        self.text(text);
        self.end(name);
        Ok(())
    }

    pub fn text(&mut self, text: &str) {
        self.indent();
        escape_text_into(&mut self.out, text);
        self.out.push('\n');
    }

    pub fn raw(&mut self, text: &str) {
        self.out.push_str(text);
    }

    pub fn finish(self) -> String {
        self.out
    }

    fn indent(&mut self) {
        for _ in 0..self.depth {
            self.out.push_str("  ");
        }
    }
}

pub fn escape_text_into(out: &mut String, text: &str) {
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
}

pub fn escape_attr_into(out: &mut String, text: &str) {
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            '\n' => out.push_str("&#10;"),
            '\r' => out.push_str("&#13;"),
            '\t' => out.push_str("&#9;"),
            _ => out.push(c),
        }
    }
}

pub fn escape_text(text: &str) -> String {
    let mut out = String::new();
    escape_text_into(&mut out, text);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested_musicxml_shape() {
        let xml = r#"<?xml version="1.0"?>
            <!-- comment -->
            <score-partwise version="4.0">
              <part id="P1"><measure number="1">
                <note><pitch><step>C</step><octave>4</octave></pitch></note>
              </measure></part>
            </score-partwise>"#;
        let document = parse(xml).unwrap();
        assert_eq!(document.root.local_name(), "score-partwise");
        let part = document.root.child("part").unwrap();
        assert_eq!(part.attr("id"), Some("P1"));
        let note = part.child("measure").unwrap().child("note").unwrap();
        assert_eq!(note.child("pitch").unwrap().child_text("step"), Some("C"));
    }

    #[test]
    fn decodes_entities_and_cdata() {
        let xml = "<a title=\"x &amp; &quot;y&quot;\">A &lt; B &#65; &#x42;<![CDATA[ <raw> ]]></a>";
        let document = parse(xml).unwrap();
        assert_eq!(document.root.attr("title"), Some("x & \"y\""));
        assert_eq!(document.root.text_trimmed(), "A < B A B <raw>");
    }

    #[test]
    fn namespaces_are_localized() {
        let xml = r#"<mx:score xmlns:mx="urn:test"><mx:part id="P1"/></mx:score>"#;
        let document = parse(xml).unwrap();
        assert_eq!(document.root.local_name(), "score");
        assert!(document.root.child("part").is_some());
    }

    #[test]
    fn rejects_doctype_and_external_entities() {
        let xml = "<!DOCTYPE foo [<!ENTITY x SYSTEM \"file:///etc/passwd\">]><foo>&x;</foo>";
        assert!(parse(xml).is_err());
    }

    #[test]
    fn rejects_mismatched_and_unclosed_tags() {
        assert!(parse("<a><b></a></b>").is_err());
        assert!(parse("<a><b></b>").is_err());
        assert!(parse("<a>").is_err());
        assert!(parse("").is_err());
        assert!(parse("<a></a><b></b>").is_err());
    }

    #[test]
    fn rejects_unknown_entities_and_bad_attributes() {
        assert!(parse("<a>&nope;</a>").is_err());
        assert!(parse("<a x=1></a>").is_err());
        assert!(parse("<a x=\"1\" x=\"2\"></a>").is_err());
        assert!(parse("<a x=\"<\"></a>").is_err());
    }

    #[test]
    fn enforces_depth_limit() {
        let mut xml = String::new();
        for _ in 0..(MAX_DEPTH + 2) {
            xml.push_str("<a>");
        }
        for _ in 0..(MAX_DEPTH + 2) {
            xml.push_str("</a>");
        }
        assert!(matches!(parse(&xml), Err(Error::Limit(_))));
    }

    #[test]
    fn writer_escapes_and_round_trips() {
        let mut writer = XmlWriter::with_declaration();
        writer.start("score", &[("title", "A&B \"quoted\"")]).unwrap();
        writer.text_element("lyric", "rock & roll <3").unwrap();
        writer.end("score");
        let xml = writer.finish();
        let document = parse(&xml).unwrap();
        assert_eq!(document.root.attr("title"), Some("A&B \"quoted\""));
        assert_eq!(
            document.root.child("lyric").unwrap().text_trimmed(),
            "rock & roll <3"
        );
    }

    #[test]
    fn numeric_entities_are_bounded() {
        assert!(parse("<a>&#x110000;</a>").is_err());
        assert!(parse("<a>&#xD800;</a>").is_err());
        assert_eq!(parse("<a>&#32;</a>").unwrap().root.text, " ");
    }
}
