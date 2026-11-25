//! Built-in demonstration score.
//!
//! The demo is intentionally small but exercises everything the studio,
//! engraver and exporters need: two parts, chords, rests, beams, ties, slurs,
//! lyrics, dynamics, articulations, key/time/clef changes and a pickup-free
//! eight-bar form.

use super::attributes::{
    Attributes, Clef, TimeSignature, Key, KeyMode, TimeSymbol,
};
use super::duration::{Duration, NoteValue};
use super::marks::{Articulation, Dynamic, Lyric, SlurKind, SlurMark, Syllabic, Tie};
use super::measure::Tempo;
use super::note::{Event, Note};
use super::part::Part;
use super::pitch::Pitch;
use super::score::{Metadata, Score};

fn pitch(name: &str) -> Pitch {
    Pitch::parse(name).unwrap_or_else(|_| Pitch::c4())
}

fn add<'a>(
    part: &'a mut Part,
    measure: usize,
    voice: u8,
    name: &str,
    duration: Duration,
) -> &'a mut Event {
    let event = Event::note(pitch(name), duration);
    part.measures[measure].push_event(voice, event)
}

fn add_rest(part: &mut Part, measure: usize, voice: u8, duration: Duration) {
    part.measures[measure].push_rest(voice, duration);
}

fn add_lyric(event: &mut Event, text: &str, syllabic: Syllabic) {
    if let Some(note) = event.notes_mut().first_mut() {
        note.lyrics.push(Lyric {
            number: 1,
            text: text.to_string(),
            syllabic,
            extend: false,
        });
    }
}

fn add_slur(event: &mut Event, number: u8, kind: SlurKind) {
    if let Some(note) = event.notes_mut().first_mut() {
        note.slurs.push(SlurMark::new(number, kind));
    }
}

fn add_articulation(event: &mut Event, articulation: Articulation) {
    if let Some(note) = event.notes_mut().first_mut() {
        note.articulations.push(articulation);
    }
}

pub fn demo_score() -> Score {
    let mut score = Score {
        metadata: Metadata {
            title: "Cadence in G".to_string(),
            subtitle: "Demonstration score".to_string(),
            composer: "Cadence".to_string(),
            lyricist: "Cadence".to_string(),
            arranger: "Cadence".to_string(),
            copyright: "MIT licensed example".to_string(),
            source: "Cadence built-in demo".to_string(),
        },
        parts: Vec::new(),
    };

    let mut key = Key::new(1, KeyMode::Major);
    let mut melody_attributes = Attributes {
        divisions: 480,
        key,
        time: TimeSignature {
            beats: 4,
            beat_type: 4,
            symbol: TimeSymbol::Common,
        },
        clef: Clef::treble(),
    };
    let bass_attributes = Attributes {
        divisions: 480,
        key,
        time: TimeSignature {
            beats: 4,
            beat_type: 4,
            symbol: TimeSymbol::Common,
        },
        clef: Clef::bass(),
    };

    let mut melody = Part::new("P1", "Melody").with_measures(8, melody_attributes);
    melody.abbreviation = "Mel.".to_string();
    melody.midi_program = 73;
    melody.midi_channel = 0;
    melody.measures[0].tempo = Some(Tempo::new(96, NoteValue::Quarter));
    melody.measures[0].rehearsal = Some("A".to_string());
    melody.measures[4].rehearsal = Some("B".to_string());

    let m0 = 0;
    add(&mut melody, m0, 1, "G4", Duration::quarter()).dynamics = Some(Dynamic::Mf);
    add_lyric(
        &mut melody.measures[m0].voice_mut(1).events[0],
        "Ca",
        Syllabic::Begin,
    );
    add(&mut melody, m0, 1, "A4", Duration::quarter());
    add_lyric(
        &mut melody.measures[m0].voice_mut(1).events[1],
        "dence",
        Syllabic::End,
    );
    add(&mut melody, m0, 1, "B4", Duration::quarter());
    add_lyric(
        &mut melody.measures[m0].voice_mut(1).events[2],
        "in",
        Syllabic::Single,
    );
    let last = add(&mut melody, m0, 1, "D5", Duration::quarter());
    add_lyric(last, "G", Syllabic::Single);
    add_articulation(last, Articulation::Accent);

    let m1 = 1;
    let note = add(&mut melody, m1, 1, "B4", Duration::eighth());
    add_lyric(note, "gar", Syllabic::Begin);
    add_articulation(note, Articulation::Staccato);
    let note = add(&mut melody, m1, 1, "A4", Duration::eighth());
    add_lyric(note, "den", Syllabic::End);
    add(&mut melody, m1, 1, "G4", Duration::quarter());
    add(&mut melody, m1, 1, "E4", Duration::half());

    let m2 = 2;
    let slur_start = add(&mut melody, m2, 1, "E4", Duration::quarter());
    add_slur(slur_start, 1, SlurKind::Start);
    let slur_stop = add(&mut melody, m2, 1, "G4", Duration::quarter());
    add_slur(slur_stop, 1, SlurKind::Stop);
    add(&mut melody, m2, 1, "C5", Duration::half());

    let m3 = 3;
    add(&mut melody, m3, 1, "B4", Duration::quarter());
    add(&mut melody, m3, 1, "A4", Duration::quarter());
    let tied = add(&mut melody, m3, 1, "G4", Duration::half());
    if let Some(note) = tied.notes_mut().first_mut() {
        note.tie = Tie::new(true, false);
    }

    let m4 = 4;
    let tied_stop = add(&mut melody, m4, 1, "G4", Duration::quarter());
    if let Some(note) = tied_stop.notes_mut().first_mut() {
        note.tie = Tie::new(false, true);
    }
    add(&mut melody, m4, 1, "E5", Duration::quarter()).dynamics = Some(Dynamic::P);
    add(&mut melody, m4, 1, "D5", Duration::quarter());
    add(&mut melody, m4, 1, "B4", Duration::quarter());

    let m5 = 5;
    add(&mut melody, m5, 1, "C5", Duration::quarter());
    add(&mut melody, m5, 1, "B4", Duration::quarter());
    add(&mut melody, m5, 1, "A4", Duration::quarter());
    add(&mut melody, m5, 1, "G4", Duration::quarter());

    let m6 = 6;
    add(&mut melody, m6, 1, "A4", Duration::eighth());
    add(&mut melody, m6, 1, "B4", Duration::eighth());
    add(&mut melody, m6, 1, "C5", Duration::eighth());
    add(&mut melody, m6, 1, "D5", Duration::eighth());
    let peak = add(&mut melody, m6, 1, "E5", Duration::half());
    add_articulation(peak, Articulation::Fermata);
    peak.dynamics = Some(Dynamic::F);

    let m7 = 7;
    // A final chord: D5, G4 and B4 sounding together for two beats.
    let chord = Event::chord(
        vec![
            Note::new(pitch("G4")),
            Note::new(pitch("B4")),
            Note::new(pitch("D5")),
        ],
        Duration::half(),
    );
    melody.measures[m7].push_event(1, chord);
    add(&mut melody, m7, 1, "G4", Duration::half());
    add_rest(&mut melody, m7, 2, Duration::whole());

    let mut bass = Part::new("P2", "Bass").with_measures(8, bass_attributes);
    bass.abbreviation = "Bass".to_string();
    bass.midi_program = 42;
    bass.midi_channel = 1;

    let patterns: [(&str, &str); 8] = [
        ("G3", "D4"),
        ("E3", "C4"),
        ("C3", "G3"),
        ("D3", "D3"),
        ("B3", "G3"),
        ("A3", "E3"),
        ("F#3", "D3"),
        ("G3", "G3"),
    ];
    for (index, (low, high)) in patterns.iter().enumerate() {
        add(&mut bass, index, 1, low, Duration::half());
        add(&mut bass, index, 1, high, Duration::half());
    }
    if let Some(first) = bass.measures[0].voice(1).and_then(|v| v.events.first()) {
        let _ = first;
    }
    bass.measures[7].barline = super::measure::BarlineStyle::Final;

    // A short second voice in the bass adds a written rest so the engraver has
    // to place two voices on one staff without collisions.
    for measure in 0..8 {
        add_rest(&mut bass, measure, 2, Duration::whole());
    }

    // Mid-score key change on the last two bars. Attributes are resolved and
    // copied forward for every following measure.
    key = Key::new(-1, KeyMode::Major);
    for measure in &mut melody.measures[6..] {
        measure.attributes.key = key;
        measure.explicit_attributes = true;
    }
    for measure in &mut bass.measures[6..] {
        measure.attributes.key = key;
        measure.explicit_attributes = true;
    }

    score.parts.push(melody);
    score.parts.push(bass);
    score
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_is_structural_and_musically_clean() {
        let score = demo_score();
        score.validate().unwrap();
        let report = score.inspect();
        assert!(report.is_ok(), "{:?}", report.issues);
    }

    #[test]
    fn demo_has_two_parts_and_expected_measures() {
        let score = demo_score();
        assert_eq!(score.parts.len(), 2);
        assert_eq!(score.measure_count(), 8);
        assert!(score.note_count() > 40);
        assert!(score.total_ticks() > 0);
    }
}
