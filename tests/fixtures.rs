//! End-to-end fixture tests: every checked-in MusicXML fixture must import,
//! validate, export and round-trip without losing notes or notation marks.

use cadence_core::{Score, Severity};
use cadence_import::{export_score, import_score};
use std::fs;
use std::path::PathBuf;

fn fixture_paths() -> Vec<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/fixtures");
    let mut paths: Vec<PathBuf> = fs::read_dir(&dir)
        .expect("examples/fixtures exists")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("musicxml"))
        .collect();
    paths.sort();
    paths
}

#[test]
fn at_least_one_hundred_fixtures_are_checked_in() {
    let paths = fixture_paths();
    assert!(
        paths.len() >= 100,
        "expected at least 100 fixtures, found {}",
        paths.len()
    );
}

#[test]
fn all_fixtures_round_trip_without_lost_notes() {
    let paths = fixture_paths();
    assert!(!paths.is_empty());
    let mut total_notes = 0usize;
    for path in paths {
        let xml = fs::read_to_string(&path).expect("read fixture");
        let score = import_score(&xml)
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        score
            .validate()
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        let report = score.inspect();
        assert!(
            report.is_ok(),
            "{}: {:?}",
            path.display(),
            report.issues
        );
        assert!(
            !report
                .issues
                .iter()
                .any(|issue| issue.severity == Severity::Error),
            "{}",
            path.display()
        );
        assert!(score.note_count() > 0, "{} has no notes", path.display());
        total_notes += score.note_count();
        let first_export = export_score(&score)
            .unwrap_or_else(|error| panic!("{}: export: {error}", path.display()));
        let second_export = export_score(&score).unwrap();
        assert_eq!(first_export, second_export, "{} export not stable", path.display());
        let again = import_score(&first_export)
            .unwrap_or_else(|error| panic!("{}: reimport: {error}", path.display()));
        assert_same_notation(&score, &again, &path);
    }
    assert!(total_notes > 1000, "fixtures are too small: {total_notes} notes");
}

#[test]
fn every_fixture_has_stable_bytes() {
    for path in fixture_paths().into_iter().take(20) {
        let xml = fs::read_to_string(&path).unwrap();
        let score = import_score(&xml).unwrap();
        assert_eq!(export_score(&score).unwrap(), export_score(&score).unwrap());
    }
}

fn assert_same_notation(a: &Score, b: &Score, path: &PathBuf) {
    let context = path.display().to_string();
    assert_eq!(a.metadata.title, b.metadata.title, "{context}");
    assert_eq!(a.metadata.subtitle, b.metadata.subtitle, "{context}");
    assert_eq!(a.metadata.composer, b.metadata.composer, "{context}");
    assert_eq!(a.parts.len(), b.parts.len(), "{context}");
    for (part_a, part_b) in a.parts.iter().zip(&b.parts) {
        assert_eq!(part_a.id, part_b.id, "{context}");
        assert_eq!(part_a.name, part_b.name, "{context}");
        assert_eq!(part_a.measures.len(), part_b.measures.len(), "{context}");
        for (measure_a, measure_b) in part_a.measures.iter().zip(&part_b.measures) {
            assert_eq!(measure_a.attributes, measure_b.attributes, "{context}");
            assert_eq!(measure_a.voices.len(), measure_b.voices.len(), "{context}");
            for (voice_a, voice_b) in measure_a.voices.iter().zip(&measure_b.voices) {
                assert_eq!(voice_a.number, voice_b.number, "{context}");
                assert_eq!(
                    voice_a.events.len(),
                    voice_b.events.len(),
                    "{context} measure {} voice {}",
                    measure_a.number,
                    voice_a.number
                );
                for (event_a, event_b) in voice_a.events.iter().zip(&voice_b.events) {
                    assert_eq!(event_a.duration, event_b.duration, "{context}");
                    assert_eq!(event_a.dynamics, event_b.dynamics, "{context}");
                    assert_eq!(event_a.beams, event_b.beams, "{context}");
                    assert_eq!(event_a.notes().len(), event_b.notes().len(), "{context}");
                    for (note_a, note_b) in event_a.notes().iter().zip(event_b.notes()) {
                        assert_eq!(note_a.pitch, note_b.pitch, "{context}");
                        assert_eq!(note_a.tie, note_b.tie, "{context}");
                        assert_eq!(note_a.accidental, note_b.accidental, "{context}");
                        assert_eq!(
                            note_a.articulations, note_b.articulations,
                            "{context}"
                        );
                        assert_eq!(note_a.lyrics, note_b.lyrics, "{context}");
                        assert_eq!(note_a.slurs, note_b.slurs, "{context}");
                    }
                }
            }
        }
    }
}
