//! Pitch and transposition acceptance tests.

use cadence_core::{Pitch, Score, Step};

#[test]
fn every_pitch_class_round_trips_through_scientific_notation() {
    for octave in -1..=8 {
        for step in Step::ALL {
            for alter in -2..=2 {
                let pitch = Pitch::new(step, alter, octave);
                if pitch.midi().is_none() {
                    continue;
                }
                let name = pitch.name();
                let parsed = Pitch::parse(&name).unwrap();
                assert_eq!(parsed, pitch, "{name}");
            }
        }
    }
}

#[test]
fn midi_spelling_covers_the_full_range() {
    for midi in 0..=127 {
        for flats in [false, true] {
            let pitch = Pitch::spell_midi(midi, flats);
            assert_eq!(pitch.midi(), Some(midi));
            assert_eq!(Pitch::parse(&pitch.name()).unwrap(), pitch);
        }
    }
}

#[test]
fn diatonic_steps_cross_octave_boundaries() {
    let b4 = Pitch::parse("B4").unwrap();
    assert_eq!(b4.step_by(1).name(), "C5");
    let c4 = Pitch::parse("C4").unwrap();
    assert_eq!(c4.step_by(-1).name(), "B3");
    assert_eq!(c4.step_by(7).name(), "C5");
    let f_sharp = Pitch::parse("F#4").unwrap();
    assert_eq!(f_sharp.step_by(2).name(), "A#4");
}

#[test]
fn transposition_is_atomic_when_a_note_would_leave_midi_range() {
    let mut score = Score::demo();
    let original = score.parts[0].clone();
    // The demo melody contains high notes; +127 must fail and leave the part
    // exactly as it was instead of partially transposing earlier measures.
    let error = score.transpose_part(0, 127).unwrap_err();
    assert!(error.to_string().contains("transposed") || error.to_string().contains("pitch"));
    assert_eq!(score.parts[0].measures, original.measures);
    assert_eq!(score.parts[0].name, original.name);
}

#[test]
fn transposing_by_zero_is_a_no_op() {
    let mut score = Score::demo();
    let before = score.clone();
    score.transpose_part(0, 0).unwrap();
    assert_eq!(score, before);
}
