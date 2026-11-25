//! SVG acceptance tests.

use cadence_core::Score;
use cadence_engrave::{engrave, Options};
use cadence_export::{render_svg, render_svg_page, render_svg_with_hits};
use cadence_import::import_score;

#[test]
fn svg_is_well_formed_xml_and_self_contained() {
    let layout = engrave(&Score::demo(), &Options::default()).unwrap();
    let svg = render_svg(&layout);
    let document = cadence_import::xml::parse(&svg).expect("SVG must be valid XML");
    assert_eq!(document.root.local_name(), "svg");
    assert!(document.root.attr("viewBox").is_some());
    assert!(!svg.contains("<text"));
    assert!(!svg.contains("font-family"));
    assert!(!svg.contains("href"));
    assert!(!svg.contains("NaN"));
    assert!(!svg.contains("Infinity"));
    assert_eq!(svg.matches("http://").count(), 1);
}

#[test]
fn combined_and_single_page_output_agree_on_page_count() {
    let layout = engrave(&Score::demo(), &Options::default()).unwrap();
    let combined = render_svg(&layout);
    assert_eq!(
        combined.matches("<g id=\"page-").count(),
        layout.page_count()
    );
    for index in 0..layout.page_count() {
        let page = render_svg_page(&layout, index, false).unwrap();
        assert!(page.contains("width=\"595.276\""));
        assert!(page.contains("height=\"841.89\""));
        cadence_import::xml::parse(&page).unwrap();
    }
    assert!(render_svg_page(&layout, layout.page_count(), false).is_none());
}

#[test]
fn hit_regions_are_emitted_for_every_event() {
    let layout = engrave(&Score::demo(), &Options::default()).unwrap();
    let svg = render_svg_with_hits(&layout, true);
    assert_eq!(svg.matches("class=\"hit\"").count(), layout.hits().count());
    assert!(svg.contains("data-part="));
    assert!(svg.contains("data-measure="));
    assert!(svg.contains("data-voice="));
    assert!(svg.contains("data-event="));
    assert!(!render_svg_with_hits(&layout, false).contains("class=\"hit\""));
}

#[test]
fn svg_is_deterministic_for_fixtures() {
    let directory = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/fixtures");
    let mut paths: Vec<_> = std::fs::read_dir(directory)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("musicxml"))
        .collect();
    paths.sort();
    for path in paths.into_iter().take(12) {
        let xml = std::fs::read_to_string(&path).unwrap();
        let score = import_score(&xml).unwrap();
        let layout = engrave(&score, &Options::default()).unwrap();
        let a = render_svg(&layout);
        let b = render_svg(&layout);
        assert_eq!(a, b, "{}", path.display());
        cadence_import::xml::parse(&a).unwrap();
    }
}

#[test]
fn shape_primitives_render_as_expected_tags() {
    let layout = engrave(&Score::demo(), &Options::default()).unwrap();
    let svg = render_svg(&layout);
    assert!(svg.contains("<line "));
    assert!(svg.contains("<ellipse "));
    assert!(svg.contains("<path "));
    assert!(svg.contains("<rect "));
    assert!(svg.contains("<g class=\"text\">"));
}
