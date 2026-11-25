//! Key, time, clef, repeat and heading layout tests.

use cadence_core::{
    Attributes, BarlineStyle, Clef, Duration, Ending, EndingKind, Part, Pitch, Repeat, Score,
    TimeSignature,
};
use cadence_engrave::{engrave, Element, Options, SymbolKind};

fn single_part_score(name: &str) -> Score {
    let mut score = Score::new(name);
    let mut part = Part::new("P1", "Piano").with_measures(4, Attributes::default());
    for measure in &mut part.measures {
        measure.push_note(1, Pitch::parse("C4").unwrap(), Duration::whole());
    }
    score.parts.push(part);
    score
}

#[test]
fn key_time_and_clef_changes_start_new_systems() {
    let mut score = single_part_score("Changes");
    score.parts[0].measures[1].attributes.key =
        cadence_core::Key::new(2, cadence_core::KeyMode::Major);
    score.parts[0].measures[1].explicit_attributes = true;
    score.parts[0].measures[2].attributes.time = TimeSignature::new(3, 4);
    score.parts[0].measures[2].explicit_attributes = true;
    score.parts[0].measures[2].voices.clear();
    score.parts[0].measures[2].push_note(
        1,
        Pitch::parse("D4").unwrap(),
        Duration::new(cadence_core::NoteValue::Half, 1),
    );
    score.parts[0].measures[3].attributes.clef = Clef::bass();
    score.parts[0].measures[3].explicit_attributes = true;
    score.parts[0].measures[3].voices.clear();
    score.parts[0].measures[3].push_note(
        1,
        Pitch::parse("E4").unwrap(),
        Duration::new(cadence_core::NoteValue::Half, 1),
    );
    let layout = engrave(&score, &Options::default()).unwrap();
    let mut first_measures = Vec::new();
    for page in &layout.pages {
        for system in &page.systems {
            first_measures.push(*system.measures.first().unwrap());
        }
    }
    assert!(first_measures.contains(&1));
    assert!(first_measures.contains(&2));
    assert!(first_measures.contains(&3));
    let clefs = layout
        .symbols()
        .filter(|symbol| symbol.kind == SymbolKind::Clef)
        .count();
    assert!(clefs >= 4);
}

#[test]
fn title_rehearsal_and_page_number_are_emitted() {
    let mut score = single_part_score("Headings");
    score.metadata.composer = "Cadence".to_string();
    score.parts[0].measures[0].rehearsal = Some("A".to_string());
    score.parts[0].measures[0].tempo = Some(cadence_core::Tempo::new(120, cadence_core::NoteValue::Quarter));
    let layout = engrave(&score, &Options::default()).unwrap();
    assert!(layout
        .pages[0]
        .symbols
        .iter()
        .any(|symbol| symbol.kind == SymbolKind::Title));
    assert!(layout
        .symbols()
        .any(|symbol| symbol.kind == SymbolKind::Rehearsal));
    assert!(layout
        .symbols()
        .any(|symbol| symbol.kind == SymbolKind::Tempo));
    assert!(layout
        .symbols()
        .any(|symbol| symbol.kind == SymbolKind::Footer));
    assert!(layout.pages[0]
        .elements
        .iter()
        .any(|element| matches!(element, Element::Text { text, .. } if text == "Headings")));
}

#[test]
fn repeat_ending_and_final_barline_have_visible_ink() {
    let mut score = single_part_score("Repeats");
    score.parts[0].measures[1].repeat = Some(Repeat::backward());
    score.parts[0].measures[2].ending = Some(Ending {
        number: 1,
        kind: EndingKind::Start,
    });
    score.parts[0].measures[3].barline = BarlineStyle::Final;
    let layout = engrave(&score, &Options::default()).unwrap();
    let symbols = layout.pages[0].symbols.len();
    assert!(symbols > 0);
    let rects = layout.pages[0]
        .elements
        .iter()
        .filter(|element| matches!(element, Element::Rect { filled: true, .. }))
        .count();
    assert!(rects >= 1);
    assert!(layout
        .symbols()
        .any(|symbol| symbol.kind == SymbolKind::MeasureNumber));
}

#[test]
fn part_names_are_shown_only_when_enabled() {
    let score = single_part_score("Names");
    let with_names = engrave(&score, &Options::default()).unwrap();
    assert!(with_names
        .symbols()
        .any(|symbol| symbol.kind == SymbolKind::PartName));
    let mut options = Options::default();
    options.show_part_names = false;
    let without = engrave(&score, &options).unwrap();
    assert!(!without
        .symbols()
        .any(|symbol| symbol.kind == SymbolKind::PartName));
}

#[test]
fn measure_numbers_can_be_suppressed() {
    let score = single_part_score("Numbers");
    let mut options = Options::default();
    options.show_measure_numbers = false;
    let layout = engrave(&score, &options).unwrap();
    assert!(!layout
        .symbols()
        .any(|symbol| symbol.kind == SymbolKind::MeasureNumber));
}

#[test]
fn mid_score_key_change_is_rendered_at_a_system_start() {
    let score = Score::demo();
    let layout = engrave(&score, &Options::default()).unwrap();
    let system = layout
        .pages
        .iter()
        .flat_map(|page| page.systems.iter())
        .find(|system| system.measures.contains(&6))
        .unwrap();
    assert_eq!(system.measures.first(), Some(&6));
    assert!(system.staff_tops.len() == 2);
}
