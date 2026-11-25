//! Lyric lane and dynamic placement tests.

use cadence_core::{Attributes, Duration, Event, Lyric, Part, Pitch, Score};
use cadence_engrave::{engrave, Options, SymbolKind};

fn lyric(text: &str, number: u8) -> Lyric {
    Lyric {
        number,
        text: text.to_string(),
        syllabic: cadence_core::Syllabic::Single,
        extend: false,
    }
}

#[test]
fn voice_two_lyrics_sit_below_voice_one_lyrics() {
    let mut score = Score::new("Lyrics");
    let mut part = Part::new("P1", "Piano").with_measures(1, Attributes::default());
    let mut v1 = Event::note(Pitch::parse("C5").unwrap(), Duration::half());
    v1.notes_mut()[0].lyrics.push(lyric("upper", 1));
    let mut v2 = Event::note(Pitch::parse("C4").unwrap(), Duration::half());
    v2.notes_mut()[0].lyrics.push(lyric("lower", 1));
    v2.dynamics = Some(cadence_core::Dynamic::P);
    part.measures[0].push_event(1, v1);
    part.measures[0].push_event(2, v2);
    part.measures[0].push_rest(1, Duration::half());
    part.measures[0].push_rest(2, Duration::half());
    score.parts.push(part);
    let layout = engrave(&score, &Options::default()).unwrap();
    let lyrics: Vec<_> = layout
        .symbols()
        .filter(|symbol| symbol.kind == SymbolKind::Lyric)
        .collect();
    let v1: Vec<_> = lyrics.iter().filter(|symbol| symbol.voice == 1).collect();
    let v2: Vec<_> = lyrics.iter().filter(|symbol| symbol.voice == 2).collect();
    assert!(!v1.is_empty());
    assert!(!v2.is_empty());
    let v1_bottom = v1.iter().map(|symbol| symbol.bbox.bottom()).fold(f64::MIN, f64::max);
    let v2_top = v2.iter().map(|symbol| symbol.bbox.y).fold(f64::MAX, f64::min);
    assert!(v2_top > v1_bottom, "{v2_top} <= {v1_bottom}");
    let dynamic = layout
        .symbols()
        .find(|symbol| symbol.kind == SymbolKind::Dynamic)
        .expect("dynamic");
    assert!(dynamic.bbox.y > v1_bottom);
}

#[test]
fn multiple_lyric_lines_stack_downward() {
    let mut score = Score::new("Stack");
    let mut part = Part::new("P1", "Piano").with_measures(1, Attributes::default());
    let mut event = Event::note(Pitch::parse("E4").unwrap(), Duration::whole());
    event.notes_mut()[0].lyrics.push(lyric("one", 1));
    event.notes_mut()[0].lyrics.push(lyric("two", 2));
    event.notes_mut()[0].lyrics.push(lyric("three", 3));
    part.measures[0].push_event(1, event);
    score.parts.push(part);
    let layout = engrave(&score, &Options::default()).unwrap();
    let mut ys: Vec<f64> = layout
        .symbols()
        .filter(|symbol| symbol.kind == SymbolKind::Lyric)
        .map(|symbol| symbol.bbox.y)
        .collect();
    assert_eq!(ys.len(), 3);
    ys.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert!(ys[1] > ys[0]);
    assert!(ys[2] > ys[1]);
}

#[test]
fn long_lyrics_widen_the_layout() {
    let mut short = Score::new("Short");
    let mut part = Part::new("P1", "Piano").with_measures(1, Attributes::default());
    let mut event = Event::note(Pitch::parse("C4").unwrap(), Duration::eighth());
    event.notes_mut()[0].lyrics.push(lyric("la", 1));
    part.measures[0].push_event(1, event);
    part.measures[0].push_rest(1, Duration::new(cadence_core::NoteValue::Half, 1));
    short.parts.push(part);
    let mut long = short.clone();
    long.parts[0].measures[0].voice_mut(1).events[0].notes_mut()[0].lyrics[0].text =
        "extraordinarilylongsyllable".to_string();
    let short_layout = engrave(&short, &Options::default()).unwrap();
    let long_layout = engrave(&long, &Options::default()).unwrap();
    let short_width = short_layout.pages[0].systems[0].x_end;
    let long_width = long_layout.pages[0].systems[0].x_end;
    assert!(long_width > short_width);
}

#[test]
fn lyrics_are_not_drawn_for_rests() {
    let mut score = Score::new("Rest");
    let mut part = Part::new("P1", "Piano").with_measures(1, Attributes::default());
    part.measures[0].push_rest(1, Duration::whole());
    score.parts.push(part);
    let layout = engrave(&score, &Options::default()).unwrap();
    assert!(!layout
        .symbols()
        .any(|symbol| symbol.kind == SymbolKind::Lyric));
}
