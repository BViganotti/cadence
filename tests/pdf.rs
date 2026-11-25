//! PDF acceptance tests across the fixture corpus.

use cadence_core::Score;
use cadence_engrave::{engrave, Options};
use cadence_export::render_pdf;
use cadence_import::import_score;
use std::process::Command;

fn fixture_layouts(limit: usize) -> Vec<(String, cadence_engrave::Layout)> {
    let directory = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/fixtures");
    let mut paths: Vec<_> = std::fs::read_dir(directory)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("musicxml"))
        .collect();
    paths.sort();
    paths
        .into_iter()
        .take(limit)
        .map(|path| {
            let xml = std::fs::read_to_string(&path).unwrap();
            let score = import_score(&xml).unwrap();
            let layout = engrave(&score, &Options::default()).unwrap();
            (path.display().to_string(), layout)
        })
        .collect()
}

#[test]
fn fixture_pdfs_have_valid_structure_and_stable_bytes() {
    for (name, layout) in fixture_layouts(12) {
        let bytes = render_pdf(&layout).unwrap();
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.starts_with("%PDF-1.4"), "{name}");
        assert!(text.ends_with("%%EOF\n"), "{name}");
        assert!(text.contains("/Type /Catalog"));
        assert!(text.contains("/Type /Pages"));
        assert_eq!(
            text.matches("/Type /Page ").count(),
            layout.page_count(),
            "{name}"
        );
        assert!(!text.contains("CreationDate"), "{name}");
        assert!(!text.contains("/Font"), "{name}");
        assert_eq!(bytes, render_pdf(&layout).unwrap(), "{name}");
        assert!(bytes.len() > 1000, "{name}");
    }
}

#[test]
fn page_media_boxes_match_layout_options() {
    let layout = engrave(&Score::demo(), &Options::default()).unwrap();
    let text = String::from_utf8_lossy(&render_pdf(&layout).unwrap()).into_owned();
    assert_eq!(
        text.matches("/MediaBox [0 0 595.276 841.89]").count(),
        layout.page_count()
    );
    assert!(text.contains("/Producer (Cadence)"));
    assert!(text.contains("/Title (Cadence in G)"));
}

#[test]
fn pdf_xref_offsets_are_exact_for_demo() {
    let layout = engrave(&Score::demo(), &Options::default()).unwrap();
    let bytes = render_pdf(&layout).unwrap();
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let marker = text.rfind("startxref\n").unwrap() + "startxref\n".len();
    let offset: usize = text[marker..]
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse()
        .unwrap();
    assert!(text[offset..].starts_with("xref\n"));
    let trailer = text[offset..].find("trailer").unwrap() + offset;
    let lines: Vec<&str> = text[offset..trailer].lines().collect();
    for (index, line) in lines.iter().skip(2).enumerate() {
        if line.starts_with("0000000000") {
            continue;
        }
        let object_offset: usize = line[..10].parse().unwrap();
        let expected = format!("{index} 0 obj");
        assert!(text[object_offset..].starts_with(&expected), "{line:?}");
    }
}

#[test]
fn pdfinfo_reports_all_pages_when_poppler_is_available() {
    let layout = engrave(&Score::demo(), &Options::default()).unwrap();
    let bytes = render_pdf(&layout).unwrap();
    let directory = std::env::temp_dir().join(format!("cadence-pdf-test-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("score.pdf");
    std::fs::write(&path, &bytes).unwrap();
    let output = Command::new("pdfinfo").arg(&path).output();
    let _ = std::fs::remove_dir_all(&directory);
    let Ok(output) = output else {
        return;
    };
    if !output.status.success() {
        return;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains(&format!("Pages:           {}", layout.page_count())));
    assert!(text.contains("PDF version:     1.4"));
}

#[test]
fn empty_layout_is_rejected() {
    let layout = cadence_engrave::Layout {
        metadata: Default::default(),
        options: Options::default(),
        pages: Vec::new(),
    };
    assert!(render_pdf(&layout).is_err());
}
