//! Key, clef and time-signature acceptance tests.

use cadence_core::{Accidental, Attributes, Clef, ClefSign, Key, KeyMode, Pitch, Step, TimeSignature};

#[test]
fn all_major_and_minor_keys_describe_themselves() {
    for fifths in -7..=7 {
        for mode in [KeyMode::Major, KeyMode::Minor] {
            let key = Key::new(fifths, mode);
            assert!(key.validate().is_ok());
            let description = key.describe();
            assert!(description.contains(if mode == KeyMode::Major {
                "major"
            } else {
                "minor"
            }));
            assert_eq!(
                key.accidentals().len(),
                fifths.unsigned_abs() as usize
            );
        }
    }
}

#[test]
fn key_signature_order_and_alterations_match_the_circle_of_fifths() {
    let sharps = Key::new(7, KeyMode::Major).accidentals();
    let sharp_steps: Vec<Step> = sharps.iter().map(|(step, _)| *step).collect();
    assert_eq!(
        sharp_steps,
        vec![Step::F, Step::C, Step::G, Step::D, Step::A, Step::E, Step::B]
    );
    assert!(sharps
        .iter()
        .all(|(_, accidental)| *accidental == Accidental::Sharp));
    let flats = Key::new(-7, KeyMode::Major).accidentals();
    let flat_steps: Vec<Step> = flats.iter().map(|(step, _)| *step).collect();
    assert_eq!(
        flat_steps,
        vec![Step::B, Step::E, Step::A, Step::D, Step::G, Step::C, Step::F]
    );
    assert!(flats
        .iter()
        .all(|(_, accidental)| *accidental == Accidental::Flat));
    for step in Step::ALL {
        assert_eq!(Key::new(7, KeyMode::Major).alter_for(step), 1);
        assert_eq!(Key::new(-7, KeyMode::Major).alter_for(step), -1);
        assert_eq!(Key::new(0, KeyMode::Major).alter_for(step), 0);
    }
}

#[test]
fn clefs_map_pitches_to_expected_staff_steps() {
    let treble = Clef::treble();
    assert_eq!(treble.staff_step(Pitch::parse("E4").unwrap().diatonic_index()), 0);
    assert_eq!(treble.staff_step(Pitch::parse("F5").unwrap().diatonic_index()), 8);
    assert_eq!(treble.staff_step(Pitch::parse("C4").unwrap().diatonic_index()), -2);
    let bass = Clef::bass();
    assert_eq!(bass.staff_step(Pitch::parse("G2").unwrap().diatonic_index()), 0);
    assert_eq!(bass.staff_step(Pitch::parse("F3").unwrap().diatonic_index()), 6);
    let alto = Clef::alto();
    assert_eq!(alto.staff_step(Pitch::parse("F3").unwrap().diatonic_index()), 0);
    assert_eq!(alto.staff_step(Pitch::parse("C4").unwrap().diatonic_index()), 4);
}

#[test]
fn clef_validation_rejects_impossible_lines_and_octaves() {
    assert!(Clef::new(ClefSign::G, 0, 0).validate().is_err());
    assert!(Clef::new(ClefSign::C, 6, 0).validate().is_err());
    assert!(Clef::new(ClefSign::F, 4, 3).validate().is_err());
    assert!(Clef::new(ClefSign::F, 4, 0).validate().is_ok());
}

#[test]
fn beam_groups_sum_to_the_measure_beats() {
    for (beats, beat_type) in [
        (4, 4),
        (3, 4),
        (2, 4),
        (5, 4),
        (6, 8),
        (9, 8),
        (12, 8),
        (2, 2),
    ] {
        let time = TimeSignature::new(beats, beat_type);
        assert_eq!(time.beam_groups().iter().sum::<u32>(), beats);
        assert!(time.validate().is_ok());
    }
    assert!(TimeSignature::new(6, 8).is_compound());
    assert!(!TimeSignature::new(3, 4).is_compound());
}

#[test]
fn attributes_validation_covers_every_field() {
    let mut attributes = Attributes::default();
    assert!(attributes.validate().is_ok());
    attributes.divisions = 0;
    assert!(attributes.validate().is_err());
    attributes.divisions = 480;
    attributes.key = Key::new(8, KeyMode::Major);
    assert!(attributes.validate().is_err());
    attributes.key = Key::new(0, KeyMode::Major);
    attributes.time = TimeSignature::new(0, 4);
    assert!(attributes.validate().is_err());
    attributes.time = TimeSignature::new(4, 4);
    attributes.clef = Clef::new(ClefSign::G, 9, 0);
    assert!(attributes.validate().is_err());
}
