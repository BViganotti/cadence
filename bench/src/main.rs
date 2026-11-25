//! Dependency-free benchmark harness. Prints honest wall-clock timings for a
//! deterministic synthetic score. Run in release mode for representative
//! numbers:
//!
//!     cargo run --release -p cadence-bench

use cadence_core::{
    Attributes, Duration, Dynamic, Lyric, Note, Pitch, Score, Syllabic, Part,
};
use cadence_engrave::{engrave, Options};
use cadence_export::{render_midi, render_pdf, render_svg};
use cadence_import::{export_score, import_score};
use std::time::{Duration as Clock, Instant};

fn main() {
    let measures = std::env::args()
        .nth(1)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(120);
    let score = synthetic_score(measures);
    let xml = export_score(&score).expect("export synthetic score");
    let options = Options::default();

    println!("Cadence benchmark");
    println!(
        "score: {} measures, {} parts, {} notes, {} XML bytes",
        score.measure_count(),
        score.parts.len(),
        score.note_count(),
        xml.len()
    );

    let import_time = median(20, || {
        let parsed = import_score(&xml).expect("import synthetic score");
        std::hint::black_box(parsed.note_count())
    });
    let layout = engrave(&score, &options).expect("engrave synthetic score");
    println!(
        "layout: {} pages, {} systems, {} symbols",
        layout.page_count(),
        layout.system_count(),
        layout.symbols().count()
    );
    let engrave_time = median(20, || {
        let result = engrave(&score, &options).expect("engrave");
        std::hint::black_box(result.page_count())
    });
    let svg = render_svg(&layout);
    let svg_time = median(20, || {
        let result = render_svg(&layout);
        std::hint::black_box(result.len())
    });
    let pdf = render_pdf(&layout).expect("pdf");
    let pdf_time = median(20, || {
        let result = render_pdf(&layout).expect("pdf");
        std::hint::black_box(result.len())
    });
    let midi = render_midi(&score).expect("midi");
    let midi_time = median(20, || {
        let result = render_midi(&score).expect("midi");
        std::hint::black_box(result.len())
    });

    println!("import MusicXML : {:>10.3} ms   ({} bytes)", ms(import_time), xml.len());
    println!("engrave layout  : {:>10.3} ms", ms(engrave_time));
    println!("render SVG      : {:>10.3} ms   ({} bytes)", ms(svg_time), svg.len());
    println!("render PDF      : {:>10.3} ms   ({} bytes)", ms(pdf_time), pdf.len());
    println!("render MIDI     : {:>10.3} ms   ({} bytes)", ms(midi_time), midi.len());
    println!(
        "total pipeline  : {:>10.3} ms",
        ms(import_time + engrave_time + svg_time + pdf_time + midi_time)
    );
    println!("timings are medians of 20 measured iterations after one warmup");
}

fn synthetic_score(measures: usize) -> Score {
    let measures = measures.clamp(1, 2_000);
    let attributes = Attributes::default();
    let mut part = Part::new("P1", "Benchmark").with_measures(measures, attributes);
    let scale = ["C", "D", "E", "F", "G", "A", "B"];
    for measure_index in 0..measures {
        for beat in 0..4 {
            let name = format!(
                "{}{}",
                scale[(measure_index + beat) % scale.len()],
                4 + ((measure_index / 8) % 2)
            );
            let pitch = Pitch::parse(&name).unwrap_or_else(|_| Pitch::c4());
            let mut event = cadence_core::Event::note(pitch, Duration::quarter());
            if measure_index % 3 == 0 && beat == 2 {
                let chord_pitch = Pitch::parse(&format!(
                    "{}{}",
                    scale[(measure_index + beat + 2) % scale.len()],
                    4
                ))
                .unwrap_or(pitch);
                event.add_note(Note::new(chord_pitch));
            }
            if measure_index % 5 == 0 && beat == 0 {
                event.dynamics = Some(Dynamic::Mf);
            }
            if measure_index % 7 == 0 && beat == 1 {
                if let Some(note) = event.notes_mut().first_mut() {
                    note.lyrics.push(Lyric {
                        number: 1,
                        text: format!("la{measure_index}"),
                        syllabic: Syllabic::Single,
                        extend: false,
                    });
                }
            }
            part.measures[measure_index].push_event(1, event);
        }
        part.measures[measure_index].push_rest(2, Duration::whole());
    }
    part.measures[measures - 1].barline = cadence_core::BarlineStyle::Final;
    let mut score = Score::new("Benchmark score");
    score.metadata.composer = "Cadence bench".to_string();
    score.parts.push(part);
    score
}

fn median<F>(iterations: usize, mut run: F) -> Clock
where
    F: FnMut() -> usize,
{
    let _ = run();
    let mut samples = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let start = Instant::now();
        std::hint::black_box(run());
        samples.push(start.elapsed());
    }
    samples.sort();
    samples[samples.len() / 2]
}

fn ms(duration: Clock) -> f64 {
    duration.as_secs_f64() * 1000.0
}
