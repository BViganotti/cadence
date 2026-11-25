//! Pitches, steps, spelling and MIDI conversion.

use crate::error::{Error, Result};
use std::fmt;

/// Diatonic step name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Step {
    C,
    D,
    E,
    F,
    G,
    A,
    B,
}

impl Step {
    pub const ALL: [Step; 7] = [
        Step::C,
        Step::D,
        Step::E,
        Step::F,
        Step::G,
        Step::A,
        Step::B,
    ];

    /// Semitones above C within an octave.
    pub const fn semitone(self) -> i8 {
        match self {
            Step::C => 0,
            Step::D => 2,
            Step::E => 4,
            Step::F => 5,
            Step::G => 7,
            Step::A => 9,
            Step::B => 11,
        }
    }

    /// Index within the octave, C = 0.
    pub const fn index(self) -> i8 {
        match self {
            Step::C => 0,
            Step::D => 1,
            Step::E => 2,
            Step::F => 3,
            Step::G => 4,
            Step::A => 5,
            Step::B => 6,
        }
    }

    pub fn from_index(index: i32) -> Option<Step> {
        let index = index.rem_euclid(7);
        Some(match index {
            0 => Step::C,
            1 => Step::D,
            2 => Step::E,
            3 => Step::F,
            4 => Step::G,
            5 => Step::A,
            6 => Step::B,
            _ => return None,
        })
    }

    pub fn parse(text: &str) -> Option<Step> {
        let mut chars = text.chars();
        let first = chars.next()?.to_ascii_uppercase();
        if chars.next().is_some() {
            return None;
        }
        Some(match first {
            'C' => Step::C,
            'D' => Step::D,
            'E' => Step::E,
            'F' => Step::F,
            'G' => Step::G,
            'A' => Step::A,
            'B' => Step::B,
            _ => return None,
        })
    }

    pub const fn name(self) -> char {
        match self {
            Step::C => 'C',
            Step::D => 'D',
            Step::E => 'E',
            Step::F => 'F',
            Step::G => 'G',
            Step::A => 'A',
            Step::B => 'B',
        }
    }

    pub const fn musicxml_step(self) -> &'static str {
        match self {
            Step::C => "C",
            Step::D => "D",
            Step::E => "E",
            Step::F => "F",
            Step::G => "G",
            Step::A => "A",
            Step::B => "B",
        }
    }
}

/// A written pitch: step, chromatic alteration and octave.
///
/// `octave` uses scientific pitch notation: C4 is middle C (MIDI 60).
/// Alterations are limited to double flat/sharp, matching the engraved
/// accidental set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Pitch {
    pub step: Step,
    pub alter: i8,
    pub octave: i8,
}

impl Pitch {
    pub const MIN_ALTER: i8 = -2;
    pub const MAX_ALTER: i8 = 2;
    pub const MIN_OCTAVE: i8 = -1;
    pub const MAX_OCTAVE: i8 = 9;

    pub const fn new(step: Step, alter: i8, octave: i8) -> Pitch {
        Pitch {
            step,
            alter,
            octave,
        }
    }

    pub fn c4() -> Pitch {
        Pitch::new(Step::C, 0, 4)
    }

    /// Validate range so every pitch maps to a legal MIDI note.
    pub fn validate(self) -> Result<()> {
        if self.alter < Self::MIN_ALTER || self.alter > Self::MAX_ALTER {
            return Err(Error::out_of_range(format!(
                "alteration {:+} on {}",
                self.alter,
                self.name()
            )));
        }
        if self.octave < Self::MIN_OCTAVE || self.octave > Self::MAX_OCTAVE {
            return Err(Error::out_of_range(format!("octave {}", self.octave)));
        }
        if self.midi().is_none() {
            return Err(Error::out_of_range(format!("pitch {}", self.name())));
        }
        Ok(())
    }

    /// MIDI note number, or `None` when outside 0..=127.
    pub fn midi(self) -> Option<u8> {
        let value = (self.octave as i32 + 1) * 12
            + self.step.semitone() as i32
            + self.alter as i32;
        if (0..=127).contains(&value) {
            Some(value as u8)
        } else {
            None
        }
    }

    pub fn midi_unchecked(self) -> i32 {
        (self.octave as i32 + 1) * 12 + self.step.semitone() as i32 + self.alter as i32
    }

    /// Diatonic position counting C0 as 0. Used for staff placement and
    /// chord notehead stacking.
    pub fn diatonic_index(self) -> i32 {
        self.octave as i32 * 7 + self.step.index() as i32
    }

    /// Chromatic position relative to C0. Used for key signature placement.
    pub fn chromatic_index(self) -> i32 {
        self.octave as i32 * 12 + self.step.semitone() as i32 + self.alter as i32
    }

    /// Spelled name such as `C#4`, `Bb3` or `F##2`.
    pub fn name(self) -> String {
        let mut out = String::new();
        out.push(self.step.name());
        match self.alter {
            2 => out.push_str("##"),
            1 => out.push('#'),
            0 => {}
            -1 => out.push('b'),
            -2 => out.push_str("bb"),
            other => {
                if other > 0 {
                    for _ in 0..other {
                        out.push('#');
                    }
                } else {
                    for _ in 0..(-other) {
                        out.push('b');
                    }
                }
            }
        }
        out.push_str(&self.octave.to_string());
        out
    }

    /// Parse scientific pitch notation with optional `#`, `b`, `##` or `bb`.
    pub fn parse(text: &str) -> Result<Pitch> {
        let text = text.trim();
        if text.is_empty() {
            return Err(Error::msg("empty pitch"));
        }
        let mut chars = text.chars().peekable();
        let step = chars
            .next()
            .and_then(|c| Step::parse(&c.to_string()))
            .ok_or_else(|| Error::msg(format!("invalid pitch step in {text:?}")))?;
        let mut alter = 0i8;
        while let Some(&c) = chars.peek() {
            match c {
                '#' => {
                    alter += 1;
                    chars.next();
                }
                'b' | 'B' => {
                    alter -= 1;
                    chars.next();
                }
                _ => break,
            }
        }
        let rest: String = chars.collect();
        if rest.is_empty() {
            return Err(Error::msg(format!("missing octave in {text:?}")));
        }
        let octave: i8 = rest
            .parse()
            .map_err(|_| Error::msg(format!("invalid octave in {text:?}")))?;
        let pitch = Pitch::new(step, alter, octave);
        pitch.validate()?;
        Ok(pitch)
    }

    /// Prefer the spelling that matches a key signature, then sharps or flats.
    pub fn spell_midi(midi: u8, prefer_flats: bool) -> Pitch {
        const SHARP_STEPS: [(Step, i8); 12] = [
            (Step::C, 0),
            (Step::C, 1),
            (Step::D, 0),
            (Step::D, 1),
            (Step::E, 0),
            (Step::F, 0),
            (Step::F, 1),
            (Step::G, 0),
            (Step::G, 1),
            (Step::A, 0),
            (Step::A, 1),
            (Step::B, 0),
        ];
        const FLAT_STEPS: [(Step, i8); 12] = [
            (Step::C, 0),
            (Step::D, -1),
            (Step::D, 0),
            (Step::E, -1),
            (Step::E, 0),
            (Step::F, 0),
            (Step::G, -1),
            (Step::G, 0),
            (Step::A, -1),
            (Step::A, 0),
            (Step::B, -1),
            (Step::B, 0),
        ];
        let table = if prefer_flats { FLAT_STEPS } else { SHARP_STEPS };
        let pc = (midi as i32).rem_euclid(12) as usize;
        let (step, alter) = table[pc];
        Pitch::new(step, alter, (midi as i32 / 12 - 1) as i8)
    }

    /// Move by a number of diatonic steps while preserving the accidental.
    pub fn step_by(self, steps: i32) -> Pitch {
        let index = self.diatonic_index() + steps;
        let octave = index.div_euclid(7);
        let step = Step::from_index(index.rem_euclid(7)).unwrap_or(Step::C);
        Pitch::new(step, self.alter, octave as i8)
    }

    /// Nearest diatonic step name for a MIDI number, without accidentals.
    pub fn white_key_name(midi: u8) -> (Step, i8) {
        let pitch = Pitch::spell_midi(midi, false);
        (pitch.step, pitch.octave)
    }
}

impl fmt::Display for Pitch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn midi_numbers() {
        assert_eq!(Pitch::parse("C4").unwrap().midi(), Some(60));
        assert_eq!(Pitch::parse("A4").unwrap().midi(), Some(69));
        assert_eq!(Pitch::parse("C-1").unwrap().midi(), Some(0));
        assert_eq!(Pitch::parse("G9").unwrap().midi(), Some(127));
    }

    #[test]
    fn spelling_round_trip() {
        for name in ["C4", "C#4", "Db4", "F##2", "Bbb5", "A0", "G9"] {
            let pitch = Pitch::parse(name).unwrap();
            assert_eq!(Pitch::parse(&pitch.name()).unwrap(), pitch);
        }
    }

    #[test]
    fn out_of_range_is_error() {
        assert!(Pitch::parse("C10").is_err());
        assert!(Pitch::parse("B#9").is_err());
        assert!(Pitch::parse("H4").is_err());
        assert!(Pitch::parse("C").is_err());
    }

    #[test]
    fn diatonic_and_chromatic_indexes() {
        assert_eq!(Pitch::parse("C4").unwrap().diatonic_index(), 28);
        assert_eq!(Pitch::parse("D4").unwrap().diatonic_index(), 29);
        assert_eq!(Pitch::parse("B3").unwrap().diatonic_index(), 27);
        assert_eq!(Pitch::parse("C4").unwrap().chromatic_index(), 48);
    }

    #[test]
    fn spell_round_trips_midi() {
        for midi in 0..=127u8 {
            for flats in [false, true] {
                let p = Pitch::spell_midi(midi, flats);
                assert_eq!(p.midi(), Some(midi));
            }
        }
    }
}
