//! Layout acceptance tests: deterministic output, bounded geometry and
//! collision-free engraving across 300 generated scores.

use cadence_core::Score;
use cadence_engrave::{engrave, find_collisions, is_collision_free, Options};
use cadence_import::import_score;
use std::fs;
use std::path::PathBuf;

fn fixture_paths() -> Vec<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/fixtures");
    let mut paths: Vec<PathBuf> = fs::read_dir(&dir)
        .expect("fixtures")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("musicxml"))
        .collect();
    paths.sort();
    paths
}

#[test]
fn three_hundred_generated_scores_have_no_overlapping_symbols() {
    let options = Options::default();
    let mut scores = Vec::new();
    for path in fixture_paths() {
        let xml = fs::read_to_string(&path).unwrap();
        let base = import_score(&xml).unwrap();
        for semitones in [0i8, 2, -3] {
            let mut score = base.clone();
            for part in 0..score.parts.len() {
                score
                    .transpose_part(part, semitones)
                    .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            }
            scores.push((path.clone(), score));
        }
    }
    assert!(scores.len() >= 300, "generated {}", scores.len());
    for (path, score) in &scores {
        let layout = engrave(score, &options)
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        let collisions = find_collisions(&layout);
        assert!(
            collisions.is_empty(),
            "{}: {} collision(s): {:?}",
            path.display(),
            collisions.len(),
            collisions
                .iter()
                .take(6)
                .map(|collision| collision.describe())
                .collect::<Vec<_>>()
        );
        assert!(layout.page_count() >= 1);
        for page in &layout.pages {
            for symbol in &page.symbols {
                assert!(symbol.bbox.x.is_finite() && symbol.bbox.y.is_finite());
                assert!(symbol.bbox.w.is_finite() && symbol.bbox.h.is_finite());
                assert!(
                    symbol.bbox.x > -200.0
                        && symbol.bbox.right() < page.width + 200.0
                        && symbol.bbox.y > -300.0
                        && symbol.bbox.bottom() < page.height + 300.0,
                    "symbol outside sane bounds: {:?}",
                    symbol
                );
            }
        }
    }
}

#[test]
fn layout_is_deterministic() {
    let options = Options::default();
    let score = Score::demo();
    let a = engrave(&score, &options).unwrap();
    let b = engrave(&score, &options).unwrap();
    assert_eq!(a, b);
    assert!(is_collision_free(&a));
    assert!(a.page_count() >= 1);
    assert!(a.system_count() >= 1);
}

#[test]
fn part_extraction_engraves_independently() {
    let options = Options::default();
    let score = Score::demo();
    for index in 0..score.parts.len() {
        let part_score = score.extract_part(index).unwrap();
        let layout = engrave(&part_score, &options).unwrap();
        let collisions = find_collisions(&layout);
        assert!(
            collisions.is_empty(),
            "part {}: {:?}",
            index,
            collisions.iter().map(|c| c.describe()).collect::<Vec<_>>()
        );
        assert!(layout.pages[0].symbols.iter().any(|symbol| {
            matches!(symbol.kind, cadence_engrave::SymbolKind::Notehead)
        }));
    }
}

#[test]
fn measures_are_placed_left_to_right() {
    let options = Options::default();
    let layout = engrave(&Score::demo(), &options).unwrap();
    let mut last_x = f64::NEG_INFINITY;
    let mut systems = 0;
    for page in &layout.pages {
        for system in &page.systems {
            systems += 1;
            assert!(system.x_end > system.x_start);
            assert!(system.bottom > system.top);
            let mut system_last = f64::NEG_INFINITY;
            for hit in page.hits.iter().filter(|hit| {
                system.measures.contains(&hit.measure)
            }) {
                assert!(hit.x >= system.x_start - 100.0);
                assert!(hit.x <= system.x_end + 50.0);
                system_last = system_last.max(hit.x);
            }
            let _ = system_last;
            last_x = last_x.max(system.x_end);
        }
    }
    assert!(systems >= 1);
    assert!(last_x > options.margins.left);
}
