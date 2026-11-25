//! Golden byte-stability hashes for the demo pipeline.
//!
//! The hash is FNV-1a 64. These values intentionally change only when the
//! engraved layout or a writer changes; the test then forces a deliberate
//! review of the output instead of silently accepting drift.

use cadence_core::Score;
use cadence_engrave::{engrave, Options};
use cadence_export::{render_midi, render_pdf, render_svg};
use cadence_import::export_score;

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

#[test]
fn demo_exports_match_golden_hashes() {
    let score = Score::demo();
    let layout = engrave(&score, &Options::default()).unwrap();
    let svg = render_svg(&layout);
    let pdf = render_pdf(&layout).unwrap();
    let midi = render_midi(&score).unwrap();
    let musicxml = export_score(&score).unwrap();
    assert_eq!(fnv1a(svg.as_bytes()), 112_618_641_382_481_016);
    assert_eq!(fnv1a(&pdf), 9_142_888_463_643_682_141);
    assert_eq!(fnv1a(&midi), 7_133_691_583_623_973_938);
    assert_eq!(fnv1a(musicxml.as_bytes()), 15_465_127_307_851_594_771);
}

#[test]
fn repeated_exports_are_identical() {
    let score = Score::demo();
    let layout = engrave(&score, &Options::default()).unwrap();
    for _ in 0..3 {
        assert_eq!(render_svg(&layout), render_svg(&layout));
        assert_eq!(render_pdf(&layout).unwrap(), render_pdf(&layout).unwrap());
        assert_eq!(render_midi(&score).unwrap(), render_midi(&score).unwrap());
        assert_eq!(export_score(&score).unwrap(), export_score(&score).unwrap());
    }
}

#[test]
fn hash_function_detects_single_byte_changes() {
    let base = b"cadence";
    let mut changed = base.to_vec();
    changed[0] ^= 1;
    assert_ne!(fnv1a(base), fnv1a(&changed));
}
