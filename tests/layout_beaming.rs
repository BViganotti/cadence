//! Beaming acceptance tests.

use cadence_core::{
    Attributes, Beam, BeamKind, Duration, Measure, NoteValue, Pitch, Score, TimeSignature,
};
use cadence_engrave::beaming::{beam_groups, beam_levels};
use cadence_engrave::{engrave, Element, Options};

fn pitch(name: &str) -> Pitch {
    Pitch::parse(name).unwrap()
}

fn eighths(measure: &mut Measure, count: usize) {
    for index in 0..count {
        measure.push_note(
            1,
            pitch(if index % 2 == 0 { "C5" } else { "D5" }),
            Duration::eighth(),
        );
    }
}

#[test]
fn beam_levels_follow_written_duration() {
    assert_eq!(beam_levels(Duration::eighth()), 1);
    assert_eq!(beam_levels(Duration::sixteenth()), 2);
    assert_eq!(beam_levels(Duration::new(NoteValue::ThirtySecond, 0)), 3);
    assert_eq!(beam_levels(Duration::quarter()), 0);
    assert_eq!(beam_levels(Duration::whole()), 0);
}

#[test]
fn explicit_beams_override_automatic_grouping() {
    let attributes = Attributes {
        time: TimeSignature::new(4, 4),
        ..Attributes::default()
    };
    let mut measure = Measure::new(1, attributes);
    eighths(&mut measure, 8);
    let voice = measure.voice_mut(1);
    voice.events[0].beams = vec![Beam::new(1, BeamKind::Begin)];
    voice.events[1].beams = vec![Beam::new(1, BeamKind::End)];
    let groups = beam_groups(&measure, 1);
    assert!(groups.iter().any(|group| {
        group.start_event == 0 && group.end_event == 1
    }));
    assert!(groups.len() >= 2);
}

#[test]
fn six_eight_groups_into_two_threes() {
    let attributes = Attributes {
        time: TimeSignature::new(6, 8),
        ..Attributes::default()
    };
    let mut measure = Measure::new(1, attributes);
    eighths(&mut measure, 6);
    let groups = beam_groups(&measure, 1);
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].start_event, 0);
    assert_eq!(groups[0].end_event, 2);
    assert_eq!(groups[1].start_event, 3);
    assert_eq!(groups[1].end_event, 5);
}

#[test]
fn mixed_sixteenths_create_multiple_beam_levels() {
    let mut measure = Measure::new(1, Attributes::default());
    measure.push_note(1, pitch("C5"), Duration::eighth());
    measure.push_note(1, pitch("D5"), Duration::sixteenth());
    measure.push_note(1, pitch("E5"), Duration::sixteenth());
    let groups = beam_groups(&measure, 1);
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].start_event, 0);
    assert_eq!(groups[0].end_event, 2);
}

#[test]
fn layout_draws_beam_paths_and_is_deterministic() {
    let mut score = Score::new("Beams");
    let attributes = Attributes {
        time: TimeSignature::new(6, 8),
        ..Attributes::default()
    };
    let mut part = cadence_core::Part::new("P1", "Flute").with_measures(1, attributes);
    eighths(&mut part.measures[0], 6);
    score.parts.push(part);
    let a = engrave(&score, &Options::default()).unwrap();
    let b = engrave(&score, &Options::default()).unwrap();
    assert_eq!(a, b);
    let filled_paths = a.pages[0]
        .elements
        .iter()
        .filter(|element| matches!(element, Element::Path { fill: true, .. }))
        .count();
    assert!(filled_paths >= 2);
}

#[test]
fn rests_break_beam_groups() {
    let attributes = Attributes {
        time: TimeSignature::new(6, 8),
        ..Attributes::default()
    };
    let mut measure = Measure::new(1, attributes);
    measure.push_note(1, pitch("C5"), Duration::eighth());
    measure.push_note(1, pitch("D5"), Duration::eighth());
    measure.push_rest(1, Duration::eighth());
    measure.push_note(1, pitch("E5"), Duration::eighth());
    measure.push_note(1, pitch("F5"), Duration::eighth());
    let groups = beam_groups(&measure, 1);
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].end_event, 1);
    assert_eq!(groups[1].start_event, 3);
}

#[test]
fn trailing_compound_event_does_not_panic() {
    let mut measure = Measure::new(1, Attributes::default());
    measure.push_note(1, pitch("C5"), Duration::eighth());
    measure.push_rest(1, Duration::eighth());
    measure.push_note(1, pitch("D5"), Duration::eighth());
    assert!(beam_groups(&measure, 1).is_empty());
}

#[test]
fn empty_voice_and_missing_index_are_safe() {
    let measure = Measure::new(1, Attributes::default());
    assert!(beam_groups(&measure, 99).is_empty());
    assert!(beam_groups(&measure, 1).is_empty());
}
