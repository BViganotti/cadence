//! Slur, tie, beam-run and lyric analysis tests.

use cadence_core::notation::{
    beam_runs, first_pitch, lyric_line_count, resolve_slurs, resolve_ties, SpanProblemKind,
};
use cadence_core::{
    Attributes, Duration, Event, Lyric, Note, Part, Pitch, SlurKind, SlurMark, Syllabic, Tie,
};

fn pitch(name: &str) -> Pitch {
    Pitch::parse(name).unwrap()
}

#[test]
fn nested_slurs_pair_by_number_and_position() {
    let mut part = Part::new("p", "Piano").with_measures(2, Attributes::default());
    let mut first = Event::note(pitch("C4"), Duration::half());
    first.notes_mut()[0]
        .slurs
        .push(SlurMark::new(1, SlurKind::Start));
    first.notes_mut()[0]
        .slurs
        .push(SlurMark::new(2, SlurKind::Start));
    part.measures[0].push_event(1, first);
    let mut middle = Event::note(pitch("D4"), Duration::quarter());
    middle.notes_mut()[0]
        .slurs
        .push(SlurMark::new(2, SlurKind::Stop));
    part.measures[1].push_event(1, middle);
    let mut last = Event::note(pitch("E4"), Duration::quarter());
    last.notes_mut()[0]
        .slurs
        .push(SlurMark::new(1, SlurKind::Stop));
    part.measures[1].push_event(1, last);
    let resolution = resolve_slurs(&part);
    assert_eq!(resolution.spans.len(), 2);
    assert!(resolution.problems.is_empty());
    assert_eq!(resolution.spans[0].number, 1);
    assert_eq!(resolution.spans[0].start.measure, 0);
    assert_eq!(resolution.spans[0].end.measure, 1);
}

#[test]
fn tie_chains_require_matching_pitch_and_report_problems() {
    let mut part = Part::new("p", "Piano").with_measures(2, Attributes::default());
    let mut first = Event::note(pitch("G4"), Duration::whole());
    first.notes_mut()[0].tie = Tie::new(true, false);
    part.measures[0].push_event(1, first);
    let mut wrong_pitch = Event::note(pitch("A4"), Duration::whole());
    wrong_pitch.notes_mut()[0].tie = Tie::new(false, true);
    part.measures[1].push_event(1, wrong_pitch);
    let resolution = resolve_ties(&part);
    assert!(resolution.spans.is_empty());
    assert_eq!(
        resolution.problems[0].kind,
        SpanProblemKind::StopWithoutStart
    );
    assert_eq!(
        resolution.problems[1].kind,
        SpanProblemKind::StartWithoutStop
    );
}

#[test]
fn beam_runs_split_at_quarters_and_rests() {
    let mut part = Part::new("p", "Piano").with_measures(1, Attributes::default());
    for name in ["C4", "D4"] {
        part.measures[0].push_note(1, pitch(name), Duration::eighth());
    }
    part.measures[0].push_note(1, pitch("E4"), Duration::quarter());
    for name in ["F4", "G4", "A4"] {
        part.measures[0].push_note(1, pitch(name), Duration::sixteenth());
    }
    part.measures[0].push_rest(1, Duration::quarter());
    let runs = beam_runs(&part, 0, 1);
    assert_eq!(runs.len(), 2);
    assert_eq!((runs[0].start_event, runs[0].end_event), (0, 1));
    assert_eq!((runs[1].start_event, runs[1].end_event), (3, 5));
}

#[test]
fn lyric_lines_and_first_pitch_are_reported() {
    let mut part = Part::new("p", "Piano").with_measures(1, Attributes::default());
    let mut event = Event::note(pitch("C4"), Duration::quarter());
    let mut chord_note = Note::new(pitch("E4"));
    chord_note.lyrics.push(Lyric {
        number: 3,
        text: "la".to_string(),
        syllabic: Syllabic::Single,
        extend: false,
    });
    event.add_note(chord_note);
    event.notes_mut()[0].lyrics.push(Lyric {
        number: 1,
        text: "do".to_string(),
        syllabic: Syllabic::Begin,
        extend: false,
    });
    part.measures[0].push_event(1, event);
    assert_eq!(lyric_line_count(&part), 3);
    assert_eq!(
        first_pitch(&part, 0, 1, 0).unwrap().name(),
        "C4"
    );
    assert!(first_pitch(&part, 0, 2, 0).is_none());
}

#[test]
fn continuing_slurs_do_not_create_or_close_spans() {
    let mut part = Part::new("p", "Piano").with_measures(1, Attributes::default());
    let mut event = Event::note(pitch("C4"), Duration::quarter());
    event.notes_mut()[0]
        .slurs
        .push(SlurMark::new(1, SlurKind::Continue));
    part.measures[0].push_event(1, event);
    let resolution = resolve_slurs(&part);
    assert!(resolution.spans.is_empty());
    assert!(resolution.problems.is_empty());
}
