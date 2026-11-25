//! Measure, part and score acceptance tests.

use cadence_core::{
    Attributes, BarlineStyle, Duration, Ending, EndingKind, Event, Measure, Note, Part, Pitch,
    Repeat, RepeatDirection, Score, Tie, TimeSignature,
};

fn pitch(name: &str) -> Pitch {
    Pitch::parse(name).unwrap()
}

#[test]
fn measure_voices_stay_sorted_and_capacity_is_enforced_by_inspection() {
    let mut measure = Measure::new(1, Attributes::default());
    measure.push_note(3, pitch("E4"), Duration::quarter());
    measure.push_note(1, pitch("C4"), Duration::quarter());
    measure.push_note(2, pitch("G4"), Duration::quarter());
    assert_eq!(
        measure.voices.iter().map(|voice| voice.number).collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    assert_eq!(measure.event_count(), 3);
    assert_eq!(measure.note_count(), 3);
    assert_eq!(measure.longest_voice_ticks(), 480);
    assert!(measure.capacity_ticks() > measure.longest_voice_ticks());
}

#[test]
fn measure_removal_and_validation_behave_soundly() {
    let mut measure = Measure::new(1, Attributes::default());
    measure.push_note(1, pitch("C4"), Duration::quarter());
    assert!(measure.validate().is_ok());
    assert!(measure.remove_event(1, 0).is_some());
    assert!(measure.remove_event(1, 0).is_none());
    assert!(measure.voices.is_empty());
    measure.repeat = Some(Repeat {
        direction: RepeatDirection::Backward,
        times: 1,
    });
    assert!(measure.validate().is_err());
    measure.repeat = Some(Repeat::backward());
    measure.ending = Some(Ending {
        number: 0,
        kind: EndingKind::Start,
    });
    assert!(measure.validate().is_err());
    measure.ending = Some(Ending {
        number: 1,
        kind: EndingKind::Stop,
    });
    assert!(measure.validate().is_ok());
    measure.barline = BarlineStyle::Final;
    assert_eq!(measure.barline.musicxml_name(), "light-heavy");
}

#[test]
fn pickup_measure_changes_the_part_timeline() {
    let mut part = Part::new("p", "Piano").with_measures(3, Attributes::default());
    part.measures[0].push_note(1, pitch("C4"), Duration::quarter());
    part.measures[1].push_note(1, pitch("D4"), Duration::whole());
    part.measures[2].push_note(1, pitch("E4"), Duration::whole());
    assert_eq!(part.measure_start_ticks(1), 480);
    assert_eq!(part.measure_start_ticks(2), 2400);
    assert_eq!(part.total_ticks(), 4320);
    let report = Score {
        metadata: Default::default(),
        parts: vec![part],
    }
    .inspect();
    assert!(report.is_ok(), "{:?}", report.issues);
}

#[test]
fn cross_measure_ties_are_inspected_as_one_chain() {
    let mut part = Part::new("p", "Piano").with_measures(2, Attributes::default());
    let mut event = Event::note(pitch("G4"), Duration::whole());
    event.notes_mut()[0].tie = Tie::new(true, false);
    part.measures[0].push_event(1, event);
    let mut event = Event::note(pitch("G4"), Duration::whole());
    event.notes_mut()[0].tie = Tie::new(false, true);
    part.measures[1].push_event(1, event);
    let score = Score {
        metadata: Default::default(),
        parts: vec![part],
    };
    let report = score.inspect();
    assert!(report.is_ok(), "{:?}", report.issues);
    assert!(!report.issues.iter().any(|issue| issue.message.contains("tie")));
}

#[test]
fn unmatched_ties_and_slurs_are_warnings_not_structural_errors() {
    let mut part = Part::new("p", "Piano").with_measures(1, Attributes::default());
    let mut event = Event::note(pitch("C4"), Duration::whole());
    let note = &mut event.notes_mut()[0];
    note.tie = Tie::new(false, true);
    note.slurs.push(cadence_core::SlurMark::new(
        2,
        cadence_core::SlurKind::Stop,
    ));
    part.measures[0].push_event(1, event);
    let score = Score {
        metadata: Default::default(),
        parts: vec![part],
    };
    assert!(score.validate().is_ok());
    let report = score.inspect();
    assert!(report.is_ok());
    assert!(report.warning_count() >= 2);
}

#[test]
fn flatten_notes_reports_absolute_starts_and_velocity() {
    let mut part = Part::new("p", "Piano").with_measures(2, Attributes::default());
    let mut event = Event::note(pitch("C4"), Duration::half());
    event.dynamics = Some(cadence_core::Dynamic::Ff);
    event.add_note(Note::new(pitch("E4")));
    part.measures[0].push_event(1, event);
    part.measures[0].push_rest(1, Duration::half());
    part.measures[1].push_note(1, pitch("G4"), Duration::whole());
    let score = Score {
        metadata: Default::default(),
        parts: vec![part],
    };
    let notes = score.flatten_notes();
    assert_eq!(notes.len(), 3);
    assert_eq!(notes[0].start_ticks, 0);
    assert_eq!(notes[1].start_ticks, 0);
    assert_eq!(notes[2].start_ticks, 1920);
    assert!(notes[0..2].iter().all(|note| note.velocity == cadence_core::Dynamic::Ff.velocity()));
    assert_eq!(notes[2].velocity, 80);
}

#[test]
fn overfull_and_underfull_measures_are_reported_without_panics() {
    let mut part = Part::new("p", "Piano").with_measures(2, Attributes::default());
    part.measures[0].push_note(1, pitch("C4"), Duration::whole());
    part.measures[0].push_note(1, pitch("D4"), Duration::quarter());
    part.measures[1].push_rest(1, Duration::quarter());
    let score = Score {
        metadata: Default::default(),
        parts: vec![part],
    };
    let report = score.inspect();
    assert!(report.error_count() >= 1);
    assert!(report.warning_count() >= 1);
    assert!(!report.summary().is_empty());
}

#[test]
fn changing_time_signature_changes_measure_capacity() {
    let mut attributes = Attributes::default();
    attributes.time = TimeSignature::new(6, 8);
    let measure = Measure::new(1, attributes);
    assert_eq!(measure.capacity_ticks(), 1440);
    assert_eq!(measure.attributes.time.beats, 6);
}
