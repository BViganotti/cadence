//! Original vector glyphs for clefs, accidentals, rests, flags and marks.

use crate::geom::{Align, Element, PathSeg, Point};
use crate::text;
use cadence_core::{Accidental, Articulation, Clef, ClefSign, Key, NoteValue, TimeSignature, TimeSymbol};

pub fn accidental(accidental: Accidental, x: f64, y: f64, s: f64) -> Vec<Element> {
    let mut out = Vec::new();
    match accidental {
        Accidental::Sharp => sharp(&mut out, x, y, s),
        Accidental::Flat => flat(&mut out, x, y, s, 0.0),
        Accidental::Natural => natural(&mut out, x, y, s),
        Accidental::DoubleSharp => {
            let h = 0.55 * s;
            out.push(Element::line(x, y - h, x + 0.9 * s, y + h, 0.16 * s));
            out.push(Element::line(x + 0.9 * s, y - h, x, y + h, 0.16 * s));
        }
        Accidental::DoubleFlat => {
            flat(&mut out, x, y, s, 0.0);
            flat(&mut out, x, y + 0.18 * s, s, 0.75 * s);
        }
    }
    out
}

fn sharp(out: &mut Vec<Element>, x: f64, y: f64, s: f64) {
    let w = 0.8 * s;
    let h = 0.85 * s;
    out.push(Element::line(x + 0.18 * w, y - h, x + 0.18 * w, y + h, 0.14 * s));
    out.push(Element::line(x + 0.72 * w, y - h, x + 0.72 * w, y + h, 0.14 * s));
    out.push(Element::line(x, y - 0.22 * h, x + w, y - 0.22 * h, 0.32 * s));
    out.push(Element::line(x, y + 0.28 * h, x + w, y + 0.28 * h, 0.32 * s));
}

fn flat(out: &mut Vec<Element>, x: f64, y: f64, s: f64, offset_x: f64) {
    let x = x + offset_x;
    out.push(Element::line(x + 0.12 * s, y - 0.95 * s, x + 0.12 * s, y + 0.72 * s, 0.15 * s));
    let mut segments = Vec::new();
    segments.push(PathSeg::Move(Point::new(x + 0.12 * s, y - 0.15 * s)));
    segments.push(PathSeg::Cubic {
        c1: Point::new(x + 0.75 * s, y - 0.45 * s),
        c2: Point::new(x + 0.95 * s, y + 0.35 * s),
        to: Point::new(x + 0.12 * s, y + 0.72 * s),
    });
    segments.push(PathSeg::Close);
    out.push(Element::Path {
        segments,
        stroke: 0.0,
        fill: true,
    });
}

fn natural(out: &mut Vec<Element>, x: f64, y: f64, s: f64) {
    out.push(Element::line(x + 0.12 * s, y - 0.95 * s, x + 0.12 * s, y + 0.38 * s, 0.15 * s));
    out.push(Element::line(x + 0.62 * s, y - 0.38 * s, x + 0.62 * s, y + 0.95 * s, 0.15 * s));
    out.push(Element::line(x + 0.02 * s, y - 0.12 * s, x + 0.72 * s, y - 0.12 * s, 0.24 * s));
    out.push(Element::line(x + 0.02 * s, y + 0.34 * s, x + 0.72 * s, y + 0.34 * s, 0.24 * s));
}

pub fn clef(clef: Clef, x: f64, top: f64, s: f64) -> Vec<Element> {
    let mut out = Vec::new();
    match clef.sign {
        ClefSign::G => {
            let mut segments = Vec::new();
            segments.push(PathSeg::Move(Point::new(x + 1.6 * s, top + 1.0 * s)));
            segments.push(PathSeg::Cubic {
                c1: Point::new(x + 0.2 * s, top + 1.8 * s),
                c2: Point::new(x + 0.1 * s, top + 4.0 * s),
                to: Point::new(x + 0.95 * s, top + 5.25 * s),
            });
            segments.push(PathSeg::Cubic {
                c1: Point::new(x + 1.75 * s, top + 6.25 * s),
                c2: Point::new(x + 2.45 * s, top + 5.2 * s),
                to: Point::new(x + 2.1 * s, top + 3.6 * s),
            });
            segments.push(PathSeg::Cubic {
                c1: Point::new(x + 1.8 * s, top + 2.0 * s),
                c2: Point::new(x + 0.85 * s, top + 1.05 * s),
                to: Point::new(x + 1.45 * s, top - 0.35 * s),
            });
            segments.push(PathSeg::Cubic {
                c1: Point::new(x + 1.95 * s, top - 1.35 * s),
                c2: Point::new(x + 2.5 * s, top + 0.15 * s),
                to: Point::new(x + 2.0 * s, top + 1.5 * s),
            });
            segments.push(PathSeg::Cubic {
                c1: Point::new(x + 1.55 * s, top + 2.8 * s),
                c2: Point::new(x + 0.8 * s, top + 2.65 * s),
                to: Point::new(x + 0.95 * s, top + 1.8 * s),
            });
            out.push(Element::Path {
                segments,
                stroke: 0.22 * s,
                fill: false,
            });
            out.push(Element::line(
                x + 1.25 * s,
                top + 1.8 * s,
                x + 1.25 * s,
                top + 6.2 * s,
                0.16 * s,
            ));
            out.push(Element::Path {
                segments: vec![
                    PathSeg::Move(Point::new(x + 1.25 * s, top + 6.2 * s)),
                    PathSeg::Cubic {
                        c1: Point::new(x + 0.85 * s, top + 6.9 * s),
                        c2: Point::new(x + 0.45 * s, top + 6.4 * s),
                        to: Point::new(x + 0.6 * s, top + 5.9 * s),
                    },
                ],
                stroke: 0.18 * s,
                fill: false,
            });
        }
        ClefSign::F => {
            let mut segments = Vec::new();
            segments.push(PathSeg::Move(Point::new(x + 0.7 * s, top + 1.0 * s)));
            segments.push(PathSeg::Cubic {
                c1: Point::new(x + 2.6 * s, top + 0.5 * s),
                c2: Point::new(x + 2.7 * s, top + 2.8 * s),
                to: Point::new(x + 1.1 * s, top + 3.2 * s),
            });
            segments.push(PathSeg::Cubic {
                c1: Point::new(x + 0.15 * s, top + 3.5 * s),
                c2: Point::new(x + 0.25 * s, top + 5.0 * s),
                to: Point::new(x + 1.35 * s, top + 5.1 * s),
            });
            out.push(Element::Path {
                segments,
                stroke: 0.28 * s,
                fill: false,
            });
            out.push(Element::ellipse(x + 2.15 * s, top + 1.0 * s, 0.18 * s, 0.18 * s, true));
            out.push(Element::ellipse(x + 2.15 * s, top + 1.9 * s, 0.18 * s, 0.18 * s, true));
        }
        ClefSign::C => {
            for offset in [0.2 * s, 2.2 * s] {
                let mut segments = Vec::new();
                segments.push(PathSeg::Move(Point::new(x + 1.55 * s, top + offset + 0.2 * s)));
                segments.push(PathSeg::Cubic {
                    c1: Point::new(x + 0.15 * s, top + offset + 0.3 * s),
                    c2: Point::new(x + 0.15 * s, top + offset + 1.6 * s),
                    to: Point::new(x + 1.55 * s, top + offset + 1.7 * s),
                });
                out.push(Element::Path {
                    segments,
                    stroke: 0.22 * s,
                    fill: false,
                });
            }
            out.push(Element::line(
                x + 1.55 * s,
                top + 0.9 * s,
                x + 1.55 * s,
                top + 3.1 * s,
                0.16 * s,
            ));
        }
        ClefSign::Percussion => {
            out.push(Element::line(
                x + 0.75 * s,
                top + 0.7 * s,
                x + 0.75 * s,
                top + 3.3 * s,
                0.3 * s,
            ));
            out.push(Element::line(
                x + 1.65 * s,
                top + 0.7 * s,
                x + 1.65 * s,
                top + 3.3 * s,
                0.3 * s,
            ));
        }
    }
    out
}

pub fn clef_width(clef: Clef, s: f64) -> f64 {
    match clef.sign {
        ClefSign::G => 3.0 * s,
        ClefSign::F => 2.8 * s,
        ClefSign::C => 2.2 * s,
        ClefSign::Percussion => 2.2 * s,
    }
}

pub fn time_signature(time: TimeSignature, x: f64, top: f64, s: f64) -> Vec<Element> {
    let mut out = Vec::new();
    let size = 1.95 * s;
    match time.symbol {
        TimeSymbol::Common => {
            out.push(Element::text(x + 1.2 * s, top + 0.55 * s, size * 1.3, "C", Align::Center));
        }
        TimeSymbol::Cut => {
            out.push(Element::text(x + 1.2 * s, top + 0.55 * s, size * 1.3, "C", Align::Center));
            out.push(Element::line(
                x + 0.55 * s,
                top + 0.4 * s,
                x + 0.55 * s,
                top + 3.6 * s,
                0.16 * s,
            ));
        }
        TimeSymbol::Normal => {
            out.push(Element::text(
                x + 1.1 * s,
                top + 0.55 * s,
                size,
                time.beats.to_string(),
                Align::Center,
            ));
            out.push(Element::text(
                x + 1.1 * s,
                top + 2.35 * s,
                size,
                time.beat_type.to_string(),
                Align::Center,
            ));
        }
    }
    out
}

pub fn time_signature_width(_time: TimeSignature, s: f64) -> f64 {
    2.4 * s
}

/// Draw the key signature; returns the used width.
pub fn key_signature(key: Key, clef: Clef, x: f64, top: f64, s: f64) -> (Vec<Element>, f64) {
    let mut out = Vec::new();
    let middle = clef.middle_line_diatonic();
    let bottom = clef.bottom_line_diatonic();
    let accidentals = key.accidentals();
    let mut width = 0.0;
    for (index, (step, accidental_value)) in accidentals.iter().enumerate() {
        let offset = match accidental_value {
            Accidental::Sharp => match step {
                cadence_core::Step::F => 4,
                cadence_core::Step::C => 1,
                cadence_core::Step::G => 5,
                cadence_core::Step::D => 2,
                cadence_core::Step::A => -1,
                cadence_core::Step::E => 3,
                cadence_core::Step::B => 0,
            },
            _ => match step {
                cadence_core::Step::B => 0,
                cadence_core::Step::E => 3,
                cadence_core::Step::A => -1,
                cadence_core::Step::D => 2,
                cadence_core::Step::G => -3,
                cadence_core::Step::C => 1,
                cadence_core::Step::F => -4,
            },
        };
        let diatonic = middle + offset;
        let y = top + 4.0 * s - (diatonic - bottom) as f64 * s * 0.5;
        let x_offset = index as f64 * 1.35 * s;
        out.extend(accidental(*accidental_value, x + x_offset, y, s));
        width = x_offset + 1.35 * s;
    }
    if !accidentals.is_empty() {
        width += 0.4 * s;
    }
    (out, width)
}

pub fn rest(value: NoteValue, x: f64, top: f64, s: f64) -> Vec<Element> {
    let mut out = Vec::new();
    match value {
        NoteValue::Whole => {
            out.push(Element::rect(x - 0.65 * s, top + 0.85 * s, 1.3 * s, 0.5 * s, true));
        }
        NoteValue::Half => {
            out.push(Element::rect(x - 0.65 * s, top + 2.15 * s, 1.3 * s, 0.5 * s, true));
        }
        NoteValue::Quarter => {
            let mut segments = Vec::new();
            segments.push(PathSeg::Move(Point::new(x + 0.2 * s, top + 1.35 * s)));
            segments.push(PathSeg::Line(Point::new(x - 0.35 * s, top + 2.3 * s)));
            segments.push(PathSeg::Line(Point::new(x + 0.35 * s, top + 2.55 * s)));
            segments.push(PathSeg::Line(Point::new(x - 0.3 * s, top + 3.45 * s)));
            segments.push(PathSeg::Line(Point::new(x + 0.1 * s, top + 2.85 * s)));
            segments.push(PathSeg::Line(Point::new(x + 0.45 * s, top + 3.15 * s)));
            segments.push(PathSeg::Line(Point::new(x + 0.05 * s, top + 2.35 * s)));
            segments.push(PathSeg::Close);
            out.push(Element::Path {
                segments,
                stroke: 0.0,
                fill: true,
            });
            out.push(Element::line(
                x + 0.45 * s,
                top + 2.3 * s,
                x + 0.2 * s,
                top + 3.3 * s,
                0.18 * s,
            ));
        }
        other => {
            let hooks = other.flags().max(1);
            out.push(Element::line(x, top + 1.5 * s, x, top + 3.15 * s, 0.16 * s));
            for hook in 0..hooks {
                let y = top + (1.55 + hook as f64 * 0.45) * s;
                let mut segments = Vec::new();
                segments.push(PathSeg::Move(Point::new(x, y)));
                segments.push(PathSeg::Cubic {
                    c1: Point::new(x + 0.85 * s, y + 0.1 * s),
                    c2: Point::new(x + 0.85 * s, y + 0.6 * s),
                    to: Point::new(x + 0.3 * s, y + 0.72 * s),
                });
                out.push(Element::Path {
                    segments,
                    stroke: 0.18 * s,
                    fill: false,
                });
            }
            out.push(Element::ellipse(x + 0.4 * s, top + 2.1 * s, 0.1 * s, 0.1 * s, true));
        }
    }
    out
}

pub fn flag(x: f64, y: f64, s: f64, up: bool, count: u8) -> Vec<Element> {
    let mut out = Vec::new();
    let direction = if up { 1.0 } else { -1.0 };
    for index in 0..count {
        let offset = index as f64 * 0.55 * s * direction;
        let mut segments = Vec::new();
        segments.push(PathSeg::Move(Point::new(x, y + offset)));
        segments.push(PathSeg::Cubic {
            c1: Point::new(x + 0.9 * s, y + offset + 0.2 * s * direction),
            c2: Point::new(x + 0.9 * s, y + offset + 0.9 * s * direction),
            to: Point::new(x + 0.25 * s, y + offset + 1.05 * s * direction),
        });
        out.push(Element::Path {
            segments,
            stroke: 0.2 * s,
            fill: false,
        });
    }
    out
}

pub fn beam(x1: f64, y1: f64, x2: f64, y2: f64, thickness: f64) -> Element {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let length = (dx * dx + dy * dy).sqrt().max(0.001);
    let nx = -dy / length * thickness / 2.0;
    let ny = dx / length * thickness / 2.0;
    Element::Path {
        segments: vec![
            PathSeg::Move(Point::new(x1 + nx, y1 + ny)),
            PathSeg::Line(Point::new(x2 + nx, y2 + ny)),
            PathSeg::Line(Point::new(x2 - nx, y2 - ny)),
            PathSeg::Line(Point::new(x1 - nx, y1 - ny)),
            PathSeg::Close,
        ],
        stroke: 0.0,
        fill: true,
    }
}

pub fn articulation(articulation: Articulation, x: f64, y: f64, s: f64, above: bool) -> Vec<Element> {
    let mut out = Vec::new();
    let direction = if above { -1.0 } else { 1.0 };
    match articulation {
        Articulation::Staccato => {
            out.push(Element::ellipse(x, y, 0.22 * s, 0.18 * s, true));
        }
        Articulation::Staccatissimo => {
            let mut segments = vec![
                PathSeg::Move(Point::new(x, y - direction * 0.35 * s)),
                PathSeg::Line(Point::new(x - 0.25 * s, y + direction * 0.25 * s)),
                PathSeg::Line(Point::new(x + 0.25 * s, y + direction * 0.25 * s)),
                PathSeg::Close,
            ];
            segments.push(PathSeg::Close);
            out.push(Element::Path {
                segments,
                stroke: 0.0,
                fill: true,
            });
        }
        Articulation::Accent => {
            out.push(Element::line(
                x - 0.45 * s,
                y - direction * 0.25 * s,
                x,
                y + direction * 0.2 * s,
                0.16 * s,
            ));
            out.push(Element::line(
                x,
                y + direction * 0.2 * s,
                x + 0.45 * s,
                y - direction * 0.25 * s,
                0.16 * s,
            ));
        }
        Articulation::Tenuto => {
            out.push(Element::line(x - 0.45 * s, y, x + 0.45 * s, y, 0.18 * s));
        }
        Articulation::Marcato => {
            out.push(Element::line(
                x - 0.5 * s,
                y + direction * 0.3 * s,
                x,
                y - direction * 0.25 * s,
                0.18 * s,
            ));
            out.push(Element::line(
                x,
                y - direction * 0.25 * s,
                x + 0.5 * s,
                y + direction * 0.3 * s,
                0.18 * s,
            ));
        }
        Articulation::Fermata => {
            let mut segments = Vec::new();
            segments.push(PathSeg::Move(Point::new(x - 0.7 * s, y + direction * 0.25 * s)));
            segments.push(PathSeg::Cubic {
                c1: Point::new(x - 0.4 * s, y - direction * 0.55 * s),
                c2: Point::new(x + 0.4 * s, y - direction * 0.55 * s),
                to: Point::new(x + 0.7 * s, y + direction * 0.25 * s),
            });
            out.push(Element::Path {
                segments,
                stroke: 0.16 * s,
                fill: false,
            });
            out.push(Element::ellipse(
                x,
                y - direction * 0.35 * s,
                0.1 * s,
                0.1 * s,
                true,
            ));
        }
        Articulation::BreathMark => {
            out.push(Element::line(
                x - 0.2 * s,
                y - 0.2 * s,
                x + 0.25 * s,
                y + 0.25 * s,
                0.18 * s,
            ));
            out.push(Element::line(
                x - 0.2 * s,
                y + 0.05 * s,
                x + 0.1 * s,
                y + 0.35 * s,
                0.14 * s,
            ));
        }
        Articulation::UpBow => {
            out.push(Element::line(
                x - 0.35 * s,
                y - 0.25 * s,
                x,
                y + 0.3 * s,
                0.15 * s,
            ));
            out.push(Element::line(
                x,
                y + 0.3 * s,
                x + 0.35 * s,
                y - 0.25 * s,
                0.15 * s,
            ));
        }
        Articulation::DownBow => {
            out.push(Element::line(
                x - 0.35 * s,
                y + 0.25 * s,
                x,
                y - 0.3 * s,
                0.15 * s,
            ));
            out.push(Element::line(
                x,
                y - 0.3 * s,
                x + 0.35 * s,
                y + 0.25 * s,
                0.15 * s,
            ));
        }
        Articulation::Harmonic | Articulation::OpenString => {
            out.push(Element::ellipse(x, y, 0.25 * s, 0.22 * s, false));
        }
        Articulation::Pizzicato | Articulation::SnapPizzicato => {
            out.push(Element::ellipse(x, y, 0.16 * s, 0.16 * s, true));
            out.push(Element::line(x, y, x, y - 0.55 * s, 0.12 * s));
        }
    }
    out
}

pub fn accidental_width(_accidental: Accidental, s: f64) -> f64 {
    1.05 * s
}

pub fn rest_width(_value: NoteValue, s: f64) -> f64 {
    1.4 * s
}

pub fn dynamic_text(dynamic: cadence_core::Dynamic) -> &'static str {
    dynamic.musicxml_name()
}

pub fn lyrics_text(text_value: &str, size: f64) -> f64 {
    text::width(text_value, size)
}
