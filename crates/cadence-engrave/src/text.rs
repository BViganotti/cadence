//! Original built-in 5x7 bitmap font used for every text run in the score.
//!
//! The music glyphs are vector paths; text is rendered from this tiny font so
//! SVG and PDF output do not depend on host fonts. Lowercase letters map to
//! their uppercase forms, which keeps lyric text legible at engraving sizes.
//! Unknown characters render as a small box, never as a missing glyph.

pub const GLYPH_W: f64 = 5.0;
pub const GLYPH_H: f64 = 7.0;
pub const ADVANCE: f64 = 6.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Start,
    Center,
    End,
}

/// Returns the seven rows of a glyph, bit 4 is the leftmost column.
pub fn glyph(c: char) -> [u8; 7] {
    let upper = if c.is_ascii_lowercase() {
        c.to_ascii_uppercase()
    } else {
        c
    };
    match upper {
        'A' => [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'B' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110],
        'C' => [0b01111, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b01111],
        'D' => [0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110],
        'E' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
        'F' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000],
        'G' => [0b01111, 0b10000, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111],
        'H' => [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'I' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111],
        'J' => [0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100],
        'K' => [0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001],
        'L' => [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111],
        'M' => [0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001],
        'N' => [0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001],
        'O' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'P' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000],
        'Q' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101],
        'R' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
        'S' => [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110],
        'T' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
        'U' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'V' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100],
        'W' => [0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001],
        'X' => [0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001],
        'Y' => [0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100],
        'Z' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111],
        '0' => [0b01110, 0b10011, 0b10101, 0b10101, 0b11001, 0b10001, 0b01110],
        '1' => [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        '2' => [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111],
        '3' => [0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110],
        '4' => [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
        '5' => [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110],
        '6' => [0b01110, 0b10000, 0b11110, 0b10001, 0b10001, 0b10001, 0b01110],
        '7' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
        '8' => [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
        '9' => [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001, 0b01110],
        ' ' => [0, 0, 0, 0, 0, 0, 0],
        '.' => [0, 0, 0, 0, 0, 0b01100, 0b01100],
        ',' => [0, 0, 0, 0, 0b00110, 0b00110, 0b00100],
        '-' => [0, 0, 0, 0b11111, 0, 0, 0],
        '_' => [0, 0, 0, 0, 0, 0, 0b11111],
        '+' => [0, 0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0],
        '=' => [0, 0b11111, 0, 0b11111, 0, 0, 0],
        '/' => [0b00001, 0b00010, 0b00010, 0b00100, 0b01000, 0b01000, 0b10000],
        '\\' => [0b10000, 0b01000, 0b01000, 0b00100, 0b00010, 0b00010, 0b00001],
        ':' => [0, 0b01100, 0b01100, 0, 0b01100, 0b01100, 0],
        ';' => [0, 0b01100, 0b01100, 0, 0b01100, 0b00100, 0b01000],
        '!' => [0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0, 0b00100],
        '?' => [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0, 0b00100],
        '\'' => [0b00100, 0b00100, 0b01000, 0, 0, 0, 0],
        '"' => [0b01010, 0b01010, 0, 0, 0, 0, 0],
        '(' => [0b00010, 0b00100, 0b01000, 0b01000, 0b01000, 0b00100, 0b00010],
        ')' => [0b01000, 0b00100, 0b00010, 0b00010, 0b00010, 0b00100, 0b01000],
        '[' => [0b01110, 0b01000, 0b01000, 0b01000, 0b01000, 0b01000, 0b01110],
        ']' => [0b01110, 0b00010, 0b00010, 0b00010, 0b00010, 0b00010, 0b01110],
        '{' => [0b00110, 0b00100, 0b00100, 0b01000, 0b00100, 0b00100, 0b00110],
        '}' => [0b01100, 0b00100, 0b00100, 0b00010, 0b00100, 0b00100, 0b01100],
        '<' => [0b00010, 0b00100, 0b01000, 0b10000, 0b01000, 0b00100, 0b00010],
        '>' => [0b01000, 0b00100, 0b00010, 0b00001, 0b00010, 0b00100, 0b01000],
        '&' => [0b01100, 0b10010, 0b10100, 0b01000, 0b10101, 0b10010, 0b01101],
        '*' => [0, 0b10101, 0b01110, 0b11111, 0b01110, 0b10101, 0],
        '#' => [0b01010, 0b01010, 0b11111, 0b01010, 0b11111, 0b01010, 0b01010],
        '%' => [0b11001, 0b11010, 0b00010, 0b00100, 0b01000, 0b01011, 0b10011],
        '@' => [0b01110, 0b10001, 0b10111, 0b10101, 0b10111, 0b10000, 0b01110],
        '\u{00b7}' => [0, 0, 0, 0b01100, 0b01100, 0, 0],
        _ => [0b11111, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11111],
    }
}

/// Width of a text run in points.
pub fn width(text: &str, size: f64) -> f64 {
    if text.is_empty() {
        return 0.0;
    }
    let scale = size / GLYPH_H;
    let chars = text.chars().count() as f64;
    (chars * ADVANCE - 1.0).max(0.0) * scale
}

/// Pixel-geometry for a text run: a list of filled rectangles.
pub fn rectangles(text: &str, x: f64, y: f64, size: f64, align: Align) -> Vec<(f64, f64, f64, f64)> {
    let total = width(text, size);
    let origin = match align {
        Align::Start => x,
        Align::Center => x - total / 2.0,
        Align::End => x - total,
    };
    let scale = size / GLYPH_H;
    let mut out = Vec::new();
    for (index, c) in text.chars().enumerate() {
        let glyph = glyph(c);
        let char_x = origin + index as f64 * ADVANCE * scale;
        for (row, bits) in glyph.iter().enumerate() {
            let mut col = 0;
            while col < 5 {
                if bits & (1 << (4 - col)) != 0 {
                    let start = col;
                    while col < 5 && bits & (1 << (4 - col)) != 0 {
                        col += 1;
                    }
                    out.push((
                        char_x + start as f64 * scale,
                        y + row as f64 * scale,
                        (col - start) as f64 * scale,
                        scale * 1.05,
                    ));
                } else {
                    col += 1;
                }
            }
        }
    }
    out
}

/// Approximate width at a given size, including hyphens/underscores.
pub fn lyric_width(text: &str, size: f64, hyphen: bool) -> f64 {
    width(text, size) + if hyphen { width("-", size) + 2.0 } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn width_scales_with_size_and_length() {
        assert_eq!(width("", 10.0), 0.0);
        assert!(width("A", 10.0) > 0.0);
        assert!(width("AB", 10.0) > width("A", 10.0));
        assert!((width("A", 20.0) - width("A", 10.0) * 2.0).abs() < 1e-9);
    }

    #[test]
    fn rectangles_are_finite_and_positive() {
        let rects = rectangles("Cadence 123", 0.0, 0.0, 9.0, Align::Start);
        assert!(!rects.is_empty());
        for (_, _, w, h) in rects {
            assert!(w.is_finite() && w > 0.0);
            assert!(h.is_finite() && h > 0.0);
        }
    }

    #[test]
    fn unknown_characters_have_a_box() {
        let glyph = glyph('\u{2603}');
        assert_eq!(glyph[0], 0b11111);
    }

    #[test]
    fn alignment_is_respected() {
        let start = rectangles("AB", 100.0, 0.0, 7.0, Align::Start);
        let end = rectangles("AB", 100.0, 0.0, 7.0, Align::End);
        let center = rectangles("AB", 100.0, 0.0, 7.0, Align::Center);
        let start_x = start.iter().map(|r| r.0).fold(f64::INFINITY, f64::min);
        let end_x = end.iter().map(|r| r.0).fold(f64::INFINITY, f64::min);
        let center_x = center
            .iter()
            .map(|r| r.0)
            .fold(f64::INFINITY, f64::min);
        assert!(start_x > end_x);
        assert!(center_x > end_x && center_x < start_x);
    }
}
