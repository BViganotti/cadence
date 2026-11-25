//! Score and model hard-limit tests.

use cadence_core::{
    Attributes, Duration, Event, Lyric, Note, Part, Pitch, Score, Tie,
};

fn pitch(name: &str) -> Pitch {
    Pitch::parse(name).unwrap()
}

#[test]
fn too_many_parts_is_rejected() {
    let mut score = Score::new("Many parts");
    for index in 0..=Score::MAX_PARTS {
        score.parts.push(Part::new(format!("P{index}"), format!("Part {index}")));
    }
    assert!(score.validate().is_err());
}

#[test]
fn chord_and_lyric_limits_are_enforced() {
    let mut event = Event::note(pitch("C4"), Duration::quarter());
    for index in 0..24 {
        let step = cadence_core::Step::ALL[index % 7];
        let octave = 3 + (index / 7) as i8;
        event.add_note(Note::new(Pitch::new(step, 0, octave)));
    }
    assert!(event.validate().is_ok());
    for index in 0..10 {
        event.notes_mut()[0].lyrics.push(Lyric::new(format!("line {index}")));
    }
    assert!(event.validate().is_err());
    event.notes_mut()[0].lyrics.truncate(8);
    assert!(event.validate().is_ok());
    event.notes_mut()[0].lyrics[0] = Lyric::new("x".repeat(513));
    assert!(event.validate().is_err());
}

#[test]
fn invalid_velocity_and_underfull_events_are_rejected() {
    let mut event = Event::note(pitch("C4"), Duration::quarter());
    event.notes_mut()[0].velocity = 0;
    assert!(event.validate().is_err());
    event.notes_mut()[0].velocity = 128;
    assert!(event.validate().is_err());
    event.notes_mut()[0].velocity = 1;
    assert!(event.validate().is_ok());
    let empty = Event {
        kind: cadence_core::EventKind::Pitched(Vec::new()),
        duration: Duration::quarter(),
        beams: Vec::new(),
        dynamics: None,
    };
    assert!(empty.validate().is_err());
}

#[test]
fn tie_flags_are_independent_and_default_silent() {
    let note = Note::new(pitch("C4"));
    assert_eq!(note.tie, Tie::NONE);
    assert!(!note.tie.is_tied());
    let mut note = Note::new(pitch("C4"));
    note.tie = Tie::new(true, true);
    assert!(note.tie.is_tied());
    assert_eq!(note.effective_velocity(), 80);
}

#[test]
fn part_and_measure_limits_are_bounded() {
    let mut part = Part::new("P1", "Piano");
    assert!(part.validate().is_ok());
    part.id = "x".repeat(65);
    assert!(part.validate().is_err());
    part.id = "P1".to_string();
    part.name = "n".repeat(257);
    assert!(part.validate().is_err());
    part.name = "Piano".to_string();
    part.midi_channel = 16;
    assert!(part.validate().is_err());
    part.midi_channel = 0;
    assert!(part.validate().is_ok());
}

#[test]
fn measure_voice_limit_is_enforced() {
    let mut measure = cadence_core::Measure::new(1, Attributes::default());
    for voice in 1..=16 {
        measure.push_rest(voice, Duration::quarter());
    }
    assert!(measure.validate().is_ok());
    measure.push_rest(17, Duration::quarter());
    assert!(measure.validate().is_err());
}

#[test]
fn score_total_duration_uses_the_longest_part() {
    let mut score = Score::new("Lengths");
    let mut short = Part::new("P1", "Short").with_measures(1, Attributes::default());
    short.measures[0].push_rest(1, Duration::whole());
    let mut long = Part::new("P2", "Long").with_measures(2, Attributes::default());
    long.measures[0].push_rest(1, Duration::whole());
    long.measures[1].push_rest(1, Duration::whole());
    score.parts.push(short);
    score.parts.push(long);
    assert_eq!(score.total_ticks(), 3840);
    assert_eq!(score.measure_count(), 2);
    assert!(score.inspect().warning_count() >= 1);
}
