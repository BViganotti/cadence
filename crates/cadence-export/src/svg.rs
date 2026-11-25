//! Deterministic SVG writer. Text is drawn with the built-in bitmap font as
//! rectangles, so the output never references host fonts or external assets.

use cadence_engrave::text;
use cadence_engrave::{Element, Layout, PageLayout, PathSeg};

pub fn render_svg(layout: &Layout) -> String {
    render_svg_with_hits(layout, false)
}

/// Render every page stacked vertically. `include_hits` adds transparent
/// rectangles carrying stable data attributes for the Studio editor.
pub fn render_svg_with_hits(layout: &Layout, include_hits: bool) -> String {
    let page_gap = 24.0;
    let width = layout
        .pages
        .iter()
        .map(|page| page.width)
        .fold(0.0f64, f64::max)
        .max(1.0);
    let height = layout
        .pages
        .iter()
        .map(|page| page.height)
        .sum::<f64>()
        + page_gap * layout.pages.len().saturating_sub(1) as f64;
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">\n",
        num(width),
        num(height),
        num(width),
        num(height)
    ));
    out.push_str("<title>Cadence score</title>\n");
    let mut y = 0.0;
    for page in &layout.pages {
        out.push_str(&format!(
            "<g id=\"page-{}\" transform=\"translate(0 {})\">\n",
            page.number,
            num(y)
        ));
        out.push_str(&format!(
            "<rect x=\"0\" y=\"0\" width=\"{}\" height=\"{}\" fill=\"#ffffff\"/>\n",
            num(page.width),
            num(page.height)
        ));
        for element in &page.elements {
            render_element(&mut out, element);
        }
        if include_hits {
            for hit in &page.hits {
                out.push_str(&format!(
                    "<rect class=\"hit\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"transparent\" pointer-events=\"all\" data-part=\"{}\" data-measure=\"{}\" data-voice=\"{}\" data-event=\"{}\"/>\n",
                    num(hit.x),
                    num(hit.y),
                    num(hit.w),
                    num(hit.h),
                    hit.part,
                    hit.measure,
                    hit.voice,
                    hit.event
                ));
            }
        }
        out.push_str("</g>\n");
        y += page.height + page_gap;
    }
    out.push_str("</svg>\n");
    out
}

pub fn render_svg_page(layout: &Layout, page_index: usize, include_hits: bool) -> Option<String> {
    let page = layout.pages.get(page_index)?;
    Some(render_single_page(page, include_hits))
}

fn render_single_page(page: &PageLayout, include_hits: bool) -> String {
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">\n",
        num(page.width),
        num(page.height),
        num(page.width),
        num(page.height)
    ));
    out.push_str("<title>Cadence score</title>\n");
    out.push_str(&format!(
        "<rect x=\"0\" y=\"0\" width=\"{}\" height=\"{}\" fill=\"#ffffff\"/>\n",
        num(page.width),
        num(page.height)
    ));
    for element in &page.elements {
        render_element(&mut out, element);
    }
    if include_hits {
        for hit in &page.hits {
            out.push_str(&format!(
                "<rect class=\"hit\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"transparent\" pointer-events=\"all\" data-part=\"{}\" data-measure=\"{}\" data-voice=\"{}\" data-event=\"{}\"/>\n",
                num(hit.x),
                num(hit.y),
                num(hit.w),
                num(hit.h),
                hit.part,
                hit.measure,
                hit.voice,
                hit.event
            ));
        }
    }
    out.push_str("</svg>\n");
    out
}

fn render_element(out: &mut String, element: &Element) {
    match element {
        Element::Line {
            x1,
            y1,
            x2,
            y2,
            width,
        } => {
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#000000\" stroke-width=\"{}\" stroke-linecap=\"round\"/>\n",
                num(*x1),
                num(*y1),
                num(*x2),
                num(*y2),
                num(*width)
            ));
        }
        Element::Rect {
            x,
            y,
            w,
            h,
            filled,
        } => {
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"0.6\"/>\n",
                num(*x),
                num(*y),
                num(*w),
                num(*h),
                if *filled { "#000000" } else { "none" },
                if *filled { "none" } else { "#000000" }
            ));
        }
        Element::Ellipse {
            cx,
            cy,
            rx,
            ry,
            filled,
        } => {
            out.push_str(&format!(
                "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"0.6\"/>\n",
                num(*cx),
                num(*cy),
                num(*rx),
                num(*ry),
                if *filled { "#000000" } else { "none" },
                if *filled { "none" } else { "#000000" }
            ));
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
                        d.push_str(&format!("M {} {} ", num(point.x), num(point.y)));
                    }
                    PathSeg::Line(point) => {
                        d.push_str(&format!("L {} {} ", num(point.x), num(point.y)));
                    }
                    PathSeg::Cubic { c1, c2, to } => {
                        d.push_str(&format!(
                            "C {} {} {} {} {} {} ",
                            num(c1.x),
                            num(c1.y),
                            num(c2.x),
                            num(c2.y),
                            num(to.x),
                            num(to.y)
                        ));
                    }
                    PathSeg::Close => d.push_str("Z "),
                }
            }
            out.push_str(&format!(
                "<path d=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>\n",
                d.trim_end(),
                if *fill { "#000000" } else { "none" },
                if *stroke > 0.0 { "#000000" } else { "none" },
                num(*stroke)
            ));
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
                return;
            }
            out.push_str("<g class=\"text\">\n");
            for (rx, ry, rw, rh) in rects {
                out.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#000000\"/>\n",
                    num(rx),
                    num(ry),
                    num(rw),
                    num(rh)
                ));
            }
            out.push_str("</g>\n");
        }
    }
}

/// Fixed three-decimal formatting with no negative zero or trailing zeros.
pub fn num(value: f64) -> String {
    if !value.is_finite() {
        return "0".to_string();
    }
    let mut text = format!("{value:.3}");
    while text.contains('.') && text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    if text == "-0" {
        text = "0".to_string();
    }
    text
}

pub fn escape_xml(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadence_engrave::{engrave, Options};

    #[test]
    fn svg_is_deterministic_and_self_contained() {
        let layout = engrave(&cadence_core::Score::demo(), &Options::default()).unwrap();
        let a = render_svg(&layout);
        let b = render_svg(&layout);
        assert_eq!(a, b);
        assert!(a.starts_with("<?xml"));
        assert!(a.contains("<svg"));
        assert!(a.contains("</svg>"));
        assert!(!a.contains("<text"));
        assert_eq!(a.matches("http://").count(), 1);
        assert!(!a.contains("https://"));
        assert!(!a.contains("href"));
        assert!(!a.contains("font-family"));
        assert!(a.contains("page-1"));
    }

    #[test]
    fn svg_pages_can_be_rendered_individually() {
        let layout = engrave(&cadence_core::Score::demo(), &Options::default()).unwrap();
        for index in 0..layout.page_count() {
            let page = render_svg_page(&layout, index, true).unwrap();
            assert!(page.contains("data-part="));
            assert!(page.ends_with("</svg>\n"));
        }
        assert!(render_svg_page(&layout, 999, false).is_none());
    }

    #[test]
    fn numbers_format_consistently() {
        assert_eq!(num(1.0), "1");
        assert_eq!(num(1.23456), "1.235");
        assert_eq!(num(-0.0), "0");
        assert_eq!(num(-1.2), "-1.2");
        assert_eq!(num(f64::NAN), "0");
    }

    #[test]
    fn xml_is_escaped() {
        assert_eq!(escape_xml("a&b<c>\"d\""), "a&amp;b&lt;c&gt;&quot;d&quot;");
    }
}
