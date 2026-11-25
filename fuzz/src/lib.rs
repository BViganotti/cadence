//! Deterministic malformed-input harness. This is a development tool; it is
//! also used by the workspace test suite to prove that parsing never panics.

use cadence_engrave::{engrave, Options};
use cadence_export::{render_midi, render_pdf, render_svg};
use cadence_import::import_score;
use std::panic::{catch_unwind, AssertUnwindSafe};

pub const BASE_XML: &str = include_str!("../../examples/demo.musicxml");

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FuzzReport {
    pub total: usize,
    pub rejected: usize,
    pub accepted: usize,
    pub panics: usize,
    pub panic_samples: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(seed.max(1))
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn below(&mut self, limit: usize) -> usize {
        if limit == 0 {
            0
        } else {
            (self.next() as usize) % limit
        }
    }
}

fn boundary(text: &str, position: usize) -> usize {
    let mut position = position.min(text.len());
    while position > 0 && !text.is_char_boundary(position) {
        position -= 1;
    }
    position
}

pub fn mutate(index: usize) -> String {
    let mut rng = Rng::new(0x9e37_79b9_7f4a_7c15 ^ index as u64);
    let base = BASE_XML;
    let specials = [
        "<", ">", "&", "\"", "'", "<?", "<!--", "]]>", "<![CDATA[", "</",
        "&bogus;", "&#x110000;", "&#xD800;", "<!DOCTYPE x>", "<a>", "</a>",
        "x=\"1\"", "\0", "\u{1f600}", " ", "\n",
    ];
    match index % 12 {
        0 => {
            let position = boundary(base, rng.below(base.len().max(1)));
            base[..position].to_string()
        }
        1 => {
            let mut text = base.to_string();
            for _ in 0..(1 + rng.below(12)) {
                let position = boundary(&text, rng.below(text.len().max(1)));
                let special = specials[rng.below(specials.len())];
                text.insert_str(position, special);
            }
            text
        }
        2 => {
            let start = boundary(base, rng.below(base.len().max(1)));
            let end = boundary(base, (start + rng.below(2048)).min(base.len()));
            let mut text = base.to_string();
            text.replace_range(start..end, "");
            text
        }
        3 => {
            let depth = 1 + rng.below(120);
            let mut text = String::new();
            for _ in 0..depth {
                text.push_str("<a>");
            }
            text.push_str("x");
            for _ in 0..depth {
                text.push_str("</a>");
            }
            text
        }
        4 => {
            let mut text = String::from("<score-partwise>");
            for _ in 0..(1 + rng.below(300)) {
                text.push_str("<part id=\"P1\" a=\"");
                for _ in 0..rng.below(200) {
                    text.push('x');
                }
                text.push_str("\"/>");
            }
            text.push_str("</score-partwise>");
            text
        }
        5 => {
            let length = 1 + rng.below(4096);
            (0..length)
                .map(|_| char::from((rng.next() % 95 + 32) as u8))
                .collect()
        }
        6 => {
            let position = boundary(base, rng.below(base.len().max(1)));
            let start = boundary(base, rng.below(base.len().max(1)));
            let end = boundary(base, (start + 1 + rng.below(512)).min(base.len()));
            let mut text = base.to_string();
            let chunk = base[start..end].to_string();
            text.insert_str(position, &chunk);
            text
        }
        7 => format!("<!DOCTYPE foo [<!ENTITY x SYSTEM \"file:///etc/passwd\">]>{base}"),
        8 => base.replace("</", "&"),
        9 => base.replace("480", &format!("{}", rng.next())),
        10 => {
            let insertion = format!(
                "<note><pitch><step>C</step><octave>4</octave></pitch><duration>320</duration><type>eighth</type><time-modification><actual-notes>3</actual-notes><normal-notes>2</normal-notes></time-modification></note>"
            );
            let position = base.find("<note>").unwrap_or(0);
            let mut text = base.to_string();
            text.insert_str(position, &insertion);
            text
        }
        _ => {
            let mut text = base.to_string();
            let position = boundary(&text, rng.below(text.len().max(1)));
            let replacement = specials[rng.below(specials.len())];
            let length = 1 + rng.below(8);
            let end = boundary(&text, (position + length).min(text.len()));
            text.replace_range(position..end, replacement);
            text
        }
    }
}

pub fn run(iterations: usize) -> FuzzReport {
    let mut report = FuzzReport::default();
    for index in 0..iterations {
        let input = mutate(index);
        report.total += 1;
        let result = catch_unwind(AssertUnwindSafe(|| import_score(&input)));
        match result {
            Ok(Ok(score)) => {
                report.accepted += 1;
                // Accepted mutated scores must also engrave/export without
                // panicking. Layout may reject extreme but valid scores.
                let options = Options::default();
                let layout_result = catch_unwind(AssertUnwindSafe(|| engrave(&score, &options)));
                match layout_result {
                    Ok(Ok(layout)) => {
                        let _ = render_svg(&layout);
                        let _ = render_pdf(&layout);
                        let _ = render_midi(&score);
                    }
                    Ok(Err(_)) => {}
                    Err(_) => {
                        report.panics += 1;
                        if report.panic_samples.len() < 8 {
                            report.panic_samples.push(format!("layout panic at {index}"));
                        }
                    }
                }
            }
            Ok(Err(_)) => report.rejected += 1,
            Err(_) => {
                report.panics += 1;
                if report.panic_samples.len() < 8 {
                    report.panic_samples.push(format!("parse panic at {index}"));
                }
            }
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_thousand_malformed_inputs_never_panic() {
        let report = run(1000);
        assert_eq!(report.total, 1000);
        assert_eq!(report.panics, 0, "{:?}", report.panic_samples);
        assert!(
            report.rejected >= 900,
            "expected most mutations to be rejected cleanly, got {report:?}"
        );
    }

    #[test]
    fn mutations_are_deterministic() {
        for index in [0, 1, 2, 99, 999] {
            assert_eq!(mutate(index), mutate(index));
        }
    }

    #[test]
    fn oversized_input_is_rejected() {
        let huge = "x".repeat(cadence_core::error::MAX_INPUT_BYTES + 1);
        assert!(import_score(&huge).is_err());
    }
}
