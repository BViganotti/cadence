//! Original dependency-free PDF 1.4 writer.
//!
//! Every glyph, including text, is emitted as a vector path or filled
//! rectangle. No font objects, external resources, timestamps or host data are
//! embedded, so identical layouts always produce identical bytes.

use crate::svg::num;
use cadence_core::{Error, Result};
use cadence_engrave::text;
use cadence_engrave::{Element, Layout, PathSeg};

pub fn render_pdf(layout: &Layout) -> Result<Vec<u8>> {
    if layout.pages.is_empty() {
        return Err(Error::msg("cannot export an empty layout"));
    }
    let page_count = layout.pages.len();
    let mut objects: Vec<Vec<u8>> = Vec::new();
    objects.push(
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
    );
    let mut kids = String::new();
    for index in 0..page_count {
        let page_object = 4 + index * 2;
        kids.push_str(&format!("{page_object} 0 R "));
    }
    objects.push(format!("<< /Type /Pages /Kids [{}] /Count {page_count} >>", kids.trim_end()).into_bytes());
    objects.push(
        format!(
            "<< /Producer (Cadence) /Creator (Cadence) /Title ({}) >>",
            escape_pdf_string(&layout.metadata.title)
        )
        .into_bytes(),
    );
    for page in &layout.pages {
        let content_number = 4 + 2 * (page.number - 1) + 1;
        objects.push(
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Resources << >> /Contents {content_number} 0 R >>",
                num(page.width),
                num(page.height)
            )
            .into_bytes(),
        );
        let content = page_content(page, page.height);
        objects.push(wrap_stream(&content));
    }
    let mut out = Vec::new();
    out.extend_from_slice(b"%PDF-1.4\n");
    out.extend_from_slice(b"%Cadence PDF\n");
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        out.extend_from_slice(object);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for offset in &offsets {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R /Info 3 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    Ok(out)
}

fn wrap_stream(content: &str) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(format!("<< /Length {} >>\nstream\n", content.len()).as_bytes());
    out.extend_from_slice(content.as_bytes());
    out.extend_from_slice(b"endstream");
    out
}

fn page_content(page: &cadence_engrave::PageLayout, page_height: f64) -> String {
    let mut out = String::new();
    out.push_str("q\n");
    out.push_str("1 J\n1 j\n");
    for element in &page.elements {
        match element {
            Element::Line {
                x1,
                y1,
                x2,
                y2,
                width,
            } => {
                out.push_str(&format!(
                    "{} w\n{} {} m\n{} {} l\nS\n",
                    num(*width),
                    num(*x1),
                    num(page_height - *y1),
                    num(*x2),
                    num(page_height - *y2)
                ));
            }
            Element::Rect {
                x,
                y,
                w,
                h,
                filled,
            } => {
                if *filled {
                    out.push_str(&format!(
                        "{} {} {} {} re\nf\n",
                        num(*x),
                        num(page_height - *y - *h),
                        num(*w),
                        num(*h)
                    ));
                } else {
                    out.push_str(&format!(
                        "0.6 w\n{} {} {} {} re\nS\n",
                        num(*x),
                        num(page_height - *y - *h),
                        num(*w),
                        num(*h)
                    ));
                }
            }
            Element::Ellipse {
                cx,
                cy,
                rx,
                ry,
                filled,
            } => {
                let k = 0.552_284_749_8;
                let x = *cx;
                let y = page_height - *cy;
                let path = format!(
                    "{} {} m\n{} {} {} {} {} {} c\n{} {} {} {} {} {} c\n{} {} {} {} {} {} c\n{} {} {} {} {} {} c\n{}",
                    num(x + rx),
                    num(y),
                    num(x + rx),
                    num(y + k * ry),
                    num(x + k * rx),
                    num(y + ry),
                    num(x),
                    num(y + ry),
                    num(x - k * rx),
                    num(y + ry),
                    num(x - rx),
                    num(y + k * ry),
                    num(x - rx),
                    num(y),
                    num(x - rx),
                    num(y - k * ry),
                    num(x - k * rx),
                    num(y - ry),
                    num(x),
                    num(y - ry),
                    num(x + k * rx),
                    num(y - ry),
                    num(x + rx),
                    num(y - k * ry),
                    num(x + rx),
                    num(y),
                    if *filled { "f" } else { "S" }
                );
                out.push_str(&path);
                out.push('\n');
            }
            Element::Path {
                segments,
                stroke,
                fill,
            } => {
                let mut d = String::new();
                for segment in segments {
                    match segment {
                        PathSeg::Move(point) => {
                            d.push_str(&format!(
                                "{} {} m\n",
                                num(point.x),
                                num(page_height - point.y)
                            ));
                        }
                        PathSeg::Line(point) => {
                            d.push_str(&format!(
                                "{} {} l\n",
                                num(point.x),
                                num(page_height - point.y)
                            ));
                        }
                        PathSeg::Cubic { c1, c2, to } => {
                            d.push_str(&format!(
                                "{} {} {} {} {} {} c\n",
                                num(c1.x),
                                num(page_height - c1.y),
                                num(c2.x),
                                num(page_height - c2.y),
                                num(to.x),
                                num(page_height - to.y)
                            ));
                        }
                        PathSeg::Close => d.push_str("h\n"),
                    }
                }
                if *stroke > 0.0 {
                    d.push_str(&format!("{} w\n", num(*stroke)));
                }
                d.push_str(if *fill && *stroke > 0.0 {
                    "B\n"
                } else if *fill {
                    "f\n"
                } else {
                    "S\n"
                });
                out.push_str(&d);
            }
            Element::Text {
                x,
                y,
                size,
                text: value,
                align,
            } => {
                let rects = text::rectangles(value, *x, *y, *size, *align);
                if rects.is_empty() {
                    continue;
                }
                out.push_str("q\n");
                for (rx, ry, rw, rh) in rects {
                    out.push_str(&format!(
                        "{} {} {} {} re\n",
                        num(rx),
                        num(page_height - ry - rh),
                        num(rw),
                        num(rh)
                    ));
                }
                out.push_str("f\nQ\n");
            }
        }
    }
    out.push_str("Q\n");
    out
}

fn escape_pdf_string(text_value: &str) -> String {
    let mut out = String::new();
    for c in text_value.chars() {
        match c {
            '(' => out.push_str("\\("),
            ')' => out.push_str("\\)"),
            '\\' => out.push_str("\\\\"),
            '\n' | '\r' | '\t' => out.push(' '),
            c if c.is_ascii() => out.push(c),
            _ => {
                // PDF literal strings are byte strings. Use a safe ASCII
                // replacement rather than embedding host encoding.
                out.push('?');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadence_engrave::{engrave, Options};

    fn layout() -> Layout {
        engrave(&cadence_core::Score::demo(), &Options::default()).unwrap()
    }

    #[test]
    fn pdf_has_header_pages_and_trailer() {
        let bytes = render_pdf(&layout()).unwrap();
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.starts_with("%PDF-1.4"));
        assert!(text.ends_with("%%EOF\n"));
        assert!(text.contains("/Type /Catalog"));
        assert!(text.contains("/Type /Pages"));
        assert_eq!(text.matches("/Type /Page ").count(), layout().page_count());
        assert!(!text.contains("CreationDate"));
        assert!(!text.contains("Cadence-"));
    }

    #[test]
    fn pdf_is_byte_stable() {
        let layout = layout();
        assert_eq!(render_pdf(&layout).unwrap(), render_pdf(&layout).unwrap());
    }

    #[test]
    fn xref_offsets_point_at_objects() {
        let bytes = render_pdf(&layout()).unwrap();
        let text = String::from_utf8_lossy(&bytes);
        let startxref = text.rfind("startxref\n").unwrap() + "startxref\n".len();
        let offset_text: String = text[startxref..]
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        let offset: usize = offset_text.parse().unwrap();
        assert!(text[offset..].starts_with("xref\n"));
        let xref_end = text[offset..].find("trailer").unwrap() + offset;
        let table = &text[offset..xref_end];
        let entries: Vec<&str> = table.lines().skip(2).collect();
        for (index, entry) in entries.iter().enumerate() {
            if entry.starts_with("0000000000") {
                continue;
            }
            let object_offset: usize = entry[..10].parse().unwrap();
            let expected = format!("{} 0 obj", index);
            assert!(
                text[object_offset..].starts_with(&expected),
                "object {} offset {}",
                index,
                object_offset
            );
        }
    }

    #[test]
    fn stream_lengths_match() {
        let bytes = render_pdf(&layout()).unwrap();
        let text = String::from_utf8_lossy(&bytes);
        let mut search = 0;
        while let Some(found) = text[search..].find("<< /Length ") {
            let start = search + found + "<< /Length ".len();
            let rest = &text[start..];
            let length: usize = rest
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>()
                .parse()
                .unwrap();
            let stream_start = rest.find("stream\n").unwrap() + start + "stream\n".len();
            let stream_end = stream_start + length;
            assert!(text[stream_end..].starts_with("endstream"));
            search = stream_end + 1;
        }
    }

    #[test]
    fn empty_layout_is_rejected() {
        let empty = Layout {
            metadata: cadence_core::Metadata::default(),
            options: Options::default(),
            pages: Vec::new(),
        };
        assert!(render_pdf(&empty).is_err());
    }
}
