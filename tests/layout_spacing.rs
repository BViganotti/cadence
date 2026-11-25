//! Layout spacing, system coverage and hit-region acceptance tests.

use cadence_core::{Attributes, Duration, Measure, Pitch, Score, TimeSignature};
use cadence_engrave::spacing::{prefix_width, space_measure};
use cadence_engrave::{engrave, Options};

fn pitch(name: &str) -> Pitch {
    Pitch::parse(name).unwrap()
}

#[test]
fn spacing_columns_are_shared_and_monotonic() {
    let options = Options::default();
    let mut measure = Measure::new(1, Attributes::default());
    measure.push_note(1, pitch("C4"), Duration::quarter());
    measure.push_note(1, pitch("D4"), Duration::quarter());
    measure.push_rest(1, Duration::half());
    measure.push_note(2, pitch("E4"), Duration::eighth());
    measure.push_note(2, pitch("F4"), Duration::eighth());
    measure.push_rest(2, Duration::half());
    let spacing = space_measure(&measure, &options);
    assert_eq!(spacing.columns.first().unwrap().start_tick, 0);
    assert_eq!(spacing.columns.last().unwrap().end_tick, 1920);
    let mut previous = f64::NEG_INFINITY;
    for column in &spacing.columns {
        assert!(column.x > previous);
        assert!(column.width > 0.0);
        previous = column.x;
    }
    assert_eq!(spacing.x_for_tick(0), 0.0);
    assert!(spacing.x_for_tick(480) > 0.0);
    assert!(spacing.width > 52.0);
}

#[test]
fn longer_time_signatures_get_more_measure_space() {
    let options = Options::default();
    let mut four = Measure::new(1, Attributes::default());
    four.push_rest(1, Duration::whole());
    let mut twelve = Measure::new(
        1,
        Attributes {
            time: TimeSignature::new(12, 8),
            ..Attributes::default()
        },
    );
    twelve.push_rest(1, Duration::new(cadence_core::NoteValue::Whole, 1));
    let four_width = space_measure(&four, &options).width;
    let twelve_width = space_measure(&twelve, &options).width;
    assert!(twelve_width > four_width);
}

#[test]
fn prefix_width_accounts_for_key_accidentals() {
    let options = Options::default();
    let plain = prefix_width(Attributes::default(), &options);
    let mut keyed = Attributes::default();
    keyed.key = cadence_core::Key::new(-7, cadence_core::KeyMode::Major);
    assert!(prefix_width(keyed, &options) > plain);
}

#[test]
fn full_layout_hits_cover_every_event() {
    let score = Score::demo();
    let layout = engrave(&score, &Options::default()).unwrap();
    assert_eq!(layout.hits().count(), score.event_count());
    for hit in layout.hits() {
        assert!(hit.part < score.parts.len());
        assert!(hit.measure < score.parts[hit.part].measures.len());
        assert!(hit.w > 0.0 && hit.h > 0.0);
    }
}

#[test]
fn systems_partition_measures_and_include_every_staff() {
    let score = Score::demo();
    let layout = engrave(&score, &Options::default()).unwrap();
    let mut seen = vec![0usize; score.measure_count()];
    for page in &layout.pages {
        for system in &page.systems {
            assert_eq!(system.staff_tops.len(), score.parts.len());
            assert!(system.x_end > system.x_start);
            assert!(system.bottom > system.top);
            for measure in &system.measures {
                seen[*measure] += 1;
            }
        }
    }
    assert!(seen.iter().all(|count| *count == 1), "{seen:?}");
}

#[test]
fn element_geometry_is_bounded_and_finite() {
    let layout = engrave(&Score::demo(), &Options::default()).unwrap();
    for page in &layout.pages {
        for element in &page.elements {
            match element {
                cadence_engrave::Element::Line { x1, y1, x2, y2, width } => {
                    assert!(x1.is_finite() && y1.is_finite());
                    assert!(x2.is_finite() && y2.is_finite());
                    assert!(*width >= 0.0);
                }
                cadence_engrave::Element::Text { size, .. } => {
                    assert!(*size > 0.0 && size.is_finite());
                }
                _ => {}
            }
        }
    }
}

#[test]
fn options_reject_degenerate_pages() {
    let mut options = Options::default();
    options.page_width = 10.0;
    assert!(options.validate().is_err());
    options = Options::default();
    options.system_gap = -1.0;
    assert!(options.validate().is_ok()); // negative gaps are finite and clamped by usage
    options.system_gap = f64::INFINITY;
    assert!(options.validate().is_err());
}

#[test]
fn a_two_part_score_aligns_measures_across_staves() {
    let mut score = Score::demo();
    let layout = engrave(&score, &Options::default()).unwrap();
    for page in &layout.pages {
        for system in &page.systems {
            assert_eq!(system.staff_tops.len(), 2);
            let distance = system.staff_tops[1] - system.staff_tops[0];
            assert!(distance > 40.0);
        }
    }
    score.parts.truncate(1);
    let layout = engrave(&score, &Options::default()).unwrap();
    assert!(layout.pages.iter().all(|page| page
        .systems
        .iter()
        .all(|system| system.staff_tops.len() == 1)));
}
