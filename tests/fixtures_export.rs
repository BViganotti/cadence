//! Whole-corpus engraving and export smoke test.

use cadence_engrave::{engrave, is_collision_free, Options};
use cadence_export::{render_midi, render_pdf, render_svg};
use cadence_import::import_score;

#[test]
fn every_fixture_engraves_and_exports_to_all_formats() {
    let directory = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/fixtures");
    let mut paths: Vec<_> = std::fs::read_dir(directory)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("musicxml"))
        .collect();
    paths.sort();
    assert!(paths.len() >= 100);
    let mut svg_bytes = 0usize;
    let mut pdf_bytes = 0usize;
    let mut midi_bytes = 0usize;
    for path in &paths {
        let xml = std::fs::read_to_string(path).unwrap();
        let score = import_score(&xml).unwrap();
        let layout = engrave(&score, &Options::default()).unwrap();
        assert!(is_collision_free(&layout), "{}", path.display());
        let svg = render_svg(&layout);
        assert!(svg.starts_with("<?xml"), "{}", path.display());
        assert!(svg.ends_with("</svg>\n"), "{}", path.display());
        let pdf = render_pdf(&layout).unwrap();
        assert!(pdf.starts_with(b"%PDF-1.4"), "{}", path.display());
        let midi = render_midi(&score).unwrap();
        assert!(midi.starts_with(b"MThd"), "{}", path.display());
        assert!(svg.len() > 1000);
        assert!(pdf.len() > 1000);
        assert!(midi.len() > 100);
        svg_bytes += svg.len();
        pdf_bytes += pdf.len();
        midi_bytes += midi.len();
    }
    assert!(svg_bytes > 1_000_000);
    assert!(pdf_bytes > 500_000);
    assert!(midi_bytes > 10_000);
}
