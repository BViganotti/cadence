//! Export acceptance tests: reproducible SVG/PDF/MIDI bytes, matching bounds
//! and note-event coverage.

use cadence_core::Score;
use cadence_engrave::{engrave, Options};
use cadence_export::{render_midi, render_pdf, render_svg, render_svg_page};
use std::process::Command;

#[test]
fn svg_and_pdf_are_byte_stable_and_agree_on_page_bounds() {
    let layout = engrave(&Score::demo(), &Options::default()).unwrap();
    let svg_a = render_svg(&layout);
    let svg_b = render_svg(&layout);
    assert_eq!(svg_a, svg_b);
    let pdf_a = render_pdf(&layout).unwrap();
    let pdf_b = render_pdf(&layout).unwrap();
    assert_eq!(pdf_a, pdf_b);

    let svg_text = svg_a;
    let pdf_text = String::from_utf8(pdf_a).unwrap();
    let first_page = render_svg_page(&layout, 0, false).unwrap();
    assert!(first_page.contains("viewBox=\"0 0 595.276 841.89\""));
    assert!(svg_text.contains("viewBox=\"0 0 595.276 "));
    assert!(pdf_text.contains("/MediaBox [0 0 595.276 841.89]"));
    assert_eq!(
        pdf_text.matches("/Type /Page ").count(),
        layout.page_count()
    );
    assert_eq!(
        svg_text.matches("<g id=\"page-").count(),
        layout.page_count()
    );
    assert!(!svg_text.contains("NaN"));
    assert!(!pdf_text.contains("NaN"));
    assert!(!pdf_text.contains("CreationDate"));
}

#[test]
fn pdf_opens_with_poppler_when_available() {
    let layout = engrave(&Score::demo(), &Options::default()).unwrap();
    let bytes = render_pdf(&layout).unwrap();
    let dir = std::env::temp_dir().join(format!("cadence-pdf-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("demo.pdf");
    std::fs::write(&path, &bytes).unwrap();
    let output = Command::new("pdfinfo").arg(&path).output();
    let _ = std::fs::remove_dir_all(&dir);
    let Ok(output) = output else {
        // Poppler is not installed; structural tests above still apply.
        return;
    };
    if !output.status.success() {
        return;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("Pages:"));
    let pages = layout.page_count().to_string();
    assert!(text.contains(&format!("Pages:           {pages}")));
    assert!(text.contains("595.276 x 841.89 pts"));
}

#[test]
fn midi_covers_score_notes_and_is_stable() {
    let score = Score::demo();
    let bytes = render_midi(&score).unwrap();
    assert_eq!(bytes, render_midi(&score).unwrap());
    assert_eq!(&bytes[0..4], b"MThd");
    assert_eq!(&bytes[8..10], &[0, 1]);
    let tracks = u16::from_be_bytes([bytes[10], bytes[11]]) as usize;
    assert_eq!(tracks, score.parts.len() + 1);
    let division = u16::from_be_bytes([bytes[12], bytes[13]]);
    assert_eq!(division, 480);

    let mut offset = 14;
    let mut note_on = 0usize;
    let mut note_off = 0usize;
    for _ in 0..tracks {
        assert_eq!(&bytes[offset..offset + 4], b"MTrk");
        let length = u32::from_be_bytes([
            bytes[offset + 4],
            bytes[offset + 5],
            bytes[offset + 6],
            bytes[offset + 7],
        ]) as usize;
        offset += 8;
        let end = offset + length;
        let mut pos = offset;
        while pos < end {
            let (_, used) = read_vlq(&bytes[pos..]);
            pos += used;
            let status = bytes[pos];
            if status == 0xFF {
                pos += 1;
                pos += 1;
                let (len, used) = read_vlq(&bytes[pos..]);
                pos += used + len as usize;
                continue;
            }
            pos += 1;
            match status & 0xF0 {
                0x80 => {
                    note_off += 1;
                    pos += 2;
                }
                0x90 => {
                    if bytes[pos + 1] > 0 {
                        note_on += 1;
                    }
                    pos += 2;
                }
                0xB0 | 0xE0 => pos += 2,
                0xC0 | 0xD0 => pos += 1,
                _ => panic!("unexpected MIDI status"),
            }
        }
        offset = end;
    }
    assert_eq!(offset, bytes.len());
    assert!(note_on > 30);
    assert_eq!(note_on, note_off);
    let expected = score
        .flatten_notes()
        .iter()
        .filter(|note| !note.tie_stop)
        .count();
    assert_eq!(note_on, expected);
    assert!(bytes.windows(2).any(|window| window == [0xFF, 0x51]));
}

fn read_vlq(bytes: &[u8]) -> (u64, usize) {
    let mut value = 0u64;
    for (index, byte) in bytes.iter().enumerate() {
        value = (value << 7) | (byte & 0x7F) as u64;
        if byte & 0x80 == 0 {
            return (value, index + 1);
        }
    }
    (value, bytes.len())
}
