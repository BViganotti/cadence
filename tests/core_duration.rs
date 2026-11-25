//! Duration arithmetic acceptance tests.

use cadence_core::{Duration, NoteValue, TimeSignature};

#[test]
fn every_written_duration_has_a_stable_display_and_type() {
    for value in NoteValue::ALL {
        for dots in 0..=Duration::MAX_DOTS {
            let duration = Duration::new(value, dots);
            let text = duration.to_string();
            assert_eq!(text.matches('.').count(), dots as usize);
            assert!(text.starts_with(value.label()));
            assert_eq!(NoteValue::from_musicxml_type(value.musicxml_type()), Some(value));
        }
    }
}

#[test]
fn tick_conversion_is_monotonic() {
    let divisions = 480;
    let mut previous = 0;
    for value in NoteValue::ALL.iter().rev() {
        for dots in 0..=Duration::MAX_DOTS {
            let duration = Duration::new(*value, dots);
            let ticks = duration.ticks(divisions);
            assert!(ticks > previous, "{duration}");
            previous = ticks;
        }
    }
}

#[test]
fn dot_factors_are_exact_for_standard_divisions() {
    let divisions = 480;
    let mut exact_count = 0;
    for value in NoteValue::ALL {
        for dots in 0..=Duration::MAX_DOTS {
            let duration = Duration::new(value, dots);
            if let Some(exact) = duration.ticks_exact(divisions) {
                exact_count += 1;
                assert_eq!(duration.ticks(divisions), exact);
                assert_eq!(Duration::from_ticks(divisions, exact), Some(duration));
            }
        }
    }
    assert!(exact_count >= 24, "only {exact_count} exact durations");
}

#[test]
fn unrepresentable_tick_counts_are_rejected() {
    assert_eq!(Duration::from_ticks(480, 0), None);
    assert_eq!(Duration::from_ticks(480, 1), None);
    assert_eq!(Duration::from_ticks(0, 480), None);
    // A quarter plus a third of a quarter is not a supported written value.
    assert_eq!(Duration::from_ticks(480, 640), None);
}

#[test]
fn measure_capacity_matches_time_signature() {
    let cases = [
        (4, 4, 1920),
        (3, 4, 1440),
        (6, 8, 1440),
        (2, 2, 1920),
        (5, 4, 2400),
        (12, 8, 2880),
    ];
    for (beats, beat_type, expected) in cases {
        let time = TimeSignature::new(beats, beat_type);
        assert_eq!(time.measure_ticks(480), expected);
        assert!((time.beats_per_measure() - expected as f64 / 480.0).abs() < 1e-9);
    }
}

#[test]
fn beats_are_quarter_relative() {
    assert!((Duration::whole().beats() - 4.0).abs() < 1e-9);
    assert!((Duration::eighth().beats() - 0.5).abs() < 1e-9);
    assert!((Duration::new(NoteValue::Half, 2).beats() - 3.5).abs() < 1e-9);
}
