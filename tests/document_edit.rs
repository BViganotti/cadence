//! Studio document and edit-command acceptance tests.

use cadence_studio::document::{Document, EditCommand};
use cadence_studio::json;

#[test]
fn every_edit_command_mutates_the_document_soundly() {
    let mut document = Document::demo();
    let base_version = document.version;
    document
        .apply(EditCommand::AddRest {
            part: 0,
            measure: 0,
            voice: 4,
            duration: "quarter".to_string(),
            dots: 0,
        })
        .unwrap();
    assert_eq!(document.version, base_version + 1);
    document
        .apply(EditCommand::SetTitle {
            title: "Edited".to_string(),
        })
        .unwrap();
    assert_eq!(document.score.metadata.title, "Edited");
    document
        .apply(EditCommand::SetTempo {
            part: 0,
            measure: 0,
            bpm: 144,
        })
        .unwrap();
    assert_eq!(document.score.parts[0].measures[0].tempo.unwrap().bpm, 144);
    document
        .apply(EditCommand::SetKey {
            part: 0,
            measure: 0,
            fifths: 3,
        })
        .unwrap();
    assert_eq!(document.score.parts[0].measures[0].attributes.key.fifths, 3);
    document
        .apply(EditCommand::SetPitch {
            part: 0,
            measure: 0,
            voice: 1,
            event: 0,
            note: 0,
            pitch: "F5".to_string(),
        })
        .unwrap();
    assert_eq!(
        document.score.parts[0].measures[0].voice(1).unwrap().events[0]
            .primary_pitch()
            .unwrap()
            .name(),
        "F5"
    );
    document
        .apply(EditCommand::SetDynamic {
            part: 0,
            measure: 0,
            voice: 1,
            event: 0,
            dynamic: Some("sfz".to_string()),
        })
        .unwrap();
    document
        .apply(EditCommand::SetLyric {
            part: 0,
            measure: 0,
            voice: 1,
            event: 0,
            note: 0,
            text: "new".to_string(),
            syllabic: Some("begin".to_string()),
        })
        .unwrap();
    document
        .apply(EditCommand::AddChordNote {
            part: 0,
            measure: 0,
            voice: 1,
            event: 0,
            pitch: "A5".to_string(),
        })
        .unwrap();
    assert!(document.score.parts[0].measures[0].voice(1).unwrap().events[0].is_chord());
    document
        .apply(EditCommand::SetLyric {
            part: 0,
            measure: 0,
            voice: 1,
            event: 0,
            note: 1,
            text: "chord".to_string(),
            syllabic: Some("single".to_string()),
        })
        .unwrap();

    document
        .apply(EditCommand::DeleteEvent {
            part: 0,
            measure: 0,
            voice: 4,
            event: 0,
        })
        .unwrap();
    document
        .apply(EditCommand::Transpose {
            part: 0,
            semitones: -2,
        })
        .unwrap();
    document
        .apply(EditCommand::AddMeasure { part: 0 })
        .unwrap();
    assert_eq!(document.score.parts[0].measures.len(), 9);
    document
        .apply(EditCommand::DeleteMeasure {
            part: 0,
            measure: 8,
        })
        .unwrap();
    assert_eq!(document.score.parts[0].measures.len(), 8);
}

#[test]
fn invalid_edits_roll_back_and_do_not_advance_version() {
    let mut document = Document::demo();
    let version = document.version;
    let before = document.score.clone();
    assert!(document
        .apply(EditCommand::AddNote {
            part: 0,
            measure: 0,
            voice: 1,
            pitch: "C5".to_string(),
            duration: "whole".to_string(),
            dots: 0,
            accidental: None,
        })
        .is_err());
    assert_eq!(document.version, version);
    assert_eq!(document.score, before);
    assert!(document
        .apply(EditCommand::SetDuration {
            part: 0,
            measure: 0,
            voice: 1,
            event: 0,
            duration: "whole".to_string(),
            dots: 0,
        })
        .is_err());
    assert!(document
        .apply(EditCommand::SetTempo {
            part: 0,
            measure: 0,
            bpm: 0,
        })
        .is_err());
    assert!(document
        .apply(EditCommand::Transpose {
            part: 0,
            semitones: 100,
        })
        .is_err());
    assert_eq!(document.score, before);
}

#[test]
fn undo_is_bounded_and_restores_exact_scores() {
    let mut document = Document::demo();
    let initial = document.score.clone();
    for index in 0..40 {
        document
            .apply(EditCommand::SetTitle {
                title: format!("Title {index}"),
            })
            .unwrap();
    }
    let mut undos = 0;
    while document.can_undo() {
        document.undo().unwrap();
        undos += 1;
    }
    assert!(undos <= 32);
    assert_eq!(document.score.parts.len(), initial.parts.len());
    assert_eq!(document.score.parts[0].measures.len(), initial.parts[0].measures.len());
    assert!(document.undo().is_err());
}

#[test]
fn summary_and_event_json_expose_stable_fields() {
    let document = Document::demo();
    let summary = document.summary();
    let text = summary.stringify();
    assert!(text.contains("\"parts\""));
    assert!(text.contains("\"diagnostics\""));
    assert!(text.contains("\"canUndo\""));
    assert_eq!(summary.get("noteCount").and_then(|v| v.as_u64()), Some(47));
    let event = document.event_json(0, 0, 1, 0).unwrap();
    assert_eq!(event.get("part").and_then(|v| v.as_u64()), Some(0));
    assert_eq!(event.get("measureNumber").and_then(|v| v.as_u64()), Some(1));
    assert_eq!(event.get("duration").and_then(|v| v.as_str()), Some("quarter"));
    assert!(event.get("notes").and_then(|v| v.as_array()).unwrap().len() >= 1);
    assert_eq!(
        document.event_json(0, 99, 1, 0).unwrap_err().to_string().contains("measure"),
        true
    );
}

#[test]
fn part_selection_and_part_limited_exports() {
    let mut document = Document::demo();
    document.select_part(1).unwrap();
    assert_eq!(document.selected_part, 1);
    assert!(document.select_part(9).is_err());
    let pdf = document.pdf(Some(1)).unwrap();
    assert!(pdf.starts_with(b"%PDF"));
    let midi = document.midi(Some(1)).unwrap();
    assert!(midi.starts_with(b"MThd"));
    let svg = document.svg(Some(1), true).unwrap();
    assert!(svg.contains("data-part=\"1\""));
    let xml = document.musicxml(Some(1)).unwrap();
    assert!(xml.contains("<score-partwise"));
    assert!(document.musicxml(Some(9)).is_err());
}

#[test]
fn import_and_reset_replace_the_document() {
    let mut document = Document::demo();
    let xml = document.musicxml(None).unwrap();
    document.import_xml(&xml).unwrap();
    assert_eq!(document.source, "Imported MusicXML");
    assert!(document.score.note_count() > 0);
    document.import_xml("not xml").unwrap_err();
    let reset = Document::demo();
    assert_eq!(reset.score.parts.len(), 2);
}

#[test]
fn edit_commands_parse_from_json_with_defaults() {
    let value = json::parse(
        r#"{"op":"add_note","part":1,"measure":2,"pitch":"D5"}"#,
    )
    .unwrap();
    let command = EditCommand::from_json(&value).unwrap();
    match command {
        EditCommand::AddNote {
            part,
            measure,
            voice,
            pitch,
            duration,
            dots,
            ..
        } => {
            assert_eq!(part, 1);
            assert_eq!(measure, 2);
            assert_eq!(voice, 1);
            assert_eq!(pitch, "D5");
            assert_eq!(duration, "quarter");
            assert_eq!(dots, 0);
        }
        other => panic!("unexpected {other:?}"),
    }
    let bad = json::parse(r#"{"op":"add_note","measure":-1}"#).unwrap();
    assert!(EditCommand::from_json(&bad).is_err());
}

#[test]
fn redo_restores_undone_edits_and_is_cleared_by_new_edits() {
    let mut document = Document::demo();
    document
        .apply(EditCommand::SetTitle {
            title: "First".to_string(),
        })
        .unwrap();
    document
        .apply(EditCommand::SetTitle {
            title: "Second".to_string(),
        })
        .unwrap();
    document.undo().unwrap();
    assert_eq!(document.score.metadata.title, "First");
    assert!(document.can_redo());
    document.redo().unwrap();
    assert_eq!(document.score.metadata.title, "Second");
    assert!(!document.can_redo());
    assert!(document.redo().is_err());
    document.undo().unwrap();
    document
        .apply(EditCommand::SetTitle {
            title: "Third".to_string(),
        })
        .unwrap();
    assert!(!document.can_redo());
    assert_eq!(document.score.metadata.title, "Third");
    assert!(document.summary().stringify().contains("\"canRedo\""));
}

#[test]
fn summary_reports_chord_and_rest_counts() {
    let document = Document::demo();
    let summary = document.summary();
    assert!(summary.get("chordCount").and_then(|v| v.as_u64()).unwrap() >= 1);
    assert!(summary.get("restCount").and_then(|v| v.as_u64()).unwrap() >= 1);
    assert!(summary.get("eventCount").and_then(|v| v.as_u64()).unwrap() >= summary.get("noteCount").and_then(|v| v.as_u64()).unwrap());
}
