//! Accidental spelling and placement tests.

use cadence_core::{Attributes, Duration, Key, KeyMode, Part, Pitch, Score};
use cadence_engrave::{engrave, Options, SymbolKind};

fn score_with(names: &[&str], key_fifths: i8) -> Score {
    let mut attributes = Attributes::default();
    attributes.key = Key::new(key_fifths, KeyMode::Major);
    let mut part = Part::new("P1", "Piano").with_measures(1, attributes);
    for name in names {
        part.measures[0].push_note(1, Pitch::parse(name).unwrap(), Duration::quarter());
    }
    let mut score = Score::new("Accidentals");
    score.parts.push(part);
    score
}

fn accidental_count(layout: &cadence_engrave::Layout) -> usize {
    layout
        .symbols()
        .filter(|symbol| symbol.kind == SymbolKind::Accidental)
        .count()
}

#[test]
fn accidental_state_resets_pitch_and_cancels_with_naturals() {
    let layout = engrave(
        &score_with(&["C#4", "C4", "C#4", "F#4"], 0),
        &Options::default(),
    )
    .unwrap();
    // C# (sharp), C (natural), C# (sharp again), F# (sharp).
    assert_eq!(accidental_count(&layout), 4);
}

#[test]
fn key_signature_suppresses_expected_accidentals() {
    let layout = engrave(
        &score_with(&["F#4", "F4", "C#4"], 1),
        &Options::default(),
    )
    .unwrap();
    // F# is in G major, F natural cancels it, C# is outside the key.
    assert_eq!(accidental_count(&layout), 2);
}

#[test]
fn accidentals_reset_at_measure_boundaries() {
    let mut score = score_with(&["C#4"], 0);
    score.parts[0].measures.push(cadence_core::Measure::new(
        2,
        score.parts[0].measures[0].attributes,
    ));
    score.parts[0].measures[1].push_note(
        1,
        Pitch::parse("C#4").unwrap(),
        Duration::whole(),
    );
    let layout = engrave(&score, &Options::default()).unwrap();
    assert_eq!(accidental_count(&layout), 2);
}

#[test]
fn accidental_boxes_do_not_overlap_noteheads() {
    let layout = engrave(
        &score_with(&["C#4", "D#4", "E#4", "F#4"], 0),
        &Options::default(),
    )
    .unwrap();
    let accidentals: Vec<_> = layout
        .symbols()
        .filter(|symbol| symbol.kind == SymbolKind::Accidental)
        .map(|symbol| symbol.bbox)
        .collect();
    let noteheads: Vec<_> = layout
        .symbols()
        .filter(|symbol| symbol.kind == SymbolKind::Notehead)
        .map(|symbol| symbol.bbox)
        .collect();
    assert_eq!(accidentals.len(), 4);
    for accidental in &accidentals {
        for notehead in &noteheads {
            assert!(!accidental.intersects(*notehead));
        }
    }
}

#[test]
fn explicit_accidentals_are_honoured_even_when_key_matches() {
    let mut score = score_with(&["F#4"], 1);
    score.parts[0].measures[0].voice_mut(1).events[0].notes_mut()[0].accidental =
        Some(cadence_core::Accidental::Sharp);
    let layout = engrave(&score, &Options::default()).unwrap();
    assert_eq!(accidental_count(&layout), 1);
}
