//! Deterministic fixture generator. Not part of the product.
//!
//! Writes 100 varied MusicXML scores plus the built-in demo into
//! `examples/fixtures`. The test suite imports every fixture, exports it again
//! and verifies that notes, durations, voices and metadata survive.

use cadence_core::{
    Articulation, Attributes, BarlineStyle, Clef, Duration, Dynamic, Ending, EndingKind, Event,
    Key, KeyMode, Lyric, Metadata, Note, NoteValue, Part, Pitch, Repeat, Score, SlurKind,
    SlurMark, Syllabic, Tempo, Tie, TimeSignature,
};
use cadence_import::{export_score, import_score};
use std::fs;
use std::path::PathBuf;

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf();
    let out = root.join("examples/fixtures");
    fs::create_dir_all(&out).expect("create fixtures directory");
    let mut written = 0usize;
    for index in 0..100u32 {
        let score = fixture_score(index);
        let xml = export_score(&score).expect("export fixture");
        import_score(&xml).expect("re-import generated fixture");
        let path = out.join(format!("fixture_{index:03}.musicxml"));
        fs::write(&path, xml).expect("write fixture");
        written += 1;
    }
    let demo = Score::demo();
    let demo_xml = export_score(&demo).expect("export demo");
    fs::write(root.join("examples/demo.musicxml"), demo_xml).expect("write demo");
    println!("wrote {written} fixtures and examples/demo.musicxml to {}", out.display());
}

fn fixture_score(index: u32) -> Score {
    let keys = [-3i8, -2, -1, 0, 1, 2, 3];
    let times = [
        TimeSignature::new(4, 4),
        TimeSignature::new(3, 4),
        TimeSignature::new(6, 8),
        TimeSignature::new(2, 2),
        TimeSignature::new(5, 4),
    ];
    let key = Key::new(keys[index as usize % keys.len()], if index % 9 == 0 { KeyMode::Minor } else { KeyMode::Major });
    let time = times[(index as usize / keys.len()) % times.len()];
    let two_parts = (index / 35) % 2 == 1;
    let mut score = Score {
        metadata: Metadata {
            title: format!("Fixture {index:03}"),
            subtitle: format!("{} in {}", time.describe(), key.describe()),
            composer: "Cadence fixtures".to_string(),
            lyricist: if index % 2 == 0 { "Cadence".to_string() } else { String::new() },
            arranger: String::new(),
            copyright: "MIT".to_string(),
            source: "generated".to_string(),
        },
        parts: Vec::new(),
    };
    let tuning = index % 5;
    let mut melody = Part::new("P1", "Melody").with_measures(
        4,
        Attributes {
            divisions: 480,
            key,
            time,
            clef: Clef::treble(),
        },
    );
    melody.midi_program = 0;
    melody.midi_channel = 0;
    fill_part(&mut melody, index, tuning, true);
    score.parts.push(melody);
    if two_parts {
        let mut bass = Part::new("P2", "Bass").with_measures(
            4,
            Attributes {
                divisions: 480,
                key,
                time,
                clef: Clef::bass(),
            },
        );
        bass.midi_program = 32;
        bass.midi_channel = 1;
        fill_bass(&mut bass, index, key);
        score.parts.push(bass);
    }
    score
}

fn fill_part(part: &mut Part, index: u32, tuning: u32, melody: bool) {
    let scale = ["C", "D", "E", "F", "G", "A", "B"];
    for measure_index in 0..part.measures.len() {
        let capacity = part.measures[measure_index].capacity_ticks();
        let divisions = part.measures[measure_index].attributes.divisions;
        let time = part.measures[measure_index].attributes.time;
        let mut events: Vec<(Pitch, Duration)> = Vec::new();
        if time.beat_type == 8 && time.beats % 3 == 0 && index % 4 == 0 {
            let dotted_quarter = Duration::new(NoteValue::Quarter, 1);
            events.push((pitch(scale[(index as usize + measure_index) % 7], 4, 0), dotted_quarter));
            events.push((pitch(scale[(index as usize + measure_index + 2) % 7], 4, 0), dotted_quarter));
        } else if time.beat_type == 8 {
            for beat in 0..time.beats {
                events.push((
                    pitch(scale[(index as usize + measure_index + beat as usize) % 7], 4, 0),
                    Duration::eighth(),
                ));
            }
        } else if index % 6 == 0 && time.beats >= 2 {
            let half_beats = time.beats / 2;
            for beat in 0..half_beats {
                events.push((
                    pitch(scale[(index as usize + measure_index + beat as usize) % 7], 4, 0),
                    Duration::half(),
                ));
            }
            if time.beats % 2 == 1 {
                events.push((
                    pitch(scale[(index as usize + 4) % 7], 4, 0),
                    Duration::quarter(),
                ));
            }
        } else {
            for beat in 0..time.beats {
                events.push((
                    pitch(scale[(index as usize + measure_index + beat as usize) % 7], 4, 0),
                    Duration::quarter(),
                ));
            }
        }
        let total: u64 = events.iter().map(|(_, duration)| duration.ticks(divisions)).sum();
        if total != capacity {
            // Fallback: fill with quarter notes.
            events.clear();
            let quarter_count = capacity / 480;
            for beat in 0..quarter_count {
                events.push((
                    pitch(scale[(index as usize + beat as usize) % 7], 4, 0),
                    Duration::quarter(),
                ));
            }
        }
        for (event_index, (pitch, duration)) in events.iter().enumerate() {
            let mut event = Event::note(*pitch, *duration);
            if melody && (index + event_index as u32 + measure_index as u32) % 7 == 0 {
                if let Some(note) = event.notes_mut().first_mut() {
                    note.lyrics.push(Lyric {
                        number: 1,
                        text: format!("la{}", event_index),
                        syllabic: if event_index % 2 == 0 { Syllabic::Begin } else { Syllabic::End },
                        extend: false,
                    });
                }
            }
            if (index + event_index as u32) % 11 == 0 {
                if let Some(note) = event.notes_mut().first_mut() {
                    note.articulations.push(Articulation::Staccato);
                }
            }
            if event_index == 0 {
                event.dynamics = Some(if index % 3 == 0 { Dynamic::F } else { Dynamic::Mf });
            }
            let base_pitch = event.primary_pitch();
            part.measures[measure_index].push_event(1, event);
            if index % 3 == 1 && event_index % 2 == 0 && tuning != 3 {
                let third = base_pitch.unwrap_or(Pitch::c4()).step_by(2);
                let mut chord_note = Note::new(third);
                chord_note.velocity = 72;
                if let Some(base) = part.measures[measure_index]
                    .voice_mut(1)
                    .events
                    .last_mut()
                {
                    base.add_note(chord_note);
                }
            }
        }
        let voice_len = part.measures[measure_index]
            .voice(1)
            .map(|voice| voice.events.len())
            .unwrap_or(0);
        if index % 13 == 0 && voice_len >= 2 {
            let first_pitch = part.measures[measure_index]
                .voice(1)
                .and_then(|voice| voice.events[0].primary_pitch());
            if let Some(first_pitch) = first_pitch {
                let voice = &mut part.measures[measure_index].voice_mut(1);
                if let Some(note) = voice.events[0].notes_mut().first_mut() {
                    note.tie = Tie::new(true, false);
                }
                if let Some(note) = voice.events[1].notes_mut().first_mut() {
                    note.pitch = first_pitch;
                    note.tie = Tie::new(false, true);
                }
            }
        }
        if index % 17 == 0 && voice_len >= 2 {
            let voice = &mut part.measures[measure_index].voice_mut(1);
            if let Some(note) = voice.events[0].notes_mut().first_mut() {
                note.slurs.push(SlurMark::new(1, SlurKind::Start));
            }
            if let Some(note) = voice.events[1].notes_mut().first_mut() {
                note.slurs.push(SlurMark::new(1, SlurKind::Stop));
            }
        }
        if index % 19 == 0 && voice_len >= 3 {
            let voice = &mut part.measures[measure_index].voice_mut(1);
            if let Some(note) = voice.events[2].notes_mut().first_mut() {
                note.accidental = Some(cadence_core::Accidental::Sharp);
                note.pitch = Pitch::parse(&format!("{}#4", note.pitch.step.name()))
                    .unwrap_or(note.pitch);
            }
        }
        // A second voice of rests exercises multi-voice spacing.
        if index % 4 == 1 {
            let rest = Duration::from_ticks(divisions, capacity).unwrap_or(Duration::whole());
            part.measures[measure_index].push_rest(2, rest);
        }
        if index % 23 == 0 && measure_index == 2 {
            part.measures[measure_index].ending = Some(Ending {
                number: 1,
                kind: EndingKind::Start,
            });
        }
        if index % 29 == 0 && measure_index == 3 {
            part.measures[measure_index].barline = BarlineStyle::Final;
        }
    }
    part.measures[0].tempo = Some(Tempo::new(80 + index % 70, NoteValue::Quarter));
    part.measures[0].rehearsal = Some(format!("F{index:03}"));
    if index % 31 == 0 {
        part.measures[3].repeat = Some(Repeat::backward());
    }
}

fn fill_bass(part: &mut Part, index: u32, key: Key) {
    for measure_index in 0..part.measures.len() {
        let notes = if key.fifths >= 0 {
            ["C3", "G3", "F3", "C3"]
        } else {
            ["F3", "C3", "Bb2", "F3"]
        };
        let name = notes[(index as usize + measure_index) % notes.len()];
        let capacity = part.measures[measure_index].capacity_ticks();
        let duration = Duration::from_ticks(480, capacity).unwrap_or(Duration::whole());
        part.measures[measure_index].push_note(
            1,
            Pitch::parse(name).unwrap(),
            duration,
        );
        part.measures[measure_index].push_rest(2, duration);
    }
}

fn pitch(step: &str, octave: i8, alter: i8) -> Pitch {
    Pitch::new(
        cadence_core::Step::parse(step).unwrap_or(cadence_core::Step::C),
        alter,
        octave,
    )
}

