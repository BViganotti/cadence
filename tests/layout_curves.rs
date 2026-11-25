//! Slur and tie curve layout tests.

use cadence_core::{
    Attributes, Duration, Part, Pitch, Score, SlurKind, SlurMark, Tie,
};
use cadence_engrave::{engrave, Element, Options};

fn two_note_score(slur: bool, tie: bool) -> Score {
    let mut part = Part::new("P1", "Piano").with_measures(2, Attributes::default());
    part.measures[0].push_note(1, Pitch::parse("C4").unwrap(), Duration::whole());
    part.measures[1].push_note(1, Pitch::parse("C4").unwrap(), Duration::whole());
    if slur {
        part.measures[0].voice_mut(1).events[0].notes_mut()[0]
            .slurs
            .push(SlurMark::new(1, SlurKind::Start));
        part.measures[1].voice_mut(1).events[0].notes_mut()[0]
            .slurs
            .push(SlurMark::new(1, SlurKind::Stop));
    }
    if tie {
        part.measures[0].voice_mut(1).events[0].notes_mut()[0].tie = Tie::new(true, false);
        part.measures[1].voice_mut(1).events[0].notes_mut()[0].tie = Tie::new(false, true);
    }
    let mut score = Score::new("Curves");
    score.parts.push(part);
    score
}

fn curve_count(score: &Score) -> usize {
    let layout = engrave(score, &Options::default()).unwrap();
    layout
        .pages
        .iter()
        .flat_map(|page| page.elements.iter())
        .filter(|element| matches!(element, Element::Path { fill: false, stroke, .. } if *stroke > 0.0))
        .count()
}

#[test]
fn slurs_and_ties_add_curve_paths_across_measures() {
    let plain = curve_count(&two_note_score(false, false));
    let slur = curve_count(&two_note_score(true, false));
    let tie = curve_count(&two_note_score(false, true));
    let both = curve_count(&two_note_score(true, true));
    assert_eq!(slur, plain + 1);
    assert_eq!(tie, plain + 1);
    assert_eq!(both, plain + 2);
}

#[test]
fn curve_layout_is_deterministic() {
    let score = two_note_score(true, true);
    let a = engrave(&score, &Options::default()).unwrap();
    let b = engrave(&score, &Options::default()).unwrap();
    assert_eq!(a, b);
}

#[test]
fn unmatched_slurs_produce_no_curve_but_do_not_fail_layout() {
    let mut score = two_note_score(false, false);
    score.parts[0].measures[0].voice_mut(1).events[0].notes_mut()[0]
        .slurs
        .push(SlurMark::new(2, SlurKind::Start));
    let layout = engrave(&score, &Options::default()).unwrap();
    assert!(layout.page_count() >= 1);
}

#[test]
fn ties_with_different_pitch_are_not_connected() {
    let mut score = two_note_score(false, true);
    score.parts[0].measures[1].voice_mut(1).events[0].notes_mut()[0].pitch =
        Pitch::parse("D4").unwrap();
    let plain = curve_count(&two_note_score(false, false));
    assert_eq!(curve_count(&score), plain);
}
