//! MusicXML export acceptance tests.

use cadence_core::{Attributes, Duration, Event, Measure, Note, Part, Pitch, Score};
use cadence_import::{export_score, import_score};

fn sample() -> Score {
    let mut part = Part::new("P1", "Piano").with_measures(2, Attributes::default());
    part.abbreviation = "Pno.".to_string();
    part.midi_program = 4;
    part.midi_channel = 2;
    let mut chord = Event::note(Pitch::parse("C4").unwrap(), Duration::half());
    chord.add_note(Note::new(Pitch::parse("E4").unwrap()));
    chord.add_note(Note::new(Pitch::parse("G4").unwrap()));
    part.measures[0].push_event(1, chord);
    part.measures[1].push_rest(1, Duration::whole());
    let mut score = Score::new("Export Test");
    score.metadata.composer = "Composer".to_string();
    score.metadata.lyricist = "Lyricist".to_string();
    score.metadata.copyright = "Public domain".to_string();
    score.parts.push(part);
    score
}

#[test]
fn export_contains_expected_structure_and_metadata() {
    let xml = export_score(&sample()).unwrap();
    assert!(xml.starts_with("<?xml"));
    assert!(xml.contains("<score-partwise version=\"4.0\">"));
    assert!(xml.contains("<work-title>"));
    assert!(xml.contains("Export Test"));
    assert!(xml.contains("<creator type=\"composer\">"));
    assert!(xml.contains("Composer"));
    assert!(xml.contains("<part-name>"));
    assert!(xml.contains("Piano"));
    assert!(xml.contains("<part-abbreviation>"));
    assert!(xml.contains("Pno."));
    assert!(xml.contains("<midi-program>"));
    assert!(xml.contains("<midi-channel>"));
    assert!(xml.contains("<chord/>"));
    assert!(xml.contains("<rest measure=\"yes\"/>"));
    assert!(xml.contains("<bar-style>light-heavy</bar-style>") == false);
    assert!(!xml.contains("<!DOCTYPE"));
    assert!(!xml.contains("2026"));
}

#[test]
fn export_is_deterministic_for_identical_scores() {
    let score = sample();
    assert_eq!(export_score(&score).unwrap(), export_score(&score).unwrap());
    let mut clone = score.clone();
    clone.metadata.title = "Other".to_string();
    assert_ne!(export_score(&score).unwrap(), export_score(&clone).unwrap());
}

#[test]
fn export_import_round_trip_preserves_chord_and_part_metadata() {
    let original = sample();
    let xml = export_score(&original).unwrap();
    let again = import_score(&xml).unwrap();
    assert_eq!(again.metadata, original.metadata);
    assert_eq!(again.parts.len(), 1);
    assert_eq!(again.parts[0].abbreviation, "Pno.");
    assert_eq!(again.parts[0].midi_program, 4);
    assert_eq!(again.parts[0].midi_channel, 2);
    assert_eq!(again.parts[0].note_count(), original.parts[0].note_count());
    assert!(again.parts[0].measures[0].voices[0].events[0].is_chord());
}

#[test]
fn export_rejects_overfull_and_malformed_scores() {
    let mut score = sample();
    score.parts[0].measures[1].push_note(
        1,
        Pitch::parse("C5").unwrap(),
        Duration::quarter(),
    );
    assert!(export_score(&score).is_err());
    let mut score = sample();
    score.parts[0].id = String::new();
    assert!(export_score(&score).is_err());
}

#[test]
fn explicit_attribute_changes_are_emitted() {
    let mut score = sample();
    score.parts[0].measures[1].attributes.time =
        cadence_core::TimeSignature::new(3, 4);
    score.parts[0].measures[1].explicit_attributes = true;
    score.parts[0].measures[1].voices.clear();
    score.parts[0].measures[1].push_rest(1, Duration::new(cadence_core::NoteValue::Half, 1));
    let xml = export_score(&score).unwrap();
    assert!(xml.contains("<time>"));
    let again = import_score(&xml).unwrap();
    assert_eq!(again.parts[0].measures[1].attributes.time.beats, 3);
}

#[test]
fn measures_without_notes_are_exported_but_not_invented() {
    let mut score = sample();
    score.parts[0].measures.push(Measure::new(3, Attributes::default()));
    let xml = export_score(&score).unwrap();
    assert_eq!(xml.matches("<measure number=").count(), 3);
    let again = import_score(&xml).unwrap();
    assert_eq!(again.parts[0].measures.len(), 3);
}
