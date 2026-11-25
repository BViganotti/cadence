//! Pagination and forced-break acceptance tests.

use cadence_core::{Attributes, Duration, Part, Pitch, Score};
use cadence_engrave::pagination::{paginate, plan_systems, MeasurePlan};
use cadence_engrave::{engrave, Options};

fn plans(widths: &[f64]) -> Vec<MeasurePlan> {
    widths
        .iter()
        .enumerate()
        .map(|(index, width)| MeasurePlan {
            index,
            width: *width,
            system_break: false,
            page_break: false,
        })
        .collect()
}

#[test]
fn systems_never_exceed_content_width_when_measures_fit() {
    let systems = plan_systems(&plans(&[120.0; 10]), 50.0, 480.0, 12);
    for system in &systems {
        let width: f64 = system
            .measures
            .iter()
            .map(|index| 120.0)
            .sum::<f64>()
            + if system.measures.is_empty() { 0.0 } else { 50.0 };
        assert!(width <= 480.0 + 0.01, "{system:?} -> {width}");
    }
    assert_eq!(systems.iter().map(|system| system.measures.len()).sum::<usize>(), 10);
}

#[test]
fn max_measures_per_system_is_respected() {
    let systems = plan_systems(&plans(&[10.0; 20]), 0.0, 10_000.0, 3);
    assert!(systems.iter().all(|system| system.measures.len() <= 3));
    assert_eq!(systems.len(), 7);
}

#[test]
fn forced_breaks_and_page_flags_are_preserved() {
    let mut plans = plans(&[100.0; 6]);
    plans[2].system_break = true;
    plans[4].page_break = true;
    let systems = plan_systems(&plans, 20.0, 2_000.0, 12);
    assert_eq!(systems.len(), 3);
    assert_eq!(systems[0].measures, vec![0, 1]);
    assert_eq!(systems[1].measures, vec![2, 3]);
    assert_eq!(systems[2].measures, vec![4, 5]);
    assert!(systems[2].new_page);
}

#[test]
fn height_pagination_respects_gaps_and_forced_pages() {
    let pages = paginate(&[100.0, 100.0, 100.0, 100.0, 100.0], 20.0, 230.0, 230.0, &[false; 5]);
    assert_eq!(pages.len(), 3);
    assert_eq!(pages[0], vec![0, 1]);
    assert_eq!(pages[1], vec![2, 3]);
    assert_eq!(pages[2], vec![4]);
    let forced = paginate(&[10.0, 10.0], 0.0, 1000.0, 1000.0, &[false, true]);
    assert_eq!(forced.len(), 2);
    assert!(paginate(&[], 10.0, 100.0, 100.0, &[]).is_empty());
}

#[test]
fn zero_or_negative_heights_do_not_loop_or_panic() {
    let pages = paginate(&[0.0, -5.0, 0.0], 0.0, 1.0, 1.0, &[false; 3]);
    assert_eq!(pages.iter().map(|page| page.len()).sum::<usize>(), 3);
}

#[test]
fn layout_inserts_a_page_when_a_measure_requests_one() {
    let mut score = Score::new("Pages");
    let mut part = Part::new("P1", "Piano").with_measures(4, Attributes::default());
    for measure in &mut part.measures {
        measure.push_note(1, Pitch::parse("C4").unwrap(), Duration::whole());
    }
    part.measures[2].page_break = true;
    score.parts.push(part);
    let layout = engrave(&score, &Options::default()).unwrap();
    assert!(layout.page_count() >= 2);
    let second_page = &layout.pages[1];
    assert!(second_page.systems.iter().any(|system| system.measures.contains(&2)));
}

#[test]
fn layout_keeps_systems_inside_page_content_height() {
    let mut score = Score::new("Tall pages");
    let mut part = Part::new("P1", "Piano").with_measures(40, Attributes::default());
    for measure in &mut part.measures {
        measure.push_note(1, Pitch::parse("C4").unwrap(), Duration::whole());
    }
    part.measures[10].page_break = true;
    score.parts.push(part);
    let options = Options::default();
    let layout = engrave(&score, &options).unwrap();
    for page in &layout.pages {
        for system in &page.systems {
            assert!(system.bottom <= page.height - 1.0);
            assert!(system.top >= 0.0);
        }
    }
    assert!(layout.page_count() >= 2);
}
