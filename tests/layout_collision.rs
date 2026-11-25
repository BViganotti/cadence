//! Collision-checker and dense-notation acceptance tests.

use cadence_core::{
    Accidental, Attributes, Duration, Event, Lyric, Note, Part, Pitch, Score,
};
use cadence_engrave::{
    engrave, find_collisions, Bbox, Layout, Options, PageLayout, Symbol, SymbolKind,
};

fn empty_layout(page_count: usize) -> Layout {
    let options = Options::default();
    let pages = (1..=page_count)
        .map(|number| PageLayout {
            number,
            width: options.page_width,
            height: options.page_height,
            elements: Vec::new(),
            symbols: Vec::new(),
            hits: Vec::new(),
            systems: Vec::new(),
        })
        .collect();
    Layout {
        metadata: Default::default(),
        options,
        pages,
    }
}

fn symbol(kind: SymbolKind, page: usize, bbox: Bbox) -> Symbol {
    Symbol::new(kind, page, 0, 0, 0, 1, 0, bbox)
}

#[test]
fn checker_detects_overlap_and_respects_pages_and_connectors() {
    let mut layout = empty_layout(2);
    layout.pages[0].symbols.push(symbol(
        SymbolKind::Notehead,
        0,
        Bbox::new(10.0, 10.0, 10.0, 8.0),
    ));
    layout.pages[0].symbols.push(symbol(
        SymbolKind::Accidental,
        0,
        Bbox::new(14.0, 9.0, 5.0, 10.0),
    ));
    assert_eq!(find_collisions(&layout).len(), 1);
    layout.pages[0].symbols.clear();
    layout.pages[0].symbols.push(symbol(
        SymbolKind::Notehead,
        0,
        Bbox::new(10.0, 10.0, 10.0, 8.0),
    ));
    layout.pages[0].symbols.push(symbol(
        SymbolKind::Notehead,
        1,
        Bbox::new(10.0, 10.0, 10.0, 8.0),
    ));
    assert!(find_collisions(&layout).is_empty());
    layout.pages[0].symbols.push(symbol(
        SymbolKind::Barline,
        0,
        Bbox::new(10.0, 10.0, 10.0, 8.0),
    ));
    assert!(find_collisions(&layout).is_empty());
}

#[test]
fn dense_chords_accidentals_and_two_voices_do_not_collide() {
    let mut part = Part::new("P1", "Dense").with_measures(2, Attributes::default());
    for measure_index in 0..2 {
        for index in 0..8 {
            let name = ["C#5", "D#5", "F#5", "G#5", "A#5", "B5", "C#6", "D#6"][index];
            let mut event = Event::note(Pitch::parse(name).unwrap(), Duration::eighth());
            event.notes_mut()[0].accidental = Some(Accidental::Sharp);
            event.add_note(Note::new(
                Pitch::parse(&name.replace('#', "##")).unwrap_or(Pitch::parse("C6").unwrap()),
            ));
            if index % 2 == 0 {
                event.notes_mut()[0].lyrics.push(Lyric::new("dense"));
            }
            part.measures[measure_index].push_event(1, event.clone());
        }
        for index in 0..4 {
            let name = ["C4", "D4", "E4", "F4"][index];
            let mut event = Event::note(Pitch::parse(name).unwrap(), Duration::quarter());
            event.dynamics = Some(cadence_core::Dynamic::Mf);
            part.measures[measure_index].push_event(2, event);
        }
        part.measures[measure_index].push_rest(3, Duration::whole());
    }
    let mut score = Score::new("Dense");
    score.parts.push(part);
    let layout = engrave(&score, &Options::default()).unwrap();
    let collisions = find_collisions(&layout);
    assert!(
        collisions.is_empty(),
        "{:?}",
        collisions.iter().map(|c| c.describe()).collect::<Vec<_>>()
    );
}

#[test]
fn very_long_lyrics_and_dynamics_remain_collision_free() {
    let mut part = Part::new("P1", "Words").with_measures(1, Attributes::default());
    for (index, name) in ["C4", "D4", "E4", "F4"].iter().enumerate() {
        let mut event = Event::note(Pitch::parse(name).unwrap(), Duration::quarter());
        event.notes_mut()[0].lyrics.push(Lyric::new(format!(
            "extraordinarilylongsyllable{index}"
        )));
        event.dynamics = Some(cadence_core::Dynamic::Sffz);
        part.measures[0].push_event(1, event);
    }
    let mut score = Score::new("Words");
    score.parts.push(part);
    let layout = engrave(&score, &Options::default()).unwrap();
    assert!(find_collisions(&layout).is_empty());
    assert!(layout.pages[0].elements.iter().any(|element| matches!(
        element,
        cadence_engrave::Element::Text { text, .. } if text.contains("extraordinarily")
    )));
}
